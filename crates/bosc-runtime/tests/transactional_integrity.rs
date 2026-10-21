//! End-to-end checks of the unit-of-work, schema sync and execution-limit guarantees against a
//! real PostgreSQL server.
//!
//! Set `BOS_TEST_DATABASE_URL` to a connection URL for a role allowed to create databases
//! (e.g. `postgres://postgres:secret@localhost:5432/postgres`). Every test creates its own
//! throwaway database and drops it afterwards. Without the variable the tests are skipped.

use bosc_ir::{CodeunitIR, MethodIR};
use bosc_runtime::{
    BosEngine, CompileScriptRequest, RegisterMetaObjectRequest, RuntimeError, SessionContext,
};
use serde_json::json;
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use uuid::Uuid;

struct TestDb {
    admin_url: String,
    name: String,
    engine: BosEngine,
    ctx: SessionContext,
}

impl TestDb {
    async fn create() -> Option<Self> {
        let Ok(admin_url) = std::env::var("BOS_TEST_DATABASE_URL") else {
            eprintln!("BOS_TEST_DATABASE_URL is not set; skipping database integration test");
            return None;
        };
        let name = format!("bos_it_{}", Uuid::new_v4().simple());
        let admin = PgPoolOptions::new()
            .max_connections(1)
            .connect(&admin_url)
            .await
            .expect("connect admin");
        sqlx::query(&format!("CREATE DATABASE \"{name}\""))
            .execute(&admin)
            .await
            .expect("create database");
        admin.close().await;

        let base = admin_url.split('?').next().unwrap_or(&admin_url);
        let prefix = &base[..base
            .rfind('/')
            .expect("database URL must contain a database name")];
        let engine = BosEngine::connect(&format!("{prefix}/{name}"))
            .await
            .expect("connect test database");
        engine
            .provision_system(Some("shared_rls"))
            .await
            .expect("provision kernel");

        let ctx = SessionContext {
            tenant_id: Uuid::new_v4(),
            company_id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            roles: vec!["Admin".to_string()],
        };
        Some(Self {
            admin_url,
            name,
            engine,
            ctx,
        })
    }

    async fn pool(&self) -> PgPool {
        self.engine.pool().await
    }

    async fn apply(
        &self,
        script: &str,
        allow_destructive: bool,
    ) -> Result<bosc_runtime::CompileScriptResponse, RuntimeError> {
        self.engine
            .compile_script(CompileScriptRequest {
                script: script.to_string(),
                package_name: Some("it.pkg".to_string()),
                apply: true,
                allow_destructive,
            })
            .await
    }

    async fn count(&self, sql: &str) -> i64 {
        sqlx::query_scalar::<_, i64>(sql)
            .fetch_one(&self.pool().await)
            .await
            .expect(sql)
    }

    async fn drop(self) {
        self.engine.pool().await.close().await;
        let admin = PgPoolOptions::new()
            .max_connections(1)
            .connect(&self.admin_url)
            .await
            .expect("connect admin");
        sqlx::query(&format!(
            "DROP DATABASE IF EXISTS \"{}\" WITH (FORCE)",
            self.name
        ))
        .execute(&admin)
        .await
        .expect("drop database");
        admin.close().await;
    }
}

fn script_method(name: &str, script: &str) -> MethodIR {
    MethodIR {
        name: name.to_string(),
        is_public: true,
        is_static: false,
        params: vec![],
        return_type: None,
        trigger_event: None,
        annotations: vec![],
        actions: vec![],
        script: Some(script.to_string()),
        is_pushdown: false,
    }
}

fn codeunit(name: &str, methods: Vec<MethodIR>) -> CodeunitIR {
    CodeunitIR {
        name: name.to_string(),
        package_name: "it.pkg".to_string(),
        bound_table: None,
        is_exposed: true,
        is_transactional: true,
        is_pushdown: false,
        execution_mode: None,
        max_fuel: None,
        methods,
    }
}

const ACCOUNT_TABLE: &str = r#"
    table ItAccount {
        code: String(20) { notNull = true; };
        balance: Decimal(18, 2);
        primaryKey(code);
    }
