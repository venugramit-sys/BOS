use crate::token::Span;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Program {
    pub items: Vec<Item>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Item {
    Table(TableDef),
    TableLogic(TableLogicDef),
    TableExtension(TableExtDef),
    TableLogicExtension(TableLogicExtDef),
    StateMachine(StateMachineDef),
    StateMachineExtension(StateMachineExtDef),
    Codeunit(CodeunitDef),
    Interface(InterfaceDef),
    Enum(EnumDef),
    EnumExtension(EnumExtDef),
    PermissionSet(PermissionSetDef),
    View(ViewDef),
    Query(QueryDef),
    Page(PageDef),
    PageExtension(PageExtDef),
    Menu(MenuDef),
    Report(ReportDef),
    CustomMetadata(CustomMetadataDef),
}

// ----------------------------------------------------
// TABLE AST
// ----------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FieldGroupDef {
    pub name: String,
    pub fields: Vec<String>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TableDef {
    pub name: String,
    pub properties: Vec<Property>,
    pub fields: Vec<FieldDef>,
    pub primary_key: Vec<String>,
    pub keys: Vec<KeyDef>,
    #[serde(default)]
    pub field_groups: Vec<FieldGroupDef>,
    pub triggers: Vec<TriggerDef>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TableLogicDef {
    pub name: String,
    pub target_table: String,
    pub field_validations: Vec<FieldValidationDef>,
    pub triggers: Vec<TriggerDef>,
    pub methods: Vec<MethodDef>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TableLogicExtDef {
    pub name: String,
    pub target_table: String,
    pub field_validations: Vec<FieldValidationDef>,
    pub triggers: Vec<TriggerDef>,
    pub methods: Vec<MethodDef>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FieldValidationDef {
    pub field_name: String,
    pub param_name: String,
    pub old_param_name: Option<String>,
    pub body: Block,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TableExtDef {
    pub name: String,
    pub target_table: String,
    pub fields: Vec<FieldDef>,
    pub keys: Vec<KeyDef>,
    pub triggers: Vec<TriggerDef>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Property {
    pub name: String,
    pub value: Expr,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FieldDef {
    pub name: String,
    pub field_type: FieldType,
    pub properties: Vec<Property>,
    pub computed: Option<ComputedFormula>,
    pub validate_block: Option<Block>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FieldType {
    UUID,
    String(Option<usize>),
    Integer,
    BigInt,
    Decimal(usize, usize),
    Boolean,
    Date,
    DateTime,
    Enum(String),
    Json,
    Blob,
    FlowFilter(String),
    Custom(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KeyDef {
    pub fields: Vec<String>,
    pub unique: bool,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AggregationFunc {
    Sum,
    Average,
    Count,
    Min,
    Max,
    Lookup,
    Exist,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComputedFormula {
    pub func: AggregationFunc,
    pub target_table: String,
    pub target_field: Option<String>,
    pub filter: Expr,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TriggerDef {
    pub name: String,
    pub params: Vec<String>,
    pub body: Block,
    pub span: Span,
}

// ----------------------------------------------------
// STATE MACHINE AST
// ----------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StateMachineDef {
    pub name: String,
    pub target_table: String,
    pub status_field: String,
    pub properties: Vec<Property>,
    pub states: Vec<StateDef>,
    pub transitions: Vec<TransitionDef>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StateMachineExtDef {
    pub name: String,
    pub target_state_machine: String,
    pub states: Vec<StateDef>,
    pub transitions: Vec<TransitionDef>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StateDef {
    pub name: String,
    pub properties: Vec<Property>,
    pub timeout: Option<TimeoutDef>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TimeoutDef {
    pub duration_expr: Expr,
    pub escalate_to: Option<String>,
    pub action_name: Option<String>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TransitionDef {
    pub name: String,
    pub from_states: Vec<String>,
    pub to_state: String,
    pub properties: Vec<Property>,
    pub guard: Option<Block>,
    pub approval_matrix: Option<Vec<ApprovalTierDef>>,
    pub params: Vec<FieldDef>,
    pub on_transition: Option<Block>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApprovalTierDef {
    pub name: String,
    pub condition: Expr,
    pub roles: Vec<String>,
    pub span: Span,
}

// ----------------------------------------------------
// CODEUNIT, INTERFACE, ENUM AST
// ----------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CodeunitDef {
    pub name: String,
    pub bound_table: Option<String>,
    pub annotations: Vec<String>,
    pub properties: Vec<Property>,
    pub methods: Vec<MethodDef>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MethodDef {
    pub name: String,
    pub is_public: bool,
    pub is_static: bool,
    pub annotations: Vec<String>,
    pub params: Vec<ParamDef>,
    pub return_type: Option<String>,
    pub body: Block,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParamDef {
    pub name: String,
    pub param_type: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InterfaceDef {
    pub name: String,
    pub properties: Vec<Property>,
    pub methods: Vec<InterfaceMethodDef>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InterfaceMethodDef {
    pub name: String,
    pub params: Vec<ParamDef>,
    pub return_type: Option<String>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnumDef {
    pub name: String,
    pub implements_interface: Option<String>,
    pub extensible: bool,
    pub properties: Vec<Property>,
    pub members: Vec<EnumMemberDef>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnumExtDef {
    pub name: String,
    pub target_enum: String,
    pub members: Vec<EnumMemberDef>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnumMemberDef {
    pub name: String,
    pub bound_codeunit: Option<String>,
    pub span: Span,
}

// ----------------------------------------------------
// PERMISSIONSET AST
// ----------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PermissionSetDef {
    pub name: String,
    pub properties: Vec<Property>,
    pub permissions: Vec<TablePermissionDef>,
    pub execute: Vec<String>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TablePermissionDef {
    pub table_name: String,
    pub actions: Vec<PermissionAction>,
    pub row_filter: Option<Expr>,
    #[serde(default)]
    pub exclude_fields: Vec<String>,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PermissionAction {
    Read,
    Insert,
    Modify,
    Delete,
}

// ----------------------------------------------------
// VIEW AST
// ----------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ViewDef {
    pub name: String,
    pub properties: Vec<Property>,
    pub select_fields: Vec<ViewSelectField>,
    pub source_table: String,
    pub joins: Vec<ViewJoinDef>,
    pub where_filter: Option<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ViewSelectField {
    pub field_expr: Expr,
    pub alias: Option<String>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ViewJoinDef {
    pub join_type: JoinType,
    pub target_table: String,
    pub alias: Option<String>,
    pub condition: Expr,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JoinType {
    Inner,
    Left,
}

// ----------------------------------------------------
// QUERY AST
// ----------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QueryDef {
    pub name: String,
    pub properties: Vec<Property>,
    pub elements: Vec<DataItemDef>,
    pub group_by: Vec<String>,
    pub having: Option<Expr>,
    pub order_by: Vec<QueryOrderByDef>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DataItemDef {
    pub name: String,
    pub table_name: String,
    pub columns: Vec<QueryColumnDef>,
    pub filter: Option<Expr>,
    pub join_type: Option<JoinType>,
    pub join_condition: Option<Expr>,
    pub data_items: Vec<DataItemDef>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QueryColumnDef {
    pub name: String,
    pub source_field: String,
    pub aggregation: Option<AggregationFunc>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QueryOrderByDef {
    pub column: String,
    pub ascending: bool,
    pub span: Span,
}

// ----------------------------------------------------
// PAGE, PAGE EXTENSION, REPORT & CUSTOM METADATA AST
// ----------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PageViewDef {
    pub name: String,
    pub properties: Vec<Property>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PageDef {
    pub name: String,
    pub source_table: Option<String>,
    pub page_type: String,
    pub properties: Vec<Property>,
    #[serde(default)]
    pub views: Vec<PageViewDef>,
    pub controls: Vec<PageControlDef>,
    pub actions: Vec<PageActionDef>,
    pub triggers: Vec<TriggerDef>,
    /// `variables { name: Type; }`: page globals bound by `field(name)` when no column matches.
    #[serde(default)]
    pub variables: Vec<PageVariableDef>,
    /// Page-level `style { ... }` block.
    #[serde(default)]
    pub style: Vec<Property>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PageExtDef {
    pub name: String,
    pub target_page: String,
    pub properties: Vec<Property>,
    pub controls: Vec<PageControlDef>,
    pub actions: Vec<PageActionDef>,
    pub triggers: Vec<TriggerDef>,
    #[serde(default)]
    pub variables: Vec<PageVariableDef>,
    #[serde(default)]
    pub style: Vec<Property>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PageVariableDef {
    pub name: String,
    pub field_type: FieldType,
    #[serde(default)]
    pub properties: Vec<Property>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PageControlDef {
    pub kind: String, // "area", "group", "repeater", "part", "field"
    pub name: String,
    pub source_expr: Option<String>,
    pub properties: Vec<Property>,
    pub children: Vec<PageControlDef>,
    pub triggers: Vec<TriggerDef>,
    /// `style { background = primary; radius = md; }` block.
    #[serde(default)]
    pub style: Vec<Property>,
    /// `responsive { md = 6; hideBelow = sm; }` block.
    #[serde(default)]
    pub responsive: Vec<Property>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PageActionDef {
    /// `action`, `area`, `group`, `actionref`, `separator`, or an extension directive
    /// (`addFirst`, `addLast`, `addBefore`, `addAfter`, `modify`, `hide`, `moveBefore`, `moveAfter`).
    #[serde(default = "default_action_kind")]
    pub kind: String,
    pub name: String,
    /// Second positional argument: the referenced action of an `actionref(Name; Target)`.
    #[serde(default)]
    pub target: Option<String>,
    pub properties: Vec<Property>,
    pub on_action: Option<Block>,
    #[serde(default)]
    pub children: Vec<PageActionDef>,
    #[serde(default)]
    pub style: Vec<Property>,
    pub span: Span,
}

fn default_action_kind() -> String {
    "action".to_string()
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MenuItemDef {
    pub name: String,
    pub properties: Vec<Property>,
    pub items: Vec<MenuItemDef>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MenuDef {
    pub name: String,
    pub properties: Vec<Property>,
    pub items: Vec<MenuItemDef>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReportDef {
    pub name: String,
    pub source_table: Option<String>,
    pub properties: Vec<Property>,
    pub elements: Vec<DataItemDef>,
    pub triggers: Vec<TriggerDef>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CustomMetadataDef {
    pub name: String,
    pub properties: Vec<Property>,
    pub fields: Vec<FieldDef>,
    pub records: Vec<Vec<Property>>,
    pub span: Span,
}


// ----------------------------------------------------
// STATEMENTS & EXPRESSIONS AST
// ----------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Block {
    pub statements: Vec<Stmt>,
    #[serde(default)]
    pub raw_source: Option<String>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Stmt {
    Expr(Expr),
    Return(Option<Expr>),
    If {
        condition: Expr,
        then_branch: Block,
        else_branch: Option<Block>,
    },
    Assign {
        target: Expr,
        value: Expr,
    },
    VarDecl {
        name: String,
        var_type: Option<String>,
        init: Option<Expr>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Expr {
    Ident(String),
    StringLit(String),
    IntLit(i64),
    FloatLit(f64),
    BoolLit(bool),
    NullLit,
    ArrayLit(Vec<Expr>),
    MemberAccess {
        object: Box<Expr>,
        member: String,
    },
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
    },
    Binary {
        left: Box<Expr>,
        op: BinaryOp,
        right: Box<Expr>,
    },
    Unary {
        op: UnaryOp,
        operand: Box<Expr>,
    },
    Range {
        start: Box<Expr>,
        end: Box<Expr>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BinaryOp {
    Eq,
    NotEq,
    Lt,
    Lte,
    Gt,
    Gte,
    And,
    Or,
    In,
    Add,
    Sub,
    Mul,
    Div,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum UnaryOp {
    Not,
    Neg,
}

impl std::fmt::Display for FieldType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FieldType::UUID => write!(f, "UUID"),
            FieldType::String(Some(len)) => write!(f, "String({})", len),
            FieldType::String(None) => write!(f, "String"),
            FieldType::Integer => write!(f, "Integer"),
            FieldType::BigInt => write!(f, "BigInt"),
            FieldType::Decimal(p, s) => write!(f, "Decimal({}, {})", p, s),
            FieldType::Boolean => write!(f, "Boolean"),
            FieldType::Date => write!(f, "Date"),
            FieldType::DateTime => write!(f, "DateTime"),
            FieldType::Enum(name) => write!(f, "Enum({})", name),
            FieldType::Json => write!(f, "Json"),
            FieldType::Blob => write!(f, "Blob"),
            FieldType::FlowFilter(t) => write!(f, "FlowFilter({})", t),
            FieldType::Custom(c) => write!(f, "{}", c),
        }
    }
}
