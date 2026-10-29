//! Page compilation: merge extensions, resolve every control against its source table and produce
//! the [`CompiledPageIR`] consumed by the Svelte page code generator.
//!
//! Like Dynamics NAV / Business Central, most field properties come from the table: caption, control
//! type, mandatory flag, editability, decimal places, option values, lookups and drill-downs. Anything
//! authored on the page wins; every auto-filled property is listed in `ResolvedFieldIR::auto_filled`.
//!
//! Diagnostic codes:
//! - BOS3001 unknown source table
//! - BOS3002 unknown field (control source, caption field, sub-page link field)
//! - BOS3003 invalid style value (colour, icon)
//! - BOS3004 unknown referenced object (page, codeunit, action target, extension target)
//! - BOS3005 invalid client expression
//! - BOS3006 duplicate control or action name
//! - BOS3007 extension anchor not found
//! - BOS3008 control kind not allowed on the page type / unknown kind
//! - BOS3009 directive not allowed in a customization layer
//! - BOS3010 unknown page type
//! - BOS3011 invalid property value
//! - BOS3012 unknown enum on an option field

use bosc_ir::*;
use std::collections::{BTreeMap, HashSet};

use crate::page_expr::{self, RefTarget};
use crate::page_merger::{merge_page_detailed, MergeOutcome};

// ----------------------------------------------------
// CATALOG
// ----------------------------------------------------

/// Read access to the objects a page can reference.
pub trait PageCatalog {
    /// Table by authored name or SQL name (case-insensitive).
    fn table(&self, name: &str) -> Option<&TableIR>;
    /// Enum members (base + every enum extension) by enum name.
    fn enum_members(&self, name: &str) -> Option<Vec<String>>;
    fn page(&self, name: &str) -> Option<&PageIR>;
    fn pages(&self) -> Vec<&PageIR>;
    fn has_codeunit(&self, name: &str) -> bool;
    /// `true` when the catalog holds every object of the database. When `false` (compiling a single
    /// package), unknown references are reported as warnings because they may live in another package.
    fn is_closed(&self) -> bool;
}

/// Catalog over one or more compiled models.
pub struct ModelCatalog<'a> {
    models: Vec<&'a ModelIR>,
    closed: bool,
}

impl<'a> ModelCatalog<'a> {
    pub fn new(models: Vec<&'a ModelIR>) -> Self {
        ModelCatalog { models, closed: false }
    }

    /// Marks the catalog as complete (all packages of the database are present).
    pub fn closed(mut self) -> Self {
        self.closed = true;
        self
    }
}

impl PageCatalog for ModelCatalog<'_> {
    fn table(&self, name: &str) -> Option<&TableIR> {
        self.models
            .iter()
            .flat_map(|m| m.tables.iter())
            .find(|t| t.name.eq_ignore_ascii_case(name) || t.sql_table_name.eq_ignore_ascii_case(name))
    }

    fn enum_members(&self, name: &str) -> Option<Vec<String>> {
        let base = self.models.iter().flat_map(|m| m.enums.iter()).find(|e| e.name.eq_ignore_ascii_case(name))?;
        let mut members: Vec<String> = base.members.iter().map(|m| m.name.clone()).collect();
        for ext in self.models.iter().flat_map(|m| m.enum_extensions.iter()) {
            if ext.target_enum.eq_ignore_ascii_case(name) {
                for m in &ext.members {
                    if !members.iter().any(|x| x.eq_ignore_ascii_case(&m.name)) {
                        members.push(m.name.clone());
                    }
                }
            }
        }
        Some(members)
    }

    fn page(&self, name: &str) -> Option<&PageIR> {
        self.models.iter().flat_map(|m| m.pages.iter()).find(|p| p.name.eq_ignore_ascii_case(name))
    }

    fn pages(&self) -> Vec<&PageIR> {
        self.models.iter().flat_map(|m| m.pages.iter()).collect()
    }

    fn has_codeunit(&self, name: &str) -> bool {
        self.models.iter().flat_map(|m| m.codeunits.iter()).any(|c| c.name.eq_ignore_ascii_case(name))
    }

    fn is_closed(&self) -> bool {
        self.closed
    }
}

// ----------------------------------------------------
// HELPERS
// ----------------------------------------------------

