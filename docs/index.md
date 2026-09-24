---
layout: home

hero:
  name: "FastrMail"
  text: "Next-Gen Rust Mail Engine"
  tagline: "Single-binary, memory-safe, ultra-low resource email server in Rust with native IMAP4rev2, JMAP, Tantivy full-text search, SpamGuard, and embedded Svelte 5 webmail."
  actions:
    - theme: brand
      text: Get Started
      link: /guide/getting-started
    - theme: alt
      text: System Architecture
      link: /guide/architecture
    - theme: alt
      text: GitHub Repo
      link: https://github.com/atifjubaer/fastrmail

features:
  - icon: 🚀
    title: Single-Binary Simplicity
    details: Zero Postfix, zero Dovecot, zero Redis, zero Python. One compiled binary runs SMTP, IMAP, JMAP, FTS, REST APIs, and embedded frontends.
  - icon: 🛡️
    title: SpamGuard Defense Layer
    details: Multi-zone DNSBL lookups (Spamhaus, Barracuda), RFC greylisting with 5-minute tracking, and cryptographic SPF/DKIM/DMARC verification.
  - icon: ⚡
    title: JMAP & IMAP4rev2 Standards
    details: Full support for modern JSON-over-HTTP email (RFC 8620 / 8621) and next-generation IMAP4rev2 (RFC 9051) with transactional MODSEQ synchronization.
  - icon: 🔍
    title: Tantivy Full-Text Search
    details: Embedded Lucene-grade search engine indexing sender, recipient, subject, and body in real-time with strict per-tenant index isolation.
  - icon: 📬
    title: Outbound MTA & Bounce Engine
    details: Automated MX resolution, opportunistic STARTTLS delivery, DKIM signing, exponential backoff retries, and automatic NDR bounce delivery.
  - icon: 🎨
    title: Embedded Svelte 5 SPAs
    details: Elegant Webmail reader/composer and Admin tenant management dashboard powered by Svelte 5 runes and served directly via Axum.
---

## Feature Comparison Matrix

| Feature | **FastrMail** | **Stalwart** | **Mailcow** | **Postfix + Dovecot** |
|:--------|:--------------|:-------------|:------------|:----------------------|
| **Binary Packaging** | **1 Single Binary** | 1 Binary | 15+ Docker Containers | 8+ System Packages |
| **Memory Footprint (Idle)** | **< 35 MB** | ~60 MB | > 3.5 GB | ~250 MB |
| **Core Language** | **Rust 2021** | Rust | PHP / Python / C | C / Perl / Shell |
| **Inbound SMTP (RFC 5321)** | ✅ Built-in | ✅ Built-in | ✅ Postfix | ✅ Postfix |
| **Outbound MTA + NDR Bounces** | ✅ Built-in | ✅ Built-in | ✅ Postfix | ✅ Postfix |
| **IMAP4rev2 (RFC 9051)** | ✅ Native Async | ✅ Native Async | ⚠️ Dovecot (v1) | ⚠️ Dovecot (v1) |
| **JMAP (RFC 8620 / RFC 8621)** | ✅ Native JSON | ✅ Native JSON | ❌ None | ❌ None |
| **Full-Text Search** | ✅ Tantivy Embedded | ✅ Tantivy | ⚠️ Solr / Dovecot FTS | ⚠️ External Lucene/Solr |
| **Spam Defense** | ✅ DNSBL + Greylist | ✅ Sieve / DNSBL | ⚠️ Rspamd | ⚠️ SpamAssassin |
| **Embedded Webmail** | ✅ Svelte 5 Runes | ❌ External | ⚠️ SOGo / Roundcube | ❌ External |
| **Storage Engine** | ✅ SQLite WAL Mode | SQLite / RocksDB | MySQL / MariaDB | Maildir / MySQL |
