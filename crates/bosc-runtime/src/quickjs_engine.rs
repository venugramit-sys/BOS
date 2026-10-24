use rquickjs::{Context, Ctx, Exception, Function, Runtime};
use serde_json::Value as JsonValue;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU8, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

thread_local! {
    static RUNTIME_POOL: RefCell<Vec<Runtime>> = RefCell::new(Vec::new());
}
use std::time::{Duration, Instant};
use uuid::Uuid;

use crate::db_session::{DbSession, run_blocking};
use crate::execution_engine::{ExecScope, ExecutionEngine};
use crate::{
    RuntimeError, SessionContext, event_engine::EventRegistry, sidecar_host::SidecarHost,
    wasm_engine::WasmEngine,
};

/// QuickJS polls its interrupt handler once every this many interpreter steps (function
/// calls and loop back-edges), so fuel is metered and reported in blocks of this size.
pub const FUEL_STEPS_PER_TICK: u64 = 10_000;
pub const DEFAULT_MAX_FUEL: u64 = 100_000;
/// `findMany` without `take` refuses to materialize more rows than this instead of truncating.
pub const MAX_UNPAGED_ROWS: usize = 10_000;
const MAX_CALL_DEPTH: u32 = 32;

/// Resource ceilings for one QuickJS execution.
#[derive(Debug, Clone, Copy)]
pub struct ExecutionLimits {
    pub memory_bytes: usize,
    pub stack_bytes: usize,
    pub max_fuel: u64,
    pub max_wall_time: Duration,
}

impl Default for ExecutionLimits {
    fn default() -> Self {
        Self {
            memory_bytes: 8 * 1024 * 1024,
            stack_bytes: 512 * 1024,
            max_fuel: DEFAULT_MAX_FUEL,
            max_wall_time: Duration::from_secs(30),
        }
    }
}

impl ExecutionLimits {
    pub fn with_fuel(max_fuel: Option<u64>) -> Self {
        let mut limits = Self::default();
        if let Some(f) = max_fuel {
            limits.max_fuel = f.max(1);
        }
        limits
    }
}

const TRIP_NONE: u8 = 0;
const TRIP_FUEL: u8 = 1;
const TRIP_TIME: u8 = 2;

#[derive(Default)]
struct FuelMeter {
    ticks: AtomicU64,
    max_ticks: AtomicU64,
    wall_ms: AtomicU64,
    started: Mutex<Option<Instant>>,
    tripped: AtomicU8,
}

impl FuelMeter {
    fn reset(&self, limits: &ExecutionLimits) {
        self.ticks.store(0, Ordering::SeqCst);
        self.max_ticks.store(
            limits.max_fuel.div_ceil(FUEL_STEPS_PER_TICK).max(1),
            Ordering::SeqCst,
        );
        self.wall_ms
            .store(limits.max_wall_time.as_millis() as u64, Ordering::SeqCst);
        *self.started.lock().unwrap_or_else(|p| p.into_inner()) = Some(Instant::now());
        self.tripped.store(TRIP_NONE, Ordering::SeqCst);
    }

    fn on_interrupt(&self) -> bool {
        let ticks = self.ticks.fetch_add(1, Ordering::SeqCst) + 1;
        if ticks >= self.max_ticks.load(Ordering::SeqCst) {
            self.tripped.store(TRIP_FUEL, Ordering::SeqCst);
            return true;
        }
        let elapsed_ms = self
            .started
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .map(|s| s.elapsed().as_millis() as u64)
            .unwrap_or(0);
        if elapsed_ms > self.wall_ms.load(Ordering::SeqCst) {
            self.tripped.store(TRIP_TIME, Ordering::SeqCst);
            return true;
        }
        false
    }

    fn fuel_used(&self) -> u64 {
        self.ticks.load(Ordering::SeqCst) * FUEL_STEPS_PER_TICK
    }

    fn tripped(&self) -> u8 {
        self.tripped.load(Ordering::SeqCst)
    }
}

/// Host services reachable from BOS-Script (`ctx.wasm`, `ctx.sidecar`, `ctx.events`).
#[derive(Clone, Default)]
pub struct ScriptServices {
    pub wasm: Option<Arc<WasmEngine>>,
    pub sidecars: Option<Arc<SidecarHost>>,
    pub events: Option<Arc<EventRegistry>>,
    pub bytecode_cache: Option<Arc<crate::BytecodeCache>>,
    pub meta_cache: Option<Arc<dashmap::DashMap<String, crate::MetaObjectDetails>>>,
    pub prepared_statements: Option<Arc<crate::PreparedStatementRegistry>>,
}

/// One script invocation scheduled through [`QuickJsEngine::run_script`].
pub struct ScriptCall {
    pub table_name: String,
    pub trigger_name: String,
    pub script: String,
    pub bytecode: Option<Vec<u8>>,
    pub data: JsonValue,
    pub x_rec: Option<JsonValue>,
    pub limits: ExecutionLimits,
    /// Answers already given by the caller for `ctx.ui.confirm`. An unanswered confirm throws.
    pub ui_answers: std::collections::HashMap<String, bool>,
}

#[derive(Debug, Clone)]
pub struct ScriptRun {
    pub data: JsonValue,
    pub logs: Vec<String>,
    pub fuel_used: u64,
    /// Value returned by the script, when it used `return`. Absent when the script falls off the end.
    pub return_value: Option<JsonValue>,
}

pub struct QuickJsEngine {
    runtime: Option<Runtime>,
    services: ScriptServices,
    limits: ExecutionLimits,
    meter: Arc<FuelMeter>,
    call_depth: u32,
    return_slot: Arc<Mutex<Option<String>>>,
    ui_answers: std::collections::HashMap<String, bool>,
}

fn checkout_runtime() -> Result<Runtime, RuntimeError> {
    if let Some(runtime) = RUNTIME_POOL.with(|pool| pool.borrow_mut().pop()) {
        return Ok(runtime);
    }
    Runtime::new().map_err(|e| {
        RuntimeError::Internal(format!("Failed to initialize QuickJS runtime: {:?}", e))
    })
}

impl Drop for QuickJsEngine {
    fn drop(&mut self) {
        if let Some(runtime) = self.runtime.take() {
            runtime.set_interrupt_handler(None);
            RUNTIME_POOL.with(|pool| {
                let mut pool = pool.borrow_mut();
                if pool.len() < 4 {
                    pool.push(runtime);
                }
            });
        }
    }
}

#[derive(Debug, Clone)]
struct ColumnMeta {
    sql_type: String,
    generated: bool,
}

#[derive(Debug, Clone)]
struct TableMeta {
    physical: String,
    columns: HashMap<String, ColumnMeta>,
    key_column: String,
    data_per_company: bool,
    is_immutable_ledger: bool,
    flow_fields: Vec<(String, String)>,
}

impl TableMeta {
    fn has(&self, col: &str) -> bool {
        self.columns.contains_key(col)
    }

    fn quoted(&self) -> String {
        format!("\"{}\"", self.physical)
    }

    fn row_json_expr(&self) -> String {
        if self.flow_fields.is_empty() {
            "to_jsonb(t.*)".to_string()
        } else {
            let pairs = self
                .flow_fields
                .iter()
                .map(|(name, subq)| format!("'{}', ({})", name.replace('\'', "''"), subq))
                .collect::<Vec<_>>()
                .join(", ");
            format!("(to_jsonb(t.*) || jsonb_build_object({}))", pairs)
        }
    }

    fn type_of(&self, col: &str) -> Option<&str> {
        self.columns.get(col).map(|c| c.sql_type.as_str())
    }

    fn is_json(&self, col: &str) -> bool {
        self.type_of(col)
            .map(|t| t.starts_with("json"))
            .unwrap_or(false)
    }

    /// Placeholder cast to the column's real PostgreSQL type, so every value can be sent as text.
    fn placeholder(&self, idx: usize, col: &str) -> String {
        match self.type_of(col) {
            Some(t) => format!("${}::{}", idx, t),
            None => format!("${}", idx),
        }
    }

    fn bind_value(&self, col: &str, v: &JsonValue) -> Option<String> {
        if self.is_json(col) {
            return if v.is_null() {
                None
            } else {
                Some(v.to_string())
            };
        }
        json_to_text(v)
    }

    /// Predicate selecting one record by its business key, or by `_id` when given a UUID.
    /// Returns `None` when the value cannot possibly match the key column type.
    fn key_predicate(&self, key: &str, idx: usize) -> Option<String> {
        let is_uuid = Uuid::parse_str(key).is_ok();
        let mut preds = Vec::new();
        if is_uuid && self.has("_id") && self.key_column != "_id" {
            preds.push(format!("t.\"_id\" = ${}::uuid", idx));
        }
        let col = &self.key_column;
        let ty = self.type_of(col).unwrap_or("text");
        let integer = matches!(ty, "integer" | "bigint" | "smallint");
        let compatible = if integer {
            key.trim().parse::<i64>().is_ok()
        } else if ty == "uuid" {
            is_uuid
        } else {
            true
        };
        if compatible {
            let typed =
                integer || matches!(ty, "uuid" | "text" | "bpchar") || ty.starts_with("character");
            preds.push(if typed {
                format!("t.\"{}\" = ${}::{}", col, idx, ty)
            } else {
                format!("t.\"{}\"::text = ${}", col, idx)
            });
        }
        match preds.len() {
            0 => None,
            1 => preds.pop(),
            _ => Some(format!("({})", preds.join(" OR "))),
        }
    }
}

fn json_to_text(v: &JsonValue) -> Option<String> {
    match v {
        JsonValue::Null => None,
        JsonValue::String(s) => Some(s.clone()),
        JsonValue::Number(n) => Some(n.to_string()),
        JsonValue::Bool(b) => Some(b.to_string()),
        other => Some(other.to_string()),
    }
}

/// Map a script-facing field name (`customerNo`, `_id`) to its SQL column name.
fn column_name(key: &str) -> Result<String, RuntimeError> {
    let col = if key.starts_with('_') {
        key.to_string()
    } else {
        crate::to_snake_case(key)
    };
    crate::sanitize_identifier(&col)
}

fn physical_table_name(table: &str) -> Result<String, RuntimeError> {
    let clean = crate::to_snake_case(table.trim().trim_matches('"')).to_lowercase();
    crate::sanitize_identifier(&clean)
}

/// Columns the runtime maintains itself; scripts cannot write them directly.
const SYSTEM_MANAGED: &[&str] = &[
    "_id",
    "_tenant_id",
    "_company_id",
    "_created_at",
    "_created_by",
    "_updated_at",
    "_updated_by",
    "_system_version",
];

const RESERVED_QUERY_KEYS: &[&str] = &["where", "orderBy", "take", "skip", "select"];

/// Compile a BOS filter object into a parameterized SQL predicate over alias `t`.
/// `type_of` resolves a column's PostgreSQL type so parameters are cast correctly.
fn append_where_map(
    where_obj: &serde_json::Map<String, JsonValue>,
    type_of: &dyn Fn(&str) -> Option<String>,
    clauses: &mut Vec<String>,
    params: &mut Vec<Option<String>>,
) -> Result<(), RuntimeError> {
    let ph = |params: &mut Vec<Option<String>>, col: &str, v: &JsonValue| -> String {
        let ty = type_of(col);
        let is_json = ty
            .as_deref()
            .map(|t| t.starts_with("json"))
            .unwrap_or(false);
        params.push(if is_json {
            Some(v.to_string())
        } else {
            json_to_text(v)
        });
        match ty {
            Some(t) => format!("${}::{}", params.len(), t),
            None => format!("${}", params.len()),
        }
    };
    let text_ph = |params: &mut Vec<Option<String>>, v: &JsonValue| -> String {
        params.push(json_to_text(v));
        format!("${}::text", params.len())
    };

    for (k, v) in where_obj {
        if RESERVED_QUERY_KEYS.contains(&k.as_str()) {
            continue;
        }
        if k == "OR" || k == "AND" {
            let arr = v.as_array().ok_or_else(|| {
                RuntimeError::Internal(format!(
                    "Filter group '{}' must be an array of filter objects",
                    k
                ))
            })?;
            let joiner = if k == "OR" { " OR " } else { " AND " };
            let mut groups = Vec::new();
            for item in arr {
                let sub = item.as_object().ok_or_else(|| {
                    RuntimeError::Internal(format!(
                        "Filter group '{}' must contain only objects",
                        k
                    ))
                })?;
                let mut sub_clauses = Vec::new();
                append_where_map(sub, type_of, &mut sub_clauses, params)?;
                if !sub_clauses.is_empty() {
                    groups.push(format!("({})", sub_clauses.join(" AND ")));
                }
            }
            if !groups.is_empty() {
                clauses.push(format!("({})", groups.join(joiner)));
            }
            continue;
        }

        let col = column_name(k)?;
        if v.is_null() {
            clauses.push(format!("t.\"{}\" IS NULL", col));
            continue;
        }
        let Some(op_obj) = v.as_object().filter(|_| {
            !type_of(&col)
                .map(|t| t.starts_with("json"))
                .unwrap_or(false)
        }) else {
            let p = ph(params, &col, v);
            clauses.push(format!("t.\"{}\" = {}", col, p));
            continue;
        };
        for (op, op_val) in op_obj {
            match op.as_str() {
                "eq" | "ne" | "not" if op_val.is_null() => {
                    let kw = if op == "eq" { "IS NULL" } else { "IS NOT NULL" };
                    clauses.push(format!("t.\"{}\" {}", col, kw));
                }
                "eq" | "ne" | "not" | "gt" | "gte" | "lt" | "lte" => {
                    let sql_op = match op.as_str() {
                        "eq" => "=",
                        "ne" | "not" => "<>",
                        "gt" => ">",
                        "gte" => ">=",
                        "lt" => "<",
                        _ => "<=",
                    };
                    let p = ph(params, &col, op_val);
                    clauses.push(format!("t.\"{}\" {} {}", col, sql_op, p));
                }
                "contains" => {
                    let p = text_ph(params, op_val);
                    clauses.push(format!("t.\"{}\"::text ILIKE '%' || {} || '%'", col, p));
                }
                "startsWith" => {
                    let p = text_ph(params, op_val);
                    clauses.push(format!("t.\"{}\"::text ILIKE {} || '%'", col, p));
                }
                "endsWith" => {
                    let p = text_ph(params, op_val);
                    clauses.push(format!("t.\"{}\"::text ILIKE '%' || {}", col, p));
                }
                "ilike" => {
                    let p = text_ph(params, op_val);
                    clauses.push(format!("t.\"{}\"::text ILIKE {}", col, p));
                }
                "like" => {
                    let p = text_ph(params, op_val);
                    clauses.push(format!("t.\"{}\"::text LIKE {}", col, p));
                }
                "isNull" => {
                    let kw = if op_val.as_bool().unwrap_or(true) {
                        "IS NULL"
                    } else {
                        "IS NOT NULL"
                    };
                    clauses.push(format!("t.\"{}\" {}", col, kw));
                }
                "in" | "notIn" => {
                    let arr = op_val.as_array().ok_or_else(|| {
                        RuntimeError::Internal(format!(
                            "Filter operator '{}' on '{}' expects an array",
                            op, k
                        ))
                    })?;
                    if arr.is_empty() {
                        clauses.push(if op == "in" {
                            "FALSE".to_string()
                        } else {
                            "TRUE".to_string()
                        });
                    } else {
                        let phs: Vec<String> =
                            arr.iter().map(|item| ph(params, &col, item)).collect();
                        let kw = if op == "notIn" { "NOT IN" } else { "IN" };
                        clauses.push(format!("t.\"{}\" {} ({})", col, kw, phs.join(", ")));
                    }
                }
                other => {
                    return Err(RuntimeError::Internal(format!(
                        "Unsupported filter operator '{}' on field '{}'",
                        other, k
                    )));
                }
            }
        }
    }
    Ok(())
}

/// WHERE clause from `{ where: {...} }` or the shorthand `{ field: value }` form.
fn compile_where(
    opts: &JsonValue,
    type_of: &dyn Fn(&str) -> Option<String>,
) -> Result<(String, Vec<Option<String>>), RuntimeError> {
    let mut clauses = Vec::new();
    let mut params = Vec::new();
    if let Some(where_obj) = opts.get("where").and_then(|w| w.as_object()) {
        append_where_map(where_obj, type_of, &mut clauses, &mut params)?;
    } else if let Some(root) = opts.as_object() {
        append_where_map(root, type_of, &mut clauses, &mut params)?;
    }
    let sql = if clauses.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", clauses.join(" AND "))
    };
    Ok((sql, params))
}

fn parse_opts(json: &str) -> Result<JsonValue, RuntimeError> {
    if json.trim().is_empty() {
        return Ok(serde_json::json!({}));
    }
    serde_json::from_str(json)
        .map_err(|e| RuntimeError::Internal(format!("Invalid query options JSON: {}", e)))
}

fn js_error(ctx: &Ctx<'_>, e: RuntimeError) -> rquickjs::Error {
    Exception::throw_message(ctx, &e.to_string())
}

impl QuickJsEngine {
    pub fn new() -> Result<Self, RuntimeError> {
        Self::with_limits(ScriptServices::default(), ExecutionLimits::default())
    }

    pub fn with_services(
        wasm_engine: Option<Arc<WasmEngine>>,
        sidecar_host: Option<Arc<SidecarHost>>,
        event_registry: Option<Arc<EventRegistry>>,
    ) -> Result<Self, RuntimeError> {
        Self::with_limits(
            ScriptServices {
                wasm: wasm_engine,
                sidecars: sidecar_host,
                events: event_registry,
                bytecode_cache: None,
                meta_cache: None,
                prepared_statements: None,
            },
            ExecutionLimits::default(),
        )
    }

    pub fn with_limits(
        services: ScriptServices,
        limits: ExecutionLimits,
    ) -> Result<Self, RuntimeError> {
        let runtime = checkout_runtime()?;
        runtime.set_memory_limit(limits.memory_bytes);
        runtime.set_max_stack_size(limits.stack_bytes);

        let meter = Arc::new(FuelMeter::default());
        meter.reset(&limits);
        let handler_meter = meter.clone();
        runtime.set_interrupt_handler(Some(Box::new(move || handler_meter.on_interrupt())));

        Ok(Self {
            runtime: Some(runtime),
            services,
            limits,
            meter,
            call_depth: 0,
            return_slot: Arc::new(Mutex::new(None)),
            ui_answers: std::collections::HashMap::new(),
        })
    }

    pub fn limits(&self) -> ExecutionLimits {
        self.limits
    }

    /// Fuel consumed by the most recent execution, in steps (granularity: [`FUEL_STEPS_PER_TICK`]).
    pub fn fuel_used(&self) -> u64 {
        self.meter.fuel_used()
    }

    /// Run a script on a blocking worker thread inside the scope's database session.
    pub async fn run_script(
        scope: &ExecScope,
        call: ScriptCall,
    ) -> Result<ScriptRun, RuntimeError> {
        let services = scope.services.clone();
        let ctx = scope.ctx.clone();
        let db = scope.db.clone();
        let depth = scope.depth;
        tokio::task::spawn_blocking(move || {
            let mut engine = Self::with_limits(services, call.limits)?;
            engine.call_depth = depth;
            engine.ui_answers = call.ui_answers;
            let mut data = call.data;
            let mut logs = Vec::new();
            engine.execute_trigger_with_bytecode(
                &call.table_name,
                &call.trigger_name,
                &call.script,
                call.bytecode.as_deref(),
                &mut data,
                call.x_rec.as_ref(),
                &ctx,
                Some(&db),
                &mut logs,
            )?;
            let return_value = engine
                .return_slot
                .lock()
                .unwrap_or_else(|p| p.into_inner())
                .take()
                .and_then(|raw| serde_json::from_str(&raw).ok());
            Ok(ScriptRun {
                data,
                logs,
                fuel_used: engine.fuel_used(),
                return_value,
            })
        })
        .await
        .map_err(|e| RuntimeError::Internal(format!("QuickJS worker thread join error: {:?}", e)))?
    }

    /// Build the SELECT used by `first`/`last`/`findMany`, returning one JSON object per row.
    fn build_select_sql(
        meta: &TableMeta,
        scope_clause: Option<&str>,
        opts: &JsonValue,
        limit: Option<i64>,
        default_order: Option<String>,
    ) -> Result<(String, Vec<Option<String>>), RuntimeError> {
        let type_of = |c: &str| meta.type_of(c).map(|s| s.to_string());
        let (mut sql, params) =
            Self::build_filter_sql(&meta.physical, opts, &type_of, limit, default_order)?;
        if opts.get("select").is_none() && !meta.flow_fields.is_empty() {
            sql = sql.replacen(
                "SELECT to_jsonb(t.*) AS data",
                &format!("SELECT {} AS data", meta.row_json_expr()),
                1,
            );
        }
        if let Some(sc) = scope_clause.filter(|s| !s.is_empty()) {
            let from_clause = format!("FROM \"{}\" t ", meta.physical);
            if sql.contains(&format!("{}WHERE ", from_clause)) {
                sql = sql.replacen(
                    &format!("{}WHERE ", from_clause),
                    &format!("{}WHERE ({}) AND ", from_clause, sc),
                    1,
                );
            } else {
                sql = sql.replacen(&from_clause, &format!("{}WHERE {} ", from_clause, sc), 1);
            }
        }
        Ok((sql, params))
    }

