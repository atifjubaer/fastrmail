<div align="center">

  <img src="https://raw.githubusercontent.com/atifjubaer/fastrmail/main/docs/public/logo.svg" width="96" height="96" alt="FastrMail Logo" onerror="this.style.display='none'" />

  # FastrMail

  ### ⚡ The Open-Source, Enterprise-Grade Mail Server in Rust

  <p>
    A high-performance, single-binary email engine replacing Postfix, Dovecot, Rspamd, SpamAssassin, and Roundcube.<br>
    <strong>&lt; 35 MB idle RAM • 100% Rust • Zero external dependencies • Zero container sprawl</strong>
  </p>

  <p>
    <a href="https://github.com/atifjubaer/fastrmail/releases/tag/v1.0.0"><img src="https://img.shields.io/badge/release-v1.0.0-blue.svg?style=flat-square&logo=git" alt="Release v1.0.0" /></a>
    <a href="https://github.com/atifjubaer/fastrmail"><img src="https://img.shields.io/badge/tests-52%2F52%20passing-brightgreen.svg?style=flat-square&logo=githubactions&logoColor=white" alt="Tests 52/52" /></a>
    <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/rust-1.81+-orange.svg?style=flat-square&logo=rust" alt="Rust 1.81+" /></a>
    <a href="https://www.docker.com/"><img src="https://img.shields.io/badge/docker-ready-blue.svg?style=flat-square&logo=docker" alt="Docker Ready" /></a>
    <a href="https://opensource.org/licenses/MIT"><img src="https://img.shields.io/badge/license-MIT-yellow.svg?style=flat-square" alt="MIT License" /></a>
    <img src="https://img.shields.io/badge/memory-%3C35MB%20idle-2ea44f.svg?style=flat-square" alt="Memory Footprint" />
  </p>

  <p>
    <a href="https://fastrmail.vercel.app"><strong>🌐 Website</strong></a> •
    <a href="https://fastrmail.vercel.app"><strong>📖 Documentation</strong></a> •
    <a href="#-enterprise-protocol-matrix"><strong>Protocols</strong></a> •
    <a href="#-feature-comparison-matrix"><strong>Comparison</strong></a> •
    <a href="#-quick-start"><strong>Quick Start</strong></a> •
    <a href="#-architecture"><strong>Architecture</strong></a>
  </p>

</div>

---

## 🌟 Overview

**FastrMail** is an all-in-one, modern mail server built from the ground up in memory-safe Rust. Traditional self-hosted email stacks require configuring 8–15 separate Linux packages or Docker containers (Postfix, Dovecot, Rspamd, ClamAV, Redis, MySQL, Nginx, Roundcube) that consume 3–4 GB of RAM.

FastrMail consolidates the entire stack into a **single, lightweight native binary** (<35MB RAM) that handles inbound delivery, authenticated submission, legacy POP3, next-gen IMAP4rev2, modern JMAP, Lucene-grade full-text search, multi-zone anti-spam filtering, and embedded Svelte 5 web applications.

---

## 🚀 Enterprise Protocol Matrix

| Protocol | Port | RFC Standard | Security / Features | Status |
|:---|:---:|:---|:---|:---:|
| **Inbound SMTP** | `25` / `2525` | RFC 5321 | Opportunistic STARTTLS, Greylisting, DNSBL, SPF/DMARC | ✅ Active |
| **Auth Submission** | `587` / `2526` | RFC 6409 | SASL `AUTH PLAIN` & `AUTH LOGIN`, Argon2id credential hashing | ✅ Active |
| **POP3 Server** | `110` / `1110` | RFC 1939 | Full 3-State FSM (`USER`, `PASS`, `STAT`, `LIST`, `RETR`, `DELE`, `UIDL`) | ✅ Active |
| **IMAP4rev2** | `143` / `1143` | RFC 9051 | Transactional `MODSEQ` sync, streaming body parser, IDLE push | ✅ Active |
| **JMAP Core & Mail** | `8080` | RFC 8620 / 8621 | Modern JSON-over-HTTP API, offline-first client synchronization | ✅ Active |
| **ManageSieve** | In-Memory | RFC 5228 | Inbound `DATA` evaluation: `Discard`, `Reject`, `FileInto`, `MarkRead` | ✅ Active |
| **SpamGuard** | Dynamic | RFC 5782 / 7208 | Multi-zone DNSBL (Spamhaus, Barracuda), 5-min Greylist, DKIM 2048 | ✅ Active |
| **Full-Text Search** | Embedded | Tantivy Engine | Sub-millisecond indexed search across headers, sender & body | ✅ Active |

