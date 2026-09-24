//! FastrMail Search — Tantivy-based full-text search engine.
//!
//! Provides embedded, high-performance indexing and querying of email messages
//! across subject, body, sender, recipient, and mailbox fields.

use std::path::Path;
use std::sync::Mutex;

use anyhow::{Context, Result};
use tantivy::collector::TopDocs;
use tantivy::directory::MmapDirectory;
use tantivy::query::{BooleanQuery, Query, QueryParser, TermQuery};
use tantivy::schema::{Field, IndexRecordOption, Schema, Value, STORED, STRING, TEXT};
use tantivy::{Index, IndexReader, IndexWriter, ReloadPolicy, TantivyDocument, Term};
use tracing::info;

use fastrmail_core::Message;

/// Holds the schema field references for the search index.
#[derive(Clone, Copy)]
pub struct SearchFields {
    pub message_id: Field,
    pub account_id: Field,
    pub mailbox_id: Field,
    pub subject: Field,
    pub body: Field,
    pub from: Field,
    pub to: Field,
}

impl SearchFields {
    pub fn build_schema() -> (Schema, Self) {
        let mut builder = Schema::builder();
        let message_id = builder.add_text_field("message_id", STRING | STORED);
        let account_id = builder.add_text_field("account_id", STRING | STORED);
        let mailbox_id = builder.add_text_field("mailbox_id", STRING | STORED);
        let subject = builder.add_text_field("subject", TEXT | STORED);
        let body = builder.add_text_field("body", TEXT);
        let from = builder.add_text_field("from", TEXT | STORED);
        let to = builder.add_text_field("to", TEXT | STORED);
        let schema = builder.build();

        (
            schema,
            Self {
                message_id,
                account_id,
                mailbox_id,
                subject,
                body,
                from,
                to,
            },
        )
    }
}

/// The core Tantivy search engine.
pub struct SearchEngine {
    index: Index,
    reader: IndexReader,
    writer: Mutex<IndexWriter>,
    pub fields: SearchFields,
}

impl SearchEngine {
    /// Open or create a persistent Tantivy search index at `index_path`.
    pub fn new(index_path: &str) -> Result<Self> {
        let path = Path::new(index_path);
        std::fs::create_dir_all(path)
            .with_context(|| format!("Failed to create search index directory: {index_path}"))?;

        let (schema, fields) = SearchFields::build_schema();
        let dir = MmapDirectory::open(path)
            .context("Failed to open MmapDirectory for Tantivy")?;
        let index = Index::open_or_create(dir, schema)
            .context("Failed to open or create Tantivy index")?;

        let writer = index
            .writer(50_000_000)
            .context("Failed to create Tantivy IndexWriter")?;
        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()
            .context("Failed to create Tantivy IndexReader")?;

        info!("Tantivy search engine initialized at {index_path}");

        Ok(Self {
            index,
            reader,
            writer: Mutex::new(writer),
            fields,
        })
    }

    /// Create an in-memory Tantivy search index (ideal for unit testing).
    pub fn new_in_ram() -> Result<Self> {
        let (schema, fields) = SearchFields::build_schema();
        let index = Index::create_in_ram(schema);

        let writer = index
            .writer(15_000_000)
            .context("Failed to create in-ram Tantivy IndexWriter")?;
        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()
            .context("Failed to create in-ram Tantivy IndexReader")?;

        Ok(Self {
            index,
            reader,
            writer: Mutex::new(writer),
            fields,
        })
    }

    /// Index or update an email message with its metadata and extracted plain-text body.
    pub fn index_message(&self, msg: &Message, raw_body_text: &str) -> Result<()> {
        let mut document = TantivyDocument::default();
        document.add_text(self.fields.message_id, &msg.id);
        document.add_text(self.fields.account_id, &msg.account_id);
        document.add_text(self.fields.mailbox_id, &msg.mailbox_id);

        if let Some(sub) = &msg.parsed_subject {
            document.add_text(self.fields.subject, sub);
        }
        if !raw_body_text.is_empty() {
            document.add_text(self.fields.body, raw_body_text);
        }
        if let Some(from) = &msg.parsed_from {
            document.add_text(self.fields.from, from);
        }
        if let Some(to) = &msg.parsed_to {
            document.add_text(self.fields.to, to);
        }

        let term = Term::from_field_text(self.fields.message_id, &msg.id);
        let mut writer = self.writer.lock().unwrap();
        writer.delete_term(term);
        writer.add_document(document)?;
        writer.commit()?;

        // Trigger reader reload
        self.reader.reload()?;
        Ok(())
    }