"#;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn codeunit_failure_rolls_back_every_write() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    db.apply(ACCOUNT_TABLE, false).await.expect("deploy table");
    db.engine
        .register_codeunit(codeunit(
            "ItPosting",
            vec![script_method(
                "PostTwice",
                r#"
                ItAccount.create({ code: rec.code, balance: 1 });
                ItAccount.create({ code: rec.code + "-2", balance: 2 });
                if (rec.fail) { throw new Error("posting failed after two writes"); }
                "#,
            )],
        ))
        .await
        .expect("register codeunit");

    let err = db
        .engine
        .execute_procedure(
            "ItPosting",
            "PostTwice",
            json!({ "code": "A", "fail": true }),
            &db.ctx,
        )
        .await
        .expect_err("procedure must fail");
    assert!(err.to_string().contains("posting failed"), "{err}");
    assert_eq!(
        db.count("SELECT COUNT(*) FROM it_account").await,
        0,
        "failed codeunit must leave no rows"
    );

    db.engine
        .execute_procedure(
            "ItPosting",
            "PostTwice",
            json!({ "code": "B", "fail": false }),
            &db.ctx,
        )
        .await
        .expect("procedure succeeds");
    assert_eq!(db.count("SELECT COUNT(*) FROM it_account").await, 2);
    db.drop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn trigger_failure_rolls_back_insert_and_trigger_writes() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    db.apply(
        r#"
        table ItAudit {
            code: String(20) { notNull = true; };
            primaryKey(code);
        }
        "#,
        false,
    )
    .await
    .expect("deploy audit table");

    let definition = json!({
        "name": "ItItem",
        "sql_table_name": "it_item",
        "caption": "Item",
        "table_type": "Normal",
        "data_per_company": true,
        "columns": [{
            "name": "code", "sql_name": "code", "sql_type": "VARCHAR(20)", "is_nullable": false,
            "is_unique": true, "default_val": null, "caption": "Code", "foreign_key": null
        }],
        "primary_key": ["code"],
        "indexes": [],
        "flow_fields": [],
        "triggers": [{
            "name": "onAfterInsert",
            "params": [],
            "actions": [],
            "script": "ItAudit.create({ code: rec.code }); if (rec.code === 'BAD') { throw new Error('rejected after insert'); }"
        }]
    });
    db.engine
        .register_meta_object(RegisterMetaObjectRequest {
            object_type: "table".to_string(),
            name: "ItItem".to_string(),
            package_name: Some("it.pkg".to_string()),
            version: None,
            caption: None,
            sub_type: None,
            data_per_company: Some(true),
            columns: None,
            primary_key: None,
            check_constraints: None,
            state_machine: None,
            definition: Some(definition),
            allow_destructive: false,
        })
        .await
        .expect("register table with trigger");

    let err = db
        .engine
        .insert_record("it_item", json!({ "code": "BAD" }), &db.ctx)
        .await
        .expect_err("trigger rejects");
    assert!(err.to_string().contains("rejected after insert"), "{err}");
    assert_eq!(
        db.count("SELECT COUNT(*) FROM it_item").await,
        0,
        "insert must be rolled back"
    );
    assert_eq!(
        db.count("SELECT COUNT(*) FROM it_audit").await,
        0,
        "trigger write must be rolled back"
    );

    db.engine
        .insert_record("it_item", json!({ "code": "GOOD" }), &db.ctx)
        .await
        .expect("insert succeeds");
    assert_eq!(db.count("SELECT COUNT(*) FROM it_item").await, 1);
    assert_eq!(db.count("SELECT COUNT(*) FROM it_audit").await, 1);
    db.drop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn updates_require_the_current_version() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    db.apply(ACCOUNT_TABLE, false).await.expect("deploy table");
    let id = db
        .engine
        .insert_record(
            "it_account",
            json!({ "code": "V1", "balance": 10 }),
            &db.ctx,
        )
        .await
        .expect("insert");

    let missing = db
        .engine
        .update_record("it_account", id, json!({ "balance": 20 }), &db.ctx)
        .await;
    assert!(
        matches!(missing, Err(RuntimeError::VersionRequired(_))),
        "{missing:?}"
    );

    let stale = db
        .engine
        .update_record(
            "it_account",
            id,
            json!({ "balance": 20, "_system_version": 7 }),
            &db.ctx,
        )
        .await;
    assert!(
        matches!(stale, Err(RuntimeError::ConcurrencyConflict(_))),
        "{stale:?}"
    );

    let updated = db
        .engine
        .update_record(
            "it_account",
            id,
            json!({ "balance": 20, "_system_version": 1 }),
            &db.ctx,
        )
        .await
        .expect("update with current version");
    assert_eq!(updated["_system_version"], json!(2));

    let replay = db
        .engine
        .update_record(
            "it_account",
            id,
            json!({ "balance": 30, "_system_version": 1 }),
            &db.ctx,
        )
        .await;
    assert!(
        matches!(replay, Err(RuntimeError::ConcurrencyConflict(_))),
        "second writer with the old version must lose"
    );
    assert_eq!(
        db.count("SELECT COUNT(*) FROM it_account WHERE balance = 20")
            .await,
        1
    );
    db.drop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn relations_become_enforced_foreign_keys() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let report = db
        .apply(
            r#"
            table ItOrder {
                no: String(20) { notNull = true; };
                customerNo: String(20) { relation = ItCustomer.no; onDelete = cascade; };
                primaryKey(no);
            }
            table ItCustomer {
                no: String(20) { notNull = true; };
                name: String(100);
                primaryKey(no);
            }
            "#,
            false,
        )
        .await
        .expect("deploy related tables")
        .schema_sync
        .expect("schema report");
    assert!(
        report.relations.iter().any(|r| r.column == "customer_no"
            && r.status == "created"
            && r.on_delete == "CASCADE"),
        "{:?}",
        report.relations
    );

    let orphan = db
        .engine
        .insert_record(
            "it_order",
            json!({ "no": "O1", "customer_no": "NOPE" }),
            &db.ctx,
        )
        .await;
    assert!(
        orphan.is_err(),
        "an order for a missing customer must be rejected"
    );

    let customer = db
        .engine
        .insert_record(
            "it_customer",
            json!({ "no": "C1", "name": "Acme" }),
            &db.ctx,
        )
        .await
        .expect("customer");
    db.engine
        .insert_record(
            "it_order",
            json!({ "no": "O2", "customer_no": "C1" }),
            &db.ctx,
        )
        .await
        .expect("order");
    db.engine
        .delete_record("it_customer", customer, &db.ctx)
        .await
        .expect("delete customer");
    assert_eq!(
        db.count("SELECT COUNT(*) FROM it_order").await,
        0,
        "cascade removes the customer's orders"
    );

    let again = db
        .apply(
            r#"
            table ItOrder {
                no: String(20) { notNull = true; };
                customerNo: String(20) { relation = ItCustomer.no; onDelete = cascade; };
                primaryKey(no);
            }
            table ItCustomer {
                no: String(20) { notNull = true; };
                name: String(100);
                primaryKey(no);
            }
            "#,
            false,
        )
        .await
        .expect("redeploy")
        .schema_sync
        .expect("schema report");
    assert!(
        again.steps.is_empty(),
        "redeploying an unchanged package changes nothing: {:?}",
        again.steps
    );
    assert!(
        again.relations.iter().all(|r| r.status == "unchanged"),
        "{:?}",
        again.relations
    );

    let restrict = db
        .apply(
            r#"
            table ItOrder {
                no: String(20) { notNull = true; };
                customerNo: String(20) { relation = ItCustomer.no; onDelete = restrict; };
                primaryKey(no);
            }
            "#,
            false,
        )
        .await
        .expect("change delete rule")
        .schema_sync
        .expect("schema report");
    assert!(
        restrict
            .relations
            .iter()
            .any(|r| r.on_delete == "RESTRICT" && r.status == "created"),
        "{:?}",
        restrict.relations
    );
    let customer = db
        .engine
        .insert_record("it_customer", json!({ "no": "C2" }), &db.ctx)
        .await
        .expect("customer");
    db.engine
        .insert_record(
            "it_order",
            json!({ "no": "O3", "customer_no": "C2" }),
            &db.ctx,
        )
        .await
        .expect("order");
    assert!(
        db.engine
            .delete_record("it_customer", customer, &db.ctx)
            .await
            .is_err(),
        "restrict blocks the delete"
    );
    db.drop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn schema_changes_migrate_data_and_guard_destructive_steps() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    db.apply(
        r#"
        table ItContact {
            code: String(20) { notNull = true; };
            phone: String(20);
            primaryKey(code);
        }
        "#,
        false,
    )
    .await
    .expect("v1");
    db.engine
        .insert_record(
            "it_contact",
            json!({ "code": "K1", "phone": "555-0100" }),
            &db.ctx,
        )
        .await
        .expect("seed row");

    let v2 = db
        .apply(
            r#"
            table ItContact {
                code: String(40) { notNull = true; };
                mobile: String(20) { renamedFrom = "phone"; };
                visits: Integer { notNull = true; default = 0; };
                primaryKey(code);
            }
            "#,
            false,
        )
        .await
        .expect("v2 is non-destructive")
        .schema_sync
        .expect("schema report");
    let actions: Vec<&str> = v2.steps.iter().map(|s| s.action.as_str()).collect();
    assert!(actions.contains(&"rename_column"), "{actions:?}");
    assert!(actions.contains(&"change_type"), "{actions:?}");
    assert!(actions.contains(&"add_column"), "{actions:?}");
    let (mobile, visits): (Option<String>, i32) =
        sqlx::query_as("SELECT mobile, visits FROM it_contact WHERE code = 'K1'")
            .fetch_one(&db.pool().await)
            .await
            .expect("row kept");
    assert_eq!(mobile.as_deref(), Some("555-0100"), "rename keeps the data");
    assert_eq!(visits, 0);

    let v3 = r#"
        table ItContact {
            code: String(40) { notNull = true; };
            visits: Integer { notNull = true; default = 0; };
            primaryKey(code);
        }
    "#;
    let blocked = db.apply(v3, false).await;
    assert!(
        matches!(blocked, Err(RuntimeError::SchemaSyncBlocked(_))),
        "{blocked:?}"
    );
    assert_eq!(
        db.count("SELECT COUNT(*) FROM information_schema.columns WHERE table_name = 'it_contact' AND column_name = 'mobile'").await,
        1,
        "blocked deployment leaves the column and its data in place"
    );

    let forced = db
        .apply(v3, true)
        .await
        .expect("explicitly allowed")
        .schema_sync
        .expect("schema report");
    assert!(
        forced.steps.iter().any(|s| s.action == "drop_column"),
        "{:?}",
        forced.steps
    );
    assert_eq!(
        db.count("SELECT COUNT(*) FROM information_schema.columns WHERE table_name = 'it_contact' AND column_name = 'mobile'").await,
        0
    );
    assert!(
        db.count("SELECT COUNT(*) FROM _bos_schema_migrations WHERE package_name = 'it.pkg' AND destructive").await >= 1,
        "destructive migrations are journaled"
    );
    db.drop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_runner_rolls_back_test_writes() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    db.apply(ACCOUNT_TABLE, false).await.expect("deploy table");
    db.engine
        .register_codeunit(codeunit(
            "ItAccountTests",
            vec![
                script_method(
                    "testCreatesAccount",
                    r#"ItAccount.create({ code: "T1", balance: 5 });"#,
                ),
                script_method(
                    "testFails",
                    r#"ItAccount.create({ code: "T2" }); throw new Error("expected failure");"#,
                ),
            ],
        ))
        .await
        .expect("register test codeunit");

    let results = db
        .engine
        .run_tests(Some("ItAccountTests"), &db.ctx)
        .await
        .expect("run tests");
    assert_eq!(results["total"], json!(2), "{results}");
    assert_eq!(results["passed"], json!(1), "{results}");
    assert_eq!(results["failed"], json!(1), "{results}");
    assert_eq!(
        db.count("SELECT COUNT(*) FROM it_account").await,
        0,
        "tests never leave data behind"
    );
    db.drop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn fuel_budget_stops_runaway_scripts_and_rolls_back() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    db.apply(ACCOUNT_TABLE, false).await.expect("deploy table");
    db.engine
        .register_codeunit(codeunit(
            "ItRunaway",
            vec![script_method(
                "Spin",
                r#"ItAccount.create({ code: "R1" }); while (true) {}"#,
            )],
        ))
        .await
        .expect("register codeunit");

    let started = std::time::Instant::now();
    let err = db
        .engine
        .execute_procedure("ItRunaway", "Spin", json!({}), &db.ctx)
        .await
        .expect_err("runaway script is stopped");
    assert!(err.to_string().contains("fuel exhausted"), "{err}");
    assert!(
        started.elapsed() < std::time::Duration::from_secs(5),
        "stopped by fuel, not wall clock"
    );
    assert_eq!(db.count("SELECT COUNT(*) FROM it_account").await, 0);
    db.drop().await;
}
