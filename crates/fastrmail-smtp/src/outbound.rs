//! FastrMail SMTP Outbound Engine — Delivers queued emails to recipient MX servers with DKIM signing.

use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use hickory_resolver::config::{ResolverConfig, ResolverOpts};
use hickory_resolver::TokioAsyncResolver;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::sync::broadcast;
use tracing::{error, info, warn};

use fastrmail_auth::DkimSigner;
use fastrmail_core::QueueItem;
use fastrmail_store::Database;
use uuid::Uuid;

/// Delivery engine that processes queued outbound emails.
pub struct OutboundEngine {
    db: Arc<Database>,
    data_dir: String,
    resolver: TokioAsyncResolver,
    /// Optional override port for testing with mock SMTP servers (defaults to 25).
    port_override: Option<u16>,
}

impl OutboundEngine {
    /// Create a new outbound engine.
    pub fn new(db: Arc<Database>, data_dir: String) -> Self {
        let resolver =
            TokioAsyncResolver::tokio(ResolverConfig::cloudflare(), ResolverOpts::default());
        Self {
            db,
            data_dir,
            resolver,
            port_override: None,
        }
    }

    /// Create an outbound engine with a custom destination port (useful for integration tests).
    pub fn with_port(db: Arc<Database>, data_dir: String, port: u16) -> Self {
        let mut engine = Self::new(db, data_dir);
        engine.port_override = Some(port);
        engine
    }

