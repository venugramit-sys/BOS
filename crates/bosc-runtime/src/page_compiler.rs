//! Svelte compiler worker pool.
//!
//! Generated page components are compiled with the real Svelte 5 compiler running inside QuickJS, so a
//! BOS node needs no Node.js. Each worker is a dedicated OS thread owning one QuickJS runtime that has
//! the compiler loaded once (warm). It is separate from the business-logic sandbox: no fuel metering,
//! no host bridge, a larger memory limit, and an explicit native stack large enough for the compiler's
//! deep recursion (QuickJS' own stack limit must stay below the thread stack).

use rquickjs::{CatchResultExt, Context, Runtime};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};

/// File name of the compiler bundle inside the runtime folder (copied by `ui/vite.runtime.config.js`).
pub const COMPILER_FILE: &str = "svelte-compiler.js";
/// Version descriptor written next to the compiler and the shared runtime chunks.
pub const VERSION_FILE: &str = "version.json";

const WORKER_STACK_BYTES: usize = 64 * 1024 * 1024;
const QJS_STACK_BYTES: usize = 8 * 1024 * 1024;
const QJS_MEMORY_BYTES: usize = 512 * 1024 * 1024;

const PRELUDE: &str = r#"
globalThis.self = globalThis;
globalThis.btoa = function () { throw new Error("btoa is unavailable: page modules are compiled without source maps"); };
"#;

const COMPILE: &str = r#"
(() => {
  const r = svelte.compile(globalThis.__bosSource, {
    generate: "client",
    css: "external",
    runes: true,
    dev: false,
    sourcemap: false,
    discloseVersion: false,
    filename: globalThis.__bosFilename,
  });
  return JSON.stringify({ code: r.js.code, warnings: r.warnings.map((w) => w.code + ": " + w.message) });
})()
"#;

#[derive(Debug, thiserror::Error)]
pub enum PageCompilerError {
    #[error("Svelte compiler bundle not found at {0}; build the UI runtime (`npm run build` in ui/)")]
    NotFound(PathBuf),
    #[error("failed to read {path}: {reason}")]
    Io { path: PathBuf, reason: String },
    #[error("Svelte compiler version mismatch: bundle is {bundle}, {file} declares {declared}")]
    VersionMismatch { bundle: String, declared: String, file: PathBuf },
    #[error("failed to start the Svelte compiler: {0}")]
    Startup(String),
    #[error("Svelte failed to compile page '{page}': {reason}")]
    Compile { page: String, reason: String },
    #[error("the Svelte compiler pool has shut down")]
    Closed,
}

/// Identity of the compiler in use; part of every artifact hash.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompilerInfo {
    pub svelte_version: String,
    pub compiler_sha256: String,
    pub path: PathBuf,
}

/// Output of one compilation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledModule {
    pub js: String,
    pub warnings: Vec<String>,
}

struct Job {
    name: String,
    source: String,
    reply: tokio::sync::oneshot::Sender<Result<CompiledModule, String>>,
}

pub struct SvelteCompiler {
    sender: Mutex<Option<mpsc::Sender<Job>>>,
    info: CompilerInfo,
    workers: usize,
}

impl SvelteCompiler {
    /// Resolves the compiler bundle: `<runtime_dir>/svelte-compiler.js`.
    pub fn locate(runtime_dir: &Path) -> Result<PathBuf, PageCompilerError> {
        let path = runtime_dir.join(COMPILER_FILE);
        if path.is_file() { Ok(path) } else { Err(PageCompilerError::NotFound(path)) }
    }

