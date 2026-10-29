use std::collections::HashMap;
use anyhow::Result;
use bosc_syntax::{Item, Lexer, Parser};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};

#[derive(Debug, Serialize, Deserialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    #[serde(default)]
    id: Option<Value>,
    method: String,
    #[serde(default)]
    params: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize)]
struct JsonRpcNotification {
    jsonrpc: String,
    method: String,
    params: Value,
}

pub async fn run_lsp_server() -> Result<()> {
    tracing::info!("Starting BOS Language Server (bosc lsp) on stdio...");
    let mut stdin = BufReader::new(tokio::io::stdin());
    let mut stdout = tokio::io::stdout();

    let mut documents: HashMap<String, String> = HashMap::new();

    loop {
        // Read Content-Length header
        let mut content_length: Option<usize> = None;
        loop {
            let mut line = String::new();
            let bytes_read = stdin.read_line(&mut line).await?;
            if bytes_read == 0 {
                // EOF reached
                tracing::info!("BOS LSP: stdio closed, exiting.");
                return Ok(());
            }

            let trimmed = line.trim();
            if trimmed.is_empty() {
                // Header section ends with blank line
                break;
            }

            if let Some(len_str) = trimmed.strip_prefix("Content-Length:") {
                content_length = len_str.trim().parse::<usize>().ok();
            }
        }

        let length = match content_length {
            Some(len) => len,
            None => continue,
        };

        // Read exact payload bytes
        let mut body_bytes = vec![0u8; length];
        stdin.read_exact(&mut body_bytes).await?;

        let body_str = match std::str::from_utf8(&body_bytes) {
            Ok(s) => s,
            Err(_) => continue,
        };

        let req: JsonRpcRequest = match serde_json::from_str(body_str) {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!("BOS LSP: Failed to parse JSON-RPC payload: {}", e);
                continue;
            }
        };

        match req.method.as_str() {
            "initialize" => {
                let res = serde_json::json!({
                    "capabilities": {
                        "textDocumentSync": 1, // Full document sync
                        "completionProvider": {
                            "resolveProvider": false,
                            "triggerCharacters": [".", ":", "@", " "]
                        },
                        "hoverProvider": true,
                        "documentSymbolProvider": true
                    },
                    "serverInfo": {
                        "name": "bosc-lsp",
                        "version": env!("CARGO_PKG_VERSION")
                    }
                });
                send_response(&mut stdout, req.id, Some(res), None).await?;
            }
            "initialized" => {
                // No-op confirmation notification
            }
            "shutdown" => {
                send_response(&mut stdout, req.id, Some(Value::Null), None).await?;
            }
            "exit" => {
                break;
            }
            "textDocument/didOpen" => {
                if let Some(params) = req.params {
                    if let Some(td) = params.get("textDocument") {
                        if let (Some(uri), Some(text)) = (td.get("uri").and_then(|u| u.as_str()), td.get("text").and_then(|t| t.as_str())) {
                            documents.insert(uri.to_string(), text.to_string());
                            validate_and_publish_diagnostics(&mut stdout, uri, text).await?;
                        }
                    }
                }
            }
            "textDocument/didChange" => {
                if let Some(params) = req.params {
                    if let (Some(uri), Some(changes)) = (
                        params.get("textDocument").and_then(|td| td.get("uri")).and_then(|u| u.as_str()),
                        params.get("contentChanges").and_then(|c| c.as_array()),
                    ) {
                        if let Some(last_change) = changes.last() {
                            if let Some(text) = last_change.get("text").and_then(|t| t.as_str()) {
                                documents.insert(uri.to_string(), text.to_string());
                                validate_and_publish_diagnostics(&mut stdout, uri, text).await?;
                            }
                        }
                    }
                }
            }
            "textDocument/completion" => {
                let completions = get_completions();
                send_response(&mut stdout, req.id, Some(completions), None).await?;
            }
            "textDocument/hover" => {
                let hover = get_hover(&req.params, &documents);
                send_response(&mut stdout, req.id, Some(hover), None).await?;
            }
            "textDocument/documentSymbol" => {
                let symbols = get_document_symbols(&req.params, &documents);
                send_response(&mut stdout, req.id, Some(symbols), None).await?;
            }
            _ => {
                if req.id.is_some() {
                    let err = serde_json::json!({
                        "code": -32601,
                        "message": format!("Method not found: {}", req.method)
                    });
                    send_response(&mut stdout, req.id, None, Some(err)).await?;
                }
            }
        }
    }

    Ok(())
}

async fn send_response(
    stdout: &mut tokio::io::Stdout,
    id: Option<Value>,
    result: Option<Value>,
    error: Option<Value>,
) -> Result<()> {
    let resp = JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        id,
        result,
        error,
    };
    let json_bytes = serde_json::to_vec(&resp)?;
    let header = format!("Content-Length: {}\r\n\r\n", json_bytes.len());
    stdout.write_all(header.as_bytes()).await?;
    stdout.write_all(&json_bytes).await?;
    stdout.flush().await?;
    Ok(())
}

