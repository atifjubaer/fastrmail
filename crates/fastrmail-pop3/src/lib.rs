//! FastrMail POP3 Server — RFC 1939 compliant Post Office Protocol version 3 engine.

use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::{Context, Result};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tracing::{error, info, warn};

use fastrmail_core::{Account, Message};
use fastrmail_store::Database;

#[derive(Debug, PartialEq, Eq)]
enum ConnectionState {
    Authorization,
    Transaction,
    Update,
}

#[derive(Debug, Clone)]
struct Pop3Message {
    message: Message,
    deleted: bool,
}

struct Pop3Session {
    state: ConnectionState,
    user: Option<String>,
    account: Option<Account>,
    mailbox_id: Option<String>,
    messages: Vec<Pop3Message>,
}

impl Pop3Session {
    fn new() -> Self {
        Self {
            state: ConnectionState::Authorization,
            user: None,
            account: None,
            mailbox_id: None,
            messages: Vec::new(),
        }
    }
}

pub struct Pop3Server {
    db: Arc<Database>,
    data_dir: String,
}

impl Pop3Server {
    pub fn new(db: Arc<Database>, data_dir: String) -> Self {
        Self { db, data_dir }
    }

    pub async fn start(&self, addr: &str) -> Result<()> {
        let listener = TcpListener::bind(addr)
            .await
            .with_context(|| format!("Failed to bind POP3 listener on {addr}"))?;

        info!("POP3 listening on {addr}");

        loop {
            match listener.accept().await {
                Ok((stream, peer_addr)) => {
                    info!("POP3 connection from {peer_addr}");
                    let db = Arc::clone(&self.db);
                    let data_dir = self.data_dir.clone();
                    tokio::spawn(async move {
                        if let Err(e) = handle_pop3_connection(stream, peer_addr, db, data_dir).await {
                            error!("POP3 session error from {peer_addr}: {e}");
                        }
                    });
                }
                Err(e) => {
                    error!("Failed to accept POP3 connection: {e}");
                }
            }
        }
    }

    pub async fn start_with_addr(&self, addr: &str) -> Result<SocketAddr> {
        let listener = TcpListener::bind(addr)
            .await
            .with_context(|| format!("Failed to bind POP3 listener on {addr}"))?;

        let local_addr = listener.local_addr()?;
        info!("POP3 listening on {local_addr}");

        let db = Arc::clone(&self.db);
        let data_dir = self.data_dir.clone();

        tokio::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((stream, peer_addr)) => {
                        info!("POP3 connection from {peer_addr}");
                        let db = Arc::clone(&db);
                        let data_dir = data_dir.clone();
                        tokio::spawn(async move {
                            if let Err(e) = handle_pop3_connection(stream, peer_addr, db, data_dir).await {
                                error!("POP3 session error from {peer_addr}: {e}");
                            }
                        });
                    }
                    Err(e) => {
                        error!("Failed to accept POP3 connection: {e}");
                        break;
                    }
                }
            }
        });

        Ok(local_addr)
    }
}