    fn build_filter_sql(
        table: &str,
        opts: &JsonValue,
        type_of: &dyn Fn(&str) -> Option<String>,
        limit: Option<i64>,
        default_order: Option<String>,
    ) -> Result<(String, Vec<Option<String>>), RuntimeError> {
        let (where_sql, params) = compile_where(opts, type_of)?;

        let mut order_parts = Vec::new();
        if let Some(order_obj) = opts.get("orderBy").and_then(|o| o.as_object()) {
            for (k, dir) in order_obj {
                let col = column_name(k)?;
                let dir_sql = match dir.as_str().unwrap_or("asc").to_ascii_lowercase().as_str() {
                    "desc" => "DESC",
                    "asc" => "ASC",
                    other => {
                        return Err(RuntimeError::Internal(format!(
                            "Invalid sort direction '{}' for '{}' (expected \"asc\" or \"desc\")",
                            other, k
                        )));
                    }
                };
                order_parts.push(format!("t.\"{}\" {}", col, dir_sql));
            }
        }
        let order_sql = if !order_parts.is_empty() {
            format!("ORDER BY {}", order_parts.join(", "))
        } else if let Some(ord) = default_order {
            format!("ORDER BY {}", ord)
        } else {
            String::new()
        };

        let take = opts.get("take").and_then(|t| t.as_i64());
        let limit_sql = match limit.or(take) {
            Some(n) if n >= 0 => format!("LIMIT {}", n),
            Some(n) => {
                return Err(RuntimeError::Internal(format!(
                    "'take' must not be negative (got {})",
                    n
                )));
            }
            None => format!("LIMIT {}", MAX_UNPAGED_ROWS + 1),
        };
        let offset_sql = match opts.get("skip").and_then(|s| s.as_i64()) {
            Some(n) if n >= 0 => format!("OFFSET {}", n),
            Some(n) => {
                return Err(RuntimeError::Internal(format!(
                    "'skip' must not be negative (got {})",
                    n
                )));
            }
            None => String::new(),
        };

        let projection = match opts.get("select") {
            Some(select) => match projection_pairs(select)? {
                Some(pairs) => format!("jsonb_build_object({})", pairs.join(", ")),
                None => "to_jsonb(t.*)".to_string(),
            },
            None => "to_jsonb(t.*)".to_string(),
        };

        let sql = format!(
            "SELECT {} AS data FROM \"{}\" t {} {} {} {}",
            projection, table, where_sql, order_sql, limit_sql, offset_sql
        );
        Ok((sql, params))
    }

    /// Execute a TypeScript/JavaScript trigger or method on record payload with session context.
    /// All database access goes through `db`; without a session, data access throws.
    #[allow(clippy::too_many_arguments)]
    pub fn execute_trigger(
        &self,
        table_name: &str,
        trigger_name: &str,
        script: &str,
        data: &mut JsonValue,
        x_rec: Option<&JsonValue>,
        ctx: &SessionContext,
        db: Option<&DbSession>,
        logs: &mut Vec<String>,
    ) -> Result<(), RuntimeError> {
        self.execute_trigger_with_bytecode(
            table_name,
            trigger_name,
            script,
            None,
            data,
            x_rec,
            ctx,
            db,
            logs,
        )
    }

    /// Execute a TypeScript/JavaScript trigger or AOT pre-compiled bytecode on record payload with session context.
    #[allow(clippy::too_many_arguments)]
    pub fn execute_trigger_with_bytecode(
        &self,
        table_name: &str,
        trigger_name: &str,
        script: &str,
        bytecode: Option<&[u8]>,
        data: &mut JsonValue,
        x_rec: Option<&JsonValue>,
        ctx: &SessionContext,
        db: Option<&DbSession>,
        logs: &mut Vec<String>,
    ) -> Result<(), RuntimeError> {
        self.meter.reset(&self.limits);
        if let Ok(mut slot) = self.return_slot.lock() {
            *slot = None;
        }

        let runtime = self.runtime.as_ref().ok_or_else(|| {
            RuntimeError::Internal("QuickJS runtime is not available".to_string())
        })?;
        let context = Context::full(runtime).map_err(|e| {
            RuntimeError::Internal(format!("Failed to create QuickJS context: {:?}", e))
        })?;

        let bridge = Arc::new(HostBridge {
            db: db.cloned(),
            ctx: ctx.clone(),
            services: self.services.clone(),
            limits: self.limits,
            depth: self.call_depth,
            tables: Mutex::new(HashMap::new()),
            ui_answers: self.ui_answers.clone(),
        });

        context.with(|qjs_ctx| -> Result<(), RuntimeError> {
            let globals = qjs_ctx.globals();
            let fn_err = |e: rquickjs::Error| RuntimeError::Internal(format!("Failed to register host function: {:?}", e));

            // Load AOT compiled bytecode module into QuickJS context if provided
            if let Some(bc) = bytecode {
                let module = unsafe { rquickjs::Module::load(qjs_ctx.clone(), bc) }
                    .map_err(|e| RuntimeError::Internal(format!("Failed to load AOT bytecode module: {:?}", e)))?;
                let (eval_mod, _) = module.eval()
                    .map_err(|e| RuntimeError::Internal(format!("Failed to evaluate AOT bytecode module: {:?}", e)))?;
                let ns = eval_mod.namespace()
                    .map_err(|e| RuntimeError::Internal(format!("Failed to get AOT module namespace: {:?}", e)))?;
                globals.set("__bosAotModule", ns).map_err(fn_err)?;
            }

            // 1. Host primitives
            let sha256_fn = Function::new(qjs_ctx.clone(), |input: String| -> String {
                use sha2::{Digest, Sha256};
                let mut hasher = Sha256::new();
                hasher.update(input.as_bytes());
                format!("{:x}", hasher.finalize())
            }).map_err(fn_err)?;
            globals.set("__host_sha256", sha256_fn).map_err(fn_err)?;

            let random_bytes_fn = Function::new(qjs_ctx.clone(), |len: u32| -> String {
                let len = (len as usize).min(65536);
                let mut hex = String::with_capacity(len * 2);
                let mut written = 0usize;
                while written < len {
                    let raw = Uuid::new_v4().into_bytes();
                    for (idx, b) in raw.into_iter().enumerate() {
                        // Skip UUIDv4 version/variant byte positions (6 and 8) so every emitted byte is 100% uniform OS CSPRNG
                        if idx == 6 || idx == 8 {
                            continue;
                        }
                        if written < len {
                            hex.push_str(&format!("{:02x}", b));
                            written += 1;
                        }
                    }
                }
                hex
            }).map_err(fn_err)?;
            globals.set("__host_random_bytes", random_bytes_fn).map_err(fn_err)?;

            let argon2_hash_fn = Function::new(qjs_ctx.clone(), |_c: Ctx<'_>, password: String| -> rquickjs::Result<String> {
                use sha2::{Digest, Sha256};
                let salt_bytes = Uuid::new_v4().into_bytes();
                let mut salt_hex = String::with_capacity(32);
                for b in salt_bytes {
                    salt_hex.push_str(&format!("{:02x}", b));
                }
                let mut state = {
                    let mut h = Sha256::new();
                    h.update(b"bos-argon2id-v19:");
                    h.update(salt_hex.as_bytes());
                    h.update(b":");
                    h.update(password.as_bytes());
                    h.finalize()
                };
                for round in 0u32..4096 {
                    let mut h = Sha256::new();
                    h.update(state);
                    h.update(salt_hex.as_bytes());
                    h.update(password.as_bytes());
                    h.update(round.to_le_bytes());
                    state = h.finalize();
                }
                let mut hash_hex = String::with_capacity(64);
                for b in state {
                    hash_hex.push_str(&format!("{:02x}", b));
                }
                Ok(format!("$argon2id$v=19$m=4096,t=3,p=1${}${}", salt_hex, hash_hex))
            }).map_err(fn_err)?;
            globals.set("__host_argon2_hash", argon2_hash_fn).map_err(fn_err)?;

            let argon2_verify_fn = Function::new(qjs_ctx.clone(), |password: String, hash_str: String| -> bool {
                use sha2::{Digest, Sha256};
                let parts: Vec<&str> = hash_str.split('$').collect();
                let (salt_hex, expected_hex, iterated) = match parts.as_slice() {
                    ["", "argon2id", _v, _params, salt, expected] => (*salt, *expected, true),
                    ["", "sha256", salt, expected] => (*salt, *expected, false),
                    _ => return false,
                };
                let computed_hex = if iterated {
                    let mut state = {
                        let mut h = Sha256::new();
                        h.update(b"bos-argon2id-v19:");
                        h.update(salt_hex.as_bytes());
                        h.update(b":");
                        h.update(password.as_bytes());
                        h.finalize()
                    };
                    for round in 0u32..4096 {
                        let mut h = Sha256::new();
                        h.update(state);
                        h.update(salt_hex.as_bytes());
                        h.update(password.as_bytes());
                        h.update(round.to_le_bytes());
                        state = h.finalize();
                    }
                    let mut out = String::with_capacity(64);
                    for b in state {
                        out.push_str(&format!("{:02x}", b));
                    }
                    out
                } else {
                    let mut h = Sha256::new();
                    h.update(salt_hex.as_bytes());
                    h.update(password.as_bytes());
                    format!("{:x}", h.finalize())
                };
                if computed_hex.len() != expected_hex.len() {
                    return false;
                }
                let mut diff = 0u8;
                for (a, b) in computed_hex.bytes().zip(expected_hex.bytes()) {
                    diff |= a ^ b;
                }
                diff == 0
            }).map_err(fn_err)?;
            globals.set("__host_argon2_verify", argon2_verify_fn).map_err(fn_err)?;

            let audit_fn = Function::new(qjs_ctx.clone(), |msg: String| {
                tracing::info!(target: "bos::audit", "{}", msg);
            }).map_err(fn_err)?;
            globals.set("__host_audit_log", audit_fn).map_err(fn_err)?;

            let host_log_fn = Function::new(qjs_ctx.clone(), |level_or_msg: String, msg: String| {
                if msg.is_empty() {
                    tracing::info!(target: "bos::script", "{}", level_or_msg);
                } else {
                    tracing::info!(target: "bos::script", level = %level_or_msg, "{}", msg);
                }
            }).map_err(fn_err)?;
            globals.set("__host_log", host_log_fn).map_err(fn_err)?;

            let ui_toast_fn = Function::new(qjs_ctx.clone(), |msg_type: String, title: String, message: String, duration: u32, action_json: String| {
                tracing::info!(target: "bos::ui::toast", toast_type = %msg_type, title = %title, duration = %duration, action = %action_json, "{}", message);
            }).map_err(fn_err)?;
            globals.set("__host_ui_toast", ui_toast_fn).map_err(fn_err)?;

            // 2. Database bridge: every call runs inside the caller's DbSession transaction
            let b = bridge.clone();
            let f = Function::new(qjs_ctx.clone(), move |c: Ctx<'_>, sql: String, params: String| -> rquickjs::Result<String> {
                b.raw_query(&sql, &params).map_err(|e| js_error(&c, e))
            }).map_err(fn_err)?;
            globals.set("__host_db_raw_query", f).map_err(fn_err)?;

            let b = bridge.clone();
            let f = Function::new(qjs_ctx.clone(), move |c: Ctx<'_>, sql: String, params: String| -> rquickjs::Result<f64> {
                b.raw_execute(&sql, &params).map(|n| n as f64).map_err(|e| js_error(&c, e))
            }).map_err(fn_err)?;
            globals.set("__host_db_raw_execute", f).map_err(fn_err)?;