/// `fullName` -> `Full Name`, `ipAddress` -> `IP Address`, `sso_login_url` -> `SSO Login URL`.
pub fn prettify_caption(name: &str) -> String {
    const ACRONYMS: &[&str] = &["id", "url", "uuid", "ip", "sso", "mfa", "vat", "gst", "sku", "api", "otp", "sms", "pk"];
    let mut words: Vec<String> = Vec::new();
    let mut current = String::new();
    let chars: Vec<char> = name.chars().collect();
    for (i, &c) in chars.iter().enumerate() {
        if c == '_' || c == '-' || c == ' ' {
            if !current.is_empty() {
                words.push(std::mem::take(&mut current));
            }
            continue;
        }
        let boundary = c.is_ascii_uppercase()
            && i > 0
            && (chars[i - 1].is_ascii_lowercase()
                || chars[i - 1].is_ascii_digit()
                || (chars[i - 1].is_ascii_uppercase() && chars.get(i + 1).is_some_and(|n| n.is_ascii_lowercase())));
        if boundary && !current.is_empty() {
            words.push(std::mem::take(&mut current));
        }
        current.push(c);
    }
    if !current.is_empty() {
        words.push(current);
    }
    words
        .iter()
        .map(|w| {
            let lower = w.to_ascii_lowercase();
            if ACRONYMS.contains(&lower.as_str()) {
                lower.to_ascii_uppercase()
            } else {
                let mut cs = lower.chars();
                cs.next().map(|f| f.to_ascii_uppercase().to_string() + cs.as_str()).unwrap_or_default()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn snake(s: &str) -> String {
    crate::to_snake_case(s)
}

/// Parsed BOS data type.
#[derive(Debug, Clone, PartialEq)]
enum BosType {
    Uuid,
    Text(Option<u32>),
    Integer,
    BigInt,
    Decimal(u32, u32),
    Boolean,
    Date,
    DateTime,
    Time,
    Enum(String),
    Json,
    Blob,
    Other(String),
}

fn parse_bos_type(raw: &str) -> BosType {
    let t = raw.trim();
    let (head, args) = match t.find('(') {
        Some(i) => (&t[..i], t[i + 1..].trim_end_matches(')')),
        None => (t, ""),
    };
    let nums: Vec<u32> = args.split(',').filter_map(|a| a.trim().parse().ok()).collect();
    match head.trim().to_ascii_lowercase().as_str() {
        "uuid" | "guid" => BosType::Uuid,
        "string" | "code" | "varchar" | "text" => BosType::Text(nums.first().copied()),
        "integer" | "int" => BosType::Integer,
        "bigint" | "biginteger" => BosType::BigInt,
        "decimal" => BosType::Decimal(nums.first().copied().unwrap_or(18), nums.get(1).copied().unwrap_or(2)),
        "boolean" | "bool" => BosType::Boolean,
        "date" => BosType::Date,
        "datetime" => BosType::DateTime,
        "time" => BosType::Time,
        "enum" | "option" => BosType::Enum(args.trim().trim_matches('"').to_string()),
        "json" => BosType::Json,
        "blob" | "media" | "image" => BosType::Blob,
        _ => BosType::Other(t.to_string()),
    }
}

fn prop<'a>(props: &'a BTreeMap<String, String>, names: &[&str]) -> Option<&'a str> {
    names.iter().find_map(|n| props.get(*n)).map(String::as_str)
}

fn prop_bool(props: &BTreeMap<String, String>, names: &[&str]) -> Option<bool> {
    prop(props, names).and_then(|v| match v.to_ascii_lowercase().as_str() {
        "true" | "yes" => Some(true),
        "false" | "no" => Some(false),
        _ => None,
    })
}

/// Cleans a SQL default (`'Internal'`, `0`, `true`) into a display value; function defaults are skipped.
fn clean_default(sql_default: &str) -> Option<String> {
    let d = sql_default.trim();
    if d.contains('(') {
        return None;
    }
    let d = d.split("::").next().unwrap_or(d).trim();
    if d.eq_ignore_ascii_case("true") || d.eq_ignore_ascii_case("false") {
        return Some(d.to_ascii_lowercase());
    }
    Some(d.trim_matches('\'').to_string())
}

/// Per-property values that are reserved for the resolver and must not be passed through.
const CONSUMED_CONTROL_PROPS: &[&str] = &[
    "caption", "label", "tooltip", "toolTip", "instructionalText", "visible", "editable", "required", "showMandatory",
    "notBlank", "control", "extendedDatatype", "multiLine", "multiline", "decimalPlaces", "numberFormat", "blankZero",
    "lookup", "lookupPageId", "lookupPage", "drillDown", "drillDownPageId", "drillDownPage", "assistEdit", "quickEntry",
    "placeholder", "showWhen", "enableWhen", "editableWhen", "styleExpr", "subPage", "pagePart", "page",
    "updatePropagation",
];

pub struct PageCompileResult {
    pub page: CompiledPageIR,
    pub diagnostics: Vec<PageDiagnosticIR>,
}

struct Ctx<'a> {
    catalog: &'a dyn PageCatalog,
    page_name: String,
    page_type: PageType,
    table: Option<&'a TableIR>,
    /// Source table is named but could not be found (and the catalog is open): skip field checks.
    table_unresolved: bool,
    variables: Vec<PageVariableIR>,
    page_editable: bool,
    diags: Vec<PageDiagnosticIR>,
    names: HashSet<String>,
}

impl<'a> Ctx<'a> {
    fn push(&mut self, severity: PageDiagnosticSeverity, code: &str, path: Option<&str>, message: String, hint: Option<String>) {
        self.diags.push(PageDiagnosticIR {
            severity,
            code: code.to_string(),
            message,
            object: self.page_name.clone(),
            path: path.map(str::to_string),
            hint,
        });
    }

    fn error(&mut self, code: &str, path: Option<&str>, message: String, hint: Option<String>) {
        self.push(PageDiagnosticSeverity::Error, code, path, message, hint);
    }

    fn warning(&mut self, code: &str, path: Option<&str>, message: String, hint: Option<String>) {
        self.push(PageDiagnosticSeverity::Warning, code, path, message, hint);
    }

    /// Error when the catalog is complete, warning when the object may live in another package.
    fn reference(&mut self, code: &str, path: Option<&str>, message: String, hint: Option<String>) {
        let sev = if self.catalog.is_closed() { PageDiagnosticSeverity::Error } else { PageDiagnosticSeverity::Warning };
        self.push(sev, code, path, message, hint);
    }

    fn page_exists(&self, name: &str) -> bool {
        self.catalog.page(name).is_some()
    }

    /// Default list page of a table (first `List` page whose source is that table).
    fn list_page_for(&self, table: &TableIR) -> Option<String> {
        self.catalog
            .pages()
            .into_iter()
            .find(|p| {
                p.page_type_kind() == Some(PageType::List)
                    && p.source_table.as_deref().is_some_and(|t| {
                        t.eq_ignore_ascii_case(&table.name) || t.eq_ignore_ascii_case(&table.sql_table_name)
                    })
            })
            .map(|p| p.name.clone())
    }

    fn column(&self, name: &str) -> Option<&'a ColumnIR> {
        let t = self.table?;
        t.columns.iter().find(|c| c.name.eq_ignore_ascii_case(name) || c.sql_name.eq_ignore_ascii_case(name))
    }

    fn flow_field(&self, name: &str) -> Option<&'a FlowFieldIR> {
        let t = self.table?;
        t.flow_fields.iter().find(|f| f.name.eq_ignore_ascii_case(name) || snake(&f.name).eq_ignore_ascii_case(name))
    }

    fn variable(&self, name: &str) -> Option<&PageVariableIR> {
        self.variables.iter().find(|v| v.name.eq_ignore_ascii_case(name))
    }

    /// Resolves an expression reference against this page's record and variables.
    fn resolve_ref(&self, path: &[String]) -> Result<RefTarget, String> {
        let (scope, name) = match path {
            [scope, name] if scope == "rec" || scope == "page" => (Some(scope.as_str()), name.as_str()),
            [name] => (None, name.as_str()),
            _ => return Err(format!("'{}' is not a field or page variable reference", path.join("."))),
        };
        if scope != Some("page") {
            if let Some(c) = self.column(name) {
                return Ok(RefTarget::Field(c.sql_name.clone()));
            }
            if let Some(f) = self.flow_field(name) {
                return Ok(RefTarget::Field(snake(&f.name)));
            }
            if scope == Some("rec") && self.table_unresolved {
                return Ok(RefTarget::Field(snake(name)));
            }
        }
        if scope != Some("rec")
            && let Some(v) = self.variable(name)
        {
            return Ok(RefTarget::Variable(v.name.clone()));
        }
        if self.table_unresolved && scope.is_none() {
            return Ok(RefTarget::Field(snake(name)));
        }
        Err(format!("unknown field or variable '{}'", path.join(".")))
    }

    fn compile_expr(&mut self, src: &str, path: &str, what: &str) -> Option<ClientExprIR> {
        let mut resolve = |p: &[String]| self.resolve_ref(p);
        match page_expr::compile_source(src, &mut resolve) {
            Ok(e) => Some(e),
            Err(msg) => {
                self.error("BOS3005", Some(path), format!("Invalid {what} expression '{src}': {msg}"), None);
                None
            }
        }
    }

    fn check_style(&mut self, style: &Option<PageStyleIR>, path: &str) -> Option<PageStyleIR> {
        let mut s = style.clone()?;
        if let Some(icon) = &s.icon {
            match normalize_icon_name(icon) {
                Some(n) => s.icon = Some(n),
                None => {
                    self.error("BOS3003", Some(path), format!("Icon '{icon}' is not a valid icon name"), Some("Use a lucide icon name such as `truck` or `user-minus`.".into()));
                    s.icon = None;
                }
            }
        }
        (!s.is_empty()).then_some(s)
    }

    fn unique(&mut self, kind: &str, name: &str, path: &str) {
        let key = format!("{kind}:{}", name.to_ascii_lowercase());
        if !self.names.insert(key) {
            self.error("BOS3006", Some(path), format!("Duplicate {kind} name '{name}' on page '{}'", self.page_name), Some("Control and action names must be unique per page.".into()));
        }
    }
}

