//! FastrMail IMAP — Production-grade RFC 9051 / RFC 3501 IMAP4rev2 Server.
//!
//! Provides a full-featured IMAP listener with session state machine:
//! Unauthenticated -> Authenticated -> Selected -> Logout.

use std::collections::HashSet;
use std::sync::Arc;

use anyhow::{Context, Result};
use mail_parser::MessageParser;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use fastrmail_core::{Account, Mailbox, Message};
use fastrmail_store::Database;

/// IMAP connection state according to RFC 9051 / RFC 3501.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    Unauthenticated,
    Authenticated,
    Selected,
    Logout,
}

/// Active IMAP client session tracking authentication and selected mailbox.
pub struct Session {
    pub state: ConnectionState,
    pub account: Option<Account>,
    pub selected_mailbox: Option<Mailbox>,
}

impl Session {
    pub fn new() -> Self {
        Self {
            state: ConnectionState::Unauthenticated,
            account: None,
            selected_mailbox: None,
        }
    }
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}

/// Production IMAP4rev2 server instance.
pub struct ImapServer {
    db: Arc<Database>,
    data_dir: String,
}

impl ImapServer {
    /// Create a new IMAP server with a database handle and blob storage directory.
    pub fn new(db: Arc<Database>, data_dir: String) -> Self {
        Self { db, data_dir }
    }

    /// Bind the IMAP TCP listener and serve incoming connections indefinitely.
    pub async fn start(&self, addr: &str) -> Result<()> {
        let listener = TcpListener::bind(addr)
            .await
            .with_context(|| format!("Failed to bind IMAP server on {addr}"))?;
        info!("IMAP server listening on {addr}");

        loop {
            match listener.accept().await {
                Ok((stream, peer_addr)) => {
                    debug!("Accepted IMAP connection from {peer_addr}");
                    let db = Arc::clone(&self.db);
                    let data_dir = self.data_dir.clone();
                    tokio::spawn(async move {
                        if let Err(e) = handle_imap_connection(stream, db, data_dir).await {
                            warn!("IMAP connection ended with error: {e}");
                        }
                    });
                }
                Err(e) => {
                    error!("Error accepting IMAP connection: {e}");
                }
            }
        }
    }
}

/// Tokenize an IMAP command line respecting quoted strings, parenthesis lists, and literal tokens.
pub fn tokenize_imap_line(line: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut chars = line.chars().peekable();

    while let Some(&ch) = chars.peek() {
        if ch.is_whitespace() {
            chars.next();
            continue;
        }

        if ch == '"' {
            // Quoted string
            chars.next();
            let mut s = String::new();
            while let Some(c) = chars.next() {
                if c == '\\' {
                    if let Some(escaped) = chars.next() {
                        s.push(escaped);
                    }
                } else if c == '"' {
                    break;
                } else {
                    s.push(c);
                }
            }
            tokens.push(s);
        } else if ch == '(' {
            // Parenthesized list
            chars.next();
            let mut depth = 1;
            let mut s = String::new();
            while let Some(c) = chars.next() {
                if c == '(' {
                    depth += 1;
                    s.push(c);
                } else if c == ')' {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                    s.push(c);
                } else {
                    s.push(c);
                }
            }
            tokens.push(s);
        } else {
            // Atom or literal marker like {123}
            let mut s = String::new();
            while let Some(&c) = chars.peek() {
                if c.is_whitespace() || c == '(' || c == ')' || c == '"' {
                    break;
                }
                s.push(c);
                chars.next();
            }
            tokens.push(s);
        }
    }

    tokens
}

/// Parse a sequence set (e.g., "1:*", "1", "1:3", "1,2,5") into sorted, deduplicated sequence numbers or UIDs.
pub fn parse_sequence_set(set_str: &str, max_val: i64) -> Vec<i64> {
    let mut result = HashSet::new();

    for part in set_str.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }

        if let Some((start_s, end_s)) = part.split_once(':') {
            let start = if start_s == "*" {
                max_val
            } else {
                start_s.parse::<i64>().unwrap_or(1)
            };
            let end = if end_s == "*" {
                max_val
            } else {
                end_s.parse::<i64>().unwrap_or(max_val)
            };

            let (low, high) = if start <= end {
                (start, end)
            } else {
                (end, start)
            };
            let low = low.max(1);
            let high = high.min(max_val);

            for val in low..=high {
                result.insert(val);
            }
        } else if part == "*" {
            if max_val >= 1 {
                result.insert(max_val);
            }
        } else if let Ok(val) = part.parse::<i64>() {
            if val >= 1 && (max_val == 0 || val <= max_val) {
                result.insert(val);
            }
        }
    }

    let mut vec: Vec<i64> = result.into_iter().collect();
    vec.sort_unstable();
    vec
}

/// Convert JSON flags string (e.g. `["\\Seen", "\\Flagged"]`) to space-separated IMAP flags (e.g. `\Seen \Flagged`).
pub fn format_flags_for_imap(json_flags: &str) -> String {
    if let Ok(flags) = serde_json::from_str::<Vec<String>>(json_flags) {
        flags.join(" ")
    } else {
        String::new()
    }
}

/// Parse a space-separated string of flags into a JSON array string.
pub fn flags_to_json(flags: &[String]) -> String {
    let formatted: Vec<String> = flags
        .iter()
        .map(|f| {
            if f.starts_with('\\') {
                f.clone()
            } else {
                format!("\\{f}")
            }
        })
        .collect();
    serde_json::to_string(&formatted).unwrap_or_else(|_| "[]".to_string())
}

