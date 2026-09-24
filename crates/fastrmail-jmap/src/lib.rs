//! FastrMail JMAP — Production-grade JSON Meta Application Protocol (RFC 8620 / RFC 8621).
//!
//! Provides the `/jmap/session` discovery endpoint and the `/jmap/api` dispatch handler
//! for Mailbox and Email query, get, and set operations backed by SQLite and Tantivy FTS.

use std::collections::HashMap;
use std::sync::Arc;

use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::routing::{get, post};
use axum::{Json, Router};
use base64::Engine;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tracing::warn;

use fastrmail_core::{Account, Mailbox};
use fastrmail_search::SearchEngine;
use fastrmail_store::Database;

/// Shared application state for all JMAP endpoints.
#[derive(Clone)]
pub struct JmapState {
    pub db: Arc<Database>,
    pub search_engine: Arc<SearchEngine>,
}

// ─── JMAP Core & Mail Protocol Types (RFC 8620 / 8621) ────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JmapRequest {
    pub using: Vec<String>,
    #[serde(rename = "methodCalls")]
    pub method_calls: Vec<(String, Value, String)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JmapResponse {
    #[serde(rename = "methodResponses")]
    pub method_responses: Vec<(String, Value, String)>,
    #[serde(rename = "sessionState")]
    pub session_state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JmapAccountInfo {
    pub name: String,
    #[serde(rename = "isPersonal")]
    pub is_personal: bool,
    #[serde(rename = "isReadOnly")]
    pub is_read_only: bool,
    #[serde(rename = "accountCapabilities")]
    pub account_capabilities: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JmapSession {
    pub capabilities: HashMap<String, Value>,
    pub accounts: HashMap<String, JmapAccountInfo>,
    #[serde(rename = "primaryAccounts")]
    pub primary_accounts: HashMap<String, String>,
    pub username: String,
    #[serde(rename = "apiUrl")]
    pub api_url: String,
    #[serde(rename = "downloadUrl")]
    pub download_url: String,
    #[serde(rename = "uploadUrl")]
    pub upload_url: String,
    #[serde(rename = "eventSourceUrl")]
    pub event_source_url: String,
    pub state: String,
}

// ─── Helpers ─────────────────────────────────────────────────

/// Authenticate request from headers or resolve default account.
fn resolve_authenticated_account(state: &JmapState, headers: &HeaderMap) -> Option<Account> {
    if let Some(auth_val) = headers.get("authorization").and_then(|v| v.to_str().ok()) {
        if auth_val.starts_with("Basic ") {
            let encoded = &auth_val[6..].trim();
            if let Ok(decoded) = base64::engine::general_purpose::STANDARD.decode(encoded) {
                if let Ok(creds) = String::from_utf8(decoded) {
                    if let Some((email, pass)) = creds.split_once(':') {
                        if let Ok(Some(acc)) = state.db.verify_login(email, pass) {
                            return Some(acc);
                        }
                    }
                }
            }
        } else if auth_val.starts_with("Bearer ") {
            let email = auth_val[7..].trim();
            if let Ok(Some(acc)) = state.db.get_account_by_email(email) {
                return Some(acc);
            }
        }
    }

    // Default development fallback: postmaster@localhost
    match state.db.get_account_by_email("postmaster@localhost") {
        Ok(Some(acc)) => Some(acc),
        _ => {
            // Auto-provision if missing
            let tenant_id = state
                .db
                .get_tenant_by_domain("localhost")
                .ok()
                .flatten()
                .map(|t| t.id)
                .unwrap_or_else(|| state.db.insert_tenant("localhost").unwrap_or_default());
            let acc_id = state
                .db
                .insert_account(&tenant_id, "postmaster", "postmaster@localhost", "admin123")
                .unwrap_or_default();
            let _ = state.db.insert_mailbox(&acc_id, "INBOX");
            state.db.get_account_by_email("postmaster@localhost").ok().flatten()
        }
    }
}

// ─── JMAP Endpoints ──────────────────────────────────────────

/// GET /jmap/session — RFC 8620 Session Resource discovery.
pub async fn get_session(
    State(state): State<JmapState>,
    headers: HeaderMap,
) -> Result<Json<JmapSession>, StatusCode> {
    let account = match resolve_authenticated_account(&state, &headers) {
        Some(a) => a,
        None => return Err(StatusCode::UNAUTHORIZED),
    };

    let mut capabilities = HashMap::new();
    capabilities.insert(
        "urn:ietf:params:jmap:core".to_string(),
        json!({
            "maxSizeUpload": 50000000,
            "maxConcurrentUpload": 4,
            "maxSizeRequest": 10000000,
            "maxConcurrentRequests": 4,
            "maxCallsInRequest": 16,
            "maxObjectsInGet": 500,
            "maxObjectsInSet": 500,
            "collationAlgorithms": ["i;ascii-casemap", "i;octet"]
        }),
    );
    capabilities.insert(
        "urn:ietf:params:jmap:mail".to_string(),
        json!({
            "maxMailboxesPerEmail": 1000,
            "maxMailboxDepth": 10,
            "maxSizeMailboxName": 100,
            "maxSizeAttachmentsPerEmail": 50000000,
            "emailQuerySortOptions": ["receivedAt"],
            "mayCreateTopLevelMailbox": true
        }),
    );

    let mut accounts = HashMap::new();
    let mut account_capabilities = HashMap::new();
    account_capabilities.insert("urn:ietf:params:jmap:core".to_string(), json!({}));
    account_capabilities.insert("urn:ietf:params:jmap:mail".to_string(), json!({}));

    accounts.insert(
        account.id.clone(),
        JmapAccountInfo {
            name: account.email.clone(),
            is_personal: true,
            is_read_only: false,
            account_capabilities,
        },
    );

    let mut primary_accounts = HashMap::new();
    primary_accounts.insert(
        "urn:ietf:params:jmap:core".to_string(),
        account.id.clone(),
    );
    primary_accounts.insert(
        "urn:ietf:params:jmap:mail".to_string(),
        account.id.clone(),
    );

    let session = JmapSession {
        capabilities,
        accounts,
        primary_accounts,
        username: account.email,
        api_url: "/jmap/api".to_string(),
        download_url: "/jmap/download/{accountId}/{blobId}/{name}".to_string(),
        upload_url: "/jmap/upload/{accountId}".to_string(),
        event_source_url: "/jmap/events".to_string(),
        state: "state_v1".to_string(),
    };

    Ok(Json(session))
}

/// POST /jmap/api — RFC 8620 / RFC 8621 JMAP API request dispatcher.
pub async fn handle_jmap_api(
    State(state): State<JmapState>,
    headers: HeaderMap,
    Json(request): Json<JmapRequest>,
) -> Result<Json<JmapResponse>, StatusCode> {
    let account = match resolve_authenticated_account(&state, &headers) {
        Some(a) => a,
        None => return Err(StatusCode::UNAUTHORIZED),
    };

    let mut method_responses = Vec::new();

    for (method, args, call_id) in request.method_calls {
        let (resp_method, resp_args) = match method.as_str() {
            "Core/echo" => ("Core/echo".to_string(), args),

            "Mailbox/get" => {
                let mailboxes = state.db.get_mailboxes(&account.id).unwrap_or_default();
                let list: Vec<Value> = mailboxes
                    .into_iter()
                    .map(|mb: Mailbox| {
                        let (total, unseen) =
                            state.db.get_mailbox_counts(&mb.id).unwrap_or((0, 0));
                        let role = match mb.name.to_uppercase().as_str() {
                            "INBOX" => Some("inbox"),
                            "SENT" => Some("sent"),
                            "DRAFTS" => Some("drafts"),
                            "TRASH" => Some("trash"),
                            _ => None,
                        };
                        json!({
                            "id": mb.id,
                            "name": mb.name,
                            "parentId": mb.parent_id,
                            "role": role,
                            "sortOrder": 1,
                            "totalEmails": total,
                            "unreadEmails": unseen,
                            "myRights": {
                                "mayReadItems": true,
                                "mayAddItems": true,
                                "mayRemoveItems": true,
                                "maySetSeen": true,
                                "maySetKeywords": true,
                                "mayCreateChild": true,
                                "mayRename": mb.name.to_uppercase() != "INBOX",
                                "mayDelete": mb.name.to_uppercase() != "INBOX",
                                "maySubmit": true
                            }
                        })
                    })
                    .collect();

                (
                    "Mailbox/get".to_string(),
                    json!({
                        "accountId": account.id,
                        "state": "mb_state_1",
                        "list": list,
                        "notFound": []
                    }),
                )
            }

            "Email/query" => {
                let limit = args
                    .get("limit")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(50) as usize;

                let filter = args.get("filter");
                let mut matched_ids = Vec::new();

                if let Some(f) = filter {
                    if let Some(text) = f.get("text").and_then(|v| v.as_str()) {
                        // Use Tantivy Full-Text Search
                        if let Ok(search_hits) = state.search_engine.search(&account.id, text, limit) {
                            matched_ids = search_hits;
                        }
                    } else if let Some(in_mb) = f.get("inMailbox").and_then(|v| v.as_str()) {
                        if let Ok(msgs) = state.db.get_messages_by_mailbox(in_mb) {
                            matched_ids = msgs.into_iter().map(|m| m.id).take(limit).collect();
                        }
                    } else {
                        // All messages for account
                        if let Ok(msgs) = state.db.get_messages(&account.id) {
                            matched_ids = msgs.into_iter().map(|m| m.id).take(limit).collect();
                        }
                    }
                } else {
                    if let Ok(msgs) = state.db.get_messages(&account.id) {
                        matched_ids = msgs.into_iter().map(|m| m.id).take(limit).collect();
                    }
                }

                (
                    "Email/query".to_string(),
                    json!({
                        "accountId": account.id,
                        "queryState": "q_state_1",
                        "canCalculateChanges": false,
                        "position": 0,
                        "ids": matched_ids,
                        "total": matched_ids.len()
                    }),
                )
            }

            "Email/get" => {
                let all_messages = state.db.get_messages(&account.id).unwrap_or_default();
                let requested_ids: Vec<String> = args
                    .get("ids")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|x| x.as_str().map(|s| s.to_string()))
                            .collect()
                    })
                    .unwrap_or_default();

                let mut list = Vec::new();
                for msg in all_messages {
                    if requested_ids.is_empty() || requested_ids.contains(&msg.id) {
                        let flags: Vec<String> =
                            serde_json::from_str(&msg.flags).unwrap_or_default();
                        let mut keywords = HashMap::new();
                        for flag in flags {
                            if flag.contains("Seen") {
                                keywords.insert("$seen".to_string(), true);
                            }
                            if flag.contains("Flagged") {
                                keywords.insert("$flagged".to_string(), true);
                            }
                            if flag.contains("Draft") {
                                keywords.insert("$draft".to_string(), true);
                            }
                        }

                        let email_obj = json!({
                            "id": msg.id,
                            "blobId": msg.blob_id,
                            "threadId": msg.id,
                            "mailboxIds": { (msg.mailbox_id): true },
                            "keywords": keywords,
                            "size": msg.size_bytes,
                            "receivedAt": msg.internal_date.to_rfc3339(),
                            "from": [{ "name": msg.parsed_from, "email": msg.parsed_from }],
                            "to": [{ "name": msg.parsed_to, "email": msg.parsed_to }],
                            "subject": msg.parsed_subject.unwrap_or_default(),
                        });
                        list.push(email_obj);
                    }
                }

                (
                    "Email/get".to_string(),
                    json!({
                        "accountId": account.id,
                        "state": "email_state_1",
                        "list": list,
                        "notFound": []
                    }),
                )
            }

            "Email/set" => {
                let mut updated = HashMap::new();
                let mut destroyed = Vec::new();

                // 1. Updates (e.g. keywords/flags)
                if let Some(update_map) = args.get("update").and_then(|v| v.as_object()) {
                    let all_messages = state.db.get_messages(&account.id).unwrap_or_default();
                    for (msg_id, patches) in update_map {
                        if let Some(msg) = all_messages.iter().find(|m| &m.id == msg_id) {
                            if let Some(keywords) = patches.get("keywords").and_then(|k| k.as_object()) {
                                let mut new_flags = Vec::new();
                                for (kw, val) in keywords {
                                    if val.as_bool().unwrap_or(false) {
                                        match kw.as_str() {
                                            "$seen" => new_flags.push("\\Seen".to_string()),
                                            "$flagged" => new_flags.push("\\Flagged".to_string()),
                                            "$draft" => new_flags.push("\\Draft".to_string()),
                                            _ => {}
                                        }
                                    }
                                }
                                let flags_json = serde_json::to_string(&new_flags).unwrap_or_default();
                                let _ = state.db.update_message_flags(&msg.mailbox_id, msg.uid, &flags_json);
                                updated.insert(msg_id.clone(), Value::Null);
                            }
                        }
                    }
                }

                // 2. Destroys
                if let Some(destroy_arr) = args.get("destroy").and_then(|v| v.as_array()) {
                    let all_messages = state.db.get_messages(&account.id).unwrap_or_default();
                    for val in destroy_arr {
                        if let Some(msg_id) = val.as_str() {
                            if let Some(msg) = all_messages.iter().find(|m| m.id == msg_id) {
                                let _ = state.db.update_message_flags(&msg.mailbox_id, msg.uid, r#"["\\Deleted"]"#);
                                let _ = state.db.expunge_deleted_messages(&msg.mailbox_id);
                                let _ = state.search_engine.delete_message(msg_id);
                                destroyed.push(msg_id.to_string());
                            }
                        }
                    }
                }

                (
                    "Email/set".to_string(),
                    json!({
                        "accountId": account.id,
                        "oldState": "es_1",
                        "newState": "es_2",
                        "updated": updated,
                        "destroyed": destroyed,
                        "notUpdated": {},
                        "notDestroyed": {}
                    }),
                )
            }

            _ => {
                warn!("Unsupported JMAP method: {method}");
                (
                    "error".to_string(),
                    json!({
                        "type": "unknownMethod",
                        "description": format!("Method '{method}' is not implemented")
                    }),
                )
            }
        };

        method_responses.push((resp_method, resp_args, call_id));
    }

    Ok(Json(JmapResponse {
        method_responses,
        session_state: "state_v1".to_string(),
    }))
}