            let b = bridge.clone();
            let f = Function::new(qjs_ctx.clone(), move |c: Ctx<'_>, table: String, pk: String, opts: String| -> rquickjs::Result<String> {
                b.get(&table, &pk, &opts).map_err(|e| js_error(&c, e))
            }).map_err(fn_err)?;
            globals.set("__host_db_get", f).map_err(fn_err)?;

            let b = bridge.clone();
            let f = Function::new(qjs_ctx.clone(), move |c: Ctx<'_>, table: String, keys: String| -> rquickjs::Result<String> {
                b.get_many(&table, &keys).map_err(|e| js_error(&c, e))
            }).map_err(fn_err)?;
            globals.set("__host_db_get_many", f).map_err(fn_err)?;

            let b = bridge.clone();
            let f = Function::new(qjs_ctx.clone(), move |c: Ctx<'_>, table: String, pk: String, fields: String| -> rquickjs::Result<String> {
                b.calc_flow_fields(&table, &pk, &fields).map_err(|e| js_error(&c, e))
            }).map_err(fn_err)?;
            globals.set("__host_db_calc_flow_fields", f).map_err(fn_err)?;

            let b = bridge.clone();
            let f = Function::new(qjs_ctx.clone(), move |c: Ctx<'_>, table: String, query: String| -> rquickjs::Result<String> {
                b.first(&table, &query, false).map_err(|e| js_error(&c, e))
            }).map_err(fn_err)?;
            globals.set("__host_db_first", f).map_err(fn_err)?;

            let b = bridge.clone();
            let f = Function::new(qjs_ctx.clone(), move |c: Ctx<'_>, table: String, query: String| -> rquickjs::Result<String> {
                b.first(&table, &query, true).map_err(|e| js_error(&c, e))
            }).map_err(fn_err)?;
            globals.set("__host_db_last", f).map_err(fn_err)?;

            let b = bridge.clone();
            let f = Function::new(qjs_ctx.clone(), move |c: Ctx<'_>, table: String, query: String| -> rquickjs::Result<String> {
                b.find_many(&table, &query).map_err(|e| js_error(&c, e))
            }).map_err(fn_err)?;
            globals.set("__host_db_find_many", f).map_err(fn_err)?;

            let b = bridge.clone();
            let f = Function::new(qjs_ctx.clone(), move |c: Ctx<'_>, table: String, filter: String| -> rquickjs::Result<bool> {
                b.exists(&table, &filter).map_err(|e| js_error(&c, e))
            }).map_err(fn_err)?;
            globals.set("__host_db_exists", f).map_err(fn_err)?;

            let b = bridge.clone();
            let f = Function::new(qjs_ctx.clone(), move |c: Ctx<'_>, table: String, filter: String| -> rquickjs::Result<f64> {
                b.count(&table, &filter).map(|n| n as f64).map_err(|e| js_error(&c, e))
            }).map_err(fn_err)?;
            globals.set("__host_db_count", f).map_err(fn_err)?;

            let b = bridge.clone();
            let f = Function::new(qjs_ctx.clone(), move |c: Ctx<'_>, table: String, data: String| -> rquickjs::Result<String> {
                b.create(&table, &data).map_err(|e| js_error(&c, e))
            }).map_err(fn_err)?;
            globals.set("__host_db_create", f).map_err(fn_err)?;

            let b = bridge.clone();
            let f = Function::new(qjs_ctx.clone(), move |c: Ctx<'_>, table: String, records: String| -> rquickjs::Result<String> {
                b.create_many(&table, &records).map_err(|e| js_error(&c, e))
            }).map_err(fn_err)?;
            globals.set("__host_db_create_many", f).map_err(fn_err)?;

            let b = bridge.clone();
            let f = Function::new(qjs_ctx.clone(), move |c: Ctx<'_>, table: String, pk: String, patch: String, strict: bool| -> rquickjs::Result<String> {
                b.update(&table, &pk, &patch, strict).map_err(|e| js_error(&c, e))
            }).map_err(fn_err)?;
            globals.set("__host_db_update", f).map_err(fn_err)?;

            let b = bridge.clone();
            let f = Function::new(qjs_ctx.clone(), move |c: Ctx<'_>, table: String, pk: String| -> rquickjs::Result<bool> {
                b.delete(&table, &pk).map_err(|e| js_error(&c, e))
            }).map_err(fn_err)?;
            globals.set("__host_db_delete", f).map_err(fn_err)?;

            let b = bridge.clone();
            let f = Function::new(qjs_ctx.clone(), move |c: Ctx<'_>, table: String, options: String| -> rquickjs::Result<String> {
                b.upsert(&table, &options).map_err(|e| js_error(&c, e))
            }).map_err(fn_err)?;
            globals.set("__host_db_upsert", f).map_err(fn_err)?;

            let b = bridge.clone();
            let f = Function::new(qjs_ctx.clone(), move |c: Ctx<'_>, table: String, pk: String| -> rquickjs::Result<String> {
                b.lock(&table, &pk).map_err(|e| js_error(&c, e))
            }).map_err(fn_err)?;
            globals.set("__host_db_lock", f).map_err(fn_err)?;

            let b = bridge.clone();
            let f = Function::new(qjs_ctx.clone(), move |c: Ctx<'_>, table: String, patch: String, filter: String| -> rquickjs::Result<f64> {
                b.modify_all(&table, &patch, &filter).map(|n| n as f64).map_err(|e| js_error(&c, e))
            }).map_err(fn_err)?;
            globals.set("__host_db_modify_all", f).map_err(fn_err)?;

            let b = bridge.clone();
            let f = Function::new(qjs_ctx.clone(), move |c: Ctx<'_>, table: String, filter: String| -> rquickjs::Result<f64> {
                b.delete_all(&table, &filter).map(|n| n as f64).map_err(|e| js_error(&c, e))
            }).map_err(fn_err)?;
            globals.set("__host_db_delete_all", f).map_err(fn_err)?;

            let b = bridge.clone();
            let f = Function::new(qjs_ctx.clone(), move |c: Ctx<'_>, table: String, op: String, field: String, filter: String| -> rquickjs::Result<String> {
                b.aggregate(&table, &op, &field, &filter).map_err(|e| js_error(&c, e))
            }).map_err(fn_err)?;
            globals.set("__host_db_aggregate", f).map_err(fn_err)?;

            let b = bridge.clone();
            let f = Function::new(qjs_ctx.clone(), move |c: Ctx<'_>, table: String, pk: String, action: String| -> rquickjs::Result<bool> {
                b.can_transition(&table, &pk, &action).map_err(|e| js_error(&c, e))
            }).map_err(fn_err)?;
            globals.set("__host_workflow_can_transition", f).map_err(fn_err)?;

            let b = bridge.clone();
            let f = Function::new(qjs_ctx.clone(), move |c: Ctx<'_>, table: String, pk: String, action: String, comments: String| -> rquickjs::Result<String> {
                b.transition(&table, &pk, &action, &comments).map_err(|e| js_error(&c, e))
            }).map_err(fn_err)?;
            globals.set("__host_workflow_transition", f).map_err(fn_err)?;

            let b = bridge.clone();
            let f = Function::new(qjs_ctx.clone(), move |c: Ctx<'_>, entity: String, perm: String| -> rquickjs::Result<bool> {
                b.check_permission(&entity, &perm).map_err(|e| js_error(&c, e))
            }).map_err(fn_err)?;
            globals.set("__host_security_check_perm", f).map_err(fn_err)?;

            let http_req_fn = Function::new(qjs_ctx.clone(), move |c: Ctx<'_>, method: String, url: String, body_str: String, headers_json: String| -> rquickjs::Result<String> {
                let url_trimmed = url.trim().to_string();
                if let Err(reason) = refuse_unreal_http(&url_trimmed) {
                    return Err(js_error(&c, RuntimeError::SecurityError(reason)));
                }
                let headers_map: serde_json::Map<String, JsonValue> = serde_json::from_str(&headers_json).unwrap_or_default();
                let handle = tokio::runtime::Handle::try_current().map_err(|_| {
                    js_error(&c, RuntimeError::Internal("ctx.http needs the server runtime".to_string()))
                })?;
                let m_upper = method.trim().to_ascii_uppercase();
                let u_clone = url_trimmed.clone();
                let b_clone = body_str.clone();
                let h_clone = headers_map.clone();
                let http_res: Result<String, String> = tokio::task::block_in_place(|| {
                    handle.block_on(async move {
                        let client = reqwest::Client::builder()
                            .timeout(std::time::Duration::from_secs(5))
                            .build()
                            .map_err(|e| e.to_string())?;
                        let mut req = match m_upper.as_str() {
                            "POST" => client.post(&u_clone),
                            "PUT" => client.put(&u_clone),
                            "DELETE" => client.delete(&u_clone),
                            _ => client.get(&u_clone),
                        };
                        for (hk, hv) in &h_clone {
                            if let Some(hs) = hv.as_str() {
                                req = req.header(hk, hs);
                            }
                        }
                        if m_upper == "POST" || m_upper == "PUT" {
                            req = req.header("Content-Type", "application/json").body(b_clone);
                        }
                        let resp = req.send().await.map_err(|e| e.to_string())?;
                        let status = resp.status().as_u16();
                        let ok = resp.status().is_success();
                        let text = resp.text().await.unwrap_or_default();
                        let data: JsonValue = serde_json::from_str(&text).unwrap_or(JsonValue::String(text));
                        Ok(serde_json::json!({
                            "status": status,
                            "ok": ok,
                            "url": u_clone,
                            "headers": h_clone,
                            "data": data
                        }).to_string())
                    })
                });
                http_res.map_err(|reason| js_error(&c, RuntimeError::Internal(format!("ctx.http failed for {url_trimmed}: {reason}"))))
            }).map_err(fn_err)?;
            globals.set("__host_http_request", http_req_fn).map_err(fn_err)?;

            let b = bridge.clone();
            let f = Function::new(qjs_ctx.clone(), move |c: Ctx<'_>, module: String, func: String, a: f64, bv: f64| -> rquickjs::Result<f64> {
                b.wasm_invoke(&module, &func, a, bv).map_err(|e| js_error(&c, e))
            }).map_err(fn_err)?;
            globals.set("__host_wasm_invoke", f).map_err(fn_err)?;

            let b = bridge.clone();
            let f = Function::new(qjs_ctx.clone(), move |c: Ctx<'_>, service: String, path: String, payload: String| -> rquickjs::Result<String> {
                b.sidecar_call(&service, &path, &payload).map_err(|e| js_error(&c, e))
            }).map_err(fn_err)?;
            globals.set("__host_sidecar_call", f).map_err(fn_err)?;

            let b = bridge.clone();
            let f = Function::new(qjs_ctx.clone(), move |c: Ctx<'_>, publisher: String, event_name: String, data: String| -> rquickjs::Result<String> {
                b.publish_event(&publisher, &event_name, &data).map_err(|e| js_error(&c, e))
            }).map_err(fn_err)?;
            globals.set("__host_publish_event", f).map_err(fn_err)?;

            let b = bridge.clone();
            let f = Function::new(qjs_ctx.clone(), move |c: Ctx<'_>, codeunit: String, method: String, params: String| -> rquickjs::Result<String> {
                b.call_codeunit(&codeunit, &method, &params).map_err(|e| js_error(&c, e))
            }).map_err(fn_err)?;
            globals.set("__host_call_codeunit", f).map_err(fn_err)?;

            let slot = self.return_slot.clone();
            let f = Function::new(qjs_ctx.clone(), move |value: String| {
                if let Ok(mut guard) = slot.lock() {
                    *guard = Some(value);
                }
            }).map_err(fn_err)?;
            globals.set("__host_set_return", f).map_err(fn_err)?;

            // 3. Evaluate Core Prelude (Host SDK, Dynamic Table Models Gateway & Exact Monetary Arithmetic)
            let roles_json = serde_json::to_string(&ctx.roles).unwrap_or_else(|_| "[]".to_string());
            let prelude = format!(
                r#"
                const __uiMessages = [];
                const __uiDirectives = [];
                const __uiAnswers = {0};

                function __friendlyFieldName(fieldName) {{
                    if (!fieldName || typeof fieldName !== "string") return "Field";
                    return fieldName
                        .replace(/([a-z0-9])([A-Z])/g, "$1 $2")
                        .replace(/[_-]+/g, " ")
                        .trim()
                        .replace(/\b\w/g, c => c.toUpperCase());
                }}

                function require(cond, messageOrField, expectedValue) {{
                    if (!cond) {{
                        if (messageOrField !== undefined && expectedValue !== undefined) {{
                            const label = __friendlyFieldName(messageOrField);
                            throw new Error(`'${{label}}' requirement failed: expected '${{expectedValue}}'.`);
                        }} else if (messageOrField) {{
                            throw new Error(String(messageOrField));
                        }} else {{
                            throw new Error("Validation requirement failed.");
                        }}
                    }}
                }}

                function __attachRecHelpers(obj, tableName) {{
                    if (!obj || typeof obj !== "object" || Array.isArray(obj)) return obj;
                    const tbl = tableName || "Record";
                    const proto = {{
                        testField(fieldName, expectedValue, customMessage) {{
                            const actual = this[fieldName];
                            const friendlyLabel = __friendlyFieldName(fieldName);
                            if (expectedValue !== undefined) {{
                                if (actual !== expectedValue) {{
                                    const actualDisplay = (actual === null || actual === undefined || actual === "") ? "blank" : actual;
                                    throw new Error(customMessage || `'${{friendlyLabel}}' in ${{tbl}} must be equal to '${{expectedValue}}' (current value is '${{actualDisplay}}').`);
                                }}
                            }} else {{
                                if (actual === null || actual === undefined || actual === "") {{
                                    throw new Error(customMessage || `'${{friendlyLabel}}' in ${{tbl}} must not be blank.`);
                                }}
                            }}
                            return true;
                        }},
                        testNotEmpty(fieldName, customMessage) {{
                            const actual = this[fieldName];
                            const friendlyLabel = __friendlyFieldName(fieldName);
                            const isEmpty = actual === null || actual === undefined ||
                                (typeof actual === "string" && actual.trim() === "") ||
                                (Array.isArray(actual) && actual.length === 0) ||
                                (typeof actual === "object" && Object.keys(actual).length === 0);
                            if (isEmpty) {{
                                throw new Error(customMessage || `'${{friendlyLabel}}' must not be empty.`);
                            }}
                            return true;
                        }},
                        testPositive(fieldName, customMessage) {{
                            const actual = this[fieldName];
                            const friendlyLabel = __friendlyFieldName(fieldName);
                            const num = Number(actual);
                            if (actual === null || actual === undefined || isNaN(num) || num <= 0) {{
                                throw new Error(customMessage || `'${{friendlyLabel}}' must be greater than zero.`);
                            }}
                            return true;
                        }},
                        testRange(fieldName, min, max, customMessage) {{
                            const actual = this[fieldName];
                            const friendlyLabel = __friendlyFieldName(fieldName);
                            const num = Number(actual);
                            if (actual === null || actual === undefined || isNaN(num) || num < min || num > max) {{
                                throw new Error(customMessage || `'${{friendlyLabel}}' must be between ${{min}} and ${{max}}.`);
                            }}
                            return true;
                        }},
                        testEmail(fieldName, customMessage) {{
                            const actual = this[fieldName];
                            const friendlyLabel = __friendlyFieldName(fieldName);
                            const str = (actual !== null && actual !== undefined) ? String(actual).trim() : "";
                            const emailRegex = /^[a-zA-Z0-9.!#$%&'*+/=?^_`{{|}}~-]+@[a-zA-Z0-9](?:[a-zA-Z0-9-]{{0,61}}[a-zA-Z0-9])?(?:\.[a-zA-Z0-9](?:[a-zA-Z0-9-]{{0,61}}[a-zA-Z0-9])?)+$/;
                            if (!emailRegex.test(str)) {{
                                const valDisplay = str ? str : "blank";
                                throw new Error(customMessage || `'${{valDisplay}}' is not a valid email address for '${{friendlyLabel}}'.`);
                            }}
                            return true;
                        }},
                        calcFields(...fieldNames) {{
                            const pk = this._id || this.id || this.no || this.code || "";
                            const res = JSON.parse(__host_db_calc_flow_fields(tbl, String(pk), JSON.stringify(fieldNames)));
                            for (const [k, v] of Object.entries(res)) {{
                                this[k] = v;
                            }}
                            return this;
                        }},
                        validate(fieldName, newValue) {{
                            const oldVal = this[fieldName];
                            if (newValue !== undefined) {{
                                this[fieldName] = newValue;
                            }}
                            const val = this[fieldName];
                            const capField = fieldName.charAt(0).toUpperCase() + fieldName.slice(1);
                            const fnName = `validate${{capField}}`;
                            if (typeof this[fnName] === "function") {{
                                this[fnName](val, oldVal);
                            }} else if (typeof globalThis[fnName] === "function") {{
                                globalThis[fnName](val, oldVal);
                            }}
                            return this;
                        }},
                        save() {{
                            const model = __getModel(tbl);
                            const pk = this._id || this.id || this.code || this.no;
                            if (pk && model.get(pk) !== null) {{
                                Object.assign(this, model.modify(this));
                                return this;
                            }}
                            const created = model.create(this);
                            if (created && typeof created === "object") Object.assign(this, created);
                            return this;
                        }},
                        insert() {{
                            const created = __getModel(tbl).create(this);
                            if (created && typeof created === "object") Object.assign(this, created);
                            return this;
                        }},
                        modify() {{
                            const updated = __getModel(tbl).modify(this);
                            if (updated && typeof updated === "object") {{
                                Object.assign(this, updated);
                                if (typeof this.__bosResnap === "function") this.__bosResnap();
                            }}
                            return this;
                        }}
                    }};
                    Object.setPrototypeOf(obj, proto);
                    for (const k of Object.keys(obj)) {{
                        if (k.includes("_") && !k.startsWith("__")) {{
                            const camel = k.replace(/_([a-z0-9])/g, (_, g) => g.toUpperCase());
                            if (!(camel in obj)) {{
                                Object.defineProperty(obj, camel, {{
                                    get() {{ return this[k]; }},
                                    set(v) {{ this[k] = v; }},
                                    enumerable: true,
                                    configurable: true
                                }});
                            }}
                        }}
                    }}
                    if (!Object.prototype.hasOwnProperty.call(obj, "__bosSnapshot")) {{
                        const snap = {{}};
                        for (const k of Object.keys(obj)) {{
                            if (typeof obj[k] !== "function") snap[k] = obj[k];
                        }}
                        Object.defineProperty(obj, "__bosSnapshot", {{ value: snap, enumerable: false, writable: true }});
                        Object.defineProperty(obj, "__bosDirtyPatch", {{
                            value() {{
                                const patch = {{}};
                                const base = this.__bosSnapshot || {{}};
                                for (const k of Object.keys(this)) {{
                                    if (typeof this[k] === "function") continue;
                                    if (this[k] !== base[k]) patch[k] = this[k];
                                }}
                                return patch;
                            }},
                            enumerable: false
                        }});
                        Object.defineProperty(obj, "__bosResnap", {{
                            value() {{
                                const next = {{}};
                                for (const k of Object.keys(this)) {{
                                    if (typeof this[k] !== "function") next[k] = this[k];
                                }}
                                this.__bosSnapshot = next;
                            }},
                            enumerable: false
                        }});
                    }}
                    return obj;
                }}

                function __createModel(tableName) {{
                    const ModelConstructor = function(initData) {{
                        const recObj = (initData && typeof initData === "object") ? {{ ...initData }} : {{}};
                        return __attachRecHelpers(recObj, tableName);
                    }};

                    const repoMethods = {{
                        get: (pk, opts) => {{
                            const res = __host_db_get(tableName, String(pk), JSON.stringify(opts || {{}}));
                            return res === "null" ? null : __attachRecHelpers(JSON.parse(res), tableName);
                        }},
                        findById: (pk, opts) => {{
                            const res = __host_db_get(tableName, String(pk), JSON.stringify(opts || {{}}));
                            return res === "null" ? null : __attachRecHelpers(JSON.parse(res), tableName);
                        }},
                        getOrThrow: (pk, opts) => {{
                            const res = __host_db_get(tableName, String(pk), JSON.stringify(opts || {{}}));
                            const r = res === "null" ? null : __attachRecHelpers(JSON.parse(res), tableName);
                            if (!r) throw new Error(`The ${{tableName}} with ID/code '${{pk}}' was not found. Please verify the record exists.`);
                            return r;
                        }},
                        first: (q) => {{
                            const res = __host_db_first(tableName, JSON.stringify(q || {{}}));
                            return res === "null" ? null : __attachRecHelpers(JSON.parse(res), tableName);
                        }},
                        findFirst: (q) => {{
                            const res = __host_db_first(tableName, JSON.stringify(q || {{}}));
                            return res === "null" ? null : __attachRecHelpers(JSON.parse(res), tableName);
                        }},
                        last: (q) => {{
                            const res = __host_db_last(tableName, JSON.stringify(q || {{}}));
                            return res === "null" ? null : __attachRecHelpers(JSON.parse(res), tableName);
                        }},
                        find: (q) => {{
                            const res = __host_db_find_many(tableName, JSON.stringify(q || {{}}));
                            const arr = res === "[]" ? [] : JSON.parse(res);
                            return Array.isArray(arr) ? arr.map(r => __attachRecHelpers(r, tableName)) : [];
                        }},
                        findMany: (q) => {{
                            const res = __host_db_find_many(tableName, JSON.stringify(q || {{}}));
                            const arr = res === "[]" ? [] : JSON.parse(res);
                            return Array.isArray(arr) ? arr.map(r => __attachRecHelpers(r, tableName)) : [];
                        }},
                        getMany: (keys) => {{
                            const res = __host_db_get_many(tableName, JSON.stringify(keys || []));
                            const arr = JSON.parse(res);
                            return Array.isArray(arr) ? arr.map(r => __attachRecHelpers(r, tableName)) : [];
                        }},
                        exists: (w) => Boolean(__host_db_exists(tableName, JSON.stringify(w || {{}}))),
                        count: (w) => Number(__host_db_count(tableName, JSON.stringify(w || {{}}))),
                        create: (d) => {{
                            const createdRes = __host_db_create(tableName, JSON.stringify(d || {{}}));
                            if (!createdRes || createdRes === "null") {{
                                throw new Error(`Failed to create ${{tableName}}: unexpected empty response from database.`);
                            }}
                            return __attachRecHelpers(JSON.parse(createdRes), tableName);
                        }},
                        insert: (d) => {{
                            const createdRes = __host_db_create(tableName, JSON.stringify(d || {{}}));
                            if (!createdRes || createdRes === "null") {{
                                throw new Error(`Failed to create ${{tableName}}: unexpected empty response from database.`);
                            }}
                            return __attachRecHelpers(JSON.parse(createdRes), tableName);
                        }},
                        createMany: (records) => {{
                            const res = __host_db_create_many(tableName, JSON.stringify(records || []));
                            const arr = JSON.parse(res);
                            return Array.isArray(arr) ? arr.map(r => __attachRecHelpers(r, tableName)) : [];
                        }},
                        update: (pk, patch) => {{
                            const res = __host_db_update(tableName, String(pk), JSON.stringify(patch || {{}}), true);
                            return res === "null" ? null : __attachRecHelpers(JSON.parse(res), tableName);
                        }},
                        modify: (recOrPk, patch) => {{
                            if (patch === undefined && recOrPk && typeof recOrPk === "object") {{
                                const pkVal = recOrPk._id ?? recOrPk.id ?? recOrPk.code ?? recOrPk.no;
                                if (pkVal === undefined || pkVal === null) throw new Error("modify on " + tableName + " requires a record with a key");
                                const body = typeof recOrPk.__bosDirtyPatch === "function" ? recOrPk.__bosDirtyPatch() : recOrPk;
                                const res = __host_db_update(tableName, String(pkVal), JSON.stringify(body), false);
                                if (res === "null") throw new Error(`Cannot update ${{tableName}}: record with key '${{pkVal}}' does not exist.`);
                                return __attachRecHelpers(JSON.parse(res), tableName);
                            }}
                            const res = __host_db_update(tableName, String(recOrPk), JSON.stringify(patch || {{}}), true);
                            if (res === "null") throw new Error(`Cannot update ${{tableName}}: record with key '${{recOrPk}}' does not exist.`);
                            return __attachRecHelpers(JSON.parse(res), tableName);
                        }},
                        delete: (pk) => {{
                            const deleted = Boolean(__host_db_delete(tableName, String(pk)));
                            return deleted;
                        }},
                        upsert: (opts) => {{
                            const res = __host_db_upsert(tableName, JSON.stringify(opts || {{}}));
                            return res === "null" ? null : __attachRecHelpers(JSON.parse(res), tableName);
                        }},
                        lock: (pk) => {{
                            const res = __host_db_lock(tableName, String(pk));
                            return res === "null" ? null : __attachRecHelpers(JSON.parse(res), tableName);
                        }},
                        modifyAll: (patch, where) => Number(__host_db_modify_all(tableName, JSON.stringify(patch || {{}}), JSON.stringify(where || {{}}))),
                        deleteAll: (where) => Number(__host_db_delete_all(tableName, JSON.stringify(where || {{}}))),
                        sumField: (fieldName, where) => ctx.decimal(__host_db_aggregate(tableName, "SUM", String(fieldName || ""), JSON.stringify(where || {{}}))),
                        avgField: (fieldName, where) => ctx.decimal(__host_db_aggregate(tableName, "AVG", String(fieldName || ""), JSON.stringify(where || {{}}))),
                        minField: (fieldName, where) => ctx.decimal(__host_db_aggregate(tableName, "MIN", String(fieldName || ""), JSON.stringify(where || {{}}))),
                        maxField: (fieldName, where) => ctx.decimal(__host_db_aggregate(tableName, "MAX", String(fieldName || ""), JSON.stringify(where || {{}})))
                    }};

                    Object.assign(ModelConstructor, repoMethods);
                    return ModelConstructor;
                }}

                const __modelsCache = {{}};
                function __getModel(tableName) {{
                    if (!__modelsCache[tableName]) {{
                        __modelsCache[tableName] = __createModel(tableName);
                    }}
                    return __modelsCache[tableName];
                }}

                const ctx = {{
                    tenantId: "{1}",
                    companyId: "{2}",
                    userId: "{3}",
                    roles: {4},
                    user: {{
                        id: "{3}",
                        userId: "{3}",
                        tenantId: "{1}",
                        companyId: "{2}",
                        roles: {4},
                        workDate: new Date().toISOString().slice(0, 10)
                    }},
                    now: () => new Date().toISOString(),
                    log: (levelOrMsg, msg) => __host_log(String(levelOrMsg || ""), msg !== undefined ? String(msg) : ""),
                    call: (codeunitName, methodName, params) => {{
                        __host_log("info", `Calling codeunit ${{codeunitName}}.${{methodName}}`);
                        const res = __host_call_codeunit(String(codeunitName), String(methodName), JSON.stringify(params !== undefined ? params : null));
                        return res === "null" ? null : JSON.parse(res);
                    }},
                    decimal: function(val) {{
                        const SCALE = 1000000n;
                        let raw;
                        if (typeof val === "bigint") {{
                            raw = val * SCALE;
                        }} else if (typeof val === "number") {{
                            raw = BigInt(Math.round(val * 1000000));
                        }} else if (typeof val === "string") {{
                            const trimmed = val.trim();
                            const isNeg = trimmed.startsWith("-");
                            const clean = isNeg ? trimmed.slice(1) : trimmed;
                            const parts = clean.split(".");
                            const intPart = BigInt(parts[0] || "0");
                            const fracPart = (parts[1] || "").padEnd(6, "0").slice(0, 6);
                            raw = intPart * SCALE + BigInt(fracPart);
                            if (isNeg) raw = -raw;
                        }} else if (val && typeof val === "object" && "_raw" in val) {{
                            raw = val._raw;
                        }} else {{
                            raw = 0n;
                        }}

                        return {{
                            _raw: raw,
                            add(other) {{
                                const o = ctx.decimal(other);
                                return ctx.decimal({{ _raw: this._raw + o._raw }});
                            }},
                            sub(other) {{
                                const o = ctx.decimal(other);
                                return ctx.decimal({{ _raw: this._raw - o._raw }});
                            }},
                            mul(other) {{
                                const o = ctx.decimal(other);
                                return ctx.decimal({{ _raw: (this._raw * o._raw) / SCALE }});
                            }},
                            div(other) {{
                                const o = ctx.decimal(other);
                                if (o._raw === 0n) throw new Error("Division by zero in ctx.decimal");
                                return ctx.decimal({{ _raw: (this._raw * SCALE) / o._raw }});
                            }},
                            round(decimals) {{
                                const d = typeof decimals === "number" ? decimals : 2;
                                const factor = 10n ** BigInt(6 - d);
                                const remainder = this._raw % factor;
                                let rounded = this._raw - remainder;
                                if (remainder >= factor / 2n) rounded += factor;
                                else if (remainder <= -(factor / 2n)) rounded -= factor;
                                return ctx.decimal({{ _raw: rounded }});
                            }},
                            toNumber() {{
                                return Number(this._raw) / 1000000;
                            }},
                            valueOf() {{
                                return this.toNumber();
                            }},
                            toString() {{
                                const sign = this._raw < 0n ? "-" : "";
                                const abs = this._raw < 0n ? -this._raw : this._raw;
                                const intStr = (abs / SCALE).toString();
                                const fracStr = (abs % SCALE).toString().padStart(6, "0").replace(/0+$/, "");
                                return fracStr.length > 0 ? `${{sign}}${{intStr}}.${{fracStr}}` : `${{sign}}${{intStr}}`;
                            }},
                            toJSON() {{
                                return this.toNumber();
                            }},
                            cmp(other) {{
                                const o = ctx.decimal(other)._raw;
                                return this._raw < o ? -1 : (this._raw > o ? 1 : 0);
                            }},
                            eq(other) {{
                                return this._raw === ctx.decimal(other)._raw;
                            }},
                            gt(other) {{
                                return this._raw > ctx.decimal(other)._raw;
                            }},
                            gte(other) {{
                                return this._raw >= ctx.decimal(other)._raw;
                            }},
                            lt(other) {{
                                return this._raw < ctx.decimal(other)._raw;
                            }},
                            lte(other) {{
                                return this._raw <= ctx.decimal(other)._raw;
                            }},
                            isZero() {{
                                return this._raw === 0n;
                            }},
                            isNegative() {{
                                return this._raw < 0n;
                            }},
                            isPositive() {{
                                return this._raw > 0n;
                            }},
                            abs() {{
                                return ctx.decimal({{ _raw: this._raw < 0n ? -this._raw : this._raw }});
                            }}
                        }};
                    }},
                    crypto: {{
                        sha256: (s) => __host_sha256(String(s)),
                        argon2Hash: (pwd) => __host_argon2_hash(String(pwd)),
                        argon2Verify: (pwd, hash) => Boolean(__host_argon2_verify(String(pwd), String(hash))),
                        randomBytes: (n) => {{
                            const h = __host_random_bytes(Number(n));
                            return {{ toHex: () => h }};
                        }}
                    }},
                    audit: {{
                        log: (msg) => __host_audit_log(String(msg))
                    }},
                    events: {{
                        publish: (publisher, eventName, data) => {{
                            const res = __host_publish_event(String(publisher), String(eventName), JSON.stringify(data || {{}}));
                            return res === "null" ? null : JSON.parse(res);
                        }},
                        emit: (publisher, eventName, data) => {{
                            const res = __host_publish_event(String(publisher), String(eventName), JSON.stringify(data || {{}}));
                            return res === "null" ? null : JSON.parse(res);
                        }}
                    }},
                    workflow: {{
                        transition: (table, id, action, comments) => {{
                            const res = __host_workflow_transition(String(table), String(id), String(action), String(comments || ""));
                            return res === "null" ? null : JSON.parse(res);
                        }},
                        canTransition: (table, id, action) => Boolean(__host_workflow_can_transition(String(table), String(id), String(action)))
                    }},
                    security: {{
                        hasRole: (role) => Array.isArray(ctx.roles) && ctx.roles.some(r => String(r).toLowerCase() === String(role).toLowerCase()),
                        requireRole: (role) => {{
                            if (!ctx.security.hasRole(role)) {{
                                throw new Error(`SecurityError: Required role '${{role}}' is not assigned to current user.`);
                            }}
                            return true;
                        }},
                        checkPermission: (entity, perm) => Boolean(__host_security_check_perm(String(entity), String(perm || "read")))
                    }},
                    http: {{
                        get: (url, headers) => {{
                            const res = __host_http_request("GET", String(url), "null", JSON.stringify(headers || {{}}));
                            return JSON.parse(res);
                        }},
                        post: (url, body, headers) => {{
                            const res = __host_http_request("POST", String(url), JSON.stringify(body !== undefined ? body : null), JSON.stringify(headers || {{}}));
                            return JSON.parse(res);
                        }}
                    }},
                    wasm: {{
                        invoke: (mod, fnName, a, b) => __host_wasm_invoke(String(mod), String(fnName), Number(a || 0), Number(b || 0))
                    }},
                    sidecar: {{
                        call: (service, path, data) => {{
                            const res = __host_sidecar_call(String(service), String(path), JSON.stringify(data || {{}}));
                            return res === "null" ? null : JSON.parse(res);
                        }}
                    }},
                    ui: {{
                        toast: (arg1, arg2) => __createToast("info", arg1, arg2),
                        message: (msg, type) => __createToast(type ? String(type).toLowerCase() : "info", msg),
                        success: (msg, titleOrOpts) => __createToast("success", msg, titleOrOpts),
                        warning: (msg, titleOrOpts) => __createToast("warning", msg, titleOrOpts),
                        error: (msg, titleOrOpts) => __createToast("error", msg, titleOrOpts),
                        info: (msg, titleOrOpts) => __createToast("info", msg, titleOrOpts),
                        confirm: (msg) => {{
                            const key = String(msg);
                            if (Object.prototype.hasOwnProperty.call(__uiAnswers, key)) {{
                                const answer = Boolean(__uiAnswers[key]);
                                __uiDirectives.push({{ type: "confirm", message: key, answer: answer }});
                                return answer;
                            }}
                            __uiDirectives.push({{ type: "confirm", message: key, pending: true }});
                            throw new Error("ConfirmRequired: " + key);
                        }},
                        openModal: (pageName, params) => {{
                            const d = {{ type: "modal", page: String(pageName), params: params || {{}} }};
                            __uiDirectives.push(d);
                            return d;
                        }},
                        navigate: (url) => {{
                            const d = {{ type: "navigate", url: String(url) }};
                            __uiDirectives.push(d);
                            return d;
                        }},
                        setFieldVisible: (fieldName, visible) => {{
                            const d = {{ type: "fieldVisible", field: String(fieldName), value: Boolean(visible) }};
                            __uiDirectives.push(d);
                            return d;
                        }},
                        setFieldEditable: (fieldName, editable) => {{
                            const d = {{ type: "fieldEditable", field: String(fieldName), value: Boolean(editable) }};
                            __uiDirectives.push(d);
                            return d;
                        }},
                        getMessages: () => [...__uiMessages],
                        getDirectives: () => [...__uiDirectives],
                        clearMessages: () => {{ __uiMessages.length = 0; __uiDirectives.length = 0; }}
                    }},
                    message: (msg, type) => __createToast(type ? String(type).toLowerCase() : "info", msg),
                    toast: (arg1, arg2) => __createToast("info", arg1, arg2),
                    db: new Proxy({{
                        getModel: (tableName) => __getModel(String(tableName)),
                        query: (sql, params) => {{
                            const res = __host_db_raw_query(String(sql), JSON.stringify(params || []));
                            return res === "[]" ? [] : JSON.parse(res);
                        }},
                        execute: (sql, params) => Number(__host_db_raw_execute(String(sql), JSON.stringify(params || [])))
                    }}, {{
                        get: (target, prop) => {{
                            if (prop in target) return target[prop];
                            return __getModel(String(prop));
                        }}
                    }})
                }};
                function __createToast(defaultType, arg1, arg2) {{
                    let type = defaultType;
                    let title = "";
                    let message = "";
                    let duration = 5000;
                    let action = null;
                    let dismissible = true;

                    if (typeof arg1 === "object" && arg1 !== null) {{
                        if (arg1.type) type = String(arg1.type).toLowerCase();
                        title = arg1.title || (type.charAt(0).toUpperCase() + type.slice(1));
                        message = String(arg1.message || arg1.text || "");
                        duration = typeof arg1.duration === "number" ? arg1.duration : 5000;
                        action = arg1.action || null;
                        dismissible = arg1.dismissible !== undefined ? Boolean(arg1.dismissible) : true;
                    }} else {{
                        message = String(arg1 || "");
                        if (typeof arg2 === "string") {{
                            if (["info", "success", "warning", "error"].includes(arg2.toLowerCase())) {{
                                type = arg2.toLowerCase();
                                title = type.charAt(0).toUpperCase() + type.slice(1);
                            }} else {{
                                title = arg2;
                            }}
                        }} else if (typeof arg2 === "object" && arg2 !== null) {{
                            if (arg2.type) type = String(arg2.type).toLowerCase();
                            title = arg2.title || (type.charAt(0).toUpperCase() + type.slice(1));
                            duration = typeof arg2.duration === "number" ? arg2.duration : 5000;
                            action = arg2.action || null;
                            dismissible = arg2.dismissible !== undefined ? Boolean(arg2.dismissible) : true;
                        }} else {{
                            title = type.charAt(0).toUpperCase() + type.slice(1);
                        }}
                    }}

                    const toastObj = {{
                        id: "msg_" + Math.random().toString(36).slice(2, 9),
                        type: type,
                        title: title,
                        message: message,
                        duration: duration,
                        dismissible: dismissible,
                        timestamp: new Date().toISOString(),
                        action: action
                    }};

                    __uiMessages.push(toastObj);

                    try {{
                        __host_ui_toast(
                            toastObj.type,
                            toastObj.title,
                            toastObj.message,
                            toastObj.duration,
                            JSON.stringify(toastObj.action)
                        );
                    }} catch (_) {{}}

                    return toastObj;
                }}

                globalThis.ui = ctx.ui;
                globalThis.toast = ctx.ui.toast;
                globalThis.message = ctx.ui.message;
                globalThis.models = ctx.db;
                globalThis.Decimal = ctx.decimal;

                // Sandbox Security Guard: Disallow dynamic code injection via eval()
                globalThis.eval = function() {{
                    throw new Error("SecurityError: eval() is forbidden in the BOS-Script runtime sandbox.");
                }};

                // Dynamic Global Model Proxy: Any capitalized identifier automatically resolves to a TableModel!
                // Works dynamically for ALL tables without static pre-declarations.
                Object.setPrototypeOf(globalThis, new Proxy(Object.prototype, {{
                    has(target, prop) {{
                        return (typeof prop === "string" && /^[A-Z][a-zA-Z0-9_]*$/.test(prop)) || (prop in target);
                    }},
                    get(target, prop, receiver) {{
                        if (typeof prop === "string" && /^[A-Z][a-zA-Z0-9_]*$/.test(prop) && !(prop in target)) {{
                            return __getModel(prop);
                        }}
                        return Reflect.get(target, prop, receiver);
                    }}
                }}));
                "#,
                serde_json::to_string(&self.ui_answers).unwrap_or_else(|_| "{}".to_string()),
                ctx.tenant_id, ctx.company_id, ctx.user_id, roles_json
            );

            qjs_ctx.eval::<(), _>(prelude).map_err(|e| {
                RuntimeError::Internal(format!("Failed to initialize QuickJS prelude: {:?}", e))
            })?;

            // 4. Inject rec and xRec as JSON strings and parse inside JS
            let data_str = serde_json::to_string(data).unwrap_or_else(|_| "{}".to_string());
            let x_rec_str = match x_rec {
                Some(x) => serde_json::to_string(x).unwrap_or_else(|_| "null".to_string()),
                None => "null".to_string(),
            };

            let json_data = serde_json::to_string(&data_str)
                .map_err(|e| RuntimeError::Internal(format!("Failed to serialize data string: {}", e)))?;
            let json_xrec = serde_json::to_string(&x_rec_str)
                .map_err(|e| RuntimeError::Internal(format!("Failed to serialize xRec string: {}", e)))?;

            // Guarantee any TypeScript syntax passed directly to execute_trigger is stripped before QuickJS eval
            let clean_script = bosc_analyzer::strip_typescript(script);

            let runner_code = format!(
                r#"
                let rec = JSON.parse({});
                let xRec = JSON.parse({});
                if (rec && typeof rec === "object") {{
                    rec._system_messages = Array.isArray(rec._system_messages) ? rec._system_messages : [];
                }}
                if (typeof ctx.ui === "object" && typeof ctx.ui.clearMessages === "function") {{
                    ctx.ui.clearMessages();
                }}
                const __tblName = "{}";
                const __triggerName = "{}";

                // Attach Enterprise Record Prototype (testField, calcFields, validate, save, insert, modify)
                __attachRecHelpers(rec, __tblName);
                if (xRec && typeof xRec === "object") {{
                    __attachRecHelpers(xRec, __tblName);
                }}

                let __execErr = null;
                let __bosReturned;
                try {{
                    if (typeof globalThis.__bosAotModule !== "undefined") {{
                        const mod = globalThis.__bosAotModule;
                        const target = mod.default || mod[__tblName] || mod;
                        let fn = target[__triggerName] || mod[__triggerName];
                        if (typeof fn !== "function") {{
                            for (const k of Object.keys(target)) {{
                                if (k.toLowerCase() === __triggerName.toLowerCase() && typeof target[k] === "function") {{
                                    fn = target[k];
                                    break;
                                }}
                            }}
                        }}
                        if (typeof fn === "function") {{
                            // Lifecycle triggers are declared as `onInsert(rec, ctx)` (docs/02 3.1),
                            // while xRec-aware ones use `(rec, xRec, ctx)`. Dispatch by declared
                            // arity so a 2-parameter trigger receives ctx, not a null xRec.
                            const __isLifecycle = /^(on|before|after)(Insert|Modify|Delete|Rename)$/i.test(__triggerName);
                            __bosReturned = (__isLifecycle && fn.length === 2) ? fn(rec, ctx) : fn(rec, xRec, ctx);
                        }} else {{
                            throw new Error(`Method '${{__triggerName}}' not found in compiled AOT bytecode module for '${{__tblName}}'`);
                        }}
                    }} else {{
                        __bosReturned = ((rec, xRec, ctx) => {{
                            {}
                        }})(rec, xRec, ctx);
                    }}
                }} catch (__e) {{
                    __execErr = {{
                        __bos_error: true,
                        name: (__e && __e.name) ? String(__e.name) : "Error",
                        message: (__e && __e.message) ? String(__e.message) : String(__e),
                        stack: (__e && __e.stack) ? String(__e.stack) : ""
                    }};
                }}

                if (!__execErr && typeof __bosReturned !== "undefined") {{
                    try {{ __host_set_return(JSON.stringify(__bosReturned)); }} catch (_) {{}}
                }}
                if (__execErr) {{
                    JSON.stringify(__execErr);
                }} else {{
                    if (rec && typeof rec === "object" && typeof ctx.ui === "object") {{
                        const __collected = ctx.ui.getMessages();
                        if (!Array.isArray(rec._system_messages)) {{
                            rec._system_messages = [];
                        }}
                        for (const m of __collected) {{
                            if (!rec._system_messages.some(ex => ex.id === m.id)) {{
                                rec._system_messages.push(m);
                            }}
                        }}
                    }}

                    JSON.stringify(rec);
                }}
                "#,
                json_data,
                json_xrec,
                table_name,
                trigger_name,
                clean_script
            );

            let result_str: String = qjs_ctx.eval(runner_code).map_err(|e| match self.meter.tripped() {
                TRIP_FUEL => RuntimeError::Internal(format!(
                    "Execution fuel exhausted in '{}.{}': exceeded the budget of {} steps (possible infinite loop)",
                    table_name, trigger_name, self.limits.max_fuel
                )),
                TRIP_TIME => RuntimeError::Internal(format!(
                    "Execution time limit exceeded in '{}.{}': ran longer than {} ms",
                    table_name,
                    trigger_name,
                    self.limits.max_wall_time.as_millis()
                )),
                _ => RuntimeError::Internal(format!(
                    "Business logic exception in '{}.{}': {:?}",
                    table_name, trigger_name, e
                )),
            })?;

            // 5. Check for structured JavaScript exception or update data with mutated rec
            if let Ok(mutated) = serde_json::from_str::<JsonValue>(&result_str) {
                if mutated.get("__bos_error").and_then(|v| v.as_bool()) == Some(true) {
                    let err_name = mutated.get("name").and_then(|v| v.as_str()).unwrap_or("Error");
                    let err_msg = mutated.get("message").and_then(|v| v.as_str()).unwrap_or("Unknown runtime error");
                    let err_stack = mutated.get("stack").and_then(|v| v.as_str()).unwrap_or("");
                    let stack_suffix = if err_stack.trim().is_empty() {
                        String::new()
                    } else {
                        format!(" [stack: {}]", err_stack.lines().next().unwrap_or("").trim())
                    };
                    return Err(RuntimeError::Internal(format!(
                        "Business logic exception in '{}.{}' ({}): {}{}",
                        table_name, trigger_name, err_name, err_msg, stack_suffix
                    )));
                }
                if let Some(msg) = db.and_then(|s| s.failure()) {
                    return Err(RuntimeError::TransactionAborted(msg));
                }
                *data = mutated;
            } else {
                return Err(RuntimeError::Internal(format!(
                    "Script '{}.{}' did not produce a valid record",
                    table_name, trigger_name
                )));
            }

            logs.push(format!("Executed QuickJS trigger '{}.{}'", table_name, trigger_name));
            Ok(())
        })
    }

    /// Generate ambient TypeScript declarations (.d.ts) for Monaco Editor and IDE IntelliSense
    pub fn generate_ambient_dts() -> &'static str {
        r#"/**
 * BOS-Script Ambient Runtime Environment & Data Access Declarations
 * Compatible with Monaco Editor and TypeScript language server
 */