// ----------------------------------------------------
// FIELD RESOLUTION
// ----------------------------------------------------

fn name_hint(name: &str) -> Option<ExtendedDatatype> {
    let n = name.to_ascii_lowercase();
    if n.contains("email") {
        Some(ExtendedDatatype::Email)
    } else if n.contains("phone") || n.contains("mobile") {
        Some(ExtendedDatatype::Phone)
    } else if n.ends_with("url") || n.contains("website") {
        Some(ExtendedDatatype::Url)
    } else if n.contains("password") || n.contains("secret") {
        Some(ExtendedDatatype::Masked)
    } else if n.ends_with("color") || n.ends_with("colour") {
        Some(ExtendedDatatype::Color)
    } else {
        None
    }
}

fn control_for_type(ty: &BosType, list_like: bool) -> FieldControl {
    match ty {
        BosType::Uuid => FieldControl::ReadOnly,
        BosType::Text(Some(n)) if *n >= 500 => FieldControl::TextArea,
        BosType::Text(_) => FieldControl::Text,
        BosType::Integer | BosType::BigInt | BosType::Decimal(..) => FieldControl::Number,
        BosType::Boolean => {
            if list_like {
                FieldControl::Checkbox
            } else {
                FieldControl::Switch
            }
        }
        BosType::Date => FieldControl::Date,
        BosType::DateTime => FieldControl::DateTime,
        BosType::Time => FieldControl::Time,
        BosType::Enum(_) => FieldControl::Select,
        BosType::Json => FieldControl::Json,
        BosType::Blob => FieldControl::Image,
        BosType::Other(_) => FieldControl::Text,
    }
}

fn control_for_extended(ext: ExtendedDatatype) -> Option<FieldControl> {
    match ext {
        ExtendedDatatype::Email => Some(FieldControl::Email),
        ExtendedDatatype::Phone => Some(FieldControl::Phone),
        ExtendedDatatype::Url => Some(FieldControl::Url),
        ExtendedDatatype::Masked => Some(FieldControl::Masked),
        ExtendedDatatype::Color => Some(FieldControl::Color),
        ExtendedDatatype::Ratio | ExtendedDatatype::Barcode | ExtendedDatatype::Person => None,
    }
}

/// Parses NAV `DecimalPlaces`: `2` or `2:5`.
fn parse_decimal_places(raw: &str) -> Option<(u8, u8)> {
    let mut parts = raw.split(':').map(|p| p.trim().parse::<u8>());
    let min = parts.next()?.ok()?;
    let max = match parts.next() {
        Some(p) => p.ok()?,
        None => min,
    };
    (min <= max && max <= 10 && parts.next().is_none()).then_some((min, max))
}

struct FieldSource<'a> {
    binding: FieldBinding,
    key: String,
    data_type: String,
    column: Option<&'a ColumnIR>,
    flow: Option<&'a FlowFieldIR>,
    variable: Option<PageVariableIR>,
    table_caption: Option<String>,
}

