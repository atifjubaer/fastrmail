//! FastrMail SMTP — Inbound SMTP server with full state machine and security verification,
//! plus outbound delivery queue processing.

pub mod outbound;
pub use outbound::OutboundEngine;

use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::{Context, Result};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tracing::{error, info, warn};
use uuid::Uuid;

use mail_parser::MessageParser;
use fastrmail_auth::{DkimVerifier, DmarcEvaluator, DnsblVerifier, SpfVerifier};
use fastrmail_core::Message;
use fastrmail_search::SearchEngine;
use fastrmail_store::Database;

/// SMTP server that accepts inbound email connections.
pub struct SmtpServer {
    db: Arc<Database>,
    data_dir: String,
    search_engine: Option<Arc<SearchEngine>>,
    dnsbl_verifier: Option<Arc<DnsblVerifier>>,
    bypass_spam_check: bool,
    is_submission: bool,
}

/// Internal state of a single SMTP session.
#[derive(Debug)]
struct SmtpSession {
    /// HELO/EHLO domain.
    helo: Option<String>,
    /// The sender address from MAIL FROM.
    sender: Option<String>,
    /// The recipient addresses from RCPT TO.
    recipients: Vec<String>,
    /// Whether the client has sent EHLO/HELO.
    greeted: bool,
    /// The authenticated username/email, if AUTH was successful.
    authenticated_user: Option<String>,
}

