# CLI Management Commands

FastrMail ships as a self-contained command-line executable.

---

## Command Reference

### Starting the Server
Runs SMTP (2525), IMAP (1143), HTTP/JMAP/UI (8080), and outbound retry workers concurrently:
```bash
./fastrmail
```

### Generating DKIM Keys
Generates a 2048-bit RSA key pair for a domain, records it into the database, and outputs the exact DNS TXT record for publication:
```bash
./fastrmail --generate-dkim example.com
```

### Running Tests
Executes the comprehensive workspace test suite:
```bash
cargo test --workspace
```
