//! FastrMail Auth — DKIM/SPF/DMARC/ARC verification and user authentication.
//!
//! This crate will be fully implemented in Phase 2+ with mail-auth integration.

/// Placeholder for authentication service.
pub struct AuthService;

impl AuthService {
    /// Create a new authentication service instance.
    pub fn new() -> Self {
        Self
    }
}

impl Default for AuthService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_service_creation() {
        let _service = AuthService::new();
        let _default = AuthService::default();
    }
}
