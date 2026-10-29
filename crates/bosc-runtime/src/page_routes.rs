//! HTTP routes of the compiled page service.
//!
//! | Route | Purpose |
//! | :--- | :--- |
//! | `GET  /api/v1/pages/manifest` | page manifest (ETag, `no-cache`) |
//! | `GET  /api/v1/pages/manifest/stream` | SSE: one `manifest` event per manifest change |
//! | `GET  /page-artifacts/{db}/{file}` | compiled page module (`immutable`; compiled on first request) |
//! | `GET  /page-artifacts/{db}/preview/{file}` | designer preview module |
//! | `GET  /api/v1/pages/{name}/artifact.svelte` | generated Svelte source (Vite dev plugin) |
//! | `GET  /api/v1/pages/{name}/compiled` | compiled page IR, diagnostics and artifact URL |
//! | `POST /api/v1/pages/preview` | compile an unsaved page definition |
//! | `POST /api/v1/pages/{name}/triggers/{trigger}` | run a server-side page trigger / action script |

use axum::{
    Json, Router,
    body::Body,
    extract::{Extension, Path},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::sse::{Event, KeepAlive, Sse},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use tokio_stream::StreamExt;
use tokio_stream::wrappers::WatchStream;

use crate::page_artifacts::{PageService, PageServiceError};
use crate::server::{AppError, extract_context};
use crate::BosEngine;

pub(crate) fn page_routes() -> Router {
    Router::new()
        .merge(crate::page_designer::designer_routes())
        .route("/api/v1/pages/manifest", get(manifest_handler))
        .route("/api/v1/pages/manifest/stream", get(manifest_stream_handler))
        .route("/api/v1/pages/preview", post(preview_handler))
        .route("/api/v1/pages/{name}/artifact.svelte", get(page_source_handler))
        .route("/api/v1/pages/{name}/compiled", get(compiled_page_handler))
        .route("/api/v1/pages/{name}/triggers/{trigger}", post(trigger_handler))
        .route("/page-artifacts/{db}/preview/{file}", get(preview_artifact_handler))
        .route("/page-artifacts/{db}/{file}", get(artifact_handler))
}

// Same early-return shape as `server::extract_context`: the error is the finished HTTP response.
#[allow(clippy::result_large_err)]
fn service(engine: &BosEngine) -> Result<Arc<PageService>, Response> {
    engine.page_service().ok_or_else(|| {
        AppError::new(StatusCode::SERVICE_UNAVAILABLE, "page service is not running on this node")
            .with_code("PAGES_UNAVAILABLE")
            .into_response()
    })
}

fn error_response(e: PageServiceError) -> Response {
    let (status, code) = match &e {
        PageServiceError::NotReady => (StatusCode::SERVICE_UNAVAILABLE, "PAGES_LOADING"),
        PageServiceError::UnknownPage(_) | PageServiceError::UnknownArtifact(_) => (StatusCode::NOT_FOUND, "NOT_FOUND"),
        PageServiceError::CompilerUnavailable(_) => (StatusCode::SERVICE_UNAVAILABLE, "COMPILER_UNAVAILABLE"),
        PageServiceError::BadRequest(_) => (StatusCode::BAD_REQUEST, "BAD_REQUEST"),
        PageServiceError::Generate { .. } => (StatusCode::UNPROCESSABLE_ENTITY, "PAGE_INVALID"),
        PageServiceError::Compiler(_) | PageServiceError::Io { .. } | PageServiceError::Catalog(_) => {
            (StatusCode::INTERNAL_SERVER_ERROR, "PAGE_COMPILE_FAILED")
        }
    };
    AppError::new(status, e.to_string()).with_code(code).into_response()
}

fn quoted_etag(etag: &str) -> String {
    format!("\"{etag}\"")
}

async fn manifest_handler(headers: HeaderMap, Extension(engine): Extension<Arc<BosEngine>>) -> Response {
    let svc = match service(&engine) {
        Ok(s) => s,
        Err(r) => return r,
    };
    let Some((etag, body)) = svc.manifest() else {
        return error_response(PageServiceError::NotReady);
    };
    let tag = quoted_etag(&etag);
    let matches = headers
        .get(header::IF_NONE_MATCH)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.split(',').any(|t| t.trim() == tag || t.trim() == "*"));
    let mut res = if matches {
        StatusCode::NOT_MODIFIED.into_response()
    } else {
        let mut r = Response::new(Body::from(body.as_ref().clone()));
        r.headers_mut().insert(header::CONTENT_TYPE, HeaderValue::from_static("application/json"));
        r
    };
    let h = res.headers_mut();
    h.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-cache"));
    if let Ok(v) = HeaderValue::from_str(&tag) {
        h.insert(header::ETAG, v);
    }
    res
}

