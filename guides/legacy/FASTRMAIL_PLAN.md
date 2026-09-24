# FastrMail — Complete Technical Blueprint & Build Plan

**Project Name:** FastrMail
**Tagline:** The 100% free, open-source, single-binary mail & collaboration server.
**Author:** Atif Jubaer
**License:** Apache-2.0 + MIT (dual-licensed, commercially friendly)
**Repository:** github.com/fastrsoft/fastrmail

---

## 1. Executive Summary

**FastrMail** is a high-performance, single-binary email server written in Rust with a built-in modern webmail and admin UI (Svelte 5). 

Unlike legacy solutions (Postfix/Dovecot/Mailcow) that consume 3-4 GB RAM across 15+ containers, FastrMail runs in **< 100 MB RAM** in a single Docker container. 

This document serves as the absolute source of truth for an autonomous AI developer (e.g., Claude) to build the entire platform from scratch. 

---

## 2. Competitive Positioning (Brief)

FastrMail aims to displace legacy systems and proprietary variants by offering enterprise features for free.

| Feature | Legacy (Mailcow) | Competitors (e.g., Stalwart) | **FastrMail** |
|---------|------------------|------------------------------|-------------|
| **Language** | PHP/Python/C | Rust | **Rust** |
| **Footprint** | 15+ Containers | 1 Container | **1 Container (< 100MB RAM)** |
| **Built-in Webmail**| SOGo (Outdated) | ❌ None | **✅ Modern Svelte 5 SPA** |
| **Multi-Tenancy** | ❌ None | 💰 Paid | **✅ 100% Free** |
| **HTTP Send API** | ❌ None | ❌ None | **✅ Free (Resend compatible)** |
| **License** | GPL-3.0 | AGPL-3.0 | **Apache-2.0 / MIT** |

---

## 3. Technology Stack

*   **Core Backend**: Rust (tokio, axum, rustls, serde)
*   **Mail Protocol Parsers**: `smtp-proto`, `imap-proto`, `mail-parser`, `mail-builder`
*   **Security & Deliverability**: `mail-auth` (DKIM/SPF/DMARC/ARC), `hickory-dns`
*   **Storage**: `rusqlite` (SQLite default for metadata), `redb` (indexing), Filesystem/S3 (blobs)
*   **Search Engine**: `tantivy` (embedded pure-Rust full-text search)
*   **Frontend (Admin + Webmail)**: Svelte 5 + Vite + TailwindCSS + shadcn-svelte
*   **Documentation Site**: VitePress (Vue-based, highly optimized for Vercel hosting)

---

## 4. Workspace & Directory Structure

The project uses a Rust Cargo Workspace to keep boundaries clean.

```text
fastrmail/
├── Cargo.toml                      # Workspace member definitions
├── docker-compose.yml              # VPS / Dokploy deployment
├── Dockerfile                      # Multi-stage production build
├── .env.example                    
│
├── crates/                         # RUST BACKEND
│   ├── fastrmail-binary/           # Entry point (`src/main.rs`)
│   │
│   ├── fastrmail-smtp/             # SMTP Listener & Outbound Delivery Queue
│   ├── fastrmail-imap/             # IMAP4rev2 Server & State tracking
│   ├── fastrmail-jmap/             # JMAP JSON API (RFC 8620/8621)
│   │
│   ├── fastrmail-core/             # Shared Types, Config, Sieve execution
│   ├── fastrmail-auth/             # DKIM/SPF/DMARC/ARC + User Auth (JWT/OAuth)
│   ├── fastrmail-store/            # SQLite/Postgres schemas + S3 Blobs
│   └── fastrmail-search/           # Tantivy indexing engine
│
├── web/                            # FRONTEND (Svelte 5)
│   ├── admin/                      # Admin Dashboard UI
│   └── webmail/                    # End-user Inbox UI
│
└── docs/                           # DOCUMENTATION (VitePress)
    ├── package.json
    ├── .vitepress/
    │   └── config.ts               # Vercel/Vitepress router config
    └── src/
        ├── index.md                # Landing page
        ├── installation/
        ├── configuration/
        └── api-reference/
```

