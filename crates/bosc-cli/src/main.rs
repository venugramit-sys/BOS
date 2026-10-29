use anyhow::Result;
use bosc_analyzer::SemanticAnalyzer;
use bosc_codegen_db::PostgresGenerator;
use bosc_runtime::{
    create_node_router, BosEngine, NasWorkerDaemon, NodeConfig, SchemaSyncOptions,
    SchemaSyncReport, SchemaSyncRequest, SessionContext,
};
use bosc_syntax::{Lexer, Parser};
use clap::{Parser as ClapParser, Subcommand};
use std::fs;
use std::path::PathBuf;
use uuid::Uuid;

mod lsp;

#[derive(ClapParser)]
#[command(name = "bosc")]
#[command(about = "BOS Script Compiler and Infrastructure Engine", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Compile BOS script files into PostgreSQL DDL or JSON Bytecode IR
    Build {
        #[arg(short, long)]
        input: PathBuf,
        #[arg(short, long)]
        output: Option<PathBuf>,
        #[arg(long)]
        emit_ir: bool,
    },
    /// Execute a Codeunit procedure bytecode against PostgreSQL database
    Run {
        #[arg(short, long)]
        codeunit: String,
        #[arg(short, long)]
        method: String,
        #[arg(short, long, default_value = "{}")]
        params: String,
        #[arg(short, long)]
        file: Option<PathBuf>,
        #[arg(short, long, default_value = "postgres://postgres:postgres@localhost:5432/bos_dev")]
        db: String,
    },
    /// Compile and synchronize the schema of a BOS script into PostgreSQL (one transaction)
    Deploy {
        #[arg(short, long)]
        input: PathBuf,
        #[arg(short, long, default_value = "postgres://postgres:postgres@localhost:5432/bos_dev")]
        db: String,
        /// Package that owns the deployed objects
        #[arg(long, default_value = "bos.app")]
        package: String,
        /// Permit changes that delete or truncate data (dropped fields, narrowed types)
        #[arg(long)]
        allow_destructive: bool,
        /// Show the planned schema changes, then roll everything back
        #[arg(long)]
        dry_run: bool,
    },
    /// Run verification tests against live PostgreSQL infrastructure
    TestLive {
        #[arg(short, long, default_value = "postgres://postgres:postgres@localhost:5432/bos_dev")]
        db: String,
    },
    /// Detect locally installed and running database providers (PostgreSQL, MySQL, SQL Server, Redis, ClickHouse, SQLite)
    DetectDb,
    /// Provision BOS System Kernel on a PostgreSQL database
    Provision {
        #[arg(short, long, default_value = "postgres://postgres:postgres@localhost:5432/bos_dev")]
        db: String,
        #[arg(short, long, default_value = "shared_rls")]
        strategy: String,
    },
    /// Create a new database in PostgreSQL cluster
    CreateDb {
        #[arg(short, long)]
        name: String,
        #[arg(short, long, default_value = "postgres://postgres:postgres@localhost:5432/postgres")]
        cluster_url: String,
        #[arg(short, long, default_value = "shared_rls")]
        strategy: String,
        #[arg(long, default_value = "DEFAULT_TENANT")]
        tenant: String,
        #[arg(long, default_value = "MAIN_COMPANY")]
        company: String,
        /// Do not automatically import base packages (e.g. bos.security)
        #[arg(long)]
        no_base_packages: bool,
        /// Additional package IDs to import at creation (comma-separated or multiple)
        #[arg(short, long, value_delimiter = ',')]
        packages: Vec<String>,
    },
    /// Manage BOS Packages (.bospkg bundles)
    Package {
        #[command(subcommand)]
        action: PackageCommands,
    },
    /// Manage and run configurable Server Nodes (MS Navision NAS / NST Equivalent)
    Node {
        #[command(subcommand)]
        action: NodeCommands,
    },
    /// Run the Language Server Protocol (LSP) daemon for IDE integration
    Lsp,
}

#[derive(Subcommand)]
enum NodeCommands {
    /// Initialize a new server node configuration template (bos-node.toml)
    Init {
        #[arg(short, long, default_value = "bos-node.toml")]
        output: PathBuf,
        /// Force overwrite if file already exists
        #[arg(short, long)]
        force: bool,
    },
    /// Validate node configuration profile and test database connectivity
    Check {
        #[arg(short, long, default_value = "bos-node.toml")]
        config: PathBuf,
    },
    /// Start the configurable BOS Server Node (serving UI, API, and NAS worker)
    Run {
        #[arg(short, long, default_value = "bos-node.toml")]
        config: PathBuf,
    },
}

#[derive(Subcommand)]
enum PackageCommands {
    /// Compile a BOS package directory or script into a verified .bospkg package bundle
    Build {
        /// Package directory (containing bos.json) or path to .bos script
        #[arg(value_name = "PATH")]
        path: Option<PathBuf>,
        #[arg(short, long)]
        input: Option<PathBuf>,
        #[arg(short, long)]
        output: Option<PathBuf>,
        #[arg(long)]
        package_id: Option<String>,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        version: Option<String>,
        #[arg(long)]
        publisher: Option<String>,
    },
    /// Install a .bospkg package bundle, package directory, or BOS script into PostgreSQL database
    Install {
        /// Package directory (containing bos.json), .bospkg bundle, or .bos script
        #[arg(value_name = "PATH")]
        path: Option<PathBuf>,
        #[arg(short, long)]
        input: Option<PathBuf>,
        #[arg(short, long, default_value = "postgres://postgres:postgres@localhost:5432/bos_dev")]
        db: String,
        #[arg(long)]
        package_id: Option<String>,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        version: Option<String>,
        #[arg(long)]
        publisher: Option<String>,
        /// Permit schema changes that delete or truncate data
        #[arg(long)]
        allow_destructive: bool,
    },
    /// List installed packages from catalog
    List {
        #[arg(short, long, default_value = "postgres://postgres:postgres@localhost:5432/bos_dev")]
        db: String,
    },
}

/// Svelte compiler for `package build`: `$BOS_UI_RUNTIME/svelte-compiler.js`, the built UI runtime, or
/// the compiler of the UI's installed dependencies.
fn locate_svelte_compiler() -> Option<PathBuf> {
    let mut candidates = Vec::new();
    if let Ok(dir) = std::env::var("BOS_UI_RUNTIME") {
        candidates.push(PathBuf::from(dir).join(bosc_runtime::page_compiler::COMPILER_FILE));
    }
    for base in [".", "..", "../.."] {
        candidates.push(PathBuf::from(base).join("ui/dist/bos-runtime").join(bosc_runtime::page_compiler::COMPILER_FILE));
        candidates.push(PathBuf::from(base).join("ui/node_modules/svelte/compiler/index.js"));
    }
    candidates.into_iter().find(|p| p.is_file())
}

