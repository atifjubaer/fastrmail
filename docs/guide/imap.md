# IMAP4rev2 Engine (RFC 9051)

FastrMail implements a next-generation IMAP4rev2 protocol engine operating on TCP port `1143` (mapped to `143` in Docker).

---

## Supported Commands & State Machine

FastrMail maintains a strict session state machine:
- `NotAuthenticated`
- `Authenticated`
- `Selected`
- `Logout`

### Command Reference

| Command | State Allowed | Description |
|:--------|:--------------|:------------|
| `CAPABILITY` | Any | Reports IMAP4rev2, IMAP4rev1, AUTH=PLAIN, MOVE, IDLE, CONDSTORE |
| `LOGIN` | NotAuthenticated | Verifies user credentials using Argon2id |
| `LIST` | Authenticated, Selected | Lists available mailboxes and flags |
| `SELECT` | Authenticated, Selected | Opens a mailbox, reports `FLAGS`, `EXISTS`, `RECENT`, `UIDVALIDITY`, and `UIDNEXT` |
| `CREATE` / `DELETE` / `RENAME` | Authenticated, Selected | Manages mailbox lifecycle |
| `FETCH` / `UID FETCH` | Selected | Retrieves flags, envelopes, structure, headers, bodies, RFC822, and MODSEQ |
| `STORE` / `UID STORE` | Selected | Sets, adds, or removes `\Seen`, `\Answered`, `\Flagged`, `\Deleted`, `\Draft` |
| `EXPUNGE` / `UID EXPUNGE` | Selected | Permanently removes messages marked `\Deleted` |
| `NOOP` / `CHECK` | Authenticated, Selected | Keeps session alive and updates mailbox counts |
| `LOGOUT` | Any | Gracefully shuts down connection |

---

## Client Compatibility

FastrMail IMAP works seamlessly with:
- Mozilla Thunderbird
- Apple Mail (macOS & iOS)
- Microsoft Outlook
- Mutt / Neomutt / Alpine
- Android FairEmail / K-9 Mail