    /// Default worker count: one per core, at most four.
    pub fn default_workers() -> usize {
        std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1).clamp(1, 4)
    }

    /// Loads the compiler into `workers` warm QuickJS runtimes. Blocks until every worker is ready
    /// (≈0.3 s each, in parallel); call it from a blocking context.
    pub fn start(compiler_path: &Path, workers: usize) -> Result<Self, PageCompilerError> {
        let bundle = std::fs::read_to_string(compiler_path)
            .map_err(|e| PageCompilerError::Io { path: compiler_path.to_path_buf(), reason: e.to_string() })?;
        let compiler_sha256 = format!("{:x}", Sha256::digest(bundle.as_bytes()));
        let bundle = Arc::new(bundle);
        let workers = workers.max(1);

        let (tx, rx) = mpsc::channel::<Job>();
        let rx = Arc::new(Mutex::new(rx));
        let (ready_tx, ready_rx) = mpsc::channel::<Result<String, String>>();

        for i in 0..workers {
            let bundle = bundle.clone();
            let rx = rx.clone();
            let ready_tx = ready_tx.clone();
            std::thread::Builder::new()
                .name(format!("bos-svelte-{i}"))
                .stack_size(WORKER_STACK_BYTES)
                .spawn(move || worker(bundle, rx, ready_tx))
                .map_err(|e| PageCompilerError::Startup(e.to_string()))?;
        }
        drop(ready_tx);

        let mut version = None;
        for _ in 0..workers {
            match ready_rx.recv() {
                Ok(Ok(v)) => version = Some(v),
                Ok(Err(e)) => return Err(PageCompilerError::Startup(e)),
                Err(_) => return Err(PageCompilerError::Startup("a compiler worker exited during startup".into())),
            }
        }
        let svelte_version = version.ok_or_else(|| PageCompilerError::Startup("no compiler worker started".into()))?;

        // The compiler must match the shared Svelte runtime the browser loads (same UI build).
        let version_file = compiler_path.with_file_name(VERSION_FILE);
        if let Ok(text) = std::fs::read_to_string(&version_file) {
            let declared = serde_json::from_str::<serde_json::Value>(&text)
                .ok()
                .and_then(|v| v.get("svelte").and_then(|s| s.as_str()).map(str::to_string))
                .ok_or_else(|| PageCompilerError::Io { path: version_file.clone(), reason: "missing \"svelte\" version".into() })?;
            if declared != svelte_version {
                return Err(PageCompilerError::VersionMismatch { bundle: svelte_version, declared, file: version_file });
            }
        }

        Ok(SvelteCompiler {
            sender: Mutex::new(Some(tx)),
            info: CompilerInfo { svelte_version, compiler_sha256, path: compiler_path.to_path_buf() },
            workers,
        })
    }

    pub fn info(&self) -> &CompilerInfo {
        &self.info
    }

    pub fn workers(&self) -> usize {
        self.workers
    }

    /// Compiles one component to an ES module.
    pub async fn compile(&self, name: &str, source: String) -> Result<CompiledModule, PageCompilerError> {
        let (reply, wait) = tokio::sync::oneshot::channel();
        {
            let guard = self.sender.lock().map_err(|_| PageCompilerError::Closed)?;
            let sender = guard.as_ref().ok_or(PageCompilerError::Closed)?;
            sender
                .send(Job { name: name.to_string(), source, reply })
                .map_err(|_| PageCompilerError::Closed)?;
        }
        wait.await
            .map_err(|_| PageCompilerError::Closed)?
            .map_err(|reason| PageCompilerError::Compile { page: name.to_string(), reason })
    }
}

impl Drop for SvelteCompiler {
    fn drop(&mut self) {
        // Dropping the sender ends every worker loop.
        if let Ok(mut guard) = self.sender.lock() {
            guard.take();
        }
    }
}

