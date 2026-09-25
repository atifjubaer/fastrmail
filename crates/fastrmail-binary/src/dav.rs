//! FastrMail CalDAV & CardDAV — RFC 4791 & RFC 6352 WebDAV server for Calendar and Contacts synchronization.
//!
//! Provides discovery (.well-known), multi-status XML PROPFIND/REPORT responses,
//! and full CRUD synchronization for Apple Calendar/Contacts, Thunderbird, and mobile clients.

use std::sync::Arc;

use axum::body::Bytes;
use axum::http::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE, LOCATION};
use axum::http::{Method, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::any;
use axum::Router;
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;

use fastrmail_auth::LdapAuthGateway;
use fastrmail_core::Account;
use fastrmail_store::Database;

use crate::AppState;

const XML_CONTENT_TYPE: &str = "application/xml; charset=utf-8";

// ─── Router Builder ──────────────────────────────────────────

/// Construct the CalDAV and CardDAV router with discovery redirects and WebDAV methods.
pub fn build_dav_router(state: Arc<AppState>) -> Router {
    let state_cal = Arc::clone(&state);
    let cal_service = tower::service_fn(move |req: axum::extract::Request| {
        let state = Arc::clone(&state_cal);
        async move {
            let method = req.method().clone();
            let path = req.uri().path().to_string();
            let headers = req.headers().clone();
            let body = match axum::body::to_bytes(req.into_body(), 10 * 1024 * 1024).await {
                Ok(b) => b,
                Err(_) => {
                    return Ok::<_, std::convert::Infallible>(
                        StatusCode::BAD_REQUEST.into_response(),
                    )
                }
            };
            let resp = handle_caldav(state, method, headers, &path, body).await;
            Ok::<_, std::convert::Infallible>(resp)
        }
    });

    let state_card = Arc::clone(&state);
    let card_service = tower::service_fn(move |req: axum::extract::Request| {
        let state = Arc::clone(&state_card);
        async move {
            let method = req.method().clone();
            let path = req.uri().path().to_string();
            let headers = req.headers().clone();
            let body = match axum::body::to_bytes(req.into_body(), 10 * 1024 * 1024).await {
                Ok(b) => b,
                Err(_) => {
                    return Ok::<_, std::convert::Infallible>(
                        StatusCode::BAD_REQUEST.into_response(),
                    )
                }
            };
            let resp = handle_carddav(state, method, headers, &path, body).await;
            Ok::<_, std::convert::Infallible>(resp)
        }
    });

    Router::new()
        // RFC 6764 Well-Known Discovery Redirects
        .route("/.well-known/caldav", any(well_known_caldav))
        .route("/.well-known/carddav", any(well_known_carddav))
        .nest_service("/caldav", cal_service)
        .nest_service("/carddav", card_service)
}

// ─── Well-Known Discovery ────────────────────────────────────

async fn well_known_caldav() -> Response {
    Response::builder()
        .status(StatusCode::MOVED_PERMANENTLY)
        .header(LOCATION, "/caldav/")
        .body(Bytes::new().into_response().into_body())
        .unwrap()
}

async fn well_known_carddav() -> Response {
    Response::builder()
        .status(StatusCode::MOVED_PERMANENTLY)
        .header(LOCATION, "/carddav/")
        .body(Bytes::new().into_response().into_body())
        .unwrap()
}

// ─── Authentication ──────────────────────────────────────────

fn unauthorized_response() -> Response {
    Response::builder()
        .status(StatusCode::UNAUTHORIZED)
        .header(
            "WWW-Authenticate",
            HeaderValue::from_static("Basic realm=\"FastrMail CalDAV/CardDAV\""),
        )
        .body(Bytes::from("Unauthorized").into_response().into_body())
        .unwrap()
}

async fn authenticate(headers: &HeaderMap, db: &Database) -> Result<Account, Response> {
    let auth_header = match headers.get(AUTHORIZATION) {
        Some(h) => match h.to_str() {
            Ok(s) => s,
            Err(_) => return Err(unauthorized_response()),
        },
        None => return Err(unauthorized_response()),
    };

    if !auth_header.starts_with("Basic ") {
        return Err(unauthorized_response());
    }

    let b64_token = auth_header.trim_start_matches("Basic ");
    let decoded = match BASE64.decode(b64_token) {
        Ok(bytes) => String::from_utf8_lossy(&bytes).to_string(),
        Err(_) => return Err(unauthorized_response()),
    };

    let mut parts = decoded.splitn(2, ':');
    let username = parts.next().unwrap_or("").trim();
    let password = parts.next().unwrap_or("").trim();

    if username.is_empty() || password.is_empty() {
        return Err(unauthorized_response());
    }

    // 1. Direct login verification
    if let Ok(Some(account)) = db.verify_login(username, password) {
        return Ok(account);
    }

    // 2. Try with default tenant domain if username doesn't contain '@'
    if !username.contains('@') {
        if let Ok(tenants) = db.list_all_tenants() {
            if let Some(t) = tenants.first() {
                let full_email = format!("{}@{}", username, t.domain);
                if let Ok(Some(account)) = db.verify_login(&full_email, password) {
                    return Ok(account);
                }
            }
        }
    }

    // 3. LDAP Gateway fallback if enabled
    let ldap = LdapAuthGateway::default();
    if ldap.is_enabled() {
        if let Ok(true) = ldap.authenticate(username, password).await {
            // Find or auto-provision account
            let email = if username.contains('@') {
                username.to_string()
            } else {
                let domain = db
                    .list_all_tenants()
                    .ok()
                    .and_then(|t| t.into_iter().next())
                    .map(|t| t.domain)
                    .unwrap_or_else(|| "fastrmail.local".to_string());
                format!("{username}@{domain}")
            };

            if let Ok(Some(acc)) = db.get_account_by_email(&email) {
                return Ok(acc);
            } else {
                // Auto-provision
                let tenant_id = match db.list_all_tenants() {
                    Ok(t) if !t.is_empty() => t[0].id.clone(),
                    _ => db.insert_tenant("fastrmail.local").unwrap_or_default(),
                };
                if let Ok(_new_id) = db.insert_account(&tenant_id, username, &email, password) {
                    if let Ok(Some(acc)) = db.get_account_by_email(&email) {
                        return Ok(acc);
                    }
                }
            }
        }
    }

    Err(unauthorized_response())
}

// ─── CalDAV Handlers ─────────────────────────────────────────

async fn handle_caldav(
    state: Arc<AppState>,
    method: Method,
    headers: HeaderMap,
    path: &str,
    body: Bytes,
) -> Response {
    let method_str = method.as_str();

    // Handle OPTIONS without requiring auth
    if method_str == "OPTIONS" {
        return dav_options_response();
    }

    let account = match authenticate(&headers, &state.db).await {
        Ok(a) => a,
        Err(resp) => return resp,
    };

    let cal = match state.db.get_or_create_default_calendar(&account.id) {
        Ok(c) => c,
        Err(e) => {
            return Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Bytes::from(format!("Calendar error: {e}")).into_response().into_body())
                .unwrap();
        }
    };

    let clean_path = path.trim_matches('/');
    let path_segments: Vec<&str> = if clean_path.is_empty() {
        Vec::new()
    } else {
        clean_path.split('/').collect()
    };

    match method_str {
        "PROPFIND" => handle_caldav_propfind(&account, &cal, &path_segments, &state.db).await,
        "REPORT" => handle_caldav_report(&account, &cal, &path_segments, &state.db, body).await,
        "GET" => handle_caldav_get(&cal, &path_segments, &state.db).await,
        "PUT" => handle_caldav_put(&cal, &path_segments, &state.db, body).await,
        "DELETE" => handle_caldav_delete(&cal, &path_segments, &state.db).await,
        "MKCOL" | "MKCALENDAR" => Response::builder()
            .status(StatusCode::CREATED)
            .body(Bytes::new().into_response().into_body())
            .unwrap(),
        _ => Response::builder()
            .status(StatusCode::METHOD_NOT_ALLOWED)
            .body(Bytes::from("Method Not Allowed").into_response().into_body())
            .unwrap(),
    }
}