fn find_field_source<'a>(ctx: &Ctx<'a>, source: &str) -> Option<FieldSource<'a>> {
    if let Some(c) = ctx.column(source) {
        let explicit_caption = (!c.caption.eq_ignore_ascii_case(&c.name)).then(|| c.caption.clone());
        return Some(FieldSource {
            binding: FieldBinding::Column,
            key: c.sql_name.clone(),
            data_type: c.data_type.clone().unwrap_or_else(|| sql_type_to_bos_type(&c.sql_type)),
            column: Some(c),
            flow: None,
            variable: None,
            table_caption: explicit_caption,
        });
    }
    if let Some(f) = ctx.flow_field(source) {
        let data_type = match f.func {
            FlowFieldFuncIR::Count => "Integer".to_string(),
            FlowFieldFuncIR::Exist => "Boolean".to_string(),
            _ => f
                .target_column
                .as_deref()
                .and_then(|col| {
                    let target = ctx.catalog.table(&f.target_table)?;
                    target.columns.iter().find(|c| c.name.eq_ignore_ascii_case(col) || c.sql_name.eq_ignore_ascii_case(col))
                })
                .and_then(|c| c.data_type.clone())
                .unwrap_or_else(|| "Decimal(18, 2)".to_string()),
        };
        return Some(FieldSource {
            binding: FieldBinding::FlowField,
            key: snake(&f.name),
            data_type,
            column: None,
            flow: Some(f),
            variable: None,
            table_caption: None,
        });
    }
    ctx.variable(source).map(|v| FieldSource {
        binding: FieldBinding::Variable,
        key: v.name.clone(),
        data_type: v.data_type.clone(),
        column: None,
        flow: None,
        variable: Some(v.clone()),
        table_caption: v.caption.clone(),
    })
}

/// Builds the resolved field, applying table metadata first and authored properties on top.
fn resolve_field(ctx: &mut Ctx<'_>, ctrl: &PageControlIR, path: &str) -> (Option<ResolvedFieldIR>, Option<String>) {
    let source = ctrl.source_expr.clone().unwrap_or_else(|| ctrl.name.clone());
    let Some(src) = find_field_source(ctx, &source) else {
        if ctx.table_unresolved {
            return (None, None);
        }
        let on = ctx.table.map(|t| format!("table '{}'", t.name)).unwrap_or_else(|| "this page (it has no source table)".to_string());
        let hint = Some(format!("Add '{source}' to {on}, or declare it in the page `variables {{ }}` block."));
        ctx.error("BOS3002", Some(path), format!("Field '{source}' does not exist on {on}"), hint);
        return (None, None);
    };

    let props = &ctrl.properties;
    let mut auto = Vec::new();
    let ty = parse_bos_type(&src.data_type);
    let list_like = ctx.page_type.is_list_like();

    // Caption: authored > explicit table caption > prettified name.
    let caption = match &ctrl.caption {
        Some(c) => c.clone(),
        None => {
            auto.push("caption".to_string());
            src.table_caption.clone().unwrap_or_else(|| prettify_caption(&source))
        }
    };

    // Extended datatype: authored > name heuristic (text fields only).
    let mut extended = prop(props, &["extendedDatatype"]).and_then(|raw| {
        let parsed = ExtendedDatatype::parse(raw);
        if parsed.is_none() {
            ctx.error("BOS3011", Some(path), format!("extendedDatatype '{raw}' is not one of {}", ExtendedDatatype::names().join(", ")), None);
        }
        parsed
    });
    if extended.is_none() && matches!(ty, BosType::Text(_)) {
        extended = name_hint(&source);
        if extended.is_some() {
            auto.push("extendedDatatype".to_string());
        }
    }

    // Lookup from the table relation.
    let mut lookup = None;
    if let Some(fk) = src.column.and_then(|c| c.foreign_key.as_ref())
        && prop_bool(props, &["lookup"]) != Some(false)
    {
        let target = ctx.catalog.table(&fk.target_table);
        let explicit_page = prop(props, &["lookupPageId", "lookupPage"]).map(str::to_string);
        let page = explicit_page.clone().or_else(|| target.and_then(|t| ctx.list_page_for(t)));
        if explicit_page.is_none() {
            auto.push("lookup".to_string());
        }
        let mut display_fields = vec![fk.target_column.clone()];
        if let Some(desc) = target.and_then(|t| {
            t.columns.iter().find(|c| {
                matches!(c.sql_name.as_str(), "name" | "description" | "full_name" | "title") && c.sql_name != fk.target_column
            })
        }) {
            display_fields.push(desc.sql_name.clone());
        }
        lookup = Some(LookupIR {
            table: target.map(|t| t.name.clone()).unwrap_or_else(|| fk.target_table.clone()),
            sql_table: target.map(|t| t.sql_table_name.clone()).unwrap_or_else(|| fk.target_table.clone()),
            sql_field: fk.target_column.clone(),
            page,
            display_fields,
        });
    }
    if let Some(p) = prop(props, &["lookupPageId", "lookupPage"])
        && !ctx.page_exists(p)
    {
        ctx.reference("BOS3004", Some(path), format!("Lookup page '{p}' does not exist"), None);
    }

    // Control type: authored > extended datatype > lookup > data type.
    let control = match prop(props, &["control"]) {
        Some(raw) => match FieldControl::parse(raw) {
            Some(c) => c,
            None => {
                ctx.error("BOS3011", Some(path), format!("control '{raw}' is not a known field control"), None);
                control_for_type(&ty, list_like)
            }
        },
        None => {
            auto.push("control".to_string());
            if prop_bool(props, &["multiLine", "multiline"]) == Some(true) {
                FieldControl::TextArea
            } else if let Some(c) = extended.and_then(control_for_extended) {
                c
            } else if lookup.is_some() {
                FieldControl::Lookup
            } else {
                control_for_type(&ty, list_like)
            }
        }
    };

    // Options for enum fields.
    let mut options = Vec::new();
    if let BosType::Enum(enum_name) = &ty {
        match ctx.catalog.enum_members(enum_name) {
            Some(members) => {
                auto.push("options".to_string());
                options = members.iter().map(|m| FieldOptionIR { value: m.clone(), caption: prettify_caption(m) }).collect();
            }
            None => ctx.reference("BOS3012", Some(path), format!("Enum '{enum_name}' used by field '{source}' was not found"), None),
        }
    }

    // Decimal places / numbers.
    let decimals = match prop(props, &["decimalPlaces"]) {
        Some(raw) => {
            let parsed = parse_decimal_places(raw);
            if parsed.is_none() {
                ctx.error("BOS3011", Some(path), format!("decimalPlaces '{raw}' must look like `2` or `2:5`"), None);
            }
            parsed
        }
        None => match ty {
            BosType::Decimal(_, s) => {
                auto.push("decimals".to_string());
                let s = s.min(10) as u8;
                Some((s, s))
            }
            BosType::Integer | BosType::BigInt => {
                auto.push("decimals".to_string());
                Some((0, 0))
            }
            _ => None,
        },
    };
    let max_length = match ty {
        BosType::Text(n) => {
            if n.is_some() {
                auto.push("maxLength".to_string());
            }
            n
        }
        _ => None,
    };

    // Mandatory & editability.
    let is_pk = ctx.table.is_some_and(|t| src.column.is_some_and(|c| t.primary_key.iter().any(|k| k.eq_ignore_ascii_case(&c.sql_name))));
    let table_required = src.column.is_some_and(|c| !c.is_nullable && c.default_val.is_none() && c.formula.is_none());
    let required = match prop_bool(props, &["required", "showMandatory", "notBlank"]) {
        Some(b) => b,
        None => {
            if table_required {
                auto.push("required".to_string());
            }
            table_required
        }
    };
    let table_read_only = src.binding == FieldBinding::FlowField
        || src.column.is_some_and(|c| c.formula.is_some() || c.sql_name.starts_with('_'))
        || matches!(ty, BosType::Uuid) && src.column.is_some_and(|c| c.default_val.is_some());
    let editable = match prop_bool(props, &["editable"]) {
        Some(b) => b && !table_read_only,
        None => {
            if table_read_only || !ctx.page_editable {
                auto.push("editable".to_string());
            }
            !table_read_only && (ctx.page_editable || src.binding == FieldBinding::Variable)
        }
    };
    if prop_bool(props, &["editable"]) == Some(true) && table_read_only {
        ctx.push(
            PageDiagnosticSeverity::Warning,
            "BOS3011",
            Some(path),
            format!("Field '{source}' is calculated by the table and can never be edited"),
            None,
        );
    }

    // Drill-down for FlowFields.
    let drill_down = src.flow.and_then(|f| {
        if prop_bool(props, &["drillDown"]) == Some(false) {
            return None;
        }
        let explicit = prop(props, &["drillDownPageId", "drillDownPage"]).map(str::to_string);
        let page = explicit.clone().or(f.drill_down_page.clone()).or_else(|| ctx.catalog.table(&f.target_table).and_then(|t| ctx.list_page_for(t)));
        if explicit.is_none() {
            auto.push("drillDown".to_string());
        }
        Some(DrillDownIR { table: f.target_table.clone(), page })
    });
    if let Some(p) = prop(props, &["drillDownPageId", "drillDownPage"])
        && !ctx.page_exists(p)
    {
        ctx.reference("BOS3004", Some(path), format!("Drill-down page '{p}' does not exist"), None);
    }

    let default_value = src
        .column
        .and_then(|c| c.default_val.as_deref().and_then(clean_default))
        .or_else(|| src.variable.as_ref().and_then(|v| v.default_value.clone()));

    let field = ResolvedFieldIR {
        source,
        binding: src.binding,
        key: src.key,
        data_type: src.data_type,
        control,
        required,
        editable,
        is_primary_key: is_pk,
        max_length,
        decimals,
        number_format: prop(props, &["numberFormat"]).map(str::to_string),
        blank_zero: prop_bool(props, &["blankZero"]).unwrap_or(false),
        extended_type: extended,
        options,
        lookup,
        drill_down,
        default_value,
        assist_edit: prop_bool(props, &["assistEdit"]).unwrap_or(false),
        quick_entry: prop_bool(props, &["quickEntry"]).unwrap_or(false),
        placeholder: prop(props, &["placeholder"]).map(str::to_string),
        auto_filled: auto,
    };
    (Some(field), Some(caption))
}

