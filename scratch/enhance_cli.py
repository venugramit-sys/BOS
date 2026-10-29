with open('crates/bosc-cli/src/main.rs', 'r', encoding='utf-8') as f:
    content = f.read()

# 1. Update PackageCommands enum definition
old_pkg_commands = """enum PackageCommands {
    /// Compile a BOS script into a verified .bospkg package bundle
    Build {
        #[arg(short, long)]
        input: PathBuf,
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
    /// Install a .bospkg package bundle or BOS script into PostgreSQL database
    Install {
        #[arg(short, long)]
        input: PathBuf,
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
}"""

new_pkg_commands = """enum PackageCommands {
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
}"""

# 2. Update PackageCommands::Build and Install dispatch
old_build_dispatch = """            PackageCommands::Build { input, output, package_id, name, version, publisher } => {
                let (resolved_id, resolved_name, resolved_ver, resolved_pub) =
                    resolve_package_metadata(&input, package_id, name, version, publisher);
                let source = fs::read_to_string(&input)?;
                let (bundle, _ddl, model_ir) = compile_package_bundle(&source, &resolved_id, &resolved_name, &resolved_ver, &resolved_pub)?;"""

new_build_dispatch = """            PackageCommands::Build { path, input, output, package_id, name, version, publisher } => {
                let target = path.or(input).unwrap_or_else(|| PathBuf::from("."));
                let (source, source_label) = collect_package_source(&target)?;
                let (resolved_id, resolved_name, resolved_ver, resolved_pub) =
                    resolve_package_metadata(&target, package_id, name, version, publisher);
                let (bundle, _ddl, model_ir) = compile_package_bundle(&source, &resolved_id, &resolved_name, &resolved_ver, &resolved_pub, &source_label)?;"""

old_out_path = """                let out_path = output.unwrap_or_else(|| {
                    let dir = PathBuf::from("packages");
                    let _ = fs::create_dir_all(&dir);
                    dir.join(format!("{}.bospkg", resolved_id))
                });"""

new_out_path = """                let out_path = output.unwrap_or_else(|| {
                    if target.is_dir() {
                        target.join(format!("{}.bospkg", resolved_id))
                    } else if let Some(parent) = target.parent() {
                        parent.join(format!("{}.bospkg", resolved_id))
                    } else {
                        PathBuf::from(format!("{}.bospkg", resolved_id))
                    }
                });"""

old_install_full = """            PackageCommands::Install { input, db, package_id, name, version, publisher, allow_destructive } => {
                println!("Connecting to PostgreSQL database at {}...", db);
                let engine = BosEngine::connect(&db).await?;

                let sys_status = engine.get_system_status().await?;
                if !sys_status.is_provisioned {
                    println!("New database detected. Auto-provisioning BOS System Kernel...");
                    engine.provision_system(None).await?;
                    println!("✓ BOS System Kernel provisioned successfully.");
                }

                let content = fs::read_to_string(&input)?;
                let bundle = if input.extension().and_then(|s| s.to_str()) == Some("bospkg") || content.trim_start().starts_with('{') {
                    serde_json::from_str::<bosc_ir::PackageBundleIR>(&content)?
                } else {
                    let (resolved_id, resolved_name, resolved_ver, resolved_pub) =
                        resolve_package_metadata(&input, package_id, name, version, publisher);
                    let (b, ddl, _model) = compile_package_bundle(
                        &content,
                        &resolved_id,
                        &resolved_name,
                        &resolved_ver,
                        &resolved_pub,
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
                };

                let res = engine.install_package(bundle, allow_destructive).await?;"""

new_install_full = """            PackageCommands::Install { path, input, db, package_id, name, version, publisher, allow_destructive } => {
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

                let res = engine.install_package(bundle, allow_destructive).await?;"""

old_compile_fn = """fn compile_package_bundle(
    source: &str,
    package_id: &str,
    name: &str,
    version: &str,
    publisher: &str,
) -> Result<(bosc_ir::PackageBundleIR, String, bosc_ir::ModelIR)> {
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize()?;
    let mut parser = Parser::with_source(tokens, source.to_string());
    let program = parser.parse_program()?;"""

new_compile_fn = """fn collect_package_source(input: &std::path::Path) -> Result<(String, String)> {
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
            source.push_str(&format!("\\n// === Source: {} ===\\n", path.display()));
            source.push_str(&file_content);
            source.push('\\n');
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
            eprintln!("\\n{}", visual);
            anyhow::bail!("Syntax error in '{}': {}", source_label, err);
        }
    };"""

old_resolve_meta = """fn resolve_package_metadata(
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

    // Check adjacent bos.json or package.json
    if let Some(parent) = input.parent() {
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
    }"""

new_resolve_meta = """fn resolve_package_metadata(
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
    }"""

content_norm = content.replace("\r\n", "\n")

assert old_pkg_commands.replace("\r\n", "\n") in content_norm, "old_pkg_commands not found"
assert old_build_dispatch.replace("\r\n", "\n") in content_norm, "old_build_dispatch not found"
assert old_out_path.replace("\r\n", "\n") in content_norm, "old_out_path not found"
assert old_install_full.replace("\r\n", "\n") in content_norm, "old_install_full not found"
assert old_compile_fn.replace("\r\n", "\n") in content_norm, "old_compile_fn not found"
assert old_resolve_meta.replace("\r\n", "\n") in content_norm, "old_resolve_meta not found"

content_norm = content_norm.replace(old_pkg_commands.replace("\r\n", "\n"), new_pkg_commands.replace("\r\n", "\n"), 1)
content_norm = content_norm.replace(old_build_dispatch.replace("\r\n", "\n"), new_build_dispatch.replace("\r\n", "\n"), 1)
content_norm = content_norm.replace(old_out_path.replace("\r\n", "\n"), new_out_path.replace("\r\n", "\n"), 1)
content_norm = content_norm.replace(old_install_full.replace("\r\n", "\n"), new_install_full.replace("\r\n", "\n"), 1)
content_norm = content_norm.replace(old_compile_fn.replace("\r\n", "\n"), new_compile_fn.replace("\r\n", "\n"), 1)
content_norm = content_norm.replace(old_resolve_meta.replace("\r\n", "\n"), new_resolve_meta.replace("\r\n", "\n"), 1)

with open('crates/bosc-cli/src/main.rs', 'w', encoding='utf-8', newline='\r\n') as f:
    f.write(content_norm)

print("Updated crates/bosc-cli/src/main.rs successfully!")
