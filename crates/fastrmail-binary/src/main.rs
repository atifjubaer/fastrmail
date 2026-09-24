//! FastrMail Binary — Entry point wiring SMTP, IMAP, Outbound delivery, and full Webmail/Admin HTTP APIs.

use std::sync::Arc;

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use mail_parser::MessageParser;
use serde::{Deserialize, Serialize};
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;
use tracing::info;
use uuid::Uuid;

use fastrmail_auth::DkimSigner;
use fastrmail_core::{Account, QueueItem, SystemStats, Tenant};
use fastrmail_imap::ImapServer;
use fastrmail_jmap::{build_jmap_router, JmapState};
use fastrmail_pop3::Pop3Server;
use fastrmail_search::SearchEngine;
use fastrmail_smtp::{OutboundEngine, SmtpServer};
use fastrmail_store::Database;

/// Shared application state passed to all HTTP handlers.
pub struct AppState {
    pub db: Arc<Database>,
    pub data_dir: String,
    pub search_engine: Arc<SearchEngine>,
}

// ─── Request / Response Types ────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct SendEmailRequest {
    pub from: String,
    pub to: Vec<String>,
    pub subject: String,
    pub html: Option<String>,
    pub text: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SendEmailResponse {
    pub success: bool,
    pub message_id: String,
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct MessageResponse {
    pub id: String,
    pub mailbox_id: String,
    pub uid: i64,
    pub blob_id: String,
    pub size_bytes: i64,
    pub subject: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub internal_date: String,
    pub flags: String,
}

#[derive(Debug, Serialize)]
pub struct MessageDetailResponse {
    pub id: String,
    pub mailbox_id: String,
    pub uid: i64,
    pub blob_id: String,
    pub size_bytes: i64,
    pub subject: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub internal_date: String,
    pub flags: Vec<String>,
    pub text_body: Option<String>,
    pub html_body: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct MailboxSummaryResponse {
    pub id: String,
    pub name: String,
    pub uid_validity: i64,
    pub uid_next: i64,
    pub total_messages: i64,
    pub unseen_messages: i64,
}

#[derive(Debug, Deserialize)]
pub struct CreateMailboxRequest {
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateFlagsRequest {
    pub mailbox_id: String,
    pub uid: i64,
    pub flags: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct MessageQuery {
    pub id: Option<String>,
    pub mailbox_id: Option<String>,
    pub uid: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct MailboxFilterQuery {
    pub mailbox_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateTenantRequest {
    pub domain: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateAccountRequest {
    pub tenant_id: String,
    pub username: String,
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct AdminAccountsQuery {
    pub tenant_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct DeleteItemQuery {
    pub id: String,
}

#[derive(Debug, Deserialize)]
pub struct GenerateDkimRequest {
    pub domain: String,
}

#[derive(Debug, Serialize)]
pub struct GenerateDkimResponse {
    pub domain: String,
    pub selector: String,
    pub dns_record: String,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

// ─── Helpers ─────────────────────────────────────────────────

pub fn get_or_create_default_account(db: &Database) -> anyhow::Result<(Tenant, Account)> {
    let tenant = match db.get_tenant_by_domain("localhost")? {
        Some(t) => t,
        None => {
            let _id = db.insert_tenant("localhost")?;
            db.get_tenant_by_domain("localhost")?.unwrap()
        }
    };

    let account = match db.get_account_by_email("postmaster@localhost")? {
        Some(a) => a,
        None => {
            let _id = db.insert_account(
                &tenant.id,
                "postmaster",
                "postmaster@localhost",
                "admin123",
            )?;
            db.get_account_by_email("postmaster@localhost")?.unwrap()
        }
    };

    // Ensure standard mailboxes exist
    let standard_mailboxes = ["INBOX", "Sent", "Drafts", "Trash"];
    for name in &standard_mailboxes {
        if db.get_mailbox_by_name(&account.id, name)?.is_none() {
            db.insert_mailbox(&account.id, name)?;
        }
    }

    Ok((tenant, account))
}

// ─── HTTP Handlers: Public & Webmail ─────────────────────────

/// GET /api/health — Returns server health status.
async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
    })
}

/// POST /api/v1/email/send — Queue an email for outbound delivery.
async fn send_email(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<SendEmailRequest>,
) -> Result<Json<SendEmailResponse>, (StatusCode, Json<ErrorResponse>)> {
    let blob_id = Uuid::new_v4().to_string();

    let body = payload.text.as_deref().unwrap_or("");
    let html = payload.html.as_deref().unwrap_or("");
    let raw_email = format!(
        "From: {}\r\nTo: {}\r\nSubject: {}\r\nContent-Type: text/html; charset=utf-8\r\n\r\n{}{}",
        payload.from,
        payload.to.join(", "),
        payload.subject,
        if html.is_empty() { body } else { html },
        if !body.is_empty() && !html.is_empty() {
            format!("\r\n\r\n{body}")
        } else {
            String::new()
        }
    );

    let blob_dir = format!("{}/blobs", state.data_dir);
    std::fs::create_dir_all(&blob_dir).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to create blob directory: {e}"),
            }),
        )
    })?;

    let blob_path = format!("{blob_dir}/{blob_id}.eml");
    std::fs::write(&blob_path, raw_email.as_bytes()).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to write blob: {e}"),
            }),
        )
    })?;

    let (tenant, account) = get_or_create_default_account(&state.db).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Account error: {e}"),
            }),
        )
    })?;

    // Also store copy into "Sent" mailbox
    if let Ok(Some(sent_mb)) = state.db.get_mailbox_by_name(&account.id, "Sent") {
        let _ = state.db.insert_message(
            &sent_mb.id,
            &account.id,
            &blob_id,
            raw_email.len() as i64,
            Some(&payload.subject),
            Some(&payload.from),
            payload.to.first().map(|s| s.as_str()),
        );
    }

    for recipient in &payload.to {
        state
            .db
            .queue_email(&tenant.id, &blob_id, &payload.from, recipient)
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: format!("Failed to queue email: {e}"),
                    }),
                )
            })?;
    }

    let message_id = format!("msg_{}", &blob_id[..10]);
    info!(
        "Email queued: message_id={message_id} from={} to={:?} subject={}",
        payload.from, payload.to, payload.subject
    );

    Ok(Json(SendEmailResponse {
        success: true,
        message_id,
        status: "queued".to_string(),
    }))
}

