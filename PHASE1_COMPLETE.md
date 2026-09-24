# FastrMail — Phase 1 Complete

**Project:** FastrMail — The 100% free, open-source, single-binary mail & collaboration server  
**Author:** Atif Jubaer  
**Status:** Phase 1 (Project Scaffold + Core Systems) 100% Complete  

---

## 1. Crates Created & Architecture

All 8 crates have been created within the Cargo workspace with exact dependency versions:

| Crate | Path | Responsibility | Dependencies / Tech |
|---|---|---|---|
| `fastrmail-binary` | `crates/fastrmail-binary` | Main entry point, spawns SMTP + HTTP API servers, prints banner | `tokio`, `axum`, `tower-http`, `serde`, `serde_json`, `tracing`, `uuid` |
| `fastrmail-core` | `crates/fastrmail-core` | Shared domain models (`Tenant`, `Account`, `Mailbox`, `Message`, `QueueItem`, `DkimKey`, `Config`) | `serde`, `serde_json`, `uuid`, `chrono`, `tracing`, `anyhow` |
| `fastrmail-store` | `crates/fastrmail-store` | SQLite relational layer (`Mutex<rusqlite::Connection>`), WAL mode, foreign keys, schema initialization, full CRUD | `rusqlite` (bundled), `uuid`, `chrono`, `anyhow` |
| `fastrmail-smtp` | `crates/fastrmail-smtp` | Inbound SMTP server (tokio TCP), RFC 5321 state machine (`EHLO`, `MAIL FROM`, `RCPT TO`, `DATA`, `RSET`, `QUIT`), .eml disk storage, SQLite indexing | `tokio`, `uuid`, `chrono`, `anyhow` |
| `fastrmail-imap` | `crates/fastrmail-imap` | IMAP4rev2 server scaffold (Phase 3 foundation) | `tokio`, `tracing`, `anyhow` |
| `fastrmail-jmap` | `crates/fastrmail-jmap` | JMAP JSON API scaffold (RFC 8620/8621) | `tokio`, `serde`, `serde_json`, `anyhow` |
| `fastrmail-auth` | `crates/fastrmail-auth` | Authentication & verification service scaffold (Phase 2 foundation) | `serde`, `anyhow`, `tracing` |
| `fastrmail-search` | `crates/fastrmail-search` | Tantivy embedded full-text search engine scaffold | `tantivy`, `serde`, `anyhow` |

---

## 2. Frontend Scaffolding

Two Svelte 5 + TypeScript + Vite + TailwindCSS SPAs scaffolded and tested:

- **Admin Dashboard (`web/admin`)**: Svelte 5 with `@tailwindcss/vite`, builds to `web/admin/dist/`
- **Webmail Inbox (`web/webmail`)**: Svelte 5 with `@tailwindcss/vite`, builds to `web/webmail/dist/`

---

## 3. SQLite Database Schema (6 Tables)

1. `tenants`: Multi-tenant domain isolation (`id`, `domain`, `created_at`)
2. `accounts`: User accounts per tenant (`id`, `tenant_id`, `username`, `email`, `password_hash`, `quota_bytes`, `created_at`)
3. `dkim_keys`: Domain authentication keys (`id`, `tenant_id`, `selector`, `private_key_pem`, `is_active`)
4. `mailboxes`: IMAP/JMAP mailbox folders with UIDVALIDITY and MODSEQ tracking (`id`, `account_id`, `name`, `parent_id`, `uid_validity`, `uid_next`, `modseq`)
5. `messages`: Email message metadata linked to disk/S3 blob IDs (`id`, `mailbox_id`, `account_id`, `uid`, `modseq`, `blob_id`, `size_bytes`, `parsed_subject`, `parsed_from`, `parsed_to`, `internal_date`, `flags`)
6. `smtp_queue`: Outbound email queue (`id`, `tenant_id`, `raw_blob_id`, `sender`, `recipient`, `status`, `next_retry_at`, `retry_count`)

---

## 4. Endpoints & Protocols Available

### Inbound SMTP Server
- **Port:** `2525` (TCP)
- **Commands:** `EHLO`/`HELO`, `MAIL FROM`, `RCPT TO`, `DATA` (with dot-stuffing & RFC 5322 header parsing), `RSET`, `NOOP`, `QUIT`
- **Storage:** Saves raw email blobs to `data/blobs/{uuid}.eml` and records metadata into SQLite `messages`

### HTTP REST API Server
- **Port:** `8080` (HTTP)
- **Routes:**
  - `GET /api/health` — returns `{"status":"ok"}`
  - `POST /api/v1/email/send` — accepts JSON `{from, to, subject, html, text}`, writes blob, enqueues to `smtp_queue`, returns `{"success": true, "message_id": "...", "status": "queued"}`
  - `GET /api/v1/mailbox` — returns JSON array of messages for the account

---

## 5. Verification & Test Results

All 21 automated tests pass with **zero warnings and zero failures**:

| Test Suite | Total Tests | Passed | Failed |
|---|---|---|---|
| `fastrmail-store` | 7 | 7 | 0 |
| `fastrmail-smtp` | 4 | 4 | 0 |
| `fastrmail-binary` | 3 | 3 | 0 |
| `fastrmail-core` | 3 | 3 | 0 |
| `fastrmail-auth` | 1 | 1 | 0 |
| `fastrmail-imap` | 1 | 1 | 0 |
| `fastrmail-jmap` | 1 | 1 | 0 |
| `fastrmail-search` | 1 | 1 | 0 |
| **Total** | **21** | **21** | **0** |

### Test Breakdown
- `fastrmail-store::tests::test_init_schema` — verified all 6 tables exist in SQLite
- `fastrmail-store::tests::test_insert_and_get` — tenant -> account -> mailbox -> message insertion & retrieval
- `fastrmail-store::tests::test_multiple_messages_uid_increment` — verifies UID & MODSEQ auto-increment per mailbox
- `fastrmail-store::tests::test_smtp_queue` — enqueues and inspects pending items
- `fastrmail-store::tests::test_dkim_keys` — stores and retrieves PEM keys
- `fastrmail-store::tests::test_nonexistent_tenant` — returns None
- `fastrmail-store::tests::test_nonexistent_account` — returns None
- `fastrmail-smtp::tests::test_smtp_full_session` — runs live TCP SMTP connection, executes EHLO, MAIL FROM, RCPT TO, DATA, QUIT, verifies SQLite message insertion and .eml blob on disk
- `fastrmail-smtp::tests::test_smtp_protocol_errors` — tests 503 EHLO requirement and 500 unrecognized command
- `fastrmail-smtp::tests::test_parse_address` — parses angled bracket address formats
- `fastrmail-smtp::tests::test_parse_header` — extracts Subject, From, To
- `fastrmail-binary::tests::test_health_check` — asserts health check handler returns status ok
- `fastrmail-binary::tests::test_send_email` — verifies HTTP email submission enqueues item into database
- `fastrmail-binary::tests::test_get_mailbox_empty` — returns empty list for new mailbox

---

## 6. Build Times

- `cargo check`: **1.08s** (cached)
- `cargo test`: **0.22s** (cached)
- `cargo build --release` (LTO enabled, single codegen unit): **1m 06s** (full build), **0.23s** (cached)
- `npm run build` (`web/admin`): **88ms**
- `npm run build` (`web/webmail`): **89ms**

---

## 7. Zero Warning Guarantee

Every Rust file compiles cleanly under the Rust 2021 edition compiler with zero compiler warnings.