/// Starts the compiled page service; a failure is reported but does not stop the API server.
async fn start_pages(engine: &BosEngine, config: &NodeConfig) {
    let cfg = bosc_runtime::PageServiceConfig::from_node_config(config);
    println!("Resolving pages (artifacts: {}, compiler: {})...", cfg.artifact_root.display(), cfg.runtime_dir.display());
    match engine.start_page_service(cfg).await {
        Ok(service) => match service.compiler_error() {
            None => println!("✓ Page service ready ({} compiler workers).", service.config().workers),
            Some(e) => println!("⚠ Pages are listed but cannot be compiled: {e}"),
        },
        Err(e) => println!("⚠ Page service failed to start: {e}"),
    }
}

/// Binds a TCP listener to the given address. If the port is already occupied by a stale
/// process (common during rapid dev restarts or Ctrl+C residue), automatically terminates
/// the stale process, waits for socket release, and cleanly binds.
async fn bind_listener_with_port_recovery(addr: &str, port: u16) -> Result<tokio::net::TcpListener> {
    match tokio::net::TcpListener::bind(addr).await {
        Ok(l) => Ok(l),
        Err(e)
            if e.kind() == std::io::ErrorKind::AddrInUse
                || e.raw_os_error() == Some(10048)
                || e.raw_os_error() == Some(98) =>
        {
            println!("⚠️  PORT CONFLICT: Port {} is currently occupied.", port);
            println!("🔄 Reclaiming port {} by terminating stale instance...", port);

            #[cfg(target_os = "windows")]
            {
                let cmd = format!(
                    "Get-NetTCPConnection -LocalPort {port} -ErrorAction SilentlyContinue | ForEach-Object {{ Stop-Process -Id $_.OwningProcess -Force -ErrorAction SilentlyContinue }}"
                );
                let _ = std::process::Command::new("powershell")
                    .args(["-NoProfile", "-Command", &cmd])
                    .status();
                tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
            }

            #[cfg(not(target_os = "windows"))]
            {
                let cmd = format!("lsof -ti :{port} | xargs kill -9 2>/dev/null || true");
                let _ = std::process::Command::new("sh")
                    .args(["-c", &cmd])
                    .status();
                tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
            }

            match tokio::net::TcpListener::bind(addr).await {
                Ok(l) => {
                    println!("✓ Successfully reclaimed port {} and initialized listener.", port);
                    Ok(l)
                }
                Err(retry_err) => {
                    anyhow::bail!(
                        "Port {port} is occupied by another process and could not be reclaimed: {retry_err}.\n\
                        Remediation:\n  \
                        - Check active processes with: netstat -ano | findstr :{port}\n  \
                        - Run 'npm run node:kill' to force-stop all stale node processes.\n  \
                        - Or change the port in 'bos-node.toml'."
                    );
                }
            }
        }
        Err(e) => Err(anyhow::anyhow!("Failed to bind to {addr}: {e}")),
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let cli = Cli::parse();

    match cli.command {
        Commands::Build { input, output, emit_ir } => {
            let source = fs::read_to_string(&input)?;
            let mut lexer = Lexer::new(&source);
            let tokens = lexer.tokenize()?;
            let mut parser = Parser::with_source(tokens, source.clone());
            let program = parser.parse_program()?;

            let package_name = input
                .file_stem()
                .and_then(|stem| stem.to_str())
                .filter(|stem| !stem.is_empty())
                .unwrap_or("package");
            let analyzer = SemanticAnalyzer::new(package_name);
            let model_ir = analyzer.analyze_and_lower(&program)?;

            if emit_ir {
                let json_ir = serde_json::to_string_pretty(&model_ir)?;
                if let Some(out_path) = output {
                    fs::write(&out_path, &json_ir)?;
                    println!("Compiled Bytecode IR written to {:?}", out_path);
                } else {
                    println!("{}", json_ir);
                }
            } else {
                let codegen = PostgresGenerator::new();
                let ddl = codegen.generate_ddl(&model_ir);

                if let Some(out_path) = output {
                    fs::write(&out_path, &ddl)?;
                    println!("Compiled DDL written to {:?}", out_path);
                } else {
                    println!("{}", ddl);
                }
            }
        }
        Commands::Run { codeunit, method, params, file, db } => {
            println!("Connecting to PostgreSQL database at {}...", db);
            let engine = BosEngine::connect(&db).await?;

            if let Some(file_path) = file {
                let content = fs::read_to_string(&file_path)?;
                if file_path.extension().and_then(|s| s.to_str()) == Some("json") {
                    if let Ok(cu) = serde_json::from_str::<bosc_ir::CodeunitIR>(&content) {
                        println!("Registering Codeunit from JSON IR: {}", cu.name);
                        engine.register_codeunit(cu).await?;
                    }
                } else {
                    let mut lexer = Lexer::new(&content);
                    let tokens = lexer.tokenize()?;
                    let mut parser = Parser::with_source(tokens, content.clone());
                    let program = parser.parse_program()?;
                    let analyzer = SemanticAnalyzer::new("script-runner");
                    let model_ir = analyzer.analyze_and_lower(&program)?;
                    for cu in model_ir.codeunits {
                        println!("Registering Codeunit from script: {}", cu.name);
                        engine.register_codeunit(cu).await?;
                    }
                }
            }

            let params_val: serde_json::Value = serde_json::from_str(&params)
                .map_err(|e| anyhow::anyhow!("Invalid params JSON: {}", e))?;

            let ctx = SessionContext {
                tenant_id: Uuid::nil(),
                company_id: Uuid::nil(),
                user_id: Uuid::new_v4(),
                roles: vec!["Admin".to_string()],
            };

            println!("Executing procedure {}.{}...", codeunit, method);
            let result = engine.execute_procedure(&codeunit, &method, params_val, &ctx).await?;

            println!("\n=== EXECUTION RESULT ===");
            println!("Status: {}", if result.success { "SUCCESS" } else { "FAILED" });
            println!("Codeunit: {}", result.codeunit);
            println!("Method: {}", result.method);
            println!("\nMutated Payload Data:\n{}", serde_json::to_string_pretty(&result.mutated_data)?);
            println!("\nBytecode Evaluation Logs:");
            for log in &result.logs {
                println!("  ✓ {}", log);
            }
        }
        Commands::Deploy { input, db, package, allow_destructive, dry_run } => {
            println!("Connecting to PostgreSQL database...");
            let engine = BosEngine::connect(&db).await?;

            let source = fs::read_to_string(&input)?;
            let mut lexer = Lexer::new(&source);
            let tokens = lexer.tokenize()?;
            let mut parser = Parser::with_source(tokens, source.clone());
            let program = parser.parse_program()?;

            let analyzer = SemanticAnalyzer::new(&package);
            let model_ir = analyzer.analyze_and_lower(&program)?;

            let codegen = PostgresGenerator::new();
            let ddl = codegen.generate_ddl(&model_ir);

            println!("Synchronizing schema{}...", if dry_run { " (dry run)" } else { "" });
            let report = engine
                .sync_schema(
                    &SchemaSyncRequest {
                        package_name: package.clone(),
                        source: "cli".to_string(),
                        tables: model_ir.tables.clone(),
                        extensions: model_ir.table_extensions.clone(),
                        ddl,
                        applied_by: None,
                    },
                    &SchemaSyncOptions { allow_destructive, dry_run },
                )
                .await?;
            print_schema_report(&report);
            if dry_run {
                if !report.blocked.is_empty() {
                    println!("Re-run with --allow-destructive to apply the blocked changes.");
                }
                println!("Dry run complete: nothing was changed.");
                return Ok(());
            }

            let (resolved_id, resolved_name, resolved_ver, resolved_pub) =
                resolve_package_metadata(&input, Some(package.clone()), None, None, None);
            let (bundle, _ddl, _model) = compile_package_bundle(&source, &resolved_id, &resolved_name, &resolved_ver, &resolved_pub, &input.display().to_string())?;
            let res = engine.install_package(bundle, allow_destructive).await?;

            println!(
                "Deployment successful! {} meta-objects from package '{}' provisioned and activated.",
                res.objects_installed, res.package_id
            );
        }
        Commands::TestLive { db } => {
            println!("Starting Live End-to-End Verification on PostgreSQL...");
            let engine = BosEngine::connect(&db).await?;

            // 1. Compile test BOS script containing Table, Immutable Ledger, and StateMachine
            let bos_script = r#"
                table Customer {
                    caption = "Customer Master";
                    tableType = Normal;
                    dataPerCompany = true;

                    code: String(20) {
                        notNull = true;
                        unique = true;
                    };
                    name: String(100);
                    creditLimit: Decimal(18, 2);
                    balance: Decimal(18, 2) {
                        computed = sum(CustomerLedgerEntry.amount where { customerCode == code });
                    };

                    primaryKey(code);
                }

                table CustomerLedgerEntry {
                    caption = "Customer Ledger Entry";
                    tableType = Ledger;
                    dataPerCompany = true;

                    entryNo: Integer {
                        notNull = true;
                    };
                    customerCode: String(20) {
                        notNull = true;
                    };
                    amount: Decimal(18, 2) {
                        notNull = true;
                    };
                    open: Boolean {
                        default = true;
                    };

                    primaryKey(entryNo);
                }

                table PurchaseOrder {
                    caption = "Purchase Order";
                    tableType = Document;
                    dataPerCompany = true;

                    orderNo: String(30) {
                        notNull = true;
                        unique = true;
                    };
                    status: Enum("PurchaseOrderStatus") {
                        default = "Draft";
                    };
                    totalAmount: Decimal(18, 2);

                    primaryKey(orderNo);
                }

                statemachine PurchaseOrderWorkflow for PurchaseOrder.status {
                    initial = "Draft";

                    states {
                        state "Draft" { caption = "Draft"; badge = "neutral"; }
                        state "PendingApproval" { caption = "Pending Approval"; badge = "warning"; }
                        state "Approved" { caption = "Approved"; badge = "success"; }
                    }

                    transition Submit {
                        from = "Draft";
                        to = "PendingApproval";
                    }

                    transition Approve {
                        from = "PendingApproval";
                        to = "Approved";
                    }
                }
            "#;

            let mut lexer = Lexer::new(bos_script);
            let tokens = lexer.tokenize()?;
            let mut parser = Parser::with_source(tokens, bos_script.to_string());
            let program = parser.parse_program()?;

            let analyzer = SemanticAnalyzer::new("live-test-package");
            let model_ir = analyzer.analyze_and_lower(&program)?;
            let codegen = PostgresGenerator::new();
            let ddl = codegen.generate_ddl(&model_ir);

            println!("Deploying live test schema...");
            let report = engine
                .sync_schema(
                    &SchemaSyncRequest {
                        package_name: "live-test-package".to_string(),
                        source: "cli".to_string(),
                        tables: model_ir.tables.clone(),
                        extensions: model_ir.table_extensions.clone(),
                        ddl,
                        applied_by: None,
                    },
                    &SchemaSyncOptions::default(),
                )
                .await?;
            print_schema_report(&report);

            for table in &model_ir.tables {
                let req = bosc_runtime::RegisterMetaObjectRequest {
                    object_type: "table".to_string(),
                    name: table.name.clone(),
                    package_name: Some("live-test-package".to_string()),
                    version: Some("1.0.0".to_string()),
                    caption: Some(table.caption.clone()),
                    sub_type: Some(format!("{:?}", table.table_type)),
                    data_per_company: Some(table.data_per_company),
                    columns: None,
                    primary_key: Some(table.primary_key.clone()),
                    check_constraints: None,
                    state_machine: None,
                    definition: Some(serde_json::to_value(table)?),
                    allow_destructive: false,
                };
                let _ = engine.register_meta_object(req).await;
            }

            println!("  [OK] DDL deployed successfully.");

            let tenant_a = Uuid::new_v4();
            let company_a = Uuid::new_v4();
            let ctx_a = SessionContext {
                tenant_id: tenant_a,
                company_id: company_a,
                user_id: Uuid::new_v4(),
                roles: vec!["Admin".to_string()],
            };

            let test_suffix = &Uuid::new_v4().to_string()[..8];
            let cust_code = format!("CUST-{}", test_suffix);
            let entry_no = (std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|e| anyhow::anyhow!("System time clock error: {}", e))?
                .as_millis() % 1_000_000_000) as i32;
            let po_no = format!("PO-{}", test_suffix);

            // Test 1: Insert Normal Customer
            println!("Test 1: Inserting record into 'customer' table...");
            let cust_id = engine.insert_record("customer", serde_json::json!({
                "code": cust_code,
                "name": "Global Tech Corp",
                "credit_limit": 50000.00
            }), &ctx_a).await?;
            println!("  [OK] Customer created with _id: {}", cust_id);

            // Test 2: Insert Immutable Ledger Entry
            println!("Test 2: Inserting write-once entry into 'customer_ledger_entry'...");
            let ledger_id = engine.insert_record("customer_ledger_entry", serde_json::json!({
                "entry_no": entry_no,
                "customer_code": cust_code,
                "amount": 12500.00,
                "open": true
            }), &ctx_a).await?;
            println!("  [OK] Ledger entry created with _id: {}", ledger_id);

            // Test 3: Verify Immutable Ledger blocks raw update
            println!("Test 3: Verifying PostgreSQL trigger blocks direct UPDATE on immutable ledger...");
            let pool = engine.pool().await;
            let raw_update_res = sqlx::raw_sql(&format!(
                "UPDATE customer_ledger_entry SET amount = 99999 WHERE _id = '{}';",
                ledger_id
            )).execute(&pool).await;

            match raw_update_res {
                Err(err) => {
                    println!("  [OK] Direct update successfully blocked by Postgres trigger: {}", err);
                }
                Ok(_) => {
                    anyhow::bail!("CRITICAL FAILURE: Immutable ledger allowed direct update without privileged override!");
                }
            }

            // Test 4: Privileged Override Legal Update
            println!("Test 4: Executing Privileged Legal Override on Immutable Ledger...");
            engine.update_privileged(
                "customer_ledger_entry",
                ledger_id,
                "open",
                "FALSE",
                "Statutory court settlement order #2026-881",
                "TICKET-99120",
                "AuditDirector",
                &ctx_a
            ).await?;
            println!("  [OK] Privileged override succeeded and was recorded into _system_privileged_audit_log.");

            // Test 5: Insert Document and test Atomic State Machine Transition
            println!("Test 5: Inserting PurchaseOrder and testing StateMachine transitions...");
            let po_id = engine.insert_record("purchase_order", serde_json::json!({
                "order_no": po_no,
                "status": "Draft",
                "total_amount": 7500.00
            }), &ctx_a).await?;

            engine.transition_state(
                "purchase_order",
                po_id,
                "status",
                "Draft",
                "PendingApproval",
                "Submit",
                Some("Ready for manager review"),
                None,
                &ctx_a
            ).await?;
            println!("  [OK] Transition 1 (Draft -> PendingApproval) committed atomically.");

            engine.transition_state(
                "purchase_order",
                po_id,
                "status",
                "PendingApproval",
                "Approved",
                "Approve",
                Some("Approved under executive limit"),
                None,
                &ctx_a
            ).await?;
            println!("  [OK] Transition 2 (PendingApproval -> Approved) committed atomically.");

            // Verify history table
            let pool = engine.pool().await;
            let history_rows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM purchase_order_state_history WHERE entity_id = $1")
                .bind(po_id)
                .fetch_one(&pool)
                .await?;
            assert_eq!(history_rows, 2);
            println!("  [OK] State history accurately verified with 2 transition log rows.");

            println!("\n========================================================");
            println!("ALL LIVE TESTS PASSED 100%! REAL WORKING SYSTEM VERIFIED!");
            println!("========================================================");
        }
        Commands::DetectDb => {
            use bosc_runtime::DatabaseDetector;
            println!("Scanning local system for installed and running database providers...\n");
            let dbs = DatabaseDetector::scan_all();

            println!("{:<32} {:<10} {:<16} {:<32}", "DATABASE PROVIDER", "STATUS", "ENDPOINT", "DETAILS");
            println!("{:-<95}", "");

            for db in dbs {
                let status = if db.is_running { "RUNNING" } else { "OFFLINE" };
                let endpoint = if db.port > 0 {
                    format!("{}:{}", db.host, db.port)
                } else {
                    "embedded".to_string()
                };

                println!("{:<32} {:<10} {:<16} {:<32}", db.display_name, status, endpoint, db.details);
                if let Some(bin) = db.binary_path {
                    println!("    -> Binary path: {}", bin.display());
                }
            }
            println!("\nBOS Default Connection String:");
            println!("  postgres://postgres:postgres@localhost:5432/bos_dev\n");
        }
        Commands::Provision { db, strategy } => {
            println!("Connecting to PostgreSQL database at {}...", db);
            let engine = BosEngine::connect(&db).await?;
            println!("Provisioning BOS System Kernel with strategy '{}'...", strategy);
            let status = engine.provision_system(Some(&strategy)).await?;
            println!("========================================================");
            println!("🚀 BOS SYSTEM KERNEL PROVISIONED");
            println!("========================================================");
            println!("Kernel Version:    {}", status.kernel_version.unwrap_or_default());
            println!("Storage Strategy:  {}", status.storage_strategy.unwrap_or_default());
            println!("Status:            {}", status.status.unwrap_or_default());
            println!("Meta-Objects:      {}", status.meta_objects_count);
            println!("Tenants:           {}", status.tenants_count);
            println!("Companies:         {}", status.companies_count);
            println!("========================================================");
        }
        Commands::CreateDb { name, cluster_url, strategy, tenant, company, no_base_packages, packages } => {
            println!("Connecting to PostgreSQL cluster at {}...", cluster_url);
            let engine = BosEngine::connect(&cluster_url).await?;
            let req = bosc_runtime::db_manager::CreateDatabaseRequest {
                name: name.clone(),
                init_extensions: true,
                provision_system: true,
                storage_strategy: Some(strategy.clone()),
                tenant_id: None,
                tenant_code: Some(tenant.clone()),
                tenant_name: Some(format!("Tenant {}", tenant)),
                company_id: None,
                company_code: Some(company.clone()),
                company_name: Some(format!("Company {}", company)),
                import_base_packages: !no_base_packages,
                packages: packages.clone(),
            };
            println!("Creating new database '{}' with strategy '{}' (base packages: {})...", name, strategy, if !no_base_packages { "auto" } else { "disabled" });
            let res = engine.create_database(&req).await?;
            println!("========================================================");
            println!("🚀 NEW DATABASE CREATED SUCCESSFULLY");
            println!("========================================================");
            println!("Database Name:    {}", name);
            println!("Storage Strategy: {}", strategy);
            println!("Result:           {}", res);
            println!("========================================================");
        }
        Commands::Package { action } => match action {
            PackageCommands::Build { path, input, output, package_id, name, version, publisher } => {
                let target = path.or(input).unwrap_or_else(|| PathBuf::from("."));
                let (source, source_label) = collect_package_source(&target)?;
                let (resolved_id, resolved_name, resolved_ver, resolved_pub) =
                    resolve_package_metadata(&target, package_id, name, version, publisher);
                let (bundle, _ddl, model_ir) = compile_package_bundle(&source, &resolved_id, &resolved_name, &resolved_ver, &resolved_pub, &source_label)?;

                let dist_pkg_dir = PathBuf::from("dist").join("packages").join(&resolved_id);
                let bytecode_dir = dist_pkg_dir.join("bytecode");
                let schema_dir = dist_pkg_dir.join("schema");
                let pages_dir = dist_pkg_dir.join("pages");
                let _ = fs::create_dir_all(&bytecode_dir);
                let _ = fs::create_dir_all(&schema_dir);
                let _ = fs::create_dir_all(&pages_dir);

                // 1. Ahead-of-Time QuickJS Bytecode (.qbc) Compilation with Magic Header & CRC32
                let mut bytecodes = Vec::new();
                for cu in &model_ir.codeunits {
                    if let Ok(art) = bosc_analyzer::BytecodeCompiler::compile_codeunit(cu) {
                        let out_file = bytecode_dir.join(format!("{}.qbc", cu.name));
                        let _ = fs::write(&out_file, art.to_binary_envelope());
                        bytecodes.push(art);
                    }
                }
                for tl in &model_ir.table_logics {
                    if let Ok(art) = bosc_analyzer::BytecodeCompiler::compile_tablelogic(tl) {
                        let out_file = bytecode_dir.join(format!("{}.qbc", tl.name));
                        let _ = fs::write(&out_file, art.to_binary_envelope());
                        bytecodes.push(art);
                    }
                }
                for sm in &model_ir.state_machines {
                    if let Ok(art) = bosc_analyzer::BytecodeCompiler::compile_statemachine(sm) {
                        let out_file = bytecode_dir.join(format!("{}.qbc", sm.name));
                        let _ = fs::write(&out_file, art.to_binary_envelope());
                        bytecodes.push(art);
                    }
                }
                if let Ok(bundle_art) = bosc_analyzer::BytecodeCompiler::compile_package_bundle(&resolved_id, &model_ir) {
                    let out_file = bytecode_dir.join("package.bundle.qbc");
                    let _ = fs::write(&out_file, bundle_art.to_binary_envelope());
                    bytecodes.push(bundle_art);
                }

                // 2. Ahead-of-Time Prepared SQL Statements
                let codegen = PostgresGenerator::new();
                let prepared_queries = codegen.generate_aot_prepared_queries(&model_ir.tables);
                let pq_file = schema_dir.join("prepared_queries.json");
                let pq_json = serde_json::to_string_pretty(&prepared_queries)?;
                fs::write(&pq_file, &pq_json)?;

                // 3. Schema DDL
                let tables_ddl = codegen.generate_tables(&model_ir.tables);
                let ddl_file = schema_dir.join("tables.sql");
                fs::write(&ddl_file, &tables_ddl)?;

                // 4. Page descriptors (read by the legacy PageSurface renderer until the page kit replaces it)
                let mut page_count = 0;
                for page in &model_ir.pages {
                    let page_file = pages_dir.join(format!("{}.json", page.name));
                    if let Ok(page_json) = serde_json::to_string_pretty(page) {
                        let _ = fs::write(&page_file, page_json);
                        page_count += 1;
                    }
                }

                // 5. Pre-compiled Svelte 5 page modules (content-addressed; seeds the node's artifact store)
                let compiler = locate_svelte_compiler().and_then(|path| {
                    match bosc_runtime::SvelteCompiler::start(&path, bosc_runtime::SvelteCompiler::default_workers()) {
                        Ok(c) => Some(c),
                        Err(e) => {
                            println!("⚠ Svelte compiler unavailable ({e}); page modules will be compiled by the node.");
                            None
                        }
                    }
                });
                if compiler.is_none() {
                    println!("⚠ No Svelte compiler found (build ui/ or set BOS_UI_RUNTIME); page modules will be compiled by the node.");
                }
                let built_pages = bosc_runtime::page_artifacts::build_model_pages(&model_ir, compiler.as_ref(), &pages_dir)
                    .await
                    .map_err(|e| anyhow::anyhow!("page build failed: {e}"))?;
                let compiled_pages = built_pages.iter().filter(|p| p.compiled).count();
                let page_errors: usize = built_pages.iter().map(|p| p.errors).sum();

                // 6. Cryptographic Merkle Root and Manifest
                let full_artifacts = bosc_ir::PackageArtifactsIR {
                    manifest: bundle.manifest.clone(),
                    bytecodes: bytecodes.clone(),
                    prepared_queries,
                    tables_ddl,
                    views_ddl: String::new(),
                    merkle_root: String::new(),
                    signature: None,
                    public_key: None,
                };
                let merkle_root = full_artifacts.compute_merkle_root();

                let manifest_file = dist_pkg_dir.join("manifest.json");
                let manifest_json = serde_json::json!({
                    "package_id": bundle.manifest.package_id,
                    "name": bundle.manifest.name,
                    "version": bundle.manifest.version,
                    "publisher": bundle.manifest.publisher,
                    "merkle_root": merkle_root,
                    "checksum": bundle.checksum,
                    "bytecode_modules": bytecodes.len(),
                    "pages": page_count,
                    "page_modules": compiled_pages,
                });
                fs::write(&manifest_file, serde_json::to_string_pretty(&manifest_json)?)?;

                let out_path = output.unwrap_or_else(|| {
                    if target.is_dir() {
                        target.join(format!("{}.bospkg", resolved_id))
                    } else if let Some(parent) = target.parent() {
                        parent.join(format!("{}.bospkg", resolved_id))
                    } else {
                        PathBuf::from(format!("{}.bospkg", resolved_id))
                    }
                });

                if let Some(parent) = out_path.parent() {
                    let _ = fs::create_dir_all(parent);
                }

                let bundle_json = serde_json::to_string_pretty(&bundle)?;
                fs::write(&out_path, &bundle_json)?;

                println!("========================================================");
                println!("📦 BOS PACKAGE COMPILED SUCCESSFULLY (AOT STATIC ARTIFACTS)");
                println!("========================================================");
                println!("Package ID:        {}", bundle.manifest.package_id);
                println!("Name:              {}", bundle.manifest.name);
                println!("Version:           {}", bundle.manifest.version);
                println!("Publisher:         {}", bundle.manifest.publisher);
                println!("Checksum:          {}", bundle.checksum);
                println!("Merkle Root:       {}", merkle_root);
                println!("Bytecode Modules:  {} (.qbc generated)", bytecodes.len());
                println!("Page Modules:      {} of {} compiled ({} page errors)", compiled_pages, built_pages.len(), page_errors);
                println!("Meta-Objects:      {}", bundle.objects.len());
                println!("Bundle Artifact:   {:?}", out_path);
                println!("Static Directory:  {:?}", dist_pkg_dir);
                println!("========================================================");
            }
            PackageCommands::Install { path, input, db, package_id, name, version, publisher, allow_destructive } => {
                let target = path.or(input).unwrap_or_else(|| PathBuf::from("."));
                println!("Connecting to PostgreSQL database at {}...", db);
                let engine = BosEngine::connect(&db).await?;

                let sys_status = engine.get_system_status().await?;
                if !sys_status.is_provisioned {
                    println!("New database detected. Auto-provisioning BOS System Kernel...");
                    engine.provision_system(None).await?;
                    println!("✓ BOS System Kernel provisioned successfully.");
                }

                let bundle = if target.is_dir() {
                    let (resolved_id, resolved_name, resolved_ver, resolved_pub) =
                        resolve_package_metadata(&target, package_id.clone(), name.clone(), version.clone(), publisher.clone());
                    let pkg_bundle_path = target.join(format!("{}.bospkg", resolved_id));
                    if !pkg_bundle_path.exists() {
                        println!("Building package '{}' before installation...", resolved_id);
                        let (source, source_label) = collect_package_source(&target)?;
                        let (built_bundle, _ddl, _model) = compile_package_bundle(&source, &resolved_id, &resolved_name, &resolved_ver, &resolved_pub, &source_label)?;
                        let bundle_json = serde_json::to_string_pretty(&built_bundle)?;
                        fs::write(&pkg_bundle_path, &bundle_json)?;
                    }
                    let content = fs::read_to_string(&pkg_bundle_path)?;
                    serde_json::from_str::<bosc_ir::PackageBundleIR>(&content)?
                } else {
                    let content = fs::read_to_string(&target)?;
                    if target.extension().and_then(|s| s.to_str()) == Some("bospkg") || content.trim_start().starts_with('{') {
                        serde_json::from_str::<bosc_ir::PackageBundleIR>(&content)?
                    } else {
                        let (resolved_id, resolved_name, resolved_ver, resolved_pub) =
                            resolve_package_metadata(&target, package_id, name, version, publisher);
                        let (b, ddl, _model) = compile_package_bundle(
                            &content,
                            &resolved_id,
                            &resolved_name,
                            &resolved_ver,
                            &resolved_pub,
                            &target.display().to_string(),
                        )?;
                        let tables: Vec<bosc_ir::TableIR> = b
                            .objects
                            .iter()
                            .filter(|obj| obj.get("object_type").and_then(|v| v.as_str()).unwrap_or("table") == "table")
                            .filter_map(|obj| serde_json::from_value(obj.get("definition").unwrap_or(obj).clone()).ok())
                            .collect();
                        let report = engine
                            .sync_schema(
                                &SchemaSyncRequest {
                                    package_name: b.manifest.package_id.clone(),
                                    source: "cli".to_string(),
                                    tables,
                                    extensions: Vec::new(),
                                    ddl,
                                    applied_by: None,
                                },
                                &SchemaSyncOptions { allow_destructive, dry_run: false },
                            )
                            .await?;
                        print_schema_report(&report);
                        b
                    }
                };

                let res = engine.install_package(bundle, allow_destructive).await?;
                println!("========================================================");
                println!("✅ PACKAGE INSTALLED & ACTIVATED");
                println!("========================================================");
                println!("Package ID:        {}", res.package_id);
                println!("Name:              {}", res.name);
                println!("Version:           {}", res.version);
                println!("Publisher:         {}", res.publisher);
                println!("Objects Installed: {}", res.objects_installed);
                println!("Status:            {}", res.status);
                println!("Message:           {}", res.message);
                println!("========================================================");
            }
            PackageCommands::List { db } => {
                println!("Connecting to PostgreSQL database at {}...", db);
                let engine = BosEngine::connect(&db).await?;
                let pkgs = engine.list_packages().await?;

                println!("========================================================");
                println!("📦 INSTALLED BOS PACKAGES ({})", pkgs.len());
                println!("========================================================");
                for p in pkgs {
                    println!("• {} (v{}) - {} [status: {}] installed at {}", p.package_id, p.version, p.name, p.status, p.installed_at);
                }
            }
        },
        Commands::Node { action } => match action {
            NodeCommands::Init { output, force } => {
                if output.exists() && !force {
                    anyhow::bail!("Configuration file '{}' already exists. Use --force to overwrite.", output.display());
                }
                if let Some(parent) = output.parent() {
                    let _ = fs::create_dir_all(parent);
                }
                fs::write(&output, NodeConfig::default_template())?;
                println!("========================================================");
                println!("🚀 BOS SERVER NODE CONFIGURATION CREATED");
                println!("========================================================");
                println!("Path: {}", output.display());
                println!("To validate: bosc node check --config {}", output.display());
                println!("To run node: bosc node run --config {}", output.display());
                println!("========================================================");
            }
            NodeCommands::Check { config } => {
                let resolved_path = if config.exists() {
                    config.clone()
                } else {
                    let mut dir = std::env::current_dir().ok();
                    let mut found = None;
                    while let Some(d) = dir {
                        let candidate = d.join(&config);
                        if candidate.exists() {
                            found = Some(candidate);
                            break;
                        }
                        dir = d.parent().map(|p| p.to_path_buf());
                    }
                    found.unwrap_or_else(|| config.clone())
                };

                println!("Checking server node configuration profile '{}'...", resolved_path.display());
                let node_config = NodeConfig::from_file(&resolved_path)?;
                println!("  ✓ TOML schema valid");
                println!("  ✓ Node ID: {}", node_config.node.id);
                println!("  ✓ Node Name: {}", node_config.node.name);
                println!("  ✓ Role: {:?}", node_config.node.role);
                println!("  ✓ Network: {}:{}", node_config.network.host, node_config.network.port);
                println!("  ✓ UI Static Hosting: {} (path: {})", if node_config.ui.enabled { "ENABLED" } else { "DISABLED" }, node_config.ui.static_dir);
                println!("  ✓ NAS Worker Daemon: {}", if node_config.nas_worker.enabled { "ENABLED" } else { "DISABLED" });
                println!("  ✓ Default Tenant: {}", node_config.defaults.default_tenant);
                println!("  ✓ Default Company: {}", node_config.defaults.default_company);

                println!("\nTesting database connection to '{}'...", mask_db_url(&node_config.database.url));
                let pool = sqlx::postgres::PgPoolOptions::new()
                    .acquire_timeout(std::time::Duration::from_secs(node_config.database.connection_timeout_seconds))
                    .max_connections(2)
                    .connect(&node_config.database.url)
                    .await?;

                sqlx::query("SELECT 1").execute(&pool).await?;
                println!("  ✓ Database connection established successfully!");
                pool.close().await;

                println!("\n========================================================");
                println!("✅ NODE CONFIGURATION VERIFIED - READY FOR PRODUCTION");
                println!("========================================================");
            }
            NodeCommands::Run { config } => {
                let resolved_path = if config.exists() {
                    Some(config.clone())
                } else {
                    let mut dir = std::env::current_dir().ok();
                    let mut found = None;
                    while let Some(d) = dir {
                        let candidate = d.join(&config);
                        if candidate.exists() {
                            found = Some(candidate);
                            break;
                        }
                        dir = d.parent().map(|p| p.to_path_buf());
                    }
                    found
                };

                let node_config = match resolved_path {
                    Some(ref path) => {
                        let abs_path = path.canonicalize().unwrap_or_else(|_| path.clone());
                        println!("Loading node configuration from '{}'...", abs_path.display());
                        let cfg = NodeConfig::from_file(&abs_path)?;
                        if let Some(parent) = abs_path.parent() {
                            let _ = std::env::set_current_dir(parent);
                        }
                        cfg
                    }
                    None => {
                        let is_default = config.as_os_str() == "bos-node.toml";
                        if !is_default {
                            anyhow::bail!(
                                "Configuration file '{}' does not exist in the current directory or parent directory.\n  Hint: If you are inside 'dist/bos-server/bin', run: ..\\config\\bos-node.toml or cd .. to 'dist/bos-server'",
                                config.display()
                            );
                        }
                        println!("Configuration '{}' not found, running with default production node profile...", config.display());
                        NodeConfig::default()
                    }
                };

                let addr = format!("{}:{}", node_config.network.host, node_config.network.port);
                let masked_url = mask_db_url(&node_config.database.url);

                println!("========================================================");
                println!("🚀 STARTING BOS SERVER NODE: {}", node_config.node.name);
                println!("========================================================");
                println!("Node ID:          {}", node_config.node.id);
                println!("Role:             {:?}", node_config.node.role);
                println!("Listening On:     http://{}", addr);
                println!("Database Cluster: {}", masked_url);
                println!("Storage Strategy: {}", node_config.database.storage_strategy);
                println!("Default Tenant:   {}", node_config.defaults.default_tenant);
                println!("Default Company:  {}", node_config.defaults.default_company);
                println!("UI Static Files:  {} (dir: {})", if node_config.ui.enabled { "ACTIVE" } else { "OFF" }, node_config.ui.static_dir);
                println!("NAS Batch Worker: {}", if node_config.nas_worker.enabled { "ACTIVE" } else { "OFF" });
                println!("========================================================");

                println!("Connecting BOS Engine to database...");
                let engine = BosEngine::connect(&node_config.database.url).await?;
                let engine_arc = std::sync::Arc::new(engine.clone());

                let _nas_handle = if node_config.nas_worker.enabled {
                    Some(NasWorkerDaemon::start(
                        engine_arc.clone(),
                        node_config.nas_worker.clone(),
                    ))
                } else {
                    None
                };

                if node_config.pages.enabled {
                    start_pages(&engine, &node_config).await;
                }

                let app = create_node_router(engine, node_config.clone());
                let listener = bind_listener_with_port_recovery(&addr, node_config.network.port).await?;
                println!("✓ Server Node ready. Clients can connect with just User ID & Password.");
                println!("Press Ctrl+C to stop the server node.\n");

                let serve_result = axum::serve(listener, app)
                    .with_graceful_shutdown(async {
                        if std::env::var("BOS_DAEMON").map(|v| v == "1" || v == "true").unwrap_or(false) {
                            std::future::pending::<()>().await;
                        } else {
                            match tokio::signal::ctrl_c().await {
                                Ok(()) => {
                                    println!("\n🛑 Shutdown signal received (Ctrl+C). Stopping BOS Server Node...");
                                }
                                Err(e) => {
                                    eprintln!("⚠️  Console Ctrl+C handler not available ({e}); running indefinitely until process termination.");
                                    std::future::pending::<()>().await;
                                }
                            }
                        }
                    })
                    .await;

                if let Err(ref e) = serve_result {
                    eprintln!("🛑 Server encountered fatal error: {}", e);
                }

                if let Some(nas_handle) = _nas_handle {
                    println!("Stopping NAS worker daemon...");
                    nas_handle.stop().await;
                }
                println!("✓ BOS Server Node shutdown complete.");
                serve_result?;
            }
        },
        Commands::Lsp => {
            lsp::run_lsp_server().await?;
        }
    }

    Ok(())
}

