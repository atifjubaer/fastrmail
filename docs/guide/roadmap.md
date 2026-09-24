# Enterprise Roadmap & Gap Analysis (vs. Stalwart)

To establish FastrMail as the definitive open-source alternative to enterprise mail solutions like Stalwart and Exchange, the following capabilities represent our strategic feature roadmap.

---

## Enterprise Feature Matrix

| Category | Capability | Priority | Rationale & Use Case |
|:---------|:-----------|:---------|:---------------------|
| **Protocols** | **POP3 Server** | High | Backward compatibility with legacy mail clients, specialized archiving appliances, and minimal fetchers. |
| **Protocols** | **ManageSieve (RFC 5804)** | High | Client-side rule management for automated mail filtering, folder filing, and out-of-office vacation auto-replies. |
| **Protocols** | **SMTP Submission (Port 587 / 465)** | High | Authenticated client outbound dispatch with mandatory TLS/STARTTLS and sender rate limiting. |
| **Collaboration** | **CalDAV (RFC 4791)** | Medium | Native calendar synchronization for macOS, iOS, Android, Thunderbird, and Outlook. |
| **Collaboration** | **CardDAV (RFC 6352)** | Medium | Native address book and contact synchronization across mobile and desktop devices. |
| **Collaboration** | **WebDAV (RFC 4918)** | Low | Remote file management and attachment cloud storage. |
| **Authentication** | **LDAP / Active Directory** | High | Enterprise Single Sign-On (SSO) and centralized organizational directory synchronization. |
| **Authentication** | **OAuth2 / OIDC** | High | Modern token-based authentication for webmail, third-party apps, and enterprise identity providers (Keycloak, Okta, Google). |
| **Administration** | **Quarantine UI** | High | Dedicated webmail and admin workspace to review, inspect, release, or purge greylisted and suspicious messages. |
| **Security** | **Brute-Force Rate Limiting** | High | Dynamic IP-based throttling and exponential lockouts on authentication failures across IMAP, SMTP, and HTTP. |
| **Security** | **SpamAssassin / Rspamd Deep Content Scanning** | Medium | Heuristic, Bayesian, and deep MIME content analysis complementing SpamGuard DNSBL reputation and greylisting. |

---

## Implementation Horizons

### Horizon 1: Protocols & Security Hardening
1. **SMTP Submission (Port 587)**: Authenticated submission with mandatory TLS and SPF/DKIM alignment checks.
2. **POP3 (RFC 1939)**: Minimal async listener on port 110/995 with UIDL and RETR support.
3. **Authentication Rate Limiting**: Leaky-bucket in-memory IP rate limiter protecting `/api/auth/login`, IMAP `LOGIN`, and SMTP `AUTH`.

### Horizon 2: Collaboration & Enterprise Directory
1. **ManageSieve Engine**: Sieve script execution engine evaluating rules during inbound message ingestion.
2. **LDAP / Active Directory Backend**: Pluggable authentication source alongside local SQLite user accounts.
3. **Quarantine Web Workspace**: Embedded interface in Svelte 5 Webmail and Admin Dashboard for spam management.

### Horizon 3: Calendaring & Contacts (CalDAV / CardDAV)
1. **vCard & iCalendar Parsers**: Storage and indexing of calendar events and address books.
2. **DAV XML Endpoint**: HTTP XML handler supporting `PROPFIND`, `REPORT`, `MKCALENDAR`.
