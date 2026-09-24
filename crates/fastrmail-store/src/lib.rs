//! FastrMail Store — SQLite storage layer for all persistent data.
//!
//! Provides the `Database` struct wrapping rusqlite for multi-tenant email storage.
//! Uses `Mutex<Connection>` to ensure `Send + Sync` for use with `Arc<Database>` across tokio tasks.

use std::sync::Mutex;

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use uuid::Uuid;

use fastrmail_auth::{hash_password, verify_password};
use fastrmail_core::{Account, DkimKey, Mailbox, Message, QueueItem, Tenant};

/// Main database handle wrapping a SQLite connection in a Mutex for thread safety.
pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    /// Open or create a SQLite database at the given path.
    pub fn new(path: &str) -> Result<Self> {
        if let Some(parent) = std::path::Path::new(path).parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory for database: {path}"))?;
        }
        let conn = Connection::open(path)
            .with_context(|| format!("Failed to open database at {path}"))?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// Create an in-memory database (useful for testing).
    pub fn new_memory() -> Result<Self> {
        let conn =
            Connection::open_in_memory().context("Failed to open in-memory database")?;
        conn.execute_batch("PRAGMA foreign_keys=ON;")?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// Initialize the database schema — creates all tables and performance indexes.
    pub fn init_schema(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS tenants (
                id TEXT PRIMARY KEY,
                domain TEXT UNIQUE NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS accounts (
                id TEXT PRIMARY KEY,
                tenant_id TEXT REFERENCES tenants(id),
                username TEXT NOT NULL,
                email TEXT UNIQUE NOT NULL,
                password_hash TEXT NOT NULL,
                quota_bytes BIGINT DEFAULT 10737418240,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(tenant_id, username)
            );

            CREATE TABLE IF NOT EXISTS dkim_keys (
                id TEXT PRIMARY KEY,
                tenant_id TEXT REFERENCES tenants(id),
                selector TEXT NOT NULL,
                private_key_pem TEXT NOT NULL,
                is_active BOOLEAN DEFAULT 0
            );

            CREATE TABLE IF NOT EXISTS mailboxes (
                id TEXT PRIMARY KEY,
                account_id TEXT REFERENCES accounts(id),
                name TEXT NOT NULL,
                parent_id TEXT,
                uid_validity INTEGER NOT NULL,
                uid_next INTEGER DEFAULT 1,
                modseq BIGINT DEFAULT 1
            );

            CREATE TABLE IF NOT EXISTS messages (
                id TEXT PRIMARY KEY,
                mailbox_id TEXT REFERENCES mailboxes(id),
                account_id TEXT REFERENCES accounts(id),
                uid INTEGER NOT NULL,
                modseq BIGINT NOT NULL,
                blob_id TEXT NOT NULL,
                size_bytes INTEGER NOT NULL,
                parsed_subject TEXT,
                parsed_from TEXT,
                parsed_to TEXT,
                internal_date DATETIME NOT NULL,
                flags TEXT,
                UNIQUE(mailbox_id, uid)
            );

            CREATE TABLE IF NOT EXISTS smtp_queue (
                id TEXT PRIMARY KEY,
                tenant_id TEXT REFERENCES tenants(id),
                raw_blob_id TEXT NOT NULL,
                sender TEXT NOT NULL,
                recipient TEXT NOT NULL,
                status TEXT DEFAULT 'pending',
                next_retry_at DATETIME,
                retry_count INTEGER DEFAULT 0
            );

            -- Performance Indexes for IMAP & Outbound Queue
            CREATE INDEX IF NOT EXISTS idx_messages_account ON messages(account_id);
            CREATE INDEX IF NOT EXISTS idx_messages_mailbox ON messages(mailbox_id, uid);
            CREATE INDEX IF NOT EXISTS idx_smtp_queue_status ON smtp_queue(status);
            ",
        )
        .context("Failed to initialize database schema")?;
        Ok(())
    }

    // ─── Tenant CRUD ─────────────────────────────────────────

    /// Insert a new tenant for the given domain. Returns the generated tenant ID.
    pub fn insert_tenant(&self, domain: &str) -> Result<String> {
        let conn = self.conn.lock().unwrap();
        let id = Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO tenants (id, domain) VALUES (?1, ?2)",
            params![id, domain],
        )?;
        Ok(id)
    }

    /// Retrieve a tenant by domain.
    pub fn get_tenant_by_domain(&self, domain: &str) -> Result<Option<Tenant>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt =
            conn.prepare("SELECT id, domain, created_at FROM tenants WHERE domain = ?1")?;
        let mut rows = stmt.query_map(params![domain], |row| {
            Ok(Tenant {
                id: row.get(0)?,
                domain: row.get(1)?,
                created_at: row.get::<_, String>(2).map(|s| {
                    chrono::NaiveDateTime::parse_from_str(&s, "%Y-%m-%d %H:%M:%S")
                        .map(|ndt| ndt.and_utc())
                        .unwrap_or_else(|_| Utc::now())
                })?,
            })
        })?;
        match rows.next() {
            Some(Ok(tenant)) => Ok(Some(tenant)),
            Some(Err(e)) => Err(e.into()),
            None => Ok(None),
        }
    }

    // ─── Account CRUD ────────────────────────────────────────

    /// Insert a new account. Hashes raw password with Argon2id if not already hashed.
    pub fn insert_account(
        &self,
        tenant_id: &str,
        username: &str,
        email: &str,
        password: &str,
    ) -> Result<String> {
        let pw_hash = if password.starts_with("$argon2") {
            password.to_string()
        } else {
            hash_password(password)?
        };

        let conn = self.conn.lock().unwrap();
        let id = Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO accounts (id, tenant_id, username, email, password_hash) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![id, tenant_id, username, email, pw_hash],
        )?;
        Ok(id)
    }

    /// Retrieve an account by email address.
    pub fn get_account_by_email(&self, email: &str) -> Result<Option<Account>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, tenant_id, username, email, password_hash, quota_bytes, created_at FROM accounts WHERE email = ?1",
        )?;
        let mut rows = stmt.query_map(params![email], |row| {
            Ok(Account {
                id: row.get(0)?,
                tenant_id: row.get(1)?,
                username: row.get(2)?,
                email: row.get(3)?,
                password_hash: row.get(4)?,
                quota_bytes: row.get(5)?,
                created_at: row.get::<_, String>(6).map(|s| {
                    chrono::NaiveDateTime::parse_from_str(&s, "%Y-%m-%d %H:%M:%S")
                        .map(|ndt| ndt.and_utc())
                        .unwrap_or_else(|_| Utc::now())
                })?,
            })
        })?;
        match rows.next() {
            Some(Ok(account)) => Ok(Some(account)),
            Some(Err(e)) => Err(e.into()),
            None => Ok(None),
        }
    }

    /// Verify an email and password combination, returning the account if valid.
    pub fn verify_login(&self, email: &str, password: &str) -> Result<Option<Account>> {
        let account = match self.get_account_by_email(email)? {
            Some(a) => a,
            None => return Ok(None),
        };

        if verify_password(password, &account.password_hash)? {
            Ok(Some(account))
        } else {
            Ok(None)
        }
    }

    // ─── Mailbox CRUD ────────────────────────────────────────

    /// Insert a new mailbox for an account. Returns the generated mailbox ID.
    pub fn insert_mailbox(&self, account_id: &str, name: &str) -> Result<String> {
        let conn = self.conn.lock().unwrap();
        let id = Uuid::new_v4().to_string();
        let uid_validity = Utc::now().timestamp();
        conn.execute(
            "INSERT INTO mailboxes (id, account_id, name, uid_validity) VALUES (?1, ?2, ?3, ?4)",
            params![id, account_id, name, uid_validity],
        )?;
        Ok(id)
    }

    /// Get all mailboxes for an account.
    pub fn get_mailboxes(&self, account_id: &str) -> Result<Vec<Mailbox>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, account_id, name, parent_id, uid_validity, uid_next, modseq FROM mailboxes WHERE account_id = ?1",
        )?;
        let rows = stmt.query_map(params![account_id], |row| {
            Ok(Mailbox {
                id: row.get(0)?,
                account_id: row.get(1)?,
                name: row.get(2)?,
                parent_id: row.get(3)?,
                uid_validity: row.get(4)?,
                uid_next: row.get(5)?,
                modseq: row.get(6)?,
            })
        })?;
        let mut mailboxes = Vec::new();
        for row in rows {
            mailboxes.push(row?);
        }
        Ok(mailboxes)
    }

    // ─── Message CRUD ────────────────────────────────────────

    /// Insert a new message. Automatically assigns UID and modseq in a single combined query.
    pub fn insert_message(
        &self,
        mailbox_id: &str,
        account_id: &str,
        blob_id: &str,
        size_bytes: i64,
        subject: Option<&str>,
        from: Option<&str>,
        to: Option<&str>,
    ) -> Result<String> {
        let conn = self.conn.lock().unwrap();
        let id = Uuid::new_v4().to_string();

        // Optimized: Single query for both uid_next and modseq
        let (uid, modseq): (i64, i64) = conn.query_row(
            "SELECT uid_next, modseq FROM mailboxes WHERE id = ?1",
            params![mailbox_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;

        let now = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

        conn.execute(
            "INSERT INTO messages (id, mailbox_id, account_id, uid, modseq, blob_id, size_bytes, parsed_subject, parsed_from, parsed_to, internal_date, flags) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![id, mailbox_id, account_id, uid, modseq, blob_id, size_bytes, subject, from, to, now, "[]"],
        )?;

        conn.execute(
            "UPDATE mailboxes SET uid_next = uid_next + 1, modseq = modseq + 1 WHERE id = ?1",
            params![mailbox_id],
        )?;

        Ok(id)
    }

    /// Get all messages for an account across all mailboxes.
    pub fn get_messages(&self, account_id: &str) -> Result<Vec<Message>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, mailbox_id, account_id, uid, modseq, blob_id, size_bytes, parsed_subject, parsed_from, parsed_to, internal_date, flags FROM messages WHERE account_id = ?1 ORDER BY internal_date DESC",
        )?;
        let rows = stmt.query_map(params![account_id], |row| {
            Ok(Message {
                id: row.get(0)?,
                mailbox_id: row.get(1)?,
                account_id: row.get(2)?,
                uid: row.get(3)?,
                modseq: row.get(4)?,
                blob_id: row.get(5)?,
                size_bytes: row.get(6)?,
                parsed_subject: row.get(7)?,
                parsed_from: row.get(8)?,
                parsed_to: row.get(9)?,
                internal_date: row.get::<_, String>(10).map(|s| {
                    chrono::NaiveDateTime::parse_from_str(&s, "%Y-%m-%d %H:%M:%S")
                        .map(|ndt| ndt.and_utc())
                        .unwrap_or_else(|_| Utc::now())
                })?,
                flags: row
                    .get::<_, Option<String>>(11)?
                    .unwrap_or_else(|| "[]".to_string()),
            })
        })?;
        let mut messages = Vec::new();
        for row in rows {
            messages.push(row?);
        }
        Ok(messages)
    }

    /// Get messages in a specific mailbox.
    pub fn get_messages_by_mailbox(&self, mailbox_id: &str) -> Result<Vec<Message>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, mailbox_id, account_id, uid, modseq, blob_id, size_bytes, parsed_subject, parsed_from, parsed_to, internal_date, flags FROM messages WHERE mailbox_id = ?1 ORDER BY uid ASC",
        )?;
        let rows = stmt.query_map(params![mailbox_id], |row| {
            Ok(Message {
                id: row.get(0)?,
                mailbox_id: row.get(1)?,
                account_id: row.get(2)?,
                uid: row.get(3)?,
                modseq: row.get(4)?,
                blob_id: row.get(5)?,
                size_bytes: row.get(6)?,
                parsed_subject: row.get(7)?,
                parsed_from: row.get(8)?,
                parsed_to: row.get(9)?,
                internal_date: row.get::<_, String>(10).map(|s| {
                    chrono::NaiveDateTime::parse_from_str(&s, "%Y-%m-%d %H:%M:%S")
                        .map(|ndt| ndt.and_utc())
                        .unwrap_or_else(|_| Utc::now())
                })?,
                flags: row
                    .get::<_, Option<String>>(11)?
                    .unwrap_or_else(|| "[]".to_string()),
            })
        })?;
        let mut messages = Vec::new();
        for row in rows {
            messages.push(row?);
        }
        Ok(messages)
    }

    // ─── SMTP Queue CRUD ─────────────────────────────────────

    /// Queue an email for outbound delivery. Returns the queue item ID.
    pub fn queue_email(
        &self,
        tenant_id: &str,
        blob_id: &str,
        sender: &str,
        recipient: &str,
    ) -> Result<String> {
        let conn = self.conn.lock().unwrap();
        let id = Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO smtp_queue (id, tenant_id, raw_blob_id, sender, recipient) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![id, tenant_id, blob_id, sender, recipient],
        )?;
        Ok(id)
    }

    /// Get all pending or retrying items in the SMTP outbound queue ready for delivery.
    pub fn get_queue_pending(&self) -> Result<Vec<QueueItem>> {
        let conn = self.conn.lock().unwrap();
        let now_str = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let mut stmt = conn.prepare(
            "SELECT id, tenant_id, raw_blob_id, sender, recipient, status, next_retry_at, retry_count
             FROM smtp_queue
             WHERE status = 'pending' OR (status = 'retrying' AND (next_retry_at IS NULL OR next_retry_at <= ?1))
             ORDER BY rowid ASC",
        )?;
        let rows = stmt.query_map(params![now_str], |row| {
            Ok(QueueItem {
                id: row.get(0)?,
                tenant_id: row.get(1)?,
                raw_blob_id: row.get(2)?,
                sender: row.get(3)?,
                recipient: row.get(4)?,
                status: row.get(5)?,
                next_retry_at: row.get::<_, Option<String>>(6).map(|opt| {
                    opt.and_then(|s| {
                        chrono::NaiveDateTime::parse_from_str(&s, "%Y-%m-%d %H:%M:%S")
                            .map(|ndt| ndt.and_utc())
                            .ok()
                    })
                })?,
                retry_count: row.get(7)?,
            })
        })?;
        let mut items = Vec::new();
        for row in rows {
            items.push(row?);
        }
        Ok(items)
    }

    /// Update the status, next_retry_at, and retry_count of a queued email.
    pub fn update_queue_status(
        &self,
        id: &str,
        status: &str,
        next_retry_at: Option<DateTime<Utc>>,
        retry_count: i64,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let retry_str = next_retry_at.map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string());
        conn.execute(
            "UPDATE smtp_queue SET status = ?1, next_retry_at = ?2, retry_count = ?3 WHERE id = ?4",
            params![status, retry_str, retry_count, id],
        )?;
        Ok(())
    }

    /// Delete a processed item from the SMTP outbound queue.
    pub fn delete_queue_item(&self, id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM smtp_queue WHERE id = ?1", params![id])?;
        Ok(())
    }

    // ─── DKIM Key CRUD ───────────────────────────────────────

    /// Insert a DKIM signing key for a tenant.
    pub fn insert_dkim_key(
        &self,
        tenant_id: &str,
        selector: &str,
        private_key_pem: &str,
    ) -> Result<String> {
        let conn = self.conn.lock().unwrap();
        let id = Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO dkim_keys (id, tenant_id, selector, private_key_pem, is_active) VALUES (?1, ?2, ?3, ?4, 1)",
            params![id, tenant_id, selector, private_key_pem],
        )?;
        Ok(id)
    }

    /// Retrieve the active DKIM key for a tenant.
    pub fn get_active_dkim_key(&self, tenant_id: &str) -> Result<Option<DkimKey>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, tenant_id, selector, private_key_pem, is_active FROM dkim_keys WHERE tenant_id = ?1 AND is_active = 1 LIMIT 1",
        )?;
        let mut rows = stmt.query_map(params![tenant_id], |row| {
            Ok(DkimKey {
                id: row.get(0)?,
                tenant_id: row.get(1)?,
                selector: row.get(2)?,
                private_key_pem: row.get(3)?,
                is_active: row.get(4)?,
            })
        })?;
        match rows.next() {
            Some(Ok(key)) => Ok(Some(key)),
            Some(Err(e)) => Err(e.into()),
            None => Ok(None),
        }
    }

    /// Get all DKIM keys for a tenant.
    pub fn get_dkim_keys(&self, tenant_id: &str) -> Result<Vec<DkimKey>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, tenant_id, selector, private_key_pem, is_active FROM dkim_keys WHERE tenant_id = ?1",
        )?;
        let rows = stmt.query_map(params![tenant_id], |row| {
            Ok(DkimKey {
                id: row.get(0)?,
                tenant_id: row.get(1)?,
                selector: row.get(2)?,
                private_key_pem: row.get(3)?,
                is_active: row.get(4)?,
            })
        })?;
        let mut keys = Vec::new();
        for row in rows {
            keys.push(row?);
        }
        Ok(keys)
    }

    // ─── Utility ─────────────────────────────────────────────

    /// Check if a specific table exists in the database.
    pub fn table_exists(&self, table_name: &str) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
            params![table_name],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_db() -> Database {
        let db = Database::new_memory().expect("Failed to create in-memory database");
        db.init_schema().expect("Failed to initialize schema");
        db
    }

    #[test]
    fn test_init_schema() {
        let db = setup_db();

        let tables = [
            "tenants",
            "accounts",
            "dkim_keys",
            "mailboxes",
            "messages",
            "smtp_queue",
        ];
        for table in &tables {
            assert!(
                db.table_exists(table).unwrap(),
                "Table '{table}' should exist after init_schema()"
            );
        }
    }

    #[test]
    fn test_account_password_hashing_and_verify_login() {
        let db = setup_db();
        let tenant_id = db.insert_tenant("auth-test.com").unwrap();

        let account_id = db
            .insert_account(&tenant_id, "bob", "bob@auth-test.com", "PlainSecretPassword!")
            .unwrap();
        assert!(!account_id.is_empty());

        // Verify account stored hash, not plain text
        let account = db.get_account_by_email("bob@auth-test.com").unwrap().unwrap();
        assert!(account.password_hash.starts_with("$argon2id$"));

        // Verify correct login
        let logged_in = db
            .verify_login("bob@auth-test.com", "PlainSecretPassword!")
            .unwrap();
        assert!(logged_in.is_some());
        assert_eq!(logged_in.unwrap().username, "bob");

        // Verify wrong password fails login
        let bad_login = db
            .verify_login("bob@auth-test.com", "IncorrectPassword")
            .unwrap();
        assert!(bad_login.is_none());
    }

    #[test]
    fn test_insert_and_get() {
        let db = setup_db();

        let tenant_id = db.insert_tenant("example.com").unwrap();
        let tenant = db.get_tenant_by_domain("example.com").unwrap().unwrap();
        assert_eq!(tenant.domain, "example.com");

        let account_id = db
            .insert_account(&tenant_id, "alice", "alice@example.com", "password123")
            .unwrap();
        let mailbox_id = db.insert_mailbox(&account_id, "INBOX").unwrap();

        let msg_id = db
            .insert_message(
                &mailbox_id,
                &account_id,
                "blob_abc123",
                2048,
                Some("Hello World"),
                Some("bob@other.com"),
                Some("alice@example.com"),
            )
            .unwrap();
        assert!(!msg_id.is_empty());

        let messages = db.get_messages(&account_id).unwrap();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].parsed_subject.as_deref(), Some("Hello World"));
        assert_eq!(messages[0].uid, 1);
        assert_eq!(messages[0].modseq, 1);

        let mailboxes = db.get_mailboxes(&account_id).unwrap();
        assert_eq!(mailboxes[0].uid_next, 2);
        assert_eq!(mailboxes[0].modseq, 2);
    }

    #[test]
    fn test_multiple_messages_uid_increment() {
        let db = setup_db();
        let tenant_id = db.insert_tenant("test.com").unwrap();
        let account_id = db
            .insert_account(&tenant_id, "user", "user@test.com", "hash")
            .unwrap();
        let mailbox_id = db.insert_mailbox(&account_id, "INBOX").unwrap();

        for i in 0..3 {
            db.insert_message(
                &mailbox_id,
                &account_id,
                &format!("blob_{i}"),
                100 * (i + 1),
                Some(&format!("Subject {i}")),
                Some("sender@test.com"),
                Some("user@test.com"),
            )
            .unwrap();
        }

        let messages = db.get_messages_by_mailbox(&mailbox_id).unwrap();
        assert_eq!(messages.len(), 3);
        assert_eq!(messages[0].uid, 1);
        assert_eq!(messages[1].uid, 2);
        assert_eq!(messages[2].uid, 3);
    }

    #[test]
    fn test_smtp_queue_operations() {
        let db = setup_db();
        let tenant_id = db.insert_tenant("queue-test.com").unwrap();

        let q1 = db
            .queue_email(&tenant_id, "blob_1", "sender@queue.com", "ext@other.com")
            .unwrap();

        let pending = db.get_queue_pending().unwrap();
        assert_eq!(pending.len(), 1);

        // Update status to retrying
        db.update_queue_status(&q1, "retrying", Some(Utc::now()), 1)
            .unwrap();

        // Delete from queue
        db.delete_queue_item(&q1).unwrap();
        let after_delete = db.get_queue_pending().unwrap();
        assert!(after_delete.is_empty());
    }

    #[test]
    fn test_dkim_keys() {
        let db = setup_db();
        let tenant_id = db.insert_tenant("dkim-test.com").unwrap();
        let key_id = db
            .insert_dkim_key(&tenant_id, "default", "-----BEGIN RSA PRIVATE KEY-----\ntest\n-----END RSA PRIVATE KEY-----")
            .unwrap();
        assert!(!key_id.is_empty());

        let active_key = db.get_active_dkim_key(&tenant_id).unwrap().unwrap();
        assert_eq!(active_key.selector, "default");
        assert!(active_key.is_active);
    }
}