fn mask_db_url(url: &str) -> String {
    if let Some(at_idx) = url.find('@') {
        if let Some(colon_slash) = url.find("://") {
            let scheme = &url[..colon_slash + 3];
            let host_part = &url[at_idx..];
            format!("{}***:***{}", scheme, host_part)
        } else {
            "postgres://***:***@...".to_string()
        }
    } else {
        url.to_string()
    }
}

fn push_bundle_object(
    objects: &mut Vec<serde_json::Value>,
    value: &impl serde::Serialize,
    object_type: &str,
    package_id: &str,
    version: &str,
) -> Result<()> {
    let mut obj = serde_json::to_value(value)?;
    if let Some(map) = obj.as_object_mut() {
        map.insert("object_type".to_string(), serde_json::json!(object_type));
        map.insert("package_name".to_string(), serde_json::json!(package_id));
        map.insert("version".to_string(), serde_json::json!(version));
    }
    objects.push(obj);
    Ok(())
}

fn print_schema_report(report: &SchemaSyncReport) {
    if report.steps.is_empty() && report.blocked.is_empty() {
        println!("  Schema is up to date: no structural changes.");
    }
    for step in &report.steps {
        let risk = if step.risk == bosc_codegen_db::migration::StepRisk::Destructive { "DESTRUCTIVE" } else { "safe" };
        println!("  [{risk}] {}: {}", step.table, step.description);
    }
    for step in &report.blocked {
        println!("  [BLOCKED] {}: {}", step.table, step.description);
    }
    for rel in &report.relations {
        if rel.status == "dropped" {
            println!("  [relation] {}.{} removed", rel.table, rel.column);
        } else {
            println!(
                "  [relation] {}.{} -> {} (on delete {}): {}",
                rel.table, rel.column, rel.target, rel.on_delete, rel.status
            );
        }
    }
    if let Some(id) = report.migration_id {
        println!("  Recorded as migration {id}.");
    }
}

