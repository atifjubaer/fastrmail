# FastrMail — Phase 2 Complete

**Project:** FastrMail — The 100% free, open-source, single-binary mail & collaboration server  
**Author:** Atif Jubaer  
**Status:** Phase 2 (SMTP Authentication, Security & Outbound Delivery) 100% Complete  

---

## 1. What Was Built in Phase 2

### 1.1 Inbound Email Security (`fastrmail-auth`)
- **DKIM Verifier (`DkimVerifier`)**: Evaluates incoming RFC 5322 messages, extracts `d=` and `s=` signatures, queries DNS TXT records via asynchronous resolvers, and validates signatures.
- **SPF Verifier (`SpfVerifier`)**: Verifies client IP address, HELO/EHLO identity, and envelope MAIL FROM sender against published SPF policies using `hickory-resolver`.
- **DMARC Evaluator (`DmarcEvaluator`)**: Implements RFC 7489 alignment checks for both DKIM and SPF against the Header From domain, evaluates published `_dmarc.<domain>` policies (`reject`, `quarantine`, `none`), and enforces policy rejection.
- **DKIM Signer (`DkimSigner`)**: Signs outbound messages using 2048-bit RSA keys with canonicalization and SHA-256 (`rsa-sha256`), prepending standard RFC 6376 `DKIM-Signature` headers.
- **DKIM Key Generation**: Generates 2048-bit RSA key pairs, outputs formatted PKCS#1 PEM private keys and base64-encoded public keys for DNS TXT records.
- **Argon2id Password Security**: Secure password hashing (`hash_password`) and constant-time verification (`verify_password`) using Argon2id with random salt.

### 1.2 Inbound SMTP Enforcement (`fastrmail-smtp`)
- Extract client IP from TCP connection stream.
- After `DATA` receipt, runs `SpfVerifier`, `DkimVerifier`, and `DmarcEvaluator`.
- Rejects forged or non-aligned messages from domains publishing `p=reject` with `550 DMARC policy rejection` before storage.
- Injects standard `Authentication-Results: fastrmail; dkim=...; spf=...; dmarc=...` headers into stored `.eml` blobs.

### 1.3 Outbound SMTP Delivery Engine (`fastrmail-smtp::outbound`)
- Background tokio worker polling `smtp_queue` every 5 seconds.
- Queries MX records via `TokioAsyncResolver` sorted by preference.
- Connects over TCP to destination port 25 with full handshake (`EHLO` -> `MAIL FROM` -> `RCPT TO` -> `DATA` -> `QUIT`).
- Loads active DKIM signing key from `dkim_keys` table and signs emails before dispatch.
- Manages delivery status (`delivered`, `retrying`, `failed`) and exponential backoff retry scheduling (5m, 15m, 1h, 4h, 24h, max 5 retries).

### 1.4 CLI DKIM Key Generator (`fastrmail-binary`)
- Command: `fastrmail --generate-dkim <domain>`
- Generates 2048-bit RSA key pair, saves private key to SQLite `dkim_keys`, and prints ready-to-copy DNS TXT record.

### 1.5 SQLite Storage Improvements (`fastrmail-store`)
- Password hashing in `insert_account` with Argon2id.
- Account authentication method `verify_login(email, password) -> Option<Account>`.
- Added performance indexes:
  - `idx_messages_account` on `messages(account_id)`
  - `idx_messages_mailbox` on `messages(mailbox_id, uid)`
  - `idx_smtp_queue_status` on `smtp_queue(status)`
- Combined `uid_next` and `modseq` lookups into a single optimized query in `insert_message`.

---

## 2. Verification & Test Results

All **28 automated tests** pass with **0 errors and 0 compiler warnings**:

| Test Suite | Total Tests | Passed | Failed |
|---|---|---|---|
| `fastrmail-auth` | 7 | 7 | 0 |
| `fastrmail-store` | 6 | 6 | 0 |
| `fastrmail-smtp` | 5 | 5 | 0 |
| `fastrmail-binary` | 4 | 4 | 0 |
| `fastrmail-core` | 3 | 3 | 0 |
| `fastrmail-imap` | 1 | 1 | 0 |
| `fastrmail-jmap` | 1 | 1 | 0 |
| `fastrmail-search` | 1 | 1 | 0 |
| **Total** | **28** | **28** | **0** |

### Key Phase 2 Tests
- `fastrmail-auth::tests::test_password_hash_and_verify` — Argon2id hash generation and positive/negative authentication.
- `fastrmail-auth::tests::test_dkim_key_generation` — 2048-bit RSA key generation and PEM parsing.
- `fastrmail-auth::tests::test_dkim_sign` — RFC 6376 DKIM signature header generation.
- `fastrmail-auth::tests::test_dmarc_evaluator_aligned` — Validates DKIM domain alignment pass and unaligned domain failure.
- `fastrmail-auth::tests::test_dmarc_evaluator_spf_aligned` — Validates SPF alignment pass.
- `fastrmail-smtp::tests::test_smtp_dmarc_policy_rejection` — Live test rejecting forged `example.com` email with `550 DMARC policy rejection`, ensuring blob is not saved.
- `fastrmail-smtp::tests::test_smtp_full_session_with_authentication_header` — Verified `Authentication-Results` header generation in stored email.
- `fastrmail-smtp::tests::test_outbound_engine_delivery` — End-to-end outbound delivery to mock SMTP server with queue clearance.
- `fastrmail-binary::tests::test_cli_generate_dkim` — CLI handler generates and records active DKIM key in SQLite.

---

## 3. Build Times

- `cargo check`: **8.27s** (clean dev check)
- `cargo test --workspace`: **30.43s** (all 28 tests passing)
- `cargo build --release`: **1m 26s** (full LTO build), instant cached execution
