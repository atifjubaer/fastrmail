# FastrMail — Phase 5 Complete

**Project:** FastrMail — The 100% free, open-source, single-binary mail & collaboration server  
**Author:** Atif Jubaer  
**Status:** Phase 5 (Spam Filtering, Greylisting, DNSBLs & Outbound NDRs) 100% Complete  

---

## 1. What Was Built in Phase 5

### 1.1 Greylisting Engine (`fastrmail-store`)
- **Greylist Schema & Indexes**:
  - `greylist` table: `id` (TEXT PRIMARY KEY), `sender_ip` (TEXT), `sender` (TEXT), `recipient` (TEXT), `first_seen` (DATETIME), `passed` (BOOLEAN DEFAULT 0).
  - Strict `UNIQUE(sender_ip, sender, recipient)` constraint.
  - Dedicated composite index `idx_greylist_lookup` for high-throughput lookup.
- **State Transition & Windowing Logic**:
  - `check_greylist(ip, sender, recipient)`:
    - If unknown tuple: records `first_seen = now()`, `passed = false`, and defers with `false`.
    - If seen within the 5-minute greylisting window: returns `false` (deferral).
    - If seen and elapsed time >= 5 minutes: updates `passed = true` and returns `true`.
    - If previously passed: returns `true` immediately without delay.
  - `check_greylist_with_window`: Configurable duration helper for testing and custom throttling.
  - `insert_greylist_record`: Test helper with conflict replacement.

### 1.2 DNS Blocklists (DNSBL) Verification (`fastrmail-auth`)
- **`DnsblVerifier`**:
  - Configured with industry standard blocklist zones:
    - `zen.spamhaus.org` (PBL, SBL, XBL)
    - `b.barracudacentral.org`
  - Reverses IPv4 octets (`1.2.3.4` -> `4.3.2.1.<zone>`) and IPv6 nibbles.
  - Asynchronous DNS resolution via `hickory-resolver::TokioAsyncResolver`.
  - **RFC 5782 & Provider Error Compliance**:
    - Specifically filters `127.0.0.x` listing codes (`127.0.0.2` - `127.0.0.127`).
    - Distinguishes actual spam listings from provider query error/refusal codes (such as `127.255.255.254` for open resolver queries), preventing false positive rejections of safe IPs.
  - `DnsblVerifier::mock_blocked()` constructor for hermetic offline test validation.

### 1.3 Inbound SMTP Security Enforcement (`fastrmail-smtp`)
- Connected `DnsblVerifier` and `Database::check_greylist` into `handle_connection` during the `RCPT TO` phase:
  1. **DNSBL Reputation Check**:
     - Non-loopback client IPs are checked against DNSBL zones.
     - If blocked: immediately issues `554 5.7.1 Service unavailable; Client host [IP] blocked using DNSBL` and terminates session.
  2. **Greylisting Check**:
     - Client IP + sender + recipient tuple is checked against the database.
     - If first-time or within deferral window: issues `451 4.7.1 Greylisting in action, please try again later` and closes connection without storing.
  3. **Bypass Flag**:
     - Configurable `with_bypass_spam_check(bool)` allowing loopback testing and authenticated whitelisting.

### 1.4 Outbound Queue Management & Non-Delivery Reports (NDR) (`fastrmail-smtp`)
- **Automatic Bounce Processing**:
  - When an outbound email exhausts all 5 retry attempts (`new_retry >= 5`), status transitions from `retrying` to `failed`.
  - Generates an RFC-compliant Non-Delivery Report (NDR) bounce email:
    - From: `MAILER-DAEMON@<tenant_domain>`
    - To: Original sender
    - Subject: `Undelivered Mail Returned to Sender: Delivery Failure to <recipient>`
    - Headers: `Auto-Submitted: auto-replied`, `Diagnostic-Code: smtp; <error_message>`, standard RFC 2822 date.
    - Body: Human-readable explanation and technical delivery failure diagnostics.
  - If original sender is a local tenant account:
    - Saves `.eml` blob to `{data_dir}/blobs/`.
    - Automatically inserts the NDR message into the sender's `INBOX` mailbox so they are immediately notified via Webmail, IMAP, and JMAP.

---

## 2. Verification & Test Results

All **47 automated tests** pass with **0 errors and 0 compiler warnings**:

| Test Suite | Total Tests | Passed | Failed |
|---|---|---|---|
| `fastrmail-smtp` | 9 | 9 | 0 |
| `fastrmail-store` | 9 | 9 | 0 |
| `fastrmail-auth` | 9 | 9 | 0 |
| `fastrmail-binary` | 6 | 6 | 0 |
| `fastrmail-imap` | 5 | 5 | 0 |
| `fastrmail-jmap` | 5 | 5 | 0 |
| `fastrmail-core` | 3 | 3 | 0 |
| `fastrmail-search` | 1 | 1 | 0 |
| **Total** | **47** | **47** | **0** |

### Key Phase 5 Tests
- `fastrmail-store::tests::test_greylist_workflow`: Tests initial deferral, retry within window deferral, delay window expiry passing, and subsequent instant pass.
- `fastrmail-auth::tests::test_dnsbl_safe_ip`: Verifies clean resolution for `8.8.8.8` without false positives.
- `fastrmail-auth::tests::test_dnsbl_empty_zones`: Validates fallback on empty zones.
- `fastrmail-smtp::tests::test_smtp_dnsbl_rejection`: Live SMTP TCP test verifying 554 rejection when client IP is blocked on DNSBL.
- `fastrmail-smtp::tests::test_smtp_greylisting_deferral_and_pass`: Live SMTP TCP test verifying 451 greylisting deferral on initial attempt, followed by 250 OK after delay window passes.
- `fastrmail-smtp::outbound::tests::test_ndr_generation_on_max_retries`: Simulates permanent delivery failure after 5 retries and verifies NDR insertion into sender's INBOX.

---

## 3. Complete Security & Delivery Architecture

| Layer | Component | Mechanism | Result |
|---|---|---|---|
| Inbound Security | DNSBL Verifier | Real-time IP query against Spamhaus & Barracuda | `554` Permanent Rejection |
| Inbound Security | Greylisting Guard | SQLite tuple tracking with 5-minute retry window | `451` Temporary Deferral |
| Inbound Security | SPF Verification | Reverse DNS + TXT SPF policy validation | Authentication-Results header |
| Inbound Security | DKIM Verification | RSA-SHA256 signature verification | Authentication-Results header |
| Inbound Security | DMARC Evaluator | Alignment check between From domain & SPF/DKIM | `550` Rejection if `p=reject` |
| Outbound Delivery | Outbound Engine | Tokio queue worker with MX resolution | Direct delivery to recipient MX |
| Outbound Security | DKIM Signer | RSA 2048-bit automated signing on departure | Cryptographic signature header |
| Outbound Reliability | Retry Manager | Exponential backoff (5m -> 15m -> 1h -> 4h -> 24h) | 5 retries over 30 hours |
| Outbound Feedback | Bounce Processor | NDR generation on retry exhaustion | NDR email delivered to sender's INBOX |