/// GET /api/v1/mailboxes — List all mailboxes with message and unseen counts.
async fn list_mailboxes(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<MailboxSummaryResponse>>, (StatusCode, Json<ErrorResponse>)> {
    let (_, account) = get_or_create_default_account(&state.db).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to get default account: {e}"),
            }),
        )
    })?;

    let mailboxes = state.db.get_mailboxes(&account.id).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Database error: {e}"),
            }),
        )
    })?;

    let mut summaries = Vec::new();
    for mb in mailboxes {
        let (total, unseen) = state.db.get_mailbox_counts(&mb.id).unwrap_or((0, 0));
        summaries.push(MailboxSummaryResponse {
            id: mb.id,
            name: mb.name,
            uid_validity: mb.uid_validity,
            uid_next: mb.uid_next,
            total_messages: total,
            unseen_messages: unseen,
        });
    }

    Ok(Json(summaries))
}

/// POST /api/v1/mailboxes — Create a custom mailbox folder.
async fn create_mailbox_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CreateMailboxRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let (_, account) = get_or_create_default_account(&state.db).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Account error: {e}"),
            }),
        )
    })?;

    if state
        .db
        .get_mailbox_by_name(&account.id, &payload.name)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Database error: {e}"),
                }),
            )
        })?
        .is_some()
    {
        return Err((
            StatusCode::CONFLICT,
            Json(ErrorResponse {
                error: "Mailbox already exists".to_string(),
            }),
        ));
    }

    let id = state
        .db
        .insert_mailbox(&account.id, &payload.name)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to create mailbox: {e}"),
                }),
            )
        })?;

    Ok(Json(serde_json::json!({
        "success": true,
        "mailbox_id": id,
        "name": payload.name,
    })))
}

