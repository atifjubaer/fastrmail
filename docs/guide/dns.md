# DNS Configuration (SPF, DKIM, DMARC)

Proper DNS records are vital for 100% email deliverability and inbound authentication verification.

---

## 1. MX Record (Mail Exchanger)

Direct incoming mail to your FastrMail server:

| Type | Name / Host | Value | Priority |
|:-----|:------------|:------|:---------|
| `MX` | `@` (root) | `mail.example.com` | `10` |

---

## 2. A / AAAA Record

Point your mail subdomain to your server's public IP address:

| Type | Name / Host | Value |
|:-----|:------------|:------|
| `A` | `mail` | `198.51.100.25` |

---

## 3. Reverse DNS (PTR Record)

Configure your VPS hosting provider to set the Reverse DNS (PTR) of your public IP address to `mail.example.com`. Major providers (Gmail, Outlook, Yahoo) reject mail without matching PTR records.

---

## 4. SPF Record (Sender Policy Framework)

Authorize your FastrMail server to send email on behalf of your domain:

| Type | Name / Host | Value |
|:-----|:------------|:------|
| `TXT` | `@` | `v=spf1 mx ip4:198.51.100.25 -all` |

---

## 5. DKIM Record (DomainKeys Identified Mail)

Generate the key in FastrMail CLI:
```bash
fastrmail --generate-dkim example.com
```

Add the generated TXT record to DNS:

| Type | Name / Host | Value |
|:-----|:------------|:------|
| `TXT` | `default._domainkey` | `v=DKIM1; k=rsa; p=MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ...` |

---

## 6. DMARC Record

Enforce policy for SPF/DKIM failures:

| Type | Name / Host | Value |
|:-----|:------------|:------|
| `TXT` | `_dmarc` | `v=DMARC1; p=reject; sp=reject; adkim=r; aspf=r; pct=100; rua=mailto:postmaster@example.com` |
