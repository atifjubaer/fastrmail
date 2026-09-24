//! FastrMail IMAP — IMAP4rev2 server implementation.
//!
//! This crate will be fully implemented in Phase 3.

/// Placeholder for the IMAP server.
pub struct ImapServer;

impl ImapServer {
    /// Create a new IMAP server instance.
    pub fn new() -> Self {
        Self
    }
}

impl Default for ImapServer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_imap_server_creation() {
        let _server = ImapServer::new();
        let _default = ImapServer::default();
    }
}
