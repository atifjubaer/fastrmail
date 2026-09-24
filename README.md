# FastrMail ⚡

> **High-Performance, Single-Binary, Self-Hosted Email Server in Rust**  
> *Native IMAP4rev2, JMAP, SMTP, Tantivy Full-Text Search, SpamGuard, and Embedded Svelte 5 Web Clients.*

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.81%2B-orange.svg)](https://www.rust-lang.org/)
[![Tests](https://img.shields.io/badge/Tests-47%2F47%20Passed-brightgreen.svg)]()
[![Docker Ready](https://img.shields.io/badge/Docker-Ready-2496ED.svg)](https://www.docker.com/)
[![Memory Idle](https://img.shields.io/badge/Idle%20Memory-%3C35MB-success.svg)]()

---

## Overview

**FastrMail** is a modern, memory-safe, ultra-lightweight open-source mail server designed to replace complex legacy email stacks (Postfix + Dovecot + SpamAssassin + Rspamd + Roundcube) with a **single, zero-dependency binary**.

Written in pure asynchronous Rust (Tokio & Axum), FastrMail boots in milliseconds, consumes less than 35MB of RAM at idle, and natively implements modern protocols including **JMAP (RFC 8620 / 8621)** and **IMAP4rev2 (RFC 9051)** with an embedded **Tantivy** search engine and built-in **Svelte 5** web applications.

---

## Feature Comparison Matrix

| Feature | **FastrMail** | **Stalwart** | **Mailcow** | **Postfix + Dovecot** |
|:--------|:--------------|:-------------|:------------|:----------------------|
| **Deployment Footprint** | **1 Single Binary** | 1 Binary | 15+ Docker Containers | 8+ Daemons & Packages |
| **Idle Memory Usage** | **< 35 MB** | ~60 MB | > 3.5 GB | ~250 MB |
| **Implementation Language** | **Rust 2021** | Rust | PHP / Python / C | C / Perl / Shell |
| **Inbound SMTP (RFC 5321)** | ✅ Built-in Async | ✅ Built-in | ✅ Postfix | ✅ Postfix |
| **Outbound MTA + NDR Bounces** | ✅ Built-in Async | ✅ Built-in | ✅ Postfix | ✅ Postfix |
| **IMAP4rev2 (RFC 9051)** | ✅ Native Async | ✅ Native Async | ⚠️ Dovecot (v1) | ⚠️ Dovecot (v1) |
| **JMAP (RFC 8620 / RFC 8621)** | ✅ Native JSON | ✅ Native JSON | ❌ None | ❌ None |
| **Full-Text Search** | ✅ Tantivy Embedded | ✅ Tantivy | ⚠️ External Solr/FTS | ⚠️ External Lucene/Solr |
| **SpamGuard (DNSBL & Greylist)**| ✅ Built-in Async | ✅ Sieve / DNSBL | ⚠️ Rspamd | ⚠️ SpamAssassin |
| **Embedded Webmail Client** | ✅ Svelte 5 Runes | ❌ External | ⚠️ SOGo / Roundcube | ❌ External |
| **Storage Engine** | ✅ SQLite WAL Mode | SQLite / RocksDB | MySQL / MariaDB | Maildir / MySQL |

---

## System Architecture

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

## Key Capabilities

- **Inbound & Outbound SMTP (RFC 5321)**:
  - Multi-threaded Tokio TCP listener on `:2525` (mapped to `:25`).
  - Cryptographic SPF, DKIM (RFC 6376), and DMARC (RFC 7489) verification with `p=reject` enforcement.
  - Background Outbound MTA with asynchronous DNS MX resolution and STARTTLS delivery.
  - Exponential backoff retry queue (`5m -> 10m -> 20m -> 40m -> 80m`).
  - Automated RFC 3464 Non-Delivery Report (NDR) bounce messages delivered to sender `INBOX`.
- **SpamGuard Anti-Spam Matrix**:
  - Concurrent multi-zone DNSBL lookups (`zen.spamhaus.org`, `b.barracudacentral.org`) with RFC 5782 error-code filtering (`127.0.0.x`) to prevent false positives from open resolvers.
  - Dynamic Greylisting tracking `(sender_ip, sender_email, recipient_email)` with 5-minute retry verification and auto-whitelisting.
- **IMAP4rev2 Engine (RFC 9051 / RFC 3501)**:
  - Stateful session state machine on `:1143` (mapped to `:143`).
  - Support for `CAPABILITY`, `LOGIN`, `SELECT`, `LIST`, `FETCH`, `STORE`, `EXPUNGE`, `NOOP`, `LOGOUT`.
  - Atomically sequenced UIDs and transactional MODSEQ synchronization.
- **JMAP Engine (RFC 8620 / RFC 8621)**:
  - Modern JSON-over-HTTP email protocol on `:8080`.
  - Session discovery at `GET /jmap/session` and `/.well-known/jmap`.
  - Methods: `Core/echo`, `Mailbox/get`, `Email/query`, `Email/get`, `Email/set`.
- **Tantivy Embedded Search**:
  - Real-time Lucene-grade indexing across `from`, `to`, `subject`, and message `body`.
  - Sub-millisecond queries with strict per-tenant index isolation.
- **Embedded Svelte 5 SPAs**:
  - Webmail client mounted at `/` with message reading, folder navigation, and drafting.
  - Admin dashboard mounted at `/admin` for domain provisioning, DKIM management, account provisioning, and live queue monitoring.

---

## ⚡ 2-Minute Quick Start

### Option 1: Docker Compose (Recommended)

```bash
# 1. Clone repository
git clone https://github.com/atifjubaer/fastrmail.git
cd fastrmail

# 2. Copy environment template
cp .env.example .env

# 3. Start FastrMail
docker compose up -d
```

Open `http://localhost:8080` for Webmail or `http://localhost:8080/admin` for the Admin Dashboard.

### Option 2: Native Rust Binary

```bash
# 1. Build release binary
cargo build --release -p fastrmail-binary

# 2. Run FastrMail
./target/release/fastrmail
```

---

## DKIM Key Generation & DNS Setup

Generate a 2048-bit RSA key pair and format the DNS TXT record in one command:

```bash
./target/release/fastrmail --generate-dkim example.com
```

Output:
```text
=== FastrMail DKIM Configuration for example.com ===
Selector: default
Record Type: TXT
Host / Name: default._domainkey.example.com
Value:
v=DKIM1; k=rsa; p=MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEA...
====================================================
```

Publish the TXT record to your DNS provider alongside your `MX`, `SPF`, and `DMARC` records.

---

## REST API Overview

FastrMail provides REST endpoints on port `8080`:

| Endpoint | Method | Description |
|:---------|:-------|:------------|
| `/api/health` | `GET` | Health check for Docker / load balancers |
| `/api/send` | `POST` | Transactional email dispatch |
| `/api/auth/login` | `POST` | Authenticate user (Argon2id) |
| `/api/mail/inbox` | `GET` | List inbox messages |
| `/api/admin/domains` | `GET` / `POST` | Manage domains and DKIM keys |
| `/api/admin/accounts` | `GET` / `POST` | Provision user email accounts |
| `/api/admin/stats` | `GET` | System queue & storage metrics |
| `/jmap/session` | `GET` | JMAP session discovery |
| `/jmap/api` | `POST` | JMAP JSON dispatcher |

---

## Running Workspace Tests

FastrMail maintains a comprehensive test suite across all 8 crates:

```bash
cargo test --workspace
```
*47 passing unit and integration tests covering SMTP, IMAP, JMAP, Tantivy, DNSBL, Greylisting, DKIM/SPF/DMARC, and REST APIs.*

---

## Documentation Site

FastrMail includes a full VitePress documentation site in `docs/`:

```bash
cd docs
npm install
npm run docs:dev
```

Build for production:
```bash
npm run docs:build
```

---

## License

FastrMail is open-source software licensed under the [MIT License](LICENSE).