async fn manifest_stream_handler(Extension(engine): Extension<Arc<BosEngine>>) -> Response {
    let svc = match service(&engine) {
        Ok(s) => s,
        Err(r) => return r,
    };
    let stream = WatchStream::new(svc.subscribe())
        .filter(|etag| !etag.is_empty())
        .map(|etag| Ok::<Event, std::convert::Infallible>(Event::default().event("manifest").data(etag)));
    Sse::new(stream).keep_alive(KeepAlive::default()).into_response()
}

fn module_response(bytes: Vec<u8>, cache: &'static str) -> Response {
    let mut res = Response::new(Body::from(bytes));
    let h = res.headers_mut();
    h.insert(header::CONTENT_TYPE, HeaderValue::from_static("text/javascript; charset=utf-8"));
    h.insert(header::CACHE_CONTROL, HeaderValue::from_static(cache));
    h.insert(header::X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static("nosniff"));
    res
}

async fn artifact_handler(Extension(engine): Extension<Arc<BosEngine>>, Path((db, file)): Path<(String, String)>) -> Response {
    let svc = match service(&engine) {
        Ok(s) => s,
        Err(r) => return r,
    };
    match svc.artifact(&db, &file).await {
        Ok(bytes) => module_response(bytes, "public, max-age=31536000, immutable"),
        Err(e) => error_response(e),
    }
}

async fn preview_artifact_handler(Extension(engine): Extension<Arc<BosEngine>>, Path((db, file)): Path<(String, String)>) -> Response {
    let svc = match service(&engine) {
        Ok(s) => s,
        Err(r) => return r,
    };
    match svc.preview_artifact(&db, &file).await {
        Ok(bytes) => module_response(bytes, "private, max-age=3600"),
        Err(e) => error_response(e),
    }
}

async fn page_source_handler(Extension(engine): Extension<Arc<BosEngine>>, Path(name): Path<String>) -> Response {
    let svc = match service(&engine) {
        Ok(s) => s,
        Err(r) => return r,
    };
    let Some(entry) = svc.page(&name) else {
        return error_response(PageServiceError::UnknownPage(name));
    };
    let mut res = Response::new(Body::from(entry.source.as_ref().clone()));
    let h = res.headers_mut();
    h.insert(header::CONTENT_TYPE, HeaderValue::from_static("text/plain; charset=utf-8"));
    h.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-cache"));
    if let Ok(v) = HeaderValue::from_str(&quoted_etag(&entry.hash[..16])) {
        h.insert(header::ETAG, v);
    }
    res
}

async fn compiled_page_handler(Extension(engine): Extension<Arc<BosEngine>>, Path(name): Path<String>) -> Response {
    let svc = match service(&engine) {
        Ok(s) => s,
        Err(r) => return r,
    };
    let Some(entry) = svc.page(&name) else {
        return error_response(PageServiceError::UnknownPage(name));
    };
    let url = svc.db_key().map(|db| format!("{}/{db}/{}", crate::page_artifacts::ARTIFACT_ROUTE, entry.file));
    // Authored base page and its extensions in merge order: the designer previews a draft against them.
    let (base, extensions) = svc.base_page(&entry.name).unzip();
    Json(serde_json::json!({
        "page": entry.compiled,
        "diagnostics": entry.diagnostics,
        "hash": entry.hash,
        "url": url,
        "base": base,
        "extensions": extensions.unwrap_or_default(),
    }))
    .into_response()
}

#[derive(Debug, Deserialize)]
pub struct PreviewRequest {
    pub page: bosc_ir::PageIR,
    #[serde(default)]
    pub extensions: Option<Vec<bosc_ir::PageExtensionIR>>,
    #[serde(default = "default_true")]
    pub design: bool,
}

