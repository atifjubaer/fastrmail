//! FastrMail Core — Shared types, configuration, and common utilities.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Represents a tenant (organization/domain owner) in the multi-tenant system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tenant {
    pub id: String,
    pub domain: String,
    pub created_at: DateTime<Utc>,
}

/// Represents a user account within a tenant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub id: String,
    pub tenant_id: String,
    pub username: String,
    pub email: String,
    pub password_hash: String,
    pub quota_bytes: i64,
    pub created_at: DateTime<Utc>,
}

/// Represents an IMAP mailbox (folder) for an account.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mailbox {
    pub id: String,
    pub account_id: String,
    pub name: String,
    pub parent_id: Option<String>,
    pub uid_validity: i64,
    pub uid_next: i64,
    pub modseq: i64,
}

/// Represents an email message's metadata (the blob is stored on disk/S3).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub mailbox_id: String,
    pub account_id: String,
    pub uid: i64,
    pub modseq: i64,
    pub blob_id: String,
    pub size_bytes: i64,
    pub parsed_subject: Option<String>,
    pub parsed_from: Option<String>,
    pub parsed_to: Option<String>,
    pub internal_date: DateTime<Utc>,
    pub flags: String,
}

/// Represents an item in the outbound SMTP delivery queue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueItem {
    pub id: String,
    pub tenant_id: String,
    pub raw_blob_id: String,
    pub sender: String,
    pub recipient: String,
    pub status: String,
    pub next_retry_at: Option<DateTime<Utc>>,
    pub retry_count: i64,
}

/// DKIM key record for a tenant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DkimKey {
    pub id: String,
    pub tenant_id: String,
    pub selector: String,
    pub private_key_pem: String,
    pub is_active: bool,
}

/// Global statistics for administrative dashboard.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SystemStats {
    pub tenants_count: i64,
    pub accounts_count: i64,
    pub messages_count: i64,
    pub queue_pending_count: i64,
}

/// Global server configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub hostname: String,
    pub smtp_bind: String,
    pub http_bind: String,
    pub data_dir: String,
    pub db_path: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            hostname: "localhost".to_string(),
            smtp_bind: "0.0.0.0:2525".to_string(),
            http_bind: "0.0.0.0:8080".to_string(),
            data_dir: "data".to_string(),
            db_path: "data/fastrmail.db".to_string(),
        }
    }
}

/// Field in an incoming email to evaluate in a Sieve filter.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SieveField {
    Subject,
    From,
    To,
    Body,
}

/// Comparison operator for Sieve evaluation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SieveOperator {
    Contains,
    Equals,
    StartsWith,
    EndsWith,
}

/// Action to execute when a Sieve rule matches.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum SieveAction {
    Discard,
    Reject { reason: Option<String> },
    FileInto { mailbox: String },
    MarkRead,
    AddFlag { flag: String },
}

/// A structured Sieve rule for automated inbound filtering.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SieveRule {
    pub id: String,
    pub name: String,
    pub field: SieveField,
    pub operator: SieveOperator,
    pub value: String,
    pub action: SieveAction,
    pub is_active: bool,
}

impl SieveRule {
    /// Evaluate if this rule matches the given message fields.
    pub fn matches(&self, subject: &str, from: &str, to: &str, body: &str) -> bool {
        if !self.is_active {
            return false;
        }
        let target = match self.field {
            SieveField::Subject => subject,
            SieveField::From => from,
            SieveField::To => to,
            SieveField::Body => body,
        };

        let target_lower = target.to_lowercase();
        let val_lower = self.value.to_lowercase();

        match self.operator {
            SieveOperator::Contains => target_lower.contains(&val_lower),
            SieveOperator::Equals => target_lower == val_lower,
            SieveOperator::StartsWith => target_lower.starts_with(&val_lower),
            SieveOperator::EndsWith => target_lower.ends_with(&val_lower),
        }
    }
}

/// Record representing a stored Sieve script/rule-set for an account.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SieveScript {
    pub id: String,
    pub account_id: String,
    pub name: String,
    pub script_json: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

/// Represents a CalDAV Calendar collection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Calendar {
    pub id: String,
    pub account_id: String,
    pub name: String,
    pub description: Option<String>,
    pub color: String,
    pub ctag: String,
    pub created_at: DateTime<Utc>,
}

/// Represents an iCalendar event within a CalDAV Calendar.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarEvent {
    pub id: String,
    pub calendar_id: String,
    pub uid: String,
    pub ical_data: String,
    pub etag: String,
    pub updated_at: DateTime<Utc>,
}

/// Represents a CardDAV Address Book collection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddressBook {
    pub id: String,
    pub account_id: String,
    pub name: String,
    pub description: Option<String>,
    pub ctag: String,
    pub created_at: DateTime<Utc>,
}

/// Represents a vCard contact within a CardDAV Address Book.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardDavContact {
    pub id: String,
    pub address_book_id: String,
    pub uid: String,
    pub vcard_data: String,
    pub etag: String,
    pub updated_at: DateTime<Utc>,
}

/// Represents an inbound email webhook automation trigger (e.g. for n8n/Zapier).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Webhook {
    pub id: String,
    pub account_id: String,
    pub url: String,
    pub secret: Option<String>,
    pub event_types: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert_eq!(config.hostname, "localhost");
        assert_eq!(config.smtp_bind, "0.0.0.0:2525");
        assert_eq!(config.http_bind, "0.0.0.0:8080");
    }

    #[test]
    fn test_tenant_serialization() {
        let tenant = Tenant {
            id: "t1".to_string(),
            domain: "example.com".to_string(),
            created_at: Utc::now(),
        };
        let json = serde_json::to_string(&tenant).unwrap();
        let deserialized: Tenant = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, "t1");
        assert_eq!(deserialized.domain, "example.com");
    }

    #[test]
    fn test_message_serialization() {
        let msg = Message {
            id: "m1".to_string(),
            mailbox_id: "mb1".to_string(),
            account_id: "a1".to_string(),
            uid: 1,
            modseq: 1,
            blob_id: "blob1".to_string(),
            size_bytes: 1024,
            parsed_subject: Some("Test".to_string()),
            parsed_from: Some("sender@example.com".to_string()),
            parsed_to: Some("recipient@example.com".to_string()),
            internal_date: Utc::now(),
            flags: r#"["\\Seen"]"#.to_string(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: Message = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, "m1");
        assert_eq!(deserialized.size_bytes, 1024);
    }

    #[test]
    fn test_sieve_rule_matching() {
        let rule = SieveRule {
            id: "r1".to_string(),
            name: "Spam discard".to_string(),
            field: SieveField::Subject,
            operator: SieveOperator::Contains,
            value: "viagra".to_string(),
            action: SieveAction::Discard,
            is_active: true,
        };

        assert!(rule.matches("Buy cheap VIAGRA today!", "bad@spam.com", "me@home.com", ""));
        assert!(!rule.matches("Clean meeting notes", "boss@corp.com", "me@home.com", ""));

        let json = serde_json::to_string(&rule).unwrap();
        let decoded: SieveRule = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.name, "Spam discard");
        assert_eq!(decoded.action, SieveAction::Discard);
    }
}
