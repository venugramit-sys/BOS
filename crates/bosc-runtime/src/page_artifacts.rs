//! Compiled page artifacts: catalog snapshot → compiled pages → content-addressed Svelte modules on disk.
//!
//! - **One catalog read per refresh.** Pages, page extensions, tables, table extensions, enums and
//!   codeunits are loaded from `_bos_meta_objects` once. Every page is merged and resolved against the
//!   complete (closed) catalog and turned into Svelte source. These steps are pure Rust and take
//!   milliseconds.
//! - **Content addressing.** The artifact hash covers the generated source (which embeds the code
//!   generator version and every resolved table property) plus the Svelte compiler identity. A page is
//!   only re-run through the Svelte compiler when that hash changes. Files are named
//!   `<Page>.<hash16>.js`, so nodes that compute the same hash write identical files and need no locks.
//! - **Hot path without database reads.** The manifest and artifacts are served from memory and disk.
//!   A missing file is compiled on first request (single-flight per hash) or copied from a pre-built
//!   seed folder (`bosc package build` output).

use bosc_analyzer::page_resolver::enforce_customization_whitelist;
use bosc_analyzer::{compile_page, merge_page_detailed, sort_extensions_for_merge, ModelCatalog};
use bosc_codegen_svelte::{CodegenError, GenerateOptions, CODEGEN_VERSION};
use bosc_ir::*;
use serde::Serialize;
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Row};
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use crate::page_compiler::{PageCompilerError, SvelteCompiler};

/// Manifest format version (bump on incompatible manifest changes).
pub const MANIFEST_FORMAT: u32 = 1;
/// URL prefix under which artifacts are served.
pub const ARTIFACT_ROUTE: &str = "/page-artifacts";

const STALE_ARTIFACT_AGE: Duration = Duration::from_secs(24 * 60 * 60);
const PREVIEW_TTL: Duration = Duration::from_secs(60 * 60);