/// Build an IMAP envelope structure string for a message.
pub fn build_envelope(msg: &Message) -> String {
    let date_str = msg.internal_date.format("%a, %d %b %Y %H:%M:%S +0000").to_string();
    let subject = msg.parsed_subject.as_deref().unwrap_or("No Subject");

    let from_addr = msg.parsed_from.as_deref().unwrap_or("unknown@localhost");
    let (from_name, from_host) = match from_addr.split_once('@') {
        Some((u, h)) => (u, h),
        None => (from_addr, "localhost"),
    };

    let to_addr = msg.parsed_to.as_deref().unwrap_or("unknown@localhost");
    let (to_name, to_host) = match to_addr.split_once('@') {
        Some((u, h)) => (u, h),
        None => (to_addr, "localhost"),
    };

    format!(
        "(\"{date_str}\" \"{subject}\" ((\"{from_name}\" NIL \"{from_name}\" \"{from_host}\")) ((\"{from_name}\" NIL \"{from_name}\" \"{from_host}\")) ((\"{from_name}\" NIL \"{from_name}\" \"{from_host}\")) ((\"{to_name}\" NIL \"{to_name}\" \"{to_host}\")) NIL NIL NIL \"<{}>@fastrmail\")",
        msg.id
    )
}

/// Handle a single IMAP TCP client connection.
pub async fn handle_imap_connection(
    stream: TcpStream,
    db: Arc<Database>,
    data_dir: String,
) -> Result<()> {
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    let mut session = Session::new();

    // Send initial IMAP banner
    writer
        .write_all(b"* OK FastrMail IMAP4rev2 Ready\r\n")
        .await?;
    writer.flush().await?;

    let mut line = String::new();

    while session.state != ConnectionState::Logout {
        line.clear();
        let bytes_read = reader.read_line(&mut line).await?;
        if bytes_read == 0 {
            // Client closed connection
            break;
        }

        let trimmed = line.trim_end_matches(['\r', '\n']);
        if trimmed.is_empty() {
            continue;
        }

        let tokens = tokenize_imap_line(trimmed);
        if tokens.is_empty() {
            continue;
        }

        let tag = &tokens[0];
        if tokens.len() < 2 {
            writer
                .write_all(format!("{tag} BAD Missing command\r\n").as_bytes())
                .await?;
            writer.flush().await?;
            continue;
        }

        let command = tokens[1].to_uppercase();
        let args = &tokens[2..];

        match command.as_str() {
            "CAPABILITY" => {
                writer
                    .write_all(b"* CAPABILITY IMAP4rev2 IMAP4rev1 AUTH=PLAIN SASL-IR\r\n")
                    .await?;
                writer
                    .write_all(format!("{tag} OK CAPABILITY completed\r\n").as_bytes())
                    .await?;
                writer.flush().await?;
            }

            "NOOP" => {
                writer
                    .write_all(format!("{tag} OK NOOP completed\r\n").as_bytes())
                    .await?;
                writer.flush().await?;
            }

            "LOGOUT" => {
                writer
                    .write_all(b"* BYE FastrMail IMAP4rev2 Server logging out\r\n")
                    .await?;
                writer
                    .write_all(format!("{tag} OK LOGOUT completed\r\n").as_bytes())
                    .await?;
                writer.flush().await?;
                session.state = ConnectionState::Logout;
                break;
            }

            "LOGIN" => {
                if session.state != ConnectionState::Unauthenticated {
                    writer
                        .write_all(format!("{tag} BAD Already authenticated\r\n").as_bytes())
                        .await?;
                    writer.flush().await?;
                    continue;
                }

                if args.len() < 2 {
                    writer
                        .write_all(format!("{tag} BAD LOGIN requires username and password\r\n").as_bytes())
                        .await?;
                    writer.flush().await?;
                    continue;
                }

                let username = &args[0];
                let password = &args[1];

                // Attempt authentication via verify_login
                match db.verify_login(username, password) {
                    Ok(Some(account)) => {
                        // Ensure standard INBOX mailbox exists
                        if db.get_mailbox_by_name(&account.id, "INBOX")?.is_none() {
                            db.insert_mailbox(&account.id, "INBOX")?;
                        }

                        session.account = Some(account);
                        session.state = ConnectionState::Authenticated;
                        writer
                            .write_all(format!("{tag} OK LOGIN completed\r\n").as_bytes())
                            .await?;
                        writer.flush().await?;
                    }
                    Ok(None) => {
                        writer
                            .write_all(format!("{tag} NO [AUTHENTICATIONFAILED] Invalid credentials\r\n").as_bytes())
                            .await?;
                        writer.flush().await?;
                    }
                    Err(e) => {
                        writer
                            .write_all(format!("{tag} NO Authentication error: {e}\r\n").as_bytes())
                            .await?;
                        writer.flush().await?;
                    }
                }
            }

            "AUTHENTICATE" => {
                if session.state != ConnectionState::Unauthenticated {
                    writer
                        .write_all(format!("{tag} BAD Already authenticated\r\n").as_bytes())
                        .await?;
                    writer.flush().await?;
                    continue;
                }

                if args.is_empty() {
                    writer
                        .write_all(format!("{tag} BAD Missing SASL mechanism\r\n").as_bytes())
                        .await?;
                    writer.flush().await?;
                    continue;
                }

                let mechanism = args[0].to_uppercase();
                if mechanism == "PLAIN" {
                    let mut auth_data = if args.len() > 1 {
                        args[1].clone()
                    } else {
                        // Send continuation
                        writer.write_all(b"+\r\n").await?;
                        writer.flush().await?;
                        let mut client_resp = String::new();
                        reader.read_line(&mut client_resp).await?;
                        client_resp.trim().to_string()
                    };

                    // Decode base64: authzid\0authcid\0password
                    auth_data.retain(|c| !c.is_whitespace());
                    use base64::Engine;
                    let decoded = base64::engine::general_purpose::STANDARD.decode(auth_data);

                    let mut authenticated = false;
                    if let Ok(bytes) = decoded {
                        let parts: Vec<&[u8]> = bytes.split(|&b| b == 0).collect();
                        if parts.len() >= 3 {
                            let user = String::from_utf8_lossy(parts[1]);
                            let pass = String::from_utf8_lossy(parts[2]);
                            if let Ok(Some(account)) = db.verify_login(&user, &pass) {
                                if db.get_mailbox_by_name(&account.id, "INBOX")?.is_none() {
                                    db.insert_mailbox(&account.id, "INBOX")?;
                                }
                                session.account = Some(account);
                                session.state = ConnectionState::Authenticated;
                                authenticated = true;
                            }
                        }
                    }

                    if authenticated {
                        writer
                            .write_all(format!("{tag} OK AUTHENTICATE completed\r\n").as_bytes())
                            .await?;
                    } else {
                        writer
                            .write_all(format!("{tag} NO [AUTHENTICATIONFAILED] Authentication failed\r\n").as_bytes())
                            .await?;
                    }
                    writer.flush().await?;
                } else {
                    writer
                        .write_all(format!("{tag} NO Unsupported authentication mechanism\r\n").as_bytes())
                        .await?;
                    writer.flush().await?;
                }
            }

            "LIST" | "LSUB" => {
                let account = match &session.account {
                    Some(a) => a,
                    None => {
                        writer
                            .write_all(format!("{tag} BAD Please login first\r\n").as_bytes())
                            .await?;
                        writer.flush().await?;
                        continue;
                    }
                };

                let mailboxes = db.get_mailboxes(&account.id)?;
                for mb in mailboxes {
                    let resp = format!("* {} () \"/\" \"{}\"\r\n", command, mb.name);
                    writer.write_all(resp.as_bytes()).await?;
                }
                writer
                    .write_all(format!("{tag} OK {command} completed\r\n").as_bytes())
                    .await?;
                writer.flush().await?;
            }

            "SELECT" | "EXAMINE" => {
                let account = match &session.account {
                    Some(a) => a,
                    None => {
                        writer
                            .write_all(format!("{tag} BAD Please login first\r\n").as_bytes())
                            .await?;
                        writer.flush().await?;
                        continue;
                    }
                };

                if args.is_empty() {
                    writer
                        .write_all(format!("{tag} BAD Missing mailbox name\r\n").as_bytes())
                        .await?;
                    writer.flush().await?;
                    continue;
                }

                let mailbox_name = &args[0];
                let mb = match db.get_mailbox_by_name(&account.id, mailbox_name)? {
                    Some(m) => m,
                    None => {
                        if mailbox_name.eq_ignore_ascii_case("INBOX") {
                            let id = db.insert_mailbox(&account.id, "INBOX")?;
                            db.get_mailbox_by_id(&id)?.unwrap()
                        } else {
                            writer
                                .write_all(format!("{tag} NO [NONEXISTENT] Mailbox does not exist\r\n").as_bytes())
                                .await?;
                            writer.flush().await?;
                            continue;
                        }
                    }
                };

                let (total, unseen) = db.get_mailbox_counts(&mb.id)?;

                writer
                    .write_all(format!("* {total} EXISTS\r\n").as_bytes())
                    .await?;
                writer.write_all(b"* 0 RECENT\r\n").await?;
                writer
                    .write_all(format!("* OK [UNSEEN {unseen}] Message unseen\r\n").as_bytes())
                    .await?;
                writer
                    .write_all(format!("* OK [UIDVALIDITY {}] UIDs valid\r\n", mb.uid_validity).as_bytes())
                    .await?;
                writer
                    .write_all(format!("* OK [UIDNEXT {}] Predicted next UID\r\n", mb.uid_next).as_bytes())
                    .await?;
                writer
                    .write_all(b"* FLAGS (\\Answered \\Flagged \\Deleted \\Seen \\Draft)\r\n")
                    .await?;
                writer
                    .write_all(b"* OK [PERMANENTFLAGS (\\Answered \\Flagged \\Deleted \\Seen \\Draft \\*)] Limited\r\n")
                    .await?;

                let read_mode = if command == "SELECT" {
                    session.state = ConnectionState::Selected;
                    "[READ-WRITE]"
                } else {
                    "[READ-ONLY]"
                };

                session.selected_mailbox = Some(mb);

                writer
                    .write_all(format!("{tag} OK {read_mode} {command} completed\r\n").as_bytes())
                    .await?;
                writer.flush().await?;
            }

            "STATUS" => {
                let account = match &session.account {
                    Some(a) => a,
                    None => {
                        writer
                            .write_all(format!("{tag} BAD Please login first\r\n").as_bytes())
                            .await?;
                        writer.flush().await?;
                        continue;
                    }
                };

                if args.is_empty() {
                    writer
                        .write_all(format!("{tag} BAD Missing mailbox name\r\n").as_bytes())
                        .await?;
                    writer.flush().await?;
                    continue;
                }

                let mailbox_name = &args[0];
                let mb = match db.get_mailbox_by_name(&account.id, mailbox_name)? {
                    Some(m) => m,
                    None => {
                        writer
                            .write_all(format!("{tag} NO [NONEXISTENT] Mailbox does not exist\r\n").as_bytes())
                            .await?;
                        writer.flush().await?;
                        continue;
                    }
                };

                let (total, unseen) = db.get_mailbox_counts(&mb.id)?;

                // Status items requested in parenthesis e.g. "(MESSAGES UIDNEXT UIDVALIDITY UNSEEN)"
                let items_req = if args.len() > 1 {
                    args[1].to_uppercase()
                } else {
                    "MESSAGES UIDNEXT UIDVALIDITY UNSEEN".to_string()
                };

                let mut status_parts = Vec::new();
                if items_req.contains("MESSAGES") {
                    status_parts.push(format!("MESSAGES {total}"));
                }
                if items_req.contains("RECENT") {
                    status_parts.push("RECENT 0".to_string());
                }
                if items_req.contains("UIDNEXT") {
                    status_parts.push(format!("UIDNEXT {}", mb.uid_next));
                }
                if items_req.contains("UIDVALIDITY") {
                    status_parts.push(format!("UIDVALIDITY {}", mb.uid_validity));
                }
                if items_req.contains("UNSEEN") {
                    status_parts.push(format!("UNSEEN {unseen}"));
                }

                writer
                    .write_all(format!("* STATUS \"{}\" ({})\r\n", mb.name, status_parts.join(" ")).as_bytes())
                    .await?;
                writer
                    .write_all(format!("{tag} OK STATUS completed\r\n").as_bytes())
                    .await?;
                writer.flush().await?;
            }

            "CREATE" => {
                let account = match &session.account {
                    Some(a) => a,
                    None => {
                        writer
                            .write_all(format!("{tag} BAD Please login first\r\n").as_bytes())
                            .await?;
                        writer.flush().await?;
                        continue;
                    }
                };

                if args.is_empty() {
                    writer
                        .write_all(format!("{tag} BAD Missing folder name\r\n").as_bytes())
                        .await?;
                    writer.flush().await?;
                    continue;
                }

                let name = &args[0];
                if db.get_mailbox_by_name(&account.id, name)?.is_some() {
                    writer
                        .write_all(format!("{tag} NO [ALREADYEXISTS] Mailbox already exists\r\n").as_bytes())
                        .await?;
                } else {
                    db.insert_mailbox(&account.id, name)?;
                    writer
                        .write_all(format!("{tag} OK CREATE completed\r\n").as_bytes())
                        .await?;
                }
                writer.flush().await?;
            }

            "DELETE" => {
                let account = match &session.account {
                    Some(a) => a,
                    None => {
                        writer
                            .write_all(format!("{tag} BAD Please login first\r\n").as_bytes())
                            .await?;
                        writer.flush().await?;
                        continue;
                    }
                };

                if args.is_empty() {
                    writer
                        .write_all(format!("{tag} BAD Missing folder name\r\n").as_bytes())
                        .await?;
                    writer.flush().await?;
                    continue;
                }

                let name = &args[0];
                if name.eq_ignore_ascii_case("INBOX") {
                    writer
                        .write_all(format!("{tag} NO Cannot delete INBOX\r\n").as_bytes())
                        .await?;
                } else if let Some(mb) = db.get_mailbox_by_name(&account.id, name)? {
                    db.delete_mailbox(&mb.id)?;
                    writer
                        .write_all(format!("{tag} OK DELETE completed\r\n").as_bytes())
                        .await?;
                } else {
                    writer
                        .write_all(format!("{tag} NO [NONEXISTENT] Mailbox not found\r\n").as_bytes())
                        .await?;
                }
                writer.flush().await?;
            }

            "RENAME" => {
                let account = match &session.account {
                    Some(a) => a,
                    None => {
                        writer
                            .write_all(format!("{tag} BAD Please login first\r\n").as_bytes())
                            .await?;
                        writer.flush().await?;
                        continue;
                    }
                };

                if args.len() < 2 {
                    writer
                        .write_all(format!("{tag} BAD RENAME requires old and new names\r\n").as_bytes())
                        .await?;
                    writer.flush().await?;
                    continue;
                }

                let old_name = &args[0];
                let new_name = &args[1];

                if let Some(mb) = db.get_mailbox_by_name(&account.id, old_name)? {
                    db.rename_mailbox(&mb.id, new_name)?;
                    writer
                        .write_all(format!("{tag} OK RENAME completed\r\n").as_bytes())
                        .await?;
                } else {
                    writer
                        .write_all(format!("{tag} NO Mailbox not found\r\n").as_bytes())
                        .await?;
                }
                writer.flush().await?;
            }

            "CLOSE" => {
                let mb = match &session.selected_mailbox {
                    Some(m) => m,
                    None => {
                        writer
                            .write_all(format!("{tag} BAD No mailbox selected\r\n").as_bytes())
                            .await?;
                        writer.flush().await?;
                        continue;
                    }
                };

                // Expunge deleted messages and clean up blobs
                let expunged = db.expunge_deleted_messages(&mb.id)?;
                for (_, blob_id) in expunged {
                    let blob_path = format!("{data_dir}/blobs/{blob_id}.eml");
                    let _ = std::fs::remove_file(blob_path);
                }

                session.selected_mailbox = None;
                session.state = ConnectionState::Authenticated;
                writer
                    .write_all(format!("{tag} OK CLOSE completed\r\n").as_bytes())
                    .await?;
                writer.flush().await?;
            }

            "EXPUNGE" => {
                let mb = match &session.selected_mailbox {
                    Some(m) => m,
                    None => {
                        writer
                            .write_all(format!("{tag} BAD No mailbox selected\r\n").as_bytes())
                            .await?;
                        writer.flush().await?;
                        continue;
                    }
                };

                let all_messages = db.get_messages_by_mailbox(&mb.id)?;
                let mut expunged_seqs = Vec::new();

                for (idx, msg) in all_messages.iter().enumerate() {
                    if msg.flags.contains("Deleted") {
                        expunged_seqs.push((idx + 1, msg.id.clone(), msg.blob_id.clone()));
                    }
                }

                let expunged = db.expunge_deleted_messages(&mb.id)?;
                for (_, blob_id) in expunged {
                    let blob_path = format!("{data_dir}/blobs/{blob_id}.eml");
                    let _ = std::fs::remove_file(blob_path);
                }

                // Send untagged response for each expunged sequence
                for (seq, _, _) in expunged_seqs {
                    writer
                        .write_all(format!("* {seq} EXPUNGE\r\n").as_bytes())
                        .await?;
                }

                writer
                    .write_all(format!("{tag} OK EXPUNGE completed\r\n").as_bytes())
                    .await?;
                writer.flush().await?;
            }

            "CHECK" => {
                writer
                    .write_all(format!("{tag} OK CHECK completed\r\n").as_bytes())
                    .await?;
                writer.flush().await?;
            }

            "FETCH" | "UID" => {
                let is_uid = command == "UID";
                let (seq_set_str, fetch_items_str) = if is_uid {
                    if args.len() < 3 || args[0].to_uppercase() != "FETCH" {
                        // Might be UID STORE or unsupported UID command
                        if args.len() >= 3 && args[0].to_uppercase() == "STORE" {
                            // Forward to STORE logic
                            handle_store_command(
                                tag,
                                &args[1..],
                                true,
                                &session,
                                &db,
                                &mut writer,
                            )
                            .await?;
                            continue;
                        } else {
                            writer
                                .write_all(format!("{tag} BAD Invalid UID command syntax\r\n").as_bytes())
                                .await?;
                            writer.flush().await?;
                            continue;
                        }
                    } else {
                        (&args[1], &args[2])
                    }
                } else {
                    if args.len() < 2 {
                        writer
                            .write_all(format!("{tag} BAD FETCH requires sequence set and items\r\n").as_bytes())
                            .await?;
                        writer.flush().await?;
                        continue;
                    }
                    (&args[0], &args[1])
                };

                let mb = match &session.selected_mailbox {
                    Some(m) => m,
                    None => {
                        writer
                            .write_all(format!("{tag} BAD No mailbox selected\r\n").as_bytes())
                            .await?;
                        writer.flush().await?;
                        continue;
                    }
                };

                let all_messages = db.get_messages_by_mailbox(&mb.id)?;
                let total_messages = all_messages.len() as i64;
                let max_uid = all_messages.iter().map(|m| m.uid).max().unwrap_or(0);

                let target_set = if is_uid {
                    parse_sequence_set(seq_set_str, max_uid)
                } else {
                    parse_sequence_set(seq_set_str, total_messages)
                };

                let uppercase_items = fetch_items_str.to_uppercase();
                let item_words: Vec<&str> = fetch_items_str.split_whitespace().collect();
                let wants_body = item_words.iter().any(|item| {
                    let u = item.to_uppercase();
                    u == "BODY[]" || u == "BODY.PEEK[]" || u == "RFC822" || u == "FULL"
                });

                for (idx, msg) in all_messages.iter().enumerate() {
                    let seq = (idx + 1) as i64;
                    let matches = if is_uid {
                        target_set.contains(&msg.uid)
                    } else {
                        target_set.contains(&seq)
                    };

                    if !matches {
                        continue;
                    }

                    // Build FETCH response parts
                    let mut parts = Vec::new();
                    parts.push(format!("UID {}", msg.uid));

                    let flags_str = format_flags_for_imap(&msg.flags);
                    parts.push(format!("FLAGS ({flags_str})"));

                    let internal_date = msg.internal_date.format("%d-%b-%Y %H:%M:%S +0000").to_string();
                    parts.push(format!("INTERNALDATE \"{internal_date}\""));

                    parts.push(format!("RFC822.SIZE {}", msg.size_bytes));

                    if uppercase_items.contains("ENVELOPE")
                        || uppercase_items.contains("ALL")
                        || uppercase_items.contains("FULL")
                    {
                        parts.push(format!("ENVELOPE {}", build_envelope(msg)));
                    }

                    if wants_body {
                        // Read message raw blob from disk
                        let blob_path = format!("{data_dir}/blobs/{}.eml", msg.blob_id);
                        let body_bytes = std::fs::read(&blob_path).unwrap_or_default();
                        let body_len = body_bytes.len();

                        // Write untagged header + literal
                        let line_header = format!("* {seq} FETCH ({} BODY[] {{{body_len}}}\r\n", parts.join(" "));
                        writer.write_all(line_header.as_bytes()).await?;
                        writer.write_all(&body_bytes).await?;
                        writer.write_all(b")\r\n").await?;
                    } else {
                        let line = format!("* {seq} FETCH ({})\r\n", parts.join(" "));
                        writer.write_all(line.as_bytes()).await?;
                    }
                }

                writer
                    .write_all(format!("{tag} OK FETCH completed\r\n").as_bytes())
                    .await?;
                writer.flush().await?;
            }

            "STORE" => {
                handle_store_command(tag, args, false, &session, &db, &mut writer).await?;
            }

            "APPEND" => {
                let account = match &session.account {
                    Some(a) => a,
                    None => {
                        writer
                            .write_all(format!("{tag} BAD Please login first\r\n").as_bytes())
                            .await?;
                        writer.flush().await?;
                        continue;
                    }
                };

                if args.is_empty() {
                    writer
                        .write_all(format!("{tag} BAD Missing APPEND mailbox argument\r\n").as_bytes())
                        .await?;
                    writer.flush().await?;
                    continue;
                }

                let mailbox_name = &args[0];
                let mb = match db.get_mailbox_by_name(&account.id, mailbox_name)? {
                    Some(m) => m,
                    None => {
                        let id = db.insert_mailbox(&account.id, mailbox_name)?;
                        db.get_mailbox_by_id(&id)?.unwrap()
                    }
                };

                // Find literal size: either last token is "{size}" or second token is
                let last_token = args.last().unwrap();
                let literal_size = if last_token.starts_with('{') && last_token.ends_with('}') {
                    let num_str = &last_token[1..last_token.len() - 1];
                    num_str.trim_end_matches('+').parse::<usize>().unwrap_or(0)
                } else {
                    0
                };

                // Send continuation
                writer.write_all(b"+ Ready for literal data\r\n").await?;
                writer.flush().await?;

                // Read literal_size bytes
                let mut email_data = vec![0u8; literal_size];
                reader.read_exact(&mut email_data).await?;

                // Read remaining CRLF of the APPEND command
                let mut trailing_line = String::new();
                reader.read_line(&mut trailing_line).await?;

                // Save blob to disk
                let blob_id = Uuid::new_v4().to_string();
                let blob_dir = format!("{data_dir}/blobs");
                std::fs::create_dir_all(&blob_dir)?;
                let blob_path = format!("{blob_dir}/{blob_id}.eml");
                std::fs::write(&blob_path, &email_data)?;

                // Parse metadata
                let parsed = MessageParser::default().parse(&email_data);
                let subject = parsed.as_ref().and_then(|p| p.subject());
                let from = parsed
                    .as_ref()
                    .and_then(|p| p.from())
                    .and_then(|f| f.first())
                    .and_then(|a| a.address());
                let to = parsed
                    .as_ref()
                    .and_then(|p| p.to())
                    .and_then(|t| t.first())
                    .and_then(|a| a.address());

                let msg_id = db.insert_message(
                    &mb.id,
                    &account.id,
                    &blob_id,
                    literal_size as i64,
                    subject,
                    from,
                    to,
                )?;

                // Retrieve assigned UID
                let assigned_uid = db
                    .get_messages_by_mailbox(&mb.id)?
                    .into_iter()
                    .find(|m| m.id == msg_id)
                    .map(|m| m.uid)
                    .unwrap_or(mb.uid_next);

                writer
                    .write_all(
                        format!(
                            "{tag} OK [APPENDUID {} {assigned_uid}] APPEND completed\r\n",
                            mb.uid_validity
                        )
                        .as_bytes(),
                    )
                    .await?;
                writer.flush().await?;
            }

            _ => {
                writer
                    .write_all(format!("{tag} BAD Command not recognized or invalid syntax\r\n").as_bytes())
                    .await?;
                writer.flush().await?;
            }
        }
    }

    Ok(())
}