---

## 5. Database Schema (SQLite Default)

The system relies on a relational model optimized for multi-tenancy. 

```sql
-- 1. TENANCY & USERS
CREATE TABLE tenants (
    id TEXT PRIMARY KEY,
    domain TEXT UNIQUE NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE accounts (
    id TEXT PRIMARY KEY,
    tenant_id TEXT REFERENCES tenants(id),
    username TEXT NOT NULL,
    password_hash TEXT NOT NULL,
    quota_bytes BIGINT DEFAULT 10737418240, -- 10GB
    UNIQUE(tenant_id, username)
);

-- 2. DOMAIN AUTHENTICATION
CREATE TABLE dkim_keys (
    id TEXT PRIMARY KEY,
    tenant_id TEXT REFERENCES tenants(id),
    selector TEXT NOT NULL,
    private_key_pem TEXT NOT NULL,
    is_active BOOLEAN DEFAULT false
);

-- 3. MAILBOXES (Folders)
CREATE TABLE mailboxes (
    id TEXT PRIMARY KEY,
    account_id TEXT REFERENCES accounts(id),
    name TEXT NOT NULL,
    parent_id TEXT,
    uid_validity INTEGER NOT NULL,
    uid_next INTEGER DEFAULT 1,
    modseq BIGINT DEFAULT 1
);

-- 4. MESSAGES & BLOBS
-- Message blobs (bodies/attachments) are stored on disk/S3. Only metadata is in SQLite.
CREATE TABLE messages (
    id TEXT PRIMARY KEY,
    mailbox_id TEXT REFERENCES mailboxes(id),
    account_id TEXT REFERENCES accounts(id),
    uid INTEGER NOT NULL,
    modseq BIGINT NOT NULL,
    blob_id TEXT NOT NULL,         -- Reference to S3/Disk filename
    size_bytes INTEGER NOT NULL,
    parsed_subject TEXT,
    parsed_from TEXT,
    parsed_to TEXT,
    internal_date DATETIME NOT NULL,
    flags JSON,                    -- '["\\Seen", "\\Flagged"]'
    UNIQUE(mailbox_id, uid)
);

-- 5. OUTBOUND QUEUE
CREATE TABLE smtp_queue (
    id TEXT PRIMARY KEY,
    tenant_id TEXT REFERENCES tenants(id),
    raw_blob_id TEXT NOT NULL,
    sender TEXT NOT NULL,
    recipient TEXT NOT NULL,
    status TEXT DEFAULT 'pending', -- pending, retrying, failed
    next_retry_at DATETIME,
    retry_count INTEGER DEFAULT 0
);
```

---

## 6. Frontend UI Architecture (Svelte 5 + Shadcn)

**Why Svelte 5?** Svelte 5 compiles directly to vanilla JavaScript. It is significantly faster and more lightweight than React. The UI components will utilize `shadcn-svelte` (similar to ChatCN/shadcn-ui) for extremely clean, accessible, and modern styling.

### 6.1 Webmail Inbox SPA (`web/webmail`)
*   **Virtualization**: Uses `svelte-virtual-list` to render mailboxes with 100k+ emails smoothly.
*   **Composer**: Integrate `tiptap` for a Notion-like rich text email editing experience.
*   **State Management**: Svelte 5 Runes (`$state`, `$derived`) for zero-overhead reactivity.
*   **Delivery**: Bundled by Vite into a static `dist/` folder, which is embedded into the Rust binary using `include_dir!()` to serve directly from RAM.

### 6.2 Admin Control Panel (`web/admin`)
*   **Dashboard**: Real-time charts (using Chart.js/Apache ECharts) for SMTP queues, bandwidth, and blocked spam.
*   **Domain Management**: Built-in wizard that generates DKIM keys and tells the user exactly what TXT/MX records to add to Cloudflare/Vercel.

---

## 7. HTTP API (Resend & Postmark Alternative)

