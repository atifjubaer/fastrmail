# FastrMail — Complete Setup & Build Guide

**For: Atif Jubaer**
**Machine: Windows PC (assumed) + Antergo / Claude Code**
**Starting From: Absolutely nothing installed**

---

## PART A: Install Everything On Your PC

### Step 1: Install Rust

Open PowerShell (or Terminal) and paste:

```powershell
# Download and run the official Rust installer
winget install Rustlang.Rustup
```

If `winget` doesn't work, go to https://rustup.rs and download the installer manually.

After install, close and reopen your terminal, then verify:

```powershell
rustc --version
# Should print: rustc 1.xx.x
cargo --version
# Should print: cargo 1.xx.x
```

### Step 2: Install Node.js (for Svelte frontend)

```powershell
winget install OpenJS.NodeJS.LTS
```

Or download from https://nodejs.org (pick the LTS version).

Verify:

```powershell
node --version
# Should print: v22.x.x or v24.x.x
npm --version
# Should print: 10.x.x+
```

### Step 3: Install Git

```powershell
winget install Git.Git
```

Or download from https://git-scm.com/download/win

Verify:

```powershell
git --version
# Should print: git version 2.x.x
```

### Step 4: Install Visual Studio C++ Build Tools (Required by Rust on Windows)

Rust needs a C linker to compile. Install it:

```powershell
winget install Microsoft.VisualStudio.2022.BuildTools
```

During installation, check the box: **"Desktop development with C++"**

If you already have Visual Studio installed, this step is done.

### Step 5: Verify Everything Works

