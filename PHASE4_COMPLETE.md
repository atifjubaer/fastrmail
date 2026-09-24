# FastrMail — Phase 4 Complete

**Project:** FastrMail — The 100% free, open-source, single-binary mail & collaboration server  
**Author:** Atif Jubaer  
**Status:** Phase 4 (Tantivy Full-Text Search Engine & JMAP Protocol RFC 8620/8621) 100% Complete  

---

## 1. What Was Built in Phase 4

### 1.1 Tantivy Full-Text Search Engine (`fastrmail-search`)
- **Tantivy 0.22 Schema Design**:
  - `message_id`: `STRING | STORED` — Unique FastrMail message ID.
  - `account_id`: `STRING` — Fast tenant/account isolation term index.
  - `mailbox_id`: `STRING` — Fast mailbox filtering term index.
  - `subject`: `TEXT | STORED` — Tokenized and indexed subject line with position offsets.
  - `body`: `TEXT` — Tokenized and indexed plain text email content.
  - `from`: `TEXT | STORED` — Sender tokenized field.
  - `to`: `TEXT | STORED` — Recipient tokenized field.
- **Engine Operations**:
  - `SearchEngine::new(path)`: Persistent search engine backing index on disk using `MmapDirectory`.
  - `SearchEngine::new_in_ram()`: Fast in-memory search engine for zero-I/O automated testing.
  - `index_message(msg, body)`: Constructs Tantivy document, indexes all fields, and commits index writer.
  - `search(query_str, account_id, limit)`: Parses user search query with `QueryParser` and executes an intersection query with `Term::from_field_text(account_id)` guaranteeing strict multi-tenant data isolation. Returns matching `message_id`s ranked by relevance.
  - `delete_message(message_id)`: Removes documents matching `message_id` from the search index.

### 1.2 SMTP Inbound Search Integration (`fastrmail-smtp`)
- `SmtpServer` upgraded with `.with_search_engine(Arc<SearchEngine>)`.
- During the inbound `DATA` phase:
  1. Incoming MIME email is parsed using `mail-parser`.
  2. Plain text body is extracted from the MIME tree (or fallback HTML-to-text).
  3. Message is inserted into SQLite and written as an `.eml` blob to disk.
  4. Search engine indexes the message immediately with parsed metadata and body.

### 1.3 JMAP Protocol Implementation (`fastrmail-jmap`)
- **RFC 8620 (JMAP Core)**:
  - `GET /jmap/session`: RFC 8620 Session Resource discovery endpoint providing:
    - Advertised capabilities: `urn:ietf:params:jmap:core` and `urn:ietf:params:jmap:mail`.
    - User accounts map, primary account ID, user display email.
    - Endpoints: `apiUrl: "/jmap/api"`, `downloadUrl`, `uploadUrl`.
    - `state` token for client cache synchronization.
  - `POST /jmap/api`: Standard batch request dispatch processor:
    - Parses JSON requests with `using` capabilities and `methodCalls`.
    - Dispatches method calls in sequence with client call ID correlation.
    - Handles `Core/echo`: Returns arguments untouched for protocol latency & connectivity checks.
- **RFC 8621 (JMAP Mail)**:
  - `Mailbox/get`: Returns list of account mailboxes with standard roles (`inbox`, `sent`, `trash`, `drafts`, `archive`), unread email counts, and total message counts.
  - `Email/query`:
    - Full-text search integration: If `filter.text` is provided, queries Tantivy `SearchEngine` for full-text matches across subject, sender, and email body.
    - Supports `filter.inMailbox` for folder-specific queries.
    - Returns ordered `ids` array with `position` and `total` count.
  - `Email/get`: Retrieves detailed email representations:
    - Fields: `id`, `blobId`, `threadId`, `mailboxIds`, `keywords` (`$seen`, `$flagged`, `$draft`, `$answered`), `size`, `receivedAt`, `from`, `to`, `subject`, `preview`.
  - `Email/set`: Handles updates and deletions:
    - Updates: Translates JMAP keywords (`$seen`, `$flagged`) to standard IMAP flags in SQLite.
    - Destroys: Marks messages deleted and expunges them from disk and database.
- `build_jmap_router(state: JmapState) -> Router`: Standalone composable Axum router.

### 1.4 Single-Binary Integration (`fastrmail-binary`)
- Tantivy search index initialized at `data/index`.
- Inbound SMTP server configured with Tantivy indexing.
- JMAP routes (`/jmap/session`, `/jmap/api`) mounted directly into the main Axum HTTP server at `:8080`.
- Full TCP-level integration tests verifying JMAP discovery and Axum routing.

---

## 2. Verification & Test Results

All **41 automated tests** pass with **0 errors and 0 compiler warnings**:

| Test Suite | Total Tests | Passed | Failed |
|---|---|---|---|
| `fastrmail-store` | 8 | 8 | 0 |
| `fastrmail-auth` | 7 | 7 | 0 |
| `fastrmail-smtp` | 6 | 6 | 0 |
| `fastrmail-binary` | 6 | 6 | 0 |
| `fastrmail-imap` | 5 | 5 | 0 |
| `fastrmail-jmap` | 5 | 5 | 0 |
| `fastrmail-core` | 3 | 3 | 0 |
| `fastrmail-search` | 1 | 1 | 0 |
| **Total** | **41** | **41** | **0** |

### Key Phase 4 Tests
- `fastrmail-search::tests::test_tantivy_index_and_search`: Indexes sample email and verifies search queries matching subject and body text with account isolation.
- `fastrmail-smtp::tests::test_smtp_inbound_with_tantivy_indexing`: Simulates inbound SMTP email delivery and verifies Tantivy full-text search indexing on arrival.
- `fastrmail-jmap::tests::test_jmap_session_discovery`: Tests `/jmap/session` RFC 8620 endpoint with Bearer auth and validates returned capabilities.
- `fastrmail-jmap::tests::test_jmap_core_echo`: Validates `Core/echo` method call and client call ID preservation.
- `fastrmail-jmap::tests::test_jmap_mailbox_get`: Validates `Mailbox/get` returning INBOX with counts.
- `fastrmail-jmap::tests::test_jmap_email_query_and_get`: Validates `Email/query` with text search hitting Tantivy, followed by `Email/get`.
- `fastrmail-jmap::tests::test_jmap_email_set`: Validates `Email/set` marking `$seen: true` and persisting changes.
- `fastrmail-binary::tests::test_jmap_router_session_integration`: Full-stack TCP socket test validating HTTP GET `/jmap/session` over live Axum server.

---

## 3. Active Service Matrix

| Service | Protocol | Port / Path | Description |
|---|---|---|---|
| SMTP Inbound | SMTP / RFC 5321 | `:2525` | Inbound email listener with SPF, DKIM, DMARC & Tantivy indexing |
| IMAP4rev2 | IMAP / RFC 9051 | `:1143` | Full IMAP server supporting standard desktop/mobile mail clients |
| HTTP REST API | HTTP / REST | `:8080/api/v1` | Webmail, Admin, and Transactional sending endpoints |
| JMAP Discovery | JMAP / RFC 8620 | `:8080/jmap/session` | Standard JMAP session resource discovery |
| JMAP API | JMAP / RFC 8621 | `:8080/jmap/api` | Fast JSON Mail API for next-generation mail clients |
| Full-Text Search | Tantivy FTS | `data/index` | Embedded full-text search index across all email subjects and bodies |
| Outbound Delivery | SMTP Outbound | Background Worker | Automatic DNS MX lookups, DKIM signing & exponential retry queue |
