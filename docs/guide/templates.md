# Templates & App Store Publishing

This guide explains how to package, configure, and publish **FastrMail** as a one-click template for self-hosting platforms like **Dokploy** and **Coolify**, and explains the project's folder architecture.

---

## 1. Directory & Root Folder Structure

Understanding which "root" is being referenced is essential when building and deploying templates:

```
fastrmail/                              <-- [Repository Root]
├── Cargo.toml                          <-- Workspace manifest
├── Dockerfile                          <-- Multi-stage build definition
├── crates/                             <-- Rust core engine & protocols
├── web/                                <-- Svelte 5 Webmail & Admin SPAs
├── docs/                               <-- VitePress documentation
│
├── deploy/                             <-- [Deploy Root: Packaging Blueprints]
│   ├── dokploy/                        <-- Dokploy Blueprint files
│   │   ├── docker-compose.yml          <-- Dokploy-compliant compose
│   │   ├── template.toml               <-- UI variable mappings & env bindings
│   │   ├── meta.json                   <-- App store metadata & catalog info
│   │   ├── fastrmail.svg               <-- Catalog card icon
│   │   └── README.md                   <-- PR submission guide
│   │
│   └── coolify/                        <-- Coolify Template files
│       ├── docker-compose.yaml         <-- Coolify compose with TCP routing
│       ├── service.yaml                <-- Service catalog schema
│       └── README.md                   <-- PR submission guide
│
└── /app/                               <-- [Container Root (Inside Docker)]
    ├── /usr/local/bin/fastrmail        <-- Compiled production binary
    ├── /app/web/admin/dist             <-- Embedded Admin UI assets
    ├── /app/web/webmail/dist           <-- Embedded Webmail assets
    └── /app/data/                      <-- Persistent volume (DB, Tantivy index)
```

### Key Distinctions
- **Repository Root (`/`)**: Contains the source code, build manifests, CI/CD workflows, and documentation.
- **Deploy Folder (`/deploy/`)**: Isolated blueprints ready to copy into official upstream template repositories (`Dokploy/templates` or `coollabsio/coolify`).
- **Container Working Directory (`/app/`)**: Where the runtime executes inside Docker, mounting persistent volumes to `/app/data`.

---

## 2. Creating a Dokploy Template

Dokploy maintains a community catalog in [Dokploy/templates](https://github.com/Dokploy/templates). In Dokploy, each template lives in `blueprints/<id>/` and requires three files plus an SVG icon.

### 1. `docker-compose.yml` Rules
- **No `container_name`**: Dokploy automatically generates unique container IDs per deployment.
- **No custom bridge networks**: Dokploy connects containers to its internal reverse proxy via `dokploy-network`.
- **Use `expose: ["8080"]`**: Do not bind host ports like `8080:8080` for web traffic. Traefik routes domain traffic internally. Mail ports (25, 587, 143, 110) remain exposed under `ports:` so external mail clients can connect.
- **Parameterized variables**: Use `${FASTRMAIL_HOSTNAME}` and `${FASTRMAIL_ADMIN_PASS}` without hardcoded secrets.

```yaml
services:
  fastrmail:
    image: ghcr.io/atifjubaer/fastrmail:latest
    restart: unless-stopped
    ports:
      - "${SMTP_PORT:-25}:2525"
      - "${SMTP_SUBMISSION_PORT:-587}:2526"
      - "${IMAP_PORT:-143}:1143"
      - "${POP3_PORT:-110}:1110"
    expose:
      - "8080"
    environment:
      - RUST_LOG=info
      - FASTRMAIL_DATA_DIR=/app/data
      - FASTRMAIL_BIND_SMTP=0.0.0.0:2525
      - FASTRMAIL_BIND_SUBMISSION=0.0.0.0:2526
      - FASTRMAIL_BIND_IMAP=0.0.0.0:1143
      - FASTRMAIL_BIND_POP3=0.0.0.0:1110
      - FASTRMAIL_BIND_HTTP=0.0.0.0:8080
      - FASTRMAIL_HOSTNAME=${FASTRMAIL_HOSTNAME}
      - FASTRMAIL_PRIMARY_DOMAIN=${FASTRMAIL_PRIMARY_DOMAIN}
      - FASTRMAIL_ADMIN_USER=${FASTRMAIL_ADMIN_USER}
      - FASTRMAIL_ADMIN_PASS=${FASTRMAIL_ADMIN_PASS}
    volumes:
      - fastrmail_data:/app/data

volumes:
  fastrmail_data:
```

### 2. `template.toml` (Variable Mapping)
`template.toml` tells Dokploy how to prompt the user during installation:
- `${domain}`: Automatically asks the user for their desired domain or generates a free `.sslip.io` hostname.
- `${password:32}`: Auto-generates a secure 32-character admin password.

```toml
[variables]
main_domain = "${domain}"
admin_password = "${password:32}"

[[config.domains]]
service = "fastrmail"
port = 8080
host = "${main_domain}"
path = "/"

[[config.env]]
service = "fastrmail"
key = "FASTRMAIL_HOSTNAME"
value = "${main_domain}"

[[config.env]]
service = "fastrmail"
key = "FASTRMAIL_PRIMARY_DOMAIN"
value = "${main_domain}"

[[config.env]]
service = "fastrmail"
key = "FASTRMAIL_ADMIN_USER"
value = "admin"

[[config.env]]
service = "fastrmail"
key = "FASTRMAIL_ADMIN_PASS"
value = "${admin_password}"
```

### 3. `meta.json` (Catalog Metadata)
```json
{
  "id": "fastrmail",
  "name": "FastrMail",
  "version": "1.0.0",
  "description": "Enterprise-grade single-binary mail server built in Rust with SMTP, IMAP4rev2, POP3, JMAP, Sieve, and Lucene search.",
  "logo": "fastrmail.svg",
  "links": {
    "github": "https://github.com/atifjubaer/fastrmail",
    "website": "https://fastrmail-landing.vercel.app",
    "docs": "https://fastrmail-landing.vercel.app"
  },
  "tags": ["mail", "email", "smtp", "imap", "rust"]
}
```

---

## 3. Creating a Coolify Template

Coolify templates reside in `templates/compose/<service>.yaml` inside the [coollabsio/coolify](https://github.com/coollabsio/coolify) repository.

### Coolify TCP Handling
Coolify uses labels to map raw TCP protocols through its proxy:
```yaml
labels:
  - "coolify.managed=true"
  - "coolify.tcp.port.2525=25"
  - "coolify.tcp.port.2526=587"
  - "coolify.tcp.port.1143=143"
  - "coolify.tcp.port.1110=110"
```

---

## 4. Troubleshooting: Fixing Dokploy "404 Page Not Found"

If your Dokploy container shows **Active / Running** but visiting the domain returns `404 page not found`:

### Cause 1: Manual Labels Conflicting with Dokploy UI
If your `docker-compose.yml` has manual `labels:` (e.g., `traefik.http.routers...`), Traefik receives duplicate, conflicting router definitions.
- **Fix**: Remove manual Traefik labels from the compose file. Let Dokploy's **Domains** tab inject the labels automatically.

### Cause 2: Domain Added but Service Not Redeployed
Dokploy does not apply routing changes immediately when a domain is saved in the **Domains** tab.
- **Fix**: Go to **Deployments** or **General** tab and click **Redeploy**.

### Cause 3: Port Mismatch in Domains Tab
The port in the **Domains** tab must match the container's internal HTTP port (`8080`), not an external mail port or host port.
- **Fix**: Verify Port is set to `8080` and Service is set to `fastrmail`.
