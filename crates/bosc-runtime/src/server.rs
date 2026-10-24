use crate::db_manager::{
    CreateCompanyRequest, CreateDatabaseRequest, CreateTenantRequest, DropDatabaseQuery,
    ManageDatabaseRequest, SwitchDatabaseRequest, TestConnectionRequest, UpdateDatabaseRequest,
};
use crate::detector::DatabaseDetector;
use crate::node_config::NodeConfig;
use crate::{
    ApplyTableScriptRequest, BosEngine, CompileScriptRequest, FirstRunProvisionRequest,
    QueryRequest, RegisterMetaObjectRequest, RegisterTableExtensionRequest, RuntimeError,
    SessionContext,
};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::{
    Json, Router,
    extract::{Extension, Path, Query},
    http::{HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde::Deserialize;
use std::sync::Arc;
use tokio_stream::StreamExt;
use tokio_stream::wrappers::BroadcastStream;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::{ServeDir, ServeFile};
use tower_http::trace::TraceLayer;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct AppError {
    pub status: StatusCode,
    pub message: String,
    pub code: Option<String>,
    pub notifications: Vec<crate::Notification>,
    pub field_errors: std::collections::HashMap<String, String>,
}

fn extract_field_from_error(msg: &str) -> Option<String> {
    if let Some(pos) = msg.find("null value in column \"") {
        let rest = &msg[pos + 22..];
        if let Some(end) = rest.find('"') {
            return Some(rest[..end].to_string());
        }
    }
    if let Some(pos) = msg.find("Key (") {
        let rest = &msg[pos + 5..];
        if let Some(end) = rest.find(')') {
            return Some(rest[..end].to_string());
        }
    }
    None
}

#[allow(non_snake_case)]
pub fn AppError(status: StatusCode, message: String) -> AppError {
    let mut field_errors = std::collections::HashMap::new();
    let mut notifications = Vec::new();

    if let Some(field) = extract_field_from_error(&message) {
        notifications.push(crate::Notification::infield(&field, &message));
        field_errors.insert(field, message.clone());
    } else {
        notifications.push(crate::Notification::error(&message));
    }

    AppError {
        status,
        message,
        code: None,
        notifications,
        field_errors,
    }
}

impl AppError {
    pub fn new(status: StatusCode, message: impl Into<String>) -> Self {
        AppError(status, message.into())
    }

    pub fn bad_request(msg: impl Into<String>) -> Self {
        AppError(StatusCode::BAD_REQUEST, msg.into())
    }

    pub fn not_found(msg: impl Into<String>) -> Self {
        AppError(StatusCode::NOT_FOUND, msg.into())
    }

    pub fn unauthorized(msg: impl Into<String>) -> Self {
        AppError(StatusCode::UNAUTHORIZED, msg.into())
    }

    pub fn with_code(mut self, code: impl Into<String>) -> Self {
        self.code = Some(code.into());
        self
    }

    pub fn with_notification(mut self, notification: crate::Notification) -> Self {
        self.notifications.push(notification);
        self
    }

    pub fn with_field_error(mut self, field: impl Into<String>, msg: impl Into<String>) -> Self {
        let f = field.into();
        let m = msg.into();
        self.notifications
            .push(crate::Notification::infield(&f, &m));
        self.field_errors.insert(f, m);
        self
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let notif_header_val = serde_json::to_string(&self.notifications)
            .ok()
            .and_then(|s| HeaderValue::from_str(&s).ok());

        let body = Json(serde_json::json!({
            "error": self.message.clone(),
            "message": self.message.clone(),
            "code": self.code.as_deref().unwrap_or(if self.status.is_client_error() { "CLIENT_ERROR" } else { "INTERNAL_ERROR" }),
            "notifications": self.notifications,
            "field_errors": self.field_errors,
        }));

        let mut res = (self.status, body).into_response();
        if let Some(h) = notif_header_val {
            res.headers_mut().insert("x-bos-notifications", h);
        }
        res
    }
}

pub fn json_response_with_notifications(
    status: StatusCode,
    mut body: serde_json::Value,
    notifications: Vec<crate::Notification>,
) -> Response {
    if let serde_json::Value::Object(ref mut map) = body {
        map.insert(
            "notifications".to_string(),
            serde_json::to_value(&notifications).unwrap_or(serde_json::Value::Array(vec![])),
        );
    }
    let notif_header_val = serde_json::to_string(&notifications)
        .ok()
        .and_then(|s| HeaderValue::from_str(&s).ok());

    let mut res = (status, Json(body)).into_response();
    if let Some(h) = notif_header_val {
        res.headers_mut().insert("x-bos-notifications", h);
    }
    res
}

pub fn create_api_router(engine: BosEngine) -> Router {
    create_node_router(engine, NodeConfig::default())
}

pub fn create_node_router(engine: BosEngine, node_config: NodeConfig) -> Router {
    let engine_arc = Arc::new(engine);
    let node_config_arc = Arc::new(node_config.clone());

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let mut router = Router::new()
        .route("/api/health", get(health_check))
        .route("/api/node/info", get(node_info_handler))
        .route(
            "/api/notifications/stream",
            get(notification_stream_handler),
        )
        .route(
            "/api/notifications",
            get(list_notifications_handler).delete(clear_notifications_handler),
        )
        .route(
            "/api/notifications/test",
            post(send_test_notification_handler),
        )
        .route("/api/auth/status", get(auth_status))
        .route("/api/auth/login", post(auth_login_handler))
        .route(
            "/api/auth/config",
            get(auth_config_get_handler).post(auth_config_post_handler),
        )
        .route("/api/setup/status", get(auth_status))
        .route("/api/setup/provision", post(setup_provision_handler))
        .route("/api/detect-db", get(detect_databases))
        .route(
            "/api/databases",
            get(list_databases_handler).post(create_database_handler),
        )
        .route(
            "/api/databases/current",
            get(get_current_database_handler)
                .put(update_current_database_handler)
                .delete(delete_current_database_handler),
        )
        .route(
            "/api/databases/current/manage",
            post(manage_current_database_handler),
        )
        .route(
            "/api/database/current",
            get(get_current_database_handler)
                .put(update_current_database_handler)
                .delete(delete_current_database_handler),
        )
        .route(
            "/api/database/current/manage",
            post(manage_current_database_handler),
        )
        .route(
            "/api/databases/{name}",
            get(get_database_handler)
                .put(update_database_handler)
                .delete(drop_database_handler),
        )
        .route(
            "/api/databases/{name}/manage",
            post(manage_database_handler),
        )
        .route(
            "/api/databases/{name}/packages/install",
            post(install_package_to_database_handler),
        )
        .route(
            "/api/databases/test-connection",
            post(test_connection_handler),
        )
        .route("/api/databases/switch", post(switch_database_handler))
        .route(
            "/api/databases/{name}/tenants",
            get(list_database_tenants_handler),
        )
        .route(
            "/api/databases/{name}/companies",
            get(list_database_companies_handler),
        )
        .route("/api/meta/provision", post(provision_system_handler))
        .route("/api/meta/system-status", get(get_system_status_handler))
        .route("/api/meta/packages", get(list_packages_handler))
        .route(
            "/api/meta/objects",
            get(list_meta_objects_handler).post(register_meta_object_handler),
        )
        .route("/api/meta/objects/{name}", get(get_meta_object_handler))
        .route("/api/pages/{name}", get(get_resolved_page_handler))
        .route("/api/v1/pages/{name}", get(get_resolved_page_handler))
        .route("/api/meta/ambient.d.ts", get(get_ambient_dts_handler))
        .route("/api/v1/meta/ambient.d.ts", get(get_ambient_dts_handler))
        .route(
            "/api/meta/table-extensions",
            post(register_table_extension_handler),
        )
        .route(
            "/api/meta/tables/{table}/script",
            post(apply_table_script_handler),
        )
        .route("/api/script/compile", post(compile_script_handler))
        .route("/api/script/apply", post(apply_script_handler))
        .route("/api/packages/install", post(install_package_handler))
        .route("/api/packages/{id}/export", get(export_package_handler))
        .route("/api/codeunits", post(register_codeunit_handler))
        .route("/api/codeunits/{name}/run", post(execute_codeunit_handler))
        .route("/api/tests/run", post(run_tests_handler))
        .route(
            "/api/system/storage-strategies",
            get(list_storage_strategies_handler),
        )
        .route(
            "/api/tenants",
            get(list_tenants_handler).post(create_tenant_handler),
        )
        .route(
            "/api/companies",
            get(list_companies_handler).post(create_company_handler),
        )
        .route("/api/views/{name}", get(get_view_handler))
        .route("/api/views/{name}/refresh", post(refresh_view_handler))
        .route("/api/queries/{name}/run", post(execute_query_handler))
        .route("/api/permissions/check", post(check_permission_handler))
        .route("/api/query", post(execute_graphql_query))
        .route(
            "/api/entities/{table}/schema",
            get(get_entity_schema_handler),
        )
        .route("/api/entities/{table}/batch", post(create_entities_batch))
        .route(
            "/api/entities/{table}",
            get(query_entities).post(create_entity),
        )
        .route(
            "/api/entities/{table}/{id}",
            get(get_entity)
                .put(update_entity)
                .patch(update_entity)
                .delete(delete_entity),
        )
        .route("/api/entities/{table}/{id}/lock", post(lock_entity))
        .route("/api/entities/{table}/lock", post(lock_table_handler))
        .route(
            "/api/entities/{table}/{id}/transition",
            post(execute_transition),
        )
        .route(
            "/api/entities/{table}/{id}/state-history",
            get(get_state_history),
        )
        .route(
            "/api/entities/{table}/{id}/privileged-override",
            post(privileged_override),
        )
        .route("/api/jobs/enqueue", post(enqueue_job_handler))
        .route("/api/jobs/process-next", post(execute_next_job_handler))
        .route(
            "/api/events/subscribers",
            get(list_subscribers_handler).post(register_subscriber_handler),
        )
        .route("/api/events/publish", post(publish_event_handler))
        .route(
            "/api/events/outbox",
            get(list_outbox_handler).post(enqueue_outbox_handler),
        )
        .route("/api/events/outbox/process", post(process_outbox_handler))
        .route(
            "/api/wasm/modules",
            get(list_wasm_modules_handler).post(load_wasm_module_handler),
        )
        .route(
            "/api/wasm/{module}/{func}/invoke",
            post(invoke_wasm_function_handler),
        )
        .route(
            "/api/sidecars",
            get(list_sidecars_handler).post(register_sidecar_handler),
        )
        .route("/api/sidecars/{name}/call", post(call_sidecar_handler))
        .route(
            "/api/sidecars/{name}/health",
            get(check_sidecar_health_handler),
        )
        .merge(crate::page_routes::page_routes())
        // docs/08 2: every non-public /api route requires an authenticated session.
        // Must sit inside the Extension layers so the guard can reach the engine.
        .layer(axum::middleware::from_fn(crate::auth_guard::require_session))
        .layer(Extension(engine_arc))
        .layer(Extension(node_config_arc))
        .layer(cors)
        .layer(TraceLayer::new_for_http());

    // Mount static compiled package artifacts (Svelte 5 ESM page chunks, styles, assets)
    let packages_artifacts_dir = std::path::Path::new("./dist/packages");
    if packages_artifacts_dir.is_dir() {
        let serve_packages = ServeDir::new(packages_artifacts_dir);
        router = router.nest_service("/packages", serve_packages);
    }

    if node_config.ui.enabled {
        let static_dir = std::path::Path::new(&node_config.ui.static_dir);
        if static_dir.is_dir() {
            let index_path = static_dir.join(&node_config.ui.index_file);
            let serve_dir = ServeDir::new(static_dir).not_found_service(ServeFile::new(index_path));
            router = router.fallback_service(serve_dir);
        }
    }

    router
}

async fn node_info_handler(
    Extension(engine): Extension<Arc<BosEngine>>,
    node_config_opt: Option<Extension<Arc<NodeConfig>>>,
) -> Response {
    let (
        node_id,
        node_name,
        role,
        default_tenant,
        default_company,
        ui_enabled,
        nas_enabled,
        maintenance_mode,
        emergency_username,
    ) = match node_config_opt {
        Some(Extension(cfg)) => (
            cfg.node.id.clone(),
            cfg.node.name.clone(),
            format!("{:?}", cfg.node.role).to_lowercase(),
            cfg.defaults.default_tenant.clone(),
            cfg.defaults.default_company.clone(),
            cfg.ui.enabled,
            cfg.nas_worker.enabled,
            cfg.security.maintenance_mode,
            cfg.security.emergency_username.clone(),
        ),
        None => (
            "node-default".to_string(),
            "BOS Default Node".to_string(),
            "all_in_one".to_string(),
            "DEFAULT_TENANT".to_string(),
            "MAIN_COMPANY".to_string(),
            true,
            false,
            false,
            "emergency.admin".to_string(),
        ),
    };

    let active_db = engine.active_db_name().await;

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "nodeId": node_id,
            "nodeName": node_name,
            "role": role,
            "activeDatabase": active_db,
            "defaultTenant": default_tenant,
            "defaultCompany": default_company,
            "uiEnabled": ui_enabled,
            "nasWorkerEnabled": nas_enabled,
            "maintenanceMode": maintenance_mode,
            "emergencyUsername": emergency_username,
            "kernelVersion": "0.1.0",
            "serverTime": chrono::Utc::now().to_rfc3339()
        })),
    )
        .into_response()
}

