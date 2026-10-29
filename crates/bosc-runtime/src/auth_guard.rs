//! Session guard for the HTTP API (docs/08 section 2, `require_session`).
//!
//! * Public routes (docs/08 2.1) pass through untouched.
//! * Every other `/api/*` route needs a session token in `Authorization: Bearer <token>`
//!   or `x-session-token`. The token is verified through the `bos.core` package
//!   (`AuthManager.VerifySession`); the kernel keeps no session state of its own.
//! * Client-supplied identity headers (`x-bos-session`, `x-bos-user-id`, `x-bos-roles`)
//!   are always stripped; on success the guard injects a server-built `x-bos-session`
//!   so the existing handlers pick up the authenticated tenant/roles and RLS
//!   (`bos.current_tenant_id`) is set from the session, never from the caller.

use crate::{BosEngine, SessionContext};
use axum::{
    Json,
    body::Body,
    extract::Extension,
    http::{HeaderMap, HeaderValue, Method, Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

/// Routes that docs/08 2.1 lists as public, plus the unauthenticated bootstrap
/// reads the Sign-In screen needs. Non-`/api` paths (SPA, static artifacts) are not guarded.
pub fn is_public_route(method: &Method, path: &str) -> bool {
    if !path.starts_with("/api/") {
        return true;
    }
    if method == Method::OPTIONS {
        return true;
    }
    let exact = [
        "/api/health",
        "/api/auth/status",
        "/api/auth/bootstrap",
        "/api/auth/login",
        "/api/detect-db",
        "/api/node/info",
        "/api/v1/pages/manifest",
        "/api/v1/pages/manifest/stream",
        "/api/pages/LoginPage",
        "/api/v1/pages/LoginPage",
        "/api/pages/SignIn",
        "/api/v1/pages/SignIn",
        "/api/notifications/stream",
    ];
    if exact.contains(&path) {
        return true;
    }
    if path == "/api/auth/config" && method == Method::GET {
        return true;
    }
    if path.starts_with("/api/setup/") {
        return true;
    }
    // Unauthenticated login pages (LoginPage & SignIn) can run their sign-in action triggers
    if (path.starts_with("/api/v1/pages/LoginPage/triggers/")
        || path.starts_with("/api/pages/LoginPage/triggers/")
        || path.starts_with("/api/v1/pages/SignIn/triggers/")
        || path.starts_with("/api/pages/SignIn/triggers/"))
        && method == Method::POST
    {
        return true;
    }
    // Setup wizard only needs to list databases and test a connection before an account exists.
    // Database management (switch/manage/install/tenants) stays behind the session guard.
    if (path == "/api/databases" && method == Method::GET) || path == "/api/databases/test-connection" {
        return true;
    }
    false
}

/// Extract the bearer / x-session-token value.
pub fn token_from_headers(headers: &HeaderMap) -> Option<String> {
    if let Some(v) = headers.get("authorization").and_then(|v| v.to_str().ok()) {
        let v = v.trim();
        if let Some(rest) = v.strip_prefix("Bearer ").or_else(|| v.strip_prefix("bearer ")) {
            let t = rest.trim();
            if !t.is_empty() {
                return Some(t.to_string());
            }
        }
    }
    headers
        .get("x-session-token")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn header_uuid(headers: &HeaderMap, name: &str) -> Uuid {
    headers
        .get(name)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| Uuid::parse_str(s.trim()).ok())
        .unwrap_or_else(Uuid::nil)
}

/// First active tenant, mirroring the login handler's default-tenant rule.
pub async fn default_tenant(engine: &BosEngine) -> Uuid {
    let pool = engine.pool().await;
    sqlx::query_scalar::<_, Uuid>(
        "SELECT tenant_id FROM \"_bos_tenants\" WHERE is_active = true ORDER BY created_at LIMIT 1",
    )
    .fetch_optional(&pool)
    .await
    .ok()
    .flatten()
    .unwrap_or_else(Uuid::nil)
}

fn unauthorized(reason: &str) -> Response {
    (
        StatusCode::UNAUTHORIZED,
        Json(serde_json::json!({
            "error": "Authentication required.",
            "message": "Authentication required.",
            "code": "UNAUTHENTICATED",
            "reason": reason,
        })),
    )
        .into_response()
}

/// Verify a token and build the authenticated session context.
pub async fn resolve_session(
    engine: &BosEngine,
    headers: &HeaderMap,
) -> Result<SessionContext, Response> {
    let token = token_from_headers(headers).ok_or_else(|| unauthorized("MISSING_TOKEN"))?;

    // Tenant used to look the session up; the session row itself is tenant-scoped by RLS,
    // so a token from another tenant is simply not found.
    let mut tenant_id = header_uuid(headers, "x-bos-tenant-id");
    if tenant_id.is_nil() {
        tenant_id = default_tenant(engine).await;
    }
    let company_id = header_uuid(headers, "x-bos-company-id");

    // Fast-path for active, registered emergency super admin sessions
    if let Some(roles) = engine.validate_emergency_session(&token) {
        return Ok(SessionContext {
            tenant_id,
            company_id,
            user_id: Uuid::nil(),
            roles,
        });
    }
    let lookup_ctx = SessionContext {
        tenant_id,
        company_id,
        user_id: Uuid::nil(),
        roles: vec!["SUPER".to_string()],
    };
    let ip = headers
        .get("x-forwarded-for")
        .or_else(|| headers.get("x-real-ip"))
        .and_then(|v| v.to_str().ok())
        .unwrap_or("127.0.0.1")
        .to_string();

    let res = engine
        .execute_procedure_with(
            "AuthManager",
            "VerifySession",
            serde_json::json!({ "token": token, "ipAddress": ip }),
            &lookup_ctx,
            HashMap::new(),
        )
        .await
        .map_err(|e| {
            tracing::warn!("AuthManager.VerifySession failed: {}", e);
            unauthorized("SESSION_VERIFY_FAILED")
        })?;
    let data = res.mutated_data;
    if !data.get("valid").and_then(|v| v.as_bool()).unwrap_or(false) {
        let reason = data
            .get("reason")
            .and_then(|v| v.as_str())
            .unwrap_or("INVALID_SESSION")
            .to_string();
        return Err(unauthorized(&reason));
    }
    let roles = data
        .get("roles")
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|r| r.as_str().map(|s| s.to_string()))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    Ok(SessionContext {
        tenant_id,
        company_id,
        user_id: Uuid::nil(),
        roles,
    })
}

/// Remove identity headers a client could use to impersonate a user or tenant.
pub fn strip_identity_headers(headers: &mut HeaderMap) {
    for h in ["x-bos-session", "x-bos-user-id", "x-bos-roles"] {
        headers.remove(h);
    }
}

/// Axum middleware implementing docs/08 `require_session`.
pub async fn require_session(
    Extension(engine): Extension<Arc<BosEngine>>,
    mut req: Request<Body>,
    next: Next,
) -> Response {
    strip_identity_headers(req.headers_mut());
    let public = is_public_route(req.method(), req.uri().path());

    // Public routes still get the caller's session when one is presented (e.g. auth/config POST
    // is protected, but status reads may want it); a bad token on a public route is ignored.
    let resolved = if public && token_from_headers(req.headers()).is_none() {
        None
    } else {
        match resolve_session(&engine, req.headers()).await {
            Ok(ctx) => Some(ctx),
            Err(resp) if !public => return resp,
            Err(_) => None,
        }
    };

    if let Some(ctx) = resolved {
        if let Ok(json) = serde_json::to_string(&ctx) {
            if let Ok(v) = HeaderValue::from_str(&json) {
                req.headers_mut().insert("x-bos-session", v);
                if let Ok(t) = HeaderValue::from_str(&ctx.tenant_id.to_string()) {
                    req.headers_mut().insert("x-bos-tenant-id", t);
                }
            }
        }
    }
    next.run(req).await
}

/// Identity table state used by first-run detection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IdentityState {
    pub user_table_exists: bool,
    pub users: i64,
    pub roles: i64,
}