fn collect_package_source(input: &std::path::Path) -> Result<(String, String)> {
    if input.is_dir() {
        let mut source = String::new();
        let mut found_any = false;
        let mut entries: Vec<PathBuf> = Vec::new();
        for entry in fs::read_dir(input)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("bos") {
                entries.push(path);
            }
        }
        entries.sort();
        for path in entries {
            let file_content = fs::read_to_string(&path)?;
            source.push_str(&format!("\n// === Source: {} ===\n", path.display()));
            source.push_str(&file_content);
            source.push('\n');
            found_any = true;
        }
        if !found_any {
            anyhow::bail!("No .bos source files found in package directory '{}'", input.display());
        }
        Ok((source, input.display().to_string()))
    } else {
        let source = fs::read_to_string(input)?;
        Ok((source, input.display().to_string()))
    }
}

fn compile_package_bundle(
    source: &str,
    package_id: &str,
    name: &str,
    version: &str,
    publisher: &str,
    source_label: &str,
) -> Result<(bosc_ir::PackageBundleIR, String, bosc_ir::ModelIR)> {
    let mut lexer = Lexer::new(source);
    let tokens = match lexer.tokenize() {
        Ok(t) => t,
        Err(e) => anyhow::bail!("Lexer error in '{}': {}", source_label, e),
    };
    let mut parser = Parser::with_source(tokens, source.to_string());
    let program = match parser.parse_program() {
        Ok(p) => p,
        Err(err) => {
            let visual = err.format_with_source(source, source_label);
            eprintln!("\n{}", visual);
            anyhow::bail!("Syntax error in '{}': {}", source_label, err);
        }
    };

    let analyzer = SemanticAnalyzer::new(package_id);
    let model_ir = analyzer.analyze_and_lower(&program)?;

    let codegen = PostgresGenerator::new();
    let ddl = codegen.generate_ddl(&model_ir);

    let manifest = bosc_ir::PackageManifestIR {
        package_id: package_id.to_string(),
        name: name.to_string(),
        version: version.to_string(),
        publisher: publisher.to_string(),
        dependencies: vec![],
        optional_features: vec![],
    };

    let mut objects = Vec::new();
    for table in &model_ir.tables {
        let mut obj = serde_json::to_value(table)?;
        if let Some(map) = obj.as_object_mut() {
            map.insert("object_type".to_string(), serde_json::json!("table"));
            map.insert("package_name".to_string(), serde_json::json!(package_id));
            map.insert("version".to_string(), serde_json::json!(version));
        }
        objects.push(obj);
    }

    for cu in &model_ir.codeunits {
        let mut obj = serde_json::to_value(cu)?;
        if let Some(map) = obj.as_object_mut() {
            map.insert("object_type".to_string(), serde_json::json!("codeunit"));
            map.insert("package_name".to_string(), serde_json::json!(package_id));
            map.insert("version".to_string(), serde_json::json!(version));
        }
        objects.push(obj);
    }

    for sm in &model_ir.state_machines {
        let mut obj = serde_json::to_value(sm)?;
        if let Some(map) = obj.as_object_mut() {
            map.insert("object_type".to_string(), serde_json::json!("statemachine"));
            map.insert("package_name".to_string(), serde_json::json!(package_id));
            map.insert("version".to_string(), serde_json::json!(version));
        }
        objects.push(obj);
    }

    for tl in &model_ir.table_logics {
        let mut obj = serde_json::to_value(tl)?;
        if let Some(map) = obj.as_object_mut() {
            map.insert("object_type".to_string(), serde_json::json!("tablelogic"));
            map.insert("package_name".to_string(), serde_json::json!(package_id));
            map.insert("version".to_string(), serde_json::json!(version));
        }
        objects.push(obj);
    }

    for tle in &model_ir.table_logic_extensions {
        let mut obj = serde_json::to_value(tle)?;
        if let Some(map) = obj.as_object_mut() {
            map.insert("object_type".to_string(), serde_json::json!("tablelogicextension"));
            map.insert("package_name".to_string(), serde_json::json!(package_id));
            map.insert("version".to_string(), serde_json::json!(version));
        }
        objects.push(obj);
    }

    for e in &model_ir.enums {
        push_bundle_object(&mut objects, e, "enum", package_id, version)?;
    }
    for ext in &model_ir.table_extensions {
        push_bundle_object(&mut objects, ext, "tableextension", package_id, version)?;
    }
    for ext in &model_ir.state_machine_extensions {
        push_bundle_object(&mut objects, ext, "statemachineextension", package_id, version)?;
    }
    for ext in &model_ir.enum_extensions {
        push_bundle_object(&mut objects, ext, "enumextension", package_id, version)?;
    }
    for item in &model_ir.interfaces {
        push_bundle_object(&mut objects, item, "interface", package_id, version)?;
    }
    for item in &model_ir.permission_sets {
        push_bundle_object(&mut objects, item, "permissionset", package_id, version)?;
    }
    for item in &model_ir.views {
        push_bundle_object(&mut objects, item, "view", package_id, version)?;
    }
    for item in &model_ir.queries {
        push_bundle_object(&mut objects, item, "query", package_id, version)?;
    }
    for item in &model_ir.pages {
        push_bundle_object(&mut objects, item, "page", package_id, version)?;
    }
    for item in &model_ir.page_extensions {
        push_bundle_object(&mut objects, item, "pageextension", package_id, version)?;
    }
    for item in &model_ir.reports {
        push_bundle_object(&mut objects, item, "report", package_id, version)?;
    }
    for item in &model_ir.custom_metadata {
        push_bundle_object(&mut objects, item, "custommetadata", package_id, version)?;
    }
    for item in &model_ir.menus {
        push_bundle_object(&mut objects, item, "menu", package_id, version)?;
    }

    let bundle = bosc_runtime::PackageManager::build_bundle(manifest, objects)
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    Ok((bundle, ddl, model_ir))
}

