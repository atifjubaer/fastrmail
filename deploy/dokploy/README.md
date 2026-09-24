# FastrMail — Dokploy Official Template

This folder contains the official blueprint files for submission to [Dokploy Templates](https://github.com/Dokploy/templates).

## Files Included

- `docker-compose.yml`: Docker Compose definition adhering to Dokploy blueprint guidelines (no `container_name`, uses `expose` instead of hardcoded host port `8080`, persistent volume `fastrmail_data`).
- `template.toml`: Dokploy variable mappings (auto-generates domain, admin password, and environment variables).
- `meta.json`: Catalog metadata, version, links, and tags.
- `fastrmail.svg`: FastrMail logo.

## Submission Instructions

1. Fork https://github.com/Dokploy/templates
2. Copy this folder into `blueprints/fastrmail` (or `templates/fastrmail`).
3. Submit a Pull Request: `Add FastrMail — Single-binary enterprise mail server`.