/// GET /api/v1/mailbox — Returns messages (optionally filtered by ?mailbox_id=X).
async fn get_mailbox(
    State(state): State<Arc<AppState>>,
    Query(query): Query<MailboxFilterQuery>,
) -> Result<Json<Vec<MessageResponse>>, (StatusCode, Json<ErrorResponse>)> {
    let (_, account) = get_or_create_default_account(&state.db).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Database error: {e}"),
            }),
        )
    })?;

    let messages = if let Some(mb_id) = query.mailbox_id {
        state.db.get_messages_by_mailbox(&mb_id)
    } else {
        state.db.get_messages(&account.id)
    }
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to get messages: {e}"),
            }),
        )
    })?;

    let response: Vec<MessageResponse> = messages
        .into_iter()
        .map(|m| MessageResponse {
            id: m.id,
            mailbox_id: m.mailbox_id,
            uid: m.uid,
            blob_id: m.blob_id,
            size_bytes: m.size_bytes,
            subject: m.parsed_subject,
            from: m.parsed_from,
            to: m.parsed_to,
            internal_date: m.internal_date.to_rfc3339(),
            flags: m.flags,
        })
        .collect();

    Ok(Json(response))
}

/// GET /api/v1/message — Returns detailed email content including parsed text and html bodies.
async fn get_message_detail(
    State(state): State<Arc<AppState>>,
    Query(query): Query<MessageQuery>,
) -> Result<Json<MessageDetailResponse>, (StatusCode, Json<ErrorResponse>)> {
    let (_, account) = get_or_create_default_account(&state.db).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Database error: {e}"),
            }),
        )
    })?;

    let message = if let (Some(mb_id), Some(uid)) = (&query.mailbox_id, query.uid) {
        state.db.get_message_by_uid(mb_id, uid)
    } else if let Some(id) = &query.id {
        let all = state.db.get_messages(&account.id).unwrap_or_default();
        Ok(all.into_iter().find(|m| m.id == *id))
    } else {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Must supply ?id=X or ?mailbox_id=Y&uid=Z".to_string(),
            }),
        ));
    }
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Database error: {e}"),
            }),
        )
    })?;

    let msg = match message {
        Some(m) => m,
        None => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: "Message not found".to_string(),
                }),
            ));
        }
    };

    // Read and parse the raw .eml blob
    let blob_path = format!("{}/blobs/{}.eml", state.data_dir, msg.blob_id);
    let mut text_body = None;
    let mut html_body = None;

    if let Ok(bytes) = std::fs::read(&blob_path) {
        if let Some(parsed) = MessageParser::default().parse(&bytes) {
            text_body = parsed.body_text(0).map(|s| s.to_string());
            html_body = parsed.body_html(0).map(|s| s.to_string());
        }
    }

    let flags: Vec<String> = serde_json::from_str(&msg.flags).unwrap_or_default();

    Ok(Json(MessageDetailResponse {
        id: msg.id,
        mailbox_id: msg.mailbox_id,
        uid: msg.uid,
        blob_id: msg.blob_id,
        size_bytes: msg.size_bytes,
        subject: msg.parsed_subject,
        from: msg.parsed_from,
        to: msg.parsed_to,
        internal_date: msg.internal_date.to_rfc3339(),
        flags,
        text_body,
        html_body,
    }))
}

/// PATCH /api/v1/message — Update message flags (e.g. mark as \Seen, \Flagged, etc.).
async fn update_message_flags_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<UpdateFlagsRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let formatted_flags: Vec<String> = payload
        .flags
        .iter()
        .map(|f| {
            if f.starts_with('\\') {
                f.clone()
            } else {
                format!("\\{f}")
            }
        })
        .collect();

    let json_flags = serde_json::to_string(&formatted_flags).unwrap_or_else(|_| "[]".to_string());

    state
        .db
        .update_message_flags(&payload.mailbox_id, payload.uid, &json_flags)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to update flags: {e}"),
                }),
            )
        })?;

    Ok(Json(serde_json::json!({
        "success": true,
        "mailbox_id": payload.mailbox_id,
        "uid": payload.uid,
        "flags": formatted_flags,
    })))
}

