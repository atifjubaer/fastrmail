# JMAP Engine (RFC 8620 / RFC 8621)

JSON Meta Application Protocol (JMAP) is the modern successor to IMAP. FastrMail provides a native Rust implementation of RFC 8620 (JMAP Core) and RFC 8621 (JMAP Mail).

---

## Endpoints

- **Session Discovery**: `GET /jmap/session` (or `/.well-known/jmap`)
- **API Dispatcher**: `POST /jmap/api`

---

## Supported Capabilities

- `urn:ietf:params:jmap:core`
- `urn:ietf:params:jmap:mail`

---

## Implemented Methods

### 1. `Core/echo`
Echoes arguments back for testing and validation.

### 2. `Mailbox/get`
Fetches a list of mailboxes for an account with unread/total message counts.

### 3. `Email/query`
Searches messages in an account using filters (e.g. `inMailbox`, `text`, `from`, `to`, `subject`). Integrates with Tantivy for full-text search.

### 4. `Email/get`
Retrieves message headers, sender, recipient, subject, preview snippet, flags, and size.

### 5. `Email/set`
Performs atomic mutations:
- `create`: Enqueues an outgoing email or saves draft
- `update`: Modifies message flags (e.g. `$seen`, `$flagged`)
- `destroy`: Deletes messages from mailbox