// ─── CardDAV Handlers ────────────────────────────────────────

async fn handle_carddav(
    state: Arc<AppState>,
    method: Method,
    headers: HeaderMap,
    path: &str,
    body: Bytes,
) -> Response {
    let method_str = method.as_str();

    if method_str == "OPTIONS" {
        return dav_options_response();
    }

    let account = match authenticate(&headers, &state.db).await {
        Ok(a) => a,
        Err(resp) => return resp,
    };

    let book = match state.db.get_or_create_default_address_book(&account.id) {
        Ok(b) => b,
        Err(e) => {
            return Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Bytes::from(format!("AddressBook error: {e}")).into_response().into_body())
                .unwrap();
        }
    };

    let clean_path = path.trim_matches('/');
    let path_segments: Vec<&str> = if clean_path.is_empty() {
        Vec::new()
    } else {
        clean_path.split('/').collect()
    };

    match method_str {
        "PROPFIND" => handle_carddav_propfind(&account, &book, &path_segments, &state.db).await,
        "REPORT" => handle_carddav_report(&account, &book, &path_segments, &state.db, body).await,
        "GET" => handle_carddav_get(&book, &path_segments, &state.db).await,
        "PUT" => handle_carddav_put(&book, &path_segments, &state.db, body).await,
        "DELETE" => handle_carddav_delete(&book, &path_segments, &state.db).await,
        "MKCOL" => Response::builder()
            .status(StatusCode::CREATED)
            .body(Bytes::new().into_response().into_body())
            .unwrap(),
        _ => Response::builder()
            .status(StatusCode::METHOD_NOT_ALLOWED)
            .body(Bytes::from("Method Not Allowed").into_response().into_body())
            .unwrap(),
    }
}

