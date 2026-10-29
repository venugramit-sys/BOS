//! HTTP routes of the page designer (Studio WYSIWYG editor).
//!
//! | Route | Purpose |
//! | :--- | :--- |
//! | `POST /api/v1/pages/parse` | `.bos` source → outline (node paths + byte ranges), lowered pages, diagnostics |
//! | `POST /api/v1/pages/edit` | apply designer operations to the source (span edits, comments kept) |
//! | `GET  /api/v1/pages/property-schema` | property vocabulary of the designer (from the page IR types) |
//! | `GET  /api/v1/pages/{name}/customization` | the super-user customization layer of a page |
//! | `PUT  /api/v1/pages/{name}/customization` | save (and publish) the customization layer |
//! | `POST /api/v1/pages/{name}/customization/preview` | compile the page with unsaved customization changes |
//!
//! Develop mode edits source text, which the Studio then applies through `/api/script/apply` like any
//! hand-written change. Customize mode never touches source: it stores the machine-owned extension
//! `<Page>_Customization` (package `bos.customizations`), restricted to the customization whitelist.

use axum::{
    Json, Router,
    extract::{Extension, Path},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use bosc_analyzer::page_resolver::enforce_customization_whitelist;
use bosc_analyzer::page_source::{self, CustomizationChanges, PageEditOp};
use bosc_analyzer::SemanticAnalyzer;
use bosc_ir::*;
use bosc_syntax::{Lexer, Parser};
use serde::Deserialize;
use std::sync::Arc;

use crate::meta_manager::RegisterMetaObjectRequest;
use crate::server::{AppError, extract_context};
use crate::BosEngine;

/// Package that owns every customization layer.
pub const CUSTOMIZATION_PACKAGE: &str = "bos.customizations";

/// Roles allowed to change pages in the designer.
const DESIGNER_ROLES: &[&str] = &["SUPER", "Admin", "Global Admin", "Developer", "PageDesigner"];

pub(crate) fn designer_routes() -> Router {
    Router::new()
        .route("/api/v1/pages/parse", post(parse_handler))
        .route("/api/v1/pages/edit", post(edit_handler))
        .route("/api/v1/pages/property-schema", get(property_schema_handler))
        .route("/api/v1/pages/{name}/customization", get(get_customization_handler).put(put_customization_handler))
        .route("/api/v1/pages/{name}/customization/preview", post(preview_customization_handler))
}

fn bad_request(message: impl Into<String>) -> Response {
    AppError::new(StatusCode::BAD_REQUEST, message.into()).with_code("INVALID_PAGE_SOURCE").into_response()
}

#[allow(clippy::result_large_err)]
fn require_designer(headers: &HeaderMap) -> Result<(), Response> {
    let ctx = extract_context(headers)?;
    if ctx.roles.iter().any(|r| DESIGNER_ROLES.iter().any(|d| d.eq_ignore_ascii_case(r))) {
        Ok(())
    } else {
        Err(AppError::new(StatusCode::FORBIDDEN, "the page designer requires a designer role").with_code("FORBIDDEN").into_response())
    }
}

/// Lowers `source` into pages / extensions with the analyzer (open catalog: references to objects that
/// are not in the source are warnings, the node's closed catalog checks them at preview / publish).
fn lower(source: &str, package: &str) -> Result<ModelIR, String> {
    let tokens = Lexer::new(source).tokenize().map_err(|e| e.to_string())?;
    let program = Parser::with_source(tokens, source.to_string()).parse_program().map_err(|e| e.to_string())?;
    SemanticAnalyzer::new(package).analyze_and_lower(&program).map_err(|e| e.to_string())
}

fn model_json(model: &ModelIR) -> serde_json::Value {
    serde_json::json!({
        "pages": model.pages,
        "page_extensions": model.page_extensions,
        "diagnostics": model.diagnostics,
    })
}

#[derive(Debug, Deserialize)]
struct ParseRequest {
    source: String,
    #[serde(default)]
    package_name: Option<String>,
}

async fn parse_handler(Json(req): Json<ParseRequest>) -> Response {
    let outline = match page_source::outline(&req.source) {
        Ok(o) => o,
        Err(e) => return bad_request(e),
    };
    let package = req.package_name.unwrap_or_else(|| "bos.app".to_string());
    match lower(&req.source, &package) {
        Ok(model) => Json(serde_json::json!({ "outline": outline, "model": model_json(&model) })).into_response(),
        Err(e) => Json(serde_json::json!({ "outline": outline, "model": null, "error": e })).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct EditRequest {
    source: String,
    page: String,
    ops: Vec<PageEditOp>,
    #[serde(default)]
    package_name: Option<String>,
}

async fn edit_handler(headers: HeaderMap, Json(req): Json<EditRequest>) -> Response {
    if let Err(r) = require_designer(&headers) {
        return r;
    }
    let result = match page_source::apply_edits(&req.source, &req.page, &req.ops) {
        Ok(r) => r,
        Err(e) => return bad_request(e),
    };
    let package = req.package_name.unwrap_or_else(|| "bos.app".to_string());
    let model = lower(&result.source, &package);
    Json(serde_json::json!({
        "source": result.source,
        "outline": result.outline,
        "selected": result.selected,
        "model": model.as_ref().ok().map(model_json),
        "error": model.err(),
    }))
    .into_response()
}

async fn property_schema_handler() -> Response {
    Json(property_schema()).into_response()
}

/// Designer property vocabulary, generated from the page IR types so it never drifts from the compiler.
pub fn property_schema() -> serde_json::Value {
    use PageControlKind as K;
    let kinds = [
        K::Area, K::Group, K::Grid, K::Fixed, K::Repeater, K::Part, K::SystemPart, K::Field, K::Label, K::Image, K::Spacer,
        K::Divider, K::CueGroup, K::Cue, K::Headline, K::UserControl, K::Tabs, K::Tab, K::Flex, K::Split, K::Card,
        K::WizardStep, K::Container, K::Section, K::Header, K::Footer, K::Nav, K::Aside, K::Article, K::Action,
    ];
    let page_types: Vec<serde_json::Value> = PageType::ALL
        .iter()
        .map(|t| {
            serde_json::json!({
                "name": t.as_str(),
                "allows": kinds.iter().filter(|k| t.allows((*k).clone())).map(|k| k.as_str()).collect::<Vec<_>>(),
            })
        })
        .collect();
    let names = |v: Vec<&'static str>| serde_json::json!(v);
    serde_json::json!({
        "page_types": page_types,
        "control_kinds": kinds.iter().map(|k| k.as_str()).collect::<Vec<_>>(),
        "field_controls": FieldControl::ALL.iter().map(|c| serde_json::to_value(c).unwrap_or_default()).collect::<Vec<_>>(),
        "color_tokens": ColorToken::ALL.iter().map(|t| t.as_str()).collect::<Vec<_>>(),
        "radius": names(Radius::names()),
        "shadow": names(Shadow::names()),
        "density": names(Density::names()),
        "emphasis": names(Emphasis::names()),
        "breakpoints": names(Breakpoint::names()),
        "importance": names(Importance::names()),
        "action_areas": names(ActionArea::names()),
        "extended_datatypes": names(ExtendedDatatype::names()),
        "open_modes": names(PageOpenMode::names()),
        "selection": names(ActionSelection::names()),
        "max_border_width": PageStyleIR::MAX_BORDER_WIDTH,
        "grid_columns": BreakpointSpans::COLUMNS,
        // Property sheet rows: `customize` marks what Customize mode may change (server-enforced).
        "page_properties": [
            { "name": "caption", "type": "text", "customize": true },
            { "name": "icon", "type": "icon", "customize": true },
            { "name": "pageType", "type": "enum:page_types" },
            { "name": "cardPageId", "type": "page" },
            { "name": "dataCaptionFields", "type": "fields" },
            { "name": "editable", "type": "bool" },
            { "name": "insertAllowed", "type": "bool" },
            { "name": "modifyAllowed", "type": "bool" },
            { "name": "deleteAllowed", "type": "bool" },
            { "name": "instructionalText", "type": "text" },
        ],
        "control_properties": [
            { "name": "caption", "type": "text", "customize": true },
            { "name": "icon", "type": "icon", "customize": true, "kinds": ["field", "group", "part", "action", "actionref", "cue"] },
            { "name": "tooltip", "type": "text", "customize": true },
            { "name": "visible", "type": "bool", "customize": true },
            { "name": "importance", "type": "enum:importance", "customize": true, "kinds": ["field"] },
            { "name": "editable", "type": "bool", "kinds": ["field"] },
            { "name": "showMandatory", "type": "bool", "kinds": ["field"] },
            { "name": "control", "type": "enum:field_controls", "kinds": ["field"] },
            { "name": "extendedDatatype", "type": "enum:extended_datatypes", "kinds": ["field"] },
            { "name": "decimalPlaces", "type": "text", "kinds": ["field"] },
            { "name": "blankZero", "type": "bool", "kinds": ["field"] },
            { "name": "placeholder", "type": "text", "kinds": ["field"] },
            { "name": "lookupPageId", "type": "page", "kinds": ["field"] },
            { "name": "drillDownPageId", "type": "page", "kinds": ["field", "cue"] },
            { "name": "instructionalText", "type": "text" },
            { "name": "showWhen", "type": "expr" },
            { "name": "enableWhen", "type": "expr" },
            { "name": "editableWhen", "type": "expr", "kinds": ["field"] },
            { "name": "styleExpr", "type": "expr" },
            { "name": "cols", "type": "int", "kinds": ["grid", "fixed"] },
            { "name": "ratio", "type": "text", "kinds": ["split"] },
            { "name": "gap", "type": "enum:sm,md,lg,none", "kinds": ["flex"] },
            { "name": "src", "type": "text", "kinds": ["image"] },
            { "name": "size", "type": "enum:sm,md,lg,xl", "kinds": ["image", "spacer"] },
            { "name": "updatePropagation", "type": "bool", "kinds": ["part"] },
            { "name": "showCaption", "type": "bool" },
            { "name": "labelPosition", "type": "enum:top,left,right,none", "kinds": ["field", "label"], "customize": true },
        ],
        "style_properties": [
            { "name": "background", "type": "color", "customize": true },
            { "name": "textColor", "type": "color", "customize": true },
            { "name": "borderColor", "type": "color", "customize": true },
            { "name": "borderWidth", "type": "int", "customize": true },
            { "name": "radius", "type": "enum:radius", "customize": true },
            { "name": "shadow", "type": "enum:shadow", "customize": true },
            { "name": "density", "type": "enum:density", "customize": true },
            { "name": "emphasis", "type": "enum:emphasis", "customize": true },
            { "name": "icon", "type": "icon", "customize": true },
            { "name": "fontSize", "type": "text", "customize": true },
            { "name": "fontWeight", "type": "text", "customize": true },
            { "name": "textAlign", "type": "enum:left,center,right,justify", "customize": true },
            { "name": "letterSpacing", "type": "text", "customize": true },
            { "name": "lineHeight", "type": "text", "customize": true },
            { "name": "textTransform", "type": "enum:none,uppercase,lowercase,capitalize", "customize": true },
            { "name": "textDecoration", "type": "enum:none,underline,line-through", "customize": true },
            { "name": "borderStyle", "type": "enum:solid,dashed,dotted,none", "customize": true },
            { "name": "padding", "type": "text", "customize": true },
            { "name": "margin", "type": "text", "customize": true },
            { "name": "gap", "type": "text", "customize": true },
            { "name": "width", "type": "text", "customize": true },
            { "name": "maxWidth", "type": "text", "customize": true },
            { "name": "minWidth", "type": "text", "customize": true },
            { "name": "height", "type": "text", "customize": true },
            { "name": "minHeight", "type": "text", "customize": true },
            { "name": "maxHeight", "type": "text", "customize": true },
            { "name": "opacity", "type": "text", "customize": true },
            { "name": "overflow", "type": "enum:visible,hidden,auto,scroll", "customize": true },
            { "name": "display", "type": "enum:block,flex,grid,inline-flex,none", "customize": true },
            { "name": "justifyContent", "type": "enum:start,center,end,between,around,evenly", "customize": true },
            { "name": "alignItems", "type": "enum:start,center,end,stretch,baseline", "customize": true },
            { "name": "flexDirection", "type": "enum:row,column,row-reverse,column-reverse", "customize": true },
            { "name": "flexWrap", "type": "enum:wrap,nowrap", "customize": true },
            { "name": "icon", "type": "icon", "customize": true },
            { "name": "iconSize", "type": "enum:xs,sm,md,lg,xl,2xl", "customize": true },
            { "name": "iconColor", "type": "color", "customize": true },
            { "name": "iconBackground", "type": "color", "customize": true },
            { "name": "iconRadius", "type": "enum:radius", "customize": true },
        ],
        "responsive_properties": [
            { "name": "xs", "type": "span", "customize": true },
            { "name": "sm", "type": "span", "customize": true },
            { "name": "md", "type": "span", "customize": true },
            { "name": "lg", "type": "span", "customize": true },
            { "name": "xl", "type": "span", "customize": true },
            { "name": "hideBelow", "type": "enum:breakpoints", "customize": true },
        ],
    })
}

fn customization_name(page: &str) -> String {
    format!("{page}_Customization")
}

/// Builds the customization extension of `page` from designer changes (printed, parsed and lowered exactly
/// like hand-written source, then filtered by the whitelist).
fn build_customization(page: &str, changes: &CustomizationChanges) -> Result<(String, PageExtensionIR, Vec<PageDiagnosticIR>), String> {
    let source = page_source::print_customization(page, changes)?;
    let model = lower(&source, CUSTOMIZATION_PACKAGE)?;
    let mut ext = model
        .page_extensions
        .into_iter()
        .next()
        .ok_or_else(|| "the customization did not produce a page extension".to_string())?;
    ext.is_customization = true;
    ext.scope = None;
    let (clean, diags) = enforce_customization_whitelist(&ext);
    Ok((source, clean, diags))
}

async fn get_customization_handler(Extension(engine): Extension<Arc<BosEngine>>, Path(name): Path<String>) -> Response {
    let Some(svc) = engine.page_service() else {
        return AppError::new(StatusCode::SERVICE_UNAVAILABLE, "page service is not running").with_code("PAGES_NOT_READY").into_response();
    };
    let Some(entry) = svc.page(&name) else {
        return AppError::new(StatusCode::NOT_FOUND, format!("page '{name}' does not exist")).with_code("UNKNOWN_PAGE").into_response();
    };
    let ext = svc.page_extension(&customization_name(&entry.name));
    let changes = ext.as_ref().map(page_source::changes_from_extension).unwrap_or_default();
    let source = page_source::print_customization(&entry.name, &changes).unwrap_or_default();
    Json(serde_json::json!({ "page": entry.name, "exists": ext.is_some(), "changes": changes, "source": source })).into_response()
}

#[derive(Debug, Deserialize)]
struct CustomizationRequest {
    changes: CustomizationChanges,
}

async fn put_customization_handler(
    headers: HeaderMap,
    Extension(engine): Extension<Arc<BosEngine>>,
    Path(name): Path<String>,
    Json(req): Json<CustomizationRequest>,
) -> Response {
    if let Err(r) = require_designer(&headers) {
        return r;
    }
    let Some(svc) = engine.page_service() else {
        return AppError::new(StatusCode::SERVICE_UNAVAILABLE, "page service is not running").with_code("PAGES_NOT_READY").into_response();
    };
    let Some(entry) = svc.page(&name) else {
        return AppError::new(StatusCode::NOT_FOUND, format!("page '{name}' does not exist")).with_code("UNKNOWN_PAGE").into_response();
    };
    let (source, ext, denied) = match build_customization(&entry.name, &req.changes) {
        Ok(v) => v,
        Err(e) => return bad_request(e),
    };
    if !denied.is_empty() {
        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(serde_json::json!({ "error": "the customization contains changes that are not allowed", "code": "BOS3009", "diagnostics": denied })),
        )
            .into_response();
    }
    let definition = match serde_json::to_value(&ext) {
        Ok(v) => v,
        Err(e) => return AppError::new(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };
    let request = RegisterMetaObjectRequest {
        object_type: "pageextension".to_string(),
        name: ext.name.clone(),
        package_name: Some(CUSTOMIZATION_PACKAGE.to_string()),
        version: None,
        caption: Some(format!("Customization of {}", entry.name)),
        sub_type: Some("PageExtension".to_string()),
        data_per_company: None,
        columns: None,
        primary_key: None,
        check_constraints: None,
        state_machine: None,
        definition: Some(definition),
        allow_destructive: false,
    };
    if let Err(e) = engine.register_meta_object(request).await {
        return AppError::new(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
    }
    let compiled = engine.page_service().and_then(|s| s.page(&entry.name));
    Json(serde_json::json!({
        "page": entry.name,
        "source": source,
        "extension": ext,
        "diagnostics": compiled.map(|c| c.diagnostics.clone()).unwrap_or_default(),
    }))
    .into_response()
}

async fn preview_customization_handler(
    headers: HeaderMap,
    Extension(engine): Extension<Arc<BosEngine>>,
    Path(name): Path<String>,
    Json(req): Json<CustomizationRequest>,
) -> Response {
    if let Err(r) = require_designer(&headers) {
        return r;
    }
    let Some(svc) = engine.page_service() else {
        return AppError::new(StatusCode::SERVICE_UNAVAILABLE, "page service is not running").with_code("PAGES_NOT_READY").into_response();
    };
    let Some((base, mut extensions)) = svc.base_page(&name) else {
        return AppError::new(StatusCode::NOT_FOUND, format!("page '{name}' does not exist")).with_code("UNKNOWN_PAGE").into_response();
    };
    let (_, draft, denied) = match build_customization(&base.name, &req.changes) {
        Ok(v) => v,
        Err(e) => return bad_request(e),
    };
    extensions.retain(|e| !e.name.eq_ignore_ascii_case(&draft.name));
    extensions.push(draft);
    match svc.preview(base, Some(extensions), true).await {
        Ok(mut result) => {
            result.diagnostics.extend(denied);
            Json(result).into_response()
        }
        Err(e) => AppError::new(StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}