/// DELETE /api/v1/message — Expunge or delete message by ID.
async fn delete_message_handler(
    State(state): State<Arc<AppState>>,
    Query(query): Query<MessageQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let (_, account) = get_or_create_default_account(&state.db).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Database error: {e}"),
            }),
        )
    })?;

    let (mailbox_id, uid) = if let (Some(mb_id), Some(uid)) = (&query.mailbox_id, query.uid) {
        (mb_id.clone(), uid)
    } else if let Some(id) = &query.id {
        let all = state.db.get_messages(&account.id).unwrap_or_default();
        if let Some(msg) = all.into_iter().find(|m| m.id == *id) {
            (msg.mailbox_id, msg.uid)
        } else {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: "Message not found".to_string(),
                }),
            ));
        }
    } else {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Missing message identifier".to_string(),
            }),
        ));
    };

    // Mark as Deleted and expunge
    state
        .db
        .update_message_flags(&mailbox_id, uid, r#"["\\Deleted"]"#)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to mark deleted: {e}"),
                }),
            )
        })?;

    let expunged = state
        .db
        .expunge_deleted_messages(&mailbox_id)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to expunge message: {e}"),
                }),
            )
        })?;

    for (_, blob_id) in expunged {
        let blob_path = format!("{}/blobs/{blob_id}.eml", state.data_dir);
        let _ = std::fs::remove_file(blob_path);
    }

    Ok(Json(serde_json::json!({
        "success": true,
        "deleted": true
    })))
}

// ─── HTTP Handlers: Admin API ────────────────────────────────

/// GET /api/v1/admin/stats — Retrieve overview system metrics.
async fn get_admin_stats(
    State(state): State<Arc<AppState>>,
) -> Result<Json<SystemStats>, (StatusCode, Json<ErrorResponse>)> {
    let stats = state.db.get_system_stats().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to get stats: {e}"),
            }),
        )
    })?;
    Ok(Json(stats))
}

/// GET /api/v1/admin/tenants — List all domains/tenants.
async fn list_admin_tenants(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<Tenant>>, (StatusCode, Json<ErrorResponse>)> {
    let tenants = state.db.list_all_tenants().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to list tenants: {e}"),
            }),
        )
    })?;
    Ok(Json(tenants))
}

/// POST /api/v1/admin/tenants — Create a new tenant domain.
async fn create_admin_tenant(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CreateTenantRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    if state
        .db
        .get_tenant_by_domain(&payload.domain)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Database error: {e}"),
                }),
            )
        })?
        .is_some()
    {
        return Err((
            StatusCode::CONFLICT,
            Json(ErrorResponse {
                error: "Domain already registered".to_string(),
            }),
        ));
    }

    let id = state.db.insert_tenant(&payload.domain).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to insert tenant: {e}"),
            }),
        )
    })?;

    Ok(Json(serde_json::json!({
        "success": true,
        "tenant_id": id,
        "domain": payload.domain,
    })))
}

/// GET /api/v1/admin/accounts — List accounts belonging to a tenant.
async fn list_admin_accounts(
    State(state): State<Arc<AppState>>,
    Query(query): Query<AdminAccountsQuery>,
) -> Result<Json<Vec<Account>>, (StatusCode, Json<ErrorResponse>)> {
    let tenant_id = if let Some(tid) = query.tenant_id {
        tid
    } else {
        let (tenant, _) = get_or_create_default_account(&state.db).map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Database error: {e}"),
                }),
            )
        })?;
        tenant.id
    };

    let accounts = state.db.get_accounts_by_tenant(&tenant_id).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to get accounts: {e}"),
            }),
        )
    })?;

    Ok(Json(accounts))
}

/// POST /api/v1/admin/accounts — Create a user account under a tenant.
async fn create_admin_account(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CreateAccountRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let id = state
        .db
        .insert_account(
            &payload.tenant_id,
            &payload.username,
            &payload.email,
            &payload.password,
        )
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to create account: {e}"),
                }),
            )
        })?;

    // Create default INBOX
    let _ = state.db.insert_mailbox(&id, "INBOX");

    Ok(Json(serde_json::json!({
        "success": true,
        "account_id": id,
        "email": payload.email,
    })))
}

/// DELETE /api/v1/admin/accounts — Delete an account by ID.
async fn delete_admin_account(
    State(state): State<Arc<AppState>>,
    Query(query): Query<DeleteItemQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    state.db.delete_account(&query.id).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to delete account: {e}"),
            }),
        )
    })?;

    Ok(Json(serde_json::json!({
        "success": true,
        "deleted_account_id": query.id
    })))
}