// ─── WebDAV Options Response ─────────────────────────────────

fn dav_options_response() -> Response {
    Response::builder()
        .status(StatusCode::OK)
        .header(
            "DAV",
            "1, 2, 3, calendar-access, addressbook, extended-mkcol",
        )
        .header(
            "Allow",
            "OPTIONS, GET, HEAD, POST, PUT, DELETE, TRACE, PROPFIND, PROPPATCH, MKCOL, MKCALENDAR, REPORT",
        )
        .body(Bytes::new().into_response().into_body())
        .unwrap()
}

// ─── CalDAV Sub-Handlers ─────────────────────────────────────

async fn handle_caldav_propfind(
    account: &Account,
    cal: &fastrmail_core::Calendar,
    segments: &[&str],
    db: &Database,
) -> Response {
    let username = &account.username;

    // Principal discovery or root PROPFIND
    if segments.is_empty() || segments.first() == Some(&"principals") {
        let xml = format!(
            r#"<?xml version="1.0" encoding="utf-8" ?>
<D:multistatus xmlns:D="DAV:" xmlns:C="urn:ietf:params:xml:ns:caldav" xmlns:CS="http://calendarserver.org/ns/">
  <D:response>
    <D:href>/caldav/principals/{username}/</D:href>
    <D:propstat>
      <D:prop>
        <D:current-user-principal>
          <D:href>/caldav/principals/{username}/</D:href>
        </D:current-user-principal>
        <C:calendar-home-set>
          <D:href>/caldav/{username}/</D:href>
        </C:calendar-home-set>
        <D:displayname>{username}</D:displayname>
        <D:resourcetype>
          <D:principal/>
        </D:resourcetype>
      </D:prop>
      <D:status>HTTP/1.1 200 OK</D:status>
    </D:propstat>
  </D:response>
</D:multistatus>"#
        );
        return multistatus_response(xml);
    }

    // Home set: /caldav/{username}/ or collection: /caldav/{username}/personal/
    let is_collection = segments.len() >= 2;
    let mut xml = format!(
        r#"<?xml version="1.0" encoding="utf-8" ?>
<D:multistatus xmlns:D="DAV:" xmlns:C="urn:ietf:params:xml:ns:caldav" xmlns:CS="http://calendarserver.org/ns/">
  <D:response>
    <D:href>/caldav/{username}/personal/</D:href>
    <D:propstat>
      <D:prop>
        <D:resourcetype>
          <D:collection/>
          <C:calendar/>
        </D:resourcetype>
        <D:displayname>{}</D:displayname>
        <CS:getctag>{}</CS:getctag>
        <C:supported-calendar-component-set>
          <C:comp name="VEVENT"/>
        </C:supported-calendar-component-set>
      </D:prop>
      <D:status>HTTP/1.1 200 OK</D:status>
    </D:propstat>
  </D:response>"#,
        cal.name, cal.ctag
    );

    // If query is for collection, also list events
    if is_collection {
        if let Ok(events) = db.get_calendar_events(&cal.id) {
            for ev in events {
                xml.push_str(&format!(
                    r#"
  <D:response>
    <D:href>/caldav/{username}/personal/{}.ics</D:href>
    <D:propstat>
      <D:prop>
        <D:getetag>{}</D:getetag>
        <D:getcontenttype>text/calendar; component=VEVENT</D:getcontenttype>
      </D:prop>
      <D:status>HTTP/1.1 200 OK</D:status>
    </D:propstat>
  </D:response>"#,
                    ev.uid, ev.etag
                ));
            }
        }
    }

    xml.push_str("\n</D:multistatus>");
    multistatus_response(xml)
}