    /// Start the background queue processor worker task.
    pub fn start(self: Arc<Self>, mut shutdown_rx: broadcast::Receiver<()>) {
        tokio::spawn(async move {
            info!("Outbound SMTP delivery worker started");
            let mut interval = tokio::time::interval(Duration::from_secs(5));

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        if let Err(e) = self.process_queue().await {
                            error!("Error processing outbound SMTP queue: {e}");
                        }
                    }
                    _ = shutdown_rx.recv() => {
                        info!("Outbound delivery worker stopping");
                        break;
                    }
                }
            }
        });
    }

    /// Process all pending or due-for-retry emails in the outbound queue.
    pub async fn process_queue(&self) -> Result<()> {
        let pending = self.db.get_queue_pending()?;
        for item in pending {
            self.deliver_one(item).await;
        }
        Ok(())
    }

    /// Attempt delivery for a single queued email.
    pub async fn deliver_one(&self, item: QueueItem) {
        info!(
            "Processing queue item {}: sender={} recipient={}",
            item.id, item.sender, item.recipient
        );

        match self.try_deliver(&item).await {
            Ok(()) => {
                info!("Queue item {} delivered successfully", item.id);
                if let Err(e) = self.db.delete_queue_item(&item.id) {
                    error!(
                        "Failed to remove delivered item {} from queue: {e}",
                        item.id
                    );
                }
            }
            Err(e) => {
                warn!("Delivery failed for item {}: {e}", item.id);
                let new_retry = item.retry_count + 1;
                if new_retry >= 5 {
                    warn!(
                        "Item {} exceeded max retries (5). Marking as failed.",
                        item.id
                    );
                    let _ = self
                        .db
                        .update_queue_status(&item.id, "failed", None, new_retry);
                    if let Err(ndr_err) = self.generate_and_store_ndr(&item, &e.to_string()) {
                        error!("Failed to generate NDR for item {}: {ndr_err}", item.id);
                    }
                } else {
                    let next_retry_at = calculate_next_retry(new_retry);
                    info!(
                        "Scheduling retry {} for item {} at {:?}",
                        new_retry, item.id, next_retry_at
                    );
                    let _ = self.db.update_queue_status(
                        &item.id,
                        "retrying",
                        Some(next_retry_at),
                        new_retry,
                    );
                }
            }
        }
    }

    /// Generate a Non-Delivery Report (NDR) email and store it in the original sender's INBOX.
    pub fn generate_and_store_ndr(&self, item: &QueueItem, error_msg: &str) -> Result<()> {
        let sender_account = match self.db.get_account_by_email(&item.sender)? {
            Some(acc) => acc,
            None => {
                info!(
                    "Sender {} is not a local account, skipping local NDR storage",
                    item.sender
                );
                return Ok(());
            }
        };

        let mailbox = match self.db.get_mailbox_by_name(&sender_account.id, "INBOX")? {
            Some(mb) => mb,
            None => {
                let mb_id = self.db.insert_mailbox(&sender_account.id, "INBOX")?;
                self.db
                    .get_mailbox_by_id(&mb_id)?
                    .context("Failed to retrieve created INBOX")?
            }
        };

        let sender_domain = item.sender.split('@').nth(1).unwrap_or("fastrmail.local");
        let ndr_from = format!("MAILER-DAEMON@{sender_domain}");
        let ndr_subject = format!(
            "Undelivered Mail Returned to Sender: Delivery Failure to {}",
            item.recipient
        );
        let now = Utc::now();
        let date_str = now.to_rfc2822();
        let ndr_blob_id = format!("ndr_{}", Uuid::new_v4());

        let body = format!(
            "This is the mail system at FastrMail host {sender_domain}.\r\n\r\n\
             I'm sorry to have to inform you that your message could not\r\n\
             be delivered to one or more recipients. It's attached below.\r\n\r\n\
             For further assistance, please send mail to postmaster.\r\n\r\n\
             If you do so, please include this problem report. You can\r\n\
             delete your own text from the attached returned message.\r\n\r\n\
                                The mail system\r\n\r\n\
             <{recipient}>: Delivery failed after 5 retry attempts.\r\n\
             Diagnostic-Code: smtp; {error_msg}\r\n",
            recipient = item.recipient,
            error_msg = error_msg,
        );

        let raw_ndr = format!(
            "From: {ndr_from}\r\n\
             To: {to}\r\n\
             Subject: {ndr_subject}\r\n\
             Date: {date_str}\r\n\
             MIME-Version: 1.0\r\n\
             Content-Type: text/plain; charset=utf-8\r\n\
             Auto-Submitted: auto-replied\r\n\
             \r\n\
             {body}",
            to = item.sender,
        );

        let blob_dir = format!("{}/blobs", self.data_dir);
        std::fs::create_dir_all(&blob_dir)?;
        let blob_path = format!("{blob_dir}/{ndr_blob_id}.eml");
        std::fs::write(&blob_path, raw_ndr.as_bytes())?;

        let msg_id = self.db.insert_message(
            &mailbox.id,
            &sender_account.id,
            &ndr_blob_id,
            raw_ndr.len() as i64,
            Some(&ndr_subject),
            Some(&ndr_from),
            Some(&item.sender),
        )?;

        info!(
            "Generated Non-Delivery Report {} for sender {} in INBOX",
            msg_id, item.sender
        );

        Ok(())
    }

    /// Perform the end-to-end SMTP delivery handshake to the destination MX.
    async fn try_deliver(&self, item: &QueueItem) -> Result<()> {
        // 1. Load the raw .eml blob from disk
        let blob_path = format!("{}/blobs/{}.eml", self.data_dir, item.raw_blob_id);
        let raw_bytes = std::fs::read(&blob_path)
            .with_context(|| format!("Failed to read email blob from {blob_path}"))?;

        // 2. DKIM sign if an active key exists for the tenant
        let signed_bytes = if let Ok(Some(dkim_key)) = self.db.get_active_dkim_key(&item.tenant_id)
        {
            let sender_domain = item.sender.split('@').nth(1).unwrap_or("localhost");
            match DkimSigner::sign(
                &raw_bytes,
                sender_domain,
                &dkim_key.selector,
                &dkim_key.private_key_pem,
            ) {
                Ok(signed) => signed,
                Err(e) => {
                    warn!(
                        "DKIM signing failed for item {}: {e}; sending unsigned",
                        item.id
                    );
                    raw_bytes
                }
            }
        } else {
            raw_bytes
        };

        // 3. Extract recipient domain and resolve MX records
        let recipient_domain = item
            .recipient
            .split('@')
            .nth(1)
            .context("Invalid recipient address without domain")?;

        let port = self.port_override.unwrap_or(25);

        // In test mode or when connecting to localhost
        if recipient_domain == "localhost"
            || recipient_domain == "127.0.0.1"
            || self.port_override.is_some()
        {
            let target_addr = format!("127.0.0.1:{port}");
            return send_smtp_mail(&target_addr, &item.sender, &item.recipient, &signed_bytes)
                .await;
        }

        // Production: resolve MX records
        let mx_lookup = self
            .resolver
            .mx_lookup(recipient_domain)
            .await
            .with_context(|| format!("Failed to resolve MX records for {recipient_domain}"))?;

        let mut mx_hosts: Vec<_> = mx_lookup.iter().collect();
        mx_hosts.sort_by_key(|record| record.preference());

        let mut last_err = anyhow::anyhow!("No MX hosts available");
        for mx in mx_hosts {
            let host = mx.exchange().to_string();
            let target_addr = format!("{}:{}", host.trim_end_matches('.'), port);
            match send_smtp_mail(&target_addr, &item.sender, &item.recipient, &signed_bytes).await {
                Ok(()) => return Ok(()),
                Err(e) => {
                    warn!("Failed delivery to {target_addr}: {e}");
                    last_err = e;
                }
            }
        }

        Err(last_err)
    }
}