/// POST /api/v1/admin/dkim/generate — Generate DKIM keys for a domain and return formatted DNS record.
async fn generate_dkim_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<GenerateDkimRequest>,
) -> Result<Json<GenerateDkimResponse>, (StatusCode, Json<ErrorResponse>)> {
    let dns_record = handle_generate_dkim(&state.db, &payload.domain).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to generate DKIM: {e}"),
            }),
        )
    })?;

    Ok(Json(GenerateDkimResponse {
        domain: payload.domain,
        selector: "default".to_string(),
        dns_record,
    }))
}

/// GET /api/v1/admin/queue — View pending outbound SMTP queue items.
async fn list_admin_queue(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<QueueItem>>, (StatusCode, Json<ErrorResponse>)> {
    let items = state.db.get_queue_pending().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to get queue: {e}"),
            }),
        )
    })?;
    Ok(Json(items))
}

// ─── Router Builder ──────────────────────────────────────────

/// Build the Axum router with all public, webmail, admin, and JMAP RFC 8620/8621 routes.
pub fn build_router(state: Arc<AppState>) -> Router {
    let jmap_state = JmapState {
        db: Arc::clone(&state.db),
        search_engine: Arc::clone(&state.search_engine),
    };

    let mut router = Router::new()
        // Health
        .route("/api/health", get(health_check))
        // Transactional Email API
        .route("/api/v1/email/send", post(send_email))
        // Webmail Endpoints
        .route("/api/v1/mailboxes", get(list_mailboxes).post(create_mailbox_handler))
        .route("/api/v1/mailbox", get(get_mailbox))
        .route(
            "/api/v1/message",
            get(get_message_detail)
                .patch(update_message_flags_handler)
                .delete(delete_message_handler),
        )
        // Admin Endpoints
        .route("/api/v1/admin/stats", get(get_admin_stats))
        .route(
            "/api/v1/admin/tenants",
            get(list_admin_tenants).post(create_admin_tenant),
        )
        .route(
            "/api/v1/admin/accounts",
            get(list_admin_accounts)
                .post(create_admin_account)
                .delete(delete_admin_account),
        )
        .route("/api/v1/admin/dkim/generate", post(generate_dkim_handler))
        .route("/api/v1/admin/queue", get(list_admin_queue))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
        .merge(build_jmap_router(jmap_state));

    if std::path::Path::new("web/admin/dist").exists() {
        router = router.nest_service("/admin", ServeDir::new("web/admin/dist"));
    }
    if std::path::Path::new("web/webmail/dist").exists() {
        router = router.fallback_service(ServeDir::new("web/webmail/dist"));
    }

    router
}