// ----------------------------------------------------
// CONTROLS
// ----------------------------------------------------

fn resolve_part(ctx: &mut Ctx<'_>, ctrl: &PageControlIR, path: &str) -> Option<ResolvedPartIR> {
    let sub = prop(&ctrl.properties, &["subPage", "pagePart", "page"]).map(str::to_string).or_else(|| ctrl.source_expr.clone());
    let Some(sub) = sub else {
        ctx.error("BOS3004", Some(path), format!("Part '{}' does not name a sub page", ctrl.name), Some(format!("Use `part({}; SubPageName)` or `subPage = SubPageName;`.", ctrl.name)));
        return None;
    };
    let sub_page = ctx.catalog.page(&sub).cloned();
    if sub_page.is_none() {
        ctx.reference("BOS3004", Some(path), format!("Sub page '{sub}' does not exist"), None);
    }
    let sub_table = sub_page.as_ref().and_then(|p| p.source_table.as_deref()).and_then(|t| ctx.catalog.table(t));
    let mut link = Vec::new();
    for (k, v) in &ctrl.properties {
        let Some(field) = k.strip_prefix("link.") else { continue };
        let key = match sub_table {
            Some(t) => match t.columns.iter().find(|c| c.name.eq_ignore_ascii_case(field) || c.sql_name.eq_ignore_ascii_case(field)) {
                Some(c) => c.sql_name.clone(),
                None => {
                    ctx.error("BOS3002", Some(path), format!("Link field '{field}' does not exist on table '{}'", t.name), None);
                    continue;
                }
            },
            None => snake(field),
        };
        if let Some(value) = ctx.compile_expr(v, path, "sub page link") {
            link.push(SubPageLinkIR { field: key, value });
        }
    }
    Some(ResolvedPartIR {
        page: sub_page.map(|p| p.name).unwrap_or(sub),
        link,
        update_propagation: prop_bool(&ctrl.properties, &["updatePropagation"]).unwrap_or(false),
    })
}