impl SmtpSession {
    fn new() -> Self {
        Self {
            helo: None,
            sender: None,
            recipients: Vec::new(),
            greeted: false,
            authenticated_user: None,
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
        Self {
            db,
            data_dir,
            search_engine: None,
            dnsbl_verifier: Some(Arc::new(DnsblVerifier::new())),
            bypass_spam_check: false,
            is_submission: false,
        }
    }

    /// Create a new SMTP submission server (e.g. for port 587) where authentication is mandatory.
    pub fn new_submission(db: Arc<Database>, data_dir: String) -> Self {
        Self {
            db,
            data_dir,
            search_engine: None,
            dnsbl_verifier: None,
            bypass_spam_check: true,
            is_submission: true,
        }
    }

    /// Configure whether this server operates in client submission mode (requires AUTH).
    pub fn with_submission(mut self, submission: bool) -> Self {
        self.is_submission = submission;
        self
    }

    /// Attach an optional Tantivy SearchEngine for automatic inbound email indexing.
    pub fn with_search_engine(mut self, search_engine: Arc<SearchEngine>) -> Self {
        self.search_engine = Some(search_engine);
        self
    }

    /// Attach an optional custom DNSBL verifier (useful for testing or custom blocklists).
    pub fn with_dnsbl_verifier(mut self, dnsbl_verifier: Arc<DnsblVerifier>) -> Self {
        self.dnsbl_verifier = Some(dnsbl_verifier);
        self
    }

    /// Configure whether spam checks (DNSBL and Greylisting) should be bypassed.
    pub fn with_bypass_spam_check(mut self, bypass: bool) -> Self {
        self.bypass_spam_check = bypass;
        self
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
                    let search_engine = self.search_engine.clone();
                    let dnsbl = self.dnsbl_verifier.clone();
                    let bypass = self.bypass_spam_check;
                    let is_sub = self.is_submission;
                    tokio::spawn(async move {
                        if let Err(e) = handle_connection(stream, peer_addr, db, data_dir, search_engine, dnsbl, bypass, is_sub).await {
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
    pub async fn start_with_addr(&self, addr: &str) -> Result<SocketAddr> {
        let listener = TcpListener::bind(addr)
            .await
            .with_context(|| format!("Failed to bind SMTP listener on {addr}"))?;

        let local_addr = listener.local_addr()?;
        info!("SMTP listening on {local_addr}");

        let db = Arc::clone(&self.db);
        let data_dir = self.data_dir.clone();
        let search_engine = self.search_engine.clone();
        let dnsbl = self.dnsbl_verifier.clone();
        let bypass = self.bypass_spam_check;
        let is_sub = self.is_submission;

        tokio::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((stream, peer_addr)) => {
                        info!("SMTP connection from {peer_addr}");
                        let db = Arc::clone(&db);
                        let data_dir = data_dir.clone();
                        let search_engine = search_engine.clone();
                        let dnsbl = dnsbl.clone();
                        tokio::spawn(async move {
                            if let Err(e) = handle_connection(stream, peer_addr, db, data_dir, search_engine, dnsbl, bypass, is_sub).await {
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

    /// Start a submission SMTP listener and return the actual bound address.
    pub async fn start_submission_with_addr(&self, addr: &str) -> Result<SocketAddr> {
        let submission_server = Self {
            db: Arc::clone(&self.db),
            data_dir: self.data_dir.clone(),
            search_engine: self.search_engine.clone(),
            dnsbl_verifier: None,
            bypass_spam_check: true,
            is_submission: true,
        };
        submission_server.start_with_addr(addr).await
    }
}

/// Parse an email address from angle brackets: `<user@example.com>` → `user@example.com`.
pub fn parse_address(input: &str) -> String {
    let trimmed = input.trim();
    if let Some(start) = trimmed.find('<') {
        if let Some(end) = trimmed.find('>') {
            return trimmed[start + 1..end].to_string();
        }
    }
    trimmed.to_string()
}

/// Parse a simple header value from raw email data.
pub fn parse_header(data: &str, header_name: &str) -> Option<String> {
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
    peer_addr: SocketAddr,
    db: Arc<Database>,
    data_dir: String,
    search_engine: Option<Arc<SearchEngine>>,
    dnsbl_verifier: Option<Arc<DnsblVerifier>>,
    bypass_spam_check: bool,
    is_submission: bool,
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
            let domain_part = if upper.starts_with("EHLO ") {
                line[5..].trim().to_string()
            } else if upper.starts_with("HELO ") {
                line[5..].trim().to_string()
            } else {
                "localhost".to_string()
            };
            session.helo = Some(domain_part);
            writer
                .write_all(b"250-FastrMail\r\n250-SIZE 52428800\r\n250-8BITMIME\r\n250-AUTH PLAIN LOGIN\r\n250 OK\r\n")
                .await?;
        } else if upper.starts_with("AUTH ") || upper == "AUTH" {
            let mechanism_part = if upper == "AUTH" { "" } else { line[4..].trim() };
            let upper_mech = mechanism_part.to_uppercase();

            if upper_mech.starts_with("PLAIN") {
                let auth_arg = mechanism_part["PLAIN".len()..].trim();
                let b64_payload = if !auth_arg.is_empty() {
                    auth_arg.to_string()
                } else {
                    writer.write_all(b"334 \r\n").await?;
                    let mut resp_buf = String::new();
                    let n = reader.read_line(&mut resp_buf).await?;
                    if n == 0 {
                        break;
                    }
                    resp_buf.trim().to_string()
                };

                let mut clean_b64 = b64_payload.clone();
                clean_b64.retain(|c| !c.is_whitespace());
                use base64::Engine;
                let decoded = base64::engine::general_purpose::STANDARD.decode(clean_b64);

                let mut authenticated = false;
                if let Ok(bytes) = decoded {
                    let parts: Vec<&[u8]> = bytes.split(|&b| b == 0).collect();
                    let (user, pass) = if parts.len() >= 3 {
                        (String::from_utf8_lossy(parts[1]), String::from_utf8_lossy(parts[2]))
                    } else if parts.len() == 2 {
                        (String::from_utf8_lossy(parts[0]), String::from_utf8_lossy(parts[1]))
                    } else {
                        (std::borrow::Cow::Borrowed(""), std::borrow::Cow::Borrowed(""))
                    };

                    if !user.is_empty() {
                        if let Ok(Some(account)) = db.verify_login(&user, &pass) {
                            session.authenticated_user = Some(account.email);
                            authenticated = true;
                        }
                    }
                }

                if authenticated {
                    writer.write_all(b"235 2.7.0 Authentication successful\r\n").await?;
                } else {
                    writer.write_all(b"535 5.7.8 Authentication credentials invalid\r\n").await?;
                }
            } else if upper_mech.starts_with("LOGIN") {
                let auth_arg = mechanism_part["LOGIN".len()..].trim();
                let user_b64 = if !auth_arg.is_empty() {
                    auth_arg.to_string()
                } else {
                    writer.write_all(b"334 VXNlcm5hbWU6\r\n").await?;
                    let mut resp_buf = String::new();
                    let n = reader.read_line(&mut resp_buf).await?;
                    if n == 0 {
                        break;
                    }
                    resp_buf.trim().to_string()
                };

                writer.write_all(b"334 UGFzc3dvcmQ6\r\n").await?;
                let mut pass_buf = String::new();
                let n = reader.read_line(&mut pass_buf).await?;
                if n == 0 {
                    break;
                }
                let pass_b64 = pass_buf.trim().to_string();

                use base64::Engine;
                let user_dec = base64::engine::general_purpose::STANDARD.decode(user_b64.trim());
                let pass_dec = base64::engine::general_purpose::STANDARD.decode(pass_b64.trim());

                let mut authenticated = false;
                if let (Ok(u_bytes), Ok(p_bytes)) = (user_dec, pass_dec) {
                    let user = String::from_utf8_lossy(&u_bytes);
                    let pass = String::from_utf8_lossy(&p_bytes);
                    if let Ok(Some(account)) = db.verify_login(&user, &pass) {
                        session.authenticated_user = Some(account.email);
                        authenticated = true;
                    }
                }

                if authenticated {
                    writer.write_all(b"235 2.7.0 Authentication successful\r\n").await?;
                } else {
                    writer.write_all(b"535 5.7.8 Authentication credentials invalid\r\n").await?;
                }
            } else {
                writer.write_all(b"504 5.5.4 Unrecognized authentication mechanism\r\n").await?;
            }
        } else if upper.starts_with("MAIL FROM:") {
            if !session.greeted {
                writer
                    .write_all(b"503 Send EHLO/HELO first\r\n")
                    .await?;
                continue;
            }
            if is_submission && session.authenticated_user.is_none() {
                writer
                    .write_all(b"530 5.7.0 Authentication required\r\n")
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
            let recipient = parse_address(addr_part);
            let sender = session.sender.as_deref().unwrap_or("");

            // Spam & Reputation Verification (DNSBL + Greylisting)
            let skip_spam = bypass_spam_check || session.authenticated_user.is_some();
            if !skip_spam {
                // 1. DNSBL check
                if let Some(dnsbl) = &dnsbl_verifier {
                    if dnsbl.check_ip(peer_addr.ip()).await.unwrap_or(false) {
                        writer
                            .write_all(
                                format!(
                                    "554 5.7.1 Service unavailable; Client host [{}] blocked using DNSBL\r\n",
                                    peer_addr.ip()
                                )
                                .as_bytes(),
                            )
                            .await?;
                        return Ok(());
                    }
                }

                // 2. Greylisting check
                let client_ip = peer_addr.ip().to_string();
                let passed = db.check_greylist(&client_ip, sender, &recipient).unwrap_or(true);
                if !passed {
                    writer
                        .write_all(b"451 4.7.1 Greylisting in action, please try again later\r\n")
                        .await?;
                    return Ok(());
                }
            }

            session.recipients.push(recipient);
            writer.write_all(b"250 OK\r\n").await?;
        } else if upper == "DATA" {
            if is_submission && session.authenticated_user.is_none() {
                writer
                    .write_all(b"530 5.7.0 Authentication required\r\n")
                    .await?;
                continue;
            }
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

            // 1. Extract sender IP
            let peer_ip = peer_addr.ip();
            let helo_domain = session.helo.as_deref().unwrap_or("localhost");
            let sender = session.sender.as_deref().unwrap_or("");

            // 2. Call SpfVerifier::verify_spf()
            let spf_result = SpfVerifier::verify_spf(peer_ip, helo_domain, sender).await?;

            // 3. Call DkimVerifier::verify_dkim()
            let dkim_result = DkimVerifier::verify_dkim(data.as_bytes()).await?;

            // 4. Extract From domain and call DmarcEvaluator::evaluate()
            let from_header = parse_header(&data, "From").unwrap_or_else(|| sender.to_string());
            let from_addr = parse_address(&from_header);
            let from_domain = from_addr.split('@').nth(1).unwrap_or("localhost");

            let dmarc_result = DmarcEvaluator::evaluate(&dkim_result, &spf_result, from_domain).await?;

            info!(
                "Inbound auth results: ip={} sender={} from_domain={} spf_pass={} dkim_pass={} dmarc_pass={} dmarc_policy={}",
                peer_ip, sender, from_domain, spf_result.pass, dkim_result.pass, dmarc_result.pass, dmarc_result.policy
            );

            // 5. If DMARC policy is "reject" and result is fail, respond "550 DMARC policy rejection"
            if dmarc_result.policy == "reject" && !dmarc_result.pass {
                warn!(
                    "Rejecting email from {} due to DMARC policy=reject and failed alignment",
                    from_domain
                );
                writer.write_all(b"550 DMARC policy rejection\r\n").await?;
                session.reset();
                continue;
            }

            // 6. Prepend Authentication-Results header
            let dkim_status = if dkim_result.pass { "pass" } else { "none" };
            let spf_status = if spf_result.pass { "pass" } else { "neutral" };
            let dmarc_status = if dmarc_result.pass { "pass" } else { "fail" };
            let auth_header = format!(
                "Authentication-Results: fastrmail; dkim={} header.d={}; spf={} smtp.mailfrom={}; dmarc={} (p={})\r\n",
                dkim_status,
                if dkim_result.domain.is_empty() { "none" } else { &dkim_result.domain },
                spf_status,
                sender,
                dmarc_status,
                dmarc_result.policy
            );

            let mut final_message = auth_header;
            final_message.push_str(&data);

            // Save the message blob to disk
            let blob_id = Uuid::new_v4().to_string();
            let blob_dir = format!("{data_dir}/blobs");
            std::fs::create_dir_all(&blob_dir)?;
            let blob_path = format!("{blob_dir}/{blob_id}.eml");
            std::fs::write(&blob_path, final_message.as_bytes())?;

            let subject = parse_header(&data, "Subject");
            let from = parse_header(&data, "From")
                .or_else(|| session.sender.clone());
            let to = parse_header(&data, "To")
                .or_else(|| session.recipients.first().cloned());
            let size_bytes = final_message.len() as i64;

            // Get or create tenant and account
            let default_tenant_id = match db.get_tenant_by_domain("localhost")? {
                Some(t) => t.id,
                None => db.insert_tenant("localhost")?,
            };

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
            let msg_id = db.insert_message(
                &inbox_id,
                &account_id,
                &blob_id,
                size_bytes,
                subject.as_deref(),
                from.as_deref(),
                to.as_deref(),
            )?;

            // Index in Tantivy search engine if configured
            if let Some(ref engine) = search_engine {
                let plain_body = MessageParser::default()
                    .parse(final_message.as_bytes())
                    .and_then(|p| p.body_text(0).map(|s| s.to_string()))
                    .unwrap_or_else(|| data.clone());

                let (uid, modseq) = match db.get_mailbox_by_id(&inbox_id) {
                    Ok(Some(mb)) => (mb.uid_next - 1, mb.modseq),
                    _ => (1, 1),
                };

                let msg_to_index = Message {
                    id: msg_id.clone(),
                    mailbox_id: inbox_id.clone(),
                    account_id: account_id.clone(),
                    uid,
                    modseq,
                    blob_id: blob_id.clone(),
                    size_bytes,
                    parsed_subject: subject.clone(),
                    parsed_from: from.clone(),
                    parsed_to: to.clone(),
                    internal_date: chrono::Utc::now(),
                    flags: "[]".to_string(),
                };

                if let Err(e) = engine.index_message(&msg_to_index, &plain_body) {
                    warn!("Failed to index message in Tantivy: {e}");
                }
            }

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
    use fastrmail_auth::DkimSigner;
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
    async fn test_smtp_full_session_with_authentication_header() {
        let db = Database::new_memory().expect("Failed to create in-memory DB");
        db.init_schema().expect("Failed to init schema");
        let db = Arc::new(db);

        let temp_dir = std::env::temp_dir().join(format!("fastrmail_test_{}", Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let data_dir = temp_dir.to_string_lossy().to_string();

        let server = SmtpServer::new(Arc::clone(&db), data_dir.clone())
            .with_bypass_spam_check(true);
        let addr = server
            .start_with_addr("127.0.0.1:0")
            .await
            .expect("Failed to start SMTP server");

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let stream = TcpStream::connect(addr).await.expect("Failed to connect");
        let (read_half, mut write_half) = stream.into_split();
        let mut reader = BufReader::new(read_half);

        let _ = read_response(&mut reader).await;

        write_half.write_all(b"EHLO test.local\r\n").await.unwrap();
        let _ = read_multiline_response(&mut reader).await;

        write_half
            .write_all(b"MAIL FROM:<sender@test.local>\r\n")
            .await
            .unwrap();
        let _ = read_response(&mut reader).await;

        write_half
            .write_all(b"RCPT TO:<recipient@localhost>\r\n")
            .await
            .unwrap();
        let _ = read_response(&mut reader).await;

        write_half.write_all(b"DATA\r\n").await.unwrap();
        let _ = read_response(&mut reader).await;

        write_half
            .write_all(
                b"From: sender@test.local\r\n\
                  To: recipient@localhost\r\n\
                  Subject: Test Inbound with Auth\r\n\
                  \r\n\
                  Hello Authenticated World!\r\n\
                  .\r\n",
            )
            .await
            .unwrap();

        let data_ok = read_response(&mut reader).await;
        assert!(data_ok.starts_with("250"), "Expected 250, got: {data_ok}");

        write_half.write_all(b"QUIT\r\n").await.unwrap();
        let _ = read_response(&mut reader).await;

        // Verify message blob on disk contains Authentication-Results header
        let blob_dir = format!("{data_dir}/blobs");
        let entries: Vec<_> = std::fs::read_dir(&blob_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .collect();
        assert_eq!(entries.len(), 1);

        let content = std::fs::read_to_string(entries[0].path()).unwrap();
        assert!(
            content.contains("Authentication-Results: fastrmail;"),
            "Saved email must contain Authentication-Results header"
        );

        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[tokio::test]
    async fn test_smtp_signed_dkim_inbound() {
        let db = Database::new_memory().expect("Failed to create in-memory DB");
        db.init_schema().expect("Failed to init schema");
        let db = Arc::new(db);

        let temp_dir = std::env::temp_dir().join(format!("fastrmail_dkim_in_{}", Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let data_dir = temp_dir.to_string_lossy().to_string();

        let server = SmtpServer::new(Arc::clone(&db), data_dir.clone())
            .with_bypass_spam_check(true);
        let addr = server
            .start_with_addr("127.0.0.1:0")
            .await
            .expect("Failed to start SMTP server");

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        // Generate key and sign email with a neutral test domain
        let keys = DkimSigner::generate_key_pair().unwrap();
        let raw_body = b"From: sender@fastrmail.local\r\n\
                         To: recipient@localhost\r\n\
                         Subject: Signed Message\r\n\
                         \r\n\
                         Body with DKIM signature.";

        let signed = DkimSigner::sign(raw_body, "fastrmail.local", "default", &keys.private_key_pem).unwrap();

        let stream = TcpStream::connect(addr).await.unwrap();
        let (read_half, mut write_half) = stream.into_split();
        let mut reader = BufReader::new(read_half);

        let _ = read_response(&mut reader).await;
        write_half.write_all(b"EHLO client.local\r\n").await.unwrap();
        let _ = read_multiline_response(&mut reader).await;
        write_half.write_all(b"MAIL FROM:<sender@fastrmail.local>\r\n").await.unwrap();
        let _ = read_response(&mut reader).await;
        write_half.write_all(b"RCPT TO:<recipient@localhost>\r\n").await.unwrap();
        let _ = read_response(&mut reader).await;
        write_half.write_all(b"DATA\r\n").await.unwrap();
        let _ = read_response(&mut reader).await;

        write_half.write_all(&signed).await.unwrap();
        write_half.write_all(b"\r\n.\r\n").await.unwrap();
        let data_ok = read_response(&mut reader).await;
        assert!(data_ok.starts_with("250"), "Expected 250 for neutral domain, got: {data_ok}");

        write_half.write_all(b"QUIT\r\n").await.unwrap();

        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[tokio::test]
    async fn test_smtp_dmarc_policy_rejection() {
        let db = Database::new_memory().expect("Failed to create in-memory DB");
        db.init_schema().expect("Failed to init schema");
        let db = Arc::new(db);

        let temp_dir = std::env::temp_dir().join(format!("fastrmail_dmarc_rej_{}", Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let data_dir = temp_dir.to_string_lossy().to_string();

        let server = SmtpServer::new(Arc::clone(&db), data_dir.clone())
            .with_bypass_spam_check(true);
        let addr = server
            .start_with_addr("127.0.0.1:0")
            .await
            .expect("Failed to start SMTP server");

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        // example.com publishes a real DNS DMARC policy of p=reject.
        // Sending from sender@example.com without valid SPF/DKIM will trigger 550 rejection.
        let stream = TcpStream::connect(addr).await.unwrap();
        let (read_half, mut write_half) = stream.into_split();
        let mut reader = BufReader::new(read_half);

        let _ = read_response(&mut reader).await;
        write_half.write_all(b"EHLO client.local\r\n").await.unwrap();
        let _ = read_multiline_response(&mut reader).await;
        write_half.write_all(b"MAIL FROM:<attacker@example.com>\r\n").await.unwrap();
        let _ = read_response(&mut reader).await;
        write_half.write_all(b"RCPT TO:<victim@localhost>\r\n").await.unwrap();
        let _ = read_response(&mut reader).await;
        write_half.write_all(b"DATA\r\n").await.unwrap();
        let _ = read_response(&mut reader).await;

        write_half
            .write_all(
                b"From: attacker@example.com\r\n\
                  To: victim@localhost\r\n\
                  Subject: Forged Email\r\n\
                  \r\n\
                  I am pretending to be from example.com!\r\n\
                  .\r\n",
            )
            .await
            .unwrap();

        let data_resp = read_response(&mut reader).await;
        assert!(
            data_resp.starts_with("550"),
            "Expected 550 DMARC policy rejection, got: {data_resp}"
        );
        assert!(data_resp.contains("DMARC policy rejection"));

        // Verify message was NOT stored
        let blob_dir = format!("{data_dir}/blobs");
        let entries_count = std::fs::read_dir(&blob_dir).map(|rd| rd.count()).unwrap_or(0);
        assert_eq!(entries_count, 0, "Rejected message must not be stored on disk");

        write_half.write_all(b"QUIT\r\n").await.unwrap();

        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[tokio::test]
    async fn test_outbound_engine_delivery() {
        let db = Database::new_memory().expect("Failed to create in-memory DB");
        db.init_schema().expect("Failed to init schema");
        let db = Arc::new(db);

        let temp_dir = std::env::temp_dir().join(format!("fastrmail_outbound_{}", Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let data_dir = temp_dir.to_string_lossy().to_string();

        // 1. Start a mock receiving SMTP server on port 0
        let mock_server = SmtpServer::new(Arc::clone(&db), data_dir.clone())
            .with_bypass_spam_check(true);
        let mock_addr = mock_server.start_with_addr("127.0.0.1:0").await.unwrap();
        let mock_port = mock_addr.port();

        // 2. Write an email blob to disk
        let blob_dir = format!("{data_dir}/blobs");
        std::fs::create_dir_all(&blob_dir).unwrap();
        let blob_id = Uuid::new_v4().to_string();
        let blob_path = format!("{blob_dir}/{blob_id}.eml");
        std::fs::write(
            &blob_path,
            b"From: outbound@fastrmail.com\r\n\
              To: mock@localhost\r\n\
              Subject: Outbound Engine Delivery Test\r\n\
              \r\n\
              Delivery body content.",
        )
        .unwrap();

        // 3. Queue the email in Database
        let tenant_id = db.insert_tenant("fastrmail.com").unwrap();
        let _queue_id = db
            .queue_email(&tenant_id, &blob_id, "outbound@fastrmail.com", "mock@localhost")
            .unwrap();

        let pending = db.get_queue_pending().unwrap();
        assert_eq!(pending.len(), 1);

        // 4. Run OutboundEngine with target port pointing to mock server
        let engine = OutboundEngine::with_port(Arc::clone(&db), data_dir.clone(), mock_port);
        engine.process_queue().await.unwrap();

        // 5. Verify the queue item was delivered and removed from queue
        let after_delivery = db.get_queue_pending().unwrap();
        assert!(
            after_delivery.is_empty(),
            "Queue should be empty after successful delivery"
        );

        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[tokio::test]
    async fn test_smtp_inbound_with_tantivy_indexing() {
        let db = Database::new_memory().expect("Failed to create in-memory DB");
        db.init_schema().expect("Failed to init schema");
        let db = Arc::new(db);

        let search_engine = Arc::new(SearchEngine::new_in_ram().unwrap());

        let temp_dir = std::env::temp_dir().join(format!("fastrmail_smtp_search_{}", Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let data_dir = temp_dir.to_string_lossy().to_string();

        let server = SmtpServer::new(Arc::clone(&db), data_dir.clone())
            .with_search_engine(Arc::clone(&search_engine))
            .with_bypass_spam_check(true);
        let server_addr = server.start_with_addr("127.0.0.1:0").await.unwrap();

        // Connect and send an email with unique keyword "quantum-teleportation"
        let stream = TcpStream::connect(server_addr).await.unwrap();
        let (read_half, mut write_half) = stream.into_split();
        let mut reader = BufReader::new(read_half);
        let mut line = String::new();

        reader.read_line(&mut line).await.unwrap();

        write_half.write_all(b"EHLO test.local\r\n").await.unwrap();
        loop {
            line.clear();
            reader.read_line(&mut line).await.unwrap();
            if line.starts_with("250 ") {
                break;
            }
        }

        write_half.write_all(b"MAIL FROM:<scientist@lab.local>\r\n").await.unwrap();
        line.clear();
        reader.read_line(&mut line).await.unwrap();
        assert!(line.starts_with("250"));

        write_half.write_all(b"RCPT TO:<alice@localhost>\r\n").await.unwrap();
        line.clear();
        reader.read_line(&mut line).await.unwrap();
        assert!(line.starts_with("250"));

        write_half.write_all(b"DATA\r\n").await.unwrap();
        line.clear();
        reader.read_line(&mut line).await.unwrap();
        assert!(line.starts_with("354"));

        let email_body = b"From: scientist@lab.local\r\n\
                           To: alice@localhost\r\n\
                           Subject: Research Paper on Quantum Computing\r\n\
                           \r\n\
                           We have achieved quantum-teleportation in our new laboratory experiments.\r\n\
                           .\r\n";
        write_half.write_all(email_body).await.unwrap();
        line.clear();
        reader.read_line(&mut line).await.unwrap();
        assert!(line.starts_with("250"));

        write_half.write_all(b"QUIT\r\n").await.unwrap();

        // Verify that Tantivy indexed the email and can find "quantum-teleportation"
        let account = db.get_account_by_email("alice@localhost").unwrap().unwrap();
        let results = search_engine.search(&account.id, "quantum-teleportation", 10).unwrap();
        assert_eq!(results.len(), 1, "Should find indexed message by unique keyword");

        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[tokio::test]
    async fn test_smtp_greylisting_deferral_and_pass() {
        let db = Database::new_memory().expect("Failed to create in-memory DB");
        db.init_schema().expect("Failed to init schema");
        let db = Arc::new(db);

        let temp_dir = std::env::temp_dir().join(format!("fastrmail_gl_{}", Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let data_dir = temp_dir.to_string_lossy().to_string();

        // Server with greylisting enabled (bypass_spam_check = false)
        let server = SmtpServer::new(Arc::clone(&db), data_dir.clone());
        let addr = server.start_with_addr("127.0.0.1:0").await.unwrap();

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        // Attempt 1: First time sender -> greylisted with 451
        let stream = TcpStream::connect(addr).await.unwrap();
        let (read_half, mut write_half) = stream.into_split();
        let mut reader = BufReader::new(read_half);

        let _ = read_response(&mut reader).await;
        write_half.write_all(b"EHLO client.local\r\n").await.unwrap();
        let _ = read_multiline_response(&mut reader).await;
        write_half.write_all(b"MAIL FROM:<new_sender@remote.org>\r\n").await.unwrap();
        let _ = read_response(&mut reader).await;
        write_half.write_all(b"RCPT TO:<alice@localhost>\r\n").await.unwrap();

        let rcpt_resp = read_response(&mut reader).await;
        assert!(rcpt_resp.starts_with("451"), "Expected 451 greylisting, got: {rcpt_resp}");
        assert!(rcpt_resp.contains("Greylisting in action"));

        // Seed DB as if 6 minutes have passed
        let six_mins_ago = chrono::Utc::now() - chrono::Duration::minutes(6);
        db.insert_greylist_record("127.0.0.1", "new_sender@remote.org", "alice@localhost", six_mins_ago, false).unwrap();

        // Attempt 2: Retry after window elapsed -> passes with 250 OK
        let stream2 = TcpStream::connect(addr).await.unwrap();
        let (read_half2, mut write_half2) = stream2.into_split();
        let mut reader2 = BufReader::new(read_half2);

        let _ = read_response(&mut reader2).await;
        write_half2.write_all(b"EHLO client.local\r\n").await.unwrap();
        let _ = read_multiline_response(&mut reader2).await;
        write_half2.write_all(b"MAIL FROM:<new_sender@remote.org>\r\n").await.unwrap();
        let _ = read_response(&mut reader2).await;
        write_half2.write_all(b"RCPT TO:<alice@localhost>\r\n").await.unwrap();

        let rcpt_resp2 = read_response(&mut reader2).await;
        assert!(rcpt_resp2.starts_with("250"), "Expected 250 OK after greylist delay, got: {rcpt_resp2}");

        write_half2.write_all(b"QUIT\r\n").await.unwrap();
        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[tokio::test]
    async fn test_smtp_dnsbl_rejection() {
        let db = Database::new_memory().expect("Failed to create in-memory DB");
        db.init_schema().expect("Failed to init schema");
        let db = Arc::new(db);

        let temp_dir = std::env::temp_dir().join(format!("fastrmail_dnsbl_{}", Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let data_dir = temp_dir.to_string_lossy().to_string();

        // Server with mock blocked DNSBL
        let server = SmtpServer::new(Arc::clone(&db), data_dir.clone())
            .with_dnsbl_verifier(Arc::new(DnsblVerifier::mock_blocked()));
        let addr = server.start_with_addr("127.0.0.1:0").await.unwrap();

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let stream = TcpStream::connect(addr).await.unwrap();
        let (read_half, mut write_half) = stream.into_split();
        let mut reader = BufReader::new(read_half);

        let _ = read_response(&mut reader).await;
        write_half.write_all(b"EHLO spammer.local\r\n").await.unwrap();
        let _ = read_multiline_response(&mut reader).await;
        write_half.write_all(b"MAIL FROM:<bad@spammer.org>\r\n").await.unwrap();
        let _ = read_response(&mut reader).await;
        write_half.write_all(b"RCPT TO:<alice@localhost>\r\n").await.unwrap();

        let rcpt_resp = read_response(&mut reader).await;
        assert!(rcpt_resp.starts_with("554"), "Expected 554 DNSBL block, got: {rcpt_resp}");
        assert!(rcpt_resp.contains("blocked using DNSBL"));

        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[tokio::test]
    async fn test_smtp_submission_auth_flow() {
        let db = Database::new_memory().expect("Failed to create in-memory DB");
        db.init_schema().expect("Failed to init schema");
        let tenant_id = db.insert_tenant("submission.test").unwrap();
        db.insert_account(&tenant_id, "alice", "alice@submission.test", "CorrectPassword123!").unwrap();
        let db = Arc::new(db);

        let temp_dir = std::env::temp_dir().join(format!("fastrmail_sub_{}", Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let data_dir = temp_dir.to_string_lossy().to_string();

        let server = SmtpServer::new_submission(Arc::clone(&db), data_dir.clone());
        let addr = server.start_with_addr("127.0.0.1:0").await.unwrap();

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let stream = TcpStream::connect(addr).await.unwrap();
        let (read_half, mut write_half) = stream.into_split();
        let mut reader = BufReader::new(read_half);

        let greeting = read_response(&mut reader).await;
        assert!(greeting.starts_with("220"));

        write_half.write_all(b"EHLO client.test\r\n").await.unwrap();
        let ehlo_resp = read_multiline_response(&mut reader).await;
        assert!(ehlo_resp.iter().any(|l| l.contains("AUTH PLAIN LOGIN")), "EHLO should advertise AUTH PLAIN LOGIN, got: {:?}", ehlo_resp);

        // 1. Attempt MAIL FROM without authentication on submission port -> 530
        write_half.write_all(b"MAIL FROM:<alice@submission.test>\r\n").await.unwrap();
        let unauth_resp = read_response(&mut reader).await;
        assert!(unauth_resp.starts_with("530"), "Expected 530 Auth required, got: {unauth_resp}");

        // 2. Attempt AUTH PLAIN with wrong password -> 535
        use base64::Engine;
        let bad_plain = "\0alice@submission.test\0WrongPassword";
        let bad_b64 = base64::engine::general_purpose::STANDARD.encode(bad_plain);
        write_half.write_all(format!("AUTH PLAIN {bad_b64}\r\n").as_bytes()).await.unwrap();
        let bad_auth_resp = read_response(&mut reader).await;
        assert!(bad_auth_resp.starts_with("535"), "Expected 535 Bad credentials, got: {bad_auth_resp}");

        // 3. Attempt AUTH PLAIN with correct credentials -> 235
        let good_plain = "\0alice@submission.test\0CorrectPassword123!";
        let good_b64 = base64::engine::general_purpose::STANDARD.encode(good_plain);
        write_half.write_all(format!("AUTH PLAIN {good_b64}\r\n").as_bytes()).await.unwrap();
        let good_auth_resp = read_response(&mut reader).await;
        assert!(good_auth_resp.starts_with("235"), "Expected 235 Auth success, got: {good_auth_resp}");

        // 4. Now MAIL FROM should succeed
        write_half.write_all(b"MAIL FROM:<alice@submission.test>\r\n").await.unwrap();
        let mail_resp = read_response(&mut reader).await;
        assert!(mail_resp.starts_with("250"), "Expected 250 OK for MAIL FROM, got: {mail_resp}");

        // 5. RCPT TO
        write_half.write_all(b"RCPT TO:<bob@submission.test>\r\n").await.unwrap();
        let rcpt_resp = read_response(&mut reader).await;
        assert!(rcpt_resp.starts_with("250"), "Expected 250 OK for RCPT TO, got: {rcpt_resp}");

        // 6. DATA
        write_half.write_all(b"DATA\r\n").await.unwrap();
        let data_prompt = read_response(&mut reader).await;
        assert!(data_prompt.starts_with("354"), "Expected 354, got: {data_prompt}");

        write_half.write_all(b"From: alice@submission.test\r\nTo: bob@submission.test\r\nSubject: Submission Test\r\n\r\nHello via port 587!\r\n.\r\n").await.unwrap();
        let data_ok = read_response(&mut reader).await;
        assert!(data_ok.starts_with("250"), "Expected 250 OK for DATA, got: {data_ok}");

        write_half.write_all(b"QUIT\r\n").await.unwrap();
        std::fs::remove_dir_all(&temp_dir).ok();
    }
}