fn default_true() -> bool {
    true
}

/// Roles allowed to compile unsaved page definitions (designer preview).
const DESIGNER_ROLES: &[&str] = &["SUPER", "Admin", "Global Admin", "Developer", "PageDesigner"];

async fn preview_handler(headers: HeaderMap, Extension(engine): Extension<Arc<BosEngine>>, Json(req): Json<PreviewRequest>) -> Response {
    let mut ctx = match extract_context(&headers) {
        Ok(ctx) => ctx,
        Err(r) => return r,
    };
    if ctx.tenant_id.is_nil() {
        let pool = engine.pool().await;
        if let Ok(Some(tid)) = sqlx::query_scalar::<_, uuid::Uuid>(
            "SELECT tenant_id FROM \"_bos_tenants\" WHERE is_active = true ORDER BY created_at LIMIT 1",
        )
        .fetch_optional(&pool)
        .await
        {
            ctx.tenant_id = tid;
        }
    }
    if !ctx.roles.iter().any(|r| DESIGNER_ROLES.iter().any(|d| d.eq_ignore_ascii_case(r))) {
        return AppError::new(StatusCode::FORBIDDEN, "page preview requires a designer role").with_code("FORBIDDEN").into_response();
    }
    let svc = match service(&engine) {
        Ok(s) => s,
        Err(r) => return r,
    };
    match svc.preview(req.page, req.extensions, req.design).await {
        Ok(result) => Json(result).into_response(),
        Err(e) => error_response(e),
    }
}

#[derive(Debug, Deserialize, Default)]
pub struct TriggerRequest {
    #[serde(default)]
    pub record: Option<serde_json::Value>,
    #[serde(default)]
    pub variables: Option<serde_json::Value>,
    #[serde(default)]
    pub ui_answers: HashMap<String, bool>,
}

/// Extracts the question of a `ctx.ui.confirm` that still needs an answer.
fn pending_confirm(message: &str) -> Option<String> {
    let start = message.find("ConfirmRequired: ")? + "ConfirmRequired: ".len();
    let rest = &message[start..];
    let end = rest.find(" [stack:").unwrap_or(rest.len());
    Some(rest[..end].trim().to_string())
}

async fn trigger_handler(
    headers: HeaderMap,
    Extension(engine): Extension<Arc<BosEngine>>,
    Path((name, trigger)): Path<(String, String)>,
    body: Option<Json<TriggerRequest>>,
) -> Response {
    let mut ctx = match extract_context(&headers) {
        Ok(ctx) => ctx,
        Err(r) => return r,
    };
    if ctx.tenant_id.is_nil() {
        let pool = engine.pool().await;
        if let Ok(Some(tid)) = sqlx::query_scalar::<_, uuid::Uuid>(
            "SELECT tenant_id FROM \"_bos_tenants\" WHERE is_active = true ORDER BY created_at LIMIT 1",
        )
        .fetch_optional(&pool)
        .await
        {
            ctx.tenant_id = tid;
        }
    }
    let req = body.map(|Json(b)| b).unwrap_or_default();
    let record = req.record.unwrap_or_else(|| serde_json::json!({}));
    let variables = req.variables.unwrap_or_else(|| serde_json::json!({}));
    match engine.run_page_trigger(&name, &trigger, record, variables, req.ui_answers, &ctx).await {
        Ok(result) => Json(result).into_response(),
        Err(e) => {
            let message = e.to_string();
            if let Some(question) = pending_confirm(&message) {
                let body = serde_json::json!({ "error": message, "code": "CONFIRM_REQUIRED", "confirm": question });
                return (StatusCode::CONFLICT, Json(body)).into_response();
            }
            let status = if message.contains("does not exist") || message.contains("has no script") {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::BAD_REQUEST
            };
            AppError::new(status, message).into_response()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn confirm_question_is_extracted() {
        let msg = "Business logic exception in 'User.action.Post' (Error): ConfirmRequired: Post the order? [stack: at x]";
        assert_eq!(pending_confirm(msg).as_deref(), Some("Post the order?"));
        assert_eq!(pending_confirm("ConfirmRequired: Delete?").as_deref(), Some("Delete?"));
        assert_eq!(pending_confirm("other failure"), None);
    }
}