fn resolve_control(ctx: &mut Ctx<'_>, ctrl: &PageControlIR, parent: &str) -> ResolvedControlIR {
    let kind = ctrl.kind_typed();
    let path = if parent.is_empty() { format!("{}:{}", kind.as_str(), ctrl.name) } else { format!("{parent}/{}:{}", kind.as_str(), ctrl.name) };

    match &kind {
        PageControlKind::Other(k) => ctx.push(
            PageDiagnosticSeverity::Warning,
            "BOS3008",
            Some(&path),
            format!("Unknown control kind '{k}'; it renders as a plain group"),
            None,
        ),
        k if k.is_directive() => ctx.error(
            "BOS3008",
            Some(&path),
            format!("Directive '{}' is only valid inside a pageextension", k.as_str()),
            None,
        ),
        k if !ctx.page_type.allows(k.clone()) => {
            let t = ctx.page_type;
            ctx.error("BOS3008", Some(&path), format!("'{}' controls are not allowed on {} pages", k.as_str(), t), None)
        }
        _ => {}
    }
    ctx.unique("control", &ctrl.name, &path);

    let (field, field_caption) = if kind.is_bound() || (kind == PageControlKind::Image && ctrl.source_expr.is_some()) {
        resolve_field(ctx, ctrl, &path)
    } else {
        (None, None)
    };
    let part = if kind == PageControlKind::Part { resolve_part(ctx, ctrl, &path) } else { None };

    let caption = field_caption.or_else(|| ctrl.caption.clone()).or_else(|| match kind {
        PageControlKind::Group | PageControlKind::Tab | PageControlKind::WizardStep | PageControlKind::CueGroup | PageControlKind::Part | PageControlKind::Card => {
            Some(prettify_caption(&ctrl.name))
        }
        _ => None,
    });

    let props = &ctrl.properties;
    let show_when = prop(props, &["showWhen"]).and_then(|s| ctx.compile_expr(s, &path, "visibility"));
    let enable_when = prop(props, &["enableWhen"]).and_then(|s| ctx.compile_expr(s, &path, "enabled"));
    let editable_when = prop(props, &["editableWhen"]).and_then(|s| ctx.compile_expr(s, &path, "editable"));
    let style_expr = prop(props, &["styleExpr"]).and_then(|s| ctx.compile_expr(s, &path, "style"));

    let mut responsive = ctrl.responsive.clone();
    if let Some(imp) = prop(props, &["importance"]) {
        match Importance::parse(imp) {
            Some(i) => responsive.get_or_insert_with(Default::default).importance = Some(i),
            None => ctx.error("BOS3011", Some(&path), format!("importance '{imp}' is not Promoted, Standard or Additional"), None),
        }
    }

    let style = ctx.check_style(&ctrl.style, &path);
    let passthrough: BTreeMap<String, String> = props
        .iter()
        .filter(|(k, _)| !CONSUMED_CONTROL_PROPS.contains(&k.as_str()) && !k.starts_with("link.") && k.as_str() != "importance")
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();

    let children = ctrl.children.iter().map(|c| resolve_control(ctx, c, &path)).collect();
    ResolvedControlIR {
        kind: kind.as_str().to_string(),
        name: ctrl.name.clone(),
        path,
        caption,
        tooltip: prop(props, &["tooltip", "toolTip"]).map(str::to_string),
        instructional_text: prop(props, &["instructionalText"]).map(str::to_string),
        visible: prop_bool(props, &["visible"]).unwrap_or(true),
        field,
        part,
        style,
        responsive: responsive.filter(|r| !r.is_empty()),
        show_when,
        enable_when,
        editable_when,
        style_expr,
        triggers: ctrl.triggers.iter().map(|t| t.name.clone()).collect(),
        properties: passthrough,
        children,
    }
}

// ----------------------------------------------------
// ACTIONS
// ----------------------------------------------------

fn resolve_actions(ctx: &mut Ctx<'_>, page: &PageIR) -> Vec<ResolvedActionIR> {
    let names: Vec<String> = page.actions.iter().map(|a| a.name.clone()).collect();
    let default_selection = if ctx.page_type.is_list_like() { ActionSelection::Single } else { ActionSelection::None };
    let mut out = Vec::new();
    for a in &page.actions {
        let path = format!("action:{}", a.name);
        ctx.unique("action", &a.name, &path);
        if let Some(g) = &a.group
            && !page.action_groups.iter().any(|x| x.name.eq_ignore_ascii_case(g))
        {
            ctx.error("BOS3004", Some(&path), format!("Action group '{g}' does not exist"), None);
        }
        if let Some(cu) = &a.run_codeunit {
            let cu_name = cu.split('.').next().unwrap_or(cu);
            if !ctx.catalog.has_codeunit(cu_name) {
                ctx.reference("BOS3004", Some(&path), format!("Codeunit '{cu_name}' run by action '{}' does not exist", a.name), None);
            }
        }
        if let Some(p) = &a.open_page
            && !ctx.page_exists(p)
        {
            ctx.reference("BOS3004", Some(&path), format!("Page '{p}' opened by action '{}' does not exist", a.name), None);
        }
        if let Some(t) = &a.target
            && (!names.iter().any(|n| n.eq_ignore_ascii_case(t)) || t.eq_ignore_ascii_case(&a.name))
        {
            ctx.error("BOS3004", Some(&path), format!("actionref '{}' points to unknown action '{t}'", a.name), None);
        }
        let selection = match a.selection.as_deref() {
            Some(raw) => ActionSelection::parse(raw).unwrap_or_else(|| {
                ctx.error("BOS3011", Some(&path), format!("selection '{raw}' must be none, single or multiple"), None);
                default_selection
            }),
            None => default_selection,
        };
        let icon = a.icon.as_ref().and_then(|i| {
            let n = normalize_icon_name(i);
            if n.is_none() {
                ctx.error("BOS3003", Some(&path), format!("Icon '{i}' is not a valid icon name"), None);
            }
            n
        });
        let show_when = if a.visible { a.show_when.as_ref().and_then(|s| ctx.compile_expr(s, &path, "visibility")) } else { None };
        let enable_when = a.enable_when.as_ref().and_then(|s| ctx.compile_expr(s, &path, "enabled"));

        let target_table = a.open_page.as_deref().and_then(|p| ctx.catalog.page(p)).and_then(|p| p.source_table.clone()).and_then(|t| ctx.catalog.table(&t).cloned());
        let mut run_page_link = Vec::new();
        for (field, value) in &a.run_page_link {
            let key = match &target_table {
                Some(t) => match t.columns.iter().find(|c| c.name.eq_ignore_ascii_case(field) || c.sql_name.eq_ignore_ascii_case(field)) {
                    Some(c) => c.sql_name.clone(),
                    None => {
                        ctx.error("BOS3002", Some(&path), format!("Link field '{field}' does not exist on table '{}'", t.name), None);
                        continue;
                    }
                },
                None => snake(field),
            };
            if let Some(value) = ctx.compile_expr(value, &path, "page link") {
                run_page_link.push(SubPageLinkIR { field: key, value });
            }
        }
        let style = ctx.check_style(&a.style, &path);
        if !a.visible {
            continue;
        }
        out.push(ResolvedActionIR {
            name: a.name.clone(),
            // An actionref without its own caption shows the referenced action's caption (as in BC).
            caption: a
                .caption
                .clone()
                .or_else(|| {
                    let t = a.target.as_deref()?;
                    let target = page.actions.iter().find(|x| x.name.eq_ignore_ascii_case(t))?;
                    Some(target.caption.clone().unwrap_or_else(|| prettify_caption(&target.name)))
                })
                .unwrap_or_else(|| prettify_caption(&a.name)),
            icon: icon.or_else(|| style.as_ref().and_then(|s| s.icon.clone())),
            tooltip: a.tooltip.clone(),
            area: a.area.unwrap_or(ActionArea::Processing),
            group: a.group.clone(),
            primary: a.primary,
            promoted_only: a.promoted_only,
            promoted_category: a.promoted_category.clone(),
            is_default: a.is_default,
            in_footer_bar: {
                let is_dialog = ctx.page_type.is_dialog();
                let promoted = a.in_footer_bar || (is_dialog && (a.area.is_none() || a.primary));
                if is_dialog && !promoted && a.visible {
                    ctx.warning(
                        "BOS3015",
                        Some(&path),
                        format!("Action '{}' is inside an area on dialog page '{}' and will not be displayed; add 'inFooterBar = true' or declare outside an area", a.name, page.name),
                        Some("Dialog pages only display actions configured for the footer bar.".to_string()),
                    );
                }
                promoted
            },
            ellipsis: a.ellipsis,
            shortcut: a.shortcut.clone(),
            confirm: a.confirm.clone(),
            selection,
            run_codeunit: a.run_codeunit.clone(),
            open_page: a.open_page.clone(),
            run_page_mode: a.run_page_mode,
            run_page_link,
            target: a.target.clone(),
            show_when,
            enable_when,
            has_script: a.script.as_deref().is_some_and(|s| !s.trim().is_empty()),
            style,
        });
    }
    out
}