---

## 📊 Feature Comparison Matrix

| Feature | **FastrMail** | **Stalwart** | **Mailcow** | **Postfix + Dovecot** |
|:---|:---:|:---:|:---:|:---:|
| **Binary Packaging** | **1 Single Binary** | 1 Binary | 15+ Docker Containers | 8+ System Packages |
| **Idle Memory Footprint** | **&lt; 35 MB** | ~60 MB | &gt; 3.5 GB | ~250 MB |
| **Implementation Language** | **Rust 2021** | Rust | PHP / Python / C | C / Perl / Shell |
| **Inbound SMTP (RFC 5321)** | ✅ Built-in | ✅ Built-in | ✅ Postfix | ✅ Postfix |
| **Authenticated Submission (RFC 6409)** | ✅ Port 587 | ✅ Port 587 | ✅ Postfix | ✅ Postfix |
| **POP3 Retrieval (RFC 1939)** | ✅ Port 110 | ✅ Port 110 | ✅ Dovecot | ✅ Dovecot |
| **IMAP4rev2 (RFC 9051)** | ✅ Native Async | ✅ Native Async | ⚠️ Dovecot (v1) | ⚠️ Dovecot (v1) |
| **JMAP (RFC 8620 / RFC 8621)** | ✅ Native JSON | ✅ Native JSON | ❌ None | ❌ None |
| **ManageSieve Rule Filtering** | ✅ In-Memory FSM | ✅ Sieve Engine | ⚠️ Dovecot Sieve | ⚠️ Sieve Plugin |
| **Full-Text Search** | ✅ Tantivy Embedded | ✅ Tantivy | ⚠️ Solr / Dovecot FTS | ⚠️ External Lucene/Solr |
| **Spam Defense** | ✅ DNSBL + Greylist | ✅ Sieve / DNSBL | ⚠️ Rspamd | ⚠️ SpamAssassin |
| **Embedded Webmail Client** | ✅ Svelte 5 Runes | ❌ External | ⚠️ SOGo / Roundcube | ❌ External |
| **Embedded Admin UI** | ✅ Svelte 5 Runes | ⚠️ Web UI | ✅ PHP Web UI | ❌ None / CLI |
| **Storage Architecture** | ✅ SQLite WAL Mode | SQLite / RocksDB | MySQL / MariaDB | Maildir / MySQL |
| **License** | **100% Free (MIT)** | Dual License | Free + Paid Support | Free (GPL/Custom) |

---

## 🏗️ Architecture

```
                                    ┌────────────────────────────────────────────────────────┐
                                    │                 FASTRMAIL UNIFIED BINARY               │
                                    │                                                        │
SMTP Inbound (25/2525) ────────────►│ ┌────────────────┐  ┌───────────────┐  ┌─────────────┐ │
                                    │ │ fastrmail-smtp │  │ fastrmail-pop3│  │fastrmail-imap│ │
SMTP Submission (587/2526) ────────►│ └───────┬────────┘  └───────┬───────┘  └──────┬──────┘ │
                                    │         │                   │                 │        │
POP3 Clients (110/1110) ───────────►│         ▼                   ▼                 ▼        │
                                    │ ┌────────────────────────────────────────────────────┐ │
IMAP Clients (143/1143) ───────────►│ │      SpamGuard & ManageSieve Filtering Pipeline    │ │
                                    │ └─────────────────────────┬──────────────────────────┘ │
Webmail & Admin HTTP (8080) ───────►│                           ▼                            │
                                    │ ┌────────────────────────────────────────────────────┐ │
JMAP API (8080 /jmap) ─────────────►│ │    fastrmail-store (SQLite WAL) + Tantivy FTS Engine│ │
                                    │ └────────────────────────────────────────────────────┘ │
Outbound MTA Queue ────────────────►│ ┌────────────────────────────────────────────────────┐ │
                                    │ │  DNS MX Resolver + STARTTLS + DKIM-2048 + Retries  │ │
                                    │ └────────────────────────────────────────────────────┘ │
                                    └────────────────────────────────────────────────────────┘
```