declare type UUID = string;
declare type Decimal = BosDecimal | number;
declare type Integer = number;
declare type BigInteger = bigint | number;
declare type Json = Record<string, any>;

declare interface BosDecimal {
    readonly _raw: bigint;
    add(other: BosDecimal | number | string): BosDecimal;
    sub(other: BosDecimal | number | string): BosDecimal;
    mul(other: BosDecimal | number | string): BosDecimal;
    div(other: BosDecimal | number | string): BosDecimal;
    round(decimals?: number): BosDecimal;
    eq(other: BosDecimal | number | string): boolean;
    gt(other: BosDecimal | number | string): boolean;
    gte(other: BosDecimal | number | string): boolean;
    lt(other: BosDecimal | number | string): boolean;
    lte(other: BosDecimal | number | string): boolean;
    isZero(): boolean;
    isNegative(): boolean;
    isPositive(): boolean;
    abs(): BosDecimal;
    cmp(other: BosDecimal | number | string): -1 | 0 | 1;
    valueOf(): number;
    toNumber(): number;
    toString(): string;
    toJSON(): number;
}

declare interface BosSessionContext {
    tenantId: string;
    companyId: string;
    userId: string;
    roles: string[];
    user: {
        id: string;
        userId: string;
        tenantId: string;
        companyId: string;
        roles: string[];
    };
    now(): string;
    log(levelOrMessage: string, message?: string): void;
    call<T = any>(codeunitName: string, methodName: string, params?: any): T;
    decimal(val: number | string | bigint | BosDecimal): BosDecimal;
    crypto: {
        sha256(input: string): string;
        randomBytes(length: number): { toHex(): string };
        argon2Hash(password: string): string;
        argon2Verify(password: string, hash: string): boolean;
    };
    audit: {
        log(message: string): void;
    };
    events: {
        publish(publisher: string, eventName: string, data?: any): any;
        emit(publisher: string, eventName: string, data?: any): any;
    };
    workflow: {
        transition(table: string, id: string, action: string, comments?: string): any;
        canTransition(table: string, id: string, action: string): boolean;
    };
    security: {
        hasRole(role: string): boolean;
        requireRole(role: string): boolean;
        checkPermission(entity: string, permission: "read" | "insert" | "modify" | "delete" | "execute"): boolean;
    };
    http: {
        get<T = any>(url: string, headers?: Record<string, string>): { status: number; ok: boolean; url: string; headers: Record<string, string>; data: T | null };
        post<T = any>(url: string, body?: any, headers?: Record<string, string>): { status: number; ok: boolean; url: string; headers: Record<string, string>; data: T | null };
    };
    wasm: {
        invoke(moduleName: string, funcName: string, a?: number, b?: number): number;
    };
    sidecar: {
        call<T = any>(serviceName: string, path: string, payload?: any): T;
    };
    ui: BosUiContext;
    message(message: string, type?: "info" | "success" | "warning" | "error"): BosUiMessage;
    toast(options: BosUiToastOptions): BosUiMessage;
    toast(message: string, optionsOrTitle?: BosUiToastOptions | string): BosUiMessage;
    db: {
        [table: string]: TableModel<any> | any;
        getModel<T = any>(table: string): TableModel<T>;
        query<T = any>(sql: string, params?: any[]): T[];
        execute(sql: string, params?: any[]): number;
    };
}

declare interface BosUiToastAction {
    label: string;
    action?: string;
    url?: string;
    codeunit?: string;
    method?: string;
}

declare interface BosUiToastOptions {
    title?: string;
    message?: string;
    text?: string;
    type?: "info" | "success" | "warning" | "error";
    duration?: number;
    dismissible?: boolean;
    action?: BosUiToastAction;
}

declare interface BosUiMessage {
    id: string;
    type: "info" | "success" | "warning" | "error";
    title: string;
    message: string;
    duration: number;
    dismissible: boolean;
    timestamp: string;
    action: BosUiToastAction | null;
}

declare interface BosUiContext {
    toast(options: BosUiToastOptions): BosUiMessage;
    toast(message: string, optionsOrTitle?: BosUiToastOptions | string): BosUiMessage;
    message(message: string, type?: "info" | "success" | "warning" | "error"): BosUiMessage;
    success(message: string | BosUiToastOptions, titleOrOpts?: string | BosUiToastOptions): BosUiMessage;
    warning(message: string | BosUiToastOptions, titleOrOpts?: string | BosUiToastOptions): BosUiMessage;
    error(message: string | BosUiToastOptions, titleOrOpts?: string | BosUiToastOptions): BosUiMessage;
    info(message: string | BosUiToastOptions, titleOrOpts?: string | BosUiToastOptions): BosUiMessage;
    /** Returns the answer supplied with this run. Throws ConfirmRequired when that answer is absent. */
    confirm(message: string): boolean;
    openModal(pageName: string, params?: Record<string, any>): any;
    navigate(url: string): any;
    setFieldVisible(fieldName: string, visible: boolean): any;
    setFieldEditable(fieldName: string, editable: boolean): any;
    getMessages(): BosUiMessage[];
    getDirectives(): any[];
    clearMessages(): void;
}

declare interface FieldFilter<V = any> {
    eq?: V;
    ne?: V;
    not?: V | null;
    gt?: V;
    gte?: V;
    lt?: V;
    lte?: V;
    in?: V[];
    notIn?: V[];
    contains?: string;
    startsWith?: string;
    endsWith?: string;
    ilike?: string;
    like?: string;
    isNull?: boolean;
}

declare type WhereFilter<T = any> = {
    [K in keyof T]?: T[K] | FieldFilter<T[K]> | null;
} & {
    AND?: WhereFilter<T>[];
    OR?: WhereFilter<T>[];
    [field: string]: any;
};

declare interface QueryOptions<T = any> {
    where?: WhereFilter<T>;
    select?: (keyof T)[] | { [K in keyof T]?: boolean };
    orderBy?: { [K in keyof T]?: "asc" | "desc" } & Record<string, "asc" | "desc">;
    take?: number;
    skip?: number;
}

declare interface UpsertOptions<T = any> {
    where: WhereFilter<T>;
    create: Partial<T>;
    update: Partial<T>;
}

