//! FastrMail Binary — Entry point that wires together SMTP, HTTP API, and storage.

use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing::info;
use uuid::Uuid;

use fastrmail_smtp::SmtpServer;
use fastrmail_store::Database;

/// Shared application state passed to all HTTP handlers.
struct AppState {
    db: Arc<Database>,
}

// ─── Request / Response Types ────────────────────────────────

#[derive(Debug, Deserialize)]
struct SendEmailRequest {
    from: String,
    to: Vec<String>,
    subject: String,
    html: Option<String>,
    text: Option<String>,
}

#[derive(Debug, Serialize)]
struct SendEmailResponse {
    success: bool,
    message_id: String,
    status: String,
}

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: String,
}

#[derive(Debug, Serialize)]
struct MessageResponse {
    id: String,
    mailbox_id: String,
    uid: i64,
    blob_id: String,
    size_bytes: i64,
    subject: Option<String>,
    from: Option<String>,
    to: Option<String>,
    internal_date: String,
    flags: String,
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: String,
}

// ─── HTTP Handlers ───────────────────────────────────────────

/// GET /api/health — Returns server health status.
async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
    })
}

/// POST /api/v1/email/send — Queue an email for delivery.
async fn send_email(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<SendEmailRequest>,
) -> Result<Json<SendEmailResponse>, (StatusCode, Json<ErrorResponse>)> {
    let blob_id = Uuid::new_v4().to_string();

    // Build a minimal RFC 5322 email
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

    // Save the blob to disk
    let blob_dir = "data/blobs";
    std::fs::create_dir_all(blob_dir).map_err(|e| {
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

    // Get or create default tenant
    let tenant_id = match state.db.get_tenant_by_domain("localhost") {
        Ok(Some(t)) => t.id,
        Ok(None) => state.db.insert_tenant("localhost").map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to create tenant: {e}"),
                }),
            )
        })?,
        Err(e) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Database error: {e}"),
                }),
            ));
        }
    };

    // Queue each recipient
    for recipient in &payload.to {
        state
            .db
            .queue_email(&tenant_id, &blob_id, &payload.from, recipient)
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

/// GET /api/v1/mailbox — Returns all messages (for demo purposes, returns all messages for a default account).
async fn get_mailbox(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<MessageResponse>>, (StatusCode, Json<ErrorResponse>)> {
    // For Phase 1, return all messages for the default account
    let account = match state.db.get_account_by_email("postmaster@localhost") {
        Ok(Some(a)) => a,
        Ok(None) => {
            // No messages yet
            return Ok(Json(Vec::new()));
        }
        Err(e) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Database error: {e}"),
                }),
            ));
        }
    };

    let messages = state.db.get_messages(&account.id).map_err(|e| {
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

/// Build the Axum router with all routes.
fn build_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/api/health", get(health_check))
        .route("/api/v1/email/send", post(send_email))
        .route("/api/v1/mailbox", get(get_mailbox))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
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

    // Spawn SMTP server
    let smtp_db = Arc::clone(&db);
    let smtp_server = SmtpServer::new(smtp_db, "data".to_string());
    tokio::spawn(async move {
        if let Err(e) = smtp_server.start("0.0.0.0:2525").await {
            tracing::error!("SMTP server error: {e}");
        }
    });
    info!("SMTP listening on :2525");

    // Start HTTP API server
    let state = Arc::new(AppState { db });
    let app = build_router(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;
    info!("HTTP API listening on :8080");

    println!("  SMTP listening on :2525");
    println!("  HTTP API listening on :8080");
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
        Arc::new(AppState { db: Arc::new(db) })
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
    }

    #[tokio::test]
    async fn test_get_mailbox_empty() {
        let state = setup_test_state();
        let res = get_mailbox(State(state)).await.unwrap();
        assert!(res.0.is_empty());
    }
}