async fn handle_caldav_report(
    account: &Account,
    cal: &fastrmail_core::Calendar,
    _segments: &[&str],
    db: &Database,
    _body: Bytes,
) -> Response {
    let username = &account.username;
    let events = db.get_calendar_events(&cal.id).unwrap_or_default();

    let mut xml = String::from(
        r#"<?xml version="1.0" encoding="utf-8" ?>
<D:multistatus xmlns:D="DAV:" xmlns:C="urn:ietf:params:xml:ns:caldav">"#,
    );

    for ev in events {
        xml.push_str(&format!(
            r#"
  <D:response>
    <D:href>/caldav/{username}/personal/{}.ics</D:href>
    <D:propstat>
      <D:prop>
        <D:getetag>{}</D:getetag>
        <C:calendar-data><![CDATA[{}]]></C:calendar-data>
      </D:prop>
      <D:status>HTTP/1.1 200 OK</D:status>
    </D:propstat>
  </D:response>"#,
            ev.uid, ev.etag, ev.ical_data
        ));
    }

    xml.push_str("\n</D:multistatus>");
    multistatus_response(xml)
}

async fn handle_caldav_get(
    cal: &fastrmail_core::Calendar,
    segments: &[&str],
    db: &Database,
) -> Response {
    let filename = match segments.last() {
        Some(f) => *f,
        None => return not_found(),
    };
    let uid = filename.trim_end_matches(".ics");

    match db.get_calendar_event_by_uid(&cal.id, uid) {
        Ok(Some(ev)) => Response::builder()
            .status(StatusCode::OK)
            .header(CONTENT_TYPE, "text/calendar; charset=utf-8")
            .header("ETag", ev.etag)
            .body(Bytes::from(ev.ical_data).into_response().into_body())
            .unwrap(),
        _ => not_found(),
    }
}

async fn handle_caldav_put(
    cal: &fastrmail_core::Calendar,
    segments: &[&str],
    db: &Database,
    body: Bytes,
) -> Response {
    let filename = match segments.last() {
        Some(f) => *f,
        None => return not_found(),
    };
    let uid = filename.trim_end_matches(".ics");
    let ical_data = String::from_utf8_lossy(&body).to_string();

    match db.put_calendar_event(&cal.id, uid, &ical_data) {
        Ok(ev) => Response::builder()
            .status(StatusCode::CREATED)
            .header("ETag", ev.etag)
            .body(Bytes::new().into_response().into_body())
            .unwrap(),
        Err(e) => Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(Bytes::from(format!("Failed to save event: {e}")).into_response().into_body())
            .unwrap(),
    }
}

async fn handle_caldav_delete(
    cal: &fastrmail_core::Calendar,
    segments: &[&str],
    db: &Database,
) -> Response {
    let filename = match segments.last() {
        Some(f) => *f,
        None => return not_found(),
    };
    let uid = filename.trim_end_matches(".ics");

    match db.delete_calendar_event(&cal.id, uid) {
        Ok(true) => Response::builder()
            .status(StatusCode::NO_CONTENT)
            .body(Bytes::new().into_response().into_body())
            .unwrap(),
        _ => not_found(),
    }
}

// ─── CardDAV Sub-Handlers ────────────────────────────────────

async fn handle_carddav_propfind(
    account: &Account,
    book: &fastrmail_core::AddressBook,
    segments: &[&str],
    db: &Database,
) -> Response {
    let username = &account.username;

    // Principal discovery or root PROPFIND
    if segments.is_empty() || segments.first() == Some(&"principals") {
        let xml = format!(
            r#"<?xml version="1.0" encoding="utf-8" ?>
<D:multistatus xmlns:D="DAV:" xmlns:CARD="urn:ietf:params:xml:ns:carddav" xmlns:CS="http://calendarserver.org/ns/">
  <D:response>
    <D:href>/carddav/principals/{username}/</D:href>
    <D:propstat>
      <D:prop>
        <D:current-user-principal>
          <D:href>/carddav/principals/{username}/</D:href>
        </D:current-user-principal>
        <CARD:addressbook-home-set>
          <D:href>/carddav/{username}/</D:href>
        </CARD:addressbook-home-set>
        <D:displayname>{username}</D:displayname>
        <D:resourcetype>
          <D:principal/>
        </D:resourcetype>
      </D:prop>
      <D:status>HTTP/1.1 200 OK</D:status>
    </D:propstat>
  </D:response>
</D:multistatus>"#
        );
        return multistatus_response(xml);
    }

    let is_collection = segments.len() >= 2;
    let mut xml = format!(
        r#"<?xml version="1.0" encoding="utf-8" ?>
<D:multistatus xmlns:D="DAV:" xmlns:CARD="urn:ietf:params:xml:ns:carddav" xmlns:CS="http://calendarserver.org/ns/">
  <D:response>
    <D:href>/carddav/{username}/contacts/</D:href>
    <D:propstat>
      <D:prop>
        <D:resourcetype>
          <D:collection/>
          <CARD:addressbook/>
        </D:resourcetype>
        <D:displayname>{}</D:displayname>
        <CS:getctag>{}</CS:getctag>
      </D:prop>
      <D:status>HTTP/1.1 200 OK</D:status>
    </D:propstat>
  </D:response>"#,
        book.name, book.ctag
    );

    if is_collection {
        if let Ok(contacts) = db.get_contacts(&book.id) {
            for c in contacts {
                xml.push_str(&format!(
                    r#"
  <D:response>
    <D:href>/carddav/{username}/contacts/{}.vcf</D:href>
    <D:propstat>
      <D:prop>
        <D:getetag>{}</D:getetag>
        <D:getcontenttype>text/vcard</D:getcontenttype>
      </D:prop>
      <D:status>HTTP/1.1 200 OK</D:status>
    </D:propstat>
  </D:response>"#,
                    c.uid, c.etag
                ));
            }
        }
    }

    xml.push_str("\n</D:multistatus>");
    multistatus_response(xml)
}

