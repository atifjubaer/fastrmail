# Tantivy Full-Text Search

FastrMail embeds **Tantivy 0.22**, a Lucene-grade search engine library written in Rust.

---

## Schema Architecture

Each ingested email is parsed into a strongly typed search document:

| Field Name | Type | Options | Description |
|:-----------|:-----|:--------|:------------|
| `tenant_id` | Str | `STRING \| FAST` | Tenant isolation key |
| `account_id` | Str | `STRING \| FAST` | User mailbox isolation key |
| `message_id` | Str | `STRING \| STORED` | Unique message UUID |
| `from` | Text | `TEXT \| STORED` | Sender name and email address |
| `to` | Text | `TEXT \| STORED` | Recipient address(es) |
| `subject` | Text | `TEXT \| STORED` | Email subject line |
| `body` | Text | `TEXT` | Plain text and parsed HTML body |
| `received_at`| I64 | `INDEXED \| FAST \| STORED` | Timestamp of arrival |

---

## Query Syntax

FastrMail supports standard Lucene query syntax:

- Word search: `urgent invoice`
- Phrase search: `"quarterly financial report"`
- Field scoping: `subject:security AND from:admin@example.com`
- Wildcards & Prefix: `deploy*`