fn worker(bundle: Arc<String>, jobs: Arc<Mutex<mpsc::Receiver<Job>>>, ready: mpsc::Sender<Result<String, String>>) {
    let setup = (|| -> Result<(Runtime, Context, String), String> {
        let rt = Runtime::new().map_err(|e| e.to_string())?;
        rt.set_memory_limit(QJS_MEMORY_BYTES);
        rt.set_max_stack_size(QJS_STACK_BYTES);
        let ctx = Context::full(&rt).map_err(|e| e.to_string())?;
        let version = ctx.with(|ctx| -> Result<String, String> {
            ctx.eval::<(), _>(PRELUDE).catch(&ctx).map_err(|e| format!("prelude: {e:?}"))?;
            ctx.eval::<(), _>(bundle.as_str()).catch(&ctx).map_err(|e| format!("loading compiler: {e:?}"))?;
            ctx.eval::<String, _>("String(svelte.VERSION)").catch(&ctx).map_err(|e| format!("reading VERSION: {e:?}"))
        })?;
        Ok((rt, ctx, version))
    })();

    let (_rt, ctx) = match setup {
        Ok((rt, ctx, version)) => {
            let _ = ready.send(Ok(version));
            (rt, ctx)
        }
        Err(e) => {
            let _ = ready.send(Err(e));
            return;
        }
    };
    drop(ready);

    loop {
        let job = {
            let Ok(rx) = jobs.lock() else { return };
            match rx.recv() {
                Ok(job) => job,
                Err(_) => return,
            }
        };
        let result = ctx.with(|ctx| -> Result<CompiledModule, String> {
            let globals = ctx.globals();
            globals.set("__bosSource", job.source.as_str()).map_err(|e| e.to_string())?;
            globals.set("__bosFilename", format!("{}.svelte", job.name)).map_err(|e| e.to_string())?;
            let out: String = ctx.eval(COMPILE).catch(&ctx).map_err(|e| format!("{e:?}"))?;
            let value: serde_json::Value = serde_json::from_str(&out).map_err(|e| e.to_string())?;
            let js = value["code"].as_str().ok_or("compiler returned no code")?.to_string();
            let warnings = value["warnings"]
                .as_array()
                .map(|w| w.iter().filter_map(|x| x.as_str().map(str::to_string)).collect())
                .unwrap_or_default();
            let _ = globals.set("__bosSource", "");
            Ok(CompiledModule { js, warnings })
        });
        let _ = job.reply.send(result);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn compiler_path() -> Option<PathBuf> {
        let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../ui/node_modules/svelte/compiler/index.js");
        p.is_file().then_some(p)
    }

    #[test]
    fn locate_reports_missing_bundle() {
        let err = SvelteCompiler::locate(Path::new("definitely/missing/dir")).unwrap_err();
        assert!(matches!(err, PageCompilerError::NotFound(_)));
    }

    #[tokio::test]
    async fn compiles_runes_components_in_parallel() {
        let Some(path) = compiler_path() else {
            panic!("ui/node_modules/svelte is required for this test: run `npm ci` in ui/");
        };
        let compiler = tokio::task::spawn_blocking(move || SvelteCompiler::start(&path, 2)).await.unwrap().unwrap();
        assert!(compiler.info().svelte_version.starts_with('5'));
        assert_eq!(compiler.info().compiler_sha256.len(), 64);
        let compiler = Arc::new(compiler);

        let mut tasks = Vec::new();
        for i in 0..4 {
            let c = compiler.clone();
            tasks.push(tokio::spawn(async move {
                c.compile(&format!("P{i}"), format!("<script>let {{ n }} = $props(); const d = $derived(n * {i});</script><p>{{d}}</p>")).await
            }));
        }
        for t in tasks {
            let m = t.await.unwrap().unwrap();
            assert!(m.js.contains("svelte/internal/client"));
            assert!(m.warnings.is_empty(), "{:?}", m.warnings);
        }

        let err = compiler.compile("Broken", "<script>let x = ;</script>".into()).await.unwrap_err();
        assert!(matches!(err, PageCompilerError::Compile { .. }), "{err}");
        // The worker survives a failed compilation.
        assert!(compiler.compile("Again", "<p>ok</p>".into()).await.is_ok());
    }
}
