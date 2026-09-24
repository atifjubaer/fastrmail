//! FastrMail SMTP — Inbound SMTP server with full state machine.
//!
//! Implements the core SMTP protocol: EHLO, MAIL FROM, RCPT TO, DATA, QUIT.
//! Saves received messages to disk (as .eml blobs) and indexes metadata in SQLite.

use std::sync::Arc;

use anyhow::{Context, Result};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tracing::{error, info, warn};
use uuid::Uuid;

use fastrmail_store::Database;

/// SMTP server that accepts inbound email connections.
pub struct SmtpServer {
    db: Arc<Database>,
    data_dir: String,
}

/// Internal state of a single SMTP session.
#[derive(Debug)]
struct SmtpSession {
    /// The sender address from MAIL FROM.
    sender: Option<String>,
    /// The recipient addresses from RCPT TO.
    recipients: Vec<String>,
    /// Whether the client has sent EHLO/HELO.
    greeted: bool,
}

impl SmtpSession {
    fn new() -> Self {
        Self {
            sender: None,
            recipients: Vec::new(),
            greeted: false,
        }
    }

    fn reset(&mut self) {
        self.sender = None;
        self.recipients.clear();
    }
}

impl SmtpServer {
    /// Create a new SMTP server backed by the given database.
    pub fn new(db: Arc<Database>, data_dir: String) -> Self {
        Self { db, data_dir }
    }

    /// Start the SMTP listener on the given address (e.g., "0.0.0.0:2525").
    pub async fn start(&self, addr: &str) -> Result<()> {
        let listener = TcpListener::bind(addr)
            .await
            .with_context(|| format!("Failed to bind SMTP listener on {addr}"))?;

        info!("SMTP listening on {addr}");

        loop {
            match listener.accept().await {
                Ok((stream, peer_addr)) => {
                    info!("SMTP connection from {peer_addr}");
                    let db = Arc::clone(&self.db);
                    let data_dir = self.data_dir.clone();
                    tokio::spawn(async move {
                        if let Err(e) = handle_connection(stream, db, data_dir).await {
                            error!("SMTP session error from {peer_addr}: {e}");
                        }
                    });
                }
                Err(e) => {
                    error!("Failed to accept SMTP connection: {e}");
                }
            }
        }
    }

    /// Start the SMTP listener and return the actual bound address.
    /// Useful for tests that bind to port 0.
    pub async fn start_with_addr(&self, addr: &str) -> Result<std::net::SocketAddr> {
        let listener = TcpListener::bind(addr)
            .await
            .with_context(|| format!("Failed to bind SMTP listener on {addr}"))?;

        let local_addr = listener.local_addr()?;
        info!("SMTP listening on {local_addr}");

        let db = Arc::clone(&self.db);
        let data_dir = self.data_dir.clone();

        tokio::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((stream, peer_addr)) => {
                        info!("SMTP connection from {peer_addr}");
                        let db = Arc::clone(&db);
                        let data_dir = data_dir.clone();
                        tokio::spawn(async move {
                            if let Err(e) = handle_connection(stream, db, data_dir).await {
                                error!("SMTP session error from {peer_addr}: {e}");
                            }
                        });
                    }
                    Err(e) => {
                        error!("Failed to accept SMTP connection: {e}");
                        break;
                    }
                }
            }
        });

        Ok(local_addr)
    }
}

/// Parse an email address from angle brackets: `<user@example.com>` → `user@example.com`.
fn parse_address(input: &str) -> String {
    let trimmed = input.trim();
    if let Some(start) = trimmed.find('<') {
        if let Some(end) = trimmed.find('>') {
            return trimmed[start + 1..end].to_string();
        }
    }
    trimmed.to_string()
}

/// Parse a simple header value from raw email data.
fn parse_header(data: &str, header_name: &str) -> Option<String> {
    let search = format!("{header_name}:");
    for line in data.lines() {
        let lower = line.to_lowercase();
        if lower.starts_with(&search.to_lowercase()) {
            return Some(line[search.len()..].trim().to_string());
        }
    }
    None
}