async fn send_notification(
    stdout: &mut tokio::io::Stdout,
    method: &str,
    params: Value,
) -> Result<()> {
    let notif = JsonRpcNotification {
        jsonrpc: "2.0".to_string(),
        method: method.to_string(),
        params,
    };
    let json_bytes = serde_json::to_vec(&notif)?;
    let header = format!("Content-Length: {}\r\n\r\n", json_bytes.len());
    stdout.write_all(header.as_bytes()).await?;
    stdout.write_all(&json_bytes).await?;
    stdout.flush().await?;
    Ok(())
}

async fn validate_and_publish_diagnostics(
    stdout: &mut tokio::io::Stdout,
    uri: &str,
    text: &str,
) -> Result<()> {
    let mut diagnostics = Vec::new();

    let mut lexer = Lexer::new(text);
    match lexer.tokenize() {
        Ok(tokens) => {
            let mut parser = Parser::new(tokens);
            match parser.parse_program() {
                Ok(_) => {
                    // Clean syntax, zero errors
                }
                Err(err) => {
                    let line = err.line();
                    let col = err.column();
                    let zero_line = line.saturating_sub(1);
                    let zero_col = col.saturating_sub(1);

                    diagnostics.push(serde_json::json!({
                        "range": {
                            "start": { "line": zero_line, "character": zero_col },
                            "end": { "line": zero_line, "character": zero_col + 4 }
                        },
                        "severity": 1, // Error
                        "code": "BOS1001",
                        "source": "bosc",
                        "message": err.to_string()
                    }));
                }
            }
        }
        Err(err) => {
            let line = err.line();
            let col = err.column();
            let zero_line = line.saturating_sub(1);
            let zero_col = col.saturating_sub(1);

            diagnostics.push(serde_json::json!({
                "range": {
                    "start": { "line": zero_line, "character": zero_col },
                    "end": { "line": zero_line, "character": zero_col + 4 }
                },
                "severity": 1, // Error
                "code": "BOS1000",
                "source": "bosc",
                "message": err.to_string()
            }));
        }
    }

    send_notification(
        stdout,
        "textDocument/publishDiagnostics",
        serde_json::json!({
            "uri": uri,
            "diagnostics": diagnostics
        }),
    )
    .await?;

    Ok(())
}

fn get_completions() -> Value {
    let keywords = [
        ("table", "Declares a persistent database table entity with Row-Level Security."),
        ("tableextension", "Extends an existing table with non-invasive fields and indexes."),
        ("tablelogic", "Companion business logic, triggers, and field validation rules."),
        ("tablelogicextension", "Extends companion table logic non-invasively."),
        ("codeunit", "Encapsulated business procedures and domain services in QuickJS."),
        ("interface", "Erased type contract for polymorphism across codeunits."),
        ("enum", "Extensible type-safe business enumeration."),
        ("enumextension", "Non-invasively appends new enum values."),
        ("statemachine", "Deterministic finite-state machine governing entity lifecycle."),
        ("statemachineextension", "Appends new states and transitions to existing workflow."),
        ("page", "Declarative enterprise UI layout (Card, List, RoleCenter)."),
        ("pageextension", "Injects groups, fields, or actions into an existing page layout."),
        ("permissionset", "Fine-grained role-based RIMD permissions and field exclusions."),
        ("view", "Declarative database view with RLS multi-tenant alignment."),
        ("query", "High-performance analytical join across multiple dataitems."),
        ("menu", "Role-tailored navigation catalog and action menus."),
        ("report", "Enterprise dataitem reporting definition and column layout."),
        ("custommetadata", "Structured metadata configuration store."),
        ("calcFormula", "SIFT FlowField aggregation formula (sum, count, avg, min, max, lookup, exist)."),
        ("onInsert", "Pre-insert trigger before record is committed to PostgreSQL."),
        ("onModify", "Pre-modify trigger validating changes against xRec."),
        ("onDelete", "Pre-delete trigger enforcing referential integrity."),
    ];

    let items: Vec<Value> = keywords
        .iter()
        .map(|(label, doc)| {
            serde_json::json!({
                "label": label,
                "kind": 14, // Keyword
                "detail": "BOS Meta-Object",
                "documentation": {
                    "kind": "markdown",
                    "value": doc
                },
                "insertText": label
            })
        })
        .collect();

    serde_json::json!({
        "isIncomplete": false,
        "items": items
    })
}