Open a NEW terminal (important — old terminals won't see the new installs):

```powershell
rustc --version
cargo --version
node --version
npm --version
git --version
```

All 5 must print version numbers. If any fails, restart your PC and try again.

---

## PART B: Create the GitHub Repository

### Step 1: Create the repo on GitHub

1. Go to https://github.com/new
2. Repository name: `fastrmail`
3. Description: `The 100% free, open-source, single-binary mail & collaboration server.`
4. Choose: **Public** (since you want open-source)
5. Check: **Add a README file**
6. License: **Apache License 2.0**
7. Click **Create repository**

### Step 2: Clone it to your PC

```powershell
cd C:\Users\YourName\Projects
git clone https://github.com/YOUR_USERNAME/fastrmail.git
cd fastrmail
```

### Step 3: Copy the blueprint into the repo

Save `FASTRMAIL_PLAN.md` into the `fastrmail/` folder root.

---

## PART C: Open in Antergo (Claude Code) and Build

### Step 1: Open the folder

Open Antergo / Claude Code and open the `fastrmail` folder as your workspace.

### Step 2: Paste this EXACT prompt into Antergo

---

```
You are building a production open-source project called FastrMail from scratch.
The machine has Rust, Node.js, npm, and Git installed.

RULES (STRICT - DO NOT VIOLATE):
1. Read FASTRMAIL_PLAN.md in this directory FIRST. It is the absolute source of truth for architecture.
2. Do NOT skip steps. Do NOT assume anything. Execute every command yourself.
3. After EVERY major step, run the verification command to confirm zero errors.
4. If ANY error occurs, fix it IMMEDIATELY before moving to the next step.
5. Use git to commit after each completed phase.
6. Do NOT ask me questions. Make the best architectural decision using the blueprint.
7. Use only stable, well-known crate versions. Do NOT use alpha/beta/nightly features.

=====================================================
PHASE 1: PROJECT SCAFFOLD + CORE SYSTEMS
=====================================================

--- STEP 1: Initialize the Rust Cargo Workspace ---

Create root Cargo.toml:

[workspace]
members = [
    "crates/fastrmail-binary",
    "crates/fastrmail-smtp",
    "crates/fastrmail-imap",
    "crates/fastrmail-jmap",
    "crates/fastrmail-core",
    "crates/fastrmail-auth",
    "crates/fastrmail-store",
    "crates/fastrmail-search",
]
resolver = "2"

[workspace.package]
version = "0.1.0"
edition = "2021"
authors = ["Atif Jubaer"]
license = "MIT OR Apache-2.0"

[profile.release]
opt-level = 3
lto = true
codegen-units = 1
strip = true

Create each crate directory with its own Cargo.toml and src/lib.rs (or src/main.rs for fastrmail-binary).

Crate dependency versions to use (EXACT — do not change):
  tokio = "1"                  (features = ["full"])
  axum = "0.7"
  tower-http = "0.5"           (features = ["fs", "cors", "trace"])
  rusqlite = "0.31"            (features = ["bundled"])
  serde = "1"                  (features = ["derive"])
  serde_json = "1"
  uuid = "1"                   (features = ["v4"])
  chrono = "0.4"               (features = ["serde"])
  tracing = "0.1"
  tracing-subscriber = "0.3"   (features = ["env-filter"])
  anyhow = "1"
  tokio-rustls = "0.25"
  tantivy = "0.22"

VERIFY: Run `cargo check` from the workspace root. Must compile with ZERO errors.

--- STEP 2: Build the SQLite Storage Layer ---

In crates/fastrmail-store/src/lib.rs, implement:

1. A `Database` struct that wraps rusqlite::Connection
2. A `Database::new(path: &str)` constructor that opens/creates the SQLite file
3. A `Database::init_schema()` method that creates these tables:

   tenants        (id TEXT PK, domain TEXT UNIQUE, created_at DATETIME)
   accounts       (id TEXT PK, tenant_id TEXT FK, username TEXT, email TEXT UNIQUE, password_hash TEXT, quota_bytes BIGINT DEFAULT 10737418240, created_at DATETIME)
   dkim_keys      (id TEXT PK, tenant_id TEXT FK, selector TEXT, private_key_pem TEXT, is_active BOOLEAN)
   mailboxes      (id TEXT PK, account_id TEXT FK, name TEXT, parent_id TEXT, uid_validity INTEGER, uid_next INTEGER DEFAULT 1, modseq BIGINT DEFAULT 1)
   messages       (id TEXT PK, mailbox_id TEXT FK, account_id TEXT FK, uid INTEGER, modseq BIGINT, blob_id TEXT, size_bytes INTEGER, parsed_subject TEXT, parsed_from TEXT, parsed_to TEXT, internal_date DATETIME, flags TEXT, UNIQUE(mailbox_id, uid))
   smtp_queue     (id TEXT PK, tenant_id TEXT FK, raw_blob_id TEXT, sender TEXT, recipient TEXT, status TEXT DEFAULT 'pending', next_retry_at DATETIME, retry_count INTEGER DEFAULT 0)

4. CRUD methods:
   - insert_tenant(domain) -> tenant_id
   - insert_account(tenant_id, username, email, password_hash) -> account_id
   - insert_mailbox(account_id, name) -> mailbox_id
   - insert_message(mailbox_id, account_id, blob_id, size, subject, from, to) -> message_id
   - get_messages(account_id) -> Vec<Message>
   - queue_email(tenant_id, blob_id, sender, recipient) -> queue_id
   - get_queue_pending() -> Vec<QueueItem>

5. Write unit tests:
   - test_init_schema: call init_schema(), verify all 6 tables exist
   - test_insert_and_get: insert tenant -> account -> mailbox -> message, then get_messages and verify

VERIFY: Run `cargo test -p fastrmail-store`. ALL tests must pass.

--- STEP 3: Build the SMTP Inbound Server ---

In crates/fastrmail-smtp/src/lib.rs, implement:

1. Add dependency: fastrmail-store = { path = "../fastrmail-store" }
2. A `SmtpServer` struct that holds an Arc<Database>
3. A `SmtpServer::start(addr: &str)` method that:
   - Binds a tokio::net::TcpListener
   - Accepts connections
   - Spawns a task per connection calling handle_connection()
4. handle_connection() must implement the SMTP state machine:
   - Send "220 FastrMail ESMTP Ready\r\n"
   - Loop reading lines:
     - EHLO/HELO -> respond "250-FastrMail\r\n250-SIZE 52428800\r\n250-8BITMIME\r\n250 OK\r\n"
     - MAIL FROM:<addr> -> parse sender, respond "250 OK\r\n"
     - RCPT TO:<addr> -> parse recipient, respond "250 OK\r\n"
     - DATA -> respond "354 Start mail input\r\n", read until lone ".\r\n", parse Subject/From/To headers
     - QUIT -> respond "221 Bye\r\n", close
     - Unknown -> respond "500 Command not recognized\r\n"
5. After DATA, save the message:
   - Write raw bytes to disk at data/blobs/{uuid}.eml
   - Insert metadata into SQLite via Database::insert_message()
   - Insert into smtp_queue if recipient is external

6. Write integration test:
   - Start SmtpServer on 127.0.0.1:2525
   - Connect with tokio::net::TcpStream
   - Send EHLO, MAIL FROM, RCPT TO, DATA with a test email body, QUIT
   - Verify the message was saved to SQLite

VERIFY: Run `cargo test -p fastrmail-smtp`. ALL tests must pass.

--- STEP 4: Build the HTTP API ---

In crates/fastrmail-binary/src/main.rs, implement:

1. Add dependencies: fastrmail-store, fastrmail-smtp, axum, tower-http, tokio, serde, serde_json, tracing, tracing-subscriber, uuid
2. Shared state: Arc<AppState> containing Database
3. Routes:
   GET  /api/health           -> returns Json({"status": "ok"})
   POST /api/v1/email/send    -> accepts Json body {from, to, subject, html, text}, queues to smtp_queue, returns Json({"success": true, "message_id": "..."})
   GET  /api/v1/mailbox       -> returns Json array of all messages for the account
4. On startup:
   - Initialize Database at "data/fastrmail.db"
   - Spawn SMTP server on 0.0.0.0:2525
   - Start Axum HTTP server on 0.0.0.0:8080
   - Print startup banner:
     "FastrMail v0.1.0"
     "SMTP listening on :2525"
     "HTTP API listening on :8080"
5. Test: Run `cargo run -p fastrmail-binary`, then in another terminal:
   curl http://localhost:8080/api/health
   Must return: {"status":"ok"}

VERIFY: Run `cargo build --release` from workspace root. Must compile with ZERO errors.

--- STEP 5: Scaffold the Svelte 5 Frontends ---

1. Create web/ directory at workspace root
2. Inside web/:
   - Run: npm create vite@latest admin -- --template svelte-ts
   - Run: npm create vite@latest webmail -- --template svelte-ts
3. In web/admin/:
   - Run: npm install
   - Run: npm install -D tailwindcss @tailwindcss/vite
   - Configure TailwindCSS in vite.config.ts (add @tailwindcss/vite plugin)
   - Add @import "tailwindcss" to src/app.css
   - Edit src/App.svelte to show: <h1 class="text-2xl font-bold">FastrMail Admin</h1>
   - Run: npm run build — must succeed
4. In web/webmail/:
   - Run: npm install
   - Run: npm install -D tailwindcss @tailwindcss/vite
   - Configure TailwindCSS in vite.config.ts
   - Add @import "tailwindcss" to src/app.css
   - Edit src/App.svelte to show: <h1 class="text-2xl font-bold">FastrMail Inbox</h1>
   - Run: npm run build — must succeed

VERIFY: Both `npm run build` commands produce dist/ folders with zero errors.

--- STEP 6: Final Commit ---

Run all verifications:
  cargo check          — zero errors
  cargo test           — all tests pass
  cargo build --release — compiles successfully
  cd web/admin && npm run build  — success
  cd web/webmail && npm run build — success

If ALL pass, commit:
  git add -A
  git commit -m "feat: Phase 1 complete - workspace, SMTP, storage, API, frontend scaffold"

Write a file called PHASE1_COMPLETE.md summarizing:
  - What crates were created
  - What tests pass
  - What endpoints are available
  - Build times
```

---

## PART D: After Phase 1 Completes

1. Push to GitHub:
```powershell
git remote add origin https://github.com/YOUR_USERNAME/fastrmail.git
git branch -M main
git push -u origin main
```

2. Come to the Discord chat with Noor and say: **"Phase 1 done, here's the repo: [link]"**

3. I will review the code and prepare the Phase 2 prompt.

---

## Troubleshooting

### "cargo not found"
→ Restart your terminal. If still fails, run: `rustup default stable`

### "LINK.exe not found" or "linker not found"
→ You need Visual Studio C++ Build Tools. Run:
```powershell
winget install Microsoft.VisualStudio.2022.BuildTools
```
Select "Desktop development with C++" during install.

### "npm not found"
→ Restart terminal after Node.js install. Or reinstall Node from https://nodejs.org

### rusqlite compile error about "sqlite3"
→ That's why we use `features = ["bundled"]` — it bundles SQLite into the binary. If you still get errors, run:
```powershell
cargo clean
cargo build
```

### Port 2525 already in use
→ Another program is using that port. Either close it or change the port in the code to 2526.

### Antergo / Claude Code session times out
→ Normal for large builds. Just re-open the folder and paste:
```
Read PHASE1_COMPLETE.md. If Phase 1 is done, verify all tests pass. If not, complete the remaining steps.
```
