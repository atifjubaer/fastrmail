# Architecture & Design

FastrMail is engineered as a zero-dependency, modular Rust workspace designed for maximum performance, safety, and operational simplicity.

```
                    ┌─────────────────────────────────────────┐
                    │               Client Layer              │
                    │  SMTP Clients │ IMAP Clients │ Web / UI │
                    └──────┬──────────────┬──────────────┬────┘
                           │              │              │
                   Port 25 │     Port 143 │    Port 8080 │
                           ▼              ▼              ▼
                 ┌──────────────┬──────────────┬────────────────┐
                 │ fastrmail-   │ fastrmail-   │ fastrmail-     │
                 │ smtp         │ imap         │ binary (Axum)  │
                 │ (RFC 5321)   │ (RFC 9051)   │ & JMAP Engine  │
                 └──────┬───────┴──────┬───────┴────────┬───────┘
                        │              │                │
                        ▼              ▼                ▼
                 ┌──────────────────────────────────────────────┐
                 │          Security & Verification             │
                 │     fastrmail-auth (SpamGuard, DKIM, SPF)    │
                 └──────────────────────┬───────────────────────┘
                                        │
                        ┌───────────────┴───────────────┐
                        ▼                               ▼
                 ┌──────────────┐                ┌──────────────┐
                 │ fastrmail-   │                │ fastrmail-   │
                 │ store        │                │ search       │
                 │ (SQLite WAL) │                │ (Tantivy FTS)│
                 └──────────────┘                └──────────────┘
```

---

## Workspace Crates

| Crate | Responsibility | Key Technologies |
|:------|:---------------|:-----------------|
| `fastrmail-core` | Shared domain primitives, models, configuration, tenant representations | Rust structs, Serde |
| `fastrmail-store` | Database transactions, UIDs, MODSEQ tracking, Greylist persistence | SQLite with WAL mode, Rusqlite |
| `fastrmail-auth` | DKIM sign/verify, SPF, DMARC, DNSBL lookups, Argon2id passwords | `mail-auth`, `argon2`, `hickory-resolver` |
| `fastrmail-smtp` | Inbound RFC 5321 server, outbound MTA queue with MX lookup & NDR bounce generator | `tokio-net`, `lettre`, `mail-parser` |
| `fastrmail-imap` | RFC 9051 IMAP4rev2 protocol engine and session state machine | Native async TCP parser |
| `fastrmail-jmap` | RFC 8620/8621 JMAP session discovery, Mailbox/get, Email/query/get/set | Axum REST router, Serde JSON |
| `fastrmail-search` | Embedded multi-tenant Lucene-like full-text search index | Tantivy 0.22 |
| `fastrmail-binary` | CLI coordinator, REST APIs, static Svelte frontend serving | Axum 0.7, Tower-HTTP |

---

## Memory & Concurrency Model

- **Tokio Multi-Threaded Runtime**: Non-blocking asynchronous I/O across all TCP listeners (SMTP, IMAP, HTTP).
- **SQLite WAL Mode**: Multiple concurrent readers with single serialized writer using transactional UID allocation and MODSEQ increments.
- **Embedded Tantivy Index**: Schema-driven Lucene indexing executed asynchronously upon incoming message ingestion without blocking SMTP connections.
- **Zero Heavy Runtime Dependencies**: Requires no JVM, no external search daemon, no Redis cache, and no C mail stack.
