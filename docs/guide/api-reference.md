# REST API & Webmail Endpoints

FastrMail provides modern HTTP REST endpoints on port `8080` for administrative automation, transactional email dispatch, and Webmail client integration.

---

## System Endpoints

### `GET /api/health`
Healthcheck endpoint for Docker and load balancers.
- **Response**: `200 OK`
```json
{
  "status": "healthy",
  "version": "0.1.0"
}
```

---

## Transactional Sending API

### `POST /api/send`
Queues an email for immediate outbound delivery.
- **Request Body**:
```json
{
  "from": "notifications@example.com",
  "to": ["user@destination.com"],
  "subject": "Order Confirmation #1024",
  "body": "Thank you for your order!"
}
```
- **Response**: `200 OK`
```json
{
  "status": "queued",
  "message_id": "9b1deb4d-3b7d-4bad-9bdd-2b0d7b3dcb6d"
}
```

---

## Webmail Client Endpoints

| Method | Endpoint | Description |
|:-------|:---------|:------------|
| `POST` | `/api/auth/login` | Authenticate user, verify password with Argon2id |
| `GET` | `/api/mail/inbox` | List messages in `INBOX` |
| `POST` | `/api/mail/send` | Send an email from webmail |
| `GET` | `/api/mail/message/:id` | Retrieve raw or parsed email content |

---

## Admin Management Endpoints

| Method | Endpoint | Description |
|:-------|:---------|:------------|
| `GET` | `/api/admin/domains` | List all domains and DKIM selectors |
| `POST` | `/api/admin/domains` | Provision new domain and DKIM key pair |
| `GET` | `/api/admin/accounts` | List email user accounts |
| `POST` | `/api/admin/accounts` | Create account with Argon2id hashed password |
| `GET` | `/api/admin/stats` | System statistics (messages sent, received, queue depth) |