async fn handle_pop3_connection(
    stream: TcpStream,
    _peer_addr: SocketAddr,
    db: Arc<Database>,
    data_dir: String,
) -> Result<()> {
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    let mut session = Pop3Session::new();

    // RFC 1939 Greeting
    writer
        .write_all(b"+OK FastrMail POP3 Server Ready\r\n")
        .await?;

    let mut line_buf = String::new();
    loop {
        line_buf.clear();
        let bytes_read = reader.read_line(&mut line_buf).await?;
        if bytes_read == 0 {
            break;
        }

        let line = line_buf.trim_end();
        if line.is_empty() {
            continue;
        }

        let mut parts = line.splitn(2, ' ');
        let cmd = parts.next().unwrap_or("").to_uppercase();
        let arg = parts.next().unwrap_or("").trim();

        match cmd.as_str() {
            "CAPA" => {
                writer
                    .write_all(b"+OK Capability list follows\r\nUSER\r\nUIDL\r\nTOP\r\nIMPLEMENTATION FastrMail-POP3\r\n.\r\n")
                    .await?;
            }
            "USER" => {
                if session.state != ConnectionState::Authorization {
                    writer.write_all(b"-ERR Unknown command in current state\r\n").await?;
                    continue;
                }
                if arg.is_empty() {
                    writer.write_all(b"-ERR Missing username\r\n").await?;
                    continue;
                }
                session.user = Some(arg.to_string());
                writer.write_all(b"+OK User accepted\r\n").await?;
            }
            "PASS" => {
                if session.state != ConnectionState::Authorization {
                    writer.write_all(b"-ERR Unknown command in current state\r\n").await?;
                    continue;
                }
                let username = match &session.user {
                    Some(u) => u.clone(),
                    None => {
                        writer.write_all(b"-ERR Send USER command first\r\n").await?;
                        continue;
                    }
                };

                match db.verify_login(&username, arg) {
                    Ok(Some(account)) => {
                        // Locate INBOX
                        let mailboxes = db.get_mailboxes(&account.id)?;
                        let inbox = match mailboxes.into_iter().find(|m| m.name.eq_ignore_ascii_case("INBOX")) {
                            Some(ib) => ib,
                            None => {
                                let inbox_id = db.insert_mailbox(&account.id, "INBOX")?;
                                db.get_mailbox_by_id(&inbox_id)?.context("INBOX not found")?
                            }
                        };

                        let msgs = db.get_messages_by_mailbox(&inbox.id)?;
                        session.messages = msgs
                            .into_iter()
                            .map(|m| Pop3Message {
                                message: m,
                                deleted: false,
                            })
                            .collect();

                        let count = session.messages.len();
                        session.mailbox_id = Some(inbox.id);
                        session.account = Some(account);
                        session.state = ConnectionState::Transaction;

                        writer
                            .write_all(format!("+OK Mailbox open, {count} messages\r\n").as_bytes())
                            .await?;
                    }
                    _ => {
                        writer.write_all(b"-ERR Authentication failed\r\n").await?;
                    }
                }
            }
            "STAT" => {
                if session.state != ConnectionState::Transaction {
                    writer.write_all(b"-ERR Not in transaction state\r\n").await?;
                    continue;
                }
                let non_deleted: Vec<&Pop3Message> = session.messages.iter().filter(|m| !m.deleted).collect();
                let count = non_deleted.len();
                let total_size: i64 = non_deleted.iter().map(|m| m.message.size_bytes).sum();
                writer
                    .write_all(format!("+OK {count} {total_size}\r\n").as_bytes())
                    .await?;
            }
            "LIST" => {
                if session.state != ConnectionState::Transaction {
                    writer.write_all(b"-ERR Not in transaction state\r\n").await?;
                    continue;
                }
                if arg.is_empty() {
                    let non_deleted: Vec<(usize, &Pop3Message)> = session
                        .messages
                        .iter()
                        .enumerate()
                        .filter(|(_, m)| !m.deleted)
                        .collect();
                    let count = non_deleted.len();
                    let total_size: i64 = non_deleted.iter().map(|(_, m)| m.message.size_bytes).sum();

                    let mut resp = format!("+OK {count} messages ({total_size} octets)\r\n");
                    for (i, m) in non_deleted {
                        resp.push_str(&format!("{} {}\r\n", i + 1, m.message.size_bytes));
                    }
                    resp.push_str(".\r\n");
                    writer.write_all(resp.as_bytes()).await?;
                } else {
                    let msg_num: usize = match arg.parse() {
                        Ok(n) if n > 0 && n <= session.messages.len() => n,
                        _ => {
                            writer.write_all(b"-ERR No such message\r\n").await?;
                            continue;
                        }
                    };
                    let m = &session.messages[msg_num - 1];
                    if m.deleted {
                        writer.write_all(b"-ERR Message marked as deleted\r\n").await?;
                    } else {
                        writer
                            .write_all(format!("+OK {} {}\r\n", msg_num, m.message.size_bytes).as_bytes())
                            .await?;
                    }
                }
            }
            "UIDL" => {
                if session.state != ConnectionState::Transaction {
                    writer.write_all(b"-ERR Not in transaction state\r\n").await?;
                    continue;
                }
                if arg.is_empty() {
                    let mut resp = String::from("+OK\r\n");
                    for (i, m) in session.messages.iter().enumerate() {
                        if !m.deleted {
                            resp.push_str(&format!("{} {}\r\n", i + 1, m.message.id));
                        }
                    }
                    resp.push_str(".\r\n");
                    writer.write_all(resp.as_bytes()).await?;
                } else {
                    let msg_num: usize = match arg.parse() {
                        Ok(n) if n > 0 && n <= session.messages.len() => n,
                        _ => {
                            writer.write_all(b"-ERR No such message\r\n").await?;
                            continue;
                        }
                    };
                    let m = &session.messages[msg_num - 1];
                    if m.deleted {
                        writer.write_all(b"-ERR Message marked as deleted\r\n").await?;
                    } else {
                        writer
                            .write_all(format!("+OK {} {}\r\n", msg_num, m.message.id).as_bytes())
                            .await?;
                    }
                }
            }
            "RETR" => {
                if session.state != ConnectionState::Transaction {
                    writer.write_all(b"-ERR Not in transaction state\r\n").await?;
                    continue;
                }
                let msg_num: usize = match arg.parse() {
                    Ok(n) if n > 0 && n <= session.messages.len() => n,
                    _ => {
                        writer.write_all(b"-ERR No such message\r\n").await?;
                        continue;
                    }
                };

                let m = &session.messages[msg_num - 1];
                if m.deleted {
                    writer.write_all(b"-ERR Message marked as deleted\r\n").await?;
                    continue;
                }

                let blob_path = format!("{}/blobs/{}.eml", data_dir, m.message.blob_id);
                let content = match std::fs::read(&blob_path) {
                    Ok(b) => b,
                    Err(_) => {
                        writer.write_all(b"-ERR Unable to read message content\r\n").await?;
                        continue;
                    }
                };

                writer
                    .write_all(format!("+OK {} octets\r\n", content.len()).as_bytes())
                    .await?;

                // Dot-stuffing for RFC 1939: lines starting with '.' must be escaped with an extra '.'
                let text = String::from_utf8_lossy(&content);
                for line in text.lines() {
                    if line.starts_with('.') {
                        writer.write_all(b".").await?;
                    }
                    writer.write_all(line.as_bytes()).await?;
                    writer.write_all(b"\r\n").await?;
                }
                writer.write_all(b".\r\n").await?;
            }
            "DELE" => {
                if session.state != ConnectionState::Transaction {
                    writer.write_all(b"-ERR Not in transaction state\r\n").await?;
                    continue;
                }
                let msg_num: usize = match arg.parse() {
                    Ok(n) if n > 0 && n <= session.messages.len() => n,
                    _ => {
                        writer.write_all(b"-ERR No such message\r\n").await?;
                        continue;
                    }
                };

                if session.messages[msg_num - 1].deleted {
                    writer.write_all(b"-ERR Message already deleted\r\n").await?;
                } else {
                    session.messages[msg_num - 1].deleted = true;
                    writer
                        .write_all(format!("+OK Message {msg_num} marked for deletion\r\n").as_bytes())
                        .await?;
                }
            }
            "RSET" => {
                if session.state != ConnectionState::Transaction {
                    writer.write_all(b"-ERR Not in transaction state\r\n").await?;
                    continue;
                }
                for m in &mut session.messages {
                    m.deleted = false;
                }
                writer.write_all(b"+OK Mailbox reset\r\n").await?;
            }
            "NOOP" => {
                writer.write_all(b"+OK\r\n").await?;
            }
            "QUIT" => {
                if session.state == ConnectionState::Transaction {
                    session.state = ConnectionState::Update;
                    for m in &session.messages {
                        if m.deleted {
                            let _ = db.delete_message(&m.message.id);
                            let blob_path = format!("{}/blobs/{}.eml", data_dir, m.message.blob_id);
                            let _ = std::fs::remove_file(&blob_path);
                        }
                    }
                }
                writer.write_all(b"+OK FastrMail POP3 server signing off\r\n").await?;
                break;
            }
            _ => {
                warn!("Unknown POP3 command: {cmd}");
                writer.write_all(b"-ERR Unknown command\r\n").await?;
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use tokio::net::TcpStream;

    async fn read_line(reader: &mut BufReader<tokio::net::tcp::OwnedReadHalf>) -> String {
        let mut buf = String::new();
        reader.read_line(&mut buf).await.unwrap();
        buf
    }

    #[tokio::test]
    async fn test_pop3_full_session_flow() {
        let db = Database::new_memory().expect("Failed to create in-memory DB");
        db.init_schema().expect("Failed to init schema");
        let tenant_id = db.insert_tenant("pop3.test").unwrap();
        let account_id = db
            .insert_account(&tenant_id, "charlie", "charlie@pop3.test", "Pop3SecretPassword!")
            .unwrap();
        let inbox_id = db.insert_mailbox(&account_id, "INBOX").unwrap();

        let temp_dir = std::env::temp_dir().join(format!("fastrmail_pop3_{}", uuid::Uuid::new_v4()));
        let blob_dir = temp_dir.join("blobs");
        std::fs::create_dir_all(&blob_dir).unwrap();
        let data_dir = temp_dir.to_string_lossy().to_string();

        // Seed 2 messages
        let blob1 = "blob-pop3-1";
        let blob2 = "blob-pop3-2";
        let path1 = blob_dir.join(format!("{blob1}.eml"));
        let path2 = blob_dir.join(format!("{blob2}.eml"));
        std::fs::write(&path1, b"From: a@test.com\r\nSubject: Test 1\r\n\r\nHello POP3 1").unwrap();
        std::fs::write(&path2, b"From: b@test.com\r\nSubject: Test 2\r\n\r\nHello POP3 2").unwrap();

        db.insert_message(&inbox_id, &account_id, blob1, 45, Some("Test 1"), Some("a@test.com"), Some("charlie@pop3.test")).unwrap();
        db.insert_message(&inbox_id, &account_id, blob2, 45, Some("Test 2"), Some("b@test.com"), Some("charlie@pop3.test")).unwrap();

        let db = Arc::new(db);
        let server = Pop3Server::new(Arc::clone(&db), data_dir.clone());
        let addr = server.start_with_addr("127.0.0.1:0").await.unwrap();

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let stream = TcpStream::connect(addr).await.unwrap();
        let (read_half, mut write_half) = stream.into_split();
        let mut reader = BufReader::new(read_half);

        // 1. Greeting
        let greeting = read_line(&mut reader).await;
        assert!(greeting.starts_with("+OK"));

        // 2. CAPA
        write_half.write_all(b"CAPA\r\n").await.unwrap();
        let capa_ok = read_line(&mut reader).await;
        assert!(capa_ok.starts_with("+OK"));
        loop {
            let line = read_line(&mut reader).await;
            if line.trim() == "." {
                break;
            }
        }

        // 3. Bad Auth
        write_half.write_all(b"USER charlie@pop3.test\r\n").await.unwrap();
        let user_resp = read_line(&mut reader).await;
        assert!(user_resp.starts_with("+OK"));

        write_half.write_all(b"PASS WrongPass\r\n").await.unwrap();
        let bad_pass = read_line(&mut reader).await;
        assert!(bad_pass.starts_with("-ERR"));

        // 4. Good Auth
        write_half.write_all(b"USER charlie@pop3.test\r\n").await.unwrap();
        let _ = read_line(&mut reader).await;
        write_half.write_all(b"PASS Pop3SecretPassword!\r\n").await.unwrap();
        let good_pass = read_line(&mut reader).await;
        assert!(good_pass.starts_with("+OK"));
        assert!(good_pass.contains("2 messages"), "got good_pass = {:?}", good_pass);

        // 5. STAT
        write_half.write_all(b"STAT\r\n").await.unwrap();
        let stat_resp = read_line(&mut reader).await;
        assert!(stat_resp.starts_with("+OK 2 90"), "Got STAT: {stat_resp}");

        // 6. LIST
        write_half.write_all(b"LIST\r\n").await.unwrap();
        let list_resp = read_line(&mut reader).await;
        assert!(list_resp.starts_with("+OK"));
        let l1 = read_line(&mut reader).await;
        assert_eq!(l1.trim(), "1 45");
        let l2 = read_line(&mut reader).await;
        assert_eq!(l2.trim(), "2 45");
        let dot = read_line(&mut reader).await;
        assert_eq!(dot.trim(), ".");

        // 7. RETR 1
        write_half.write_all(b"RETR 1\r\n").await.unwrap();
        let retr_resp = read_line(&mut reader).await;
        assert!(retr_resp.starts_with("+OK"));
        let mut email_body = String::new();
        loop {
            let line = read_line(&mut reader).await;
            if line.trim() == "." {
                break;
            }
            email_body.push_str(&line);
        }
        assert!(email_body.contains("Subject: Test 1"));

        // 8. DELE 1
        write_half.write_all(b"DELE 1\r\n").await.unwrap();
        let dele_resp = read_line(&mut reader).await;
        assert!(dele_resp.starts_with("+OK"));

        // STAT after DELE should report 1 message
        write_half.write_all(b"STAT\r\n").await.unwrap();
        let stat_after = read_line(&mut reader).await;
        assert!(stat_after.starts_with("+OK 1 45"));

        // 9. QUIT (Expunges message 1)
        write_half.write_all(b"QUIT\r\n").await.unwrap();
        let quit_resp = read_line(&mut reader).await;
        assert!(quit_resp.starts_with("+OK"));

        // Verify remaining messages in database
        let msgs_remaining = db.get_messages_by_mailbox(&inbox_id).unwrap();
        assert_eq!(msgs_remaining.len(), 1);
        assert_eq!(msgs_remaining[0].blob_id, blob2);

        std::fs::remove_dir_all(&temp_dir).ok();
    }
}
