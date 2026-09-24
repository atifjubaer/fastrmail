//! FastrMail JMAP — JMAP JSON API (RFC 8620/8621) implementation.
//!
//! This crate will be fully implemented in a future phase.

/// Placeholder for the JMAP server.
pub struct JmapServer;

impl JmapServer {
    /// Create a new JMAP server instance.
    pub fn new() -> Self {
        Self
    }
}

impl Default for JmapServer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jmap_server_creation() {
        let _server = JmapServer::new();
        let _default = JmapServer::default();
    }
}