// ----------------------------------------------------
// CUSTOMIZATION WHITELIST
// ----------------------------------------------------

/// Control properties a super user may change through the customization layer.
const CUSTOMIZABLE_PROPS: &[&str] = &["caption", "label", "visible", "importance", "tooltip", "toolTip", "colSpan", "hideBelow", "labelPosition"];

/// Removes everything a customization layer may not do and reports each removal as BOS3009.
pub fn enforce_customization_whitelist(ext: &PageExtensionIR) -> (PageExtensionIR, Vec<PageDiagnosticIR>) {
    let mut diags = Vec::new();
    let mut deny = |path: String, message: String| {
        diags.push(PageDiagnosticIR {
            severity: PageDiagnosticSeverity::Error,
            code: "BOS3009".to_string(),
            message,
            object: ext.name.clone(),
            path: Some(path),
            hint: Some("Customizations may only restyle, rename, hide, move or add table fields.".to_string()),
        })
    };
    let mut clean = ext.clone();
    clean.controls.retain(|c| {
        let path = format!("{}:{}", c.kind, c.name);
        let kind = c.kind_typed();
        match kind {
            PageControlKind::Modify => {
                if let Some(bad) = c.properties.keys().find(|k| !CUSTOMIZABLE_PROPS.contains(&k.as_str())) {
                    deny(path, format!("Property '{bad}' cannot be changed by a customization"));
                    return false;
                }
                if !c.triggers.is_empty() || !c.children.is_empty() {
                    deny(path, "A customization cannot add triggers or controls inside modify()".to_string());
                    return false;
                }
                true
            }
            PageControlKind::Hide | PageControlKind::MoveBefore | PageControlKind::MoveAfter | PageControlKind::MoveFirst | PageControlKind::MoveLast => true,
            PageControlKind::AddFirst | PageControlKind::AddLast | PageControlKind::AddBefore | PageControlKind::AddAfter => {
                let ok = c.children.iter().all(|ch| {
                    ch.kind_typed() == PageControlKind::Field
                        && ch.triggers.is_empty()
                        && ch.children.is_empty()
                        && ch.properties.keys().all(|k| CUSTOMIZABLE_PROPS.contains(&k.as_str()))
                });
                if !ok {
                    deny(path, "A customization may only add plain table fields".to_string());
                }
                ok
            }
            _ => {
                deny(path, format!("'{}' is not allowed in a customization", c.kind));
                false
            }
        }
    });
    clean.actions.retain(|a| {
        let path = format!("action:{}", a.name);
        match a.placement.as_deref() {
            Some("hide") | Some("moveBefore") | Some("moveAfter") => true,
            Some("modify") => {
                let behaviour = a.script.is_some()
                    || a.run_codeunit.is_some()
                    || a.open_page.is_some()
                    || a.confirm.is_some()
                    || a.show_when.is_some()
                    || a.enable_when.is_some()
                    || !a.run_page_link.is_empty()
                    || a.target.is_some();
                if behaviour {
                    deny(path, "A customization may only change an action's caption, icon, style or prominence".to_string());
                }
                !behaviour
            }
            _ => {
                deny(path, "A customization cannot add new actions".to_string());
                false
            }
        }
    });
    if !clean.triggers.is_empty() {
        deny("triggers".to_string(), "A customization cannot add triggers".to_string());
        clean.triggers.clear();
    }
    if !clean.variables.is_empty() {
        deny("variables".to_string(), "A customization cannot declare page variables".to_string());
        clean.variables.clear();
    }
    if !clean.action_groups.is_empty() {
        deny("actions".to_string(), "A customization cannot add action groups".to_string());
        clean.action_groups.clear();
    }
    (clean, diags)
}

