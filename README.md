# FastrMail

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.81+-orange.svg)](https://www.rust-lang.org/)
[![Docker](https://img.shields.io/badge/docker-ready-blue.svg)](https://www.docker.com/)
[![Tests](https://img.shields.io/badge/tests-47%2F47%20passing-brightgreen.svg)](https://github.com/atifjubaer/fastrmail)

<div align="center">

# The Open-Source, Enterprise-Grade Mail Server

A high-performance, single-binary mail server written in Rust. Built for speed, security, and simplicity.

[Features](#features) • [Quick Start](#quick-start) • [Documentation](#documentation) • [Contributing](#contributing)

</div>

## 🚀 Features

### Universal Protocols
- **SMTP**: RFC 5321 Inbound & Outbound (with SPF/DKIM/DMARC/SpamGuard).
- **IMAP4rev2**: RFC 9051 full-featured mail server.
- **JMAP**: RFC 8620/8621 for modern, fast, offline-first clients.
- **POP3**: Legacy protocol support (Roadmap).

### Advanced Security
- **Greylisting**: 5-minute retry window to enforce legitimate senders.
- **DNSBL Reputation**: Real-time checks against Spamhaus & Barracuda (RFC 5782 compliant).
- **Argon2id Password Hashing**: Military-grade account security.
- **DKIM Signing**: Automated 2048-bit RSA signing for outbound emails.

### Performance & Search
- **Embedded Tantivy**: Sub-millisecond full-text search across all emails.
- **Zero External Dependencies**: Runs on a single Alpine Linux binary (<35MB RAM idle).
- **Async Architecture**: 100,000+ concurrent connections handled via Tokio.

### Modern UI
- **Built-in Webmail**: Svelte 5 SPA included.
- **Admin Dashboard**: Real-time metrics, DKIM management, and queue monitoring.

## 📦 Quick Start

### Docker (Recommended)

```bash
# Clone the repository
git clone https://github.com/atifjubaer/fastrmail.git
cd fastrmail

# Copy the example environment file
cp .env.example .env

# Start the server
docker compose up -d
```

### Native Binary

```bash
# Build the binary
cargo build --release

# Run the server
./target/release/fastrmail
```

## 📖 Documentation

For detailed guides on setup, DNS configuration, and architecture, visit the [official documentation](https://fastrmail.vercel.app).

## 🤝 Contributing

We welcome contributions from the community! We are aiming for full enterprise feature parity with commercial solutions.

1. Fork the repository.
2. Create a feature branch (`git checkout -b feature/amazing-feature`).
3. Commit your changes (`git commit -m 'Add amazing feature'`).
4. Push to the branch (`git push origin feature/amazing-feature`).
5. Open a Pull Request.

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

This project was inspired by [Stalwart Mail Server](https://github.com/stalwartlabs/stalwart) and [SnappyMail](https://github.com/the-djmaze/snappymail). We aim to provide a fully free, open-source alternative to paid enterprise email solutions.