fn is_superuser_roles(roles: &[String]) -> bool {
    roles.iter().any(|r| {
        r.eq_ignore_ascii_case("SUPER")
            || r.eq_ignore_ascii_case("Admin")
            || r.eq_ignore_ascii_case("Global Admin")
    })
}

pub(crate) fn extract_context(headers: &HeaderMap) -> Result<SessionContext, Response> {
    if let Some(raw) = headers.get("x-bos-session").and_then(|value| value.to_str().ok()) {
        if let Ok(ctx) = serde_json::from_str::<SessionContext>(raw) {
            return Ok(ctx);
        }
    }
    let tenant_id = headers
        .get("x-bos-tenant-id")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| Uuid::parse_str(s).ok())
        .unwrap_or_else(Uuid::nil);
    let company_id = headers
        .get("x-bos-company-id")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| Uuid::parse_str(s).ok())
        .unwrap_or_else(Uuid::nil);
    let user_id = headers
        .get("x-bos-user-id")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| Uuid::parse_str(s).ok())
        .unwrap_or_else(Uuid::nil);
    let roles = headers
        .get("x-bos-roles")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.split(',').map(|r| r.trim().to_string()).collect())
        // No implicit privileges: protected routes always receive a server-built
        // x-bos-session from auth_guard; anything else is anonymous.
        .unwrap_or_default();

    Ok(SessionContext {
        tenant_id,
        company_id,
        user_id,
        roles,
    })
}

async fn health_check(Extension(engine): Extension<Arc<BosEngine>>) -> Response {
    let pool = engine.pool().await;
    match sqlx::query("SELECT 1").execute(&pool).await {
        Ok(_) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "status": "healthy",
                "database": "up",
                "service": "BOS Core Engine Live API Server",
                "version": "0.1.0"
            })),
        )
            .into_response(),
        Err(err) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "status": "unhealthy",
                "database": "down",
                "message": err.to_string(),
                "service": "BOS Core Engine Live API Server",
                "version": "0.1.0"
            })),
        )
            .into_response(),
    }
}

async fn auth_status(Extension(engine): Extension<Arc<BosEngine>>) -> Response {
    let sys_status =
        engine
            .get_system_status()
            .await
            .unwrap_or(crate::meta_manager::SystemProvisionStatus {
                is_provisioned: false,
                kernel_version: None,
                schema_version: None,
                status: None,
                storage_strategy: None,
                provisioned_at: None,
                meta_objects_count: 0,
                tenants_count: 0,
                companies_count: 0,
            });
    let active_db = engine.active_db_name().await;
    // A provisioned node with no users or roles cannot be signed into: it still needs setup.
    let identity = crate::auth_guard::identity_state(&engine).await;
    let needs_bootstrap = sys_status.is_provisioned && identity.needs_bootstrap();
    let needs_setup = !sys_status.is_provisioned || needs_bootstrap;

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "isProvisioned": sys_status.is_provisioned,
            "needsSetup": needs_setup,
            "needsBootstrap": needs_bootstrap,
            "usersCount": identity.users,
            "rolesCount": identity.roles,
            "activeDatabase": active_db,
            "storageStrategy": sys_status.storage_strategy.unwrap_or_else(|| "shared_rls".to_string()),
            "metaObjectsCount": sys_status.meta_objects_count,
            "tenantsCount": sys_status.tenants_count,
            "companiesCount": sys_status.companies_count
        })),
    )
        .into_response()
}

async fn auth_config_get_handler(
    Extension(engine): Extension<Arc<BosEngine>>,
) -> Response {
    let ctx = SessionContext {
        tenant_id: Uuid::nil(),
        company_id: Uuid::nil(),
        user_id: Uuid::nil(),
        roles: vec!["SUPER".to_string(), "Admin".to_string()],
    };

    // 1. Attempt to query codeunit AuthManager.GetLoginConfiguration
    match engine
        .execute_procedure_with(
            "AuthManager",
            "GetLoginConfiguration",
            serde_json::json!({}),
            &ctx,
            std::collections::HashMap::new(),
        )
        .await
    {
        Ok(exec_res) => {
            let data = &exec_res.mutated_data;
            if data.get("applicationTitle").is_some() {
                return (StatusCode::OK, Json(data.clone())).into_response();
            }
        }
        Err(e) => {
            tracing::debug!("AuthManager.GetLoginConfiguration procedure not available: {}", e);
        }
    }

    // 2. Direct database query fallback
    let pool = engine.pool().await;
    let db_row = sqlx::query(
        "SELECT id, application_title, company_logo_url, welcome_message, primary_color, allow_remember_me, allow_emergency_fallback, sso_enabled, sso_provider_name, sso_login_url, terms_of_service_url, support_email, custom_disclaimer FROM \"auth_configuration\" WHERE \"id\" = 'PRIMARY' LIMIT 1"
    )
    .fetch_optional(&pool)
    .await
    .unwrap_or(None);

    if let Some(row) = db_row {
        use sqlx::Row;
        return (
            StatusCode::OK,
            Json(serde_json::json!({
                "id": row.try_get::<String, _>("id").unwrap_or_else(|_| "PRIMARY".to_string()),
                "applicationTitle": row.try_get::<String, _>("application_title").unwrap_or_else(|_| "BOS Enterprise Platform".to_string()),
                "companyLogoUrl": row.try_get::<Option<String>, _>("company_logo_url").unwrap_or_default().unwrap_or_default(),
                "welcomeMessage": row.try_get::<String, _>("welcome_message").unwrap_or_else(|_| "Sign in to access your enterprise workspace.".to_string()),
                "primaryColor": row.try_get::<String, _>("primary_color").unwrap_or_else(|_| "#4f46e5".to_string()),
                "allowRememberMe": row.try_get::<bool, _>("allow_remember_me").unwrap_or(true),
                "allowEmergencyFallback": row.try_get::<bool, _>("allow_emergency_fallback").unwrap_or(true),
                "ssoEnabled": row.try_get::<bool, _>("sso_enabled").unwrap_or(false),
                "ssoProviderName": row.try_get::<Option<String>, _>("sso_provider_name").unwrap_or_default().unwrap_or_else(|| "Corporate SSO".to_string()),
                "ssoLoginUrl": row.try_get::<Option<String>, _>("sso_login_url").unwrap_or_default().unwrap_or_default(),
                "termsOfServiceUrl": row.try_get::<Option<String>, _>("terms_of_service_url").unwrap_or_default().unwrap_or_default(),
                "supportEmail": row.try_get::<Option<String>, _>("support_email").unwrap_or_default().unwrap_or_else(|| "admin@bos.local".to_string()),
                "customDisclaimer": row.try_get::<Option<String>, _>("custom_disclaimer").unwrap_or_default().unwrap_or_else(|| "Authorized personnel only. All access attempts are monitored and audited.".to_string()),
            })),
        )
            .into_response();
    }

    // 3. Sensible defaults
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "id": "PRIMARY",
            "applicationTitle": "BOS Enterprise Platform",
            "companyLogoUrl": "",
            "welcomeMessage": "Sign in to access your enterprise workspace.",
            "primaryColor": "#4f46e5",
            "allowRememberMe": true,
            "allowEmergencyFallback": true,
            "ssoEnabled": false,
            "ssoProviderName": "Corporate SSO",
            "ssoLoginUrl": "",
            "termsOfServiceUrl": "",
            "supportEmail": "admin@bos.local",
            "customDisclaimer": "Authorized personnel only. All access attempts are monitored and audited."
        })),
    )
        .into_response()
}