// ----------------------------------------------------
// ENTRY POINT
// ----------------------------------------------------

/// Compiles a page: applies extensions (in the given order), resolves every control and action and
/// returns the compiled page with all diagnostics. Extensions should be pre-sorted with
/// [`crate::page_merger::sort_extensions_for_merge`].
pub fn compile_page(base: &PageIR, extensions: &[PageExtensionIR], catalog: &dyn PageCatalog) -> PageCompileResult {
    let mut diags = Vec::new();
    let mut exts = Vec::with_capacity(extensions.len());
    for ext in extensions {
        if ext.is_customization {
            let (clean, d) = enforce_customization_whitelist(ext);
            diags.extend(d);
            exts.push(clean);
        } else {
            exts.push(ext.clone());
        }
    }

    let MergeOutcome { page, applied, orphans } = merge_page_detailed(base, &exts);
    for o in orphans {
        diags.push(PageDiagnosticIR {
            severity: if o.customization { PageDiagnosticSeverity::Warning } else { PageDiagnosticSeverity::Error },
            code: "BOS3007".to_string(),
            message: format!("{}({}) in '{}' was skipped: {}", o.directive, o.anchor, o.extension, o.reason),
            object: o.extension.clone(),
            path: Some(format!("{}:{}", o.directive, o.anchor)),
            hint: o.customization.then(|| "The base page changed; reopen the page designer to review this customization.".to_string()),
        });
    }

    let page_type = PageType::parse(&page.page_type).unwrap_or_else(|| {
        let names: Vec<&str> = PageType::ALL.iter().map(|t| t.as_str()).collect();
        diags.push(PageDiagnosticIR {
            severity: PageDiagnosticSeverity::Error,
            code: "BOS3010".to_string(),
            message: format!("Unknown page type '{}'", page.page_type),
            object: page.name.clone(),
            path: None,
            hint: Some(format!("Use one of {}.", names.join(", "))),
        });
        PageType::Card
    });

    let table = page.source_table.as_deref().and_then(|t| catalog.table(t));
    let mut ctx = Ctx {
        catalog,
        page_name: page.name.clone(),
        page_type,
        table,
        table_unresolved: page.source_table.is_some() && table.is_none(),
        variables: page.variables.clone(),
        page_editable: false,
        diags,
        names: HashSet::new(),
    };
    if let (Some(t), None) = (&page.source_table, table) {
        ctx.reference("BOS3001", None, format!("Source table '{t}' of page '{}' does not exist", page.name), None);
    }

    let has_source = page.source_table.is_some();
    let editable = page.editable.unwrap_or(has_source && !matches!(page_type, PageType::RoleCenter | PageType::Api | PageType::ConfirmationDialog));
    ctx.page_editable = editable;

    let card_page = match &page.card_page {
        Some(p) => {
            if !ctx.page_exists(p) {
                ctx.reference("BOS3004", None, format!("Card page '{p}' does not exist"), None);
            }
            Some(p.clone())
        }
        // NAV-style convenience: a list opens the card/document page of the same table.
        None if page_type == PageType::List => table.and_then(|t| {
            catalog
                .pages()
                .into_iter()
                .find(|p| {
                    matches!(p.page_type_kind(), Some(PageType::Card | PageType::Document))
                        && p.source_table.as_deref().is_some_and(|s| s.eq_ignore_ascii_case(&t.name) || s.eq_ignore_ascii_case(&t.sql_table_name))
                })
                .map(|p| p.name.clone())
        }),
        None => None,
    };

    let mut data_caption_fields = Vec::new();
    for f in &page.data_caption_fields {
        match ctx.column(f) {
            Some(c) => data_caption_fields.push(c.sql_name.clone()),
            None if ctx.table_unresolved => data_caption_fields.push(snake(f)),
            None => ctx.error("BOS3002", None, format!("Data caption field '{f}' does not exist on the source table"), None),
        }
    }

    let style = ctx.check_style(&page.style, "page");
    let controls: Vec<ResolvedControlIR> = page.controls.iter().map(|c| resolve_control(&mut ctx, c, "")).collect();
    let actions = resolve_actions(&mut ctx, &page);

    let caption = page
        .caption
        .clone()
        .or_else(|| table.map(|t| t.caption.clone()).filter(|c| table.is_some_and(|t| !c.eq_ignore_ascii_case(&t.name))))
        .unwrap_or_else(|| prettify_caption(&page.name));

    let compiled = CompiledPageIR {
        ir_version: PAGE_IR_VERSION,
        name: page.name.clone(),
        caption,
        page_type,
        source_table: table.map(|t| t.name.clone()).or(page.source_table.clone()),
        source_sql_table: table.map(|t| t.sql_table_name.clone()).or_else(|| page.source_table.as_deref().map(snake)),
        primary_key: table.map(|t| t.primary_key.clone()).unwrap_or_default(),
        editable,
        insert_allowed: page.insert_allowed.unwrap_or(editable),
        modify_allowed: page.modify_allowed.unwrap_or(editable),
        delete_allowed: page.delete_allowed.unwrap_or(editable),
        card_page,
        data_caption_fields,
        refresh_on_activate: page.refresh_on_activate,
        delayed_insert: page.delayed_insert,
        multiple_new_lines: page.multiple_new_lines,
        auto_split_key: page.auto_split_key,
        show_filter: page.show_filter.unwrap_or(page_type.is_list_like()),
        usage_category: page.usage_category.clone(),
        instructional_text: page.instructional_text.clone(),
        filters: page.filters.clone(),
        views: page.views.clone(),
        style,
        controls,
        actions,
        action_groups: page.action_groups.clone(),
        variables: page.variables.clone(),
        triggers: page.triggers.iter().map(|t| t.name.clone()).collect(),
        applied_extensions: applied,
        properties: page.properties.clone(),
    };
    PageCompileResult { page: compiled, diagnostics: ctx.diags }
}