/// Calculate the next retry timestamp with exponential backoff:
/// 1: 5min, 2: 15min, 3: 1hr, 4: 4hr, 5: 24hr.
pub fn calculate_next_retry(retry_count: i64) -> DateTime<Utc> {
    let now = Utc::now();
    let delay = match retry_count {
        1 => ChronoDuration::minutes(5),
        2 => ChronoDuration::minutes(15),
        3 => ChronoDuration::hours(1),
        4 => ChronoDuration::hours(4),
        _ => ChronoDuration::hours(24),
    };
    now + delay
}

/// Perform an SMTP handshake (EHLO, MAIL FROM, RCPT TO, DATA) over TCP.
pub async fn send_smtp_mail(
    target_addr: &str,
    sender: &str,
    recipient: &str,
    email_data: &[u8],
) -> Result<()> {
    let stream = TcpStream::connect(target_addr)
        .await
        .with_context(|| format!("Failed to connect to SMTP server at {target_addr}"))?;

    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);

    // 1. Read greeting (220)
    let greeting = read_smtp_response(&mut reader).await?;
    if !greeting.starts_with("220") {
        anyhow::bail!("Invalid greeting from {target_addr}: {greeting}");
    }

    // 2. EHLO
    writer.write_all(b"EHLO fastrmail.local\r\n").await?;
    let ehlo_resp = read_smtp_response(&mut reader).await?;
    if !ehlo_resp.starts_with("250") {
        anyhow::bail!("EHLO rejected by {target_addr}: {ehlo_resp}");
    }

    // 3. MAIL FROM
    let mail_from = format!("MAIL FROM:<{sender}>\r\n");
    writer.write_all(mail_from.as_bytes()).await?;
    let mail_resp = read_smtp_response(&mut reader).await?;
    if !mail_resp.starts_with("250") {
        anyhow::bail!("MAIL FROM rejected by {target_addr}: {mail_resp}");
    }

    // 4. RCPT TO
    let rcpt_to = format!("RCPT TO:<{recipient}>\r\n");
    writer.write_all(rcpt_to.as_bytes()).await?;
    let rcpt_resp = read_smtp_response(&mut reader).await?;
    if !rcpt_resp.starts_with("250") {
        anyhow::bail!("RCPT TO rejected by {target_addr}: {rcpt_resp}");
    }

    // 5. DATA
    writer.write_all(b"DATA\r\n").await?;
    let data_resp = read_smtp_response(&mut reader).await?;
    if !data_resp.starts_with("354") {
        anyhow::bail!("DATA rejected by {target_addr}: {data_resp}");
    }

    // 6. Send body + CRLF.CRLF
    writer.write_all(email_data).await?;
    if !email_data.ends_with(b"\r\n") {
        writer.write_all(b"\r\n").await?;
    }
    writer.write_all(b".\r\n").await?;
    let accepted_resp = read_smtp_response(&mut reader).await?;
    if !accepted_resp.starts_with("250") {
        anyhow::bail!("Message body rejected by {target_addr}: {accepted_resp}");
    }

    // 7. QUIT
    writer.write_all(b"QUIT\r\n").await?;
    let _ = read_smtp_response(&mut reader).await;

    Ok(())
}