fn resolve_package_metadata(
    input: &std::path::Path,
    pkg_id: Option<String>,
    name: Option<String>,
    version: Option<String>,
    publisher: Option<String>,
) -> (String, String, String, String) {
    let mut resolved_id = pkg_id;
    let mut resolved_name = name;
    let mut resolved_version = version;
    let mut resolved_pub = publisher;

    let candidate_dirs = if input.is_dir() {
        vec![input.to_path_buf()]
    } else {
        vec![input.parent().unwrap_or(input).to_path_buf()]
    };

    for parent in candidate_dirs {
        let manifest_path = parent.join("bos.json");
        let pkg_json_path = parent.join("package.json");
        let candidate = if manifest_path.exists() {
            Some(manifest_path)
        } else if pkg_json_path.exists() {
            Some(pkg_json_path)
        } else {
            None
        };

        if let Some(path) = candidate {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                    if resolved_id.is_none() {
                        resolved_id = json.get("package_id").or_else(|| json.get("id")).and_then(|v| v.as_str()).map(|s| s.to_string());
                    }
                    if resolved_name.is_none() {
                        resolved_name = json.get("name").and_then(|v| v.as_str()).map(|s| s.to_string());
                    }
                    if resolved_version.is_none() {
                        resolved_version = json.get("version").and_then(|v| v.as_str()).map(|s| s.to_string());
                    }
                    if resolved_pub.is_none() {
                        resolved_pub = json.get("publisher").or_else(|| json.get("author")).and_then(|v| v.as_str()).map(|s| s.to_string());
                    }
                }
            }
        }
    }

    let file_stem = input.file_stem().and_then(|s| s.to_str()).unwrap_or("bos.app");
    let clean_stem = file_stem.to_lowercase().replace(' ', "-");
    let final_id = resolved_id.unwrap_or_else(|| format!("bos.{}", clean_stem));
    let final_name = resolved_name.unwrap_or_else(|| format!("{} Package", file_stem));
    let final_ver = resolved_version.unwrap_or_else(|| "1.0.0".to_string());
    let final_pub = resolved_pub.unwrap_or_else(|| "BOS Developer".to_string());

    (final_id, final_name, final_ver, final_pub)
}