async fn auth_config_post_handler(
    headers: HeaderMap,
    Extension(engine): Extension<Arc<BosEngine>>,
    Json(body): Json<serde_json::Value>,
) -> Response {
    let ctx = extract_context(&headers).unwrap_or(SessionContext {
        tenant_id: Uuid::nil(),
        company_id: Uuid::nil(),
        user_id: Uuid::nil(),
        roles: vec!["SUPER".to_string(), "Admin".to_string()],
    });

    match engine
        .execute_procedure_with(
            "AuthManager",
            "UpdateLoginConfiguration",
            body.clone(),
            &ctx,
            std::collections::HashMap::new(),
        )
        .await
    {
        Ok(exec_res) => (StatusCode::OK, Json(exec_res.mutated_data)).into_response(),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": err.to_string() })),
        )
            .into_response(),
    }
}

#[derive(Debug, Deserialize)]
pub struct AuthLoginRequest {
    pub username: String,
    pub password: String,
}

async fn auth_login_handler(
    headers: HeaderMap,
    Extension(engine): Extension<Arc<BosEngine>>,
    node_config_opt: Option<Extension<Arc<NodeConfig>>>,
    Json(body): Json<AuthLoginRequest>,
) -> Response {
    let username = body.username.trim().to_string();
    let password = body.password;

    if username.is_empty() || password.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "Username and password are required."
            })),
        )
            .into_response();
    }

    let node_config = node_config_opt.map(|Extension(c)| c);

    // 1. Check emergency Super Admin bypass (maintenance mode)
    // docs/13 4.3: granted in maintenance mode, or under auto-fallback only when the
    // identity store is missing/empty (nobody could otherwise sign in).
    let is_emergency_name = username == "emergency_admin"
        || node_config
            .as_ref()
            .map(|c| c.security.emergency_username.eq_ignore_ascii_case(&username))
            .unwrap_or(false);
    if is_emergency_name {
        let maintenance = node_config.as_ref().map(|c| c.security.maintenance_mode).unwrap_or(false);
        let auto_fallback = node_config
            .as_ref()
            .map(|c| c.security.auto_fallback_on_auth_failure)
            .unwrap_or(false);
        let is_emergency_allowed = maintenance
            || (auto_fallback && crate::auth_guard::identity_state(&engine).await.needs_bootstrap());

        if is_emergency_allowed {
            let token = format!("bos_sess_emergency_{}", Uuid::new_v4().simple());
            return (
                StatusCode::OK,
                Json(serde_json::json!({
                    "token": token,
                    "user": {
                        "id": "emergency_admin",
                        "username": "emergency_admin",
                        "name": "Emergency Super Admin",
                        "email": "emergency@system.local",
                        "roles": ["SUPER", "Admin", "Global Admin"],
                        "is_emergency": true
                    },
                    "emergency_fallback": true,
                    "warning": "Emergency Super Admin session active (Maintenance Mode)."
                })),
            )
                .into_response();
        }
    }

    // 2. Extract client IP and User Agent
    let ip = headers
        .get("x-forwarded-for")
        .or_else(|| headers.get("x-real-ip"))
        .and_then(|v| v.to_str().ok())
        .unwrap_or("127.0.0.1")
        .to_string();
    let user_agent = headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("BOS-UI")
        .to_string();

    let mut ctx = extract_context(&headers).unwrap_or(SessionContext {
        tenant_id: Uuid::nil(),
        company_id: Uuid::nil(),
        user_id: Uuid::nil(),
        roles: vec!["SUPER".to_string(), "Admin".to_string()],
    });

    if ctx.tenant_id.is_nil() {
        let pool = engine.pool().await;
        let default_tenant_id = sqlx::query_scalar::<_, Uuid>(
            "SELECT tenant_id FROM \"_bos_tenants\" WHERE is_active = true ORDER BY created_at LIMIT 1",
        )
        .fetch_optional(&pool)
        .await
        .unwrap_or(None)
        .unwrap_or_else(Uuid::nil);

        ctx.tenant_id = default_tenant_id;
    }

    let params = serde_json::json!({
        "username": username,
        "password": password,
        "ipAddress": ip,
        "userAgent": user_agent,
    });

    // 3. Thin RPC Gateway: Delegate directly to .bospkg codeunit AuthManager.Authenticate
    match engine
        .execute_procedure_with(
            "AuthManager",
            "Authenticate",
            params,
            &ctx,
            std::collections::HashMap::new(),
        )
        .await
    {
        Ok(exec_result) => {
            let data = exec_result.mutated_data;
            let authenticated = data
                .get("authenticated")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            if authenticated {
                let token = data
                    .get("token")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();
                let uname = data
                    .get("username")
                    .and_then(|v| v.as_str())
                    .unwrap_or(&username)
                    .to_string();
                let full_name = data
                    .get("fullName")
                    .and_then(|v| v.as_str())
                    .unwrap_or(&uname)
                    .to_string();
                let email = data
                    .get("email")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let roles = data
                    .get("roles")
                    .cloned()
                    .unwrap_or_else(|| serde_json::json!(["User"]));

                (
                    StatusCode::OK,
                    Json(serde_json::json!({
                        "token": token,
                        "user": {
                            "id": uname,
                            "username": uname,
                            "name": full_name,
                            "email": email,
                            "roles": roles
                        },
                        "expiresAt": data.get("expiresAt")
                    })),
                )
                    .into_response()
            } else {
                let reason = data
                    .get("reason")
                    .and_then(|v| v.as_str())
                    .unwrap_or("INVALID_CREDENTIALS");
                let message = match reason {
                    "ACCOUNT_LOCKED" => "Account is locked due to too many failed attempts.",
                    "ACCOUNT_INACTIVE" => "Account is inactive. Please contact your administrator.",
                    _ => "Invalid User ID or Password.",
                };

                (
                    StatusCode::UNAUTHORIZED,
                    Json(serde_json::json!({
                        "error": message,
                        "reason": reason,
                        "attemptsRemaining": data.get("attemptsRemaining")
                    })),
                )
                    .into_response()
            }
        }
        Err(err) => {
            tracing::error!("AuthManager.Authenticate execution error: {}", err);
            let auto_fallback = node_config
                .as_ref()
                .map(|c| c.security.auto_fallback_on_auth_failure)
                .unwrap_or(false);

            // Only when the auth module is broken AND there is no identity data (docs/13 4.3).
            if auto_fallback
                && username == "admin"
                && crate::auth_guard::identity_state(&engine).await.needs_bootstrap()
            {
                tracing::warn!("Auto fallback active: issuing emergency admin session");
                let token = format!("bos_sess_fallback_{}", Uuid::new_v4().simple());
                return (
                    StatusCode::OK,
                    Json(serde_json::json!({
                        "token": token,
                        "user": {
                            "id": "admin",
                            "username": "admin",
                            "name": "System Super Administrator",
                            "email": "admin@system.local",
                            "roles": ["SUPER", "Admin", "Global Admin"]
                        },
                        "emergency_fallback": true,
                        "warning": format!("Authenticated via emergency fallback: {}", err)
                    })),
                )
                    .into_response();
            }

            (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({
                    "error": "Invalid User ID or Password.",
                    "detail": err.to_string()
                })),
            )
                .into_response()
        }
    }
}

async fn setup_provision_handler(
    Extension(engine): Extension<Arc<BosEngine>>,
    Json(body): Json<FirstRunProvisionRequest>,
) -> Response {
    match engine.first_run_provision(body).await {
        Ok(session) => (StatusCode::CREATED, Json(session)).into_response(),
        Err(err) => AppError(StatusCode::BAD_REQUEST, err.to_string()).into_response(),
    }
}

async fn detect_databases() -> Response {
    let databases = DatabaseDetector::scan_all();
    (StatusCode::OK, Json(databases)).into_response()
}

#[derive(Debug, Deserialize, Default)]
pub struct ListDatabasesQuery {
    pub url: Option<String>,
}

async fn list_databases_handler(
    Extension(engine): Extension<Arc<BosEngine>>,
    Query(query): Query<ListDatabasesQuery>,
) -> Response {
    if let Some(target_url) = query.url.filter(|u| !u.trim().is_empty()) {
        match engine.list_databases_for_url(&target_url).await {
            Ok(dbs) => (StatusCode::OK, Json(dbs)).into_response(),
            Err(e) => AppError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
        }
    } else {
        match engine.list_databases().await {
            Ok(dbs) => (StatusCode::OK, Json(dbs)).into_response(),
            Err(e) => AppError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
        }
    }
}