/// Helper function to execute STORE / UID STORE commands.
async fn handle_store_command<W: AsyncWriteExt + Unpin>(
    tag: &str,
    args: &[String],
    is_uid: bool,
    session: &Session,
    db: &Database,
    writer: &mut W,
) -> Result<()> {
    if args.len() < 3 {
        writer
            .write_all(format!("{tag} BAD STORE requires sequence set, item, and flags\r\n").as_bytes())
            .await?;
        writer.flush().await?;
        return Ok(());
    }

    let mb = match &session.selected_mailbox {
        Some(m) => m,
        None => {
            writer
                .write_all(format!("{tag} BAD No mailbox selected\r\n").as_bytes())
                .await?;
            writer.flush().await?;
            return Ok(());
        }
    };

    let seq_set_str = &args[0];
    let store_item = args[1].to_uppercase();
    let flags_str = &args[2];

    let all_messages = db.get_messages_by_mailbox(&mb.id)?;
    let total_messages = all_messages.len() as i64;
    let max_uid = all_messages.iter().map(|m| m.uid).max().unwrap_or(0);

    let target_set = if is_uid {
        parse_sequence_set(seq_set_str, max_uid)
    } else {
        parse_sequence_set(seq_set_str, total_messages)
    };

    // Extract target flags
    let raw_flags: Vec<String> = flags_str
        .trim_matches(['(', ')'])
        .split_whitespace()
        .map(|s| s.to_string())
        .collect();

    let is_silent = store_item.contains(".SILENT");
    let is_add = store_item.starts_with("+FLAGS");
    let is_remove = store_item.starts_with("-FLAGS");

    for (idx, msg) in all_messages.iter().enumerate() {
        let seq = (idx + 1) as i64;
        let matches = if is_uid {
            target_set.contains(&msg.uid)
        } else {
            target_set.contains(&seq)
        };

        if !matches {
            continue;
        }

        let mut existing_flags: HashSet<String> =
            serde_json::from_str(&msg.flags).unwrap_or_default();

        if is_add {
            for f in &raw_flags {
                let formatted = if f.starts_with('\\') {
                    f.clone()
                } else {
                    format!("\\{f}")
                };
                existing_flags.insert(formatted);
            }
        } else if is_remove {
            for f in &raw_flags {
                let formatted = if f.starts_with('\\') {
                    f.clone()
                } else {
                    format!("\\{f}")
                };
                existing_flags.remove(&formatted);
            }
        } else {
            // Replace
            existing_flags.clear();
            for f in &raw_flags {
                let formatted = if f.starts_with('\\') {
                    f.clone()
                } else {
                    format!("\\{f}")
                };
                existing_flags.insert(formatted);
            }
        }

        let flags_vec: Vec<String> = existing_flags.into_iter().collect();
        let new_flags_json = serde_json::to_string(&flags_vec)?;
        db.update_message_flags(&mb.id, msg.uid, &new_flags_json)?;

        if !is_silent {
            let imap_flags = flags_vec.join(" ");
            writer
                .write_all(format!("* {seq} FETCH (FLAGS ({imap_flags}))\r\n").as_bytes())
                .await?;
        }
    }

    writer
        .write_all(format!("{tag} OK STORE completed\r\n").as_bytes())
        .await?;
    writer.flush().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_test_db() -> Arc<Database> {
        let db = Database::new_memory().expect("Failed to create in-memory db");
        db.init_schema().expect("Failed to init schema");
        Arc::new(db)
    }

    #[test]
    fn test_tokenize_imap_line() {
        let line = r#"A01 LOGIN "user@test.com" "secret pass""#;
        let tokens = tokenize_imap_line(line);
        assert_eq!(tokens, vec!["A01", "LOGIN", "user@test.com", "secret pass"]);

        let line_with_parens = r#"A02 STORE 1:5 +FLAGS (\Seen \Flagged)"#;
        let tokens = tokenize_imap_line(line_with_parens);
        assert_eq!(tokens, vec!["A02", "STORE", "1:5", "+FLAGS", r#"\Seen \Flagged"#]);
    }

    #[test]
    fn test_parse_sequence_set() {
        assert_eq!(parse_sequence_set("1", 5), vec![1]);
        assert_eq!(parse_sequence_set("1:3", 5), vec![1, 2, 3]);
        assert_eq!(parse_sequence_set("2,4", 5), vec![2, 4]);
        assert_eq!(parse_sequence_set("3:*", 5), vec![3, 4, 5]);
        assert_eq!(parse_sequence_set("*", 5), vec![5]);
    }

    #[test]
    fn test_format_flags() {
        let json = r#"["\\Seen", "\\Flagged"]"#;
        assert_eq!(format_flags_for_imap(json), r#"\Seen \Flagged"#);
    }

    #[tokio::test]
    async fn test_imap_full_session_flow() {
        let db = setup_test_db();
        let tenant_id = db.insert_tenant("imap-test.com").unwrap();
        let _account_id = db
            .insert_account(&tenant_id, "alice", "alice@imap-test.com", "SecretPass123!")
            .unwrap();

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server_db = Arc::clone(&db);
        let temp_dir = std::env::temp_dir().join(Uuid::new_v4().to_string());
        std::fs::create_dir_all(&temp_dir).unwrap();
        let data_dir = temp_dir.to_str().unwrap().to_string();
        let server_data_dir = data_dir.clone();

        tokio::spawn(async move {
            if let Ok((stream, _)) = listener.accept().await {
                let _ = handle_imap_connection(stream, server_db, server_data_dir).await;
            }
        });

        // Connect client
        let stream = TcpStream::connect(addr).await.unwrap();
        let (reader, mut writer) = stream.into_split();
        let mut reader = BufReader::new(reader);

        let mut line = String::new();

        // 1. Banner
        reader.read_line(&mut line).await.unwrap();
        assert!(line.contains("* OK FastrMail IMAP4rev2 Ready"));

        // 2. CAPABILITY
        writer.write_all(b"A01 CAPABILITY\r\n").await.unwrap();
        writer.flush().await.unwrap();
        line.clear();
        reader.read_line(&mut line).await.unwrap();
        assert!(line.contains("CAPABILITY"));
        line.clear();
        reader.read_line(&mut line).await.unwrap();
        assert!(line.starts_with("A01 OK"));

        // 3. LOGIN
        writer
            .write_all(b"A02 LOGIN alice@imap-test.com SecretPass123!\r\n")
            .await
            .unwrap();
        writer.flush().await.unwrap();
        line.clear();
        reader.read_line(&mut line).await.unwrap();
        assert!(line.starts_with("A02 OK LOGIN completed"));

        // 4. LIST
        writer.write_all(b"A03 LIST \"\" \"*\"\r\n").await.unwrap();
        writer.flush().await.unwrap();
        line.clear();
        reader.read_line(&mut line).await.unwrap();
        assert!(line.contains("LIST () \"/\" \"INBOX\""));
        line.clear();
        reader.read_line(&mut line).await.unwrap();
        assert!(line.starts_with("A03 OK LIST completed"));

        // 5. SELECT INBOX
        writer.write_all(b"A04 SELECT INBOX\r\n").await.unwrap();
        writer.flush().await.unwrap();
        line.clear();
        reader.read_line(&mut line).await.unwrap();
        assert!(line.contains("EXISTS"));
        loop {
            line.clear();
            reader.read_line(&mut line).await.unwrap();
            if line.starts_with("A04 OK") {
                break;
            }
        }

        // 6. APPEND a test message
        let email_raw = b"From: bob@imap-test.com\r\nTo: alice@imap-test.com\r\nSubject: Hello IMAP\r\n\r\nBody content\r\n";
        let append_cmd = format!("A05 APPEND INBOX (\\Seen) {{{}}}\r\n", email_raw.len());
        writer.write_all(append_cmd.as_bytes()).await.unwrap();
        writer.flush().await.unwrap();

        line.clear();
        reader.read_line(&mut line).await.unwrap();
        assert!(line.starts_with('+'));

        writer.write_all(email_raw).await.unwrap();
        writer.write_all(b"\r\n").await.unwrap();
        writer.flush().await.unwrap();

        line.clear();
        reader.read_line(&mut line).await.unwrap();
        assert!(line.starts_with("A05 OK [APPENDUID"));

        // 7. FETCH
        writer.write_all(b"A06 FETCH 1 (FLAGS RFC822.SIZE)\r\n").await.unwrap();
        writer.flush().await.unwrap();
        line.clear();
        reader.read_line(&mut line).await.unwrap();
        assert!(line.contains("FETCH") && line.contains("FLAGS"));
        line.clear();
        reader.read_line(&mut line).await.unwrap();
        assert!(line.starts_with("A06 OK FETCH completed"));

        // 8. STORE (mark Deleted)
        writer.write_all(b"A07 STORE 1 +FLAGS (\\Deleted)\r\n").await.unwrap();
        writer.flush().await.unwrap();
        line.clear();
        reader.read_line(&mut line).await.unwrap();
        assert!(line.contains("FETCH") && line.contains("Deleted"));
        line.clear();
        reader.read_line(&mut line).await.unwrap();
        assert!(line.starts_with("A07 OK STORE completed"));

        // 9. EXPUNGE
        writer.write_all(b"A08 EXPUNGE\r\n").await.unwrap();
        writer.flush().await.unwrap();
        line.clear();
        reader.read_line(&mut line).await.unwrap();
        assert!(line.contains("1 EXPUNGE"));
        line.clear();
        reader.read_line(&mut line).await.unwrap();
        assert!(line.starts_with("A08 OK EXPUNGE completed"));

        // 10. LOGOUT
        writer.write_all(b"A09 LOGOUT\r\n").await.unwrap();
        writer.flush().await.unwrap();
        line.clear();
        reader.read_line(&mut line).await.unwrap();
        assert!(line.contains("* BYE"));
        line.clear();
        reader.read_line(&mut line).await.unwrap();
        assert!(line.starts_with("A09 OK LOGOUT completed"));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[tokio::test]
    async fn test_imap_auth_failure_and_mailbox_management() {
        let db = setup_test_db();
        let tenant_id = db.insert_tenant("manage-test.com").unwrap();
        let _account_id = db
            .insert_account(&tenant_id, "user", "user@manage-test.com", "CorrectPass123")
            .unwrap();

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server_db = Arc::clone(&db);
        let temp_dir = std::env::temp_dir().join(Uuid::new_v4().to_string());
        std::fs::create_dir_all(&temp_dir).unwrap();
        let data_dir = temp_dir.to_str().unwrap().to_string();

        tokio::spawn(async move {
            if let Ok((stream, _)) = listener.accept().await {
                let _ = handle_imap_connection(stream, server_db, data_dir).await;
            }
        });

        let stream = TcpStream::connect(addr).await.unwrap();
        let (reader, mut writer) = stream.into_split();
        let mut reader = BufReader::new(reader);
        let mut line = String::new();

        // Banner
        reader.read_line(&mut line).await.unwrap();

        // Wrong password
        writer
            .write_all(b"B01 LOGIN user@manage-test.com WrongPass\r\n")
            .await
            .unwrap();
        writer.flush().await.unwrap();
        line.clear();
        reader.read_line(&mut line).await.unwrap();
        assert!(line.starts_with("B01 NO [AUTHENTICATIONFAILED]"));

        // Correct password
        writer
            .write_all(b"B02 LOGIN user@manage-test.com CorrectPass123\r\n")
            .await
            .unwrap();
        writer.flush().await.unwrap();
        line.clear();
        reader.read_line(&mut line).await.unwrap();
        assert!(line.starts_with("B02 OK LOGIN completed"));

        // CREATE mailbox
        writer.write_all(b"B03 CREATE \"Work\"\r\n").await.unwrap();
        writer.flush().await.unwrap();
        line.clear();
        reader.read_line(&mut line).await.unwrap();
        assert!(line.starts_with("B03 OK CREATE completed"));

        // STATUS Work
        writer.write_all(b"B04 STATUS \"Work\" (MESSAGES UNSEEN UIDNEXT)\r\n").await.unwrap();
        writer.flush().await.unwrap();
        line.clear();
        reader.read_line(&mut line).await.unwrap();
        assert!(line.contains("STATUS") && line.contains("MESSAGES 0"));
        line.clear();
        reader.read_line(&mut line).await.unwrap();
        assert!(line.starts_with("B04 OK STATUS completed"));

        // RENAME Work -> Projects
        writer.write_all(b"B05 RENAME \"Work\" \"Projects\"\r\n").await.unwrap();
        writer.flush().await.unwrap();
        line.clear();
        reader.read_line(&mut line).await.unwrap();
        assert!(line.starts_with("B05 OK RENAME completed"));

        // DELETE Projects
        writer.write_all(b"B06 DELETE \"Projects\"\r\n").await.unwrap();
        writer.flush().await.unwrap();
        line.clear();
        reader.read_line(&mut line).await.unwrap();
        assert!(line.starts_with("B06 OK DELETE completed"));

        // Cannot DELETE INBOX
        writer.write_all(b"B07 DELETE \"INBOX\"\r\n").await.unwrap();
        writer.flush().await.unwrap();
        line.clear();
        reader.read_line(&mut line).await.unwrap();
        assert!(line.starts_with("B07 NO Cannot delete INBOX"));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