FastrMail acts as a full replacement for paid transactional email APIs.

**Endpoint:** `POST /api/v1/email/send`
**Auth:** Bearer Token (API Key)

```json
// Request Payload (JSON)
{
  "from": "Marketing <marketing@fastrmail.example.com>",
  "to": ["user@example.com"],
  "subject": "Welcome to FastrMail",
  "html": "<strong>Lightning fast email.</strong>",
  "text": "Lightning fast email.",
  "track_opens": true,
  "track_links": true
}

// 200 OK Response
{
  "success": true,
  "message_id": "msg_8f73b2c1d9",
  "status": "queued"
}
```

---

## 8. Documentation Site (Vercel-Hosted)

A high-quality open-source project requires tier-1 documentation. 

**Framework:** **VitePress** (Vue 3 based, standard for modern documentation).
**Hosting:** Specifically configured to auto-deploy to **Vercel** via GitHub Actions.

### Docs Structure:
1.  **Introduction**: What is FastrMail, Architecture Diagram.
2.  **Deployment**: 
    *   1-Click Dokploy & Coolify templates.
    *   Docker Compose examples.
    *   Port 25 block workarounds for AWS/Oracle.
3.  **Administrator Guide**: Setting up domains, DNS setup (SPF/DKIM/DMARC).
4.  **API Reference**: Full OpenAPI specs for the `/api/v1/email/send` endpoints.

---

## 9. Autonomous AI Developer: Execution Roadmap

*(Instructions for Claude/Claude Code)*

**Phase 1: Repo Setup & Core TCP**
1. Initialize the Cargo Workspace and create the crates listed in Section 4.
2. Implement `fastrmail-binary` to spawn tokio TCP listeners on Port 25 (SMTP) and Port 8080 (HTTP API).
3. Connect `rusqlite` and implement the schema in Section 5.

**Phase 2: SMTP Inbound & Storage**
1. Use the `smtp-proto` crate to build the SMTP state machine (`EHLO` -> `MAIL FROM` -> `RCPT TO` -> `DATA`).
2. Integrate `mail-auth` to automatically verify SPF, DKIM, and DMARC on inbound messages.
3. Save raw email byte blobs to disk, and index metadata to SQLite.

**Phase 3: IMAP4rev2 Server**
1. Use `imap-proto` to handle authentication, `LIST`, `SELECT`, and `FETCH` commands.
2. Implement UID mapping and folder synchronization logic.

**Phase 4: Svelte Frontend & Admin API**
1. Initialize `web/admin` and `web/webmail` using Svelte 5 + Tailwind.
2. Build the Axum server to serve the API routes and the embedded Svelte static files.
3. Build the `tiptap` composer and the virtualized inbox UI.

**Phase 5: Outbound MTAs & Spam Filtering**
1. Implement outbound SMTP queue processing (DNS MX lookups via `hickory-dns`).
2. Build statistical spam blocking and greylisting.
3. Complete the Documentation (`docs/` folder) with VitePress.

---

## 10. Deployment Artifacts

**Dockerfile**
```dockerfile
# Stage 1: Build Svelte Frontends
FROM node:22-alpine as frontend
WORKDIR /web
COPY web/ .
RUN cd webmail && npm ci && npm run build
RUN cd admin && npm ci && npm run build

# Stage 2: Build Rust Binary
FROM rust:1.81-alpine as backend
RUN apk add --no-cache musl-dev pkgconfig openssl-dev
WORKDIR /app
COPY . .
# Frontend assets are copied in so include_dir!() embeds them during compile
COPY --from=frontend /web/webmail/dist ./web/webmail/dist
COPY --from=frontend /web/admin/dist ./web/admin/dist
RUN cargo build --release

# Stage 3: Minimal Runtime
FROM alpine:3.19
RUN apk add --no-cache ca-certificates
COPY --from=backend /app/target/release/fastrmail /usr/local/bin/fastrmail

EXPOSE 25 587 993 8080
VOLUME ["/var/lib/fastrmail"]
CMD ["fastrmail"]
```