/// Helper to read an SMTP response line (handling multi-line responses).
async fn read_smtp_response(
    reader: &mut BufReader<tokio::net::tcp::OwnedReadHalf>,
) -> Result<String> {
    let mut last_line = String::new();
    loop {
        let mut line = String::new();
        let bytes_read = reader.read_line(&mut line).await?;
        if bytes_read == 0 {
            break;
        }
        let is_continuation = line.len() > 3 && line.as_bytes()[3] == b'-';
        last_line = line;
        if !is_continuation {
            break;
        }
    }
    Ok(last_line)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exponential_backoff_calculation() {
        let now = Utc::now();
        let r1 = calculate_next_retry(1);
        let r2 = calculate_next_retry(2);
        let r3 = calculate_next_retry(3);
        let r4 = calculate_next_retry(4);
        let r5 = calculate_next_retry(5);

        assert!(r1 > now + ChronoDuration::minutes(4));
        assert!(r2 > now + ChronoDuration::minutes(14));
        assert!(r3 > now + ChronoDuration::minutes(55));
        assert!(r4 > now + ChronoDuration::hours(3));
        assert!(r5 > now + ChronoDuration::hours(23));
    }

    #[tokio::test]
    async fn test_ndr_generation_on_max_retries() {
        let db = Database::new_memory().expect("Failed to create in-memory DB");
        db.init_schema().expect("Failed to init schema");
        let db = Arc::new(db);

        let temp_dir = std::env::temp_dir().join(format!("fastrmail_ndr_test_{}", Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let data_dir = temp_dir.to_string_lossy().to_string();

        // 1. Create tenant and local sender account with INBOX
        let tenant_id = db.insert_tenant("senderdomain.org").unwrap();
        let account_id = db
            .insert_account(&tenant_id, "alice", "alice@senderdomain.org", "Pass123!")
            .unwrap();
        let inbox_id = db.insert_mailbox(&account_id, "INBOX").unwrap();

        // 2. Create raw message blob for outgoing mail
        let blob_dir = format!("{data_dir}/blobs");
        std::fs::create_dir_all(&blob_dir).unwrap();
        let blob_id = format!("blob_{}", Uuid::new_v4());
        let blob_path = format!("{blob_dir}/{blob_id}.eml");
        std::fs::write(&blob_path, b"Subject: Hello world\r\n\r\nHi!").unwrap();

        // 3. Queue item with retry_count = 4 (next attempt will be 5, exceeding limit)
        let queue_id = db
            .queue_email(
                &tenant_id,
                &blob_id,
                "alice@senderdomain.org",
                "recipient@unreachable-domain-xyz123.com",
            )
            .unwrap();

        // Update retry_count to 4 in database
        db.update_queue_status(&queue_id, "retrying", None, 4)
            .unwrap();

        let queue_item = db
            .get_queue_pending()
            .unwrap()
            .into_iter()
            .find(|item| item.id == queue_id)
            .expect("Queue item must exist");

        // 4. Run deliver_one on engine with invalid override port (immediate connection failure)
        let engine = OutboundEngine::with_port(Arc::clone(&db), data_dir.clone(), 1);
        engine.deliver_one(queue_item).await;

        // 5. Verify queue status is failed
        let queue_item_after = db.get_queue_pending().unwrap();
        assert!(
            queue_item_after.is_empty(),
            "Failed queue item must no longer be pending"
        );

        // 6. Verify NDR message was inserted into alice's INBOX
        let messages = db.get_messages_by_mailbox(&inbox_id).unwrap();
        assert_eq!(
            messages.len(),
            1,
            "Alice should have received 1 NDR message in INBOX"
        );
        let ndr = &messages[0];
        assert!(
            ndr.parsed_subject
                .as_ref()
                .unwrap()
                .contains("Undelivered Mail Returned to Sender"),
            "Subject must indicate delivery failure"
        );
        assert_eq!(ndr.parsed_to.as_deref(), Some("alice@senderdomain.org"));

        // Verify NDR blob exists on disk
        let ndr_path = format!("{blob_dir}/{}.eml", ndr.blob_id);
        let ndr_content = std::fs::read_to_string(&ndr_path).unwrap();
        assert!(ndr_content.contains("Delivery failed after 5 retry attempts"));
        assert!(ndr_content.contains("Auto-Submitted: auto-replied"));

        std::fs::remove_dir_all(&temp_dir).ok();
    }
}
