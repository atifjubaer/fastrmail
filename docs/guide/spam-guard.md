# SpamGuard & Reputation Defense

FastrMail incorporates a multi-layer anti-spam shield called **SpamGuard**.

---

## 1. DNSBL Reputation Lookups (RFC 5782)

FastrMail queries real-time DNSBL providers concurrently via `hickory-resolver`:
- `zen.spamhaus.org`
- `b.barracudacentral.org`

### Safe IP & Open Resolver Filtering
RFC 5782 specifies that real spam listings return an `A` record in `127.0.0.2` – `127.0.0.127`. Special return codes like `127.255.255.254` or `127.255.255.255` denote open public resolver refusal (e.g. Cloudflare `1.1.1.1` or Google `8.8.8.8`).

FastrMail validates the 4th octet:
```rust
if octets[0] == 127 && octets[1] == 0 && octets[2] == 0 && octets[3] >= 2 {
    return Ok(true); // Genuinely blacklisted spam sender
}
```

If listed, the connection is instantly rejected with `554 5.7.1 Service unavailable; Client host [IP] blocked using DNSBL`.

---

## 2. Dynamic Greylisting Engine

FastrMail tracks triplets: `(sender_ip, sender_email, recipient_email)`.

1. **Initial Connection**: If a sender triplet is unknown, FastrMail records it in SQLite with `first_seen_at = now()` and rejects the transaction with:
   ```text
   451 4.7.1 Greylisting in action, please try again in 5 minutes
   ```
2. **Re-delivery Window**: Legitimate MTAs conform to RFC standards and retry.
   - If retried before 5 minutes: Deferred again (`451`).
   - If retried after 5 minutes: FastrMail accepts the message and records `passed = 1`.
3. **Whitelisting**: Once verified, subsequent messages from that sender proceed without delay.
