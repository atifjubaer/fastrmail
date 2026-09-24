# SMTP Inbound & Outbound MTA

FastrMail contains a high-performance RFC 5321 SMTP implementation built directly on Tokio TCP streams.

---

## Inbound Engine Features

- **Port**: Listens on `:2525` (mapped to `:25` in Docker).
- **Security Check Pipeline**:
  1. **DNSBL Verification**: Queries DNSBL zones (e.g., `zen.spamhaus.org`, `b.barracudacentral.org`). Listed IPs receive `554 5.7.1 Service unavailable; Client host [IP] blocked using DNSBL`.
  2. **Greylisting**: RFC deferral tracking `(sender_ip, sender_email, recipient_email)`. New senders receive `451 4.7.1 Greylisting in action, please try again in 5 minutes`. Legitimate MTAs retry after 5 minutes and are automatically whitelisted.
  3. **SPF / DKIM / DMARC**: Analyzed via `fastrmail-auth`. Unaligned messages with `p=reject` policy receive `550 5.7.1 DMARC policy violation`.
- **Tantivy Indexing**: Inbound messages passing all checks are saved into SQLite and immediately indexed in Tantivy for sub-millisecond search.

---

## Outbound MTA & NDR Bounces

- **MX Discovery**: Async DNS resolution of MX hosts for recipient domain using `hickory-resolver`.
- **Delivery Protocol**: Async SMTP delivery via `lettre` with STARTTLS opportunistic encryption.
- **DKIM Signing**: Automatic 2048-bit RSA DKIM signing (`v=1; a=rsa-sha256; d=domain; s=selector`) using stored tenant keys.
- **Exponential Backoff**:
  - Retries: Up to 5 attempts.
  - Interval: `base_delay * 2^retries` (5m, 10m, 20m, 40m, 80m).
- **Non-Delivery Report (NDR)**:
  - If a message exhausts all retries, FastrMail automatically generates an RFC 3464 Non-Delivery Report email from `MAILER-DAEMON@<domain>` and places it directly into the sender's `INBOX`.