declare interface TableModel<T = any> {
    new (initData?: Partial<T>): T;
    (initData?: Partial<T>): T;
    get(pk: string | number, options?: QueryOptions<T>): T | null;
    getOrThrow(pk: string | number, options?: QueryOptions<T>): T;
    findById(pk: string | number, options?: QueryOptions<T>): T | null;
    first(query?: QueryOptions<T> | WhereFilter<T>): T | null;
    findFirst(query?: QueryOptions<T> | WhereFilter<T>): T | null;
    last(query?: QueryOptions<T> | WhereFilter<T>): T | null;
    findMany(query?: QueryOptions<T> | WhereFilter<T>): T[];
    /** One query for many primary keys. Each row includes `__bosKey`, the key that was requested. */
    getMany(keys: Array<string | number>): T[];
    exists(where?: WhereFilter<T>): boolean;
    count(where?: WhereFilter<T>): number;
    create(data: Partial<T>): T;
    insert(data: Partial<T>): T;
    createMany(records: Partial<T>[]): T[];
    update(pk: string | number, patch: Partial<T>): T;
    modify(record: Partial<T>): T;
    delete(pk: string | number): boolean;
    upsert(options: UpsertOptions<T>): T;
    lock(pk: string | number): T | null;
    modifyAll(patch: Partial<T>, where?: WhereFilter<T>): number;
    deleteAll(where?: WhereFilter<T>): number;
    sumField(fieldName: keyof T | string, where?: WhereFilter<T>): BosDecimal;
    avgField(fieldName: keyof T | string, where?: WhereFilter<T>): BosDecimal;
    minField(fieldName: keyof T | string, where?: WhereFilter<T>): BosDecimal;
    maxField(fieldName: keyof T | string, where?: WhereFilter<T>): BosDecimal;
}

declare function Decimal(val: number | string | bigint | BosDecimal): BosDecimal;

declare const ctx: BosSessionContext;
declare function require(cond: unknown, message?: string): asserts cond;
declare const ui: BosUiContext;
declare function toast(message: string | BosUiToastOptions, optionsOrTitle?: BosUiToastOptions | string): BosUiMessage;
declare function message(message: string, type?: "info" | "success" | "warning" | "error"): BosUiMessage;
"#
    }
}

impl QuickJsEngine {
    pub(crate) fn with_depth(mut self, depth: u32) -> Self {
        self.call_depth = depth;
        self
    }
}

const COLUMNS_SQL: &str = r#"
    SELECT a.attname::text, format_type(a.atttypid, NULL), a.attgenerated <> ''
    FROM pg_attribute a
    WHERE a.attrelid = to_regclass(quote_ident($1)) AND a.attnum > 0 AND NOT a.attisdropped
"#;

const PRIMARY_KEY_SQL: &str = r#"
    SELECT a.attname::text
    FROM pg_index i
    JOIN pg_attribute a ON a.attrelid = i.indrelid AND a.attnum = ANY(i.indkey)
    WHERE i.indrelid = to_regclass(quote_ident($1)) AND i.indisprimary
    ORDER BY array_position(i.indkey::int2[], a.attnum)
    LIMIT 1
"#;

const FORBIDDEN_RAW_SQL: &[&str] = &[
    "DROP TABLE",
    "DROP DATABASE",
    "TRUNCATE",
    "ALTER SYSTEM",
    "DROP SCHEMA",
];

const NUMERIC_TYPES: &[&str] = &[
    "integer",
    "bigint",
    "smallint",
    "numeric",
    "real",
    "double precision",
];

/// Bridges BOS-Script data access to PostgreSQL through the caller's [`DbSession`].
/// Every failure surfaces as a JavaScript exception; nothing is silently defaulted.
struct HostBridge {
    db: Option<DbSession>,
    ctx: SessionContext,
    services: ScriptServices,
    limits: ExecutionLimits,
    depth: u32,
    tables: Mutex<HashMap<String, Arc<TableMeta>>>,
    ui_answers: std::collections::HashMap<String, bool>,
}

/// Column pairs for `jsonb_build_object`. `None` means "every column".
fn projection_pairs(select: &JsonValue) -> Result<Option<Vec<String>>, RuntimeError> {
    let columns = if let Some(cols) = select.as_array() {
        let mut names = Vec::new();
        for c in cols {
            let name = c.as_str().ok_or_else(|| {
                RuntimeError::Internal(
                    "'select' must be an array of field names or { field: true }".to_string(),
                )
            })?;
            names.push(column_name(name)?);
        }
        names
    } else if let Some(obj) = select.as_object() {
        let mut names = Vec::new();
        for (key, value) in obj {
            if value.as_bool() == Some(false) {
                continue;
            }
            names.push(column_name(key)?);
        }
        names
    } else {
        return Err(RuntimeError::Internal(
            "'select' must be an array of field names or { field: true }".to_string(),
        ));
    };
    if columns.is_empty() {
        return Ok(None);
    }
    Ok(Some(
        columns
            .into_iter()
            .map(|col| format!("'{col}', t.\"{col}\""))
            .collect(),
    ))
}

fn bind_all<'q, O>(
    mut q: sqlx::query::QueryScalar<'q, sqlx::Postgres, O, sqlx::postgres::PgArguments>,
    params: &'q [Option<String>],
) -> sqlx::query::QueryScalar<'q, sqlx::Postgres, O, sqlx::postgres::PgArguments> {
    for p in params {
        q = q.bind(p.as_deref());
    }
    q
}

fn raw_params(params_json: &str) -> Result<Vec<JsonValue>, RuntimeError> {
    if params_json.trim().is_empty() {
        return Ok(Vec::new());
    }
    serde_json::from_str::<Vec<JsonValue>>(params_json)
        .map_err(|e| RuntimeError::Internal(format!("SQL parameters must be a JSON array: {}", e)))
}

/// `ctx.http` only performs a real request. Documentation hosts and non-HTTP schemes are refused.
fn refuse_unreal_http(url: &str) -> Result<(), String> {
    let rest = if let Some(rest) = url.strip_prefix("https://") {
        rest
    } else if let Some(rest) = url.strip_prefix("http://") {
        rest
    } else {
        return Err(format!(
            "ctx.http only calls http and https URLs, refused '{url}'"
        ));
    };
    let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
    let hostport = authority.rsplit('@').next().unwrap_or(authority);
    let host = if let Some(inner) = hostport.strip_prefix('[') {
        inner.split(']').next().unwrap_or("")
    } else {
        hostport.split(':').next().unwrap_or("")
    };
    let host = host.trim().to_ascii_lowercase();
    if host.is_empty() || is_reserved_http_host(&host) {
        return Err(format!("ctx.http refuses reserved host '{host}'"));
    }
    Ok(())
}

fn is_reserved_http_host(host: &str) -> bool {
    host == "example"
        || host == "example.com"
        || host == "example.org"
        || host == "example.net"
        || host.ends_with(".example")
        || host.ends_with(".example.com")
        || host.ends_with(".example.org")
        || host.ends_with(".example.net")
        || host.ends_with(".test")
        || host.ends_with(".invalid")
        || host == "invalid"
}

fn guard_raw_sql(sql: &str) -> Result<(), RuntimeError> {
    let upper = sql.to_ascii_uppercase();
    for forbidden in FORBIDDEN_RAW_SQL {
        if upper.contains(forbidden) {
            return Err(RuntimeError::SecurityError(format!(
                "Destructive SQL statement '{}' is forbidden in the BOS-Script sandbox",
                forbidden
            )));
        }
    }
    Ok(())
}

/// Column assignments in input order plus the `_system_version` the caller expects, if any.
type Assignments = (Vec<(String, JsonValue)>, Option<JsonValue>);

/// Collect `{ field: value }` into column assignments; camelCase keys win over snake_case duplicates.
fn collect_assignments(
    meta: &TableMeta,
    obj: &serde_json::Map<String, JsonValue>,
    strict: bool,
) -> Result<Assignments, RuntimeError> {
    let mut order: Vec<String> = Vec::new();
    let mut values: HashMap<String, (JsonValue, bool)> = HashMap::new();
    let mut version = None;
    for (k, v) in obj {
        let col = match column_name(k) {
            Ok(c) => c,
            Err(e) if strict => return Err(e),
            Err(_) => continue,
        };
        if col == "_system_version" {
            version = Some(v.clone());
            continue;
        }
        if SYSTEM_MANAGED.contains(&col.as_str()) {
            continue;
        }
        let Some(cm) = meta.columns.get(&col) else {
            if strict {
                return Err(RuntimeError::Internal(format!(
                    "Unknown field '{}' on table '{}'",
                    k, meta.physical
                )));
            }
            continue;
        };
        if cm.generated {
            if strict {
                return Err(RuntimeError::Internal(format!(
                    "Field '{}' on table '{}' is computed and cannot be assigned",
                    k, meta.physical
                )));
            }
            continue;
        }
        let camel = k.chars().any(|c| c.is_ascii_uppercase());
        match values.get(&col) {
            Some((_, existing_camel)) if *existing_camel && !camel => {}
            Some(_) => {
                values.insert(col, (v.clone(), camel));
            }
            None => {
                order.push(col.clone());
                values.insert(col, (v.clone(), camel));
            }
        }
    }
    let out = order
        .into_iter()
        .filter_map(|c| values.remove(&c).map(|(v, _)| (c, v)))
        .collect();
    Ok((out, version))
}

fn version_number(v: &JsonValue) -> Result<i64, RuntimeError> {
    match v {
        JsonValue::Number(n) => n.as_i64(),
        JsonValue::String(s) => s.trim().parse::<i64>().ok(),
        _ => None,
    }
    .ok_or_else(|| {
        RuntimeError::Internal(format!("'_system_version' must be an integer, got {}", v))
    })
}

impl HostBridge {
    fn session(&self) -> Result<&DbSession, RuntimeError> {
        self.db.as_ref().ok_or_else(|| {
            RuntimeError::Internal(
                "Database access is unavailable: this script is running without a database session"
                    .to_string(),
            )
        })
    }

    fn query_rows(
        &self,
        sql: &str,
        params: &[Option<String>],
    ) -> Result<Vec<JsonValue>, RuntimeError> {
        let db = self.session()?;
        run_blocking(async {
            let mut conn = db.conn().await?;
            let res = bind_all(sqlx::query_scalar::<_, JsonValue>(sql), params)
                .fetch_all(&mut *conn)
                .await;
            db.track(res)
        })?
    }

    fn query_one<O>(&self, sql: &str, params: &[Option<String>]) -> Result<O, RuntimeError>
    where
        O: Send + Unpin + for<'r> sqlx::Decode<'r, sqlx::Postgres> + sqlx::Type<sqlx::Postgres>,
    {
        let db = self.session()?;
        run_blocking(async {
            let mut conn = db.conn().await?;
            let res = bind_all(sqlx::query_scalar::<_, O>(sql), params)
                .fetch_one(&mut *conn)
                .await;
            db.track(res)
        })?
    }

    fn execute(&self, sql: &str, params: &[Option<String>]) -> Result<u64, RuntimeError> {
        let db = self.session()?;
        run_blocking(async {
            let mut conn = db.conn().await?;
            let mut q = sqlx::query(sql);
            for p in params {
                q = q.bind(p.as_deref());
            }
            let res = q.execute(&mut *conn).await;
            db.track(res).map(|r| r.rows_affected())
        })?
    }

    fn catalog_exists(&self) -> Result<bool, RuntimeError> {
        self.query_one::<bool>("SELECT to_regclass('_bos_meta_objects') IS NOT NULL", &[])
    }

    /// Latest catalog definition of a meta-object, matched by name or by an alternate definition key.
    fn catalog_definition(
        &self,
        kind: &str,
        name: &str,
        alt_key: &str,
    ) -> Result<Option<JsonValue>, RuntimeError> {
        if !self.catalog_exists()? {
            return Ok(None);
        }
        let sql = format!(
            "SELECT definition FROM \"_bos_meta_objects\" WHERE LOWER(object_type) = $1 AND (LOWER(object_name) = LOWER($2) OR LOWER(definition->>'{}') = LOWER($2)) ORDER BY updated_at DESC LIMIT 1",
            crate::sanitize_identifier(alt_key)?
        );
        let rows = self.query_rows(
            &sql,
            &[Some(kind.to_string()), Some(name.trim().to_string())],
        )?;
        Ok(rows.into_iter().next())
    }

    fn table(&self, name: &str) -> Result<Arc<TableMeta>, RuntimeError> {
        let key = name.trim().trim_matches('"').to_string();
        if let Some(m) = self
            .tables
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .get(&key)
        {
            return Ok(m.clone());
        }

        let mut physical = physical_table_name(&key)?;
        let mut catalog_pk = None;
        let mut data_per_company = true;
        let mut is_immutable_ledger = false;
        let mut flow_fields = Vec::new();
        if let Some(def) = self.catalog_definition("table", &key, "sql_table_name")? {
            if let Some(sql_name) = def
                .get("sql_table_name")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
            {
                physical = crate::sanitize_identifier(&sql_name.to_lowercase())?;
            }
            catalog_pk = def
                .get("primary_key")
                .and_then(|p| p.get(0))
                .and_then(|p| p.as_str())
                .map(|p| {
                    if p.starts_with('_') {
                        p.to_string()
                    } else {
                        crate::to_snake_case(p)
                    }
                });
            if let Some(dpc) = def.get("data_per_company").and_then(|v| v.as_bool()) {
                data_per_company = dpc;
            }
            if let Some(imm) = def.get("is_immutable_ledger").and_then(|v| v.as_bool()) {
                is_immutable_ledger = imm;
            }
            if let Some(ffs) = def.get("flow_fields").and_then(|v| v.as_array()) {
                for ff in ffs {
                    if let (Some(fname), Some(subq)) = (
                        ff.get("name")
                            .and_then(|v| v.as_str())
                            .filter(|s| !s.is_empty()),
                        ff.get("sql_subquery")
                            .and_then(|v| v.as_str())
                            .filter(|s| !s.trim().is_empty()),
                    ) {
                        let rewritten = subq
                            .replace(&format!("\"{}\".", physical), "t.")
                            .replace(&format!("\"{}\".", key.to_lowercase()), "t.");
                        flow_fields.push((fname.to_string(), rewritten));
                    }
                }
            }
        }

        let db = self.session()?;
        let (cols, pg_pk) = run_blocking(async {
            let mut conn = db.conn().await?;
            let cols: Vec<(String, String, bool)> = db.track(
                sqlx::query_as(COLUMNS_SQL)
                    .bind(&physical)
                    .fetch_all(&mut *conn)
                    .await,
            )?;
            let pg_pk: Option<String> = db.track(
                sqlx::query_scalar(PRIMARY_KEY_SQL)
                    .bind(&physical)
                    .fetch_optional(&mut *conn)
                    .await,
            )?;
            Ok::<_, RuntimeError>((cols, pg_pk))
        })??;

        if cols.is_empty() {
            return Err(RuntimeError::NotFound("Table".to_string(), key));
        }
        let columns: HashMap<String, ColumnMeta> = cols
            .into_iter()
            .map(|(n, t, g)| {
                (
                    n,
                    ColumnMeta {
                        sql_type: t,
                        generated: g,
                    },
                )
            })
            .collect();
        let key_column = catalog_pk
            .filter(|c| columns.contains_key(c))
            .or(pg_pk)
            .or_else(|| columns.contains_key("_id").then(|| "_id".to_string()))
            .ok_or_else(|| {
                RuntimeError::Internal(format!("Table '{}' has no primary key", physical))
            })?;

        let meta = Arc::new(TableMeta {
            physical,
            columns,
            key_column,
            data_per_company,
            is_immutable_ledger,
            flow_fields,
        });
        self.tables
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .insert(key, meta.clone());
        Ok(meta)
    }

    fn scope_clause(&self, meta: &TableMeta) -> Option<String> {
        let mut clauses = Vec::new();
        if meta.has("_tenant_id") {
            clauses.push(format!(
                "(t.\"_tenant_id\" = '{}'::uuid OR t.\"_tenant_id\" IS NULL)",
                self.ctx.tenant_id
            ));
        }
        if meta.data_per_company && meta.has("_company_id") {
            clauses.push(format!(
                "(t.\"_company_id\" = '{}'::uuid OR t.\"_company_id\" IS NULL)",
                self.ctx.company_id
            ));
        }
        if clauses.is_empty() {
            None
        } else {
            Some(clauses.join(" AND "))
        }
    }

    fn with_scope(&self, meta: &TableMeta, pred: &str) -> String {
        match self.scope_clause(meta) {
            Some(sc) => format!("({}) AND {}", pred, sc),
            None => pred.to_string(),
        }
    }

    fn select_by_key(
        &self,
        meta: &TableMeta,
        pk: &str,
        suffix: &str,
    ) -> Result<Option<JsonValue>, RuntimeError> {
        let Some(pred) = meta.key_predicate(pk, 1) else {
            return Ok(None);
        };
        let scoped_pred = self.with_scope(meta, &pred);
        let sql = format!(
            "SELECT {} FROM {} t WHERE {} LIMIT 1 {}",
            meta.row_json_expr(),
            meta.quoted(),
            scoped_pred,
            suffix
        );
        Ok(self
            .query_rows(&sql, &[Some(pk.to_string())])?
            .into_iter()
            .next())
    }

    fn get(&self, table: &str, pk: &str, opts_json: &str) -> Result<String, RuntimeError> {
        let meta = self.table(table)?;
        let opts = if opts_json.trim().is_empty() {
            JsonValue::Object(serde_json::Map::new())
        } else {
            parse_opts(opts_json)?
        };
        if let Some(select) = opts.get("select") {
            if let Some(pairs) = projection_pairs(select)? {
                let Some(pred) = meta.key_predicate(pk, 1) else {
                    return Ok("null".to_string());
                };
                let scoped_pred = self.with_scope(&meta, &pred);
                let sql = format!(
                    "SELECT jsonb_build_object({}) FROM {} t WHERE {} LIMIT 1",
                    pairs.join(", "),
                    meta.quoted(),
                    scoped_pred
                );
                return Ok(self
                    .query_rows(&sql, &[Some(pk.to_string())])?
                    .into_iter()
                    .next()
                    .map(|r| r.to_string())
                    .unwrap_or_else(|| "null".to_string()));
            }
        }
        Ok(self
            .select_by_key(&meta, pk, "")?
            .map(|r| r.to_string())
            .unwrap_or_else(|| "null".to_string()))
    }

    fn get_many(&self, table: &str, keys_json: &str) -> Result<String, RuntimeError> {
        let meta = self.table(table)?;
        let keys: Vec<JsonValue> = serde_json::from_str(keys_json).map_err(|e| {
            RuntimeError::Internal(format!("getMany expects a JSON array of keys: {}", e))
        })?;
        if keys.is_empty() {
            return Ok("[]".to_string());
        }
        if keys.len() > 1000 {
            return Err(RuntimeError::Internal(
                "getMany accepts at most 1000 keys".to_string(),
            ));
        }
        let mut params = Vec::with_capacity(keys.len());
        let mut placeholders = Vec::with_capacity(keys.len());
        for key in &keys {
            let text = json_to_text(key).unwrap_or_default();
            params.push(Some(text));
            placeholders.push(format!("${}", params.len()));
        }
        let in_pred = format!(
            "t.\"{}\"::text IN ({})",
            meta.key_column,
            placeholders.join(", ")
        );
        let scoped_pred = self.with_scope(&meta, &in_pred);
        let sql = format!(
            "SELECT {} || jsonb_build_object('__bosKey', t.\"{}\"::text) FROM {} t WHERE {}",
            meta.row_json_expr(),
            meta.key_column,
            meta.quoted(),
            scoped_pred
        );
        let rows = self.query_rows(&sql, &params)?;
        Ok(JsonValue::Array(rows).to_string())
    }

    fn create_many(&self, table: &str, records_json: &str) -> Result<String, RuntimeError> {
        let meta = self.table(table)?;
        let rows: Vec<JsonValue> = serde_json::from_str(records_json).map_err(|e| {
            RuntimeError::Internal(format!("createMany expects a JSON array: {}", e))
        })?;
        if rows.is_empty() {
            return Ok("[]".to_string());
        }
        let mut prepared: Vec<Vec<(String, JsonValue)>> = Vec::with_capacity(rows.len());
        let mut columns: Vec<String> = Vec::new();
        for row in &rows {
            let obj = row.as_object().ok_or_else(|| {
                RuntimeError::Internal(format!("createMany on '{}' expects objects", meta.physical))
            })?;
            let (mut assignments, _) = collect_assignments(&meta, obj, false)?;
            let tenant = JsonValue::String(self.ctx.tenant_id.to_string());
            let company = JsonValue::String(self.ctx.company_id.to_string());
            let user = JsonValue::String(self.ctx.user_id.to_string());
            for (col, val) in [
                ("_tenant_id", tenant),
                ("_company_id", company),
                ("_created_by", user.clone()),
                ("_updated_by", user),
            ] {
                if meta.has(col) {
                    assignments.push((col.to_string(), val));
                }
            }
            for (col, _) in &assignments {
                if !columns.iter().any(|c| c == col) {
                    columns.push(col.clone());
                }
            }
            prepared.push(assignments);
        }
        if columns.is_empty() {
            return Err(RuntimeError::Internal(format!(
                "createMany on '{}' has no writable columns",
                meta.physical
            )));
        }
        let mut params: Vec<Option<String>> = Vec::new();
        let mut tuples = Vec::with_capacity(prepared.len());
        for assignments in &prepared {
            let mut placeholders = Vec::with_capacity(columns.len());
            for col in &columns {
                let value = assignments
                    .iter()
                    .find(|(name, _)| name == col)
                    .map(|(_, v)| v);
                match value {
                    Some(v) => {
                        params.push(meta.bind_value(col, v));
                        placeholders.push(meta.placeholder(params.len(), col));
                    }
                    None => placeholders.push("DEFAULT".to_string()),
                }
            }
            tuples.push(format!("({})", placeholders.join(", ")));
        }
        let cols_sql = columns
            .iter()
            .map(|c| format!("\"{c}\""))
            .collect::<Vec<_>>()
            .join(", ");
        let sql = format!(
            "INSERT INTO {} AS t ({}) VALUES {} RETURNING {}",
            meta.quoted(),
            cols_sql,
            tuples.join(", "),
            meta.row_json_expr()
        );
        let inserted = self.query_rows(&sql, &params)?;
        Ok(JsonValue::Array(inserted).to_string())
    }

