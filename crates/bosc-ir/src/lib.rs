use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelIR {
    pub package_name: String,
    pub tables: Vec<TableIR>,
    #[serde(default)]
    pub table_logics: Vec<TableLogicIR>,
    #[serde(default)]
    pub table_extensions: Vec<TableExtensionIR>,
    #[serde(default)]
    pub table_logic_extensions: Vec<TableLogicExtensionIR>,
    pub state_machines: Vec<StateMachineIR>,
    #[serde(default)]
    pub state_machine_extensions: Vec<StateMachineExtensionIR>,
    pub codeunits: Vec<CodeunitIR>,
    pub interfaces: Vec<InterfaceIR>,
    pub enums: Vec<EnumIR>,
    #[serde(default)]
    pub enum_extensions: Vec<EnumExtensionIR>,
    #[serde(default)]
    pub permission_sets: Vec<PermissionSetIR>,
    #[serde(default)]
    pub views: Vec<ViewIR>,
    #[serde(default)]
    pub queries: Vec<QueryIR>,
    #[serde(default)]
    pub pages: Vec<PageIR>,
    #[serde(default)]
    pub page_extensions: Vec<PageExtensionIR>,
    #[serde(default)]
    pub menus: Vec<MenuIR>,
    #[serde(default)]
    pub reports: Vec<ReportIR>,
    #[serde(default)]
    pub custom_metadata: Vec<CustomMetadataIR>,
    #[serde(default)]
    pub diagnostics: Vec<CompilerDiagnosticIR>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompilerDiagnosticIR {
    pub severity: String, // "error" | "warning" | "info"
    pub code: String,
    pub message: String,
    pub line: usize,
    pub column: usize,
    pub end_line: usize,
    pub end_column: usize,
    #[serde(default)]
    pub hint: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum StorageStrategyIR {
    #[serde(rename = "shared_rls")]
    #[default]
    SharedRLS,
    #[serde(rename = "partitioned")]
    Partitioned,
    #[serde(rename = "schema_per_company")]
    SchemaPerCompany,
    #[serde(rename = "table_per_company")]
    TablePerCompany,
}

impl StorageStrategyIR {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::SharedRLS => "shared_rls",
            Self::Partitioned => "partitioned",
            Self::SchemaPerCompany => "schema_per_company",
            Self::TablePerCompany => "table_per_company",
        }
    }

    pub fn from_str_loose(s: &str) -> Self {
        match s.to_lowercase().replace('-', "_").as_str() {
            "partitioned" | "partition" => Self::Partitioned,
            "schema_per_company" | "schemapercompany" | "schema" => Self::SchemaPerCompany,
            "table_per_company" | "tablepercompany" | "nav" | "bc" => Self::TablePerCompany,
            _ => Self::SharedRLS,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TableTypeIR {
    Normal,
    Document,
    Ledger,
    Temporal,
    Temporary,
    Virtual,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FieldGroupIR {
    pub name: String,
    pub fields: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TableIR {
    pub name: String,
    pub sql_table_name: String,
    pub caption: String,
    pub table_type: TableTypeIR,
    pub data_per_company: bool,
    pub columns: Vec<ColumnIR>,
    pub primary_key: Vec<String>,
    pub indexes: Vec<IndexIR>,
    pub flow_fields: Vec<FlowFieldIR>,
    #[serde(default)]
    pub field_groups: Vec<FieldGroupIR>,
    pub triggers: Vec<TriggerIR>,
    #[serde(default)]
    pub check_constraints: Vec<CheckConstraintIR>,
    #[serde(default)]
    pub on_delete: Option<String>,
    /// Previous SQL name of this table; schema sync renames it in place instead of recreating it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub renamed_from: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CheckConstraintIR {
    pub name: String,
    pub condition: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ColumnIR {
    pub name: String,
    pub sql_name: String,
    pub sql_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_type: Option<String>,
    pub is_nullable: bool,
    pub is_unique: bool,
    pub default_val: Option<String>,
    pub caption: String,
    pub foreign_key: Option<ForeignKeyIR>,
    #[serde(default)]
    pub check_constraint: Option<String>,
    #[serde(default)]
    pub formula: Option<String>,
    /// Previous SQL name of this column; schema sync renames it in place so its data is kept.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub renamed_from: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ForeignKeyIR {
    pub target_table: String,
    pub target_column: String,
    /// One of `RESTRICT`, `CASCADE`, `SET NULL`, `NO ACTION` (see [`ForeignKeyIR::normalize_on_delete`]).
    pub on_delete: String,
}

impl ForeignKeyIR {
    pub const ON_DELETE_ACTIONS: [&'static str; 4] = ["RESTRICT", "CASCADE", "SET NULL", "NO ACTION"];

    /// Map an authored `onDelete` value (`restrict`, `cascade`, `setNull`, `set null`, `noAction`, ...)
    /// to its SQL referential action. Returns `None` for anything else.
    pub fn normalize_on_delete(raw: &str) -> Option<&'static str> {
        let key: String = raw
            .chars()
            .filter(|c| c.is_ascii_alphanumeric())
            .collect::<String>()
            .to_ascii_lowercase();
        match key.as_str() {
            "restrict" => Some("RESTRICT"),
            "cascade" => Some("CASCADE"),
            "setnull" => Some("SET NULL"),
            "noaction" => Some("NO ACTION"),
            _ => None,
        }
    }
}

/// Maps physical SQL column types (e.g. PostgreSQL `VARCHAR(50)`, `TIMESTAMPTZ`, `INT4`, `NUMERIC(18,2)`)
/// to their canonical BOS-Script data types (`String(50)`, `DateTime`, `Integer`, `Decimal(18, 2)`).
pub fn sql_type_to_bos_type(sql_type: &str) -> String {
    let raw = sql_type.trim();
    let compact: String = raw.split_whitespace().collect();
    let upper = compact.to_ascii_uppercase();

    if upper.is_empty() {
        return "String".to_string();
    }
    if upper == "UUID" {
        return "UUID".to_string();
    }
    if upper == "BOOLEAN" || upper == "BOOL" {
        return "Boolean".to_string();
    }
    if upper == "INTEGER" || upper == "INT" || upper == "INT4" || upper == "SMALLINT" || upper == "INT2" || upper == "SERIAL" {
        return "Integer".to_string();
    }
    if upper == "BIGINT" || upper == "INT8" || upper == "BIGSERIAL" {
        return "BigInt".to_string();
    }
    if upper == "DATE" {
        return "Date".to_string();
    }
    if upper.starts_with("TIMESTAMP") || upper == "TIMESTAMPTZ" {
        return "DateTime".to_string();
    }
    if upper == "JSON" || upper == "JSONB" {
        return "Json".to_string();
    }
    if upper == "BYTEA" || upper == "BLOB" {
        return "Blob".to_string();
    }
    if upper == "TEXT" || upper == "VARCHAR" || upper == "CHARACTERVARYING" || upper == "BPCHAR" {
        return "String".to_string();
    }

    // Check VARCHAR(n), CHAR(n), CHARACTERVARYING(n)
    if let Some(rest) = upper.strip_prefix("VARCHAR(").or_else(|| upper.strip_prefix("CHARACTERVARYING(")).or_else(|| upper.strip_prefix("CHAR(")) {
        if let Some(inner) = rest.strip_suffix(')') {
            if let Ok(n) = inner.parse::<usize>() {
                return format!("String({})", n);
            }
        }
    }

    // Check NUMERIC(p, s), DECIMAL(p, s)
    if let Some(rest) = upper.strip_prefix("NUMERIC(").or_else(|| upper.strip_prefix("DECIMAL(")) {
        if let Some(inner) = rest.strip_suffix(')') {
            let parts: Vec<&str> = inner.split(',').map(|s| s.trim()).collect();
            if parts.len() == 2 {
                if let (Ok(p), Ok(s)) = (parts[0].parse::<usize>(), parts[1].parse::<usize>()) {
                    return format!("Decimal({}, {})", p, s);
                }
            } else if parts.len() == 1 {
                if let Ok(p) = parts[0].parse::<usize>() {
                    return format!("Decimal({}, 2)", p);
                }
            }
        }
    }
    if upper == "NUMERIC" || upper == "DECIMAL" || upper == "FLOAT" || upper == "REAL" || upper == "DOUBLEPRECISION" {
        return "Decimal(18, 2)".to_string();
    }

    // If already a BOS type (e.g. String(50), Decimal(18, 2), Enum(...)), return raw
    raw.to_string()
}


#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IndexIR {
    pub name: String,
    pub columns: Vec<String>,
    pub is_unique: bool,
    #[serde(default)]
    pub include_columns: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FlowFieldFuncIR {
    Sum,
    Average,
    Count,
    Min,
    Max,
    Lookup,
    Exist,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FlowFieldIR {
    pub name: String,
    pub func: FlowFieldFuncIR,
    pub target_table: String,
    pub target_column: Option<String>,
    pub sql_subquery: String,
    pub drill_down_page: Option<String>,
    /// Child-table columns compared with `=` in the FlowField filter. The SIFT index leads with these.
    #[serde(default)]
    pub index_columns: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TriggerIR {
    pub name: String,
    #[serde(default)]
    pub params: Vec<String>,
    #[serde(default)]
    pub actions: Vec<ActionIR>,
    #[serde(default)]
    pub script: Option<String>,
}

// ----------------------------------------------------
// STATE MACHINE IR
// ----------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StateMachineIR {
    pub name: String,
    pub target_table: String,
    pub status_field: String,
    pub initial_state: String,
    pub states: Vec<StateIR>,
    pub transitions: Vec<TransitionIR>,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StateIR {
    pub name: String,
    pub caption: String,
    pub badge: String,
    pub timeout_hours: Option<u32>,
    pub escalate_to: Option<String>,
    #[serde(default = "default_true")]
    pub allow_modify: bool,
    #[serde(default = "default_true")]
    pub allow_delete: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApprovalTierIR {
    pub name: String,
    pub condition: String,
    pub roles: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TransitionIR {
    pub name: String,
    pub caption: String,
    pub icon: String,
    pub from_states: Vec<String>,
    pub to_state: String,
    pub required_roles: Vec<String>,
    pub params: Vec<ColumnIR>,
    #[serde(default)]
    pub guard_script: Option<String>,
    #[serde(default)]
    pub on_transition_script: Option<String>,
    #[serde(default)]
    pub approval_tiers: Vec<ApprovalTierIR>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StateMachineExtensionIR {
    pub name: String,
    pub package_name: String,
    pub target_state_machine: String,
    #[serde(default)]
    pub states: Vec<StateIR>,
    #[serde(default)]
    pub transitions: Vec<TransitionIR>,
}

// ----------------------------------------------------
// PACKAGE & MODULE ECOSYSTEM IR
// ----------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PackageManifestIR {
    pub package_id: String,
    pub name: String,
    pub version: String,
    pub publisher: String,
    #[serde(default)]
    pub dependencies: Vec<String>,
    #[serde(default)]
    pub optional_features: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PackageBundleIR {
    pub manifest: PackageManifestIR,
    #[serde(default)]
    pub objects: Vec<serde_json::Value>,
    #[serde(default)]
    pub checksum: String,
}

pub const BCBOS_MAGIC: [u8; 4] = [0xBC, 0x42, 0x4F, 0x53]; // "BCBOS"
pub const BCBOS_VERSION: u32 = 1;

/// Ahead-of-Time Compiled Bytecode Artifact (.qbc) with Magic Header and CRC32 Verification
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BytecodeArtifactIR {
    pub name: String,
    pub object_type: String, // "codeunit" | "tablelogic" | "statemachine" | "bundle"
    pub magic: [u8; 4],
    pub version: u32,
    pub crc32: u32,
    pub source_hash: String,
    pub bytecode: Vec<u8>,
}

impl BytecodeArtifactIR {
    pub fn new(name: impl Into<String>, object_type: impl Into<String>, bytecode: Vec<u8>, source_hash: impl Into<String>) -> Self {
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(&bytecode);
        let crc32 = hasher.finalize();

        Self {
            name: name.into(),
            object_type: object_type.into(),
            magic: BCBOS_MAGIC,
            version: BCBOS_VERSION,
            crc32,
            source_hash: source_hash.into(),
            bytecode,
        }
    }

    /// Serialize into binary envelope format: [MAGIC: 4B][VERSION: 4B LE][CRC32: 4B LE][PAYLOAD_LEN: 4B LE][BYTECODE: N bytes]
    pub fn to_binary_envelope(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(16 + self.bytecode.len());
        out.extend_from_slice(&self.magic);
        out.extend_from_slice(&self.version.to_le_bytes());
        out.extend_from_slice(&self.crc32.to_le_bytes());
        out.extend_from_slice(&(self.bytecode.len() as u32).to_le_bytes());
        out.extend_from_slice(&self.bytecode);
        out
    }

    /// Parse and verify from binary envelope format
    pub fn from_binary_envelope(name: impl Into<String>, object_type: impl Into<String>, source_hash: impl Into<String>, data: &[u8]) -> Result<Self, String> {
        if data.len() < 16 {
            return Err("Bytecode binary payload is too short (< 16 bytes)".to_string());
        }
        let magic = [data[0], data[1], data[2], data[3]];
        if magic != BCBOS_MAGIC {
            return Err(format!("Invalid bytecode magic header: expected {:?}, got {:?}", BCBOS_MAGIC, magic));
        }
        let version = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
        let expected_crc = u32::from_le_bytes([data[8], data[9], data[10], data[11]]);
        let payload_len = u32::from_le_bytes([data[12], data[13], data[14], data[15]]) as usize;

        if data.len() - 16 < payload_len {
            return Err(format!("Bytecode payload truncated: expected {} bytes, found {}", payload_len, data.len() - 16));
        }

        let bytecode = data[16..16 + payload_len].to_vec();
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(&bytecode);
        let actual_crc = hasher.finalize();

        if actual_crc != expected_crc {
            return Err(format!("Bytecode CRC32 mismatch: expected 0x{:08x}, computed 0x{:08x}", expected_crc, actual_crc));
        }

        Ok(Self {
            name: name.into(),
            object_type: object_type.into(),
            magic,
            version,
            crc32: actual_crc,
            source_hash: source_hash.into(),
            bytecode,
        })
    }

    pub fn verify_integrity(&self) -> bool {
        if self.magic != BCBOS_MAGIC {
            return false;
        }
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(&self.bytecode);
        hasher.finalize() == self.crc32
    }
}

/// Ahead-of-Time Prepared SQL Query for Instant Database Pipelining
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AotPreparedStatementIR {
    pub name: String,
    pub table: String,
    pub kind: String, // "get_by_id" | "exists" | "insert" | "update_optimistic" | "delete" | "sift_sum"
    pub sql: String,
    pub param_names: Vec<String>,
}

/// Complete AOT Package Artifacts Suite
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PackageArtifactsIR {
    pub manifest: PackageManifestIR,
    pub bytecodes: Vec<BytecodeArtifactIR>,
    pub prepared_queries: Vec<AotPreparedStatementIR>,
    pub tables_ddl: String,
    pub views_ddl: String,
    pub merkle_root: String,
    pub signature: Option<String>,
    pub public_key: Option<String>,
}

impl PackageArtifactsIR {
    /// Compute cryptographic Merkle root of all bytecodes, queries, and DDL
    pub fn compute_merkle_root(&self) -> String {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(self.manifest.package_id.as_bytes());
        hasher.update(self.manifest.version.as_bytes());
        for b in &self.bytecodes {
            hasher.update(b.name.as_bytes());
            hasher.update(&b.crc32.to_le_bytes());
            hasher.update(&b.bytecode);
        }
        for q in &self.prepared_queries {
            hasher.update(q.name.as_bytes());
            hasher.update(q.sql.as_bytes());
        }
        hasher.update(self.tables_ddl.as_bytes());
        hasher.update(self.views_ddl.as_bytes());
        format!("{:064x}", hasher.finalize())
    }
}

// ----------------------------------------------------
// TABLE EXTENSION IR (Non-invasive Augmentation)
// ----------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TableExtensionIR {
    pub name: String,
    pub package_name: String,
    pub target_table: String,
    #[serde(default)]
    pub added_columns: Vec<ColumnIR>,
    #[serde(default)]
    pub added_indexes: Vec<IndexIR>,
}

// ----------------------------------------------------
// TABLE LOGIC & EXTENSION IR (Separated Business Logic)
// ----------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FieldValidationIR {
    pub field_name: String,
    pub param_name: String,
    pub old_param_name: Option<String>,
    pub script: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TableLogicIR {
    pub name: String,
    pub package_name: String,
    pub target_table: String,
    pub sql_table_name: String,
    #[serde(default)]
    pub field_validations: Vec<FieldValidationIR>,
    #[serde(default)]
    pub triggers: Vec<TriggerIR>,
    #[serde(default)]
    pub methods: Vec<MethodIR>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TableLogicExtensionIR {
    pub name: String,
    pub package_name: String,
    pub target_table: String,
    pub sql_table_name: String,
    #[serde(default)]
    pub field_validations: Vec<FieldValidationIR>,
    #[serde(default)]
    pub triggers: Vec<TriggerIR>,
    #[serde(default)]
    pub methods: Vec<MethodIR>,
}

// ----------------------------------------------------
// CODEUNIT & BUSINESS LOGIC RUNTIME IR
// ----------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TriggerEventIR {
    OnValidate(String),
    OnBeforeInsert,
    OnAfterInsert,
    OnBeforeModify,
    OnAfterModify,
    OnBeforeDelete,
    OnAfterDelete,
    OnDraftSave,
    OnDraftActivate,
    OnDraftDiscard,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SourceLocation {
    pub file: String,
    pub line: u32,
    pub column: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ActionIR {
    ValidateRequired {
        field: String,
        #[serde(default)]
        location: Option<SourceLocation>,
    },
    Assert {
        condition: String,
        message: String,
        #[serde(default)]
        location: Option<SourceLocation>,
    },
    SetField {
        field: String,
        value_expr: String,
        #[serde(default)]
        location: Option<SourceLocation>,
    },
    HashPassword {
        source_field: String,
        target_field: String,
        #[serde(default)]
        location: Option<SourceLocation>,
    },
    LogAudit {
        message: String,
        #[serde(default)]
        location: Option<SourceLocation>,
    },
    IfElse {
        condition: String,
        then_branch: Vec<ActionIR>,
        #[serde(default)]
        else_branch: Option<Vec<ActionIR>>,
        #[serde(default)]
        location: Option<SourceLocation>,
    },
    GetRecord {
        table: String,
        filter: String,
        target_buffer: String,
        #[serde(default)]
        location: Option<SourceLocation>,
    },
    ForEachRecord {
        table: String,
        filter: String,
        cursor_var: String,
        body: Vec<ActionIR>,
        #[serde(default)]
        location: Option<SourceLocation>,
    },
    CallProcedure {
        codeunit: String,
        method: String,
        #[serde(default)]
        location: Option<SourceLocation>,
    },
    /// A statement the action list keeps so it is not dropped. PostgreSQL pushdown refuses it.
    NotLowered {
        kind: String,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CodeunitIR {
    pub name: String,
    #[serde(default)]
    pub package_name: String,
    pub bound_table: Option<String>,
    pub is_exposed: bool,
    pub is_transactional: bool,
    #[serde(default)]
    pub is_pushdown: bool,
    #[serde(default)]
    pub execution_mode: Option<String>,
    #[serde(default)]
    pub max_fuel: Option<u64>,
    pub methods: Vec<MethodIR>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MethodIR {
    pub name: String,
    pub is_public: bool,
    pub is_static: bool,
    #[serde(default)]
    pub params: Vec<(String, String)>,
    pub return_type: Option<String>,
    pub trigger_event: Option<TriggerEventIR>,
    #[serde(default)]
    pub annotations: Vec<String>,
    #[serde(default)]
    pub actions: Vec<ActionIR>,
    #[serde(default)]
    pub script: Option<String>,
    #[serde(default)]
    pub is_pushdown: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InterfaceIR {
    pub name: String,
    pub methods: Vec<MethodIR>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnumIR {
    pub name: String,
    pub implements_interface: Option<String>,
    pub members: Vec<EnumMemberIR>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnumMemberIR {
    pub name: String,
    pub bound_codeunit: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnumExtensionIR {
    pub name: String,
    pub package_name: String,
    pub target_enum: String,
    pub members: Vec<EnumMemberIR>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JobQueueItemIR {
    pub job_id: uuid::Uuid,
    pub job_type: String,
    pub codeunit_name: String,
    pub method_name: String,
    pub payload: serde_json::Value,
    pub status: String, // 'ready', 'running', 'completed', 'failed'
    pub chunk_size: usize,
    pub attempts: i32,
    pub max_attempts: i32,
    pub error_message: Option<String>,
}

// ----------------------------------------------------
// PERMISSION SET IR
// ----------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PermissionSetIR {
    pub name: String,
    pub caption: Option<String>,
    pub permissions: Vec<TablePermissionIR>,
    pub execute: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TablePermissionIR {
    pub table_name: String,
    pub sql_table_name: String,
    pub can_read: bool,
    pub can_insert: bool,
    pub can_modify: bool,
    pub can_delete: bool,
    pub row_filter: Option<String>,
}

// ----------------------------------------------------
// VIEW IR
// ----------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ViewIR {
    pub name: String,
    pub sql_view_name: String,
    pub is_materialized: bool,
    pub refresh_interval: Option<String>,
    pub select_fields: Vec<ViewSelectFieldIR>,
    pub source_table: String,
    pub joins: Vec<ViewJoinIR>,
    pub where_clause: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ViewSelectFieldIR {
    pub expr: String,
    pub alias: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ViewJoinIR {
    pub join_type: String, // "LEFT" or "INNER"
    pub target_table: String,
    pub alias: Option<String>,
    pub condition: String,
}

// ----------------------------------------------------
// QUERY IR
// ----------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QueryIR {
    pub name: String,
    pub caption: Option<String>,
    pub elements: Vec<DataItemIR>,
    pub group_by: Vec<String>,
    pub having: Option<String>,
    pub order_by: Vec<(String, bool)>, // (column, ascending)
    pub sql_query: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DataItemIR {
    pub name: String,
    pub table_name: String,
    pub columns: Vec<QueryColumnIR>,
    pub filter: Option<String>,
    pub join_type: Option<String>,
    pub join_condition: Option<String>,
    pub data_items: Vec<DataItemIR>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QueryColumnIR {
    pub name: String,
    pub source_field: String,
    pub aggregation: Option<String>,
}

// ----------------------------------------------------
// PAGE, PAGE EXTENSION, REPORT & CUSTOM METADATA IR
// ----------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PageViewIR {
    pub name: String,
    #[serde(default)]
    pub caption: Option<String>,
    #[serde(default)]
    pub filter: Option<String>,
    #[serde(default)]
    pub sort: Option<String>,
    #[serde(default)]
    pub is_default: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PageIR {
    pub name: String,
    pub caption: Option<String>,
    pub page_type: String,
    pub source_table: Option<String>,
    #[serde(default)]
    pub source_kind: Option<String>,
    #[serde(default)]
    pub editable: Option<bool>,
    #[serde(default)]
    pub insert_allowed: Option<bool>,
    #[serde(default)]
    pub modify_allowed: Option<bool>,
    #[serde(default)]
    pub delete_allowed: Option<bool>,
    #[serde(default)]
    pub filters: Vec<String>,
    #[serde(default)]
    pub views: Vec<PageViewIR>,
    pub controls: Vec<PageControlIR>,
    pub actions: Vec<PageActionIR>,
    pub triggers: Vec<TriggerIR>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PageExtensionIR {
    pub name: String,
    pub target_page: String,
    pub controls: Vec<PageControlIR>,
    pub actions: Vec<PageActionIR>,
    pub triggers: Vec<TriggerIR>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PageControlIR {
    pub kind: String,
    pub name: String,
    pub source_expr: Option<String>,
    pub caption: Option<String>,
    #[serde(default)]
    pub properties: std::collections::BTreeMap<String, String>,
    pub children: Vec<PageControlIR>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PageActionIR {
    pub name: String,
    pub caption: Option<String>,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub primary: bool,
    #[serde(default)]
    pub shortcut: Option<String>,
    #[serde(default)]
    pub confirm: Option<String>,
    #[serde(default)]
    pub show_when: Option<String>,
    #[serde(default)]
    pub enable_when: Option<String>,
    #[serde(default)]
    pub run_codeunit: Option<String>,
    #[serde(default)]
    pub open_page: Option<String>,
    #[serde(default)]
    pub selection: Option<String>,
    pub script: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResolvedFieldMetaIR {
    pub name: String,
    #[serde(default)]
    pub caption: Option<String>,
    pub data_type: String,
    #[serde(default)]
    pub not_null: bool,
    #[serde(default)]
    pub is_primary_key: bool,
    #[serde(default)]
    pub default_value: Option<String>,
    #[serde(default)]
    pub relation_table: Option<String>,
    #[serde(default)]
    pub relation_field: Option<String>,
    #[serde(default)]
    pub is_flow_field: bool,
    #[serde(default)]
    pub flow_field_expr: Option<String>,
    #[serde(default = "default_true")]
    pub editable: bool,
    #[serde(default = "default_true")]
    pub visible: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResolvedPageIR {
    pub page: PageIR,
    #[serde(default)]
    pub resolved_fields: std::collections::BTreeMap<String, ResolvedFieldMetaIR>,
    #[serde(default)]
    pub state_machine: Option<String>,
    #[serde(default)]
    pub initial_state: Option<String>,
    #[serde(default)]
    pub transitions: Vec<TransitionIR>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MenuItemIR {
    pub name: String,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub page: Option<String>,
    #[serde(default)]
    pub shortcut: Option<String>,
    #[serde(default)]
    pub badge_field: Option<String>,
    #[serde(default)]
    pub items: Vec<MenuItemIR>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MenuIR {
    pub name: String,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub role: Option<String>,
    #[serde(default)]
    pub items: Vec<MenuItemIR>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReportIR {
    pub name: String,
    pub caption: Option<String>,
    pub source_table: Option<String>,
    pub elements: Vec<DataItemIR>,
    pub triggers: Vec<TriggerIR>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CustomMetadataIR {
    pub name: String,
    pub caption: Option<String>,
    pub columns: Vec<ColumnIR>,
    pub records: Vec<serde_json::Value>,
}