    /// Delete a message from the search index by ID.
    pub fn delete_message(&self, message_id: &str) -> Result<()> {
        let term = Term::from_field_text(self.fields.message_id, message_id);
        let mut writer = self.writer.lock().unwrap();
        writer.delete_term(term);
        writer.commit()?;
        self.reader.reload()?;
        Ok(())
    }

    /// Search messages for a specific account matching the given full-text query string.
    pub fn search(&self, account_id: &str, query_str: &str, limit: usize) -> Result<Vec<String>> {
        let searcher = self.reader.searcher();

        let query_parser = QueryParser::for_index(
            &self.index,
            vec![
                self.fields.subject,
                self.fields.body,
                self.fields.from,
                self.fields.to,
            ],
        );

        let user_query = if query_str.trim().is_empty() {
            Box::new(tantivy::query::AllQuery) as Box<dyn Query>
        } else {
            query_parser
                .parse_query(query_str)
                .unwrap_or_else(|_| Box::new(tantivy::query::AllQuery))
        };

        // Scope to account_id
        let account_term = Term::from_field_text(self.fields.account_id, account_id);
        let account_query: Box<dyn Query> =
            Box::new(TermQuery::new(account_term, IndexRecordOption::Basic));

        let combined_query = BooleanQuery::intersection(vec![user_query, account_query]);

        let top_docs = searcher.search(&combined_query, &TopDocs::with_limit(limit))?;

        let mut results = Vec::new();
        for (_score, doc_address) in top_docs {
            let retrieved_doc: TantivyDocument = searcher.doc(doc_address)?;
            if let Some(val) = retrieved_doc.get_first(self.fields.message_id) {
                if let Some(id_str) = val.as_str() {
                    results.push(id_str.to_string());
                }
            }
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn create_test_message(id: &str, account_id: &str, subject: &str, from: &str) -> Message {
        Message {
            id: id.to_string(),
            mailbox_id: "mb-inbox".to_string(),
            account_id: account_id.to_string(),
            uid: 1,
            modseq: 1,
            blob_id: format!("blob_{id}"),
            size_bytes: 512,
            parsed_subject: Some(subject.to_string()),
            parsed_from: Some(from.to_string()),
            parsed_to: Some("receiver@fastrmail.com".to_string()),
            internal_date: Utc::now(),
            flags: "[]".to_string(),
        }
    }

    #[test]
    fn test_tantivy_index_and_search() {
        let engine = SearchEngine::new_in_ram().unwrap();

        let m1 = create_test_message("msg-1", "acc-alice", "Monthly Invoice #1024", "billing@stripe.com");
        let m2 = create_test_message("msg-2", "acc-alice", "Urgent security update required", "support@github.com");
        let m3 = create_test_message("msg-3", "acc-alice", "Lunch meeting tomorrow?", "bob@friend.com");
        let m4_bob = create_test_message("msg-4", "acc-bob", "Invoice for Bob", "billing@stripe.com");

        engine.index_message(&m1, "Here is your invoice for $49.00 USD for the subscription.").unwrap();
        engine.index_message(&m2, "Please update your password immediately to protect your repositories.").unwrap();
        engine.index_message(&m3, "Hey Alice, let's grab pizza tomorrow at noon.").unwrap();
        engine.index_message(&m4_bob, "Bob's private invoice.").unwrap();

        // 1. Search for 'invoice' for Alice
        let alice_invoices = engine.search("acc-alice", "invoice", 10).unwrap();
        assert_eq!(alice_invoices.len(), 1);
        assert_eq!(alice_invoices[0], "msg-1");

        // 2. Search for body text 'pizza'
        let pizza_results = engine.search("acc-alice", "pizza", 10).unwrap();
        assert_eq!(pizza_results.len(), 1);
        assert_eq!(pizza_results[0], "msg-3");

        // 3. Search for sender 'github'
        let github_results = engine.search("acc-alice", "github", 10).unwrap();
        assert_eq!(github_results.len(), 1);
        assert_eq!(github_results[0], "msg-2");

        // 4. Bob's invoice should not show in Alice's search
        let alice_bobs = engine.search("acc-alice", "Bob", 10).unwrap();
        // Alice only receives msg-3 mentioning "bob@friend.com"
        assert_eq!(alice_bobs, vec!["msg-3"]);

        // 5. Delete msg-1 and verify it no longer matches
        engine.delete_message("msg-1").unwrap();
        let alice_invoices_after = engine.search("acc-alice", "invoice", 10).unwrap();
        assert!(alice_invoices_after.is_empty());
    }
}