/// Generate a DKIM key pair, record it in the database, and return the formatted DNS TXT record.
pub fn handle_generate_dkim(db: &Database, domain: &str) -> anyhow::Result<String> {
    let tenant_id = match db.get_tenant_by_domain(domain)? {
        Some(t) => t.id,
        None => db.insert_tenant(domain)?,
    };

    let key_pair = DkimSigner::generate_key_pair()?;
    let selector = "default";
    db.insert_dkim_key(&tenant_id, selector, &key_pair.private_key_pem)?;

    let record = format!(
        "{}._domainkey.{} TXT v=DKIM1; k=rsa; p={}",
        selector, domain, key_pair.public_key_base64
    );
    Ok(record)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();

    // Check for CLI argument: fastrmail --generate-dkim <domain>
    if let Some(pos) = args.iter().position(|arg| arg == "--generate-dkim") {
        let domain = args
            .get(pos + 1)
            .map(|s| s.as_str())
            .unwrap_or("example.com");

        let db = Database::new("data/fastrmail.db")?;
        db.init_schema()?;

        let record = handle_generate_dkim(&db, domain)?;

        println!("Add this TXT record to your DNS:");
        println!("{record}");
        return Ok(());
    }

    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    // Print startup banner
    println!();
    println!("  ╔═══════════════════════════════════════╗");
    println!("  ║         FastrMail v0.1.0              ║");
    println!("  ║  Open-Source Single-Binary Mail Server ║");
    println!("  ╚═══════════════════════════════════════╝");
    println!();

    // Initialize database
    let db = Database::new("data/fastrmail.db")?;
    db.init_schema()?;
    info!("Database initialized at data/fastrmail.db");

    let db = Arc::new(db);

    // Ensure default account exists
    let _ = get_or_create_default_account(&db);

    // Initialize full-text search engine
    let search_engine = Arc::new(SearchEngine::new("data/index")?);
    info!("Tantivy full-text search engine initialized at data/index");

    // 1. Spawn Inbound SMTP Server on :2525
    let smtp_db = Arc::clone(&db);
    let smtp_server = SmtpServer::new(smtp_db, "data".to_string())
        .with_search_engine(Arc::clone(&search_engine));
    tokio::spawn(async move {
        if let Err(e) = smtp_server.start("0.0.0.0:2525").await {
            tracing::error!("SMTP server error: {e}");
        }
    });
    info!("SMTP listening on :2525");

    // 2. Spawn SMTP Submission Server on :2526 (Port 587)
    let sub_db = Arc::clone(&db);
    let sub_server = SmtpServer::new_submission(sub_db, "data".to_string())
        .with_search_engine(Arc::clone(&search_engine));
    let sub_bind = std::env::var("FASTRMAIL_BIND_SUBMISSION").unwrap_or_else(|_| "0.0.0.0:2526".to_string());
    tokio::spawn(async move {
        if let Err(e) = sub_server.start(&sub_bind).await {
            tracing::error!("SMTP submission error on {sub_bind}: {e}");
        }
    });
    info!("SMTP Submission listening on :2526 (Port 587)");

    // 3. Spawn Inbound IMAP4rev2 Server on :1143
    let imap_db = Arc::clone(&db);
    let imap_server = ImapServer::new(imap_db, "data".to_string());
    tokio::spawn(async move {
        if let Err(e) = imap_server.start("0.0.0.0:1143").await {
            tracing::error!("IMAP server error: {e}");
        }
    });
    info!("IMAP listening on :1143");

    // 4. Spawn POP3 Server on :1110 (Port 110)
    let pop3_db = Arc::clone(&db);
    let pop3_server = Pop3Server::new(pop3_db, "data".to_string());
    let pop3_bind = std::env::var("FASTRMAIL_BIND_POP3").unwrap_or_else(|_| "0.0.0.0:1110".to_string());
    tokio::spawn(async move {
        if let Err(e) = pop3_server.start(&pop3_bind).await {
            tracing::error!("POP3 server error on {pop3_bind}: {e}");
        }
    });
    info!("POP3 listening on :1110 (Port 110)");

    // 5. Spawn Outbound SMTP Delivery Worker
    let outbound_db = Arc::clone(&db);
    let outbound_engine = Arc::new(OutboundEngine::new(outbound_db, "data".to_string()));
    let (_shutdown_tx, shutdown_rx) = tokio::sync::broadcast::channel(1);
    outbound_engine.start(shutdown_rx);
    info!("Outbound SMTP delivery worker running");

    // 6. Start Axum HTTP API Server on :8080 (including JMAP RFC 8620/8621)
    let state = Arc::new(AppState {
        db,
        data_dir: "data".to_string(),
        search_engine,
    });
    let app = build_router(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;
    info!("HTTP API listening on :8080");

    println!("  SMTP listening on :2525 (Inbound)");
    println!("  SMTP Submission on :2526 (Port 587 Auth)");
    println!("  IMAP listening on :1143 (RFC 9051)");
    println!("  POP3 listening on :1110 (RFC 1939)");
    println!("  HTTP API listening on :8080");
    println!("  JMAP API listening on :8080/jmap");
    println!();

    axum::serve(listener, app).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_test_state() -> Arc<AppState> {
        let db = Database::new_memory().expect("Failed to create in-memory DB");
        db.init_schema().expect("Failed to init schema");
        let temp_dir = std::env::temp_dir().join(Uuid::new_v4().to_string());
        std::fs::create_dir_all(&temp_dir).unwrap();
        let search_engine = Arc::new(SearchEngine::new_in_ram().expect("Failed to create in-ram search engine"));
        Arc::new(AppState {
            db: Arc::new(db),
            data_dir: temp_dir.to_str().unwrap().to_string(),
            search_engine,
        })
    }

    #[tokio::test]
    async fn test_health_check() {
        let res = health_check().await;
        assert_eq!(res.status, "ok");
    }

    #[tokio::test]
    async fn test_send_email() {
        let state = setup_test_state();

        let req = SendEmailRequest {
            from: "Marketing <marketing@fastrmail.example.com>".to_string(),
            to: vec!["user@example.com".to_string()],
            subject: "Welcome to FastrMail".to_string(),
            html: Some("<strong>Lightning fast email.</strong>".to_string()),
            text: Some("Lightning fast email.".to_string()),
        };

        let res = send_email(State(state.clone()), Json(req)).await.unwrap();
        assert!(res.success);
        assert_eq!(res.status, "queued");
        assert!(res.message_id.starts_with("msg_"));

        // Verify queued in database
        let pending = state.db.get_queue_pending().unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].sender, "Marketing <marketing@fastrmail.example.com>");
        assert_eq!(pending[0].recipient, "user@example.com");

        let _ = std::fs::remove_dir_all(&state.data_dir);
    }

    #[tokio::test]
    async fn test_get_mailbox_empty() {
        let state = setup_test_state();
        let res = get_mailbox(State(state.clone()), Query(MailboxFilterQuery { mailbox_id: None })).await.unwrap();
        assert!(res.0.is_empty());
        let _ = std::fs::remove_dir_all(&state.data_dir);
    }

    #[test]
    fn test_cli_generate_dkim() {
        let db = Database::new_memory().unwrap();
        db.init_schema().unwrap();

        let record = handle_generate_dkim(&db, "test.com").unwrap();
        assert!(record.starts_with("default._domainkey.test.com TXT v=DKIM1; k=rsa; p="));

        // Verify saved to database
        let tenant = db.get_tenant_by_domain("test.com").unwrap().unwrap();
        let active_key = db.get_active_dkim_key(&tenant.id).unwrap().unwrap();
        assert_eq!(active_key.selector, "default");
        assert!(active_key.is_active);
    }

    #[tokio::test]
    async fn test_webmail_and_admin_api() {
        let state = setup_test_state();

        // 1. Check stats
        let stats = get_admin_stats(State(state.clone())).await.unwrap();
        assert_eq!(stats.tenants_count, 0);

        // 2. List mailboxes
        let mailboxes = list_mailboxes(State(state.clone())).await.unwrap();
        assert!(mailboxes.0.len() >= 4); // INBOX, Sent, Drafts, Trash

        // 3. Create custom mailbox
        let create_mb_res = create_mailbox_handler(
            State(state.clone()),
            Json(CreateMailboxRequest {
                name: "Archive".to_string(),
            }),
        )
        .await
        .unwrap();
        assert_eq!(create_mb_res.0["name"], "Archive");

        // 4. Create tenant
        let create_tenant_res = create_admin_tenant(
            State(state.clone()),
            Json(CreateTenantRequest {
                domain: "customdomain.com".to_string(),
            }),
        )
        .await
        .unwrap();
        let tenant_id = create_tenant_res.0["tenant_id"].as_str().unwrap();

        // 5. Create account under tenant
        let create_acc_res = create_admin_account(
            State(state.clone()),
            Json(CreateAccountRequest {
                tenant_id: tenant_id.to_string(),
                username: "bob".to_string(),
                email: "bob@customdomain.com".to_string(),
                password: "SecurePassword123!".to_string(),
            }),
        )
        .await
        .unwrap();
        assert_eq!(create_acc_res.0["email"], "bob@customdomain.com");

        // 6. Generate DKIM
        let dkim_res = generate_dkim_handler(
            State(state.clone()),
            Json(GenerateDkimRequest {
                domain: "customdomain.com".to_string(),
            }),
        )
        .await
        .unwrap();
        assert!(dkim_res.0.dns_record.contains("v=DKIM1; k=rsa;"));

        let _ = std::fs::remove_dir_all(&state.data_dir);
    }

    #[tokio::test]
    async fn test_jmap_router_session_integration() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let state = setup_test_state();
        let app = build_router(state.clone());

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });

        let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();
        let request = format!(
            "GET /jmap/session HTTP/1.1\r\nHost: {}\r\nAuthorization: Bearer user@fastrmail.local\r\nConnection: close\r\n\r\n",
            addr
        );
        stream.write_all(request.as_bytes()).await.unwrap();

        let mut response = String::new();
        stream.read_to_string(&mut response).await.unwrap();

        assert!(response.starts_with("HTTP/1.1 200 OK"));
        assert!(response.contains("urn:ietf:params:jmap:core"));
        assert!(response.contains("urn:ietf:params:jmap:mail"));

        let _ = std::fs::remove_dir_all(&state.data_dir);
    }
}
