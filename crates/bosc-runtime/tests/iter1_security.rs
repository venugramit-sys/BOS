//! Iteration-1 security/regression tests (QA baseline F1-F3).

use bosc_runtime::SessionContext;
use bosc_runtime::auth_guard::{IdentityState, is_public_route};
use bosc_runtime::quickjs_engine::QuickJsEngine;
use axum::http::Method;
use uuid::Uuid;

fn super_ctx() -> SessionContext {
    SessionContext {
        tenant_id: Uuid::nil(),
        company_id: Uuid::nil(),
        user_id: Uuid::nil(),
        roles: vec!["SUPER".to_string()],
    }
}

fn run_aot(module_js: &str, trigger: &str) -> serde_json::Value {
    let bytecode = bosc_analyzer::BytecodeCompiler::compile_js_to_bytecode("RoleLogic", module_js)
        .expect("compile tablelogic module");
    let engine = QuickJsEngine::new().expect("quickjs");
    let mut rec = serde_json::json!({ "code": "SALES" });
    let mut logs = Vec::new();
    engine
        .execute_trigger_with_bytecode("Role", trigger, "", Some(&bytecode), &mut rec, None,
            &super_ctx(), None, &mut logs)
        .expect("trigger must not throw");
    rec
}

/// F2: `static onInsert(rec, ctx)` (docs/02 3.1) must receive ctx, not a null xRec.
#[test]
fn f2_two_param_lifecycle_trigger_receives_ctx() {
    let js = r#"
        export class RoleLogic {
            static onInsert(rec, ctx) {
                rec.ctxIsObject = (ctx !== null && typeof ctx === "object");
                rec.auditType = typeof ctx.audit;
            }
        }
        export default RoleLogic;
    "#;
    let rec = run_aot(js, "onInsert");
    assert_eq!(rec["ctxIsObject"], serde_json::json!(true), "rec = {rec}");
}

/// Three-parameter (rec, xRec, ctx) triggers keep their existing calling convention.
#[test]
fn f2_three_param_trigger_unchanged() {
    let js = r#"
        export class RoleLogic {
            static onModify(rec, xRec, ctx) {
                rec.xRecIsNull = (xRec === null || xRec === undefined);
                rec.ctxIsObject = (ctx !== null && typeof ctx === "object");
            }
        }
        export default RoleLogic;
    "#;
    let rec = run_aot(js, "onModify");
    assert_eq!(rec["ctxIsObject"], serde_json::json!(true), "rec = {rec}");
}

/// F1: a provisioned node with no users or no roles still needs setup/bootstrap.
#[test]
fn f1_empty_identity_needs_setup() {
    assert!(IdentityState { user_table_exists: false, users: 0, roles: 0 }.needs_bootstrap());
    assert!(IdentityState { user_table_exists: true, users: 0, roles: 3 }.needs_bootstrap());
    assert!(IdentityState { user_table_exists: true, users: 2, roles: 0 }.needs_bootstrap());
    assert!(!IdentityState { user_table_exists: true, users: 1, roles: 1 }.needs_bootstrap());
}

/// F3: only documented public endpoints bypass the session guard.
#[test]
fn f3_data_apis_are_not_public() {
    for p in ["/api/entities/user", "/api/entities/role", "/api/query", "/api/tenants",
              "/api/codeunits/AuthManager/run", "/api/meta/objects", "/api/script/apply"] {
        assert!(!is_public_route(&Method::GET, p), "{p} must require auth");
    }
    for p in ["/api/health", "/api/auth/status", "/api/auth/login", "/api/setup/status",
              "/api/v1/pages/manifest", "/api/v1/pages/manifest/stream"] {
        assert!(is_public_route(&Method::GET, p), "{p} must stay public");
    }
}