#[derive(Debug, thiserror::Error)]
pub enum PageServiceError {
    #[error("page service has not loaded the catalog yet")]
    NotReady,
    #[error("page '{0}' does not exist")]
    UnknownPage(String),
    #[error("artifact '{0}' does not exist")]
    UnknownArtifact(String),
    #[error("page compiler is unavailable: {0}")]
    CompilerUnavailable(String),
    #[error(transparent)]
    Compiler(#[from] PageCompilerError),
    #[error("page '{page}' cannot be generated: {reason}")]
    Generate { page: String, reason: String },
    #[error("artifact store I/O error at {path}: {reason}")]
    Io { path: PathBuf, reason: String },
    #[error("catalog query failed: {0}")]
    Catalog(String),
    #[error("{0}")]
    BadRequest(String),
}

#[derive(Debug, Clone)]
pub struct PageServiceConfig {
    pub artifact_root: PathBuf,
    pub runtime_dir: PathBuf,
    pub workers: usize,
    pub seed_dirs: Vec<PathBuf>,
}

impl PageServiceConfig {
    pub fn from_node_config(cfg: &crate::node_config::NodeConfig) -> Self {
        let runtime_dir = cfg
            .pages
            .runtime_dir
            .clone()
            .map(PathBuf::from)
            .unwrap_or_else(|| Path::new(&cfg.ui.static_dir).join("bos-runtime"));
        PageServiceConfig {
            artifact_root: PathBuf::from(&cfg.pages.artifact_dir),
            runtime_dir,
            workers: if cfg.pages.compiler_workers == 0 { SvelteCompiler::default_workers() } else { cfg.pages.compiler_workers },
            seed_dirs: cfg.pages.seed_dirs.iter().map(PathBuf::from).collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ManifestPage {
    pub file: String,
    pub url: String,
    pub hash: String,
    pub page_type: PageType,
    pub caption: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_table: Option<String>,
    pub errors: usize,
    pub warnings: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct PageManifest {
    pub format: u32,
    pub db_key: String,
    pub etag: String,
    pub codegen: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub svelte: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compiler_error: Option<String>,
    pub generated_at: String,
    pub pages: BTreeMap<String, ManifestPage>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub catalog_errors: Vec<String>,
}

/// One compiled page with everything the routes need.
#[derive(Debug)]
pub struct PageEntry {
    pub name: String,
    /// Base page merged with its extensions (authored form: holds the server-side trigger scripts).
    pub merged: PageIR,
    pub compiled: CompiledPageIR,
    pub diagnostics: Vec<PageDiagnosticIR>,
    pub source: Arc<String>,
    pub hash: String,
    pub file: String,
}

struct PageState {
    db_key: String,
    manifest: PageManifest,
    manifest_json: Arc<Vec<u8>>,
    entries: HashMap<String, Arc<PageEntry>>,
    by_file: HashMap<String, Arc<PageEntry>>,
    catalog: Arc<CatalogSnapshot>,
}

/// Catalog snapshot used to compile pages.
#[derive(Debug, Clone)]
pub struct CatalogSnapshot {
    pub model: ModelIR,
    /// Merge rank of each page extension (by its package's install order).
    pub extension_rank: HashMap<String, usize>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct RefreshReport {
    pub db_key: String,
    pub pages: usize,
    pub changed: Vec<String>,
    pub removed: Vec<String>,
    pub etag: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PreviewResult {
    pub url: String,
    pub hash: String,
    /// Generated Svelte source (the Vite dev server compiles it itself for hot reload).
    pub source: String,
    pub compiled: CompiledPageIR,
    pub diagnostics: Vec<PageDiagnosticIR>,
}

pub struct PageService {
    cfg: PageServiceConfig,
    compiler: Option<Arc<SvelteCompiler>>,
    compiler_error: Option<String>,
    state: arc_swap::ArcSwapOption<PageState>,
    refresh_lock: tokio::sync::Mutex<()>,
    inflight: dashmap::DashMap<String, Arc<tokio::sync::Mutex<()>>>,
    changes: tokio::sync::watch::Sender<String>,
    compiles: AtomicU64,
    instance: uuid::Uuid,
}

// ----------------------------------------------------
// HELPERS
// ----------------------------------------------------

/// Stable key of a database: hash of host, port and database name (credentials and options excluded).
pub fn db_key(database_url: &str) -> String {
    let without_scheme = database_url.split_once("://").map(|(_, rest)| rest).unwrap_or(database_url);
    let without_auth = without_scheme.rsplit_once('@').map(|(_, rest)| rest).unwrap_or(without_scheme);
    let target = without_auth.split('?').next().unwrap_or(without_auth).to_ascii_lowercase();
    hex16(&Sha256::digest(target.as_bytes()))
}

fn hex16(digest: &[u8]) -> String {
    digest.iter().take(8).map(|b| format!("{b:02x}")).collect()
}

fn safe_file_stem(name: &str) -> String {
    let s: String = name.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '_').collect();
    if s.is_empty() { "page".to_string() } else { s }
}

/// `<Stem>.<16 hex>.js` only: rejects traversal and anything the store never writes.
pub fn is_valid_artifact_name(file: &str) -> bool {
    let Some(base) = file.strip_suffix(".js") else { return false };
    let Some((stem, hash)) = base.rsplit_once('.') else { return false };
    !stem.is_empty()
        && stem.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        && hash.len() == 16
        && hash.chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
}

fn artifact_hash(svelte: &str, compiler_sha: &str, source: &str) -> String {
    let mut h = Sha256::new();
    h.update(b"bos-page-artifact/1\0");
    h.update(svelte.as_bytes());
    h.update(b"\0");
    h.update(compiler_sha.as_bytes());
    h.update(b"\0");
    h.update(source.as_bytes());
    format!("{:x}", h.finalize())
}

fn io_err(path: &Path, e: impl std::fmt::Display) -> PageServiceError {
    PageServiceError::Io { path: path.to_path_buf(), reason: e.to_string() }
}

/// Writes `bytes` to `path` through a temporary file and an atomic rename. An existing file is left
/// untouched: artifacts are content-addressed, so it already holds identical bytes.
fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), PageServiceError> {
    if path.is_file() {
        return Ok(());
    }
    let dir = path.parent().ok_or_else(|| io_err(path, "no parent directory"))?;
    std::fs::create_dir_all(dir).map_err(|e| io_err(dir, e))?;
    let tmp = dir.join(format!(".{}.{}.tmp", path.file_name().and_then(|n| n.to_str()).unwrap_or("artifact"), uuid::Uuid::new_v4().simple()));
    std::fs::write(&tmp, bytes).map_err(|e| io_err(&tmp, e))?;
    match std::fs::rename(&tmp, path) {
        Ok(()) => Ok(()),
        Err(_) if path.is_file() => {
            let _ = std::fs::remove_file(&tmp);
            Ok(())
        }
        Err(e) => {
            let _ = std::fs::remove_file(&tmp);
            Err(io_err(path, e))
        }
    }
}

/// Overwrites a mutable file (the manifest) through a temporary file.
fn replace_file(path: &Path, bytes: &[u8]) -> Result<(), PageServiceError> {
    let dir = path.parent().ok_or_else(|| io_err(path, "no parent directory"))?;
    std::fs::create_dir_all(dir).map_err(|e| io_err(dir, e))?;
    let tmp = dir.join(format!(".manifest.{}.tmp", uuid::Uuid::new_v4().simple()));
    std::fs::write(&tmp, bytes).map_err(|e| io_err(&tmp, e))?;
    if path.exists() {
        let _ = std::fs::remove_file(path);
    }
    std::fs::rename(&tmp, path).map_err(|e| io_err(path, e))
}

fn parse_definition<T: serde::de::DeserializeOwned>(kind: &str, name: &str, def: serde_json::Value, errors: &mut Vec<String>) -> Option<T> {
    match serde_json::from_value(def) {
        Ok(v) => Some(v),
        Err(e) => {
            errors.push(format!("{kind} '{name}' has an unreadable definition: {e}"));
            None
        }
    }
}

/// Loads every object a page can reference, in one query.
pub async fn load_catalog(pool: &PgPool) -> Result<CatalogSnapshot, PageServiceError> {
    let mut model = ModelIR {
        package_name: "catalog".to_string(),
        tables: Vec::new(),
        table_logics: Vec::new(),
        table_extensions: Vec::new(),
        table_logic_extensions: Vec::new(),
        state_machines: Vec::new(),
        state_machine_extensions: Vec::new(),
        codeunits: Vec::new(),
        interfaces: Vec::new(),
        enums: Vec::new(),
        enum_extensions: Vec::new(),
        permission_sets: Vec::new(),
        views: Vec::new(),
        queries: Vec::new(),
        pages: Vec::new(),
        page_extensions: Vec::new(),
        menus: Vec::new(),
        reports: Vec::new(),
        custom_metadata: Vec::new(),
        diagnostics: Vec::new(),
    };
    let mut errors = Vec::new();
    let mut extension_rank = HashMap::new();

    let provisioned: bool = sqlx::query_scalar("SELECT to_regclass('_bos_meta_objects') IS NOT NULL")
        .fetch_one(pool)
        .await
        .map_err(|e| PageServiceError::Catalog(e.to_string()))?;
    if !provisioned {
        return Ok(CatalogSnapshot { model, extension_rank, errors });
    }

    let has_packages: bool = sqlx::query_scalar("SELECT to_regclass('_bos_meta_packages') IS NOT NULL")
        .fetch_one(pool)
        .await
        .map_err(|e| PageServiceError::Catalog(e.to_string()))?;
    let package_order: HashMap<String, usize> = if has_packages {
        sqlx::query_scalar::<_, String>(r#"SELECT package_id FROM "_bos_meta_packages" ORDER BY installed_at, package_id"#)
            .fetch_all(pool)
            .await
            .map_err(|e| PageServiceError::Catalog(e.to_string()))?
            .into_iter()
            .enumerate()
            .map(|(i, p)| (p.to_ascii_lowercase(), i))
            .collect()
    } else {
        HashMap::new()
    };

    let rows = sqlx::query(
        r#"SELECT object_type, object_name, package_name, definition FROM "_bos_meta_objects"
           WHERE object_type IN ('table', 'tableextension', 'enum', 'enumextension', 'page', 'pageextension', 'codeunit')
           ORDER BY updated_at, object_name"#,
    )
    .fetch_all(pool)
    .await
    .map_err(|e| PageServiceError::Catalog(e.to_string()))?;

    let mut table_exts: Vec<TableExtensionIR> = Vec::new();
    for row in rows {
        let kind: String = row.try_get("object_type").map_err(|e| PageServiceError::Catalog(e.to_string()))?;
        let name: String = row.try_get("object_name").map_err(|e| PageServiceError::Catalog(e.to_string()))?;
        let package: String = row.try_get::<Option<String>, _>("package_name").ok().flatten().unwrap_or_default();
        let def: serde_json::Value = row.try_get("definition").map_err(|e| PageServiceError::Catalog(e.to_string()))?;
        // Later rows (by updated_at) replace earlier ones with the same name.
        fn upsert<T>(list: &mut Vec<T>, item: T, name_of: impl Fn(&T) -> &str) {
            let n = name_of(&item).to_ascii_lowercase();
            list.retain(|x| name_of(x).to_ascii_lowercase() != n);
            list.push(item);
        }
        match kind.as_str() {
            "table" => {
                if let Some(t) = parse_definition::<TableIR>(&kind, &name, def, &mut errors) {
                    upsert(&mut model.tables, t, |t| &t.name);
                }
            }
            "tableextension" => {
                if let Some(t) = parse_definition::<TableExtensionIR>(&kind, &name, def, &mut errors) {
                    table_exts.push(t);
                }
            }
            "enum" => {
                if let Some(e) = parse_definition::<EnumIR>(&kind, &name, def, &mut errors) {
                    upsert(&mut model.enums, e, |e| &e.name);
                }
            }
            "enumextension" => {
                if let Some(e) = parse_definition::<EnumExtensionIR>(&kind, &name, def, &mut errors) {
                    upsert(&mut model.enum_extensions, e, |e| &e.name);
                }
            }
            "page" => {
                if let Some(p) = parse_definition::<PageIR>(&kind, &name, def, &mut errors) {
                    upsert(&mut model.pages, p, |p| &p.name);
                }
            }
            "pageextension" => {
                if let Some(p) = parse_definition::<PageExtensionIR>(&kind, &name, def, &mut errors) {
                    let rank = package_order.get(&package.to_ascii_lowercase()).copied().unwrap_or(package_order.len());
                    extension_rank.insert(p.name.to_ascii_lowercase(), rank);
                    upsert(&mut model.page_extensions, p, |p| &p.name);
                }
            }
            "codeunit" => {
                let cu = serde_json::from_value::<CodeunitIR>(def).unwrap_or(CodeunitIR {
                    name: name.clone(),
                    package_name: package.clone(),
                    bound_table: None,
                    is_exposed: false,
                    is_transactional: false,
                    is_pushdown: false,
                    execution_mode: None,
                    max_fuel: None,
                    methods: Vec::new(),
                });
                upsert(&mut model.codeunits, cu, |c| &c.name);
            }
            _ => {}
        }
    }

    // Table extensions add their columns to the target table, as the schema does.
    for ext in table_exts {
        match model
            .tables
            .iter_mut()
            .find(|t| t.name.eq_ignore_ascii_case(&ext.target_table) || t.sql_table_name.eq_ignore_ascii_case(&ext.target_table))
        {
            Some(t) => {
                for col in ext.added_columns {
                    if !t.columns.iter().any(|c| c.sql_name.eq_ignore_ascii_case(&col.sql_name)) {
                        t.columns.push(col);
                    }
                }
            }
            None => errors.push(format!("tableextension '{}' targets unknown table '{}'", ext.name, ext.target_table)),
        }
    }
    Ok(CatalogSnapshot { model, extension_rank, errors })
}

/// Extensions of a page in merge order, with the customization whitelist applied.
fn extensions_for(catalog: &CatalogSnapshot, page: &str) -> Vec<PageExtensionIR> {
    let mut exts: Vec<PageExtensionIR> = catalog
        .model
        .page_extensions
        .iter()
        .filter(|e| e.target_page.eq_ignore_ascii_case(page))
        .map(|e| if e.is_customization { enforce_customization_whitelist(e).0 } else { e.clone() })
        .collect();
    sort_extensions_for_merge(&mut exts, |e| catalog.extension_rank.get(&e.name.to_ascii_lowercase()).copied().unwrap_or(usize::MAX));
    exts
}

// ----------------------------------------------------
// SERVICE
// ----------------------------------------------------

impl PageService {
    /// Starts the compiler pool. A missing or broken compiler does not stop the node: pages are still
    /// resolved and listed (with their diagnostics), and artifact requests report the compiler error.
    pub async fn start(cfg: PageServiceConfig) -> Arc<PageService> {
        let workers = cfg.workers;
        let runtime_dir = cfg.runtime_dir.clone();
        let started = tokio::task::spawn_blocking(move || {
            let path = SvelteCompiler::locate(&runtime_dir)?;
            SvelteCompiler::start(&path, workers)
        })
        .await;
        let (compiler, compiler_error) = match started {
            Ok(Ok(c)) => {
                tracing::info!(target: "bos::pages", svelte = %c.info().svelte_version, workers = c.workers(), "Svelte page compiler ready");
                (Some(Arc::new(c)), None)
            }
            Ok(Err(e)) => {
                tracing::error!(target: "bos::pages", "page compiler unavailable: {e}");
                (None, Some(e.to_string()))
            }
            Err(e) => (None, Some(format!("compiler startup panicked: {e}"))),
        };
        let (changes, _) = tokio::sync::watch::channel(String::new());
        Arc::new(PageService {
            cfg,
            compiler,
            compiler_error,
            state: arc_swap::ArcSwapOption::empty(),
            refresh_lock: tokio::sync::Mutex::new(()),
            inflight: dashmap::DashMap::new(),
            changes,
            compiles: AtomicU64::new(0),
            instance: uuid::Uuid::new_v4(),
        })
    }

    pub fn instance(&self) -> uuid::Uuid {
        self.instance
    }

    pub fn config(&self) -> &PageServiceConfig {
        &self.cfg
    }

    /// Number of Svelte compilations performed by this node (used to verify incremental behaviour).
    pub fn compile_count(&self) -> u64 {
        self.compiles.load(Ordering::SeqCst)
    }

    pub fn compiler_error(&self) -> Option<&str> {
        self.compiler_error.as_deref()
    }

    pub fn db_key(&self) -> Option<String> {
        self.state.load().as_ref().map(|s| s.db_key.clone())
    }

    /// Receives the manifest etag whenever it changes.
    pub fn subscribe(&self) -> tokio::sync::watch::Receiver<String> {
        self.changes.subscribe()
    }

    /// Current manifest as (etag, JSON bytes).
    pub fn manifest(&self) -> Option<(String, Arc<Vec<u8>>)> {
        self.state.load().as_ref().map(|s| (s.manifest.etag.clone(), s.manifest_json.clone()))
    }

    /// Base (unmerged) page and its extensions in merge order, for designer previews.
    pub fn base_page(&self, name: &str) -> Option<(PageIR, Vec<PageExtensionIR>)> {
        let state = self.state.load_full()?;
        let page = state.catalog.model.pages.iter().find(|p| p.name.eq_ignore_ascii_case(name))?.clone();
        let exts = extensions_for(&state.catalog, &page.name);
        Some((page, exts))
    }

    /// A page extension of the current catalog (case-insensitive name).
    pub fn page_extension(&self, name: &str) -> Option<PageExtensionIR> {
        let state = self.state.load_full()?;
        state.catalog.model.page_extensions.iter().find(|e| e.name.eq_ignore_ascii_case(name)).cloned()
    }

    pub fn page(&self, name: &str) -> Option<Arc<PageEntry>> {
        self.state.load().as_ref().and_then(|s| s.entries.get(&name.to_ascii_lowercase()).cloned())
    }

    fn compiler_identity(&self) -> (String, String) {
        match &self.compiler {
            Some(c) => (c.info().svelte_version.clone(), c.info().compiler_sha256.clone()),
            None => ("unavailable".to_string(), "unavailable".to_string()),
        }
    }

    fn db_dir(&self, db_key: &str) -> PathBuf {
        self.cfg.artifact_root.join(db_key)
    }

    /// Reloads the catalog and recompiles the page IR of every page. Svelte compilation of changed
    /// pages happens lazily (first request) or in [`Self::warm`].
    pub async fn refresh(&self, pool: &PgPool, database_url: &str) -> Result<RefreshReport, PageServiceError> {
        let _guard = self.refresh_lock.lock().await;
        let key = db_key(database_url);
        let catalog = Arc::new(load_catalog(pool).await?);
        let built = self.build_state(key.clone(), catalog);

        let previous = self.state.load_full();
        let mut report = RefreshReport { db_key: key.clone(), pages: built.entries.len(), etag: built.manifest.etag.clone(), ..Default::default() };
        let state = match previous.as_deref() {
            Some(prev) if prev.db_key == key => {
                for (name, entry) in &built.entries {
                    if prev.entries.get(name).map(|p| p.hash != entry.hash).unwrap_or(true) {
                        report.changed.push(entry.name.clone());
                    }
                }
                for (name, entry) in &prev.entries {
                    if !built.entries.contains_key(name) {
                        report.removed.push(entry.name.clone());
                    }
                }
                if prev.manifest.etag == built.manifest.etag {
                    // Nothing visible changed: keep the published manifest (and its timestamp).
                    let mut same = built;
                    same.manifest = prev.manifest.clone();
                    same.manifest_json = prev.manifest_json.clone();
                    same
                } else {
                    built
                }
            }
            _ => {
                report.changed = built.entries.values().map(|e| e.name.clone()).collect();
                built
            }
        };
        report.changed.sort();
        report.removed.sort();

        let manifest_path = self.db_dir(&key).join("manifest.json");
        if let Err(e) = replace_file(&manifest_path, &state.manifest_json) {
            tracing::warn!(target: "bos::pages", "could not persist manifest: {e}");
        }
        let etag = state.manifest.etag.clone();
        self.state.store(Some(Arc::new(state)));
        self.changes.send_if_modified(|current| {
            if *current != etag {
                *current = etag.clone();
                true
            } else {
                false
            }
        });
        Ok(report)
    }

    fn build_state(&self, db_key: String, catalog: Arc<CatalogSnapshot>) -> PageState {
        let (svelte, compiler_sha) = self.compiler_identity();
        let closed = ModelCatalog::new(vec![&catalog.model]).closed();
        let mut entries = HashMap::new();
        let mut by_file = HashMap::new();
        let mut pages = BTreeMap::new();
        let mut errors = catalog.errors.clone();

        for page in &catalog.model.pages {
            let exts = extensions_for(&catalog, &page.name);
            let result = compile_page(page, &exts, &closed);
            let source = match bosc_codegen_svelte::generate(&result.page, GenerateOptions::default()) {
                Ok(g) => g.source,
                Err(CodegenError::NoUserInterface(_)) => continue,
                Err(e) => {
                    errors.push(e.to_string());
                    continue;
                }
            };
            let hash = artifact_hash(&svelte, &compiler_sha, &source);
            let file = format!("{}.{}.js", safe_file_stem(&page.name), &hash[..16]);
            let merged = merge_page_detailed(page, &exts).page;
            let count = |sev| result.diagnostics.iter().filter(|d| d.severity == sev).count();
            pages.insert(
                page.name.clone(),
                ManifestPage {
                    url: format!("{ARTIFACT_ROUTE}/{db_key}/{file}"),
                    file: file.clone(),
                    hash: hash.clone(),
                    page_type: result.page.page_type,
                    caption: result.page.caption.clone(),
                    source_table: result.page.source_table.clone(),
                    errors: count(PageDiagnosticSeverity::Error),
                    warnings: count(PageDiagnosticSeverity::Warning),
                },
            );
            let entry = Arc::new(PageEntry {
                name: page.name.clone(),
                merged,
                compiled: result.page,
                diagnostics: result.diagnostics,
                source: Arc::new(source),
                hash,
                file: file.clone(),
            });
            by_file.insert(file, entry.clone());
            entries.insert(page.name.to_ascii_lowercase(), entry);
        }

        let etag = {
            let mut h = Sha256::new();
            h.update(db_key.as_bytes());
            h.update(serde_json::to_vec(&pages).unwrap_or_default());
            h.update(serde_json::to_vec(&errors).unwrap_or_default());
            h.update(self.compiler_error.as_deref().unwrap_or("").as_bytes());
            hex16(&h.finalize())
        };
        let manifest = PageManifest {
            format: MANIFEST_FORMAT,
            db_key: db_key.clone(),
            etag,
            codegen: CODEGEN_VERSION,
            svelte: self.compiler.as_ref().map(|c| c.info().svelte_version.clone()),
            compiler_error: self.compiler_error.clone(),
            generated_at: chrono::Utc::now().to_rfc3339(),
            pages,
            catalog_errors: errors,
        };
        let manifest_json = Arc::new(serde_json::to_vec(&manifest).unwrap_or_default());
        PageState { db_key, manifest, manifest_json, entries, by_file, catalog }
    }

    fn seed_artifact(&self, file: &str) -> Option<Vec<u8>> {
        for seed in &self.cfg.seed_dirs {
            let Ok(packages) = std::fs::read_dir(seed) else { continue };
            for pkg in packages.flatten() {
                let candidate = pkg.path().join("pages").join(file);
                if let Ok(bytes) = std::fs::read(&candidate) {
                    return Some(bytes);
                }
            }
        }
        None
    }

    /// Returns the compiled module for `file` of the current manifest, compiling it if needed.
    /// Files of earlier manifests are served as long as they are on disk.
    pub async fn artifact(&self, db: &str, file: &str) -> Result<Vec<u8>, PageServiceError> {
        if !is_valid_artifact_name(file) {
            return Err(PageServiceError::UnknownArtifact(file.to_string()));
        }
        let state = self.state.load_full().ok_or(PageServiceError::NotReady)?;
        let path = self.db_dir(db).join(file);
        if let Ok(bytes) = tokio::fs::read(&path).await {
            return Ok(bytes);
        }
        if state.db_key != db {
            return Err(PageServiceError::UnknownArtifact(file.to_string()));
        }
        let entry = state.by_file.get(file).cloned().ok_or_else(|| PageServiceError::UnknownArtifact(file.to_string()))?;
        self.ensure(&state.db_key, &entry).await
    }

    /// Single-flight: concurrent requests for the same hash share one compilation.
    async fn ensure(&self, db: &str, entry: &PageEntry) -> Result<Vec<u8>, PageServiceError> {
        let path = self.db_dir(db).join(&entry.file);
        let lock = self.inflight.entry(entry.hash.clone()).or_insert_with(|| Arc::new(tokio::sync::Mutex::new(()))).clone();
        let _guard = lock.lock().await;
        let result = async {
            if let Ok(bytes) = tokio::fs::read(&path).await {
                return Ok(bytes);
            }
            if let Some(bytes) = self.seed_artifact(&entry.file) {
                write_atomic(&path, &bytes)?;
                return Ok(bytes);
            }
            let compiler = self
                .compiler
                .as_ref()
                .ok_or_else(|| PageServiceError::CompilerUnavailable(self.compiler_error.clone().unwrap_or_default()))?;
            let module = compiler.compile(&entry.name, entry.source.as_ref().clone()).await?;
            self.compiles.fetch_add(1, Ordering::SeqCst);
            for w in &module.warnings {
                tracing::warn!(target: "bos::pages", page = %entry.name, "svelte: {w}");
            }
            write_atomic(&path, module.js.as_bytes())?;
            Ok(module.js.into_bytes())
        }
        .await;
        drop(_guard);
        self.inflight.remove_if(&entry.hash, |_, l| Arc::strong_count(l) <= 2);
        result
    }

    /// Compiles every page whose artifact is missing: role centers first, then lists, then the rest.
    pub async fn warm(&self) -> Vec<(String, String)> {
        let Some(state) = self.state.load_full() else { return Vec::new() };
        let mut pending_entries: Vec<Arc<PageEntry>> =
            state.entries.values().filter(|e| !self.db_dir(&state.db_key).join(&e.file).is_file()).cloned().collect();
        let rank = |t: PageType| match t {
            PageType::RoleCenter => 0,
            PageType::List => 1,
            PageType::Card | PageType::Document => 2,
            _ => 3,
        };
        pending_entries.sort_by(|a, b| (rank(a.compiled.page_type), &a.name).cmp(&(rank(b.compiled.page_type), &b.name)));
        let mut failures = Vec::new();
        let workers = self.compiler.as_ref().map(|c| c.workers()).unwrap_or(1).max(1);
        for chunk in pending_entries.chunks(workers) {
            let results = futures_util::future::join_all(chunk.iter().map(|e| self.ensure(&state.db_key, e))).await;
            for (entry, r) in chunk.iter().zip(results) {
                if let Err(e) = r {
                    tracing::error!(target: "bos::pages", page = %entry.name, "warm compile failed: {e}");
                    failures.push((entry.name.clone(), e.to_string()));
                }
            }
        }
        failures
    }

    /// Deletes artifacts no manifest references that are older than a day, and expired previews.
    pub fn cleanup(&self) -> usize {
        let Some(state) = self.state.load_full() else { return 0 };
        let now = SystemTime::now();
        let old = |p: &Path, age: Duration| {
            std::fs::metadata(p).and_then(|m| m.modified()).ok().and_then(|t| now.duration_since(t).ok()).is_some_and(|d| d > age)
        };
        let mut removed = 0;
        let dir = self.db_dir(&state.db_key);
        if let Ok(files) = std::fs::read_dir(&dir) {
            for f in files.flatten() {
                let name = f.file_name().to_string_lossy().to_string();
                if is_valid_artifact_name(&name) && !state.by_file.contains_key(&name) && old(&f.path(), STALE_ARTIFACT_AGE) && std::fs::remove_file(f.path()).is_ok() {
                    removed += 1;
                }
            }
        }
        if let Ok(files) = std::fs::read_dir(dir.join("preview")) {
            for f in files.flatten() {
                if old(&f.path(), PREVIEW_TTL) && std::fs::remove_file(f.path()).is_ok() {
                    removed += 1;
                }
            }
        }
        removed
    }

    /// Compiles an unsaved page definition (designer preview) against the current catalog.
    pub async fn preview(
        &self,
        page: PageIR,
        extensions: Option<Vec<PageExtensionIR>>,
        design: bool,
    ) -> Result<PreviewResult, PageServiceError> {
        let state = self.state.load_full().ok_or(PageServiceError::NotReady)?;
        let compiler = self
            .compiler
            .as_ref()
            .ok_or_else(|| PageServiceError::CompilerUnavailable(self.compiler_error.clone().unwrap_or_default()))?;
        let mut model = state.catalog.model.clone();
        model.pages.retain(|p| !p.name.eq_ignore_ascii_case(&page.name));
        model.pages.push(page.clone());
        let exts = match extensions {
            Some(mut list) => {
                for e in &mut list {
                    if e.is_customization {
                        *e = enforce_customization_whitelist(e).0;
                    }
                }
                sort_extensions_for_merge(&mut list, |e| state.catalog.extension_rank.get(&e.name.to_ascii_lowercase()).copied().unwrap_or(usize::MAX));
                list
            }
            None => extensions_for(&state.catalog, &page.name),
        };
        let result = compile_page(&page, &exts, &ModelCatalog::new(vec![&model]).closed());
        let generated = bosc_codegen_svelte::generate(&result.page, GenerateOptions { design })
            .map_err(|e| PageServiceError::Generate { page: page.name.clone(), reason: e.to_string() })?;
        let (svelte, sha) = self.compiler_identity();
        let hash = artifact_hash(&svelte, &sha, &generated.source);
        let file = format!("{}.{}.js", safe_file_stem(&page.name), &hash[..16]);
        let path = self.db_dir(&state.db_key).join("preview").join(&file);
        if !path.is_file() {
            let module = compiler.compile(&page.name, generated.source.clone()).await?;
            self.compiles.fetch_add(1, Ordering::SeqCst);
            write_atomic(&path, module.js.as_bytes())?;
        }
        Ok(PreviewResult {
            url: format!("{ARTIFACT_ROUTE}/{}/preview/{file}", state.db_key),
            hash,
            source: generated.source,
            compiled: result.page,
            diagnostics: result.diagnostics,
        })
    }

    /// Serves a preview module written by [`Self::preview`].
    pub async fn preview_artifact(&self, db: &str, file: &str) -> Result<Vec<u8>, PageServiceError> {
        if !is_valid_artifact_name(file) {
            return Err(PageServiceError::UnknownArtifact(file.to_string()));
        }
        tokio::fs::read(self.db_dir(db).join("preview").join(file))
            .await
            .map_err(|_| PageServiceError::UnknownArtifact(file.to_string()))
    }
}

// ----------------------------------------------------
// PACKAGE BUILD (bosc package build)
// ----------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BuiltPage {
    pub name: String,
    pub file: String,
    pub hash: String,
    /// `false` when no compiler was available: only the hash was computed.
    pub compiled: bool,
    pub errors: usize,
}

/// Compiles the pages of one package into `out_dir` (`<Page>.<hash16>.js` + `manifest.json`). Hashes are
/// computed exactly like the node's, so a node copies these files instead of compiling when the
/// package's pages resolve identically in its database.
pub async fn build_model_pages(
    model: &ModelIR,
    compiler: Option<&SvelteCompiler>,
    out_dir: &Path,
) -> Result<Vec<BuiltPage>, PageServiceError> {
    let catalog = CatalogSnapshot { model: model.clone(), extension_rank: HashMap::new(), errors: Vec::new() };
    let (svelte, sha) = match compiler {
        Some(c) => (c.info().svelte_version.clone(), c.info().compiler_sha256.clone()),
        None => ("unavailable".to_string(), "unavailable".to_string()),
    };
    let closed = ModelCatalog::new(vec![&catalog.model]).closed();
    let mut built = Vec::new();
    for page in &catalog.model.pages {
        let exts = extensions_for(&catalog, &page.name);
        let result = compile_page(page, &exts, &closed);
        let generated = match bosc_codegen_svelte::generate(&result.page, GenerateOptions::default()) {
            Ok(g) => g,
            Err(CodegenError::NoUserInterface(_)) => continue,
            Err(e) => return Err(PageServiceError::Generate { page: page.name.clone(), reason: e.to_string() }),
        };
        let hash = artifact_hash(&svelte, &sha, &generated.source);
        let file = format!("{}.{}.js", safe_file_stem(&page.name), &hash[..16]);
        let compiled = match compiler {
            Some(c) => {
                let module = c.compile(&page.name, generated.source).await?;
                write_atomic(&out_dir.join(&file), module.js.as_bytes())?;
                true
            }
            None => false,
        };
        let errors = result.diagnostics.iter().filter(|d| d.severity == PageDiagnosticSeverity::Error).count();
        built.push(BuiltPage { name: page.name.clone(), file, hash, compiled, errors });
    }
    let manifest = serde_json::json!({
        "format": MANIFEST_FORMAT,
        "codegen": CODEGEN_VERSION,
        "svelte": compiler.map(|c| c.info().svelte_version.clone()),
        "pages": built,
    });
    replace_file(&out_dir.join("manifest.json"), &serde_json::to_vec_pretty(&manifest).unwrap_or_default())?;
    Ok(built)
}

// ----------------------------------------------------
// SERVER-SIDE PAGE TRIGGERS
// ----------------------------------------------------

/// A page trigger, action script or control trigger resolved for execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageScript {
    /// Table the record belongs to (page name for pages without a source table).
    pub table: String,
    /// Stable trigger identifier (`onOpenPage`, `action.Post`, `control.amount.onValidate`).
    pub trigger: String,
    pub script: String,
}

/// Resolves `onOpenPage`, `action.<Name>` or `control.<Name>.<trigger>` on the merged page.
pub fn find_page_script(entry: &PageEntry, reference: &str) -> Result<PageScript, PageServiceError> {
    fn find_control<'a>(controls: &'a [PageControlIR], name: &str) -> Option<&'a PageControlIR> {
        controls.iter().find_map(|c| if c.name.eq_ignore_ascii_case(name) { Some(c) } else { find_control(&c.children, name) })
    }
    let page = &entry.merged;
    let table = page.source_table.clone().unwrap_or_else(|| page.name.clone());
    let missing = || PageServiceError::BadRequest(format!("page '{}' has no script '{reference}'", page.name));
    let script = match reference.split('.').collect::<Vec<_>>().as_slice() {
        ["action", name] => page
            .actions
            .iter()
            .find(|a| a.name.eq_ignore_ascii_case(name))
            .and_then(|a| a.script.clone())
            .ok_or_else(missing)?,
        ["control", control, trigger] => find_control(&page.controls, control)
            .and_then(|c| c.triggers.iter().find(|t| t.name.eq_ignore_ascii_case(trigger)))
            .and_then(|t| t.script.clone())
            .ok_or_else(missing)?,
        [trigger] => page
            .triggers
            .iter()
            .find(|t| t.name.eq_ignore_ascii_case(trigger))
            .and_then(|t| t.script.clone())
            .ok_or_else(missing)?,
        _ => return Err(PageServiceError::BadRequest(format!("'{reference}' is not a page trigger reference"))),
    };
    if script.trim().is_empty() {
        return Err(missing());
    }
    Ok(PageScript { table, trigger: reference.to_string(), script })
}

/// Wraps a page script so it sees `ctx.page` (name + variables) and reports UI directives and the
/// (possibly changed) page variables back on the record.
pub fn wrap_page_script(page: &str, trigger: &str, variables: &serde_json::Value, body: &str) -> String {
    let lit = |v: &serde_json::Value| v.to_string().replace("</", "<\\/");
    format!(
        "ctx.page = {{ name: {name}, trigger: {trigger}, variables: {vars} }};\n\
         const __bosPageResult = (() => {{\n{body}\n}})();\n\
         rec._page_variables = ctx.page.variables;\n\
         rec._ui_directives = ctx.ui.getDirectives();\n\
         return __bosPageResult;",
        name = lit(&serde_json::Value::from(page)),
        trigger = lit(&serde_json::Value::from(trigger)),
        vars = lit(variables),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn db_key_ignores_credentials_and_options() {
        let a = db_key("postgres://alice:secret@db.local:5432/bos?sslmode=require");
        let b = db_key("postgresql://bob:other@DB.local:5432/bos");
        assert_eq!(a, b);
        assert_ne!(a, db_key("postgres://alice@db.local:5432/other"));
        assert_eq!(a.len(), 16);
    }

    #[test]
    fn artifact_names_are_strict() {
        assert!(is_valid_artifact_name("CustomerCard.0123456789abcdef.js"));
        assert!(!is_valid_artifact_name("../etc/passwd"));
        assert!(!is_valid_artifact_name("Customer Card.0123456789abcdef.js"));
        assert!(!is_valid_artifact_name("CustomerCard.0123456789ABCDEF.js"));
        assert!(!is_valid_artifact_name("CustomerCard.0123.js"));
        assert!(!is_valid_artifact_name("CustomerCard.0123456789abcdef.js.map"));
        assert!(!is_valid_artifact_name(".0123456789abcdef.js"));
    }

    #[test]
    fn artifact_hash_depends_on_compiler_and_source() {
        let base = artifact_hash("5.57.1", "aa", "<p/>");
        assert_eq!(base, artifact_hash("5.57.1", "aa", "<p/>"));
        assert_ne!(base, artifact_hash("5.57.2", "aa", "<p/>"));
        assert_ne!(base, artifact_hash("5.57.1", "bb", "<p/>"));
        assert_ne!(base, artifact_hash("5.57.1", "aa", "<p></p>"));
    }

    #[test]
    fn atomic_writes_keep_existing_content_addressed_files() {
        let dir = std::env::temp_dir().join(format!("bos-pages-test-{}", uuid::Uuid::new_v4().simple()));
        let path = dir.join("db").join("P.0123456789abcdef.js");
        write_atomic(&path, b"first").unwrap();
        write_atomic(&path, b"second").unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"first");
        replace_file(&dir.join("manifest.json"), b"{\"a\":1}").unwrap();
        replace_file(&dir.join("manifest.json"), b"{\"a\":2}").unwrap();
        assert_eq!(std::fs::read(dir.join("manifest.json")).unwrap(), b"{\"a\":2}");
        let leftovers: Vec<_> = std::fs::read_dir(dir.join("db")).unwrap().flatten().filter(|f| f.file_name().to_string_lossy().ends_with(".tmp")).collect();
        assert!(leftovers.is_empty());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn page_scripts_are_wrapped_with_page_context() {
        let s = wrap_page_script("SignIn", "action.SignIn", &serde_json::json!({"username": "</script>"}), "return 1;");
        assert!(s.contains("ctx.page = { name: \"SignIn\""));
        assert!(s.contains("<\\/script>"));
        assert!(s.contains("rec._ui_directives = ctx.ui.getDirectives();"));
        assert!(s.trim_end().ends_with("return __bosPageResult;"));
    }
}