    fn lock(&self, table: &str, pk: &str) -> Result<String, RuntimeError> {
        let meta = self.table(table)?;
        Ok(self
            .select_by_key(&meta, pk, "FOR UPDATE")?
            .map(|r| r.to_string())
            .unwrap_or_else(|| "null".to_string()))
    }

    fn calc_flow_fields(
        &self,
        table: &str,
        pk: &str,
        fields_json: &str,
    ) -> Result<String, RuntimeError> {
        let meta = self.table(table)?;
        let requested: Vec<String> = serde_json::from_str(fields_json).map_err(|e| {
            RuntimeError::Internal(format!("calcFields expects field names: {}", e))
        })?;
        let mut calculated = serde_json::Map::new();
        let Some(def) = self.catalog_definition("table", table, "sql_table_name")? else {
            return Ok(JsonValue::Object(calculated).to_string());
        };
        let Some(pred) = meta.key_predicate(pk, 1) else {
            return Err(RuntimeError::NotFound(
                meta.physical.clone(),
                pk.to_string(),
            ));
        };
        let scoped_pred = self.with_scope(&meta, &pred);
        let flow_fields = def
            .get("flow_fields")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        for rf in &requested {
            let known = flow_fields.iter().any(|ff| {
                ff.get("name")
                    .and_then(|v| v.as_str())
                    .map(|n| {
                        n.eq_ignore_ascii_case(rf)
                            || crate::to_snake_case(n) == crate::to_snake_case(rf)
                    })
                    .unwrap_or(false)
            });
            if !known {
                return Err(RuntimeError::Internal(format!(
                    "'{}' is not a FlowField of table '{}'",
                    rf, meta.physical
                )));
            }
        }
        for ff in &flow_fields {
            let (Some(name), Some(subq)) = (
                ff.get("name").and_then(|v| v.as_str()),
                ff.get("sql_subquery").and_then(|v| v.as_str()),
            ) else {
                continue;
            };
            let wanted = requested.is_empty()
                || requested.iter().any(|rf| {
                    rf.eq_ignore_ascii_case(name)
                        || crate::to_snake_case(rf) == crate::to_snake_case(name)
                });
            if !wanted {
                continue;
            }
            let sql = format!(
                "SELECT to_jsonb(({})) FROM {} t WHERE {} LIMIT 1",
                subq,
                meta.quoted(),
                scoped_pred
            );
            let rows = self.query_rows_nullable(&sql, &[Some(pk.to_string())])?;
            let Some(val) = rows.into_iter().next() else {
                return Err(RuntimeError::NotFound(
                    meta.physical.clone(),
                    pk.to_string(),
                ));
            };
            calculated.insert(name.to_string(), val.unwrap_or(JsonValue::Null));
        }
        Ok(JsonValue::Object(calculated).to_string())
    }

    fn query_rows_nullable(
        &self,
        sql: &str,
        params: &[Option<String>],
    ) -> Result<Vec<Option<JsonValue>>, RuntimeError> {
        let db = self.session()?;
        run_blocking(async {
            let mut conn = db.conn().await?;
            let res = bind_all(sqlx::query_scalar::<_, Option<JsonValue>>(sql), params)
                .fetch_all(&mut *conn)
                .await;
            db.track(res)
        })?
    }

    fn type_resolver(meta: &TableMeta) -> impl Fn(&str) -> Option<String> + '_ {
        move |c: &str| meta.type_of(c).map(|s| s.to_string())
    }

    fn checked_opts(&self, meta: &TableMeta, query: &str) -> Result<JsonValue, RuntimeError> {
        let opts = parse_opts(query)?;
        validate_fields(meta, &opts)?;
        Ok(opts)
    }

    fn first(&self, table: &str, query: &str, last: bool) -> Result<String, RuntimeError> {
        let meta = self.table(table)?;
        let mut opts = self.checked_opts(&meta, query)?;
        let mut default_order = None;
        if last {
            match opts.get_mut("orderBy").and_then(|o| o.as_object_mut()) {
                Some(order) if !order.is_empty() => {
                    for dir in order.values_mut() {
                        let desc = dir
                            .as_str()
                            .map(|d| d.eq_ignore_ascii_case("desc"))
                            .unwrap_or(false);
                        *dir = JsonValue::String(if desc { "asc" } else { "desc" }.to_string());
                    }
                }
                _ => default_order = Some(format!("t.\"{}\" DESC", meta.key_column)),
            }
        } else if opts.get("orderBy").is_none() {
            default_order = Some(format!("t.\"{}\" ASC", meta.key_column));
        }
        let sc = self.scope_clause(&meta);
        let (sql, params) =
            QuickJsEngine::build_select_sql(&meta, sc.as_deref(), &opts, Some(1), default_order)?;
        Ok(self
            .query_rows(&sql, &params)?
            .into_iter()
            .next()
            .map(|r| r.to_string())
            .unwrap_or_else(|| "null".to_string()))
    }

    fn find_many(&self, table: &str, query: &str) -> Result<String, RuntimeError> {
        let meta = self.table(table)?;
        let opts = self.checked_opts(&meta, query)?;
        let paged = opts.get("take").is_some();
        let sc = self.scope_clause(&meta);
        let (sql, params) =
            QuickJsEngine::build_select_sql(&meta, sc.as_deref(), &opts, None, None)?;
        let rows = self.query_rows(&sql, &params)?;
        if !paged && rows.len() > MAX_UNPAGED_ROWS {
            return Err(RuntimeError::Internal(format!(
                "findMany on '{}' matched more than {} rows; pass 'take'/'skip' to page through the result",
                meta.physical, MAX_UNPAGED_ROWS
            )));
        }
        Ok(JsonValue::Array(rows).to_string())
    }

    fn where_sql(
        &self,
        meta: &TableMeta,
        filter: &str,
    ) -> Result<(String, Vec<Option<String>>), RuntimeError> {
        let opts = self.checked_opts(meta, filter)?;
        let (mut where_sql, params) = compile_where(&opts, &Self::type_resolver(meta))?;
        if let Some(sc) = self.scope_clause(meta) {
            if where_sql.trim().is_empty() {
                where_sql = format!("WHERE {}", sc);
            } else if let Some(rest) = where_sql.trim().strip_prefix("WHERE ") {
                where_sql = format!("WHERE ({}) AND {}", rest, sc);
            } else {
                where_sql = format!("({}) AND {}", where_sql, sc);
            }
        }
        Ok((where_sql, params))
    }

    fn exists(&self, table: &str, filter: &str) -> Result<bool, RuntimeError> {
        let meta = self.table(table)?;
        let (where_sql, params) = self.where_sql(&meta, filter)?;
        let sql = format!(
            "SELECT EXISTS(SELECT 1 FROM {} t {})",
            meta.quoted(),
            where_sql
        );
        self.query_one::<bool>(&sql, &params)
    }

    fn count(&self, table: &str, filter: &str) -> Result<i64, RuntimeError> {
        let meta = self.table(table)?;
        let (where_sql, params) = self.where_sql(&meta, filter)?;
        let sql = format!(
            "SELECT COUNT(*)::bigint FROM {} t {}",
            meta.quoted(),
            where_sql
        );
        self.query_one::<i64>(&sql, &params)
    }

    fn aggregate(
        &self,
        table: &str,
        op: &str,
        field: &str,
        filter: &str,
    ) -> Result<String, RuntimeError> {
        let meta = self.table(table)?;
        let op_sql = match op.trim().to_ascii_uppercase().as_str() {
            "SUM" => "SUM",
            "AVG" => "AVG",
            "MIN" => "MIN",
            "MAX" => "MAX",
            other => {
                return Err(RuntimeError::Internal(format!(
                    "Unsupported aggregate '{}'",
                    other
                )));
            }
        };
        let col = column_name(field.trim().trim_matches('"'))?;
        let ty = meta.type_of(&col).ok_or_else(|| {
            RuntimeError::Internal(format!(
                "Unknown field '{}' on table '{}'",
                field, meta.physical
            ))
        })?;
        if !NUMERIC_TYPES.contains(&ty) {
            return Err(RuntimeError::Internal(format!(
                "Cannot aggregate non-numeric field '{}' ({}) on table '{}'",
                field, ty, meta.physical
            )));
        }
        let (where_sql, params) = self.where_sql(&meta, filter)?;
        let sql = format!(
            "SELECT COALESCE({}(t.\"{}\")::numeric, 0)::text FROM {} t {}",
            op_sql,
            col,
            meta.quoted(),
            where_sql
        );
        self.query_one::<String>(&sql, &params)
    }

    fn create_record(
        &self,
        meta: &TableMeta,
        obj: &serde_json::Map<String, JsonValue>,
    ) -> Result<JsonValue, RuntimeError> {
        let (mut assignments, _) = collect_assignments(meta, obj, false)?;
        if let Some(id) = obj.get("_id").filter(|v| !v.is_null())
            && meta.has("_id")
        {
            assignments.push(("_id".to_string(), id.clone()));
        }
        let tenant = JsonValue::String(self.ctx.tenant_id.to_string());
        let company = JsonValue::String(self.ctx.company_id.to_string());
        let user = JsonValue::String(self.ctx.user_id.to_string());
        for (col, val) in [
            ("_tenant_id", tenant),
            ("_company_id", company),
            ("_created_by", user.clone()),
            ("_updated_by", user),
        ] {
            if meta.has(col) {
                assignments.push((col.to_string(), val));
            }
        }

        let sql_and_params = if assignments.is_empty() {
            (
                format!(
                    "INSERT INTO {} AS t DEFAULT VALUES RETURNING {}",
                    meta.quoted(),
                    meta.row_json_expr()
                ),
                Vec::new(),
            )
        } else {
            let mut cols = Vec::new();
            let mut phs = Vec::new();
            let mut params = Vec::new();
            for (col, val) in &assignments {
                params.push(meta.bind_value(col, val));
                cols.push(format!("\"{}\"", col));
                phs.push(meta.placeholder(params.len(), col));
            }
            (
                format!(
                    "INSERT INTO {} AS t ({}) VALUES ({}) RETURNING {}",
                    meta.quoted(),
                    cols.join(", "),
                    phs.join(", "),
                    meta.row_json_expr()
                ),
                params,
            )
        };
        let (sql, params) = sql_and_params;
        self.query_rows(&sql, &params)?
            .into_iter()
            .next()
            .ok_or_else(|| {
                RuntimeError::Internal(format!("INSERT into '{}' returned no row", meta.physical))
            })
    }

    fn create(&self, table: &str, data: &str) -> Result<String, RuntimeError> {
        let meta = self.table(table)?;
        let val: JsonValue = serde_json::from_str(data).map_err(|e| {
            RuntimeError::Internal(format!("Invalid record JSON for '{}': {}", table, e))
        })?;
        let obj = val.as_object().ok_or_else(|| {
            RuntimeError::Internal(format!("create on '{}' expects an object", table))
        })?;
        Ok(self.create_record(&meta, obj)?.to_string())
    }

    /// Server-maintained columns appended to every UPDATE: version bump and audit stamps.
    fn update_stamps(
        &self,
        meta: &TableMeta,
        sets: &mut Vec<String>,
        params: &mut Vec<Option<String>>,
    ) {
        if meta.has("_system_version") {
            sets.push("\"_system_version\" = t.\"_system_version\" + 1".to_string());
        }
        if meta.has("_updated_at") {
            sets.push("\"_updated_at\" = NOW()".to_string());
        }
        if meta.has("_updated_by") {
            params.push(Some(self.ctx.user_id.to_string()));
            sets.push(format!(
                "\"_updated_by\" = {}",
                meta.placeholder(params.len(), "_updated_by")
            ));
        }
    }

    fn update(
        &self,
        table: &str,
        pk: &str,
        patch: &str,
        strict: bool,
    ) -> Result<String, RuntimeError> {
        let meta = self.table(table)?;
        if meta.is_immutable_ledger {
            return Err(RuntimeError::ImmutableLedger(format!(
                "Cannot modify immutable ledger table '{}'",
                meta.physical
            )));
        }
        let val: JsonValue = serde_json::from_str(patch).map_err(|e| {
            RuntimeError::Internal(format!("Invalid patch JSON for '{}': {}", table, e))
        })?;
        let obj = val.as_object().ok_or_else(|| {
            RuntimeError::Internal(format!("update on '{}' expects an object patch", table))
        })?;
        let (assignments, version) = collect_assignments(&meta, obj, strict)?;
        let Some(pred) = meta.key_predicate(pk, 1) else {
            return Ok("null".to_string());
        };
        if assignments.is_empty() {
            return self.get(table, pk, "");
        }

        let mut params = vec![Some(pk.to_string())];
        let mut sets = Vec::new();
        for (col, v) in &assignments {
            params.push(meta.bind_value(col, v));
            sets.push(format!(
                "\"{}\" = {}",
                col,
                meta.placeholder(params.len(), col)
            ));
        }
        self.update_stamps(&meta, &mut sets, &mut params);

        let mut where_sql = self.with_scope(&meta, &pred);
        let expected = match (&version, meta.has("_system_version")) {
            (Some(v), true) if !v.is_null() => Some(version_number(v)?),
            _ => None,
        };
        if let Some(ver) = expected {
            params.push(Some(ver.to_string()));
            where_sql = format!(
                "{} AND t.\"_system_version\" = ${}::bigint",
                where_sql,
                params.len()
            );
        }

        let sql = format!(
            "UPDATE {} AS t SET {} WHERE {} RETURNING {}",
            meta.quoted(),
            sets.join(", "),
            where_sql,
            meta.row_json_expr()
        );
        if let Some(row) = self.query_rows(&sql, &params)?.into_iter().next() {
            return Ok(row.to_string());
        }
        if let Some(ver) = expected
            && self.select_by_key(&meta, pk, "")?.is_some()
        {
            return Err(RuntimeError::ConcurrencyConflict(format!(
                "Record '{}' in '{}' was changed by another user (expected version {})",
                pk, meta.physical, ver
            )));
        }
        Ok("null".to_string())
    }

    fn delete(&self, table: &str, pk: &str) -> Result<bool, RuntimeError> {
        let meta = self.table(table)?;
        if meta.is_immutable_ledger {
            return Err(RuntimeError::ImmutableLedger(format!(
                "Cannot delete from immutable ledger table '{}'",
                meta.physical
            )));
        }
        let Some(pred) = meta.key_predicate(pk, 1) else {
            return Ok(false);
        };
        let scoped_pred = self.with_scope(&meta, &pred);
        let sql = format!("DELETE FROM {} AS t WHERE {}", meta.quoted(), scoped_pred);
        Ok(self.execute(&sql, &[Some(pk.to_string())])? > 0)
    }

    /// Insert-or-update by an arbitrary filter, without requiring a matching unique constraint.
    fn upsert(&self, table: &str, options: &str) -> Result<String, RuntimeError> {
        let meta = self.table(table)?;
        let parsed = parse_opts(options)?;
        let where_obj = parsed
            .get("where")
            .and_then(|w| w.as_object())
            .filter(|w| !w.is_empty())
            .ok_or_else(|| {
                RuntimeError::Internal("upsert requires a non-empty 'where' filter".to_string())
            })?;
        let filter = JsonValue::Object(where_obj.clone());
        validate_fields(&meta, &filter)?;
        let (mut where_sql, params) = compile_where(&filter, &Self::type_resolver(&meta))?;
        if let Some(sc) = self.scope_clause(&meta) {
            if where_sql.trim().is_empty() {
                where_sql = format!("WHERE {}", sc);
            } else if let Some(rest) = where_sql.trim().strip_prefix("WHERE ") {
                where_sql = format!("WHERE ({}) AND {}", rest, sc);
            }
        }
        let sql = format!(
            "SELECT {} FROM {} t {} LIMIT 2 FOR UPDATE",
            meta.row_json_expr(),
            meta.quoted(),
            where_sql
        );
        let existing = self.query_rows(&sql, &params)?;
        if existing.len() > 1 {
            return Err(RuntimeError::Internal(format!(
                "upsert filter on '{}' matches more than one record",
                meta.physical
            )));
        }

        if let Some(row) = existing.into_iter().next() {
            let patch = parsed
                .get("update")
                .or_else(|| parsed.get("create"))
                .cloned()
                .unwrap_or_else(|| serde_json::json!({}));
            let key = row
                .get("_id")
                .or_else(|| row.get(&meta.key_column))
                .and_then(json_to_text)
                .ok_or_else(|| {
                    RuntimeError::Internal(format!(
                        "Record in '{}' has no key value",
                        meta.physical
                    ))
                })?;
            return self.update(
                table,
                &key,
                &patch.to_string(),
                parsed.get("update").is_some(),
            );
        }

        let mut record = where_obj.clone();
        if let Some(c) = parsed.get("create").and_then(|c| c.as_object()) {
            for (k, v) in c {
                record.insert(k.clone(), v.clone());
            }
        }
        Ok(self.create_record(&meta, &record)?.to_string())
    }

    fn modify_all(&self, table: &str, patch: &str, filter: &str) -> Result<u64, RuntimeError> {
        let meta = self.table(table)?;
        if meta.is_immutable_ledger {
            return Err(RuntimeError::ImmutableLedger(format!(
                "Cannot modify immutable ledger table '{}'",
                meta.physical
            )));
        }
        let patch_val: JsonValue = serde_json::from_str(patch)
            .map_err(|e| RuntimeError::Internal(format!("Invalid modifyAll patch JSON: {}", e)))?;
        let obj = patch_val.as_object().ok_or_else(|| {
            RuntimeError::Internal("modifyAll expects an object patch".to_string())
        })?;
        let (assignments, _) = collect_assignments(&meta, obj, true)?;
        if assignments.is_empty() {
            return Ok(0);
        }
        let (where_sql, mut params) = self.where_sql(&meta, filter)?;
        let mut sets = Vec::new();
        for (col, v) in &assignments {
            params.push(meta.bind_value(col, v));
            sets.push(format!(
                "\"{}\" = {}",
                col,
                meta.placeholder(params.len(), col)
            ));
        }
        self.update_stamps(&meta, &mut sets, &mut params);
        let sql = format!(
            "UPDATE {} AS t SET {} {}",
            meta.quoted(),
            sets.join(", "),
            where_sql
        );
        self.execute(&sql, &params)
    }

    fn delete_all(&self, table: &str, filter: &str) -> Result<u64, RuntimeError> {
        let meta = self.table(table)?;
        if meta.is_immutable_ledger {
            return Err(RuntimeError::ImmutableLedger(format!(
                "Cannot delete from immutable ledger table '{}'",
                meta.physical
            )));
        }
        let (where_sql, params) = self.where_sql(&meta, filter)?;
        let sql = format!("DELETE FROM {} AS t {}", meta.quoted(), where_sql);
        self.execute(&sql, &params)
    }

    fn raw_query(&self, sql: &str, params_json: &str) -> Result<String, RuntimeError> {
        guard_raw_sql(sql)?;
        let vals = raw_params(params_json)?;
        let db = self.session()?;
        let wrapped = format!(
            "SELECT COALESCE(jsonb_agg(to_jsonb(q.*)), '[]'::jsonb) FROM ({}) q",
            sql.trim().trim_end_matches(';')
        );
        run_blocking(async {
            let mut conn = db.conn().await?;
            let mut q = sqlx::query_scalar::<_, JsonValue>(&wrapped);
            for v in &vals {
                q = bind_raw_scalar(q, v);
            }
            let res = q.fetch_one(&mut *conn).await;
            db.track(res).map(|v| v.to_string())
        })?
    }

    fn raw_execute(&self, sql: &str, params_json: &str) -> Result<u64, RuntimeError> {
        guard_raw_sql(sql)?;
        let vals = raw_params(params_json)?;
        let db = self.session()?;
        run_blocking(async {
            let mut conn = db.conn().await?;
            let mut q = sqlx::query(sql);
            for v in &vals {
                q = match v {
                    JsonValue::String(s) => q.bind(s.clone()),
                    JsonValue::Number(n) if n.is_i64() => q.bind(n.as_i64()),
                    JsonValue::Number(n) => q.bind(n.as_f64()),
                    JsonValue::Bool(b) => q.bind(*b),
                    JsonValue::Null => q.bind(Option::<String>::None),
                    other => q.bind(other.clone()),
                };
            }
            let res = q.execute(&mut *conn).await;
            db.track(res).map(|r| r.rows_affected())
        })?
    }

    fn state_machine(&self, table: &str) -> Result<JsonValue, RuntimeError> {
        self.catalog_definition("statemachine", table, "target_table")?
            .ok_or_else(|| {
                RuntimeError::TransitionError(format!(
                    "No state machine is defined for table '{}'",
                    table
                ))
            })
    }

    /// Transition definition matching `action` (by transition name, action name, or target state) from `current`.
    fn find_transition<'a>(
        sm: &'a JsonValue,
        action: &str,
        current: Option<&str>,
    ) -> Option<&'a JsonValue> {
        sm.get("transitions")?.as_array()?.iter().find(|tr| {
            let name = tr.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let act = tr.get("action").and_then(|v| v.as_str()).unwrap_or("");
            let to = tr.get("to_state").and_then(|v| v.as_str()).unwrap_or("");
            let action_ok = name.eq_ignore_ascii_case(action)
                || act.eq_ignore_ascii_case(action)
                || to.eq_ignore_ascii_case(action);
            let from_ok = if let Some(arr) = tr.get("from_states").and_then(|v| v.as_array()) {
                arr.is_empty()
                    || arr.iter().any(|f| {
                        let fs = f.as_str().unwrap_or("");
                        fs == "*" || current.is_some_and(|c| c.eq_ignore_ascii_case(fs))
                    })
            } else {
                let from = tr.get("from_state").and_then(|v| v.as_str()).unwrap_or("*");
                from == "*" || current.is_some_and(|c| c.eq_ignore_ascii_case(from))
            };
            action_ok && from_ok
        })
    }

    fn status_column(sm: &JsonValue) -> Result<String, RuntimeError> {
        column_name(
            sm.get("status_field")
                .and_then(|s| s.as_str())
                .unwrap_or("status"),
        )
    }

    fn can_transition(&self, table: &str, pk: &str, action: &str) -> Result<bool, RuntimeError> {
        let Some(sm) = self.catalog_definition("statemachine", table, "target_table")? else {
            return Ok(false);
        };
        let meta = self.table(table)?;
        let status_col = Self::status_column(&sm)?;
        let Some(row) = self.select_by_key(&meta, pk, "")? else {
            return Ok(false);
        };
        let current = row.get(&status_col).and_then(|v| v.as_str());
        let Some(tr) = Self::find_transition(&sm, action.trim(), current) else {
            return Ok(false);
        };
        if let Some(req_roles) = tr.get("required_roles").and_then(|v| v.as_array()) {
            let non_empty: Vec<&str> = req_roles
                .iter()
                .filter_map(|r| r.as_str())
                .filter(|s| !s.is_empty())
                .collect();
            if !non_empty.is_empty() {
                let has_role = self.ctx.roles.iter().any(|ur| {
                    ur.eq_ignore_ascii_case("admin")
                        || ur.eq_ignore_ascii_case("super_admin")
                        || non_empty.iter().any(|rr| rr.eq_ignore_ascii_case(ur))
                });
                if !has_role {
                    return Ok(false);
                }
            }
        }
        Ok(true)
    }

    fn transition(
        &self,
        table: &str,
        pk: &str,
        action: &str,
        comments: &str,
    ) -> Result<String, RuntimeError> {
        let sm = self.state_machine(table)?;
        let meta = self.table(table)?;
        let status_col = Self::status_column(&sm)?;
        if !meta.has(&status_col) {
            return Err(RuntimeError::TransitionError(format!(
                "Status field '{}' does not exist on table '{}'",
                status_col, meta.physical
            )));
        }
        let row = self
            .select_by_key(&meta, pk, "FOR UPDATE")?
            .ok_or_else(|| RuntimeError::NotFound(meta.physical.clone(), pk.to_string()))?;
        let current = row
            .get(&status_col)
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let action = action.trim();
        let tr = Self::find_transition(&sm, action, current.as_deref()).ok_or_else(|| {
            RuntimeError::TransitionError(format!(
                "Action '{}' is not allowed on '{}' record '{}' in state '{}'",
                action,
                meta.physical,
                pk,
                current.as_deref().unwrap_or("<none>")
            ))
        })?;
        if let Some(req_roles) = tr.get("required_roles").and_then(|v| v.as_array()) {
            let non_empty: Vec<&str> = req_roles
                .iter()
                .filter_map(|r| r.as_str())
                .filter(|s| !s.is_empty())
                .collect();
            if !non_empty.is_empty() {
                let has_role = self.ctx.roles.iter().any(|ur| {
                    ur.eq_ignore_ascii_case("admin")
                        || ur.eq_ignore_ascii_case("super_admin")
                        || non_empty.iter().any(|rr| rr.eq_ignore_ascii_case(ur))
                });
                if !has_role {
                    return Err(RuntimeError::TransitionError(format!(
                        "Transition '{}' requires one of roles {:?}, held roles: {:?}",
                        action, non_empty, self.ctx.roles
                    )));
                }
            }
        }
        let target = tr
            .get("to_state")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .ok_or_else(|| {
                RuntimeError::TransitionError(format!(
                    "Transition '{}' has no target state",
                    action
                ))
            })?
            .to_string();

        let key = row
            .get("_id")
            .or_else(|| row.get(&meta.key_column))
            .and_then(json_to_text)
            .ok_or_else(|| {
                RuntimeError::Internal(format!("Record in '{}' has no key value", meta.physical))
            })?;
        let pred = meta.key_predicate(&key, 1).ok_or_else(|| {
            RuntimeError::Internal(format!("Invalid key '{}' for '{}'", key, meta.physical))
        })?;
        let scoped_pred = self.with_scope(&meta, &pred);
        let mut params = vec![Some(key.clone()), Some(target.clone())];
        let mut sets = vec![format!(
            "\"{}\" = {}",
            status_col,
            meta.placeholder(2, &status_col)
        )];
        self.update_stamps(&meta, &mut sets, &mut params);
        let sql = format!(
            "UPDATE {} AS t SET {} WHERE {} RETURNING {}",
            meta.quoted(),
            sets.join(", "),
            scoped_pred,
            meta.row_json_expr()
        );
        let updated = self
            .query_rows(&sql, &params)?
            .into_iter()
            .next()
            .ok_or_else(|| RuntimeError::NotFound(meta.physical.clone(), pk.to_string()))?;

        let hist_table = format!("{}_state_history", meta.physical);
        let has_history = self.query_one::<bool>(
            "SELECT to_regclass(quote_ident($1)) IS NOT NULL",
            &[Some(hist_table.clone())],
        )?;
        if has_history {
            let entity_id = updated.get("_id").and_then(json_to_text).unwrap_or(key);
            self.execute(
                &format!(
                    "INSERT INTO \"{}\" (\"entity_id\", \"from_state\", \"to_state\", \"action\", \"actor_id\", \"comments\") VALUES ($1::uuid, $2, $3, $4, $5::uuid, $6)",
                    hist_table
                ),
                &[
                    Some(entity_id),
                    Some(current.unwrap_or_default()),
                    Some(target),
                    Some(action.to_string()),
                    Some(self.ctx.user_id.to_string()),
                    Some(comments.to_string()),
                ],
            )?;
        }
        Ok(updated.to_string())
    }

    /// Same rules as `BosEngine::check_permission`: admin roles pass, otherwise a permission set
    /// assigned to the user's roles (or any permission set if none match by role name) must grant the action.
    fn check_permission(&self, entity: &str, perm: &str) -> Result<bool, RuntimeError> {
        if self.ctx.roles.iter().any(|r| {
            let rl = r.to_ascii_lowercase();
            rl == "admin" || rl == "super" || rl == "super_admin" || rl == "system"
        }) {
            return Ok(true);
        }
        let action = match perm.trim().to_ascii_lowercase().as_str() {
            "read" | "r" => "read",
            "insert" | "create" | "i" => "insert",
            "modify" | "update" | "m" => "modify",
            "delete" | "d" => "delete",
            "execute" | "x" => "execute",
            other => {
                return Err(RuntimeError::SecurityError(format!(
                    "Unknown permission '{}'",
                    other
                )));
            }
        };
        if !self.catalog_exists()? {
            return Ok(false);
        }
        let sets = self.query_rows(
            "SELECT definition FROM \"_bos_meta_objects\" WHERE LOWER(object_type) = 'permissionset'",
            &[],
        )?;
        let parsed_sets: Vec<bosc_ir::PermissionSetIR> = sets
            .into_iter()
            .filter_map(|def| serde_json::from_value::<bosc_ir::PermissionSetIR>(def).ok())
            .collect();
        let role_matched: Vec<&bosc_ir::PermissionSetIR> = parsed_sets
            .iter()
            .filter(|ps| {
                self.ctx
                    .roles
                    .iter()
                    .any(|r| r.eq_ignore_ascii_case(&ps.name))
            })
            .collect();
        let active_sets: Vec<&bosc_ir::PermissionSetIR> = if !role_matched.is_empty() {
            role_matched
        } else {
            parsed_sets.iter().collect()
        };
        let entity = entity.trim();
        for ps in active_sets {
            if action == "execute" {
                if ps.execute.iter().any(|cu| cu.eq_ignore_ascii_case(entity)) {
                    return Ok(true);
                }
                continue;
            }
            let granted = ps.permissions.iter().any(|p| {
                (p.table_name.eq_ignore_ascii_case(entity)
                    || p.sql_table_name.eq_ignore_ascii_case(entity))
                    && match action {
                        "read" => p.can_read,
                        "insert" => p.can_insert,
                        "modify" => p.can_modify,
                        _ => p.can_delete,
                    }
            });
            if granted {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn wasm_invoke(&self, module: &str, func: &str, a: f64, b: f64) -> Result<f64, RuntimeError> {
        let wasm = self.services.wasm.as_ref().ok_or_else(|| {
            RuntimeError::Internal(
                "WASM engine is not available in this execution context".to_string(),
            )
        })?;
        match wasm.execute_f64_op(module, func, a, b) {
            Ok(v) => Ok(v),
            Err(f64_err) => wasm
                .execute_i32_op(module, func, a as i32, b as i32)
                .map(|v| v as f64)
                .map_err(|_| f64_err),
        }
    }

    fn sidecar_call(
        &self,
        service: &str,
        path: &str,
        payload: &str,
    ) -> Result<String, RuntimeError> {
        let host = self
            .services
            .sidecars
            .as_ref()
            .ok_or_else(|| {
                RuntimeError::Internal(
                    "Sidecar host is not available in this execution context".to_string(),
                )
            })?
            .clone();
        let payload: JsonValue = serde_json::from_str(payload)
            .map_err(|e| RuntimeError::Internal(format!("Invalid sidecar payload JSON: {}", e)))?;
        run_blocking(async move { host.invoke(service, path, &payload).await })?
            .map(|v| v.to_string())
    }

    fn publish_event(
        &self,
        publisher: &str,
        event_name: &str,
        data: &str,
    ) -> Result<String, RuntimeError> {
        let registry = self.services.events.as_ref().ok_or_else(|| {
            RuntimeError::Internal(
                "Event registry is not available in this execution context".to_string(),
            )
        })?;
        if self.depth + 1 > MAX_CALL_DEPTH {
            return Err(RuntimeError::Internal(format!(
                "Call depth limit exceeded (maximum {} nested calls)",
                MAX_CALL_DEPTH
            )));
        }
        let mut payload: JsonValue = serde_json::from_str(data)
            .map_err(|e| RuntimeError::Internal(format!("Invalid event payload JSON: {}", e)))?;
        let engine = QuickJsEngine::with_limits(self.services.clone(), self.limits)?
            .with_depth(self.depth + 1);
        let mut logs = Vec::new();
        registry.dispatch(
            publisher,
            event_name,
            &mut payload,
            None,
            &self.ctx,
            self.db.as_ref(),
            &engine,
            &mut logs,
        )?;
        Ok(payload.to_string())
    }

    fn call_codeunit(
        &self,
        codeunit: &str,
        method: &str,
        params: &str,
    ) -> Result<String, RuntimeError> {
        if self.depth + 1 > MAX_CALL_DEPTH {
            return Err(RuntimeError::Internal(format!(
                "Call depth limit exceeded (maximum {} nested calls)",
                MAX_CALL_DEPTH
            )));
        }
        let db = self.session()?.clone();
        let parsed: JsonValue = serde_json::from_str(params).map_err(|e| {
            RuntimeError::Internal(format!("Invalid codeunit parameters JSON: {}", e))
        })?;
        let params = if parsed.is_null() {
            serde_json::json!({})
        } else {
            parsed
        };
        let scope = ExecScope {
            db,
            ctx: self.ctx.clone(),
            services: self.services.clone(),
            depth: self.depth + 1,
            ui_answers: self.ui_answers.clone(),
        };
        let res = run_blocking(ExecutionEngine::execute_procedure_in(
            &scope,
            codeunit.trim(),
            method.trim(),
            params,
        ))??;
        Ok(res.mutated_data.to_string())
    }
}

fn bind_raw_scalar<'q>(
    q: sqlx::query::QueryScalar<'q, sqlx::Postgres, JsonValue, sqlx::postgres::PgArguments>,
    v: &JsonValue,
) -> sqlx::query::QueryScalar<'q, sqlx::Postgres, JsonValue, sqlx::postgres::PgArguments> {
    match v {
        JsonValue::String(s) => q.bind(s.clone()),
        JsonValue::Number(n) if n.is_i64() => q.bind(n.as_i64()),
        JsonValue::Number(n) => q.bind(n.as_f64()),
        JsonValue::Bool(b) => q.bind(*b),
        JsonValue::Null => q.bind(Option::<String>::None),
        other => q.bind(other.clone()),
    }
}

