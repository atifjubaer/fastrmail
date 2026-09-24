# Docker & Dokploy Deployment

FastrMail provides a production-optimized multi-stage `Dockerfile` and a `docker-compose.yml` for turnkey hosting on Dokploy, Coolify, or any Linux VPS.

---

## Docker Compose Configuration

Here is the production `docker-compose.yml`:

```yaml
services:
  fastrmail:
    build:
      context: .
      dockerfile: Dockerfile
    image: fastrmail:latest
    container_name: fastrmail
    restart: unless-stopped
    ports:
      # SMTP Inbound (RFC 5321)
      - "25:2525"
      # IMAP4rev2 (RFC 9051)
      - "143:1143"
      # Webmail, Admin Dashboard & REST/JMAP API
      - "8080:8080"
    environment:
      - RUST_LOG=info
      - FASTRMAIL_DATA_DIR=/app/data
    volumes:
      - ./fastrmail_data:/app/data
    healthcheck:
      test: ["CMD-SHELL", "wget -qO- http://127.0.0.1:8080/api/health || exit 1"]
      interval: 30s
      timeout: 5s
      retries: 3
      start_period: 10s
```

---

## Deploying on Dokploy or Coolify

1. **Create an Application**:
   - Point your Dokploy / Coolify repository source to `https://github.com/atifjubaer/fastrmail.git` (or your private fork).
   - Select **Docker Compose** or **Dockerfile** deployment.
2. **Mount Persistent Storage**:
   - Ensure `/app/data` is mounted to a persistent volume (e.g. `/var/lib/dokploy/volumes/fastrmail-data:/app/data`).
   - This directory stores the SQLite database, attachments, DKIM private keys, and Tantivy search indexes.
3. **Expose Ports**:
   - Expose port `25` (Inbound SMTP), port `143` (IMAP), and port `8080` (HTTP Webmail & APIs).
   - Route your reverse proxy (Traefik / Caddy / Nginx) to container port `8080` with Let's Encrypt SSL/TLS certificates for `mail.yourdomain.com`.