async fn create_database_handler(
    Extension(engine): Extension<Arc<BosEngine>>,
    Json(req): Json<CreateDatabaseRequest>,
) -> Response {
    match engine.create_database(&req).await {
        Ok(res) => (StatusCode::CREATED, Json(res)).into_response(),
        Err(e) => AppError(StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

#[derive(Debug, Deserialize)]
pub struct ListTenantsQuery {
    pub database: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ListCompaniesQuery {
    pub tenant_id: Option<Uuid>,
    pub database: Option<String>,
}

async fn list_tenants_handler(
    Extension(engine): Extension<Arc<BosEngine>>,
    Query(query): Query<ListTenantsQuery>,
) -> Response {
    match engine.list_tenants_for_db(query.database.as_deref()).await {
        Ok(tenants) => (StatusCode::OK, Json(tenants)).into_response(),
        Err(e) => AppError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn list_database_tenants_handler(
    Extension(engine): Extension<Arc<BosEngine>>,
    Path(name): Path<String>,
) -> Response {
    match engine.list_tenants_for_db(Some(&name)).await {
        Ok(tenants) => (StatusCode::OK, Json(tenants)).into_response(),
        Err(e) => AppError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn create_tenant_handler(
    Extension(engine): Extension<Arc<BosEngine>>,
    Json(req): Json<CreateTenantRequest>,
) -> Response {
    match engine.create_tenant(req).await {
        Ok(t) => (StatusCode::CREATED, Json(t)).into_response(),
        Err(e) => AppError(StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

async fn list_companies_handler(
    Extension(engine): Extension<Arc<BosEngine>>,
    Query(query): Query<ListCompaniesQuery>,
) -> Response {
    match engine
        .list_companies_for_db(query.database.as_deref(), query.tenant_id)
        .await
    {
        Ok(companies) => (StatusCode::OK, Json(companies)).into_response(),
        Err(e) => AppError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn list_database_companies_handler(
    Extension(engine): Extension<Arc<BosEngine>>,
    Path(name): Path<String>,
    Query(query): Query<ListCompaniesQuery>,
) -> Response {
    match engine
        .list_companies_for_db(Some(&name), query.tenant_id)
        .await
    {
        Ok(companies) => (StatusCode::OK, Json(companies)).into_response(),
        Err(e) => AppError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn create_company_handler(
    Extension(engine): Extension<Arc<BosEngine>>,
    Json(req): Json<CreateCompanyRequest>,
) -> Response {
    match engine.create_company(req).await {
        Ok(c) => (StatusCode::CREATED, Json(c)).into_response(),
        Err(e) => AppError(StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

async fn get_current_database_handler(Extension(engine): Extension<Arc<BosEngine>>) -> Response {
    match engine.get_current_database_details().await {
        Ok(details) => (StatusCode::OK, Json(details)).into_response(),
        Err(e) => AppError(StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

async fn update_current_database_handler(
    Extension(engine): Extension<Arc<BosEngine>>,
    Json(req): Json<UpdateDatabaseRequest>,
) -> Response {
    match engine.update_current_database(req).await {
        Ok(res) => (StatusCode::OK, Json(res)).into_response(),
        Err(e) => AppError(StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

async fn delete_current_database_handler(
    Extension(engine): Extension<Arc<BosEngine>>,
    Query(query): Query<DropDatabaseQuery>,
) -> Response {
    let force = query.force.unwrap_or(true);
    match engine.delete_current_database(force).await {
        Ok(res) => (StatusCode::OK, Json(res)).into_response(),
        Err(e) => AppError(StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

async fn manage_current_database_handler(
    Extension(engine): Extension<Arc<BosEngine>>,
    Json(req): Json<ManageDatabaseRequest>,
) -> Response {
    match engine.manage_current_database(req).await {
        Ok(res) => (StatusCode::OK, Json(res)).into_response(),
        Err(e) => AppError(StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

async fn get_database_handler(
    Extension(engine): Extension<Arc<BosEngine>>,
    Path(name): Path<String>,
) -> Response {
    match engine.get_database_details(&name).await {
        Ok(details) => (StatusCode::OK, Json(details)).into_response(),
        Err(e) => AppError(StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

async fn update_database_handler(
    headers: HeaderMap,
    Extension(engine): Extension<Arc<BosEngine>>,
    Path(name): Path<String>,
    Json(req): Json<UpdateDatabaseRequest>,
) -> Response {
    if let Ok(ctx) = extract_context(&headers) {
        if !is_superuser_roles(&ctx.roles) {
            return AppError(
                StatusCode::FORBIDDEN,
                "Forbidden: Superuser privileges required to alter or update database configuration.".to_string(),
            )
            .into_response();
        }
    }
    match engine.update_database(&name, req).await {
        Ok(res) => (StatusCode::OK, Json(res)).into_response(),
        Err(e) => AppError(StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

async fn drop_database_handler(
    headers: HeaderMap,
    Extension(engine): Extension<Arc<BosEngine>>,
    Path(name): Path<String>,
    Query(query): Query<DropDatabaseQuery>,
) -> Response {
    if let Ok(ctx) = extract_context(&headers) {
        if !is_superuser_roles(&ctx.roles) {
            return AppError(
                StatusCode::FORBIDDEN,
                "Forbidden: Superuser privileges required to drop or delete databases.".to_string(),
            )
            .into_response();
        }
    }
    let force = query.force.unwrap_or(true);
    match engine.drop_database(&name, force).await {
        Ok(_) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "success": true,
                "name": name,
                "active_database": engine.active_db_name().await,
                "message": format!("Database '{}' dropped successfully", name)
            })),
        )
            .into_response(),
        Err(e) => AppError(StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

async fn manage_database_handler(
    headers: HeaderMap,
    Extension(engine): Extension<Arc<BosEngine>>,
    Path(name): Path<String>,
    Json(req): Json<ManageDatabaseRequest>,
) -> Response {
    if let Ok(ctx) = extract_context(&headers) {
        if !is_superuser_roles(&ctx.roles) {
            return AppError(
                StatusCode::FORBIDDEN,
                "Forbidden: Superuser privileges required to manage database maintenance routines."
                    .to_string(),
            )
            .into_response();
        }
    }
    match engine.manage_database(&name, req).await {
        Ok(res) => (StatusCode::OK, Json(res)).into_response(),
        Err(e) => AppError(StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

async fn test_connection_handler(
    Extension(engine): Extension<Arc<BosEngine>>,
    Json(req): Json<TestConnectionRequest>,
) -> Response {
    let result = engine.test_connection(&req.url).await;
    (StatusCode::OK, Json(result)).into_response()
}

async fn switch_database_handler(
    headers: HeaderMap,
    Extension(engine): Extension<Arc<BosEngine>>,
    Json(req): Json<SwitchDatabaseRequest>,
) -> Response {
    let authenticated_ctx: Option<SessionContext> = extract_context(&headers).ok();

    let target = req
        .name
        .as_deref()
        .or(req.url.as_deref())
        .unwrap_or("postgres");

    match engine.switch_database(target).await {
        Ok(new_db) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "success": true,
                "active_database": new_db,
                "authenticated": authenticated_ctx.is_some(),
                "user_id": authenticated_ctx.as_ref().map(|c| c.user_id),
                "roles": authenticated_ctx.as_ref().map(|c| c.roles.clone()),
                "message": format!("Switched active BOS database to '{}'", new_db)
            })),
        )
            .into_response(),
        Err(e) => AppError(StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

#[derive(Debug, Deserialize, Default)]
pub struct EntityQuery {
    pub fields: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub page: Option<i64>,
    pub sort: Option<String>,
    pub filter: Option<String>,
}

async fn query_entities(
    Extension(engine): Extension<Arc<BosEngine>>,
    Path(table): Path<String>,
    Query(query): Query<EntityQuery>,
    headers: HeaderMap,
) -> Response {
    let ctx = match extract_context(&headers) {
        Ok(ctx) => ctx,
        Err(response) => return response,
    };
    let fields_vec: Option<Vec<String>> = query.fields.as_ref().map(|s| {
        s.split(',')
            .map(|f| f.trim().to_string())
            .filter(|f| !f.is_empty())
            .collect()
    });

    let params = crate::EntityQueryParams {
        fields: fields_vec,
        limit: query.limit,
        offset: query.offset,
        page: query.page,
        sort: query.sort,
        filter: query.filter,
    };

    match engine.query_records_paginated(&table, &params, &ctx).await {
        Ok(result) => {
            let mut response = (StatusCode::OK, Json(result.clone())).into_response();
            let headers_mut = response.headers_mut();
            if let Ok(v) = HeaderValue::from_str(&result.total.to_string()) {
                headers_mut.insert("x-total-count", v);
            }
            if let Ok(v) = HeaderValue::from_str(&result.page.to_string()) {
                headers_mut.insert("x-page", v);
            }
            if let Ok(v) = HeaderValue::from_str(&result.limit.to_string()) {
                headers_mut.insert("x-limit", v);
            }
            if let Ok(v) = HeaderValue::from_str(&result.has_more.to_string()) {
                headers_mut.insert("x-has-more", v);
            }
            response
        }
        Err(e) => AppError(StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

async fn get_entity(
    Extension(engine): Extension<Arc<BosEngine>>,
    Path((table, id)): Path<(String, String)>,
    Query(query): Query<EntityQuery>,
    headers: HeaderMap,
) -> Response {
    let ctx = match extract_context(&headers) {
        Ok(ctx) => ctx,
        Err(response) => return response,
    };
    let id_uuid = match Uuid::parse_str(&id) {
        Ok(u) => u,
        Err(_) => {
            return AppError(StatusCode::BAD_REQUEST, "Invalid UUID format".to_string())
                .into_response();
        }
    };
    let fields_vec: Option<Vec<String>> = query.fields.map(|s| {
        s.split(',')
            .map(|f| f.trim().to_string())
            .filter(|f| !f.is_empty())
            .collect()
    });

    match engine
        .get_record(&table, id_uuid, fields_vec.as_deref(), &ctx)
        .await
    {
        Ok(Some(item)) => (StatusCode::OK, Json(item)).into_response(),
        Ok(None) => AppError(
            StatusCode::NOT_FOUND,
            format!("Record not found in {}", table),
        )
        .into_response(),
        Err(e) => AppError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn get_entity_schema_handler(
    Extension(engine): Extension<Arc<BosEngine>>,
    Path(table): Path<String>,
    headers: HeaderMap,
) -> Response {
    let ctx = match extract_context(&headers) {
        Ok(ctx) => ctx,
        Err(response) => return response,
    };
    match engine.get_entity_schema(&table, &ctx).await {
        Ok(schema) => (StatusCode::OK, Json(schema)).into_response(),
        Err(e) => AppError(StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

async fn create_entity(
    Extension(engine): Extension<Arc<BosEngine>>,
    Path(table): Path<String>,
    headers: HeaderMap,
    Json(payload): Json<serde_json::Value>,
) -> Response {
    let ctx = match extract_context(&headers) {
        Ok(ctx) => ctx,
        Err(response) => return response,
    };

    match engine
        .insert_record_with_messages(&table, payload, &ctx)
        .await
    {
        Ok((id, messages)) => {
            let mut notifs = vec![crate::Notification::success(format!(
                "Record created in {}",
                table
            ))];
            for m in &messages {
                if let Some(s) = m.as_str() {
                    notifs.push(crate::Notification::info(s));
                } else {
                    notifs.push(crate::Notification::info(m.to_string()));
                }
            }
            json_response_with_notifications(
                StatusCode::CREATED,
                serde_json::json!({
                    "success": true,
                    "_id": id,
                    "message": format!("Record created in {}", table),
                    "messages": messages
                }),
                notifs,
            )
        }
        Err(e) => AppError(StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

async fn create_entities_batch(
    Extension(engine): Extension<Arc<BosEngine>>,
    Path(table): Path<String>,
    headers: HeaderMap,
    Json(records): Json<Vec<serde_json::Value>>,
) -> Response {
    let ctx = match extract_context(&headers) {
        Ok(ctx) => ctx,
        Err(response) => return response,
    };

    match engine.insert_records_batch(&table, records, &ctx).await {
        Ok(ids) => (
            StatusCode::CREATED,
            Json(serde_json::json!({
                "success": true,
                "count": ids.len(),
                "ids": ids,
                "message": format!("{} records created in {}", ids.len(), table)
            })),
        )
            .into_response(),
        Err(e) => AppError(StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

#[derive(Debug, Deserialize)]
pub struct TransitionRequest {
    pub from_state: String,
    pub to_state: String,
    pub action: String,
    pub status_column: Option<String>,
    pub comments: Option<String>,
    pub params: Option<serde_json::Value>,
}

async fn execute_transition(
    Extension(engine): Extension<Arc<BosEngine>>,
    Path((table, id)): Path<(String, String)>,
    headers: HeaderMap,
    Json(req): Json<TransitionRequest>,
) -> Response {
    let ctx = match extract_context(&headers) {
        Ok(ctx) => ctx,
        Err(response) => return response,
    };
    let record_id = match Uuid::parse_str(&id) {
        Ok(u) => u,
        Err(_) => {
            return AppError(StatusCode::BAD_REQUEST, "Invalid UUID format".to_string())
                .into_response();
        }
    };
    let status_col = req.status_column.as_deref().unwrap_or("status");

    match engine
        .transition_state(
            &table,
            record_id,
            status_col,
            &req.from_state,
            &req.to_state,
            &req.action,
            req.comments.as_deref(),
            req.params,
            &ctx,
        )
        .await
    {
        Ok(_) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "success": true,
                "message": format!("Transitioned {} from '{}' to '{}'", record_id, req.from_state, req.to_state),
                "messages": [
                    {
                        "id": format!("msg_{}", &Uuid::new_v4().to_string().replace("-", "")[..8]),
                        "type": "success",
                        "title": "State Transitioned",
                        "message": format!("Entity {} transitioned from '{}' to '{}'", table, req.from_state, req.to_state),
                        "duration": 4000
                    }
                ]
            })),
        )
            .into_response(),
        Err(e) => AppError(StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

async fn get_state_history(
    Extension(engine): Extension<Arc<BosEngine>>,
    Path((table, id)): Path<(String, String)>,
) -> Response {
    let record_id = match Uuid::parse_str(&id) {
        Ok(u) => u,
        Err(_) => {
            return AppError(StatusCode::BAD_REQUEST, "Invalid UUID format".to_string())
                .into_response();
        }
    };
    let safe_table = match crate::sanitize_identifier(&crate::to_snake_case(&table)) {
        Ok(s) => s,
        Err(e) => return AppError(StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    };
    let history_table = format!("{}_state_history", safe_table);

    let query = format!(
        "SELECT to_jsonb(h.*) FROM \"{}\" h WHERE \"entity_id\" = $1 ORDER BY \"timestamp\" ASC;",
        history_table
    );

    let pool = engine.pool().await;
    match sqlx::query_scalar::<_, serde_json::Value>(&query)
        .bind(record_id)
        .fetch_all(&pool)
        .await
    {
        Ok(rows) => (StatusCode::OK, Json(rows)).into_response(),
        Err(e) => AppError(StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

#[derive(Debug, Deserialize)]
pub struct PrivilegedOverrideRequest {
    pub column: String,
    pub new_value: String,
    pub reason: String,
    pub ticket: String,
    pub authorizer: String,
}

async fn privileged_override(
    Extension(engine): Extension<Arc<BosEngine>>,
    Path((table, id)): Path<(String, String)>,
    headers: HeaderMap,
    Json(req): Json<PrivilegedOverrideRequest>,
) -> Response {
    let ctx = match extract_context(&headers) {
        Ok(ctx) => ctx,
        Err(response) => return response,
    };
    let record_id = match Uuid::parse_str(&id) {
        Ok(u) => u,
        Err(_) => {
            return AppError(StatusCode::BAD_REQUEST, "Invalid UUID format".to_string())
                .into_response();
        }
    };

    match engine
        .update_privileged(
            &table,
            record_id,
            &req.column,
            &req.new_value,
            &req.reason,
            &req.ticket,
            &req.authorizer,
            &ctx,
        )
        .await
    {
        Ok(_) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "success": true,
                "message": "Privileged audit override executed and recorded into _system_privileged_audit_log"
            })),
        )
            .into_response(),
        Err(e) => AppError(StatusCode::FORBIDDEN, e.to_string()).into_response(),
    }
}

// --- Meta-Objects & System Provisioning Handlers ---

#[derive(Debug, Deserialize, Default)]
pub struct ProvisionSystemRequest {
    pub storage_strategy: Option<String>,
}

async fn provision_system_handler(
    Extension(engine): Extension<Arc<BosEngine>>,
    body: Option<Json<ProvisionSystemRequest>>,
) -> Response {
    let strat = body.and_then(|b| b.storage_strategy.clone());
    match engine.provision_system(strat.as_deref()).await {
        Ok(status) => (StatusCode::OK, Json(status)).into_response(),
        Err(e) => AppError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn list_storage_strategies_handler() -> Response {
    (
        StatusCode::OK,
        Json(serde_json::json!([
            {
                "id": "shared_rls",
                "name": "Shared Table + Row Level Security (RLS)",
                "description": "High-density multi-tenant SaaS layout. All tenants and companies share physical tables with database-enforced row isolation via PostgreSQL RLS policies.",
                "best_for": "SaaS startups, high tenant density, cost-optimized cloud environments",
                "isolation_level": "Logical (Row-level Security)",
                "recommended": true
            },
            {
                "id": "partitioned",
                "name": "Declarative Table Partitioning (PostgreSQL Native)",
                "description": "PostgreSQL declarative list partitions per tenant/company. Query planner prunes irrelevant partitions; instant DROP TABLE on offboarding.",
                "best_for": "Medium-to-large multi-tenant ERPs with high query throughput",
                "isolation_level": "Physical (Separate Disk Partitions per Tenant)",
                "recommended": false
            },
            {
                "id": "schema_per_company",
                "name": "Dedicated Schema-per-Company (Namespace Isolation)",
                "description": "Company-specific tables live in isolated PostgreSQL schemas (comp_<code_slug>), while global references remain in public. Independent backup/restore.",
                "best_for": "Multi-subsidiary enterprise ERPs with independent reporting and schema dumps",
                "isolation_level": "Logical / Namespace (PostgreSQL Schema Separation)",
                "recommended": false
            },
            {
                "id": "table_per_company",
                "name": "Physical Table-per-Company (Dynamics NAV/BC Pattern)",
                "description": "Physical company-prefixed tables (<company>$<table>) for transactional data. Eliminates cross-company lock contention entirely.",
                "best_for": "Statutory corporate conglomerates requiring physical separation of company ledgers",
                "isolation_level": "Physical (Separate Tables per Company)",
                "recommended": false
            }
        ])),
    )
        .into_response()
}

async fn get_system_status_handler(Extension(engine): Extension<Arc<BosEngine>>) -> Response {
    match engine.get_system_status().await {
        Ok(status) => (StatusCode::OK, Json(status)).into_response(),
        Err(e) => AppError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct ListMetaObjectsQuery {
    #[serde(rename = "type")]
    object_type: Option<String>,
}

async fn list_meta_objects_handler(
    Extension(engine): Extension<Arc<BosEngine>>,
    Query(query): Query<ListMetaObjectsQuery>,
) -> Response {
    match engine.list_meta_objects(query.object_type.as_deref()).await {
        Ok(objs) => (StatusCode::OK, Json(objs)).into_response(),
        Err(e) => AppError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn register_meta_object_handler(
    Extension(engine): Extension<Arc<BosEngine>>,
    Json(req): Json<RegisterMetaObjectRequest>,
) -> Response {
    match engine.register_meta_object(req).await {
        Ok(obj) => (StatusCode::CREATED, Json(obj)).into_response(),
        Err(e) => AppError(schema_error_status(&e), e.to_string()).into_response(),
    }
}

async fn get_meta_object_handler(
    Extension(engine): Extension<Arc<BosEngine>>,
    Path(name): Path<String>,
) -> Response {
    match engine.get_meta_object(&name).await {
        Ok(Some(obj)) => (StatusCode::OK, Json(obj)).into_response(),
        Ok(None) => AppError(
            StatusCode::NOT_FOUND,
            format!("Meta-object '{}' not found", name),
        )
        .into_response(),
        Err(e) => AppError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn get_resolved_page_handler(
    Extension(engine): Extension<Arc<BosEngine>>,
    Path(name): Path<String>,
) -> Response {
    match engine.resolve_page(&name).await {
        Ok(Some(resolved)) => (StatusCode::OK, Json(resolved)).into_response(),
        Ok(None) => AppError(
            StatusCode::NOT_FOUND,
            format!("Page '{}' not found", name),
        )
        .into_response(),
        Err(e) => AppError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn get_ambient_dts_handler(Extension(engine): Extension<Arc<BosEngine>>) -> Response {
    let dts = engine.generate_ambient_dts().await;
    (
        StatusCode::OK,
        [(
            axum::http::header::CONTENT_TYPE,
            "application/typescript; charset=utf-8",
        )],
        dts,
    )
        .into_response()
}

async fn list_packages_handler(Extension(engine): Extension<Arc<BosEngine>>) -> Response {
    match engine.list_packages().await {
        Ok(packages) => (StatusCode::OK, Json(packages)).into_response(),
        Err(e) => AppError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[derive(Debug, Deserialize)]
pub struct ExecuteCodeunitRequest {
    pub method: String,
    pub params: Option<serde_json::Value>,
    #[serde(default)]
    pub ui_answers: std::collections::HashMap<String, bool>,
}

#[derive(Debug, Deserialize, Default)]
pub struct InstallPackageQuery {
    pub db: Option<String>,
    pub database: Option<String>,
    /// Permit schema changes that delete or truncate data.
    #[serde(default)]
    pub allow_destructive: bool,
}

async fn install_package_handler(
    headers: HeaderMap,
    Extension(engine): Extension<Arc<BosEngine>>,
    Query(query): Query<InstallPackageQuery>,
    Json(bundle): Json<bosc_ir::PackageBundleIR>,
) -> Response {
    let target_db = query.db.or(query.database).or_else(|| {
        headers
            .get("x-bos-database")
            .and_then(|v| v.to_str().ok().map(|s| s.to_string()))
    });

    let outcome = if let Some(db_name) = target_db {
        install_to_selected_db(&engine, &db_name, bundle, query.allow_destructive).await
    } else {
        engine
            .install_package(bundle, query.allow_destructive)
            .await
    };
    match outcome {
        Ok(res) => (StatusCode::CREATED, Json(res)).into_response(),
        Err(e) => AppError(schema_error_status(&e), e.to_string()).into_response(),
    }
}

/// 409 when a deployment was refused because it would lose data; 400 for every other failure.
fn schema_error_status(err: &RuntimeError) -> StatusCode {
    match err {
        RuntimeError::SchemaSyncBlocked(_) => StatusCode::CONFLICT,
        _ => StatusCode::BAD_REQUEST,
    }
}

async fn install_package_to_database_handler(
    Extension(engine): Extension<Arc<BosEngine>>,
    Path(name): Path<String>,
    Query(query): Query<InstallPackageQuery>,
    Json(bundle): Json<bosc_ir::PackageBundleIR>,
) -> Response {
    match install_to_selected_db(&engine, &name, bundle, query.allow_destructive).await {
        Ok(res) => (StatusCode::CREATED, Json(res)).into_response(),
        Err(e) => AppError(schema_error_status(&e), e.to_string()).into_response(),
    }
}

async fn install_to_selected_db(
    engine: &BosEngine,
    db_name: &str,
    bundle: bosc_ir::PackageBundleIR,
    allow_destructive: bool,
) -> Result<crate::package_manager::PackageInstallResult, RuntimeError> {
    let current_url = engine.active_url().await;
    let target_url = if db_name.starts_with("postgres://") || db_name.starts_with("postgresql://") {
        db_name.to_string()
    } else {
        crate::db_manager::DatabaseManager::validate_db_name(db_name)?;
        crate::db_manager::DatabaseManager::replace_database_name(&current_url, db_name)
    };

    let target_pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&target_url)
        .await?;

    let sys_status = crate::meta_manager::MetaManager::get_system_status(&target_pool).await?;
    if !sys_status.is_provisioned {
        let _ =
            crate::meta_manager::MetaManager::provision_system_kernel(&target_pool, None).await?;
    }

    let res = crate::package_manager::PackageManager::install_package(
        &target_pool,
        bundle,
        None,
        allow_destructive,
    )
    .await?;
    let _ = target_pool.close().await;
    Ok(res)
}

async fn export_package_handler(
    Extension(engine): Extension<Arc<BosEngine>>,
    Path(id): Path<String>,
) -> Response {
    match engine.export_package(&id).await {
        Ok(bundle) => (StatusCode::OK, Json(bundle)).into_response(),
        Err(e) => AppError(StatusCode::NOT_FOUND, e.to_string()).into_response(),
    }
}

async fn register_table_extension_handler(
    Extension(engine): Extension<Arc<BosEngine>>,
    Json(req): Json<RegisterTableExtensionRequest>,
) -> Response {
    match engine.register_table_extension(req).await {
        Ok(ext) => (StatusCode::CREATED, Json(ext)).into_response(),
        Err(e) => AppError(StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

async fn register_codeunit_handler(
    Extension(engine): Extension<Arc<BosEngine>>,
    Json(cu): Json<bosc_ir::CodeunitIR>,
) -> Response {
    let cu_name = cu.name.clone();
    match engine.register_codeunit(cu).await {
        Ok(id) => (
            StatusCode::CREATED,
            Json(serde_json::json!({ "success": true, "id": id, "name": cu_name })),
        )
            .into_response(),
        Err(e) => AppError(StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

async fn execute_codeunit_handler(
    headers: HeaderMap,
    Extension(engine): Extension<Arc<BosEngine>>,
    Path(name): Path<String>,
    Json(req): Json<ExecuteCodeunitRequest>,
) -> Response {
    let ctx = match extract_context(&headers) {
        Ok(ctx) => ctx,
        Err(response) => return response,
    };
    let params = req.params.unwrap_or(serde_json::json!({}));
    match engine
        .execute_procedure_with(&name, &req.method, params, &ctx, req.ui_answers)
        .await
    {
        Ok(res) => (StatusCode::OK, Json(res)).into_response(),
        Err(e) => AppError(StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

#[derive(Debug, Deserialize)]
pub struct RunTestsRequest {
    pub filter: Option<String>,
}

async fn run_tests_handler(
    headers: HeaderMap,
    Extension(engine): Extension<Arc<BosEngine>>,
    req_opt: Option<Json<RunTestsRequest>>,
) -> Response {
    let ctx = match extract_context(&headers) {
        Ok(ctx) => ctx,
        Err(response) => return response,
    };
    let filter = req_opt.and_then(|r| r.filter.clone());
    match engine.run_tests(filter.as_deref(), &ctx).await {
        Ok(results) => (StatusCode::OK, Json(results)).into_response(),
        Err(e) => AppError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn apply_table_script_handler(
    Extension(engine): Extension<Arc<BosEngine>>,
    Path(table): Path<String>,
    Json(req): Json<ApplyTableScriptRequest>,
) -> Response {
    match engine.apply_table_script(&table, req).await {
        Ok(res) => (StatusCode::OK, Json(res)).into_response(),
        Err(e) => AppError(schema_error_status(&e), e.to_string()).into_response(),
    }
}

async fn compile_script_handler(
    Extension(engine): Extension<Arc<BosEngine>>,
    Json(mut req): Json<CompileScriptRequest>,
) -> Response {
    req.apply = false;
    match engine.compile_script(req).await {
        Ok(res) => (StatusCode::OK, Json(res)).into_response(),
        Err(e) => AppError(StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

async fn apply_script_handler(
    Extension(engine): Extension<Arc<BosEngine>>,
    Json(mut req): Json<CompileScriptRequest>,
) -> Response {
    req.apply = true;
    match engine.compile_script(req).await {
        Ok(res) => (StatusCode::OK, Json(res)).into_response(),
        Err(e) => AppError(schema_error_status(&e), e.to_string()).into_response(),
    }
}

async fn update_entity(
    Extension(engine): Extension<Arc<BosEngine>>,
    Path((table, id)): Path<(String, String)>,
    headers: HeaderMap,
    Json(payload): Json<serde_json::Value>,
) -> Response {
    let ctx = match extract_context(&headers) {
        Ok(ctx) => ctx,
        Err(response) => return response,
    };
    let record_id = match Uuid::parse_str(&id) {
        Ok(u) => u,
        Err(_) => {
            return AppError(StatusCode::BAD_REQUEST, "Invalid UUID format".to_string())
                .into_response();
        }
    };

    match engine.update_record(&table, record_id, payload, &ctx).await {
        Ok(mut updated_record) => {
            let messages = if let Some(msgs) = updated_record
                .as_object_mut()
                .and_then(|o| o.remove("_system_messages"))
            {
                msgs.as_array().cloned().unwrap_or_default()
            } else {
                Vec::new()
            };
            let mut notifs = vec![crate::Notification::success(format!(
                "Record updated in {}",
                table
            ))];
            for m in &messages {
                if let Some(m_str) = m.as_str() {
                    notifs.push(crate::Notification::info(m_str));
                }
            }
            json_response_with_notifications(
                StatusCode::OK,
                serde_json::json!({
                    "success": true,
                    "_id": record_id,
                    "data": updated_record,
                    "message": format!("Record updated in {}", table),
                    "messages": messages
                }),
                notifs,
            )
        }
        Err(RuntimeError::ConcurrencyConflict(msg)) => (
            StatusCode::CONFLICT,
            Json(serde_json::json!({
                "error": msg.clone(),
                "conflict": true,
                "message": msg
            })),
        )
            .into_response(),
        Err(RuntimeError::VersionRequired(msg)) => (
            StatusCode::PRECONDITION_REQUIRED,
            Json(serde_json::json!({
                "error": msg.clone(),
                "version_required": true,
                "message": msg
            })),
        )
            .into_response(),
        Err(RuntimeError::ImmutableLedger(msg)) => (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({
                "error": msg.clone(),
                "immutable": true,
                "message": msg
            })),
        )
            .into_response(),
        Err(e) => AppError(StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

async fn delete_entity(
    Extension(engine): Extension<Arc<BosEngine>>,
    Path((table, id)): Path<(String, String)>,
    headers: HeaderMap,
) -> Response {
    let ctx = match extract_context(&headers) {
        Ok(ctx) => ctx,
        Err(response) => return response,
    };
    let record_id = match Uuid::parse_str(&id) {
        Ok(u) => u,
        Err(_) => {
            return AppError(StatusCode::BAD_REQUEST, "Invalid UUID format".to_string())
                .into_response();
        }
    };

    match engine.delete_record(&table, record_id, &ctx).await {
        Ok(deleted_rec) => {
            let notifs = vec![crate::Notification::info(format!(
                "Record deleted from {}",
                table
            ))];
            json_response_with_notifications(
                StatusCode::OK,
                serde_json::json!({
                    "success": true,
                    "_id": record_id,
                    "data": deleted_rec,
                    "message": format!("Record deleted from {}", table)
                }),
                notifs,
            )
        }
        Err(RuntimeError::NotFound(t, r)) => AppError(
            StatusCode::NOT_FOUND,
            format!("Record not found in {}: {}", t, r),
        )
        .into_response(),
        Err(RuntimeError::ImmutableLedger(msg)) => (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({
                "error": msg.clone(),
                "immutable": true,
                "message": msg
            })),
        )
            .into_response(),
        Err(e) => AppError(StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

async fn lock_entity(
    Extension(engine): Extension<Arc<BosEngine>>,
    Path((table, id)): Path<(String, String)>,
    headers: HeaderMap,
) -> Response {
    let ctx = match extract_context(&headers) {
        Ok(ctx) => ctx,
        Err(response) => return response,
    };
    let record_id = match Uuid::parse_str(&id) {
        Ok(u) => u,
        Err(_) => {
            return AppError(StatusCode::BAD_REQUEST, "Invalid UUID format".to_string())
                .into_response();
        }
    };

    match engine.lock_record(&table, record_id, &ctx).await {
        Ok(rec) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "success": true,
                "locked": true,
                "_id": record_id,
                "data": rec,
                "message": format!("Record locked in {}", table)
            })),
        )
            .into_response(),
        Err(RuntimeError::NotFound(t, r)) => AppError(
            StatusCode::NOT_FOUND,
            format!("Record not found in {}: {}", t, r),
        )
        .into_response(),
        Err(e) => AppError(StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

async fn lock_table_handler(
    Extension(engine): Extension<Arc<BosEngine>>,
    Path(table): Path<String>,
    headers: HeaderMap,
) -> Response {
    let ctx = match extract_context(&headers) {
        Ok(ctx) => ctx,
        Err(response) => return response,
    };
    match engine.lock_table(&table, &ctx).await {
        Ok(_) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "success": true,
                "locked": true,
                "table": table,
                "message": format!("Table {} locked in ROW EXCLUSIVE MODE", table)
            })),
        )
            .into_response(),
        Err(e) => AppError(StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

async fn execute_graphql_query(
    Extension(engine): Extension<Arc<BosEngine>>,
    headers: HeaderMap,
    Json(req): Json<QueryRequest>,
) -> Response {
    let ctx = match extract_context(&headers) {
        Ok(ctx) => ctx,
        Err(response) => return response,
    };
    match engine.execute_query(&req, &ctx).await {
        Ok(items) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "success": true,
                "entity": req.entity,
                "count": items.len(),
                "data": items
            })),
        )
            .into_response(),
        Err(e) => AppError(StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

#[derive(Debug, Deserialize)]
pub struct EnqueueJobRequest {
    pub job_type: String,
    pub codeunit_name: String,
    pub method_name: String,
    pub payload: Option<serde_json::Value>,
    pub chunk_size: Option<usize>,
}

async fn enqueue_job_handler(
    Extension(engine): Extension<Arc<BosEngine>>,
    Json(req): Json<EnqueueJobRequest>,
) -> Response {
    let payload = req.payload.unwrap_or(serde_json::json!({}));
    let chunk_size = req.chunk_size.unwrap_or(500);

    match engine
        .enqueue_job(
            &req.job_type,
            &req.codeunit_name,
            &req.method_name,
            payload,
            chunk_size,
        )
        .await
    {
        Ok(job_id) => (
            StatusCode::ACCEPTED,
            Json(serde_json::json!({
                "success": true,
                "job_id": job_id,
                "message": "Job enqueued successfully"
            })),
        )
            .into_response(),
        Err(e) => AppError(StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

async fn execute_next_job_handler(
    headers: HeaderMap,
    Extension(engine): Extension<Arc<BosEngine>>,
) -> Response {
    let ctx = match extract_context(&headers) {
        Ok(ctx) => ctx,
        Err(response) => return response,
    };
    match engine.execute_next_job(&ctx).await {
        Ok(Some(job_id)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "success": true,
                "processed_job_id": job_id,
                "message": "Processed next ready job from queue"
            })),
        )
            .into_response(),
        Ok(None) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "success": true,
                "processed_job_id": null,
                "message": "No ready jobs in queue"
            })),
        )
            .into_response(),
        Err(e) => AppError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

// ---------------------------------------------------------------------------
// Event Subscriber Handlers
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct RegisterSubscriberRequest {
    pub id: Option<Uuid>,
    pub publisher: String,
    pub event_name: String,
    pub subscriber_name: String,
    pub method_name: String,
    pub priority: Option<i32>,
    pub script: String,
    pub is_active: Option<bool>,
}

async fn list_subscribers_handler(Extension(engine): Extension<Arc<BosEngine>>) -> Response {
    let subs = engine.event_registry.list_all();
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "success": true,
            "count": subs.len(),
            "subscribers": subs
        })),
    )
        .into_response()
}

async fn register_subscriber_handler(
    Extension(engine): Extension<Arc<BosEngine>>,
    Json(req): Json<RegisterSubscriberRequest>,
) -> Response {
    let pool = engine.pool().await;
    let sub = crate::EventSubscriber {
        id: req.id.unwrap_or_else(Uuid::new_v4),
        publisher: req.publisher,
        event_name: req.event_name,
        subscriber_name: req.subscriber_name,
        method_name: req.method_name,
        priority: req.priority.unwrap_or(100),
        script: req.script,
        is_active: req.is_active.unwrap_or(true),
    };
    match engine.event_registry.persist_subscriber(&pool, sub).await {
        Ok(id) => (
            StatusCode::CREATED,
            Json(serde_json::json!({
                "success": true,
                "subscriber_id": id,
                "message": "Event subscriber registered and persisted successfully"
            })),
        )
            .into_response(),
        Err(e) => AppError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[derive(Debug, Deserialize)]
pub struct PublishEventRequest {
    pub publisher: String,
    pub event_name: String,
    pub data: serde_json::Value,
}

async fn publish_event_handler(
    headers: HeaderMap,
    Extension(engine): Extension<Arc<BosEngine>>,
    Json(req): Json<PublishEventRequest>,
) -> Response {
    let ctx = match extract_context(&headers) {
        Ok(ctx) => ctx,
        Err(response) => return response,
    };
    let scope = match engine.begin_scope(&ctx).await {
        Ok(scope) => scope,
        Err(e) => {
            return AppError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
        }
    };
    let outcome = engine
        .event_registry
        .dispatch_in(&req.publisher, &req.event_name, req.data, None, &scope)
        .await;

    match scope.db.finish(outcome).await {
        Ok((data, logs)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "success": true,
                "publisher": req.publisher,
                "event_name": req.event_name,
                "mutated_data": data,
                "execution_logs": logs
            })),
        )
            .into_response(),
        Err(e) => AppError(StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

// ---------------------------------------------------------------------------
// Transactional Event Outbox Handlers
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, Default)]
pub struct ListOutboxQuery {
    pub status: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct EnqueueOutboxRequest {
    pub topic: String,
    pub event_name: String,
    pub payload: serde_json::Value,
    pub aggregate_type: Option<String>,
    pub aggregate_id: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
pub struct ProcessOutboxRequest {
    pub batch_size: Option<i64>,
}

async fn list_outbox_handler(
    Extension(engine): Extension<Arc<BosEngine>>,
    Query(query): Query<ListOutboxQuery>,
) -> Response {
    let pool = engine.pool().await;
    match crate::event_engine::EventRegistry::list_outbox(
        &pool,
        query.status.as_deref(),
        query.limit.unwrap_or(50),
    )
    .await
    {
        Ok(events) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "success": true,
                "count": events.len(),
                "events": events
            })),
        )
            .into_response(),
        Err(e) => AppError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn enqueue_outbox_handler(
    headers: HeaderMap,
    Extension(engine): Extension<Arc<BosEngine>>,
    Json(req): Json<EnqueueOutboxRequest>,
) -> Response {
    let ctx = match extract_context(&headers) {
        Ok(ctx) => ctx,
        Err(response) => return response,
    };
    let pool = engine.pool().await;
    match crate::event_engine::EventRegistry::enqueue_outbox(
        &pool,
        ctx.tenant_id,
        Some(ctx.company_id),
        &req.topic,
        &req.event_name,
        &req.payload,
    )
    .await
    {
        Ok(event_id) => (
            StatusCode::CREATED,
            Json(serde_json::json!({
                "success": true,
                "id": event_id,
                "message": "Event queued into transactional outbox table (_bos_event_outbox)"
            })),
        )
            .into_response(),
        Err(e) => AppError(StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

async fn process_outbox_handler(
    Extension(engine): Extension<Arc<BosEngine>>,
    Json(req): Json<ProcessOutboxRequest>,
) -> Response {
    let pool = engine.pool().await;
    let batch_size = req.batch_size.unwrap_or(50);

    match engine
        .event_registry
        .process_outbox_batch(&pool, &engine.script_services(), batch_size)
        .await
    {
        Ok(summary) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "success": true,
                "summary": summary
            })),
        )
            .into_response(),
        Err(e) => AppError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

// ---------------------------------------------------------------------------
// WebAssembly (Wasm) Host Handlers
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct LoadWasmRequest {
    pub name: String,
    pub code: String,
}

async fn list_wasm_modules_handler(Extension(engine): Extension<Arc<BosEngine>>) -> Response {
    let modules = engine.wasm_engine.list_modules();
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "success": true,
            "count": modules.len(),
            "modules": modules
        })),
    )
        .into_response()
}

async fn load_wasm_module_handler(
    Extension(engine): Extension<Arc<BosEngine>>,
    Json(req): Json<LoadWasmRequest>,
) -> Response {
    match engine
        .wasm_engine
        .load_module(&req.name, req.code.as_bytes())
    {
        Ok(_) => (
            StatusCode::CREATED,
            Json(serde_json::json!({
                "success": true,
                "module": req.name,
                "message": "Wasm module loaded and compiled successfully"
            })),
        )
            .into_response(),
        Err(e) => AppError(StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

#[derive(Debug, Deserialize)]
pub struct InvokeWasmRequest {
    pub a: Option<f64>,
    pub b: Option<f64>,
    pub fuel_limit: Option<u64>,
}

async fn invoke_wasm_function_handler(
    Extension(engine): Extension<Arc<BosEngine>>,
    Path((module, func)): Path<(String, String)>,
    Json(req): Json<InvokeWasmRequest>,
) -> Response {
    let a = req.a.unwrap_or(0.0);
    let b = req.b.unwrap_or(0.0);
    match engine.wasm_engine.execute_f64_op(&module, &func, a, b) {
        Ok(res) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "success": true,
                "module": module,
                "function": func,
                "result": res
            })),
        )
            .into_response(),
        Err(e) => {
            // Fallback to integer operation if f64 fails
            match engine
                .wasm_engine
                .execute_i32_op(&module, &func, a as i32, b as i32)
            {
                Ok(i_res) => (
                    StatusCode::OK,
                    Json(serde_json::json!({
                        "success": true,
                        "module": module,
                        "function": func,
                        "result": i_res
                    })),
                )
                    .into_response(),
                Err(_) => AppError(StatusCode::BAD_REQUEST, e.to_string()).into_response(),
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Sidecar Microservice Host Handlers
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct RegisterSidecarRequest {
    pub name: String,
    pub endpoint_url: String,
    pub command: Option<String>,
    pub args: Option<Vec<String>>,
    pub description: Option<String>,
}

async fn list_sidecars_handler(Extension(engine): Extension<Arc<BosEngine>>) -> Response {
    let services = engine.sidecar_host.list_services();
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "success": true,
            "count": services.len(),
            "services": services
        })),
    )
        .into_response()
}

async fn register_sidecar_handler(
    Extension(engine): Extension<Arc<BosEngine>>,
    Json(req): Json<RegisterSidecarRequest>,
) -> Response {
    match engine.sidecar_host.register_service(
        &req.name,
        &req.endpoint_url,
        req.command,
        req.args.unwrap_or_default(),
        req.description,
    ) {
        Ok(()) => (
            StatusCode::CREATED,
            Json(serde_json::json!({
                "success": true,
                "name": req.name,
                "endpoint_url": req.endpoint_url,
                "message": "Sidecar registered"
            })),
        )
            .into_response(),
        Err(err) => AppError(StatusCode::BAD_REQUEST, err.to_string()).into_response(),
    }
}

#[derive(Debug, Deserialize)]
pub struct CallSidecarRequest {
    pub path: Option<String>,
    pub payload: Option<serde_json::Value>,
}

async fn call_sidecar_handler(
    Extension(engine): Extension<Arc<BosEngine>>,
    Path(name): Path<String>,
    Json(req): Json<CallSidecarRequest>,
) -> Response {
    let path = req.path.unwrap_or_default();
    let payload = req.payload.unwrap_or(serde_json::json!({}));
    match engine.sidecar_host.invoke(&name, &path, &payload).await {
        Ok(res) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "success": true,
                "sidecar": name,
                "response": res
            })),
        )
            .into_response(),
        Err(e) => AppError(StatusCode::BAD_GATEWAY, e.to_string()).into_response(),
    }
}

async fn check_sidecar_health_handler(
    Extension(engine): Extension<Arc<BosEngine>>,
    Path(name): Path<String>,
) -> Response {
    let healthy = engine.sidecar_host.check_health(&name).await;
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "success": true,
            "sidecar": name,
            "is_healthy": healthy,
            "status": if healthy { "Running" } else { "Unhealthy" }
        })),
    )
        .into_response()
}

#[derive(Debug, Deserialize)]
pub struct ViewQueryParams {
    pub filter: Option<String>,
    pub order_by: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

async fn get_view_handler(
    headers: HeaderMap,
    Extension(engine): Extension<Arc<BosEngine>>,
    Path(name): Path<String>,
    Query(params): Query<ViewQueryParams>,
) -> Response {
    let ctx = match extract_context(&headers) {
        Ok(ctx) => ctx,
        Err(response) => return response,
    };
    let filter_val = params.filter.and_then(|f| serde_json::from_str(&f).ok());
    let limit = params.limit.unwrap_or(100);
    let offset = params.offset.unwrap_or(0);

    match engine
        .execute_view(
            &name,
            filter_val,
            params.order_by.as_deref(),
            limit,
            offset,
            &ctx,
        )
        .await
    {
        Ok(results) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "success": true,
                "view": name,
                "count": results.len(),
                "data": results
            })),
        )
            .into_response(),
        Err(e) => AppError(StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

#[derive(Debug, Deserialize)]
pub struct RefreshViewRequest {
    pub concurrently: Option<bool>,
}

async fn refresh_view_handler(
    Extension(engine): Extension<Arc<BosEngine>>,
    Path(name): Path<String>,
    req_opt: Option<Json<RefreshViewRequest>>,
) -> Response {
    let conc = req_opt
        .map(|r| r.concurrently.unwrap_or(false))
        .unwrap_or(false);
    match engine.refresh_view(&name, conc).await {
        Ok(_) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "success": true,
                "view": name,
                "message": format!("View '{}' refreshed successfully", name)
            })),
        )
            .into_response(),
        Err(e) => AppError(StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

#[derive(Debug, Deserialize)]
pub struct ExecuteQueryRequest {
    pub params: Option<serde_json::Value>,
}

async fn execute_query_handler(
    headers: HeaderMap,
    Extension(engine): Extension<Arc<BosEngine>>,
    Path(name): Path<String>,
    req_opt: Option<Json<ExecuteQueryRequest>>,
) -> Response {
    let ctx = match extract_context(&headers) {
        Ok(ctx) => ctx,
        Err(response) => return response,
    };
    let params = req_opt
        .and_then(|r| r.params.clone())
        .unwrap_or(serde_json::json!({}));

    match engine.execute_named_query(&name, params, &ctx).await {
        Ok(results) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "success": true,
                "query": name,
                "count": results.len(),
                "data": results
            })),
        )
            .into_response(),
        Err(e) => AppError(StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

#[derive(Debug, Deserialize)]
pub struct CheckPermissionRequest {
    pub entity: String,
    pub action: String,
}

async fn check_permission_handler(
    headers: HeaderMap,
    Extension(engine): Extension<Arc<BosEngine>>,
    Json(req): Json<CheckPermissionRequest>,
) -> Response {
    let ctx = match extract_context(&headers) {
        Ok(ctx) => ctx,
        Err(response) => return response,
    };
    match engine
        .check_permission(&ctx, &req.entity, &req.action)
        .await
    {
        Ok(allowed) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "entity": req.entity,
                "action": req.action,
                "allowed": allowed
            })),
        )
            .into_response(),
        Err(e) => AppError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[derive(Debug, Deserialize)]
pub struct SendNotificationRequest {
    #[serde(default = "default_severity")]
    pub severity: String,
    #[serde(default = "default_target")]
    pub target: String,
    pub title: Option<String>,
    pub message: String,
    pub field: Option<String>,
    pub code: Option<String>,
    pub details: Option<String>,
    pub duration_ms: Option<u64>,
}

fn default_severity() -> String {
    "info".to_string()
}

fn default_target() -> String {
    "toast".to_string()
}

async fn notification_stream_handler(
    Extension(engine): Extension<Arc<BosEngine>>,
) -> Sse<impl futures_util::stream::Stream<Item = Result<Event, std::convert::Infallible>>> {
    let rx = engine.notification_hub.subscribe();
    let stream = BroadcastStream::new(rx).filter_map(|item| match item {
        Ok(notification) => {
            let json = serde_json::to_string(&notification).unwrap_or_default();
            Some(Ok(Event::default().data(json)))
        }
        Err(_) => None,
    });

    Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(std::time::Duration::from_secs(15))
            .text("ping"),
    )
}

async fn list_notifications_handler(
    Extension(engine): Extension<Arc<BosEngine>>,
) -> Json<serde_json::Value> {
    let history = engine.notification_hub.get_history().await;
    Json(serde_json::json!({
        "success": true,
        "count": history.len(),
        "notifications": history
    }))
}

async fn clear_notifications_handler(
    Extension(engine): Extension<Arc<BosEngine>>,
) -> Json<serde_json::Value> {
    engine.notification_hub.clear_history().await;
    Json(serde_json::json!({
        "success": true,
        "message": "Notification history cleared"
    }))
}

async fn send_test_notification_handler(
    Extension(engine): Extension<Arc<BosEngine>>,
    Json(req): Json<SendNotificationRequest>,
) -> Response {
    let severity = match req.severity.to_lowercase().as_str() {
        "success" => crate::NotificationSeverity::Success,
        "warning" => crate::NotificationSeverity::Warning,
        "error" => crate::NotificationSeverity::Error,
        _ => crate::NotificationSeverity::Info,
    };
    let target = match req.target.to_lowercase().as_str() {
        "popup" | "modal" => crate::NotificationTarget::Popup,
        "infield" | "field" => crate::NotificationTarget::Infield,
        "banner" => crate::NotificationTarget::Banner,
        _ => crate::NotificationTarget::Toast,
    };

    let mut notif = crate::Notification::new(severity, target, req.message);
    if let Some(t) = req.title {
        notif = notif.with_title(t);
    }
    if let Some(f) = req.field {
        notif = notif.with_field(f);
    }
    if let Some(c) = req.code {
        notif = notif.with_code(c);
    }
    if let Some(d) = req.details {
        notif = notif.with_details(d);
    }
    if let Some(ms) = req.duration_ms {
        notif = notif.with_duration(ms);
    }

    // Broadcast across SSE stream
    engine.notification_hub.broadcast(notif.clone()).await;

    let notifs = vec![notif.clone()];
    let body = serde_json::json!({
        "success": true,
        "message": "Notification broadcasted successfully",
        "notification": notif
    });

    json_response_with_notifications(StatusCode::OK, body, notifs)
}