impl IdentityState {
    /// No usable identity data: nobody can sign in, so the node needs bootstrap/setup.
    pub fn needs_bootstrap(&self) -> bool {
        !self.user_table_exists || self.users == 0 || self.roles == 0
    }
}

/// Count rows in the `bos.core` identity tables across all tenants.
pub async fn identity_state(engine: &BosEngine) -> IdentityState {
    let pool = engine.pool().await;
    let exists: bool = sqlx::query_scalar(
        "SELECT to_regclass('public.\"user\"') IS NOT NULL AND to_regclass('public.\"role\"') IS NOT NULL",
    )
    .fetch_one(&pool)
    .await
    .unwrap_or(false);
    if !exists {
        return IdentityState { user_table_exists: false, users: 0, roles: 0 };
    }
    let users: i64 = sqlx::query_scalar("SELECT count(*) FROM \"user\"")
        .fetch_one(&pool)
        .await
        .unwrap_or(0);
    let roles: i64 = sqlx::query_scalar("SELECT count(*) FROM \"role\"")
        .fetch_one(&pool)
        .await
        .unwrap_or(0);
    IdentityState { user_table_exists: true, users, roles }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_routes_match_docs_08() {
        for p in ["/api/health", "/api/auth/status", "/api/auth/login", "/api/auth/bootstrap",
                  "/api/setup/status", "/api/setup/provision", "/api/detect-db", "/api/databases",
                  "/api/node/info", "/api/v1/pages/manifest", "/api/v1/pages/manifest/stream",
                  "/", "/index.html"] {
            assert!(is_public_route(&Method::GET, p), "{p} should be public");
        }
        assert!(is_public_route(&Method::GET, "/api/auth/config"));
        assert!(!is_public_route(&Method::POST, "/api/auth/config"));
        for p in ["/api/entities/user", "/api/entities/role/1", "/api/query", "/api/permissions/check",
                  "/api/packages/bos.core/export", "/api/codeunits/AuthManager/run", "/api/script/apply",
                  "/api/meta/objects", "/api/tenants",
                  "/api/databases/switch", "/api/databases/current/manage", "/api/databases/x/packages/install"] {
            assert!(!is_public_route(&Method::GET, p), "{p} must be protected");
            assert!(!is_public_route(&Method::POST, p), "{p} must be protected");
        }
    }

    #[test]
    fn token_extraction() {
        let mut h = HeaderMap::new();
        assert_eq!(token_from_headers(&h), None);
        h.insert("authorization", HeaderValue::from_static("Bearer abc"));
        assert_eq!(token_from_headers(&h).as_deref(), Some("abc"));
        let mut h2 = HeaderMap::new();
        h2.insert("x-session-token", HeaderValue::from_static("xyz"));
        assert_eq!(token_from_headers(&h2).as_deref(), Some("xyz"));
    }

    #[test]
    fn identity_headers_are_stripped() {
        let mut h = HeaderMap::new();
        h.insert("x-bos-session", HeaderValue::from_static("{}"));
        h.insert("x-bos-roles", HeaderValue::from_static("SUPER"));
        h.insert("x-bos-user-id", HeaderValue::from_static("x"));
        strip_identity_headers(&mut h);
        assert!(h.is_empty());
    }

    #[test]
    fn empty_identity_needs_bootstrap() {
        assert!(IdentityState { user_table_exists: false, users: 0, roles: 0 }.needs_bootstrap());
        assert!(IdentityState { user_table_exists: true, users: 0, roles: 0 }.needs_bootstrap());
        assert!(IdentityState { user_table_exists: true, users: 1, roles: 0 }.needs_bootstrap());
        assert!(!IdentityState { user_table_exists: true, users: 1, roles: 2 }.needs_bootstrap());
    }
}