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
        let resolver = TokioAsyncResolver::tokio(
            ResolverConfig::cloudflare(),
            ResolverOpts::default(),
        );
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
                    error!("Failed to remove delivered item {} from queue: {e}", item.id);
                }
            }
            Err(e) => {
                warn!("Delivery failed for item {}: {e}", item.id);
                let new_retry = item.retry_count + 1;
                if new_retry >= 5 {
                    warn!("Item {} exceeded max retries (5). Marking as failed.", item.id);
                    let _ = self.db.update_queue_status(&item.id, "failed", None, new_retry);
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

    /// Perform the end-to-end SMTP delivery handshake to the destination MX.
    async fn try_deliver(&self, item: &QueueItem) -> Result<()> {
        // 1. Load the raw .eml blob from disk
        let blob_path = format!("{}/blobs/{}.eml", self.data_dir, item.raw_blob_id);
        let raw_bytes = std::fs::read(&blob_path)
            .with_context(|| format!("Failed to read email blob from {blob_path}"))?;

        // 2. DKIM sign if an active key exists for the tenant
        let signed_bytes = if let Ok(Some(dkim_key)) = self.db.get_active_dkim_key(&item.tenant_id) {
            let sender_domain = item
                .sender
                .split('@')
                .nth(1)
                .unwrap_or("localhost");
            match DkimSigner::sign(&raw_bytes, sender_domain, &dkim_key.selector, &dkim_key.private_key_pem) {
                Ok(signed) => signed,
                Err(e) => {
                    warn!("DKIM signing failed for item {}: {e}; sending unsigned", item.id);
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
        if recipient_domain == "localhost" || recipient_domain == "127.0.0.1" || self.port_override.is_some() {
            let target_addr = format!("127.0.0.1:{port}");
            return send_smtp_mail(&target_addr, &item.sender, &item.recipient, &signed_bytes).await;
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
async fn read_smtp_response(reader: &mut BufReader<tokio::net::tcp::OwnedReadHalf>) -> Result<String> {
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
}