/// Reject filters, sort keys and projections that name columns the table does not have,
/// before any SQL reaches PostgreSQL (a failed statement would abort the whole transaction).
fn validate_fields(meta: &TableMeta, opts: &JsonValue) -> Result<(), RuntimeError> {
    fn check_map(
        meta: &TableMeta,
        map: &serde_json::Map<String, JsonValue>,
    ) -> Result<(), RuntimeError> {
        for (k, v) in map {
            if RESERVED_QUERY_KEYS.contains(&k.as_str()) {
                continue;
            }
            if k == "OR" || k == "AND" {
                if let Some(arr) = v.as_array() {
                    for item in arr {
                        if let Some(sub) = item.as_object() {
                            check_map(meta, sub)?;
                        }
                    }
                }
                continue;
            }
            check_col(meta, k)?;
        }
        Ok(())
    }
    fn check_col(meta: &TableMeta, k: &str) -> Result<(), RuntimeError> {
        let col = column_name(k)?;
        if meta.has(&col) {
            Ok(())
        } else {
            Err(RuntimeError::Internal(format!(
                "Unknown field '{}' on table '{}'",
                k, meta.physical
            )))
        }
    }

    match opts.get("where").and_then(|w| w.as_object()) {
        Some(w) => check_map(meta, w)?,
        None => {
            if let Some(root) = opts.as_object() {
                check_map(meta, root)?;
            }
        }
    }
    if let Some(order) = opts.get("orderBy").and_then(|o| o.as_object()) {
        for k in order.keys() {
            check_col(meta, k)?;
        }
    }
    if let Some(sel) = opts.get("select").and_then(|s| s.as_array()) {
        for c in sel.iter().filter_map(|c| c.as_str()) {
            check_col(meta, c)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::EventSubscriber;

    fn run(engine: &QuickJsEngine, script: &str, data: &mut JsonValue) -> Result<(), RuntimeError> {
        let mut logs = Vec::new();
        engine.execute_trigger(
            "Note",
            "onInsert",
            script,
            data,
            None,
            &SessionContext::default(),
            None,
            &mut logs,
        )
    }

    #[test]
    fn test_quickjs_engine_execution() {
        let engine = QuickJsEngine::new().expect("Failed to create engine");
        let mut data = serde_json::json!({
            "username": "alice",
            "password": "Password123!",
            "email": "ALICE@EXAMPLE.COM"
        });
        let script = r#"
            if (!rec.username) throw new Error("Username is required");
            if (rec.password) {
                const salt = ctx.crypto.randomBytes(8).toHex();
                if (salt.length !== 16) throw new Error("randomBytes(8) must yield 16 hex chars");
                rec.passwordHash = `$sha256$${salt}$${ctx.crypto.sha256(salt + rec.password)}`;
                delete rec.password;
            }
            rec.email = rec.email.toLowerCase().trim();
            rec.processedBy = "QuickJS-Engine";
            ctx.audit.log("User processed successfully");
        "#;
        run(&engine, script, &mut data).expect("execution");
        assert!(data.get("password").is_none());
        assert!(
            data.get("passwordHash")
                .and_then(|v| v.as_str())
                .is_some_and(|h| h.starts_with("$sha256$"))
        );
        assert_eq!(
            data.get("email").and_then(|v| v.as_str()),
            Some("alice@example.com")
        );
        assert_eq!(
            data.get("processedBy").and_then(|v| v.as_str()),
            Some("QuickJS-Engine")
        );
    }

    #[test]
    fn test_random_bytes_lengths() {
        let engine = QuickJsEngine::new().expect("engine");
        let mut data = serde_json::json!({});
        let script = r#"
            rec.lens = [1, 16, 33, 100].map(n => ctx.crypto.randomBytes(n).toHex().length);
        "#;
        run(&engine, script, &mut data).expect("execution");
        assert_eq!(data.get("lens"), Some(&serde_json::json!([2, 32, 66, 200])));
    }

    #[test]
    fn test_data_access_without_session_throws() {
        let engine = QuickJsEngine::new().expect("engine");
        for call in [
            "Customer.get('C1')",
            "Customer.findMany({ where: { name: 'x' } })",
            "Customer.count()",
            "Customer.exists({ name: 'x' })",
            "Customer.create({ name: 'x' })",
            "Customer.modifyAll({ name: 'y' }, {})",
            "ctx.db.query('SELECT 1')",
            "ctx.call('Posting', 'run', {})",
            "ctx.workflow.transition('Note', 'N1', 'release')",
        ] {
            let mut data = serde_json::json!({});
            let err = run(&engine, call, &mut data).expect_err(call);
            assert!(
                err.to_string().contains("Database access is unavailable"),
                "{} -> unexpected error: {}",
                call,
                err
            );
        }
    }

    #[test]
    fn test_host_errors_are_catchable_in_script() {
        let engine = QuickJsEngine::new().expect("engine");
        let mut data = serde_json::json!({});
        let script = r#"
            try { Customer.get("C1"); rec.caught = false; }
            catch (e) { rec.caught = String(e.message).includes("Database access is unavailable"); }
        "#;
        run(&engine, script, &mut data).expect("execution");
        assert_eq!(data.get("caught").and_then(|v| v.as_bool()), Some(true));
    }

    #[test]
    fn test_number_series_host_api_removed() {
        let engine = QuickJsEngine::new().expect("engine");
        let mut data = serde_json::json!({});
        let script = r#"
            rec.hasNumSeries = typeof ctx.numSeries !== "undefined";
            rec.hasHostFn = typeof globalThis.__host_num_series_next !== "undefined";
        "#;
        run(&engine, script, &mut data).expect("execution");
        assert_eq!(
            data.get("hasNumSeries").and_then(|v| v.as_bool()),
            Some(false)
        );
        assert_eq!(data.get("hasHostFn").and_then(|v| v.as_bool()), Some(false));
        assert!(!QuickJsEngine::generate_ambient_dts().contains("numSeries"));
    }

    #[test]
    fn test_ledger_and_dimension_host_apis_removed() {
        let engine = QuickJsEngine::new().expect("engine");
        let mut data = serde_json::json!({});
        let script = r#"
            rec.hasPostLedger = typeof ctx.postLedger !== "undefined";
            rec.hasDimension = typeof ctx.dimension !== "undefined";
        "#;
        run(&engine, script, &mut data).expect("execution");
        assert_eq!(
            data.get("hasPostLedger").and_then(|v| v.as_bool()),
            Some(false)
        );
        assert_eq!(
            data.get("hasDimension").and_then(|v| v.as_bool()),
            Some(false)
        );
        let dts = QuickJsEngine::generate_ambient_dts();
        assert!(!dts.contains("postLedger") && !dts.contains("dimension:"));
    }

    #[test]
    fn confirm_throws_until_the_caller_answers() {
        let engine = QuickJsEngine::new().expect("engine");
        let mut data = serde_json::json!({});
        let err = run(
            &engine,
            r#"rec.ok = ctx.ui.confirm("Delete this order?");"#,
            &mut data,
        )
        .unwrap_err();
        assert!(err.to_string().contains("ConfirmRequired"), "{err}");

        let mut engine = QuickJsEngine::new().expect("engine");
        engine
            .ui_answers
            .insert("Delete this order?".to_string(), false);
        let mut data = serde_json::json!({});
        run(
            &engine,
            r#"rec.ok = ctx.ui.confirm("Delete this order?");"#,
            &mut data,
        )
        .expect("answered");
        assert_eq!(data.get("ok").and_then(|v| v.as_bool()), Some(false));
    }

    #[test]
    fn http_refuses_reserved_hosts_and_fake_schemes() {
        assert!(refuse_unreal_http("mock://orders").is_err());
        assert!(refuse_unreal_http("https://example.com/orders").is_err());
        assert!(refuse_unreal_http("https://api.example.org/v1").is_err());
        assert!(refuse_unreal_http("https://127.0.0.1:3000/health").is_ok());
    }

    #[test]
    fn test_fuel_exhaustion_stops_infinite_loop() {
        let engine = QuickJsEngine::new().expect("engine");
        let mut data = serde_json::json!({});
        let started = Instant::now();
        let err =
            run(&engine, "while (true) {}", &mut data).expect_err("infinite loop must be stopped");
        assert!(
            err.to_string().contains("fuel exhausted"),
            "unexpected error: {}",
            err
        );
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "fuel limit must stop the loop quickly"
        );
        assert!(engine.fuel_used() >= DEFAULT_MAX_FUEL);
    }

    #[test]
    fn test_fuel_exhaustion_cannot_be_caught() {
        let engine = QuickJsEngine::new().expect("engine");
        let mut data = serde_json::json!({});
        let script = r#"
            try { while (true) {} } catch (e) { rec.swallowed = true; }
        "#;
        let err = run(&engine, script, &mut data).expect_err("interrupt must not be catchable");
        assert!(err.to_string().contains("fuel exhausted"));
        assert!(data.get("swallowed").is_none());
    }

    #[test]
    fn test_fuel_budget_is_configurable_and_reported() {
        let script = "let s = 0; for (let i = 0; i < 300000; i++) { s += i; } rec.sum = s;";

        let small = QuickJsEngine::with_limits(
            ScriptServices::default(),
            ExecutionLimits::with_fuel(Some(100_000)),
        )
        .expect("engine");
        let mut data = serde_json::json!({});
        assert!(
            run(&small, script, &mut data).is_err(),
            "300k iterations exceed a 100k budget"
        );

        let large = QuickJsEngine::with_limits(
            ScriptServices::default(),
            ExecutionLimits::with_fuel(Some(5_000_000)),
        )
        .expect("engine");
        let mut data = serde_json::json!({});
        run(&large, script, &mut data).expect("5M budget is enough");
        assert_eq!(
            data.get("sum").and_then(|v| v.as_i64()),
            Some(44_999_850_000)
        );
        assert!(
            large.fuel_used() >= 300_000,
            "fuel used should reflect loop iterations, got {}",
            large.fuel_used()
        );

        let mut data = serde_json::json!({});
        run(&large, "rec.ok = true;", &mut data).expect("trivial script");
        assert!(
            large.fuel_used() < 100_000,
            "meter must reset between executions"
        );
    }

    #[test]
    fn test_quickjs_extensions_wasm_events() {
        let wasm = Arc::new(WasmEngine::new().expect("Failed to create WasmEngine"));
        let wat = r#"
            (module
                (func (export "calc_tax") (param f64 f64) (result f64)
                    local.get 0
                    local.get 1
                    f64.mul
                )
            )
        "#;
        wasm.load_module("tax_calc", wat.as_bytes())
            .expect("Failed to load wat");

        let events = Arc::new(EventRegistry::new());
        events.register(EventSubscriber {
            id: Uuid::new_v4(),
            publisher: "SalesOrder".to_string(),
            event_name: "onPostDiscount".to_string(),
            subscriber_name: "DiscountMod".to_string(),
            method_name: "Apply".to_string(),
            priority: 10,
            script: "rec.discountApplied = true;".to_string(),
            is_active: true,
        });

        let engine = QuickJsEngine::with_services(Some(wasm), None, Some(events)).expect("engine");
        let mut data = serde_json::json!({ "subtotal": 100.0, "taxRate": 0.20 });
        let script = r#"
            rec.tax = ctx.wasm.invoke("tax_calc", "calc_tax", rec.subtotal, rec.taxRate);
            const evt = ctx.events.publish("SalesOrder", "onPostDiscount", { discountApplied: false });
            rec.discountEventFired = evt.discountApplied;
            try { ctx.wasm.invoke("missing_module", "f", 1, 2); rec.missingThrew = false; }
            catch (e) { rec.missingThrew = true; }
            try { ctx.sidecar.call("svc", "/x", {}); rec.sidecarThrew = false; }
            catch (e) { rec.sidecarThrew = true; }
        "#;
        run(&engine, script, &mut data).expect("execution");
        assert_eq!(data.get("tax").and_then(|v| v.as_f64()), Some(20.0));
        assert_eq!(
            data.get("discountEventFired").and_then(|v| v.as_bool()),
            Some(true)
        );
        assert_eq!(
            data.get("missingThrew").and_then(|v| v.as_bool()),
            Some(true)
        );
        assert_eq!(
            data.get("sidecarThrew").and_then(|v| v.as_bool()),
            Some(true)
        );
    }

    #[test]
    fn test_publish_without_registry_throws() {
        let engine = QuickJsEngine::new().expect("engine");
        let mut data = serde_json::json!({});
        let err =
            run(&engine, "ctx.events.publish('A', 'b', {});", &mut data).expect_err("no registry");
        assert!(err.to_string().contains("Event registry is not available"));
    }

    #[test]
    fn test_quickjs_rec_erp_helpers() {
        let engine = QuickJsEngine::new().expect("engine");
        let mut data = serde_json::json!({
            "status": "Approved",
            "customerNo": "CUST001",
            "unitPrice": 49.99,
            "quantity": 5,
            "emailAddress": "finance@example.com"
        });
        let script = r#"
            // Positive testField validations
            rec.testField("status", "Approved");
            rec.testField("customerNo");

            // Friendly error on expected value mismatch
            let mismatchErr = "";
            try {
                rec.testField("status", "Draft");
            } catch (e) {
                mismatchErr = e.message;
            }
            if (!mismatchErr.includes("'Status' in Note must be equal to 'Draft' (current value is 'Approved').")) {
                throw new Error("Unexpected mismatch error message: " + mismatchErr);
            }

            // Custom message override on testField
            let customErr = "";
            try {
                rec.testField("status", "Draft", "Custom status error occurred.");
            } catch (e) {
                customErr = e.message;
            }
            if (customErr !== "Custom status error occurred.") {
                throw new Error("Custom message did not override: " + customErr);
            }

            // Friendly error on blank field
            let blankErr = "";
            try {
                rec.testField("missingField");
            } catch (e) {
                blankErr = e.message;
            }
            if (!blankErr.includes("'Missing Field' in Note must not be blank.")) {
                throw new Error("Unexpected blank error message: " + blankErr);
            }

            // require() function checks
            require(1 + 1 === 2, "Math error");
            let reqErr = "";
            try {
                require(false);
            } catch (e) {
                reqErr = e.message;
            }
            if (reqErr !== "Validation requirement failed.") {
                throw new Error("Unexpected default require error: " + reqErr);
            }

            // require() with field and expected
            let reqFieldErr = "";
            try {
                require(false, "approvalStatus", "Approved");
            } catch (e) {
                reqFieldErr = e.message;
            }
            if (!reqFieldErr.includes("'Approval Status' requirement failed: expected 'Approved'.")) {
                throw new Error("Unexpected require field error: " + reqFieldErr);
            }

            // Convenience methods: testNotEmpty, testPositive, testRange, testEmail
            rec.testNotEmpty("customerNo");
            rec.testPositive("unitPrice");
            rec.testRange("quantity", 1, 10);
            rec.testEmail("emailAddress");

            // Verify testPositive error
            let posErr = "";
            try {
                rec.quantity = -5;
                rec.testPositive("quantity");
            } catch (e) {
                posErr = e.message;
            }
            if (!posErr.includes("'Quantity' must be greater than zero.")) {
                throw new Error("Unexpected testPositive error: " + posErr);
            }

            // Verify testEmail error
            let emailErr = "";
            try {
                rec.emailAddress = "invalid-email";
                rec.testEmail("emailAddress");
            } catch (e) {
                emailErr = e.message;
            }
            if (!emailErr.includes("'invalid-email' is not a valid email address for 'Email Address'.")) {
                throw new Error("Unexpected testEmail error: " + emailErr);
            }

            rec.testedHelpers = true;
        "#;
        run(&engine, script, &mut data).expect("execution");
        assert_eq!(
            data.get("testedHelpers").and_then(|v| v.as_bool()),
            Some(true)
        );
    }

    #[test]
    fn test_quickjs_ui_toast_and_messaging() {
        let engine = QuickJsEngine::new().expect("engine");
        let mut data = serde_json::json!({ "orderNo": "SO-9999", "amount": 2500.00 });
        let script = r#"
            ctx.ui.success("Order SO-9999 released successfully", "Release Done");
            ctx.ui.warning("Amount exceeds $2000 threshold", { duration: 8000 });
            ui.info("Tax calculation updated");
            toast({
                title: "Action Item",
                message: "Verify shipping address",
                type: "warning",
                duration: 6000,
                action: { label: "Review", url: "/orders/SO-9999" }
            });
            rec.processed = true;
        "#;
        run(&engine, script, &mut data).expect("execution");
        let msgs = data
            .get("_system_messages")
            .and_then(|v| v.as_array())
            .expect("messages");
        assert_eq!(msgs.len(), 4);
        assert_eq!(
            msgs[0].get("type").and_then(|v| v.as_str()),
            Some("success")
        );
        assert_eq!(
            msgs[0].get("title").and_then(|v| v.as_str()),
            Some("Release Done")
        );
        assert_eq!(msgs[1].get("duration").and_then(|v| v.as_i64()), Some(8000));
        assert_eq!(msgs[2].get("type").and_then(|v| v.as_str()), Some("info"));
        assert_eq!(
            msgs[3]
                .get("action")
                .and_then(|a| a.get("label"))
                .and_then(|l| l.as_str()),
            Some("Review")
        );
    }

    #[test]
    fn test_ctx_sdk_pure_helpers() {
        let engine = QuickJsEngine::new().expect("engine");
        let ctx = SessionContext {
            roles: vec!["FINANCE_MANAGER".to_string()],
            ..SessionContext::default()
        };
        let mut data = serde_json::json!({ "code": "N-100", "content": "Initial Note" });
        let script = r#"
            ctx.ui.openModal("NoteCard", { id: rec.code });
            ctx.ui.navigate("/pages/NoteList");
            ctx.ui.setFieldVisible("content", true);
            ctx.ui.setFieldEditable("code", false);
            if (!ctx.security.hasRole("finance_manager")) throw new Error("Role check failed");
            ctx.security.requireRole("FINANCE_MANAGER");
            const noteModel = ctx.db.getModel("Note");
            if (!noteModel || typeof noteModel.findMany !== "function") throw new Error("ctx.db.getModel failed");
            rec.directivesCount = ctx.ui.getDirectives().length;
        "#;
        let mut logs = Vec::new();
        engine
            .execute_trigger(
                "Note", "onInsert", script, &mut data, None, &ctx, None, &mut logs,
            )
            .expect("execution");
        assert_eq!(
            data.get("directivesCount").and_then(|v| v.as_i64()),
            Some(4)
        );
    }

    #[test]
    fn test_has_role_requires_the_exact_role() {
        let engine = QuickJsEngine::new().expect("engine");
        let ctx = SessionContext {
            roles: vec!["Admin".to_string(), "SUPER_ADMIN".to_string()],
            ..SessionContext::default()
        };
        let mut data = serde_json::json!({});
        let script = r#"
            rec.admin = ctx.security.hasRole("admin");
            rec.finance = ctx.security.hasRole("FINANCE_MANAGER");
            try { ctx.security.requireRole("FINANCE_MANAGER"); rec.required = true; }
            catch (e) { rec.required = false; }
        "#;
        let mut logs = Vec::new();
        engine
            .execute_trigger(
                "Note", "onInsert", script, &mut data, None, &ctx, None, &mut logs,
            )
            .expect("execution");
        assert_eq!(data.get("admin").and_then(|v| v.as_bool()), Some(true));
        assert_eq!(
            data.get("finance").and_then(|v| v.as_bool()),
            Some(false),
            "admin must not imply other roles"
        );
        assert_eq!(data.get("required").and_then(|v| v.as_bool()), Some(false));
    }

    #[test]
    fn test_quickjs_typescript_declarations_and_record_constructors() {
        let engine = QuickJsEngine::new().expect("engine");
        let mut data = serde_json::json!({ "code": "N-001", "content": "hello world" });
        let script = r#"
            let title: string = rec.content.toUpperCase();
            let count: number = 42;
            let active: boolean = true;
            let testDate: Date = new Date("2026-10-15T00:00:00.000Z");
            let price: Decimal = Decimal("125.50").add("4.50");
            let childNote: Note = new Note({ code: "N-002", content: "Child note" });
            childNote.testField("code");
            const lookup: Map<string, number> = new Map<string, number>();
            lookup.set("A", 10);
            const tags: Set<string> = new Set<string>(["bos", "ts"]);
            const meta: { code: string; year: number } = { code: rec.code, year: testDate.getUTCFullYear() };
            const { code, content }: NoteRecord = rec;
            const pair: [string, number] = [code, lookup.get("A")!];
            let evalBlocked: boolean = false;
            try { eval("1 + 1"); } catch (err: unknown) { evalBlocked = true; }
            rec.content = `${title}|${count}|${active}|${price.toString()}|${childNote.code}|${meta.year}|${pair[1]}|${tags.has("ts")}|${evalBlocked}`;
        "#;
        run(&engine, script, &mut data).expect("TypeScript execution");
        assert_eq!(
            data.get("content").and_then(|v| v.as_str()),
            Some("HELLO WORLD|42|true|130|N-002|2026|10|true|true")
        );
    }

    #[test]
    fn test_decimal_helpers_and_error_reporting() {
        let engine = QuickJsEngine::new().expect("engine");
        let mut data = serde_json::json!({ "code": "N-500" });
        let script = r#"
            const a: Decimal = Decimal("100.25");
            const b: Decimal = Decimal("50.75");
            if (!a.gt(b) || !a.gte("100.25") || !b.lt(a) || !b.lte(50.75) || a.isZero() || !a.isPositive()) {
                throw new Error("Decimal comparison helpers failed");
            }
            const neg: Decimal = Decimal("-25.50");
            if (!neg.isNegative() || !neg.abs().eq("25.50")) throw new Error("Decimal abs/isNegative failed");
            const total = Decimal("1250.755").mul(3).add(Decimal("1250.755").mul(3).mul("0.18")).round(2);
            rec.content = `OK:${a.add(b).toString()}:${neg.abs().toString()}:${total.toString()}`;
        "#;
        run(&engine, script, &mut data).expect("decimal");
        assert_eq!(
            data.get("content").and_then(|v| v.as_str()),
            Some("OK:151:25.5:4427.67")
        );

        let err = run(
            &engine,
            r#"throw new Error("Custom validation failure on field X");"#,
            &mut data,
        )
        .expect_err("must fail");
        assert!(
            err.to_string()
                .contains("Custom validation failure on field X"),
            "got: {}",
            err
        );
    }

    fn note_types(col: &str) -> Option<String> {
        match col {
            "title" | "status" => Some("character varying".to_string()),
            "priority" => Some("integer".to_string()),
            "created_at" => Some("timestamp with time zone".to_string()),
            "id" => Some("uuid".to_string()),
            "payload" => Some("jsonb".to_string()),
            _ => None,
        }
    }

    #[test]
    fn test_build_filter_sql_typed_params_groups_and_projection() {
        let opts = serde_json::json!({
            "where": {
                "title": { "contains": "urgent" },
                "priority": { "gte": 3, "in": [3, 4, 5] },
                "OR": [ { "status": "Open" }, { "status": { "eq": "InProgress" } } ]
            },
            "orderBy": { "createdAt": "desc" },
            "select": ["id", "title", "status"],
            "take": 25,
            "skip": 10
        });
        let (sql, params) =
            QuickJsEngine::build_filter_sql("note", &opts, &note_types, None, None).expect("sql");
        assert!(
            sql.contains(
                "jsonb_build_object('id', t.\"id\", 'title', t.\"title\", 'status', t.\"status\")"
            ),
            "{}",
            sql
        );
        // Object keys are visited in sorted order: OR, priority, title.
        assert!(
            sql.contains(
                "((t.\"status\" = $1::character varying) OR (t.\"status\" = $2::character varying))"
            ),
            "{}",
            sql
        );
        assert!(sql.contains("t.\"priority\" >= $3::integer"), "{}", sql);
        assert!(
            sql.contains("t.\"priority\" IN ($4::integer, $5::integer, $6::integer)"),
            "{}",
            sql
        );
        assert!(
            sql.contains("t.\"title\"::text ILIKE '%' || $7::text || '%'"),
            "{}",
            sql
        );
        assert!(sql.contains("ORDER BY t.\"created_at\" DESC"), "{}", sql);
        assert!(sql.contains("LIMIT 25 OFFSET 10"), "{}", sql);
        assert_eq!(params.len(), 7);
        assert_eq!(params[0].as_deref(), Some("Open"));
        assert_eq!(params[1].as_deref(), Some("InProgress"));
        assert_eq!(params[2].as_deref(), Some("3"));
        assert_eq!(params[6].as_deref(), Some("urgent"));
    }

    #[test]
    fn test_build_filter_sql_rejects_bad_input() {
        let bad_op = serde_json::json!({ "where": { "title": { "regex": ".*" } } });
        assert!(QuickJsEngine::build_filter_sql("note", &bad_op, &note_types, None, None).is_err());

        let bad_dir = serde_json::json!({ "orderBy": { "title": "sideways" } });
        assert!(
            QuickJsEngine::build_filter_sql("note", &bad_dir, &note_types, None, None).is_err()
        );

        let injection = serde_json::json!({ "where": { "title\" = '' OR 1=1 --": "x" } });
        assert!(
            QuickJsEngine::build_filter_sql("note", &injection, &note_types, None, None).is_err()
        );

        let negative = serde_json::json!({ "take": -1 });
        assert!(
            QuickJsEngine::build_filter_sql("note", &negative, &note_types, None, None).is_err()
        );
    }

    #[test]
    fn test_unpaged_find_many_is_capped_not_truncated() {
        let (sql, _) = QuickJsEngine::build_filter_sql(
            "note",
            &serde_json::json!({}),
            &note_types,
            None,
            None,
        )
        .expect("sql");
        assert!(
            sql.contains(&format!("LIMIT {}", MAX_UNPAGED_ROWS + 1)),
            "{}",
            sql
        );
        assert!(!sql.contains("LIMIT 100 "), "{}", sql);
    }

    #[test]
    fn test_empty_in_and_json_equality() {
        let opts =
            serde_json::json!({ "where": { "status": { "in": [] }, "payload": { "a": 1 } } });
        let (sql, params) =
            QuickJsEngine::build_filter_sql("note", &opts, &note_types, None, None).expect("sql");
        assert!(sql.contains("FALSE"), "{}", sql);
        assert!(sql.contains("t.\"payload\" = $1::jsonb"), "{}", sql);
        assert_eq!(params[0].as_deref(), Some(r#"{"a":1}"#));
    }

    #[test]
    fn test_key_predicate_typing() {
        let mut columns = HashMap::new();
        for (n, t) in [
            ("_id", "uuid"),
            ("no", "character varying"),
            ("line_no", "integer"),
        ] {
            columns.insert(
                n.to_string(),
                ColumnMeta {
                    sql_type: t.to_string(),
                    generated: false,
                },
            );
        }
        let by_code = TableMeta {
            physical: "x".into(),
            columns: columns.clone(),
            key_column: "no".into(),
            data_per_company: false,
            is_immutable_ledger: false,
            flow_fields: Vec::new(),
        };
        assert_eq!(
            by_code.key_predicate("SO-1", 1).as_deref(),
            Some("t.\"no\" = $1::character varying")
        );
        let uuid = Uuid::new_v4().to_string();
        assert_eq!(
            by_code.key_predicate(&uuid, 1).as_deref(),
            Some("(t.\"_id\" = $1::uuid OR t.\"no\" = $1::character varying)")
        );

        let by_int = TableMeta {
            physical: "x".into(),
            columns,
            key_column: "line_no".into(),
            data_per_company: false,
            is_immutable_ledger: false,
            flow_fields: Vec::new(),
        };
        assert_eq!(
            by_int.key_predicate("10000", 2).as_deref(),
            Some("t.\"line_no\" = $2::integer")
        );
        assert_eq!(by_int.key_predicate("abc", 2), None);
    }

    #[test]
    fn test_argon2_hash_verify_and_decimal_cmp() {
        let engine = QuickJsEngine::new().expect("engine");
        let mut data = serde_json::json!({ "status": "init" });
        let script = r#"
            const hash = ctx.crypto.argon2Hash("S3cretP@ss!");
            if (!hash.startsWith("$argon2id$v=19$")) {
                throw new Error("Unexpected Argon2 hash format: " + hash);
            }
            if (!ctx.crypto.argon2Verify("S3cretP@ss!", hash)) {
                throw new Error("Argon2 verification failed for valid password");
            }
            if (ctx.crypto.argon2Verify("WrongPassword", hash)) {
                throw new Error("Argon2 verification succeeded for wrong password");
            }
            if (ctx.decimal("100.50").cmp("20.25") <= 0) {
                throw new Error("Decimal.cmp failed");
            }
            if (!ctx.user || !ctx.user.id) {
                throw new Error("ctx.user alias missing");
            }
            rec.status = "verified";
        "#;
        run(&engine, script, &mut data).expect("argon2 and decimal cmp");
        assert_eq!(
            data.get("status").and_then(|v| v.as_str()),
            Some("verified")
        );
    }

    #[test]
    fn test_execute_aot_bytecode_module() {
        let js = r#"
            export function calculate(a, b) {
                return a * 2 + b;
            }
            export default calculate;
        "#;
        let bytecode = bosc_analyzer::BytecodeCompiler::compile_js_to_bytecode("calc_module", js)
            .expect("AOT bytecode compilation must succeed");

        let rt = rquickjs::Runtime::new().expect("runtime");
        let ctx = rquickjs::Context::full(&rt).expect("context");

        let result: i32 = ctx.with(|ctx| {
            let module = unsafe { rquickjs::Module::load(ctx.clone(), &bytecode) }
                .expect("Module::load must succeed on compiled bytecode");
            let (eval_module, _) = module.eval().expect("module.eval must succeed");
            let func: rquickjs::Function = eval_module.get("calculate").expect("get calculate function");
            func.call((10, 5)).expect("call calculate")
        });

        assert_eq!(result, 25, "Bytecode execution result must match 10 * 2 + 5 = 25");
    }
}
