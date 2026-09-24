//! FastrMail Search — Tantivy-based full-text search engine.
//!
//! This crate will be fully implemented in Phase 2+ with Tantivy indexing.

/// Placeholder for the search engine.
pub struct SearchEngine;

impl SearchEngine {
    /// Create a new search engine instance.
    pub fn new() -> Self {
        Self
    }
}

impl Default for SearchEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_engine_creation() {
        let _engine = SearchEngine::new();
        let _default = SearchEngine::default();
    }
}