/// Handle a single SMTP connection through the full state machine.
async fn handle_connection(
    stream: TcpStream,
    db: Arc<Database>,
    data_dir: String,
) -> Result<()> {
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    let mut session = SmtpSession::new();

    // Send greeting banner
    writer
        .write_all(b"220 FastrMail ESMTP Ready\r\n")
        .await?;

    let mut line_buf = String::new();
    loop {
        line_buf.clear();
        let bytes_read = reader.read_line(&mut line_buf).await?;
        if bytes_read == 0 {
            // Connection closed by client
            break;
        }

        let line = line_buf.trim_end();
        let upper = line.to_uppercase();

        if upper.starts_with("EHLO") || upper.starts_with("HELO") {
            session.greeted = true;
            session.reset();
            writer
                .write_all(b"250-FastrMail\r\n250-SIZE 52428800\r\n250-8BITMIME\r\n250 OK\r\n")
                .await?;
        } else if upper.starts_with("MAIL FROM:") {
            if !session.greeted {
                writer
                    .write_all(b"503 Send EHLO/HELO first\r\n")
                    .await?;
                continue;
            }
            let addr_part = &line[10..]; // After "MAIL FROM:"
            session.sender = Some(parse_address(addr_part));
            session.recipients.clear();
            writer.write_all(b"250 OK\r\n").await?;
        } else if upper.starts_with("RCPT TO:") {
            if session.sender.is_none() {
                writer
                    .write_all(b"503 Send MAIL FROM first\r\n")
                    .await?;
                continue;
            }
            let addr_part = &line[8..]; // After "RCPT TO:"
            session.recipients.push(parse_address(addr_part));
            writer.write_all(b"250 OK\r\n").await?;
        } else if upper == "DATA" {
            if session.recipients.is_empty() {
                writer
                    .write_all(b"503 Send RCPT TO first\r\n")
                    .await?;
                continue;
            }
            writer
                .write_all(b"354 Start mail input; end with <CRLF>.<CRLF>\r\n")
                .await?;

            // Read message data until lone ".\r\n"
            let mut data = String::new();
            loop {
                let mut data_line = String::new();
                let n = reader.read_line(&mut data_line).await?;
                if n == 0 {
                    break;
                }
                if data_line.trim_end() == "." {
                    break;
                }
                // Handle dot-stuffing (RFC 5321 section 4.5.2)
                if data_line.starts_with("..") {
                    data.push_str(&data_line[1..]);
                } else {
                    data.push_str(&data_line);
                }
            }

            // Save the message
            let blob_id = Uuid::new_v4().to_string();
            let blob_dir = format!("{data_dir}/blobs");
            std::fs::create_dir_all(&blob_dir)?;
            let blob_path = format!("{blob_dir}/{blob_id}.eml");
            std::fs::write(&blob_path, data.as_bytes())?;

            let subject = parse_header(&data, "Subject");
            let from = parse_header(&data, "From")
                .or_else(|| session.sender.clone());
            let to = parse_header(&data, "To")
                .or_else(|| session.recipients.first().cloned());
            let size_bytes = data.len() as i64;

            // Try to find or create a tenant + account + mailbox for the first recipient
            // For now, use a default tenant and create on the fly if needed
            let default_tenant_id = match db.get_tenant_by_domain("localhost")? {
                Some(t) => t.id,
                None => db.insert_tenant("localhost")?,
            };

            // Create a default account if needed
            let recipient_email = session
                .recipients
                .first()
                .cloned()
                .unwrap_or_else(|| "postmaster@localhost".to_string());

            let account_id = match db.get_account_by_email(&recipient_email)? {
                Some(a) => a.id,
                None => {
                    let username = recipient_email
                        .split('@')
                        .next()
                        .unwrap_or("postmaster");
                    db.insert_account(
                        &default_tenant_id,
                        username,
                        &recipient_email,
                        "no-password-set",
                    )?
                }
            };

            // Ensure INBOX exists
            let mailboxes = db.get_mailboxes(&account_id)?;
            let inbox_id = if let Some(inbox) = mailboxes.iter().find(|m| m.name == "INBOX") {
                inbox.id.clone()
            } else {
                db.insert_mailbox(&account_id, "INBOX")?
            };

            // Insert the message into the database
            db.insert_message(
                &inbox_id,
                &account_id,
                &blob_id,
                size_bytes,
                subject.as_deref(),
                from.as_deref(),
                to.as_deref(),
            )?;

            info!(
                "Message saved: blob={blob_id} from={} to={} subject={}",
                from.as_deref().unwrap_or("?"),
                to.as_deref().unwrap_or("?"),
                subject.as_deref().unwrap_or("(no subject)")
            );

            writer
                .write_all(format!("250 OK message {blob_id} accepted\r\n").as_bytes())
                .await?;

            session.reset();
        } else if upper.starts_with("RSET") {
            session.reset();
            writer.write_all(b"250 OK\r\n").await?;
        } else if upper.starts_with("NOOP") {
            writer.write_all(b"250 OK\r\n").await?;
        } else if upper.starts_with("QUIT") {
            writer.write_all(b"221 Bye\r\n").await?;
            break;
        } else {
            warn!("Unknown SMTP command: {line}");
            writer
                .write_all(b"500 Command not recognized\r\n")
                .await?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::AsyncWriteExt;
    use tokio::net::TcpStream;

    /// Helper to read a response line from the SMTP server.
    async fn read_response(reader: &mut BufReader<tokio::net::tcp::OwnedReadHalf>) -> String {
        let mut buf = String::new();
        reader.read_line(&mut buf).await.unwrap();
        buf
    }

    /// Helper to read all multi-line response (lines starting with "250-" then final "250 ").
    async fn read_multiline_response(
        reader: &mut BufReader<tokio::net::tcp::OwnedReadHalf>,
    ) -> Vec<String> {
        let mut lines = Vec::new();
        loop {
            let mut buf = String::new();
            reader.read_line(&mut buf).await.unwrap();
            let is_continuation = buf.starts_with("250-");
            lines.push(buf);
            if !is_continuation {
                break;
            }
        }
        lines
    }

    #[tokio::test]
    async fn test_smtp_full_session() {
        // Set up in-memory database
        let db = Database::new_memory().expect("Failed to create in-memory DB");
        db.init_schema().expect("Failed to init schema");
        let db = Arc::new(db);

        // Create a temp directory for blobs
        let temp_dir = std::env::temp_dir().join(format!("fastrmail_test_{}", Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let data_dir = temp_dir.to_string_lossy().to_string();

        // Start the SMTP server on a random port
        let server = SmtpServer::new(Arc::clone(&db), data_dir.clone());
        let addr = server
            .start_with_addr("127.0.0.1:0")
            .await
            .expect("Failed to start SMTP server");

        // Give the server a moment to be ready
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        // Connect a client
        let stream = TcpStream::connect(addr)
            .await
            .expect("Failed to connect to SMTP server");
        let (read_half, mut write_half) = stream.into_split();
        let mut reader = BufReader::new(read_half);

        // 1. Read the greeting banner
        let greeting = read_response(&mut reader).await;
        assert!(
            greeting.starts_with("220"),
            "Expected 220 greeting, got: {greeting}"
        );

        // 2. Send EHLO
        write_half.write_all(b"EHLO test.local\r\n").await.unwrap();
        let ehlo_response = read_multiline_response(&mut reader).await;
        assert!(
            ehlo_response.last().unwrap().starts_with("250"),
            "Expected 250 after EHLO"
        );

        // 3. MAIL FROM
        write_half
            .write_all(b"MAIL FROM:<sender@test.local>\r\n")
            .await
            .unwrap();
        let mail_response = read_response(&mut reader).await;
        assert!(
            mail_response.starts_with("250"),
            "Expected 250 after MAIL FROM, got: {mail_response}"
        );

        // 4. RCPT TO
        write_half
            .write_all(b"RCPT TO:<recipient@localhost>\r\n")
            .await
            .unwrap();
        let rcpt_response = read_response(&mut reader).await;
        assert!(
            rcpt_response.starts_with("250"),
            "Expected 250 after RCPT TO, got: {rcpt_response}"
        );

        // 5. DATA
        write_half.write_all(b"DATA\r\n").await.unwrap();
        let data_response = read_response(&mut reader).await;
        assert!(
            data_response.starts_with("354"),
            "Expected 354 after DATA, got: {data_response}"
        );

        // 6. Send the email body
        write_half
            .write_all(
                b"From: sender@test.local\r\n\
                  To: recipient@localhost\r\n\
                  Subject: Test Email from SMTP\r\n\
                  \r\n\
                  This is the body of the test email.\r\n\
                  .\r\n",
            )
            .await
            .unwrap();

        let data_ok = read_response(&mut reader).await;
        assert!(
            data_ok.starts_with("250"),
            "Expected 250 after message data, got: {data_ok}"
        );

        // 7. QUIT
        write_half.write_all(b"QUIT\r\n").await.unwrap();
        let quit_response = read_response(&mut reader).await;
        assert!(
            quit_response.starts_with("221"),
            "Expected 221 after QUIT, got: {quit_response}"
        );

        // 8. Verify the message was saved in the database
        let account = db
            .get_account_by_email("recipient@localhost")
            .unwrap()
            .expect("Account should have been created");
        let messages = db.get_messages(&account.id).unwrap();
        assert_eq!(messages.len(), 1, "Should have exactly 1 message");
        assert_eq!(
            messages[0].parsed_subject.as_deref(),
            Some("Test Email from SMTP")
        );

        // 9. Verify the blob file was created on disk
        let blob_dir = format!("{data_dir}/blobs");
        let entries: Vec<_> = std::fs::read_dir(&blob_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .collect();
        assert_eq!(entries.len(), 1, "Should have exactly 1 blob file");
        assert!(entries[0].path().extension().unwrap() == "eml");

        // Cleanup
        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_parse_address() {
        assert_eq!(parse_address("<user@example.com>"), "user@example.com");
        assert_eq!(parse_address("  <admin@test.org>  "), "admin@test.org");
        assert_eq!(parse_address("bare@address.com"), "bare@address.com");
    }

    #[test]
    fn test_parse_header() {
        let email = "From: Alice <alice@example.com>\r\nTo: bob@test.com\r\nSubject: Hello World\r\n\r\nBody";
        assert_eq!(
            parse_header(email, "Subject"),
            Some("Hello World".to_string())
        );
        assert_eq!(
            parse_header(email, "From"),
            Some("Alice <alice@example.com>".to_string())
        );
        assert_eq!(
            parse_header(email, "To"),
            Some("bob@test.com".to_string())
        );
        assert_eq!(parse_header(email, "X-Missing"), None);
    }

    #[tokio::test]
    async fn test_smtp_protocol_errors() {
        let db = Database::new_memory().expect("Failed to create in-memory DB");
        db.init_schema().expect("Failed to init schema");
        let db = Arc::new(db);

        let temp_dir = std::env::temp_dir().join(format!("fastrmail_test_err_{}", Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let data_dir = temp_dir.to_string_lossy().to_string();

        let server = SmtpServer::new(Arc::clone(&db), data_dir);
        let addr = server
            .start_with_addr("127.0.0.1:0")
            .await
            .expect("Failed to start SMTP server");

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let stream = TcpStream::connect(addr).await.unwrap();
        let (read_half, mut write_half) = stream.into_split();
        let mut reader = BufReader::new(read_half);

        // Read greeting
        let _ = read_response(&mut reader).await;

        // Try MAIL FROM without EHLO → should get 503
        write_half
            .write_all(b"MAIL FROM:<test@test.com>\r\n")
            .await
            .unwrap();
        let resp = read_response(&mut reader).await;
        assert!(
            resp.starts_with("503"),
            "Expected 503 without EHLO, got: {resp}"
        );

        // Send unknown command → should get 500
        write_half
            .write_all(b"FOOBAR\r\n")
            .await
            .unwrap();
        let resp = read_response(&mut reader).await;
        assert!(
            resp.starts_with("500"),
            "Expected 500 for unknown cmd, got: {resp}"
        );

        // QUIT
        write_half.write_all(b"QUIT\r\n").await.unwrap();
        let resp = read_response(&mut reader).await;
        assert!(resp.starts_with("221"));

        std::fs::remove_dir_all(&temp_dir).ok();
    }
}