---

## 📦 Quick Start

### 1. Docker Compose (Recommended)

Clone the repository and spin up the complete stack with persistent storage:

```bash
git clone https://github.com/atifjubaer/fastrmail.git
cd fastrmail
cp .env.example .env
docker compose up -d
```

### 2. Dokploy & Coolify (One-Click)

Import the pre-configured deployment template directly into your Dokploy dashboard:

- **Dokploy Template**: [`dokploy-service-template.yaml`](dokploy-service-template.yaml)
- Pre-configured Traefik labels handle automatic TLS routing across SMTP, Submission, POP3, IMAP, and HTTP.

### 3. Native Cargo Build

```bash
# Build the optimized release binary
cargo build --release

# Run FastrMail directly
./target/release/fastrmail
```

---

## ⚙️ Configuration & Environment

Copy `.env.example` to `.env` to configure your domains, credentials, and ports:

```env
FASTRMAIL_PRIMARY_DOMAIN=example.com
FASTRMAIL_DB_PATH=./data/fastrmail.db
FASTRMAIL_DATA_DIR=./data
FASTRMAIL_INDEX_PATH=./data/search_index

# Port bindings
FASTRMAIL_BIND_SMTP=0.0.0.0:2525
FASTRMAIL_BIND_SUBMISSION=0.0.0.0:2526
FASTRMAIL_BIND_POP3=0.0.0.0:1110
FASTRMAIL_BIND_IMAP=0.0.0.0:1143
FASTRMAIL_BIND_HTTP=0.0.0.0:8080

# Security & Anti-Spam
FASTRMAIL_GREYLIST_DURATION=300
FASTRMAIL_MAX_MESSAGE_SIZE=26214400
```

---

## 🔒 DNS Setup Guide

For 100% email deliverability without landing in spam, configure these DNS records on your domain registrar:

| Type | Host | Value | Purpose |
|:---|:---|:---|:---|
| **MX** | `@` | `mail.example.com` (Priority `10`) | Directs incoming mail to FastrMail |
| **A** | `mail` | `YOUR_SERVER_IPV4` | Points mail subdomain to your server |
| **TXT** | `@` | `v=spf1 mx ~all` | Authorizes FastrMail server to send email |
| **TXT** | `_dmarc` | `v=DMARC1; p=quarantine; rua=mailto:dmarc@example.com` | DMARC policy enforcement |
| **TXT** | `default._domainkey` | `v=DKIM1; k=rsa; p=MIIBIjANBgkqh...` | Public 2048-bit DKIM key |

---

## 🧪 Verification & Test Suite

FastrMail enforces comprehensive automated test suites across all 9 crates:

```bash
cargo test --workspace
```

```
test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

---

## 🤝 Contributing

Contributions are welcome! Please check out the issues tab or submit pull requests.

1. Fork the Project
2. Create your Feature Branch (`git checkout -b feature/AmazingFeature`)
3. Commit your Changes (`git commit -m 'feat: Add AmazingFeature'`)
4. Push to the Branch (`git push origin feature/AmazingFeature`)
5. Open a Pull Request

---

## 📄 License

Distributed under the **MIT License**. See [`LICENSE`](LICENSE) for more information.

---

<div align="center">
  <p>Crafted with precision in Rust • Built by <a href="https://github.com/atifjubaer">Atif Jubaer</a> & the FastrMail Community</p>
</div>
