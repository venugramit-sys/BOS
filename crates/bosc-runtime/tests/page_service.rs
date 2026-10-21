//! End-to-end checks of the compiled page service against a real PostgreSQL server and the real Svelte
//! compiler: startup hash check, incremental recompilation, cross-node invalidation, single-flight
//! compilation, server-side page triggers and the HTTP routes.
//!
//! Needs `BOS_TEST_DATABASE_URL` (a role allowed to create databases, e.g.
//! `postgres://postgres:secret@localhost:5432/postgres`) and the UI runtime build
//! (`ui/dist/bos-runtime/svelte-compiler.js`, from `npm run build` in ui/). Without the database URL the
//! test is skipped, like the other database integration tests.

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use bosc_runtime::{BosEngine, CompileScriptRequest, NodeConfig, PageServiceConfig, SessionContext};
use sqlx::postgres::PgPoolOptions;
use std::path::PathBuf;
use std::time::Duration;
use tower::ServiceExt;
use uuid::Uuid;

const PACKAGE: &str = r#"
table Customer {
    no: String(20) { notNull = true; };
    name: String(100);
    creditLimit: Decimal(18, 2) { default = 0; };
    rating: Integer;
    primaryKey(no);
}

table Vendor {
    no: String(20) { notNull = true; };
    name: String(100);
    primaryKey(no);
}

page CustomerList for Customer {
    pageType = List;
    layout { area(Content) { repeater(Lines) { field(no); field(name); } } }
}

page CustomerCard for Customer {
    pageType = Card;
    variables { counter: Integer; }
    layout { area(Content) { group(General) { field(no); field(name); field(creditLimit); } } }
    actions {
        action(Touch) {
            onAction(rec, ctx) {
                rec.name = "Touched " + rec.no;
                ctx.page.variables.counter = (ctx.page.variables.counter || 0) + 1;
                ctx.ui.success("Touched");
                ctx.ui.openModal("CustomerList", { no: rec.no });
                return 42;
            }
        }
        action(Risky) {
            onAction(rec, ctx) {
                if (!ctx.ui.confirm("Really do it?")) return "cancelled";
                return "done";
            }
        }
    }
}

pageextension CustomerCardRating extends CustomerCard {
    layout { addLast(General) { field(rating) { caption = "Customer Rating"; } } }
}

page VendorCard for Vendor {
    pageType = Card;
    layout { area(Content) { group(General) { field(no); field(name); } } }
}
"#;

struct TestDb {
    admin_url: String,
    name: String,
    url: String,
    roots: Vec<PathBuf>,
}

impl TestDb {
    async fn create() -> Option<TestDb> {
        let Ok(admin_url) = std::env::var("BOS_TEST_DATABASE_URL") else {
            eprintln!("BOS_TEST_DATABASE_URL is not set; skipping page service integration test");
            return None;
        };
        let name = format!("bos_pages_it_{}", Uuid::new_v4().simple());
        let admin = PgPoolOptions::new().max_connections(1).connect(&admin_url).await.expect("connect admin");
        sqlx::query(&format!("CREATE DATABASE \"{name}\"")).execute(&admin).await.expect("create database");
        admin.close().await;
        let base = admin_url.split('?').next().unwrap_or(&admin_url);
        let url = format!("{}/{name}", &base[..base.rfind('/').expect("database URL needs a database name")]);
        Some(TestDb { admin_url, name, url, roots: Vec::new() })
    }

    fn artifact_root(&mut self) -> PathBuf {
        let root = std::env::temp_dir().join(format!("bos-pages-it-{}", Uuid::new_v4().simple()));
        self.roots.push(root.clone());
        root
    }

    async fn drop_database(self) {
        for r in &self.roots {
            let _ = std::fs::remove_dir_all(r);
        }
        let admin = PgPoolOptions::new().max_connections(1).connect(&self.admin_url).await.expect("connect admin");
        let _ = sqlx::query(&format!("DROP DATABASE IF EXISTS \"{}\" WITH (FORCE)", self.name)).execute(&admin).await;
    }
}

fn runtime_dir() -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../ui/dist/bos-runtime");
    assert!(dir.join("svelte-compiler.js").is_file(), "build the UI runtime first: `npm run build` in ui/");
    dir
}

fn config(root: PathBuf) -> PageServiceConfig {
    PageServiceConfig { artifact_root: root, runtime_dir: runtime_dir(), workers: 2, seed_dirs: Vec::new() }
}

fn ctx() -> SessionContext {
    SessionContext { tenant_id: Uuid::nil(), company_id: Uuid::nil(), user_id: Uuid::nil(), roles: vec!["Admin".into()] }
}

