//! Oxc parse of a procedure body.
//!
//! Every script is parsed as TypeScript. A syntax error becomes a compiler
//! diagnostic. Printing is left to the line-preserving eraser: Oxc's printer
//! does not keep the original line breaks, and runtime stacks use those lines.

use std::path::Path;

use bosc_ir::CompilerDiagnosticIR;
use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::SourceType;

pub fn typescript_syntax_diagnostics(source: &str, base_line: usize) -> Vec<CompilerDiagnosticIR> {
    let wrapped = format!("function __bosScript() {{\n{source}\n}}");
    let allocator = Allocator::default();
    let source_type = SourceType::script().with_typescript(true);
    let parsed = Parser::new(&allocator, &wrapped, source_type).parse();
    if parsed.diagnostics.is_empty() {
        return Vec::new();
    }

    parsed
        .diagnostics
        .iter()
        .take(8)
        .filter_map(|diag| {
            let offset = diag.labels.first().map(|label| label.offset()).unwrap_or(0);
            let (line, column) = line_col(&wrapped, offset as usize);
            // Line 1 is the wrapper. Diagnostics on the wrapper itself are ignored.
            if line <= 1 {
                return None;
            }
            let line = base_line + line - 2;
            Some(CompilerDiagnosticIR {
                severity: "error".to_string(),
                code: "BOS1001".to_string(),
                message: format!("TypeScript syntax: {diag}"),
                line,
                column,
                end_line: line,
                end_column: column.saturating_add(1),
                hint: Some(
                    "Procedure bodies are TypeScript. Fix the syntax before Apply; types are erased after this check."
                        .to_string(),
                ),
            })
        })
        .collect()
}

fn line_col(source: &str, offset: usize) -> (usize, usize) {
    let mut line = 1usize;
    let mut col = 1usize;
    for (i, ch) in source.char_indices() {
        if i >= offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }
    (line, col)
}

/// Confirm the TypeScript transform pipeline is the one Oxc ships.
/// Used by tests so a parser API drift fails here rather than at runtime.
pub fn oxc_strips_types_in_a_function() -> bool {
    let allocator = Allocator::default();
    let source = "function f(x: number): string { return String(x); }";
    let source_type = SourceType::script().with_typescript(true);
    let mut parsed = Parser::new(&allocator, source, source_type).parse();
    if !parsed.diagnostics.is_empty() {
        return false;
    }
    let options = oxc_transformer::TransformOptions::default();
    let semantic = oxc_semantic::SemanticBuilder::new()
        .with_check_syntax_error(true)
        .build(&parsed.program);
    if !semantic.diagnostics.is_empty() {
        return false;
    }
    let _ = oxc_transformer::Transformer::new(&allocator, Path::new("script.ts"), &options)
        .build_with_scoping(semantic.semantic.into_scoping(), &mut parsed.program);
    let printed = oxc_codegen::Codegen::new().build(&parsed.program).code;
    printed.contains("function f(x)") && !printed.contains(": number") && !printed.contains(": string")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn oxc_erases_a_typed_function() {
        assert!(oxc_strips_types_in_a_function());
    }

    #[test]
    fn oxc_reports_a_broken_body() {
        let diags = typescript_syntax_diagnostics("const x = ;\n", 10);
        assert!(!diags.is_empty(), "expected a syntax diagnostic");
        assert_eq!(diags[0].code, "BOS1001");
        assert!(diags[0].line >= 10, "line mapped back onto the procedure: {}", diags[0].line);
    }

    #[test]
    fn oxc_accepts_typed_business_script() {
        let src = "const item: ItemRecord = Item.get(rec.itemNo);\nif (!item) throw new Error(\"missing\");\n";
        let diags = typescript_syntax_diagnostics(src, 4);
        assert!(diags.is_empty(), "{diags:?}");
    }
}