async fn handle_carddav_report(
    account: &Account,
    book: &fastrmail_core::AddressBook,
    _segments: &[&str],
    db: &Database,
    _body: Bytes,
) -> Response {
    let username = &account.username;
    let contacts = db.get_contacts(&book.id).unwrap_or_default();

    let mut xml = String::from(
        r#"<?xml version="1.0" encoding="utf-8" ?>
<D:multistatus xmlns:D="DAV:" xmlns:CARD="urn:ietf:params:xml:ns:carddav">"#,
    );

    for c in contacts {
        xml.push_str(&format!(
            r#"
  <D:response>
    <D:href>/carddav/{username}/contacts/{}.vcf</D:href>
    <D:propstat>
      <D:prop>
        <D:getetag>{}</D:getetag>
        <CARD:address-data><![CDATA[{}]]></CARD:address-data>
      </D:prop>
      <D:status>HTTP/1.1 200 OK</D:status>
    </D:propstat>
  </D:response>"#,
            c.uid, c.etag, c.vcard_data
        ));
    }

    xml.push_str("\n</D:multistatus>");
    multistatus_response(xml)
}

async fn handle_carddav_get(
    book: &fastrmail_core::AddressBook,
    segments: &[&str],
    db: &Database,
) -> Response {
    let filename = match segments.last() {
        Some(f) => *f,
        None => return not_found(),
    };
    let uid = filename.trim_end_matches(".vcf");

    match db.get_contact_by_uid(&book.id, uid) {
        Ok(Some(c)) => Response::builder()
            .status(StatusCode::OK)
            .header(CONTENT_TYPE, "text/vcard; charset=utf-8")
            .header("ETag", c.etag)
            .body(Bytes::from(c.vcard_data).into_response().into_body())
            .unwrap(),
        _ => not_found(),
    }
}

async fn handle_carddav_put(
    book: &fastrmail_core::AddressBook,
    segments: &[&str],
    db: &Database,
    body: Bytes,
) -> Response {
    let filename = match segments.last() {
        Some(f) => *f,
        None => return not_found(),
    };
    let uid = filename.trim_end_matches(".vcf");
    let vcard_data = String::from_utf8_lossy(&body).to_string();

    match db.put_contact(&book.id, uid, &vcard_data) {
        Ok(c) => Response::builder()
            .status(StatusCode::CREATED)
            .header("ETag", c.etag)
            .body(Bytes::new().into_response().into_body())
            .unwrap(),
        Err(e) => Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(Bytes::from(format!("Failed to save contact: {e}")).into_response().into_body())
            .unwrap(),
    }
}

async fn handle_carddav_delete(
    book: &fastrmail_core::AddressBook,
    segments: &[&str],
    db: &Database,
) -> Response {
    let filename = match segments.last() {
        Some(f) => *f,
        None => return not_found(),
    };
    let uid = filename.trim_end_matches(".vcf");

    match db.delete_contact(&book.id, uid) {
        Ok(true) => Response::builder()
            .status(StatusCode::NO_CONTENT)
            .body(Bytes::new().into_response().into_body())
            .unwrap(),
        _ => not_found(),
    }
}

// ─── Helpers ─────────────────────────────────────────────────

fn multistatus_response(xml: String) -> Response {
    Response::builder()
        .status(StatusCode::MULTI_STATUS)
        .header(CONTENT_TYPE, XML_CONTENT_TYPE)
        .body(Bytes::from(xml).into_response().into_body())
        .unwrap()
}

fn not_found() -> Response {
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(Bytes::from("Not Found").into_response().into_body())
        .unwrap()
}