async fn apply(engine: &BosEngine, script: &str) {
    let res = engine
        .compile_script(CompileScriptRequest { script: script.to_string(), package_name: Some("it.pages".into()), apply: true, allow_destructive: false })
        .await
        .expect("apply script");
    assert!(res.applied, "script must apply: {}", res.message);
}

fn manifest_json(engine: &BosEngine) -> serde_json::Value {
    let (_, bytes) = engine.page_service().unwrap().manifest().expect("manifest");
    serde_json::from_slice(&bytes).unwrap()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn page_service_end_to_end() {
    let Some(mut db) = TestDb::create().await else { return };
    let engine = BosEngine::connect(&db.url).await.expect("connect");
    engine.provision_system(Some("shared_rls")).await.expect("provision");
    apply(&engine, PACKAGE).await;

    // --- startup: catalog resolved once, every UI page listed ----------------------------------
    let root = db.artifact_root();
    let service = engine.start_page_service(config(root.clone())).await.expect("start");
    assert!(service.compiler_error().is_none(), "{:?}", service.compiler_error());
    let manifest = manifest_json(&engine);
    let pages = manifest["pages"].as_object().unwrap();
    assert_eq!(pages.len(), 3, "{manifest}");
    let db_key = manifest["db_key"].as_str().unwrap().to_string();
    let card = &pages["CustomerCard"];
    assert_eq!(card["errors"], 0, "{card}");
    let card_file = card["file"].as_str().unwrap().to_string();

    // The background warm-up compiles all three; wait for it, then nothing more compiles.
    let deadline = std::time::Instant::now() + Duration::from_secs(60);
    while service.compile_count() < 3 {
        assert!(std::time::Instant::now() < deadline, "warm-up did not finish");
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let js = String::from_utf8(service.artifact(&db_key, &card_file).await.unwrap()).unwrap();
    assert!(js.contains("svelte/internal/client") && js.contains("@bos/page-kit"));
    assert_eq!(service.compile_count(), 3, "served from disk");
    let entry = service.page("CustomerCard").unwrap();
    assert_eq!(entry.compiled.applied_extensions, vec!["CustomerCardRating"]);

    // --- restart with the same store: zero recompiles --------------------------------------------
    let restarted = BosEngine::connect(&db.url).await.expect("connect");
    let svc2 = restarted.start_page_service(config(root.clone())).await.unwrap();
    assert_eq!(manifest_json(&restarted)["etag"], manifest["etag"], "same catalog -> same manifest");
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert_eq!(svc2.compile_count(), 0, "unchanged pages are never recompiled");

    // --- second node with its own store, listening for changes -----------------------------------
    let node_b = BosEngine::connect(&db.url).await.expect("connect");
    let root_b = db.artifact_root();
    let svc_b = node_b.start_page_service(config(root_b)).await.unwrap();
    let etag_b = svc_b.manifest().unwrap().0;

    // --- a table change recompiles only the pages that depend on it ------------------------------
    apply(&engine, "table Vendor { no: String(20) { notNull = true; }; name: String(100) { caption = \"Vendor Name\"; }; primaryKey(no); }").await;
    let after = manifest_json(&engine);
    assert_ne!(after["pages"]["VendorCard"]["hash"], manifest["pages"]["VendorCard"]["hash"]);
    assert_eq!(after["pages"]["CustomerCard"]["hash"], manifest["pages"]["CustomerCard"]["hash"]);
    assert_eq!(after["pages"]["CustomerList"]["hash"], manifest["pages"]["CustomerList"]["hash"]);
    let vendor = service.page("VendorCard").unwrap();
    let name_caption = vendor.compiled.controls[0].children[0].children[1].caption.clone();
    assert_eq!(name_caption.as_deref(), Some("Vendor Name"));

    // Node B hears about it through bos_pages_channel.
    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    while svc_b.manifest().unwrap().0 == etag_b {
        assert!(std::time::Instant::now() < deadline, "node B did not pick up the change");
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert_eq!(svc_b.page("VendorCard").unwrap().hash, vendor.hash, "both nodes compute the same hash");

    // --- single flight: concurrent first requests share one compilation --------------------------
    let fresh = svc2.clone();
    fresh.refresh(&restarted.pool().await, &db.url).await.unwrap();
    let vendor_file = fresh.page("VendorCard").unwrap().file.clone();
    let before = fresh.compile_count();
    let mut tasks = Vec::new();
    for _ in 0..6 {
        let s = fresh.clone();
        let (k, f) = (db_key.clone(), vendor_file.clone());
        tasks.push(tokio::spawn(async move { s.artifact(&k, &f).await.map(|b| b.len()) }));
    }
    let sizes: Vec<usize> = futures_util::future::join_all(tasks).await.into_iter().map(|r| r.unwrap().unwrap()).collect();
    assert!(sizes.windows(2).all(|w| w[0] == w[1]));
    assert!(fresh.compile_count() - before <= 1, "one compilation for six concurrent requests");

    // --- server-side page triggers ---------------------------------------------------------------
    let run = engine
        .run_page_trigger("CustomerCard", "action.Touch", serde_json::json!({"no": "C1"}), serde_json::json!({"counter": 4}), Default::default(), &ctx())
        .await
        .expect("trigger");
    assert_eq!(run.record["name"], "Touched C1");
    assert_eq!(run.variables["counter"], 5);
    assert_eq!(run.return_value, Some(serde_json::json!(42)));
    assert!(run.messages.as_array().is_some_and(|m| !m.is_empty()), "{:?}", run.messages);
    assert_eq!(run.directives[0]["type"], "modal");

    let pending = engine
        .run_page_trigger("CustomerCard", "action.Risky", serde_json::json!({}), serde_json::json!({}), Default::default(), &ctx())
        .await
        .unwrap_err();
    assert!(pending.to_string().contains("ConfirmRequired: Really do it?"), "{pending}");
    let answered = engine
        .run_page_trigger(
            "CustomerCard",
            "action.Risky",
            serde_json::json!({}),
            serde_json::json!({}),
            [("Really do it?".to_string(), true)].into_iter().collect(),
            &ctx(),
        )
        .await
        .unwrap();
    assert_eq!(answered.return_value, Some(serde_json::json!("done")));

    // --- HTTP routes -----------------------------------------------------------------------------
    let app = bosc_runtime::create_node_router(engine.clone(), NodeConfig::default());
    let call = |req: Request<Body>| {
        let app = app.clone();
        async move { app.oneshot(req).await.unwrap() }
    };
    let res = call(Request::get("/api/v1/pages/manifest").body(Body::empty()).unwrap()).await;
    assert_eq!(res.status(), StatusCode::OK);
    let etag = res.headers()[header::ETAG].to_str().unwrap().to_string();
    assert_eq!(res.headers()[header::CACHE_CONTROL], "no-cache");
    let res = call(Request::get("/api/v1/pages/manifest").header(header::IF_NONE_MATCH, &etag).body(Body::empty()).unwrap()).await;
    assert_eq!(res.status(), StatusCode::NOT_MODIFIED);

    let res = call(Request::get(format!("/page-artifacts/{db_key}/{card_file}")).body(Body::empty()).unwrap()).await;
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(res.headers()[header::CACHE_CONTROL], "public, max-age=31536000, immutable");
    assert!(res.headers()[header::CONTENT_TYPE].to_str().unwrap().starts_with("text/javascript"));
    let res = call(Request::get(format!("/page-artifacts/{db_key}/..%2Fsecret.0123456789abcdef.js")).body(Body::empty()).unwrap()).await;
    assert_eq!(res.status(), StatusCode::NOT_FOUND);

    let res = call(Request::get("/api/v1/pages/CustomerCard/artifact.svelte").body(Body::empty()).unwrap()).await;
    assert_eq!(res.status(), StatusCode::OK);
    let body = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
    assert!(String::from_utf8_lossy(&body).contains("createPage(() => ctx, META, C, X)"));

    let mut draft = entry.merged.clone();
    draft.caption = Some("Draft Card".into());
    let req = Request::post("/api/v1/pages/preview")
        .header(header::CONTENT_TYPE, "application/json")
        .header("x-bos-roles", "Admin")
        .body(Body::from(serde_json::json!({ "page": draft, "extensions": [] }).to_string()))
        .unwrap();
    let res = call(req).await;
    assert_eq!(res.status(), StatusCode::OK);
    let preview: serde_json::Value = serde_json::from_slice(&axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(preview["compiled"]["caption"], "Draft Card");
    let res = call(Request::get(preview["url"].as_str().unwrap()).body(Body::empty()).unwrap()).await;
    assert_eq!(res.status(), StatusCode::OK);
    let forbidden = Request::post("/api/v1/pages/preview")
        .header(header::CONTENT_TYPE, "application/json")
        .header("x-bos-roles", "Clerk")
        .body(Body::from(serde_json::json!({ "page": draft }).to_string()))
        .unwrap();
    assert_eq!(call(forbidden).await.status(), StatusCode::FORBIDDEN);

    let req = Request::post("/api/v1/pages/CustomerCard/triggers/action.Risky")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from("{}"))
        .unwrap();
    let res = call(req).await;
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let body: serde_json::Value = serde_json::from_slice(&axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(body["code"], "CONFIRM_REQUIRED");
    assert_eq!(body["confirm"], "Really do it?");

    let res = call(Request::post("/api/v1/pages/CustomerCard/triggers/action.Missing").body(Body::empty()).unwrap()).await;
    assert_eq!(res.status(), StatusCode::NOT_FOUND);

    // --- Designer: property schema, source outline / span edits, customization layer -----------
    let json_of = |res: axum::response::Response| async move {
        serde_json::from_slice::<serde_json::Value>(&axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap()).unwrap()
    };
    let res = call(Request::get("/api/v1/pages/property-schema").body(Body::empty()).unwrap()).await;
    assert_eq!(res.status(), StatusCode::OK);
    let schema = json_of(res).await;
    assert!(schema["page_types"].as_array().unwrap().iter().any(|t| t["name"] == "RoleCenter" && t["allows"].as_array().unwrap().contains(&"cue".into())));

    let post_json = |uri: &str, roles: &str, body: serde_json::Value| {
        Request::post(uri).header(header::CONTENT_TYPE, "application/json").header("x-bos-roles", roles).body(Body::from(body.to_string())).unwrap()
    };
    let res = call(post_json("/api/v1/pages/parse", "Admin", serde_json::json!({ "source": PACKAGE }))).await;
    assert_eq!(res.status(), StatusCode::OK);
    let parsed = json_of(res).await;
    assert!(parsed["outline"]["objects"].as_array().unwrap().iter().any(|o| o["name"] == "CustomerCard"));
    let edit = serde_json::json!({
        "source": PACKAGE,
        "page": "CustomerCard",
        "ops": [{ "op": "setProperty", "path": "area:Content/group:General/field:name", "name": "caption", "value": "\"Full Name\"" }],
    });
    let res = call(post_json("/api/v1/pages/edit", "Admin", edit.clone())).await;
    assert_eq!(res.status(), StatusCode::OK);
    let edited = json_of(res).await;
    assert!(edited["source"].as_str().unwrap().contains("field(name) { caption = \"Full Name\"; }"));
    assert!(edited["model"]["pages"].as_array().unwrap().iter().any(|p| p["name"] == "CustomerCard"));
    assert_eq!(call(post_json("/api/v1/pages/edit", "Clerk", edit)).await.status(), StatusCode::FORBIDDEN);

    let changes = serde_json::json!({ "changes": {
        "controls": [{ "name": "name", "caption": "Client Name", "style": { "emphasis": "Strong", "background": "#fff7ed" } }],
        "actions": [{ "name": "Risky", "visible": false }],
    }});
    let res = call(post_json("/api/v1/pages/CustomerCard/customization/preview", "Admin", changes.clone())).await;
    assert_eq!(res.status(), StatusCode::OK);
    assert!(json_of(res).await["compiled"].to_string().contains("Client Name"));
    let put = |roles: &str, body: serde_json::Value| {
        Request::put("/api/v1/pages/CustomerCard/customization")
            .header(header::CONTENT_TYPE, "application/json")
            .header("x-bos-roles", roles)
            .body(Body::from(body.to_string()))
            .unwrap()
    };
    assert_eq!(call(put("Clerk", changes.clone())).await.status(), StatusCode::FORBIDDEN);
    let res = call(put("Admin", changes)).await;
    assert_eq!(res.status(), StatusCode::OK);
    let saved = json_of(res).await;
    assert!(saved["source"].as_str().unwrap().contains("pageextension CustomerCard_Customization extends CustomerCard"));
    let card = engine.page_service().unwrap().page("CustomerCard").unwrap();
    let compiled = serde_json::to_string(&card.compiled).unwrap();
    assert!(compiled.contains("Client Name"), "customization must be published");
    assert!(compiled.contains("#fff7ed"));
    assert!(!card.compiled.actions.iter().any(|a| a.name == "Risky"), "hidden action must be gone");
    let res = call(Request::get("/api/v1/pages/CustomerCard/customization").body(Body::empty()).unwrap()).await;
    let current = json_of(res).await;
    assert_eq!(current["exists"], true);
    assert_eq!(current["changes"]["controls"][0]["caption"], "Client Name");
    // A forbidden directive (adding a non-field control) is refused with BOS3009.
    let bad = serde_json::json!({ "changes": { "adds": [{ "field": "rating", "anchor": "General", "position": "last" }] } });
    assert_eq!(call(put("Admin", bad)).await.status(), StatusCode::OK, "adding a table field is allowed");
    let current = json_of(call(Request::get("/api/v1/pages/CustomerCard/customization").body(Body::empty()).unwrap()).await).await;
    assert_eq!(current["changes"]["adds"][0]["field"], "rating");
    assert!(serde_json::to_string(&engine.page_service().unwrap().page("CustomerCard").unwrap().compiled).unwrap().contains("\"rating\""));

    drop((engine, restarted, node_b, app));
    db.drop_database().await;
}