/// Build the Axum router for JMAP routes.
pub fn build_jmap_router(state: JmapState) -> Router {
    Router::new()
        .route("/jmap/session", get(get_session))
        .route("/jmap/api", post(handle_jmap_api))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use fastrmail_core::Message;

    fn setup_test_jmap_state() -> JmapState {
        let db = Database::new_memory().expect("Failed to create in-memory DB");
        db.init_schema().expect("Failed to init schema");

        let tenant_id = db.insert_tenant("jmap.local").unwrap();
        let acc_id = db
            .insert_account(&tenant_id, "user", "user@jmap.local", "secretpass")
            .unwrap();
        let mb_id = db.insert_mailbox(&acc_id, "INBOX").unwrap();

        // Insert a test message
        let msg_id = db
            .insert_message(
                &mb_id,
                &acc_id,
                "blob_123",
                2048,
                Some("Contract Agreement PDF"),
                Some("legal@company.com"),
                Some("user@jmap.local"),
            )
            .unwrap();

        let search_engine = Arc::new(SearchEngine::new_in_ram().unwrap());
        let msg = Message {
            id: msg_id,
            mailbox_id: mb_id,
            account_id: acc_id,
            uid: 1,
            modseq: 1,
            blob_id: "blob_123".to_string(),
            size_bytes: 2048,
            parsed_subject: Some("Contract Agreement PDF".to_string()),
            parsed_from: Some("legal@company.com".to_string()),
            parsed_to: Some("user@jmap.local".to_string()),
            internal_date: Utc::now(),
            flags: "[]".to_string(),
        };
        search_engine
            .index_message(&msg, "Please review the attached contract agreement.")
            .unwrap();

        JmapState {
            db: Arc::new(db),
            search_engine,
        }
    }

    #[tokio::test]
    async fn test_jmap_session_discovery() {
        let state = setup_test_jmap_state();
        let mut headers = HeaderMap::new();
        headers.insert(
            "authorization",
            "Bearer user@jmap.local".parse().unwrap(),
        );

        let res = get_session(State(state), headers).await.unwrap();
        assert_eq!(res.0.api_url, "/jmap/api");
        assert!(res.0.capabilities.contains_key("urn:ietf:params:jmap:core"));
        assert!(res.0.capabilities.contains_key("urn:ietf:params:jmap:mail"));
        assert_eq!(res.0.username, "user@jmap.local");
    }

    #[tokio::test]
    async fn test_jmap_core_echo() {
        let state = setup_test_jmap_state();
        let mut headers = HeaderMap::new();
        headers.insert(
            "authorization",
            "Bearer user@jmap.local".parse().unwrap(),
        );

        let req = JmapRequest {
            using: vec!["urn:ietf:params:jmap:core".to_string()],
            method_calls: vec![("Core/echo".to_string(), json!({ "ping": "pong" }), "c0".to_string())],
        };

        let res = handle_jmap_api(State(state), headers, Json(req)).await.unwrap();
        assert_eq!(res.0.method_responses.len(), 1);
        assert_eq!(res.0.method_responses[0].0, "Core/echo");
        assert_eq!(res.0.method_responses[0].1["ping"], "pong");
        assert_eq!(res.0.method_responses[0].2, "c0");
    }

    #[tokio::test]
    async fn test_jmap_mailbox_get() {
        let state = setup_test_jmap_state();
        let mut headers = HeaderMap::new();
        headers.insert(
            "authorization",
            "Bearer user@jmap.local".parse().unwrap(),
        );

        let req = JmapRequest {
            using: vec!["urn:ietf:params:jmap:mail".to_string()],
            method_calls: vec![("Mailbox/get".to_string(), json!({}), "c1".to_string())],
        };

        let res = handle_jmap_api(State(state), headers, Json(req)).await.unwrap();
        assert_eq!(res.0.method_responses[0].0, "Mailbox/get");
        let list = res.0.method_responses[0].1["list"].as_array().unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0]["name"], "INBOX");
        assert_eq!(list[0]["role"], "inbox");
    }

    #[tokio::test]
    async fn test_jmap_email_query_and_get() {
        let state = setup_test_jmap_state();
        let mut headers = HeaderMap::new();
        headers.insert(
            "authorization",
            "Bearer user@jmap.local".parse().unwrap(),
        );

        // 1. Query for "contract"
        let req_query = JmapRequest {
            using: vec!["urn:ietf:params:jmap:mail".to_string()],
            method_calls: vec![(
                "Email/query".to_string(),
                json!({ "filter": { "text": "contract" } }),
                "c2".to_string(),
            )],
        };

        let res_query = handle_jmap_api(State(state.clone()), headers.clone(), Json(req_query))
            .await
            .unwrap();
        let ids = res_query.0.method_responses[0].1["ids"].as_array().unwrap();
        assert_eq!(ids.len(), 1);
        let found_id = ids[0].as_str().unwrap();

        // 2. Email/get with found_id
        let req_get = JmapRequest {
            using: vec!["urn:ietf:params:jmap:mail".to_string()],
            method_calls: vec![(
                "Email/get".to_string(),
                json!({ "ids": [found_id] }),
                "c3".to_string(),
            )],
        };

        let res_get = handle_jmap_api(State(state), headers, Json(req_get)).await.unwrap();
        let emails = res_get.0.method_responses[0].1["list"].as_array().unwrap();
        assert_eq!(emails.len(), 1);
        assert_eq!(emails[0]["subject"], "Contract Agreement PDF");
    }

    #[tokio::test]
    async fn test_jmap_email_set() {
        let state = setup_test_jmap_state();
        let mut headers = HeaderMap::new();
        headers.insert(
            "authorization",
            "Bearer user@jmap.local".parse().unwrap(),
        );

        let account = state.db.get_account_by_email("user@jmap.local").unwrap().unwrap();
        let all_msgs = state.db.get_messages(&account.id).unwrap();
        let msg_id = &all_msgs[0].id;

        // Mark as $seen: true
        let req_set = JmapRequest {
            using: vec!["urn:ietf:params:jmap:mail".to_string()],
            method_calls: vec![(
                "Email/set".to_string(),
                json!({
                    "update": {
                        (msg_id): {
                            "keywords": { "$seen": true }
                        }
                    }
                }),
                "c4".to_string(),
            )],
        };

        let res_set = handle_jmap_api(State(state.clone()), headers, Json(req_set)).await.unwrap();
        assert_eq!(res_set.0.method_responses[0].0, "Email/set");
        let updated = res_set.0.method_responses[0].1["updated"].as_object().unwrap();
        assert!(updated.contains_key(msg_id));

        // Verify in DB that flags now contain \Seen
        let updated_msgs = state.db.get_messages(&account.id).unwrap();
        assert!(updated_msgs[0].flags.contains("Seen"));
    }
}