fn get_hover(params: &Option<Value>, docs: &HashMap<String, String>) -> Value {
    let word = params
        .as_ref()
        .and_then(|p| p.get("position"))
        .and_then(|pos| {
            let line = pos.get("line")?.as_u64()? as usize;
            let character = pos.get("character")?.as_u64()? as usize;
            let uri = params.as_ref()?.get("textDocument")?.get("uri")?.as_str()?;
            let content = docs.get(uri)?;
            let line_str = content.lines().nth(line)?;
            extract_word_at_col(line_str, character)
        })
        .unwrap_or_default();

    let doc = match word.as_str() {
        "table" => "**BOS `table` Meta-Object**\n\nDeclares a persistent relational database table with automatic multi-tenant Row-Level Security (RLS), SIFT indexing, and declarative FlowFields.",
        "statemachine" => "**BOS `statemachine` Meta-Object**\n\nDefines a deterministic finite-state workflow engine governing entity status transitions, guards, and approval role matrices.",
        "codeunit" => "**BOS `codeunit` Meta-Object**\n\nEncapsulated domain service procedures compiled AOT into QuickJS bytecode (`.qbc`) for zero-overhead sandboxed execution.",
        "tablelogic" => "**BOS `tablelogic` Meta-Object**\n\nCompanion logic executing pre/post database triggers (`onInsert`, `onModify`, `onDelete`) and field-level validation routines.",
        "permissionset" => "**BOS `permissionset` Meta-Object**\n\nDefines granular RIMD table permissions, row-level filters (`where (...)`), and field-level exclusions (`exclude: [creditLimit]`).",
        "page" => "**BOS `page` Meta-Object**\n\nDeclarative enterprise UI layout compiled directly into high-speed Svelte 5 runes (`$state`, `$props`).",
        "calcFormula" => "**SIFT `calcFormula` Property**\n\nDefines an aggregated FlowField computed at database read time with PostgreSQL covering index pushdown.",
        _ => return Value::Null,
    };

    serde_json::json!({
        "contents": {
            "kind": "markdown",
            "value": doc
        }
    })
}

fn extract_word_at_col(line: &str, col: usize) -> Option<String> {
    if col >= line.len() {
        return None;
    }
    let chars: Vec<char> = line.chars().collect();
    if col >= chars.len() || !chars[col].is_alphanumeric() && chars[col] != '_' {
        return None;
    }
    let mut start = col;
    while start > 0 && (chars[start - 1].is_alphanumeric() || chars[start - 1] == '_') {
        start -= 1;
    }
    let mut end = col;
    while end < chars.len() && (chars[end].is_alphanumeric() || chars[end] == '_') {
        end += 1;
    }
    Some(chars[start..end].iter().collect())
}

fn get_document_symbols(params: &Option<Value>, docs: &HashMap<String, String>) -> Value {
    let uri = match params
        .as_ref()
        .and_then(|p| p.get("textDocument"))
        .and_then(|td| td.get("uri"))
        .and_then(|u| u.as_str())
    {
        Some(u) => u,
        None => return serde_json::json!([]),
    };

    let text = match docs.get(uri) {
        Some(t) => t,
        None => return serde_json::json!([]),
    };

    let mut symbols = Vec::new();
    let mut lexer = Lexer::new(text);
    if let Ok(tokens) = lexer.tokenize() {
        let mut parser = Parser::new(tokens);
        if let Ok(program) = parser.parse_program() {
            for item in program.items {
            let (name, kind, span) = match item {
                Item::Table(t) => (t.name, 5, t.span),             // Class
                Item::TableExtension(te) => (te.name, 5, te.span), // Class
                Item::TableLogic(tl) => (tl.target_table, 12, tl.span), // Function
                Item::TableLogicExtension(tle) => (tle.name, 12, tle.span),
                Item::Codeunit(cu) => (cu.name, 12, cu.span),      // Function
                Item::Interface(i) => (i.name, 11, i.span),        // Interface
                Item::Enum(e) => (e.name, 10, e.span),             // Enum
                Item::EnumExtension(ee) => (ee.name, 10, ee.span), // Enum
                Item::StateMachine(sm) => (sm.name, 5, sm.span),   // Class
                Item::StateMachineExtension(sme) => (sme.name, 5, sme.span),
                Item::Page(p) => (p.name, 7, p.span),              // Property
                Item::PageExtension(pe) => (pe.name, 7, pe.span),  // Property
                Item::PermissionSet(ps) => (ps.name, 13, ps.span), // Variable
                Item::View(v) => (v.name, 5, v.span),              // Class
                Item::Query(q) => (q.name, 5, q.span),             // Class
                Item::Menu(m) => (m.name, 13, m.span),
                Item::Report(r) => (r.name, 5, r.span),
                Item::CustomMetadata(cm) => (cm.name, 13, cm.span),
            };

            let zero_line = span.line.saturating_sub(1);
            let zero_col = span.column.saturating_sub(1);

            symbols.push(serde_json::json!({
                "name": name,
                "kind": kind,
                "range": {
                    "start": { "line": zero_line, "character": zero_col },
                    "end": { "line": zero_line, "character": zero_col + name.len() }
                },
                "selectionRange": {
                    "start": { "line": zero_line, "character": zero_col },
                    "end": { "line": zero_line, "character": zero_col + name.len() }
                }
            }));
        }
    }
}

    serde_json::json!(symbols)
}
