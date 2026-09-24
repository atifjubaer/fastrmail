# Getting Started with FastrMail

FastrMail is a single-binary, production-grade, open-source email server written in Rust with native IMAP4rev2, JMAP, SMTP, Tantivy search, SpamGuard, and embedded Svelte 5 web clients.

## Key Features

- **Single Binary**: Built with Tokio and Axum. Requires no external daemons, no database servers, and no background queues.
- **RFC Compliance**:
  - Inbound & Outbound SMTP: RFC 5321
  - IMAP4rev2: RFC 9051 / RFC 3501
  - JMAP Mail & Core: RFC 8620 / RFC 8621
  - DKIM Signatures: RFC 6376
  - SPF Verification: RFC 7208
  - DMARC Policy Evaluation: RFC 7489
  - DNSBL Lookups: RFC 5782
- **Tantivy Full-Text Search**: Embedded sub-millisecond search engine across senders, subjects, bodies, and headers with per-tenant isolation.
- **SpamGuard**: Integrated multi-zone DNSBL lookups (Spamhaus, Barracuda) and 5-minute Greylisting deferrals.
- **Modern Web Interfaces**: Svelte 5 Webmail (`/`) and Admin Dashboard (`/admin`) bundled inside the binary.

---

## 2-Minute Quick Start (Docker Compose)

The fastest way to run FastrMail is using Docker Compose:

### 1. Clone the Repository

```bash
git clone https://github.com/atifjubaer/fastrmail.git
cd fastrmail
```

### 2. Copy the Environment Template

```bash
cp .env.example .env
```

### 3. Launch FastrMail

```bash
docker compose up -d
```

FastrMail starts and listens on:
- **Port 25 (SMTP)**: Inbound mail delivery
- **Port 143 (IMAP)**: IMAP4rev2 client connections (Thunderbird, Apple Mail)
- **Port 8080 (HTTP)**: Webmail (`http://localhost:8080`), Admin (`http://localhost:8080/admin`), and JMAP API (`http://localhost:8080/jmap/api`)

---

## Native Binary Quick Start

If running without Docker:

```bash
# Clone and build in release mode
git clone https://github.com/atifjubaer/fastrmail.git
cd fastrmail
cargo build --release -p fastrmail-binary

# Run the binary
./target/release/fastrmail
```

---

## Provisioning Your First Domain & DKIM Key

Generate a DKIM 2048-bit RSA key pair and DNS record with a single command:

```bash
./target/release/fastrmail --generate-dkim mail.example.com
```

Output:
```text
=== FastrMail DKIM Configuration for mail.example.com ===
Selector: default
Record Type: TXT
Host / Name: default._domainkey.mail.example.com
Value:
v=DKIM1; k=rsa; p=MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEA...
======================================================
```
Add the TXT record to your DNS zone file and outgoing emails will be automatically signed.
