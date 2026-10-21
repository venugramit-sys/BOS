use crate::ast::*;
use crate::token::{Span, Token, TokenKind};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ParserError {
    #[error("Expected {expected}, found {found:?} at line {line}, col {col}")]
    UnexpectedToken {
        expected: String,
        found: TokenKind,
        line: usize,
        col: usize,
    },
    #[error("Unexpected end of file")]
    UnexpectedEof,
}

impl ParserError {
    pub fn line(&self) -> usize {
        match self {
            Self::UnexpectedToken { line, .. } => *line,
            Self::UnexpectedEof => 1,
        }
    }

    pub fn column(&self) -> usize {
        match self {
            Self::UnexpectedToken { col, .. } => *col,
            Self::UnexpectedEof => 1,
        }
    }

    pub fn hint(&self) -> Option<String> {
        match self {
            Self::UnexpectedToken { expected, found, .. } => {
                if expected.contains("Semicolon") {
                    Some("Add a semicolon ';' at the end of the declaration or statement.".to_string())
                } else if expected.contains("CloseBrace") || matches!(found, TokenKind::Eof) {
                    Some("Check for an unclosed '{' block.".to_string())
                } else if expected.contains("OpenBrace") {
                    Some("Open the declaration body with '{'.".to_string())
                } else {
                    Some(format!("Expected {} here.", expected))
                }
            }
            Self::UnexpectedEof => Some("Ensure all '{' blocks and parentheses are closed.".to_string()),
        }
    }
}

pub struct Parser {
    tokens: Vec<Token>,
    cursor: usize,
    source: Option<String>,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, cursor: 0, source: None }
    }

    pub fn with_source(tokens: Vec<Token>, source: String) -> Self {
        Self { tokens, cursor: 0, source: Some(source) }
    }

    fn peek(&self) -> &Token {
        self.tokens.get(self.cursor).unwrap_or(&self.tokens[self.tokens.len() - 1])
    }

    fn peek_kind(&self) -> &TokenKind {
        &self.peek().kind
    }

    fn advance(&mut self) -> Token {
        if self.cursor < self.tokens.len() {
            let tok = self.tokens[self.cursor].clone();
            self.cursor += 1;
            tok
        } else {
            self.tokens[self.tokens.len() - 1].clone()
        }
    }

    fn expect(&mut self, expected_kind: TokenKind) -> Result<Token, ParserError> {
        let current = self.peek();
        if std::mem::discriminant(&current.kind) == std::mem::discriminant(&expected_kind) {
            Ok(self.advance())
        } else {
            Err(ParserError::UnexpectedToken {
                expected: format!("{:?}", expected_kind),
                found: current.kind.clone(),
                line: current.span.line,
                col: current.span.column,
            })
        }
    }

    fn peek_ident(&self) -> Option<String> {
        match &self.peek().kind {
            TokenKind::Ident(s) => Some(s.clone()),
            TokenKind::State => Some("state".to_string()),
            TokenKind::From => Some("from".to_string()),
            TokenKind::Select => Some("select".to_string()),
            TokenKind::Elements => Some("elements".to_string()),
            TokenKind::Dataitem => Some("dataitem".to_string()),
            TokenKind::Guard => Some("guard".to_string()),
            TokenKind::Tier => Some("tier".to_string()),
            TokenKind::Params => Some("params".to_string()),
            TokenKind::Timeout => Some("timeout".to_string()),
            TokenKind::EscalateTo => Some("escalateTo".to_string()),
            TokenKind::Where => Some("where".to_string()),
            TokenKind::Relation => Some("relation".to_string()),
            TokenKind::Computed => Some("computed".to_string()),
            TokenKind::Key => Some("key".to_string()),
            TokenKind::In => Some("in".to_string()),
            TokenKind::Table => Some("table".to_string()),
            TokenKind::Page => Some("page".to_string()),
            TokenKind::View => Some("view".to_string()),
            TokenKind::Query => Some("query".to_string()),
            TokenKind::Report => Some("report".to_string()),
            TokenKind::Enum => Some("enum".to_string()),
            TokenKind::OrderBy => Some("orderBy".to_string()),
            TokenKind::GroupBy => Some("groupBy".to_string()),
            TokenKind::Having => Some("having".to_string()),
            TokenKind::Transition => Some("transition".to_string()),
            TokenKind::Codeunit => Some("codeunit".to_string()),
            TokenKind::Interface => Some("interface".to_string()),
            TokenKind::For => Some("for".to_string()),
            TokenKind::PrimaryKey => Some("primaryKey".to_string()),
            TokenKind::FlowFilter => Some("flowFilter".to_string()),
            TokenKind::ApprovalMatrix => Some("approvalMatrix".to_string()),
            TokenKind::Extends => Some("extends".to_string()),
            TokenKind::Implements => Some("implements".to_string()),
            TokenKind::Public => Some("public".to_string()),
            TokenKind::Private => Some("private".to_string()),
            TokenKind::Static => Some("static".to_string()),
            TokenKind::Function => Some("function".to_string()),
            TokenKind::Null => Some("null".to_string()),
            TokenKind::True => Some("true".to_string()),
            TokenKind::False => Some("false".to_string()),
            _ => None,
        }
    }

    fn expect_ident(&mut self) -> Result<(String, Span), ParserError> {
        if let Some(s) = self.peek_ident() {
            let tok = self.advance();
            Ok((s, tok.span))
        } else {
            let tok = self.peek().clone();
            Err(ParserError::UnexpectedToken {
                expected: "identifier".to_string(),
                found: tok.kind,
                line: tok.span.line,
                col: tok.span.column,
            })
        }
    }

    pub fn parse_program(&mut self) -> Result<Program, ParserError> {
        let start_span = self.peek().span.clone();
        let mut items = Vec::new();

        while self.peek_kind() != &TokenKind::Eof {
            items.push(self.parse_item()?);
        }

        let end_span = self.peek().span.clone();
        Ok(Program {
            items,
            span: Span::new(start_span.start, end_span.end, start_span.line, start_span.column),
        })
    }

    fn parse_item(&mut self) -> Result<Item, ParserError> {
        let mut annotations = Vec::new();
        while self.peek_kind() == &TokenKind::At {
            annotations.push(self.parse_annotation()?);
        }

        match self.peek_kind() {
            TokenKind::Table => Ok(Item::Table(self.parse_table()?)),
            TokenKind::TableLogic => Ok(Item::TableLogic(self.parse_table_logic()?)),
            TokenKind::TableExtension => Ok(Item::TableExtension(self.parse_table_ext()?)),
            TokenKind::TableLogicExtension => Ok(Item::TableLogicExtension(self.parse_table_logic_ext()?)),
            TokenKind::StateMachine => Ok(Item::StateMachine(self.parse_state_machine()?)),
            TokenKind::StateMachineExtension => Ok(Item::StateMachineExtension(self.parse_state_machine_ext()?)),
            TokenKind::Codeunit => Ok(Item::Codeunit(self.parse_codeunit_with_annotations(annotations)?)),
            TokenKind::Interface => Ok(Item::Interface(self.parse_interface()?)),
            TokenKind::Enum => Ok(Item::Enum(self.parse_enum()?)),
            TokenKind::EnumExtension => Ok(Item::EnumExtension(self.parse_enum_ext()?)),
            TokenKind::PermissionSet => Ok(Item::PermissionSet(self.parse_permission_set()?)),
            TokenKind::View => Ok(Item::View(self.parse_view()?)),
            TokenKind::Query => Ok(Item::Query(self.parse_query()?)),
            TokenKind::Page => Ok(Item::Page(self.parse_page()?)),
            TokenKind::PageExtension => Ok(Item::PageExtension(self.parse_page_ext()?)),
            TokenKind::Ident(s) if s == "menu" => Ok(Item::Menu(self.parse_menu()?)),
            TokenKind::Report => Ok(Item::Report(self.parse_report()?)),
            TokenKind::CustomMetadata => Ok(Item::CustomMetadata(self.parse_custom_metadata()?)),
            _ => Err(ParserError::UnexpectedToken {
                expected: "meta-object declaration (table, tablelogic, tableextension, tablelogicextension, statemachine, codeunit, interface, enum, enumextension, page, pageextension, menu, permissionset, view, query, report, or custommetadata)".to_string(),
                found: self.peek_kind().clone(),
                line: self.peek().span.line,
                col: self.peek().span.column,
            }),
        }
    }

    // ----------------------------------------------------
    // PARSE TABLE
    // ----------------------------------------------------
    fn parse_field_group(&mut self) -> Result<FieldGroupDef, ParserError> {
        let (_, fg_span) = self.expect_ident()?;
        let group_name = if self.peek_kind() == &TokenKind::OpenParen {
            self.advance();
            let (gn, _) = self.expect_ident()?;
            if self.peek_kind() == &TokenKind::Semicolon || self.peek_kind() == &TokenKind::Comma {
                self.advance();
            }
            gn
        } else {
            let (gn, _) = self.expect_ident()?;
            self.expect(TokenKind::OpenParen)?;
            gn
        };
        let mut fg_fields = Vec::new();
        while self.peek_kind() != &TokenKind::CloseParen && self.peek_kind() != &TokenKind::Eof {
            let (f_name, _) = self.expect_ident()?;
            fg_fields.push(f_name);
            if self.peek_kind() == &TokenKind::Comma || self.peek_kind() == &TokenKind::Semicolon {
                self.advance();
            }
        }
        let end_tok = self.expect(TokenKind::CloseParen)?;
        if self.peek_kind() == &TokenKind::Semicolon {
            self.advance();
        }
        Ok(FieldGroupDef {
            name: group_name,
            fields: fg_fields,
            span: Span::new(fg_span.start, end_tok.span.end, fg_span.line, fg_span.column),
        })
    }

    fn parse_table(&mut self) -> Result<TableDef, ParserError> {
        let start_tok = self.expect(TokenKind::Table)?;
        let (name, _) = self.expect_ident()?;
        self.expect(TokenKind::OpenBrace)?;

        let mut properties = Vec::new();
        let mut fields = Vec::new();
        let mut primary_key = Vec::new();
        let mut keys = Vec::new();
        let mut field_groups = Vec::new();
        let mut triggers = Vec::new();

        while self.peek_kind() != &TokenKind::CloseBrace && self.peek_kind() != &TokenKind::Eof {
            match self.peek_kind() {
                TokenKind::PrimaryKey => {
                    self.advance();
                    self.expect(TokenKind::OpenParen)?;
                    while self.peek_kind() != &TokenKind::CloseParen {
                        let (pk_col, _) = self.expect_ident()?;
                        primary_key.push(pk_col);
                        if self.peek_kind() == &TokenKind::Comma {
                            self.advance();
                        }
                    }
                    self.expect(TokenKind::CloseParen)?;
                    self.expect(TokenKind::Semicolon)?;
                }
                TokenKind::Key => {
                    let key_span = self.advance().span;
                    self.expect(TokenKind::OpenParen)?;
                    let mut key_cols = Vec::new();
                    while self.peek_kind() != &TokenKind::CloseParen {
                        let (k_col, _) = self.expect_ident()?;
                        key_cols.push(k_col);
                        if self.peek_kind() == &TokenKind::Comma {
                            self.advance();
                        }
                    }
                    self.expect(TokenKind::CloseParen)?;
                    if self.peek_kind() == &TokenKind::Semicolon {
                        self.advance();
                    }
                    keys.push(KeyDef {
                        fields: key_cols,
                        unique: false,
                        span: key_span,
                    });
                }
                TokenKind::Ident(s)
                    if s == "fieldgroup"
                        && self.peek_second_kind() != Some(&TokenKind::Colon)
                        && self.peek_second_kind() != Some(&TokenKind::Assign) =>
                {
                    field_groups.push(self.parse_field_group()?);
                }
                TokenKind::Ident(s) if s == "fieldgroups" && self.peek_second_kind() == Some(&TokenKind::OpenBrace) => {
                    self.advance();
                    self.expect(TokenKind::OpenBrace)?;
                    while self.peek_kind() != &TokenKind::CloseBrace && self.peek_kind() != &TokenKind::Eof {
                        if let TokenKind::Ident(inner) = self.peek_kind() {
                            if inner == "fieldgroup" {
                                field_groups.push(self.parse_field_group()?);
                                continue;
                            }
                        }
                        self.advance();
                    }
                    self.expect(TokenKind::CloseBrace)?;
                    if self.peek_kind() == &TokenKind::Semicolon {
                        self.advance();
                    }
                }
                TokenKind::Ident(s) if (s.starts_with("on") || s.starts_with("before") || s.starts_with("after")) && self.peek_second_kind() == Some(&TokenKind::OpenParen) => {
                    // Trigger
                    triggers.push(self.parse_trigger()?);
                }
                TokenKind::Ident(s) => {
                    let is_table_prop = matches!(
                        s.as_str(),
                        "tableType"
                            | "table_type"
                            | "extensible"
                            | "dataPerCompany"
                            | "data_per_company"
                            | "auditTrail"
                            | "audit_trail"
                            | "ledger"
                    );
                    if self.peek_second_kind() == Some(&TokenKind::Assign)
                        || (is_table_prop && self.peek_second_kind() == Some(&TokenKind::Colon))
                    {
                        properties.push(self.parse_property()?);
                    } else if self.peek_second_kind() == Some(&TokenKind::Colon) {
                        fields.push(self.parse_field()?);
                    } else {
                        return Err(ParserError::UnexpectedToken {
                            expected: "= for property or : for field".to_string(),
                            found: self.peek_kind().clone(),
                            line: self.peek().span.line,
                            col: self.peek().span.column,
                        });
                    }
                }
                _ => {
                    return Err(ParserError::UnexpectedToken {
                        expected: "field, property, key, fieldgroup, or trigger".to_string(),
                        found: self.peek_kind().clone(),
                        line: self.peek().span.line,
                        col: self.peek().span.column,
                    });
                }
            }
        }

        let end_tok = self.expect(TokenKind::CloseBrace)?;
        Ok(TableDef {
            name,
            properties,
            fields,
            primary_key,
            keys,
            field_groups,
            triggers,
            span: Span::new(start_tok.span.start, end_tok.span.end, start_tok.span.line, start_tok.span.column),
        })
    }

    fn peek_second_kind(&self) -> Option<&TokenKind> {
        self.tokens.get(self.cursor + 1).map(|t| &t.kind)
    }

    fn parse_table_ext(&mut self) -> Result<TableExtDef, ParserError> {
        let start_tok = self.expect(TokenKind::TableExtension)?;
        let (name, _) = self.expect_ident()?;
        self.expect(TokenKind::Extends)?;
        let (target_table, _) = self.expect_ident()?;
        self.expect(TokenKind::OpenBrace)?;

        let mut fields = Vec::new();
        let mut keys = Vec::new();
        let mut triggers = Vec::new();

        while self.peek_kind() != &TokenKind::CloseBrace && self.peek_kind() != &TokenKind::Eof {
            match self.peek_kind() {
                TokenKind::Key => {
                    let key_span = self.advance().span;
                    self.expect(TokenKind::OpenParen)?;
                    let mut key_cols = Vec::new();
                    while self.peek_kind() != &TokenKind::CloseParen {
                        let (k_col, _) = self.expect_ident()?;
                        key_cols.push(k_col);
                        if self.peek_kind() == &TokenKind::Comma {
                            self.advance();
                        }
                    }
                    self.expect(TokenKind::CloseParen)?;
                    self.expect(TokenKind::Semicolon)?;
                    keys.push(KeyDef {
                        fields: key_cols,
                        unique: false,
                        span: key_span,
                    });
                }
                TokenKind::Ident(s) if s.starts_with("on") => {
                    triggers.push(self.parse_trigger()?);
                }
                TokenKind::Ident(_) => {
                    fields.push(self.parse_field()?);
                }
                _ => {
                    return Err(ParserError::UnexpectedToken {
                        expected: "field, key, or trigger in table extension".to_string(),
                        found: self.peek_kind().clone(),
                        line: self.peek().span.line,
                        col: self.peek().span.column,
                    });
                }
            }
        }

        let end_tok = self.expect(TokenKind::CloseBrace)?;
        Ok(TableExtDef {
            name,
            target_table,
            fields,
            keys,
            triggers,
            span: Span::new(start_tok.span.start, end_tok.span.end, start_tok.span.line, start_tok.span.column),
        })
    }

    // ----------------------------------------------------
    // PARSE TABLE LOGIC & TABLE LOGIC EXTENSION
    // ----------------------------------------------------
    fn parse_table_logic(&mut self) -> Result<TableLogicDef, ParserError> {
        let start_tok = self.expect(TokenKind::TableLogic)?;
        let (name, _) = self.expect_ident()?;
        let target_table = if self.peek_kind() == &TokenKind::For {
            self.advance();
            let (target, _) = self.expect_ident()?;
            target
        } else {
            name.clone()
        };
        self.expect(TokenKind::OpenBrace)?;

        let mut field_validations = Vec::new();
        let mut triggers = Vec::new();
        let mut methods = Vec::new();

        while self.peek_kind() != &TokenKind::CloseBrace && self.peek_kind() != &TokenKind::Eof {
            if let TokenKind::Ident(s) = self.peek_kind().clone() {
                if s == "validate" {
                    self.advance();
                    let (field_name, val_span) = self.expect_ident()?;
                    self.expect(TokenKind::OpenParen)?;
                    let (param_name, _) = self.expect_ident()?;
                    if self.peek_kind() == &TokenKind::Colon {
                        self.advance();
                        self.parse_type_annotation_str()?;
                    }
                    let old_param_name = if self.peek_kind() == &TokenKind::Comma {
                        self.advance();
                        let (old_p, _) = self.expect_ident()?;
                        if self.peek_kind() == &TokenKind::Colon {
                            self.advance();
                            self.parse_type_annotation_str()?;
                        }
                        Some(old_p)
                    } else {
                        None
                    };
                    self.expect(TokenKind::CloseParen)?;
                    let body = self.parse_block()?;
                    field_validations.push(FieldValidationDef {
                        field_name,
                        param_name,
                        old_param_name,
                        body,
                        span: val_span,
                    });
                    continue;
                }
            }

            if self.peek_kind() == &TokenKind::Public
                || self.peek_kind() == &TokenKind::Private
                || self.peek_kind() == &TokenKind::Static
                || self.peek_kind() == &TokenKind::Function
            {
                let method = self.parse_method(Vec::new())?;
                methods.push(method);
            } else if let TokenKind::Ident(s) = self.peek_kind().clone() {
                if s == "async" {
                    let method = self.parse_method(Vec::new())?;
                    methods.push(method);
                } else if s.starts_with("on") || s.starts_with("before") || s.starts_with("after") {
                    triggers.push(self.parse_trigger()?);
                } else if self.peek_second_kind() == Some(&TokenKind::OpenParen) {
                    let method = self.parse_method(Vec::new())?;
                    methods.push(method);
                } else {
                    self.advance();
                }
            } else {
                self.advance();
            }
        }

        let end_tok = self.expect(TokenKind::CloseBrace)?;
        Ok(TableLogicDef {
            name,
            target_table,
            field_validations,
            triggers,
            methods,
            span: Span::new(start_tok.span.start, end_tok.span.end, start_tok.span.line, start_tok.span.column),
        })
    }

    fn parse_table_logic_ext(&mut self) -> Result<TableLogicExtDef, ParserError> {
        let start_tok = self.expect(TokenKind::TableLogicExtension)?;
        let (name, _) = self.expect_ident()?;
        if self.peek_kind() == &TokenKind::Extends {
            self.advance();
        } else {
            self.expect(TokenKind::For)?;
        }
        let (target_table, _) = self.expect_ident()?;
        self.expect(TokenKind::OpenBrace)?;

        let mut field_validations = Vec::new();
        let mut triggers = Vec::new();
        let mut methods = Vec::new();

        while self.peek_kind() != &TokenKind::CloseBrace && self.peek_kind() != &TokenKind::Eof {
            if let TokenKind::Ident(s) = self.peek_kind().clone() {
                if s == "validate" {
                    self.advance();
                    let (field_name, val_span) = self.expect_ident()?;
                    self.expect(TokenKind::OpenParen)?;
                    let (param_name, _) = self.expect_ident()?;
                    if self.peek_kind() == &TokenKind::Colon {
                        self.advance();
                        self.parse_type_annotation_str()?;
                    }
                    let old_param_name = if self.peek_kind() == &TokenKind::Comma {
                        self.advance();
                        let (old_p, _) = self.expect_ident()?;
                        if self.peek_kind() == &TokenKind::Colon {
                            self.advance();
                            self.parse_type_annotation_str()?;
                        }
                        Some(old_p)
                    } else {
                        None
                    };
                    self.expect(TokenKind::CloseParen)?;
                    let body = self.parse_block()?;
                    field_validations.push(FieldValidationDef {
                        field_name,
                        param_name,
                        old_param_name,
                        body,
                        span: val_span,
                    });
                    continue;
                }
            }

            if self.peek_kind() == &TokenKind::Public
                || self.peek_kind() == &TokenKind::Private
                || self.peek_kind() == &TokenKind::Static
                || self.peek_kind() == &TokenKind::Function
            {
                let method = self.parse_method(Vec::new())?;
                methods.push(method);
            } else if let TokenKind::Ident(s) = self.peek_kind().clone() {
                if s == "async" {
                    let method = self.parse_method(Vec::new())?;
                    methods.push(method);
                } else if s.starts_with("on") || s.starts_with("before") || s.starts_with("after") {
                    triggers.push(self.parse_trigger()?);
                } else if self.peek_second_kind() == Some(&TokenKind::OpenParen) {
                    let method = self.parse_method(Vec::new())?;
                    methods.push(method);
                } else {
                    self.advance();
                }
            } else {
                self.advance();
            }
        }

        let end_tok = self.expect(TokenKind::CloseBrace)?;
        Ok(TableLogicExtDef {
            name,
            target_table,
            field_validations,
            triggers,
            methods,
            span: Span::new(start_tok.span.start, end_tok.span.end, start_tok.span.line, start_tok.span.column),
        })
    }

    fn parse_property(&mut self) -> Result<Property, ParserError> {
        let (name, start_span) = self.expect_ident()?;
        if self.peek_kind() == &TokenKind::Colon {
            self.advance();
        } else {
            self.expect(TokenKind::Assign)?;
        }
        let mut value = self.parse_expr()?;
        if matches!(self.peek_kind(), TokenKind::Ident(s) if s.eq_ignore_ascii_case("asc") || s.eq_ignore_ascii_case("desc")) {
            let dir = match &self.advance().kind {
                TokenKind::Ident(s) if s.eq_ignore_ascii_case("asc") => "asc",
                _ => "desc",
            };
            let base = match &value {
                Expr::Ident(s) | Expr::StringLit(s) => s.clone(),
                Expr::MemberAccess { object, member } => match &**object {
                    Expr::Ident(o) => format!("{}.{}", o, member),
                    _ => member.clone(),
                },
                _ => String::new(),
            };
            value = Expr::StringLit(format!("{} {}", base, dir));
        }
        if self.peek_kind() == &TokenKind::Semicolon {
            self.advance();
        }
        Ok(Property {
            name,
            value,
            span: start_span,
        })
    }

    fn parse_field(&mut self) -> Result<FieldDef, ParserError> {
        let (name, start_span) = self.expect_ident()?;
        self.expect(TokenKind::Colon)?;
        let field_type = self.parse_field_type()?;

        let mut properties = Vec::new();
        let mut computed = None;
        let mut validate_block = None;

        // Parse optional inline modifiers before `{`, `=`, or `;` (e.g. `notNull`, `unique`, `default("x")`, `flowField(...)`)
        loop {
            match self.peek_kind().clone() {
                TokenKind::Computed => {
                    self.advance();
                    if self.peek_kind() == &TokenKind::OpenParen {
                        self.advance();
                        computed = Some(self.parse_computed_formula()?);
                        self.expect(TokenKind::CloseParen)?;
                    } else if self.peek_kind() == &TokenKind::Assign || self.peek_kind() == &TokenKind::Colon {
                        self.advance();
                        computed = Some(self.parse_computed_formula()?);
                    }
                }
                TokenKind::Ident(mod_name) => {
                    match mod_name.as_str() {
                        "notNull" | "not_null" => {
                            let sp = self.advance().span;
                            properties.push(Property {
                                name: "notNull".to_string(),
                                value: Expr::BoolLit(true),
                                span: sp,
                            });
                        }
                        "unique" => {
                            let sp = self.advance().span;
                            properties.push(Property {
                                name: "unique".to_string(),
                                value: Expr::BoolLit(true),
                                span: sp,
                            });
                        }
                        "default" | "caption" | "editable" | "visible" | "check" | "formula" | "relation" => {
                            let sp = self.advance().span;
                            let val = if self.peek_kind() == &TokenKind::OpenParen {
                                self.advance();
                                let v = self.parse_expr()?;
                                self.expect(TokenKind::CloseParen)?;
                                v
                            } else if self.peek_kind() == &TokenKind::Assign || self.peek_kind() == &TokenKind::Colon {
                                self.advance();
                                self.parse_expr()?
                            } else {
                                Expr::BoolLit(true)
                            };
                            properties.push(Property {
                                name: mod_name,
                                value: val,
                                span: sp,
                            });
                        }
                        "flowField" | "flow_field" | "computed" => {
                            self.advance();
                            if self.peek_kind() == &TokenKind::OpenParen {
                                self.advance();
                                computed = Some(self.parse_computed_formula()?);
                                self.expect(TokenKind::CloseParen)?;
                            } else if self.peek_kind() == &TokenKind::Assign || self.peek_kind() == &TokenKind::Colon {
                                self.advance();
                                computed = Some(self.parse_computed_formula()?);
                            }
                        }
                        _ => break,
                    }
                }
                _ => break,
            }
        }

        // Check if field has body block or default assignment
        if self.peek_kind() == &TokenKind::Assign {
            self.advance();
            let default_val = self.parse_expr()?;
            properties.push(Property {
                name: "default".to_string(),
                value: default_val,
                span: start_span.clone(),
            });
            if self.peek_kind() == &TokenKind::Semicolon {
                self.advance();
            }
        } else if self.peek_kind() == &TokenKind::OpenBrace {
            self.advance();
            while self.peek_kind() != &TokenKind::CloseBrace && self.peek_kind() != &TokenKind::Eof {
                if self.peek_kind() == &TokenKind::Computed {
                    self.advance();
                    if self.peek_kind() == &TokenKind::Colon {
                        self.advance();
                    } else {
                        self.expect(TokenKind::Assign)?;
                    }
                    computed = Some(self.parse_computed_formula()?);
                    if self.peek_kind() == &TokenKind::Semicolon {
                        self.advance();
                    }
                } else if let TokenKind::Ident(prop_name) = self.peek_kind().clone() {
                    if prop_name == "computed" || prop_name == "flowField" {
                        self.advance();
                        if self.peek_kind() == &TokenKind::OpenParen {
                            self.advance();
                            computed = Some(self.parse_computed_formula()?);
                            self.expect(TokenKind::CloseParen)?;
                        } else {
                            if self.peek_kind() == &TokenKind::Colon {
                                self.advance();
                            } else {
                                self.expect(TokenKind::Assign)?;
                            }
                            computed = Some(self.parse_computed_formula()?);
                        }
                        if self.peek_kind() == &TokenKind::Semicolon {
                            self.advance();
                        }
                    } else if prop_name == "validate" {
                        self.advance();
                        self.expect(TokenKind::OpenParen)?;
                        let _param = self.expect_ident()?;
                        self.expect(TokenKind::CloseParen)?;
                        validate_block = Some(self.parse_block()?);
                    } else {
                        properties.push(self.parse_property()?);
                    }
                } else if self.peek_kind() == &TokenKind::Relation {
                    properties.push(self.parse_property()?);
                } else {
                    return Err(ParserError::UnexpectedToken {
                        expected: "field property".to_string(),
                        found: self.peek_kind().clone(),
                        line: self.peek().span.line,
                        col: self.peek().span.column,
                    });
                }
            }
            self.expect(TokenKind::CloseBrace)?;
            if self.peek_kind() == &TokenKind::Semicolon {
                self.advance();
            }
        } else {
            self.expect(TokenKind::Semicolon)?;
        }

        Ok(FieldDef {
            name,
            field_type,
            properties,
            computed,
            validate_block,
            span: start_span,
        })
    }

    fn parse_field_type(&mut self) -> Result<FieldType, ParserError> {
        if self.peek_kind() == &TokenKind::FlowFilter {
            self.advance();
            self.expect(TokenKind::Lt)?;
            let (inner_type, _) = self.expect_ident()?;
            self.expect(TokenKind::Gt)?;
            return Ok(FieldType::FlowFilter(inner_type));
        }

        let (type_name, _) = self.expect_ident()?;
        match type_name.as_str() {
            "UUID" | "Guid" => Ok(FieldType::UUID),
            "String" | "Code" | "Varchar" => {
                let length = if self.peek_kind() == &TokenKind::OpenParen {
                    self.advance();
                    let len = match self.advance().kind {
                        TokenKind::IntLit(n) => Some(n as usize),
                        _ => None,
                    };
                    self.expect(TokenKind::CloseParen)?;
                    len
                } else {
                    None
                };
                Ok(FieldType::String(length))
            }
            "Text" => Ok(FieldType::String(None)),
            "Integer" | "Int" => Ok(FieldType::Integer),
            "BigInt" | "BigInteger" => Ok(FieldType::BigInt),
            "Decimal" => {
                let mut p = 18;
                let mut s = 2;
                if self.peek_kind() == &TokenKind::OpenParen {
                    self.advance();
                    if let TokenKind::IntLit(precision) = self.advance().kind {
                        p = precision as usize;
                    }
                    self.expect(TokenKind::Comma)?;
                    if let TokenKind::IntLit(scale) = self.advance().kind {
                        s = scale as usize;
                    }
                    self.expect(TokenKind::CloseParen)?;
                }
                Ok(FieldType::Decimal(p, s))
            }
            "Boolean" | "Bool" => Ok(FieldType::Boolean),
            "Date" => Ok(FieldType::Date),
            "DateTime" => Ok(FieldType::DateTime),
            "Enum" => {
                let mut enum_type = "DefaultEnum".to_string();
                if self.peek_kind() == &TokenKind::OpenParen {
                    self.advance();
                    match self.advance().kind {
                        TokenKind::StringLit(s) | TokenKind::Ident(s) => {
                            enum_type = s;
                        }
                        _ => {}
                    }
                    self.expect(TokenKind::CloseParen)?;
                }
                Ok(FieldType::Enum(enum_type))
            }
            "Json" => Ok(FieldType::Json),
            "Blob" => Ok(FieldType::Blob),
            other => Ok(FieldType::Custom(other.to_string())),
        }
    }

    fn parse_computed_formula(&mut self) -> Result<ComputedFormula, ParserError> {
        let (func_name, start_span) = self.expect_ident()?;
        let func = match func_name.to_lowercase().as_str() {
            "sum" => AggregationFunc::Sum,
            "avg" | "average" => AggregationFunc::Average,
            "count" => AggregationFunc::Count,
            "min" => AggregationFunc::Min,
            "max" => AggregationFunc::Max,
            "lookup" => AggregationFunc::Lookup,
            "exist" => AggregationFunc::Exist,
            _ => return Err(ParserError::UnexpectedToken {
                expected: "sum, avg, count, min, max, lookup, or exist".to_string(),
                found: TokenKind::Ident(func_name),
                line: start_span.line,
                col: start_span.column,
            }),
        };

        self.expect(TokenKind::OpenParen)?;
        let (target_table, _) = self.expect_ident()?;
        let target_field = if self.peek_kind() == &TokenKind::Dot {
            self.advance();
            let (f, _) = self.expect_ident()?;
            Some(f)
        } else {
            None
        };

        self.expect(TokenKind::Where)?;
        let filter = if self.peek_kind() == &TokenKind::OpenParen {
            self.advance();
            let expr = self.parse_expr()?;
            self.expect(TokenKind::CloseParen)?;
            expr
        } else {
            self.expect(TokenKind::OpenBrace)?;
            let expr = self.parse_expr()?;
            self.expect(TokenKind::CloseBrace)?;
            expr
        };
        self.expect(TokenKind::CloseParen)?;

        Ok(ComputedFormula {
            func,
            target_table,
            target_field,
            filter,
            span: start_span,
        })
    }

    fn parse_trigger(&mut self) -> Result<TriggerDef, ParserError> {
        let (name, start_span) = self.expect_ident()?;
        let mut params = Vec::new();
        if self.peek_kind() == &TokenKind::OpenParen {
            self.advance();
            while self.peek_kind() != &TokenKind::CloseParen && self.peek_kind() != &TokenKind::Eof {
                let (p, _) = self.expect_ident()?;
                params.push(p);
                if self.peek_kind() == &TokenKind::Colon {
                    self.advance();
                    self.parse_type_annotation_str()?;
                }
                if self.peek_kind() == &TokenKind::Comma {
                    self.advance();
                }
            }
            self.expect(TokenKind::CloseParen)?;
        }
        if self.peek_kind() == &TokenKind::Colon {
            self.advance();
            self.parse_type_annotation_str()?;
        }
        let when_field = if let TokenKind::Ident(word) = self.peek_kind().clone() {
            if word == "when" {
                self.advance();
                Some(self.expect_ident()?.0)
            } else {
                None
            }
        } else {
            None
        };
        let mut body = self.parse_block()?;
        if let Some(field) = when_field {
            if !field.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                return Err(ParserError::UnexpectedToken {
                    expected: "field name after when".to_string(),
                    found: TokenKind::Ident(field),
                    line: start_span.line,
                    col: start_span.column,
                });
            }
            if let Some(raw) = body.raw_source.as_mut() {
                *raw = format!(
                    "if (String(rec.{field}) !== String(xRec == null ? rec.{field} : xRec.{field})) {{\n{raw}\n}}"
                );
            }
        }

        Ok(TriggerDef {
            name,
            params,
            body,
            span: start_span,
        })
    }

    // ----------------------------------------------------
    // PARSE STATE MACHINE
    // ----------------------------------------------------
    fn parse_state_machine(&mut self) -> Result<StateMachineDef, ParserError> {
        let start_tok = self.expect(TokenKind::StateMachine)?;
        let (name, _) = self.expect_ident()?;
        let (target_table, status_field) = if self.peek_kind() == &TokenKind::For {
            self.advance();
            let (tbl, _) = self.expect_ident()?;
            let sf = if self.peek_kind() == &TokenKind::Dot {
                self.advance();
                self.expect_ident()?.0
            } else {
                "status".to_string()
            };
            (tbl, sf)
        } else {
            (name.clone(), "status".to_string())
        };
        self.expect(TokenKind::OpenBrace)?;

        let mut properties = Vec::new();
        let mut states = Vec::new();
        let mut transitions = Vec::new();

        while self.peek_kind() != &TokenKind::CloseBrace && self.peek_kind() != &TokenKind::Eof {
            match self.peek_kind().clone() {
                TokenKind::State => {
                    states.push(self.parse_state()?);
                }
                TokenKind::Ident(s) if s == "initial" && self.peek_second_kind() == Some(&TokenKind::State) => {
                    let init_span = self.advance().span;
                    let mut st = self.parse_state()?;
                    st.properties.push(Property {
                        name: "initial".to_string(),
                        value: Expr::BoolLit(true),
                        span: init_span,
                    });
                    states.push(st);
                }
                TokenKind::Transition => {
                    transitions.push(self.parse_transition()?);
                }
                TokenKind::Ident(s) if s == "states" => {
                    self.advance();
                    if self.peek_kind() == &TokenKind::Colon || self.peek_kind() == &TokenKind::Assign {
                        self.advance();
                        self.expect(TokenKind::OpenBracket)?;
                        while self.peek_kind() != &TokenKind::CloseBracket && self.peek_kind() != &TokenKind::Eof {
                            let st_span = self.peek().span.clone();
                            let st_name = match self.peek_kind().clone() {
                                TokenKind::StringLit(lit) => {
                                    self.advance();
                                    lit
                                }
                                _ => self.expect_ident()?.0,
                            };
                            states.push(StateDef {
                                name: st_name,
                                properties: Vec::new(),
                                timeout: None,
                                span: st_span,
                            });
                            if self.peek_kind() == &TokenKind::Comma {
                                self.advance();
                            }
                        }
                        self.expect(TokenKind::CloseBracket)?;
                        if self.peek_kind() == &TokenKind::Semicolon {
                            self.advance();
                        }
                    } else {
                        self.expect(TokenKind::OpenBrace)?;
                        while self.peek_kind() != &TokenKind::CloseBrace && self.peek_kind() != &TokenKind::Eof {
                            states.push(self.parse_state()?);
                        }
                        self.expect(TokenKind::CloseBrace)?;
                    }
                }
                TokenKind::Ident(_) => {
                    let (prop_name, prop_span) = self.expect_ident()?;
                    if self.peek_kind() == &TokenKind::Colon || self.peek_kind() == &TokenKind::Assign {
                        self.advance();
                        let value = self.parse_expr()?;
                        if self.peek_kind() == &TokenKind::Semicolon {
                            self.advance();
                        }
                        let normalized_name = if prop_name == "initial" {
                            "initialState".to_string()
                        } else {
                            prop_name
                        };
                        properties.push(Property {
                            name: normalized_name,
                            value,
                            span: prop_span,
                        });
                    } else {
                        return Err(ParserError::UnexpectedToken {
                            expected: "'=' or ':' after state machine property".to_string(),
                            found: self.peek_kind().clone(),
                            line: self.peek().span.line,
                            col: self.peek().span.column,
                        });
                    }
                }
                _ => {
                    return Err(ParserError::UnexpectedToken {
                        expected: "state, transition, or property".to_string(),
                        found: self.peek_kind().clone(),
                        line: self.peek().span.line,
                        col: self.peek().span.column,
                    });
                }
            }
        }

        let end_tok = self.expect(TokenKind::CloseBrace)?;
        Ok(StateMachineDef {
            name,
            target_table,
            status_field,
            properties,
            states,
            transitions,
            span: Span::new(start_tok.span.start, end_tok.span.end, start_tok.span.line, start_tok.span.column),
        })
    }

    fn parse_state_machine_ext(&mut self) -> Result<StateMachineExtDef, ParserError> {
        let start_tok = self.expect(TokenKind::StateMachineExtension)?;
        let (name, _) = self.expect_ident()?;
        self.expect(TokenKind::Extends)?;
        let (target_sm, _) = self.expect_ident()?;
        self.expect(TokenKind::OpenBrace)?;

        let mut states = Vec::new();
        let mut transitions = Vec::new();

        while self.peek_kind() != &TokenKind::CloseBrace && self.peek_kind() != &TokenKind::Eof {
            match self.peek_kind() {
                TokenKind::State => states.push(self.parse_state()?),
                TokenKind::Transition => transitions.push(self.parse_transition()?),
                TokenKind::Ident(s) if s == "states" => {
                    self.advance();
                    self.expect(TokenKind::OpenBrace)?;
                    while self.peek_kind() != &TokenKind::CloseBrace {
                        states.push(self.parse_state()?);
                    }
                    self.expect(TokenKind::CloseBrace)?;
                }
                _ => {
                    self.advance();
                }
            }
        }

        let end_tok = self.expect(TokenKind::CloseBrace)?;
        Ok(StateMachineExtDef {
            name,
            target_state_machine: target_sm,
            states,
            transitions,
            span: Span::new(start_tok.span.start, end_tok.span.end, start_tok.span.line, start_tok.span.column),
        })
    }

    fn parse_state(&mut self) -> Result<StateDef, ParserError> {
        let start_span = self.expect(TokenKind::State)?.span;
        let name = match self.peek_kind() {
            TokenKind::StringLit(s) => {
                let res = s.clone();
                self.advance();
                res
            }
            _ => self.expect_ident()?.0,
        };

        self.expect(TokenKind::OpenBrace)?;
        let mut properties = Vec::new();
        let mut timeout = None;

        while self.peek_kind() != &TokenKind::CloseBrace && self.peek_kind() != &TokenKind::Eof {
            if self.peek_kind() == &TokenKind::Timeout {
                let to_span = self.advance().span;
                self.expect(TokenKind::OpenParen)?;
                let duration_expr = self.parse_expr()?;
                self.expect(TokenKind::CloseParen)?;
                self.expect(TokenKind::OpenBrace)?;

                let mut escalate_to = None;
                let mut action_name = None;

                while self.peek_kind() != &TokenKind::CloseBrace {
                    if let TokenKind::EscalateTo = self.peek_kind() {
                        self.advance();
                        self.expect(TokenKind::Assign)?;
                        escalate_to = match self.advance().kind {
                            TokenKind::StringLit(s) => Some(s),
                            TokenKind::Ident(s) => Some(s),
                            _ => None,
                        };
                        self.expect(TokenKind::Semicolon)?;
                    } else if let TokenKind::Ident(s) = self.peek_kind().clone() {
                        if s == "action" {
                            self.advance();
                            self.expect(TokenKind::Assign)?;
                            action_name = match self.advance().kind {
                                TokenKind::Ident(act) => Some(act),
                                _ => None,
                            };
                            self.expect(TokenKind::Semicolon)?;
                        } else {
                            self.advance();
                        }
                    } else {
                        self.advance();
                    }
                }
                self.expect(TokenKind::CloseBrace)?;
                timeout = Some(TimeoutDef {
                    duration_expr,
                    escalate_to,
                    action_name,
                    span: to_span,
                });
            } else if let TokenKind::Ident(_) = self.peek_kind() {
                properties.push(self.parse_property()?);
            } else {
                self.advance();
            }
        }
        self.expect(TokenKind::CloseBrace)?;

        Ok(StateDef {
            name,
            properties,
            timeout,
            span: start_span,
        })
    }

    fn parse_transition(&mut self) -> Result<TransitionDef, ParserError> {
        let start_span = self.expect(TokenKind::Transition)?.span;
        let (name, _) = self.expect_ident()?;

        let mut from_states = Vec::new();
        let mut to_state = String::new();
        let mut properties = Vec::new();
        let mut guard = None;
        let mut approval_matrix = None;
        let mut params = Vec::new();
        let mut on_transition = None;

        // Support concise header syntax: `transition submit from Draft to PendingApproval;`
        if self.peek_kind() == &TokenKind::From || matches!(self.peek_kind(), TokenKind::Ident(s) if s == "from") {
            self.advance();
            if self.peek_kind() == &TokenKind::OpenBracket {
                self.advance();
                while self.peek_kind() != &TokenKind::CloseBracket && self.peek_kind() != &TokenKind::Eof {
                    let st = match self.peek_kind().clone() {
                        TokenKind::StringLit(s) => {
                            self.advance();
                            s
                        }
                        _ => self.expect_ident()?.0,
                    };
                    from_states.push(st);
                    if self.peek_kind() == &TokenKind::Comma {
                        self.advance();
                    }
                }
                self.expect(TokenKind::CloseBracket)?;
            } else {
                let st = match self.peek_kind().clone() {
                    TokenKind::StringLit(s) => {
                        self.advance();
                        s
                    }
                    _ => self.expect_ident()?.0,
                };
                from_states.push(st);
            }

            if matches!(self.peek_kind(), TokenKind::Ident(s) if s == "to") {
                self.advance();
                to_state = match self.peek_kind().clone() {
                    TokenKind::StringLit(s) => {
                        self.advance();
                        s
                    }
                    _ => self.expect_ident()?.0,
                };
            }

            if self.peek_kind() == &TokenKind::Semicolon {
                self.advance();
                return Ok(TransitionDef {
                    name,
                    from_states,
                    to_state,
                    properties,
                    guard,
                    approval_matrix,
                    params,
                    on_transition,
                    span: start_span,
                });
            }
        }

        self.expect(TokenKind::OpenBrace)?;

        while self.peek_kind() != &TokenKind::CloseBrace && self.peek_kind() != &TokenKind::Eof {
            match self.peek_kind() {
                TokenKind::Guard => {
                    self.advance();
                    if self.peek_kind() == &TokenKind::OpenParen {
                        self.advance();
                        let _p = self.expect_ident()?;
                        self.expect(TokenKind::CloseParen)?;
                    }
                    guard = Some(self.parse_block()?);
                }
                TokenKind::ApprovalMatrix => {
                    self.advance();
                    self.expect(TokenKind::OpenBrace)?;
                    let mut tiers = Vec::new();
                    while self.peek_kind() != &TokenKind::CloseBrace {
                        self.expect(TokenKind::Tier)?;
                        let (tier_name, tier_span) = self.expect_ident()?;
                        self.expect(TokenKind::OpenBrace)?;
                        let mut condition = Expr::BoolLit(true);
                        let mut roles = Vec::new();

                        while self.peek_kind() != &TokenKind::CloseBrace {
                            if let TokenKind::Ident(prop) = self.peek_kind().clone() {
                                if prop == "condition" {
                                    self.advance();
                                    self.expect(TokenKind::Assign)?;
                                    condition = self.parse_expr()?;
                                    self.expect(TokenKind::Semicolon)?;
                                } else if prop == "roles" {
                                    self.advance();
                                    self.expect(TokenKind::Assign)?;
                                    self.expect(TokenKind::OpenBracket)?;
                                    while self.peek_kind() != &TokenKind::CloseBracket {
                                        if let TokenKind::StringLit(r) = self.advance().kind {
                                            roles.push(r);
                                        }
                                        if self.peek_kind() == &TokenKind::Comma {
                                            self.advance();
                                        }
                                    }
                                    self.expect(TokenKind::CloseBracket)?;
                                    self.expect(TokenKind::Semicolon)?;
                                } else {
                                    self.advance();
                                }
                            } else {
                                self.advance();
                            }
                        }
                        self.expect(TokenKind::CloseBrace)?;
                        tiers.push(ApprovalTierDef {
                            name: tier_name,
                            condition,
                            roles,
                            span: tier_span,
                        });
                    }
                    self.expect(TokenKind::CloseBrace)?;
                    approval_matrix = Some(tiers);
                }
                TokenKind::Params => {
                    self.advance();
                    self.expect(TokenKind::OpenBrace)?;
                    while self.peek_kind() != &TokenKind::CloseBrace {
                        params.push(self.parse_field()?);
                    }
                    self.expect(TokenKind::CloseBrace)?;
                }
                TokenKind::From => {
                    self.advance();
                    if self.peek_kind() == &TokenKind::Colon {
                        self.advance();
                    } else {
                        self.expect(TokenKind::Assign)?;
                    }
                    if self.peek_kind() == &TokenKind::OpenBracket {
                        self.advance();
                        while self.peek_kind() != &TokenKind::CloseBracket {
                            let st = match self.advance().kind {
                                TokenKind::StringLit(s) => s,
                                TokenKind::Ident(s) => s,
                                _ => "".to_string(),
                            };
                            from_states.push(st);
                            if self.peek_kind() == &TokenKind::Comma {
                                self.advance();
                            }
                        }
                        self.expect(TokenKind::CloseBracket)?;
                    } else {
                        let st = match self.advance().kind {
                            TokenKind::StringLit(s) => s,
                            TokenKind::Ident(s) => s,
                            _ => "".to_string(),
                        };
                        from_states.push(st);
                    }
                    if self.peek_kind() == &TokenKind::Semicolon {
                        self.advance();
                    }
                }
                TokenKind::Ident(s) if s == "from" => {
                    self.advance();
                    if self.peek_kind() == &TokenKind::Colon {
                        self.advance();
                    } else {
                        self.expect(TokenKind::Assign)?;
                    }
                    if self.peek_kind() == &TokenKind::OpenBracket {
                        self.advance();
                        while self.peek_kind() != &TokenKind::CloseBracket {
                            let st = match self.advance().kind {
                                TokenKind::StringLit(s) => s,
                                TokenKind::Ident(s) => s,
                                _ => "".to_string(),
                            };
                            from_states.push(st);
                            if self.peek_kind() == &TokenKind::Comma {
                                self.advance();
                            }
                        }
                        self.expect(TokenKind::CloseBracket)?;
                    } else {
                        let st = match self.advance().kind {
                            TokenKind::StringLit(s) => s,
                            TokenKind::Ident(s) => s,
                            _ => "".to_string(),
                        };
                        from_states.push(st);
                    }
                    if self.peek_kind() == &TokenKind::Semicolon {
                        self.advance();
                    }
                }
                TokenKind::Ident(s) if s == "to" => {
                    self.advance();
                    if self.peek_kind() == &TokenKind::Colon {
                        self.advance();
                    } else {
                        self.expect(TokenKind::Assign)?;
                    }
                    to_state = match self.advance().kind {
                        TokenKind::StringLit(s) => s,
                        TokenKind::Ident(s) => s,
                        _ => "".to_string(),
                    };
                    if self.peek_kind() == &TokenKind::Semicolon {
                        self.advance();
                    }
                }
                TokenKind::Ident(s) if s == "onTransition" || s == "effect" => {
                    self.advance();
                    if self.peek_kind() == &TokenKind::OpenParen {
                        self.advance();
                        while self.peek_kind() != &TokenKind::CloseParen && self.peek_kind() != &TokenKind::Eof {
                            self.advance();
                        }
                        self.expect(TokenKind::CloseParen)?;
                    }
                    on_transition = Some(self.parse_block()?);
                }
                TokenKind::Ident(_) => {
                    properties.push(self.parse_property()?);
                }
                _ => {
                    self.advance();
                }
            }
        }
        self.expect(TokenKind::CloseBrace)?;

        Ok(TransitionDef {
            name,
            from_states,
            to_state,
            properties,
            guard,
            approval_matrix,
            params,
            on_transition,
            span: start_span,
        })
    }

    // ----------------------------------------------------
    // PARSE CODEUNIT
    // ----------------------------------------------------
    #[allow(dead_code)]
    fn parse_codeunit(&mut self) -> Result<CodeunitDef, ParserError> {
        self.parse_codeunit_with_annotations(Vec::new())
    }

    fn parse_codeunit_with_annotations(&mut self, mut annotations: Vec<String>) -> Result<CodeunitDef, ParserError> {
        let start_tok = self.expect(TokenKind::Codeunit)?;
        let (name, _) = self.expect_ident()?;
        let mut bound_table = None;
        while self.peek_kind() != &TokenKind::OpenBrace && self.peek_kind() != &TokenKind::Eof {
            if self.peek_kind() == &TokenKind::For {
                self.advance();
                let (tbl, _) = self.expect_ident()?;
                bound_table = Some(tbl);
            } else if self.peek_kind() == &TokenKind::Implements {
                self.advance();
                let (iface, _) = self.expect_ident()?;
                annotations.push(format!("implements:{}", iface));
            } else {
                break;
            }
        }
        self.expect(TokenKind::OpenBrace)?;

        let mut properties = Vec::new();
        let mut methods = Vec::new();

        while self.peek_kind() != &TokenKind::CloseBrace && self.peek_kind() != &TokenKind::Eof {
            if self.peek_kind() == &TokenKind::At {
                // Annotation(s)
                let mut method_annots = Vec::new();
                while self.peek_kind() == &TokenKind::At {
                    method_annots.push(self.parse_annotation()?);
                }
                let method = self.parse_method(method_annots)?;
                methods.push(method);
            } else if self.peek_kind() == &TokenKind::Public
                || self.peek_kind() == &TokenKind::Private
                || self.peek_kind() == &TokenKind::Static
                || self.peek_kind() == &TokenKind::Function
            {
                let method = self.parse_method(Vec::new())?;
                methods.push(method);
            } else if let TokenKind::Ident(s) = self.peek_kind().clone() {
                if s == "async" {
                    let method = self.parse_method(Vec::new())?;
                    methods.push(method);
                } else if s == "tableNo" {
                    self.advance();
                    self.expect(TokenKind::Assign)?;
                    // Table.Entity
                    if self.peek_kind() == &TokenKind::Table {
                        self.advance();
                        if self.peek_kind() == &TokenKind::Dot {
                            self.advance();
                        }
                    }
                    let (tbl, _) = self.expect_ident()?;
                    bound_table = Some(tbl);
                    self.expect(TokenKind::Semicolon)?;
                } else if self.peek_second_kind() == Some(&TokenKind::OpenParen) {
                    let method = self.parse_method(Vec::new())?;
                    methods.push(method);
                } else if self.peek_second_kind() == Some(&TokenKind::Assign) {
                    properties.push(self.parse_property()?);
                } else {
                    self.advance();
                }
            } else {
                self.advance();
            }
        }
        self.expect(TokenKind::CloseBrace)?;

        Ok(CodeunitDef {
            name,
            bound_table,
            annotations,
            properties,
            methods,
            span: start_tok.span,
        })
    }

    fn parse_annotation(&mut self) -> Result<String, ParserError> {
        self.expect(TokenKind::At)?;
        let (mut name, _) = self.expect_ident()?;
        if self.peek_kind() == &TokenKind::OpenParen {
            self.advance();
            let mut depth = 1;
            let mut args = Vec::new();
            while depth > 0 && self.peek_kind() != &TokenKind::Eof {
                match self.peek_kind() {
                    TokenKind::OpenParen => depth += 1,
                    TokenKind::CloseParen => {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                    TokenKind::Ident(s) | TokenKind::StringLit(s) => args.push(s.clone()),
                    _ => {}
                }
                self.advance();
            }
            self.expect(TokenKind::CloseParen)?;
            if !args.is_empty() {
                name = format!("{}({})", name, args.join(","));
            }
        }
        Ok(name)
    }

    fn parse_type_annotation_str(&mut self) -> Result<String, ParserError> {
        let (mut base, _) = self.expect_ident()?;
        if self.peek_kind() == &TokenKind::Lt {
            self.advance();
            let mut inner = Vec::new();
            while self.peek_kind() != &TokenKind::Gt && self.peek_kind() != &TokenKind::Eof {
                if self.peek_kind() == &TokenKind::Comma {
                    self.advance();
                    continue;
                }
                let sub = self.parse_type_annotation_str()?;
                inner.push(sub);
            }
            self.expect(TokenKind::Gt)?;
            base = format!("{}<{}>", base, inner.join(", "));
        } else if self.peek_kind() == &TokenKind::OpenParen {
            self.advance();
            let mut depth = 1;
            while depth > 0 && self.peek_kind() != &TokenKind::Eof {
                match self.peek_kind() {
                    TokenKind::OpenParen => depth += 1,
                    TokenKind::CloseParen => {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                    _ => {}
                }
                self.advance();
            }
            self.expect(TokenKind::CloseParen)?;
        }
        while self.peek_kind() == &TokenKind::OpenBracket {
            self.advance();
            if self.peek_kind() == &TokenKind::CloseBracket {
                self.advance();
                base.push_str("[]");
            }
        }
        while self.peek_kind() == &TokenKind::Pipe {
            self.advance();
            let rhs = self.parse_type_annotation_str()?;
            base = format!("{} | {}", base, rhs);
        }
        Ok(base)
    }

    fn parse_method(&mut self, annotations: Vec<String>) -> Result<MethodDef, ParserError> {
        let mut is_public = true;
        let mut is_static = false;

        loop {
            if self.peek_kind() == &TokenKind::Public {
                self.advance();
                is_public = true;
            } else if self.peek_kind() == &TokenKind::Private {
                self.advance();
                is_public = false;
            } else if self.peek_kind() == &TokenKind::Static {
                self.advance();
                is_static = true;
            } else if self.peek_kind() == &TokenKind::Function {
                self.advance();
            } else if matches!(self.peek_kind(), TokenKind::Ident(s) if s == "async") {
                self.advance();
            } else {
                break;
            }
        }

        let (name, start_span) = self.expect_ident()?;
        self.expect(TokenKind::OpenParen)?;
        let mut params = Vec::new();
        while self.peek_kind() != &TokenKind::CloseParen && self.peek_kind() != &TokenKind::Eof {
            let (p_name, _) = self.expect_ident()?;
            if self.peek_kind() == &TokenKind::Question {
                self.advance();
            }
            let p_type = if self.peek_kind() == &TokenKind::Colon {
                self.advance();
                self.parse_type_annotation_str()?
            } else {
                "any".to_string()
            };
            if self.peek_kind() == &TokenKind::Assign {
                self.advance();
                let _ = self.parse_expr()?;
            }
            params.push(ParamDef {
                name: p_name,
                param_type: p_type,
            });
            if self.peek_kind() == &TokenKind::Comma {
                self.advance();
            }
        }
        self.expect(TokenKind::CloseParen)?;

        let return_type = if self.peek_kind() == &TokenKind::Colon {
            self.advance();
            Some(self.parse_type_annotation_str()?)
        } else {
            None
        };

        let body = self.parse_block()?;

        Ok(MethodDef {
            name,
            is_public,
            is_static,
            annotations,
            params,
            return_type,
            body,
            span: start_span,
        })
    }

    // ----------------------------------------------------
    // PARSE INTERFACE
    // ----------------------------------------------------
    fn parse_interface(&mut self) -> Result<InterfaceDef, ParserError> {
        let start_tok = self.expect(TokenKind::Interface)?;
        let (name, _) = self.expect_ident()?;
        self.expect(TokenKind::OpenBrace)?;

        let mut properties = Vec::new();
        let mut methods = Vec::new();

        while self.peek_kind() != &TokenKind::CloseBrace && self.peek_kind() != &TokenKind::Eof {
            if self.peek_second_kind() == Some(&TokenKind::Assign)
                || self.peek_second_kind() == Some(&TokenKind::Colon)
            {
                properties.push(self.parse_property()?);
            } else {
                if let TokenKind::Ident(s) = self.peek_kind().clone() {
                    if s == "procedure" || s == "function" {
                        self.advance();
                    }
                }
                let (m_name, m_span) = self.expect_ident()?;
                self.expect(TokenKind::OpenParen)?;
                let mut params = Vec::new();
                while self.peek_kind() != &TokenKind::CloseParen && self.peek_kind() != &TokenKind::Eof {
                    let (p_name, _) = self.expect_ident()?;
                    self.expect(TokenKind::Colon)?;
                    let p_type = self.parse_type_annotation_str()?;
                    params.push(ParamDef {
                        name: p_name,
                        param_type: p_type,
                    });
                    if self.peek_kind() == &TokenKind::Comma {
                        self.advance();
                    }
                }
                self.expect(TokenKind::CloseParen)?;
                let return_type = if self.peek_kind() == &TokenKind::Colon {
                    self.advance();
                    Some(self.parse_type_annotation_str()?)
                } else {
                    None
                };
                if self.peek_kind() == &TokenKind::Semicolon {
                    self.advance();
                }

                methods.push(InterfaceMethodDef {
                    name: m_name,
                    params,
                    return_type,
                    span: m_span,
                });
            }
        }
        self.expect(TokenKind::CloseBrace)?;

        Ok(InterfaceDef {
            name,
            properties,
            methods,
            span: start_tok.span,
        })
    }

    // ----------------------------------------------------
    // PARSE ENUM & ENUM EXTENSION
    // ----------------------------------------------------
    fn parse_enum(&mut self) -> Result<EnumDef, ParserError> {
        let start_tok = self.expect(TokenKind::Enum)?;
        let (name, _) = self.expect_ident()?;

        let implements_interface = if self.peek_kind() == &TokenKind::Implements {
            self.advance();
            let (iface, _) = self.expect_ident()?;
            Some(iface)
        } else {
            None
        };

        self.expect(TokenKind::OpenBrace)?;
        let mut properties = Vec::new();
        let mut members = Vec::new();
        let mut extensible = true;

        while self.peek_kind() != &TokenKind::CloseBrace && self.peek_kind() != &TokenKind::Eof {
            let (mem_name, mem_span) = self.expect_ident()?;
            if mem_name == "value" && self.peek_kind() == &TokenKind::OpenParen {
                self.advance();
                // value(0, Draft) or value(Draft)
                if matches!(self.peek_kind(), TokenKind::IntLit(_) | TokenKind::StringLit(_)) {
                    self.advance();
                    if self.peek_kind() == &TokenKind::Comma || self.peek_kind() == &TokenKind::Semicolon {
                        self.advance();
                    }
                }
                let val_name = match self.peek_kind().clone() {
                    TokenKind::StringLit(s) => {
                        self.advance();
                        s
                    }
                    _ => self.expect_ident()?.0,
                };
                self.expect(TokenKind::CloseParen)?;
                let mut bound_codeunit = None;
                if self.peek_kind() == &TokenKind::Assign || self.peek_kind() == &TokenKind::Colon {
                    self.advance();
                    if self.peek_kind() == &TokenKind::Codeunit {
                        self.advance();
                        self.expect(TokenKind::Dot)?;
                    }
                    bound_codeunit = Some(self.expect_ident()?.0);
                }
                members.push(EnumMemberDef {
                    name: val_name,
                    bound_codeunit,
                    span: mem_span,
                });
            } else if self.peek_kind() == &TokenKind::Assign || self.peek_kind() == &TokenKind::Colon {
                self.advance();
                if self.peek_kind() == &TokenKind::Codeunit {
                    self.advance();
                    self.expect(TokenKind::Dot)?;
                    let (cu, _) = self.expect_ident()?;
                    members.push(EnumMemberDef {
                        name: mem_name,
                        bound_codeunit: Some(cu),
                        span: mem_span,
                    });
                } else if mem_name == "extensible" {
                    if let TokenKind::True = self.advance().kind {
                        extensible = true;
                    } else {
                        extensible = false;
                    }
                } else if matches!(mem_name.as_str(), "caption" | "description") {
                    let val = self.parse_expr()?;
                    properties.push(Property {
                        name: mem_name,
                        value: val,
                        span: mem_span,
                    });
                } else if let Some(cu) = self.peek_ident() {
                    self.advance();
                    members.push(EnumMemberDef {
                        name: mem_name,
                        bound_codeunit: Some(cu),
                        span: mem_span,
                    });
                } else {
                    let val = self.parse_expr()?;
                    properties.push(Property {
                        name: mem_name,
                        value: val,
                        span: mem_span,
                    });
                }
            } else {
                members.push(EnumMemberDef {
                    name: mem_name,
                    bound_codeunit: None,
                    span: mem_span,
                });
            }

            if self.peek_kind() == &TokenKind::Comma || self.peek_kind() == &TokenKind::Semicolon {
                self.advance();
            }
        }
        self.expect(TokenKind::CloseBrace)?;

        Ok(EnumDef {
            name,
            implements_interface,
            extensible,
            properties,
            members,
            span: start_tok.span,
        })
    }

    fn parse_enum_ext(&mut self) -> Result<EnumExtDef, ParserError> {
        let start_tok = self.expect(TokenKind::EnumExtension)?;
        let (name, _) = self.expect_ident()?;
        self.expect(TokenKind::Extends)?;
        let (target_enum, _) = self.expect_ident()?;
        self.expect(TokenKind::OpenBrace)?;

        let mut members = Vec::new();

        while self.peek_kind() != &TokenKind::CloseBrace && self.peek_kind() != &TokenKind::Eof {
            let (mem_name, mem_span) = self.expect_ident()?;
            if mem_name == "value" && self.peek_kind() == &TokenKind::OpenParen {
                self.advance();
                if matches!(self.peek_kind(), TokenKind::IntLit(_) | TokenKind::StringLit(_)) {
                    self.advance();
                    if self.peek_kind() == &TokenKind::Comma || self.peek_kind() == &TokenKind::Semicolon {
                        self.advance();
                    }
                }
                let val_name = match self.peek_kind().clone() {
                    TokenKind::StringLit(s) => {
                        self.advance();
                        s
                    }
                    _ => self.expect_ident()?.0,
                };
                self.expect(TokenKind::CloseParen)?;
                members.push(EnumMemberDef {
                    name: val_name,
                    bound_codeunit: None,
                    span: mem_span,
                });
            } else {
                let bound_cu = if self.peek_kind() == &TokenKind::Assign || self.peek_kind() == &TokenKind::Colon {
                    self.advance();
                    if self.peek_kind() == &TokenKind::Codeunit {
                        self.advance();
                        self.expect(TokenKind::Dot)?;
                    }
                    let (cu, _) = self.expect_ident()?;
                    Some(cu)
                } else {
                    None
                };

                members.push(EnumMemberDef {
                    name: mem_name,
                    bound_codeunit: bound_cu,
                    span: mem_span,
                });
            }

            if self.peek_kind() == &TokenKind::Comma || self.peek_kind() == &TokenKind::Semicolon {
                self.advance();
            }
        }
        self.expect(TokenKind::CloseBrace)?;

        Ok(EnumExtDef {
            name,
            target_enum,
            members,
            span: start_tok.span,
        })
    }

    // ----------------------------------------------------
    // PARSE PERMISSIONSET
    // ----------------------------------------------------
    fn parse_permission_set(&mut self) -> Result<PermissionSetDef, ParserError> {
        let start_tok = self.expect(TokenKind::PermissionSet)?;
        let (name, _) = self.expect_ident()?;
        self.expect(TokenKind::OpenBrace)?;

        let mut properties = Vec::new();
        let mut permissions = Vec::new();
        let mut execute = Vec::new();

        while self.peek_kind() != &TokenKind::CloseBrace && self.peek_kind() != &TokenKind::Eof {
            if let TokenKind::Ident(ref s) = self.peek_kind().clone() {
                if s == "permissions" {
                    self.advance();
                    self.expect(TokenKind::Assign)?;
                    self.expect(TokenKind::OpenBrace)?;
                    while self.peek_kind() != &TokenKind::CloseBrace && self.peek_kind() != &TokenKind::Eof {
                        let (table_name, perm_span) = self.expect_ident()?;
                        self.expect(TokenKind::Assign)?;
                        let (action_str, _) = self.expect_ident()?;
                        let mut actions = Vec::new();
                        for c in action_str.chars() {
                            match c.to_ascii_uppercase() {
                                'R' => actions.push(PermissionAction::Read),
                                'I' => actions.push(PermissionAction::Insert),
                                'M' => actions.push(PermissionAction::Modify),
                                'D' => actions.push(PermissionAction::Delete),
                                _ => {}
                            }
                        }
                        let mut row_filter = None;
                        if self.peek_kind() == &TokenKind::Where {
                            self.advance();
                            let has_brace = if self.peek_kind() == &TokenKind::OpenBrace {
                                self.advance();
                                true
                            } else {
                                false
                            };
                            row_filter = Some(self.parse_expr()?);
                            if has_brace {
                                self.expect(TokenKind::CloseBrace)?;
                            }
                        }
                        self.expect(TokenKind::Semicolon)?;
                        permissions.push(TablePermissionDef {
                            table_name,
                            actions,
                            row_filter,
                            span: perm_span,
                        });
                    }
                    self.expect(TokenKind::CloseBrace)?;
                    if self.peek_kind() == &TokenKind::Semicolon {
                        self.advance();
                    }
                } else if s == "execute" {
                    self.advance();
                    self.expect(TokenKind::Assign)?;
                    self.expect(TokenKind::OpenBracket)?;
                    while self.peek_kind() != &TokenKind::CloseBracket && self.peek_kind() != &TokenKind::Eof {
                        let (codeunit_name, _) = self.expect_ident()?;
                        execute.push(codeunit_name);
                        if self.peek_kind() == &TokenKind::Comma {
                            self.advance();
                        }
                    }
                    self.expect(TokenKind::CloseBracket)?;
                    if self.peek_kind() == &TokenKind::Semicolon {
                        self.advance();
                    }
                } else if s == "tabledata" {
                    let perm_span = self.advance().span;
                    let (table_name, _) = self.expect_ident()?;
                    if self.peek_kind() == &TokenKind::Colon {
                        self.advance();
                    } else {
                        self.expect(TokenKind::Assign)?;
                    }
                    let (perm_str, _) = self.expect_ident()?;
                    let mut actions = Vec::new();
                    for ch in perm_str.to_uppercase().chars() {
                        match ch {
                            'R' => actions.push(PermissionAction::Read),
                            'I' => actions.push(PermissionAction::Insert),
                            'M' => actions.push(PermissionAction::Modify),
                            'D' => actions.push(PermissionAction::Delete),
                            _ => {}
                        }
                    }
                    if self.peek_kind() == &TokenKind::Semicolon {
                        self.advance();
                    }
                    permissions.push(TablePermissionDef {
                        table_name,
                        actions,
                        row_filter: None,
                        span: perm_span,
                    });
                } else if self.peek_second_kind() == Some(&TokenKind::Assign)
                    || self.peek_second_kind() == Some(&TokenKind::Colon)
                {
                    properties.push(self.parse_property()?);
                } else {
                    self.advance();
                }
            } else {
                self.advance();
            }
        }
        self.expect(TokenKind::CloseBrace)?;

        Ok(PermissionSetDef {
            name,
            properties,
            permissions,
            execute,
            span: start_tok.span,
        })
    }

    // ----------------------------------------------------
    // PARSE VIEW
    // ----------------------------------------------------
    fn parse_view(&mut self) -> Result<ViewDef, ParserError> {
        let start_tok = self.expect(TokenKind::View)?;
        let (name, _) = self.expect_ident()?;
        self.expect(TokenKind::OpenBrace)?;

        let mut properties = Vec::new();
        let mut select_fields = Vec::new();
        let mut source_table = String::new();
        let mut joins = Vec::new();
        let mut where_filter = None;

        while self.peek_kind() != &TokenKind::CloseBrace && self.peek_kind() != &TokenKind::Eof {
            match self.peek_kind().clone() {
                TokenKind::Select => {
                    self.advance();
                    let has_brace = if self.peek_kind() == &TokenKind::OpenBrace {
                        self.advance();
                        true
                    } else {
                        false
                    };
                    while self.peek_kind() != &TokenKind::CloseBrace
                        && self.peek_kind() != &TokenKind::From
                        && self.peek_kind() != &TokenKind::Eof
                    {
                        if let TokenKind::Ident(ref kw) = self.peek_kind().clone() {
                            if kw.eq_ignore_ascii_case("from") {
                                break;
                            }
                        }
                        let f_span = self.peek().span.clone();
                        let field_expr = self.parse_expr()?;
                        let mut alias = None;
                        if let TokenKind::Ident(ref s) = self.peek_kind().clone() {
                            if s.eq_ignore_ascii_case("as") {
                                self.advance();
                                let (al, _) = self.expect_ident()?;
                                alias = Some(al);
                            }
                        }
                        select_fields.push(ViewSelectField {
                            field_expr,
                            alias,
                            span: f_span,
                        });
                        if self.peek_kind() == &TokenKind::Comma {
                            self.advance();
                        }
                    }
                    if has_brace {
                        self.expect(TokenKind::CloseBrace)?;
                    }
                    if self.peek_kind() == &TokenKind::Semicolon {
                        self.advance();
                    }
                }
                TokenKind::From => {
                    self.advance();
                    let (tbl, _) = self.expect_ident()?;
                    source_table = tbl;
                    if self.peek_kind() == &TokenKind::Semicolon {
                        self.advance();
                    }
                }
                TokenKind::Ident(ref kw) if kw.eq_ignore_ascii_case("select") => {
                    self.advance();
                    while self.peek_kind() != &TokenKind::CloseBrace
                        && self.peek_kind() != &TokenKind::From
                        && self.peek_kind() != &TokenKind::Eof
                    {
                        if let TokenKind::Ident(ref fkw) = self.peek_kind().clone() {
                            if fkw.eq_ignore_ascii_case("from") {
                                break;
                            }
                        }
                        let f_span = self.peek().span.clone();
                        let field_expr = self.parse_expr()?;
                        select_fields.push(ViewSelectField {
                            field_expr,
                            alias: None,
                            span: f_span,
                        });
                        if self.peek_kind() == &TokenKind::Comma {
                            self.advance();
                        }
                    }
                }
                TokenKind::Ident(ref kw) if kw.eq_ignore_ascii_case("from") => {
                    self.advance();
                    let (tbl, _) = self.expect_ident()?;
                    source_table = tbl;
                    if self.peek_kind() == &TokenKind::Semicolon {
                        self.advance();
                    }
                }
                TokenKind::LeftJoin | TokenKind::InnerJoin => {
                    let is_left = *self.peek_kind() == TokenKind::LeftJoin;
                    let j_span = self.advance().span;
                    let (target_table, _) = self.expect_ident()?;
                    let mut alias = None;
                    if let TokenKind::Ident(ref s) = self.peek_kind().clone() {
                        if s.eq_ignore_ascii_case("as") {
                            self.advance();
                            let (al, _) = self.expect_ident()?;
                            alias = Some(al);
                        }
                    }
                    if self.peek_kind() == &TokenKind::For {
                        self.advance();
                    }
                    self.expect(TokenKind::OpenParen)?;
                    let condition = self.parse_expr()?;
                    self.expect(TokenKind::CloseParen)?;
                    if self.peek_kind() == &TokenKind::Semicolon {
                        self.advance();
                    }
                    joins.push(ViewJoinDef {
                        join_type: if is_left { JoinType::Left } else { JoinType::Inner },
                        target_table,
                        alias,
                        condition,
                        span: j_span,
                    });
                }
                TokenKind::Where => {
                    self.advance();
                    where_filter = Some(self.parse_expr()?);
                    if self.peek_kind() == &TokenKind::Semicolon {
                        self.advance();
                    }
                }
                TokenKind::Ident(_) => {
                    if self.peek_second_kind() == Some(&TokenKind::Assign) {
                        properties.push(self.parse_property()?);
                    } else {
                        self.advance();
                    }
                }
                _ => {
                    self.advance();
                }
            }
        }
        self.expect(TokenKind::CloseBrace)?;

        Ok(ViewDef {
            name,
            properties,
            select_fields,
            source_table,
            joins,
            where_filter,
            span: start_tok.span,
        })
    }

    // ----------------------------------------------------
    // PARSE QUERY
    // ----------------------------------------------------
    fn parse_query(&mut self) -> Result<QueryDef, ParserError> {
        let start_tok = self.expect(TokenKind::Query)?;
        let (name, _) = self.expect_ident()?;
        self.expect(TokenKind::OpenBrace)?;

        let mut properties: Vec<Property> = Vec::new();
        let mut elements = Vec::new();
        let mut group_by = Vec::new();
        let mut having = None;
        let mut order_by = Vec::new();

        while self.peek_kind() != &TokenKind::CloseBrace && self.peek_kind() != &TokenKind::Eof {
            match self.peek_kind() {
                TokenKind::Elements => {
                    self.advance();
                    self.expect(TokenKind::OpenBrace)?;
                    while self.peek_kind() != &TokenKind::CloseBrace && self.peek_kind() != &TokenKind::Eof {
                        if self.peek_kind() == &TokenKind::Dataitem {
                            elements.push(self.parse_dataitem()?);
                        } else {
                            self.advance();
                        }
                    }
                    self.expect(TokenKind::CloseBrace)?;
                    if self.peek_kind() == &TokenKind::Semicolon {
                        self.advance();
                    }
                }
                TokenKind::GroupBy => {
                    self.advance();
                    self.expect(TokenKind::OpenParen)?;
                    while self.peek_kind() != &TokenKind::CloseParen && self.peek_kind() != &TokenKind::Eof {
                        let (mut col, _) = self.expect_ident()?;
                        while self.peek_kind() == &TokenKind::Dot {
                            self.advance();
                            let (sub, _) = self.expect_ident()?;
                            col.push('.');
                            col.push_str(&sub);
                        }
                        group_by.push(col);
                        if self.peek_kind() == &TokenKind::Comma {
                            self.advance();
                        }
                    }
                    self.expect(TokenKind::CloseParen)?;
                    if self.peek_kind() == &TokenKind::Semicolon {
                        self.advance();
                    }
                }
                TokenKind::Having => {
                    self.advance();
                    having = Some(self.parse_expr()?);
                    if self.peek_kind() == &TokenKind::Semicolon {
                        self.advance();
                    }
                }
                TokenKind::OrderBy => {
                    self.advance();
                    self.expect(TokenKind::OpenParen)?;
                    while self.peek_kind() != &TokenKind::CloseParen && self.peek_kind() != &TokenKind::Eof {
                        let (mut col, ord_span) = self.expect_ident()?;
                        while self.peek_kind() == &TokenKind::Dot {
                            self.advance();
                            let (sub, _) = self.expect_ident()?;
                            col.push('.');
                            col.push_str(&sub);
                        }
                        let mut ascending = true;
                        if let TokenKind::Ident(ref dir) = self.peek_kind().clone() {
                            if dir.eq_ignore_ascii_case("desc") {
                                ascending = false;
                                self.advance();
                            } else if dir.eq_ignore_ascii_case("asc") {
                                ascending = true;
                                self.advance();
                            }
                        }
                        order_by.push(QueryOrderByDef {
                            column: col,
                            ascending,
                            span: ord_span,
                        });
                        if self.peek_kind() == &TokenKind::Comma {
                            self.advance();
                        }
                    }
                    self.expect(TokenKind::CloseParen)?;
                    if self.peek_kind() == &TokenKind::Semicolon {
                        self.advance();
                    }
                }
                TokenKind::Ident(s) if s == "columns" => {
                    let col_block_span = self.advance().span;
                    self.expect(TokenKind::OpenBrace)?;
                    let mut cols = Vec::new();
                    while self.peek_kind() != &TokenKind::CloseBrace && self.peek_kind() != &TokenKind::Eof {
                        if let TokenKind::Ident(ref c) = self.peek_kind().clone() {
                            if c == "column" {
                                let c_span = self.advance().span;
                                self.expect(TokenKind::OpenParen)?;
                                let (col_name, _) = self.expect_ident()?;
                                let src_field = if self.peek_kind() == &TokenKind::Comma || self.peek_kind() == &TokenKind::Semicolon {
                                    self.advance();
                                    self.expect_ident()?.0
                                } else {
                                    col_name.clone()
                                };
                                self.expect(TokenKind::CloseParen)?;
                                if self.peek_kind() == &TokenKind::OpenBrace {
                                    self.advance();
                                    self.expect(TokenKind::CloseBrace)?;
                                }
                                if self.peek_kind() == &TokenKind::Semicolon {
                                    self.advance();
                                }
                                cols.push(QueryColumnDef {
                                    name: col_name,
                                    source_field: src_field,
                                    aggregation: None,
                                    span: c_span,
                                });
                                continue;
                            }
                        }
                        self.advance();
                    }
                    self.expect(TokenKind::CloseBrace)?;
                    if self.peek_kind() == &TokenKind::Semicolon {
                        self.advance();
                    }
                    let src_tbl = properties
                        .iter()
                        .find(|p| p.name == "sourceTable" || p.name == "source_table")
                        .and_then(|p| match &p.value {
                            Expr::Ident(id) | Expr::StringLit(id) => Some(id.clone()),
                            _ => None,
                        })
                        .unwrap_or_else(|| name.clone());
                    elements.push(DataItemDef {
                        name: "root".to_string(),
                        table_name: src_tbl,
                        columns: cols,
                        filter: None,
                        join_type: None,
                        join_condition: None,
                        data_items: Vec::new(),
                        span: col_block_span,
                    });
                }
                TokenKind::Ident(_) => {
                    if self.peek_second_kind() == Some(&TokenKind::Assign)
                        || self.peek_second_kind() == Some(&TokenKind::Colon)
                    {
                        properties.push(self.parse_property()?);
                    } else {
                        self.advance();
                    }
                }
                _ => {
                    self.advance();
                }
            }
        }
        self.expect(TokenKind::CloseBrace)?;

        Ok(QueryDef {
            name,
            properties,
            elements,
            group_by,
            having,
            order_by,
            span: start_tok.span,
        })
    }

    fn parse_dataitem(&mut self) -> Result<DataItemDef, ParserError> {
        let start_tok = self.expect(TokenKind::Dataitem)?;
        self.expect(TokenKind::OpenParen)?;
        let (name, _) = self.expect_ident()?;
        self.expect(TokenKind::Semicolon)?;
        let (table_name, _) = self.expect_ident()?;
        self.expect(TokenKind::CloseParen)?;
        self.expect(TokenKind::OpenBrace)?;

        let mut columns = Vec::new();
        let mut filter = None;
        let mut join_type = None;
        let mut join_condition = None;
        let mut child_dataitems = Vec::new();

        while self.peek_kind() != &TokenKind::CloseBrace && self.peek_kind() != &TokenKind::Eof {
            match self.peek_kind() {
                TokenKind::Dataitem => {
                    child_dataitems.push(self.parse_dataitem()?);
                }
                TokenKind::LeftJoin | TokenKind::InnerJoin => {
                    let is_left = *self.peek_kind() == TokenKind::LeftJoin;
                    self.advance();
                    join_type = Some(if is_left { JoinType::Left } else { JoinType::Inner });
                    if self.peek_kind() == &TokenKind::OpenParen {
                        self.advance();
                        join_condition = Some(self.parse_expr()?);
                        self.expect(TokenKind::CloseParen)?;
                    }
                    if self.peek_kind() == &TokenKind::Semicolon {
                        self.advance();
                    }
                }
                TokenKind::Where => {
                    self.advance();
                    filter = Some(self.parse_expr()?);
                    if self.peek_kind() == &TokenKind::Semicolon {
                        self.advance();
                    }
                }
                TokenKind::Ident(s) => {
                    if s == "column" {
                        self.advance();
                        self.expect(TokenKind::OpenParen)?;
                        let (col_name, col_span) = self.expect_ident()?;
                        self.expect(TokenKind::Semicolon)?;
                        let (src_field, _) = self.expect_ident()?;
                        let mut aggregation = None;
                        if self.peek_kind() == &TokenKind::Semicolon {
                            self.advance();
                            let (agg_str, _) = self.expect_ident()?;
                            aggregation = match agg_str.to_lowercase().as_str() {
                                "sum" => Some(AggregationFunc::Sum),
                                "avg" | "average" => Some(AggregationFunc::Average),
                                "count" => Some(AggregationFunc::Count),
                                "min" => Some(AggregationFunc::Min),
                                "max" => Some(AggregationFunc::Max),
                                _ => None,
                            };
                        }
                        self.expect(TokenKind::CloseParen)?;
                        self.expect(TokenKind::OpenBrace)?;
                        self.expect(TokenKind::CloseBrace)?;
                        columns.push(QueryColumnDef {
                            name: col_name,
                            source_field: src_field,
                            aggregation,
                            span: col_span,
                        });
                    } else {
                        self.advance();
                    }
                }
                _ => {
                    self.advance();
                }
            }
        }
        self.expect(TokenKind::CloseBrace)?;

        Ok(DataItemDef {
            name,
            table_name,
            columns,
            filter,
            join_type,
            join_condition,
            data_items: child_dataitems,
            span: start_tok.span,
        })
    }

    // ----------------------------------------------------
    // PARSE PAGE, PAGE EXTENSION, MENU, REPORT & CUSTOM METADATA
    // ----------------------------------------------------
    fn parse_page(&mut self) -> Result<PageDef, ParserError> {
        let start_tok = self.expect(TokenKind::Page)?;
        let (name, _) = self.expect_ident()?;
        let mut source_table = if self.peek_kind() == &TokenKind::For {
            self.advance();
            Some(self.expect_ident()?.0)
        } else {
            None
        };
        self.expect(TokenKind::OpenBrace)?;

        let mut page_type = "Card".to_string();
        let mut properties = Vec::new();
        let mut views = Vec::new();
        let mut controls = Vec::new();
        let mut actions = Vec::new();
        let mut triggers = Vec::new();

        while self.peek_kind() != &TokenKind::CloseBrace && self.peek_kind() != &TokenKind::Eof {
            if let Some(s) = self.peek_ident() {
                if s == "layout" && self.peek_second_kind() == Some(&TokenKind::OpenBrace) {
                    self.advance();
                    self.expect(TokenKind::OpenBrace)?;
                    while self.peek_kind() != &TokenKind::CloseBrace && self.peek_kind() != &TokenKind::Eof {
                        controls.push(self.parse_page_control()?);
                    }
                    self.expect(TokenKind::CloseBrace)?;
                    if self.peek_kind() == &TokenKind::Semicolon {
                        self.advance();
                    }
                } else if matches!(s.as_str(), "actions" | "commands")
                    && self.peek_second_kind() == Some(&TokenKind::OpenBrace)
                {
                    self.advance();
                    self.expect(TokenKind::OpenBrace)?;
                    while self.peek_kind() != &TokenKind::CloseBrace && self.peek_kind() != &TokenKind::Eof {
                        actions.push(self.parse_page_action()?);
                    }
                    self.expect(TokenKind::CloseBrace)?;
                    if self.peek_kind() == &TokenKind::Semicolon {
                        self.advance();
                    }
                } else if s == "views" && self.peek_second_kind() == Some(&TokenKind::OpenBrace) {
                    self.advance();
                    self.expect(TokenKind::OpenBrace)?;
                    while self.peek_kind() != &TokenKind::CloseBrace && self.peek_kind() != &TokenKind::Eof {
                        views.push(self.parse_page_view()?);
                    }
                    self.expect(TokenKind::CloseBrace)?;
                    if self.peek_kind() == &TokenKind::Semicolon {
                        self.advance();
                    }
                } else if s == "filters" && self.peek_second_kind() == Some(&TokenKind::OpenBrace) {
                    let (_, f_span) = self.expect_ident()?;
                    self.expect(TokenKind::OpenBrace)?;
                    let mut filter_items = Vec::new();
                    while self.peek_kind() != &TokenKind::CloseBrace && self.peek_kind() != &TokenKind::Eof {
                        if let Some(f_ident) = self.peek_ident() {
                            self.advance();
                            filter_items.push(Expr::Ident(f_ident));
                            if self.peek_kind() == &TokenKind::Semicolon || self.peek_kind() == &TokenKind::Comma {
                                self.advance();
                            }
                        } else {
                            self.advance();
                        }
                    }
                    self.expect(TokenKind::CloseBrace)?;
                    if self.peek_kind() == &TokenKind::Semicolon {
                        self.advance();
                    }
                    properties.push(Property {
                        name: "filters".to_string(),
                        value: Expr::ArrayLit(filter_items),
                        span: f_span,
                    });
                } else if s == "triggers" && self.peek_second_kind() == Some(&TokenKind::OpenBrace) {
                    self.advance();
                    self.expect(TokenKind::OpenBrace)?;
                    while self.peek_kind() != &TokenKind::CloseBrace && self.peek_kind() != &TokenKind::Eof {
                        triggers.push(self.parse_trigger()?);
                    }
                    self.expect(TokenKind::CloseBrace)?;
                    if self.peek_kind() == &TokenKind::Semicolon {
                        self.advance();
                    }
                } else if s.starts_with("on")
                    && (self.peek_second_kind() == Some(&TokenKind::OpenParen)
                        || self.peek_second_kind() == Some(&TokenKind::OpenBrace))
                {
                    triggers.push(self.parse_trigger()?);
                } else if matches!(s.as_str(), "action" | "command")
                    && (self.peek_second_kind() == Some(&TokenKind::OpenParen)
                        || self.peek_second_kind() == Some(&TokenKind::OpenBrace))
                {
                    actions.push(self.parse_page_action()?);
                } else if self.peek_second_kind() == Some(&TokenKind::Assign)
                    || self.peek_second_kind() == Some(&TokenKind::Colon)
                {
                    let prop = self.parse_property()?;
                    if matches!(prop.name.as_str(), "sourceTable" | "source_table" | "source") {
                        match &prop.value {
                            Expr::Ident(tbl) | Expr::StringLit(tbl) => {
                                source_table = Some(tbl.clone());
                            }
                            Expr::MemberAccess { member, .. } => {
                                source_table = Some(member.clone());
                            }
                            _ => {}
                        }
                    } else if matches!(prop.name.as_str(), "pageType" | "page_type" | "layout") {
                        if let Expr::Ident(ref pt) | Expr::StringLit(ref pt) = prop.value {
                            page_type = pt.clone();
                        }
                    }
                    properties.push(prop);
                } else {
                    controls.push(self.parse_page_control()?);
                }
            } else {
                self.advance();
            }
        }
        let end_tok = self.expect(TokenKind::CloseBrace)?;

        Ok(PageDef {
            name,
            source_table,
            page_type,
            properties,
            views,
            controls,
            actions,
            triggers,
            span: Span::new(start_tok.span.start, end_tok.span.end, start_tok.span.line, start_tok.span.column),
        })
    }

    fn parse_page_view(&mut self) -> Result<PageViewDef, ParserError> {
        let (first, start_span) = self.expect_ident()?;
        let name = if first == "view" && self.peek_kind() == &TokenKind::OpenParen {
            self.advance();
            let n = match self.peek_kind().clone() {
                TokenKind::StringLit(s) => {
                    self.advance();
                    s
                }
                _ => self.expect_ident()?.0,
            };
            self.expect(TokenKind::CloseParen)?;
            n
        } else if first == "view" && matches!(self.peek_kind(), TokenKind::StringLit(_)) {
            if let TokenKind::StringLit(s) = self.advance().kind {
                s
            } else {
                first
            }
        } else if first == "view" && self.peek_ident().is_some() && self.peek_kind() != &TokenKind::OpenBrace {
            self.expect_ident()?.0
        } else {
            first
        };

        let mut properties = Vec::new();
        if self.peek_kind() == &TokenKind::OpenBrace {
            self.advance();
            while self.peek_kind() != &TokenKind::CloseBrace && self.peek_kind() != &TokenKind::Eof {
                if let Some(s) = self.peek_ident() {
                    if self.peek_second_kind() == Some(&TokenKind::Assign)
                        || self.peek_second_kind() == Some(&TokenKind::Colon)
                    {
                        properties.push(self.parse_property()?);
                    } else if self.peek_second_kind() == Some(&TokenKind::Semicolon) {
                        let (_, sp) = self.expect_ident()?;
                        self.expect(TokenKind::Semicolon)?;
                        properties.push(Property {
                            name: s,
                            value: Expr::BoolLit(true),
                            span: sp,
                        });
                    } else {
                        self.advance();
                    }
                } else {
                    self.advance();
                }
            }
            self.expect(TokenKind::CloseBrace)?;
        }
        if self.peek_kind() == &TokenKind::Semicolon {
            self.advance();
        }
        Ok(PageViewDef {
            name,
            properties,
            span: start_span,
        })
    }

    fn parse_page_ext(&mut self) -> Result<PageExtDef, ParserError> {
        let start_tok = self.expect(TokenKind::PageExtension)?;
        let (name, _) = self.expect_ident()?;
        let target_page = if self.peek_kind() == &TokenKind::Extends || self.peek_kind() == &TokenKind::For {
            self.advance();
            self.expect_ident()?.0
        } else {
            name.clone()
        };
        self.expect(TokenKind::OpenBrace)?;

        let mut properties = Vec::new();
        let mut controls = Vec::new();
        let mut actions = Vec::new();
        let mut triggers = Vec::new();

        while self.peek_kind() != &TokenKind::CloseBrace && self.peek_kind() != &TokenKind::Eof {
            if let Some(s) = self.peek_ident() {
                if s == "layout" && self.peek_second_kind() == Some(&TokenKind::OpenBrace) {
                    self.advance();
                    self.expect(TokenKind::OpenBrace)?;
                    while self.peek_kind() != &TokenKind::CloseBrace && self.peek_kind() != &TokenKind::Eof {
                        controls.push(self.parse_page_control()?);
                    }
                    self.expect(TokenKind::CloseBrace)?;
                    if self.peek_kind() == &TokenKind::Semicolon {
                        self.advance();
                    }
                } else if matches!(s.as_str(), "actions" | "commands")
                    && self.peek_second_kind() == Some(&TokenKind::OpenBrace)
                {
                    self.advance();
                    self.expect(TokenKind::OpenBrace)?;
                    while self.peek_kind() != &TokenKind::CloseBrace && self.peek_kind() != &TokenKind::Eof {
                        actions.push(self.parse_page_action()?);
                    }
                    self.expect(TokenKind::CloseBrace)?;
                    if self.peek_kind() == &TokenKind::Semicolon {
                        self.advance();
                    }
                } else if s.starts_with("on")
                    && (self.peek_second_kind() == Some(&TokenKind::OpenParen)
                        || self.peek_second_kind() == Some(&TokenKind::OpenBrace))
                {
                    triggers.push(self.parse_trigger()?);
                } else if matches!(s.as_str(), "action" | "command")
                    && (self.peek_second_kind() == Some(&TokenKind::OpenParen)
                        || self.peek_second_kind() == Some(&TokenKind::OpenBrace))
                {
                    actions.push(self.parse_page_action()?);
                } else if self.peek_second_kind() == Some(&TokenKind::Assign)
                    || self.peek_second_kind() == Some(&TokenKind::Colon)
                {
                    properties.push(self.parse_property()?);
                } else {
                    controls.push(self.parse_page_control()?);
                }
            } else {
                self.advance();
            }
        }
        let end_tok = self.expect(TokenKind::CloseBrace)?;

        Ok(PageExtDef {
            name,
            target_page,
            properties,
            controls,
            actions,
            triggers,
            span: Span::new(start_tok.span.start, end_tok.span.end, start_tok.span.line, start_tok.span.column),
        })
    }

    fn parse_page_control(&mut self) -> Result<PageControlDef, ParserError> {
        let (raw_kind, start_span) = self.expect_ident()?;
        let is_container_keyword = matches!(
            raw_kind.as_str(),
            "area"
                | "group"
                | "section"
                | "repeater"
                | "lines"
                | "part"
                | "factbox"
                | "tabs"
                | "tab"
                | "columns"
                | "column"
                | "card"
                | "step"
                | "steps"
                | "tile"
                | "tiles"
                | "region"
                | "regions"
                | "main"
                | "side"
                | "header"
                | "footer"
                | "content"
                | "split"
                | "grid"
                | "flex"
                | "bento"
                | "kanban"
                | "accordion"
                | "panel"
                | "drawer"
                | "modal"
                | "stepper"
                | "wrap"
                | "replace"
                | "addAfter"
                | "addBefore"
                | "addFirst"
                | "addLast"
                | "modify"
                | "moveAfter"
                | "moveBefore"
                | "hide"
        );

        let mut kind = raw_kind.clone();
        let mut name = raw_kind.clone();
        let mut source_expr = None;
        let mut properties = Vec::new();

        if self.peek_kind() == &TokenKind::OpenParen {
            self.advance();
            while self.peek_kind() != &TokenKind::CloseParen && self.peek_kind() != &TokenKind::Eof {
                if let Some(_ident_name) = self.peek_ident() {
                    if self.peek_second_kind() == Some(&TokenKind::Assign)
                        || self.peek_second_kind() == Some(&TokenKind::Colon)
                    {
                        // Named parameter in parens, e.g. cols = 4 or gap = 3
                        let prop = self.parse_property()?;
                        properties.push(prop);
                        if self.peek_kind() == &TokenKind::Comma || self.peek_kind() == &TokenKind::Semicolon {
                            self.advance();
                        }
                        continue;
                    }
                }

                // Positional target name or expression
                let first = match self.peek_kind().clone() {
                    TokenKind::StringLit(s) => {
                        self.advance();
                        s
                    }
                    _ => self.expect_ident()?.0,
                };
                name = first.clone();
                if self.peek_kind() == &TokenKind::Semicolon || self.peek_kind() == &TokenKind::Comma {
                    self.advance();
                    // Could be second positional (source_expr) or start of named properties
                    if let Some(_) = self.peek_ident() {
                        if self.peek_second_kind() == Some(&TokenKind::Assign)
                            || self.peek_second_kind() == Some(&TokenKind::Colon)
                        {
                            continue;
                        }
                    }
                    if self.peek_kind() != &TokenKind::CloseParen {
                        let second = match self.peek_kind().clone() {
                            TokenKind::StringLit(s) => {
                                self.advance();
                                s
                            }
                            _ => self.expect_ident()?.0,
                        };
                        source_expr = Some(second);
                        if self.peek_kind() == &TokenKind::Comma || self.peek_kind() == &TokenKind::Semicolon {
                            self.advance();
                        }
                    }
                } else if kind == "field" {
                    source_expr = Some(first);
                }
            }
            self.expect(TokenKind::CloseParen)?;
        } else if is_container_keyword {
            if let TokenKind::StringLit(label) = self.peek_kind().clone() {
                let sp = self.advance().span;
                name = label.clone();
                properties.push(Property {
                    name: "caption".to_string(),
                    value: Expr::StringLit(label),
                    span: sp,
                });
            } else if self.peek_ident().is_some()
                && self.peek_kind() != &TokenKind::OpenBrace
                && self.peek_kind() != &TokenKind::Semicolon
            {
                let (lbl, _) = self.expect_ident()?;
                name = lbl;
            }
        } else if raw_kind != "field" {
            // Compact delta-only field: `fieldName;` or `fieldName { readonly; label: "..."; }`
            kind = "field".to_string();
            name = raw_kind.clone();
            source_expr = Some(raw_kind);
        }

        let mut children = Vec::new();
        let mut triggers = Vec::new();

        if self.peek_kind() == &TokenKind::OpenBrace {
            self.advance();
            let is_field_like = matches!(kind.as_str(), "field" | "modify" | "tile" | "part" | "factbox");
            while self.peek_kind() != &TokenKind::CloseBrace && self.peek_kind() != &TokenKind::Eof {
                if let Some(s) = self.peek_ident() {
                    if s.starts_with("on")
                        && (self.peek_second_kind() == Some(&TokenKind::OpenParen)
                            || self.peek_second_kind() == Some(&TokenKind::OpenBrace))
                    {
                        triggers.push(self.parse_trigger()?);
                    } else if self.peek_second_kind() == Some(&TokenKind::Assign)
                        || self.peek_second_kind() == Some(&TokenKind::Colon)
                    {
                        properties.push(self.parse_property()?);
                    } else if self.peek_second_kind() == Some(&TokenKind::Semicolon)
                        && (is_field_like
                            || matches!(
                                s.as_str(),
                                "readonly"
                                    | "required"
                                    | "hidden"
                                    | "promoted"
                                    | "multiline"
                                    | "drillDown"
                                    | "editable"
                                    | "visible"
                            ))
                    {
                        let (_, sp) = self.expect_ident()?;
                        self.expect(TokenKind::Semicolon)?;
                        properties.push(Property {
                            name: s,
                            value: Expr::BoolLit(true),
                            span: sp,
                        });
                    } else {
                        children.push(self.parse_page_control()?);
                    }
                } else {
                    self.advance();
                }
            }
            self.expect(TokenKind::CloseBrace)?;
        }
        if self.peek_kind() == &TokenKind::Semicolon {
            self.advance();
        }

        Ok(PageControlDef {
            kind,
            name,
            source_expr,
            properties,
            children,
            triggers,
            span: start_span,
        })
    }

    fn parse_page_action(&mut self) -> Result<PageActionDef, ParserError> {
        let (first_ident, start_span) = self.expect_ident()?;
        let mut name = if matches!(first_ident.as_str(), "action" | "command") {
            "Action".to_string()
        } else {
            first_ident.clone()
        };
        if self.peek_kind() == &TokenKind::OpenParen {
            self.advance();
            if self.peek_kind() != &TokenKind::CloseParen {
                name = match self.peek_kind().clone() {
                    TokenKind::StringLit(s) => {
                        self.advance();
                        s
                    }
                    _ => self.expect_ident()?.0,
                };
            }
            self.expect(TokenKind::CloseParen)?;
        } else if matches!(first_ident.as_str(), "action" | "command") {
            if let TokenKind::StringLit(s) = self.peek_kind().clone() {
                self.advance();
                name = s;
            } else if self.peek_ident().is_some()
                && self.peek_kind() != &TokenKind::OpenBrace
                && self.peek_kind() != &TokenKind::Semicolon
            {
                name = self.expect_ident()?.0;
            }
        }

        let mut properties = Vec::new();
        let mut on_action = None;

        if self.peek_kind() == &TokenKind::OpenBrace {
            self.advance();
            while self.peek_kind() != &TokenKind::CloseBrace && self.peek_kind() != &TokenKind::Eof {
                if let Some(s) = self.peek_ident() {
                    if (s == "onAction" || s == "onClick" || s == "run")
                        && (self.peek_second_kind() == Some(&TokenKind::OpenParen)
                            || self.peek_second_kind() == Some(&TokenKind::OpenBrace))
                    {
                        self.advance();
                        if self.peek_kind() == &TokenKind::OpenParen {
                            self.advance();
                            while self.peek_kind() != &TokenKind::CloseParen && self.peek_kind() != &TokenKind::Eof {
                                self.advance();
                            }
                            self.expect(TokenKind::CloseParen)?;
                        }
                        on_action = Some(self.parse_block()?);
                    } else if matches!(s.as_str(), "action" | "command")
                        && self.peek_second_kind() == Some(&TokenKind::OpenParen)
                    {
                        let nested = self.parse_page_action()?;
                        properties.push(Property {
                            name: nested.name,
                            value: Expr::BoolLit(true),
                            span: nested.span,
                        });
                    } else if self.peek_second_kind() == Some(&TokenKind::Assign)
                        || self.peek_second_kind() == Some(&TokenKind::Colon)
                    {
                        properties.push(self.parse_property()?);
                    } else if self.peek_second_kind() == Some(&TokenKind::Semicolon) {
                        let (_, sp) = self.expect_ident()?;
                        self.expect(TokenKind::Semicolon)?;
                        properties.push(Property {
                            name: s,
                            value: Expr::BoolLit(true),
                            span: sp,
                        });
                    } else {
                        self.advance();
                    }
                } else {
                    self.advance();
                }
            }
            self.expect(TokenKind::CloseBrace)?;
        }
        if self.peek_kind() == &TokenKind::Semicolon {
            self.advance();
        }

        Ok(PageActionDef {
            name,
            properties,
            on_action,
            span: start_span,
        })
    }

    fn parse_menu(&mut self) -> Result<MenuDef, ParserError> {
        let (_, start_span) = self.expect_ident()?; // "menu"
        let (name, _) = self.expect_ident()?;
        self.expect(TokenKind::OpenBrace)?;

        let mut properties = Vec::new();
        let mut items = Vec::new();

        while self.peek_kind() != &TokenKind::CloseBrace && self.peek_kind() != &TokenKind::Eof {
            if let Some(_s) = self.peek_ident() {
                if self.peek_second_kind() == Some(&TokenKind::Assign)
                    || self.peek_second_kind() == Some(&TokenKind::Colon)
                {
                    properties.push(self.parse_property()?);
                } else {
                    items.push(self.parse_menu_item()?);
                }
            } else {
                self.advance();
            }
        }
        let end_tok = self.expect(TokenKind::CloseBrace)?;

        Ok(MenuDef {
            name,
            properties,
            items,
            span: Span::new(start_span.start, end_tok.span.end, start_span.line, start_span.column),
        })
    }

    fn parse_menu_item(&mut self) -> Result<MenuItemDef, ParserError> {
        let (first, start_span) = self.expect_ident()?;
        let mut name = first.clone();
        let mut properties = Vec::new();

        if self.peek_kind() == &TokenKind::OpenParen {
            self.advance();
            if self.peek_kind() != &TokenKind::CloseParen {
                name = match self.peek_kind().clone() {
                    TokenKind::StringLit(s) => {
                        self.advance();
                        s
                    }
                    _ => self.expect_ident()?.0,
                };
            }
            self.expect(TokenKind::CloseParen)?;
        } else if matches!(first.as_str(), "group" | "item" | "section") {
            if let TokenKind::StringLit(s) = self.peek_kind().clone() {
                let sp = self.advance().span;
                name = s.clone();
                properties.push(Property {
                    name: "label".to_string(),
                    value: Expr::StringLit(s),
                    span: sp,
                });
            } else if self.peek_ident().is_some()
                && self.peek_kind() != &TokenKind::OpenBrace
                && self.peek_kind() != &TokenKind::Semicolon
            {
                name = self.expect_ident()?.0;
            }
        }

        let mut items = Vec::new();
        if self.peek_kind() == &TokenKind::OpenBrace {
            self.advance();
            while self.peek_kind() != &TokenKind::CloseBrace && self.peek_kind() != &TokenKind::Eof {
                if let Some(_s) = self.peek_ident() {
                    if self.peek_second_kind() == Some(&TokenKind::Assign)
                        || self.peek_second_kind() == Some(&TokenKind::Colon)
                    {
                        properties.push(self.parse_property()?);
                    } else {
                        items.push(self.parse_menu_item()?);
                    }
                } else {
                    self.advance();
                }
            }
            self.expect(TokenKind::CloseBrace)?;
        }
        if self.peek_kind() == &TokenKind::Semicolon {
            self.advance();
        }

        Ok(MenuItemDef {
            name,
            properties,
            items,
            span: start_span,
        })
    }

    fn parse_report(&mut self) -> Result<ReportDef, ParserError> {
        let start_tok = self.expect(TokenKind::Report)?;
        let (name, _) = self.expect_ident()?;
        let mut source_table = if self.peek_kind() == &TokenKind::For {
            self.advance();
            Some(self.expect_ident()?.0)
        } else {
            None
        };
        self.expect(TokenKind::OpenBrace)?;

        let mut properties = Vec::new();
        let mut elements = Vec::new();
        let mut triggers = Vec::new();

        while self.peek_kind() != &TokenKind::CloseBrace && self.peek_kind() != &TokenKind::Eof {
            match self.peek_kind().clone() {
                TokenKind::Elements => {
                    self.advance();
                    self.expect(TokenKind::OpenBrace)?;
                    while self.peek_kind() != &TokenKind::CloseBrace && self.peek_kind() != &TokenKind::Eof {
                        if self.peek_kind() == &TokenKind::Dataitem {
                            elements.push(self.parse_dataitem()?);
                        } else {
                            self.advance();
                        }
                    }
                    self.expect(TokenKind::CloseBrace)?;
                    if self.peek_kind() == &TokenKind::Semicolon {
                        self.advance();
                    }
                }
                TokenKind::Dataitem => {
                    elements.push(self.parse_dataitem()?);
                }
                TokenKind::Ident(s) if s == "dataset" => {
                    self.advance();
                    if self.peek_kind() == &TokenKind::Colon || self.peek_kind() == &TokenKind::Assign {
                        self.advance();
                        let (tbl_name, _) = self.expect_ident()?;
                        source_table = Some(tbl_name);
                        if self.peek_kind() == &TokenKind::Semicolon {
                            self.advance();
                        }
                    } else {
                        self.expect(TokenKind::OpenBrace)?;
                        while self.peek_kind() != &TokenKind::CloseBrace && self.peek_kind() != &TokenKind::Eof {
                            if self.peek_kind() == &TokenKind::Dataitem {
                                elements.push(self.parse_dataitem()?);
                            } else {
                                self.advance();
                            }
                        }
                        self.expect(TokenKind::CloseBrace)?;
                        if self.peek_kind() == &TokenKind::Semicolon {
                            self.advance();
                        }
                    }
                }
                TokenKind::Ident(s) if s.starts_with("on") && self.peek_second_kind() == Some(&TokenKind::OpenParen) => {
                    triggers.push(self.parse_trigger()?);
                }
                TokenKind::Ident(_)
                    if self.peek_second_kind() == Some(&TokenKind::Assign)
                        || self.peek_second_kind() == Some(&TokenKind::Colon) =>
                {
                    let prop = self.parse_property()?;
                    if prop.name == "sourceTable" || prop.name == "source_table" {
                        if let Expr::Ident(ref tbl) | Expr::StringLit(ref tbl) = prop.value {
                            source_table = Some(tbl.clone());
                        }
                    }
                    properties.push(prop);
                }
                TokenKind::OpenBrace => {
                    let blk = self.parse_block()?;
                    triggers.push(TriggerDef {
                        name: "onRun".to_string(),
                        params: Vec::new(),
                        body: blk.clone(),
                        span: blk.span,
                    });
                }
                _ => {
                    self.advance();
                }
            }
        }
        let end_tok = self.expect(TokenKind::CloseBrace)?;

        Ok(ReportDef {
            name,
            source_table,
            properties,
            elements,
            triggers,
            span: Span::new(start_tok.span.start, end_tok.span.end, start_tok.span.line, start_tok.span.column),
        })
    }

    fn parse_custom_metadata(&mut self) -> Result<CustomMetadataDef, ParserError> {
        let start_tok = self.expect(TokenKind::CustomMetadata)?;
        let (name, _) = self.expect_ident()?;
        self.expect(TokenKind::OpenBrace)?;

        let mut properties = Vec::new();
        let mut fields = Vec::new();
        let mut records = Vec::new();

        while self.peek_kind() != &TokenKind::CloseBrace && self.peek_kind() != &TokenKind::Eof {
            if let TokenKind::Ident(s) = self.peek_kind().clone() {
                if s == "record" {
                    self.advance();
                    if self.peek_kind() == &TokenKind::OpenParen {
                        self.advance();
                        while self.peek_kind() != &TokenKind::CloseParen && self.peek_kind() != &TokenKind::Eof {
                            self.advance();
                        }
                        self.expect(TokenKind::CloseParen)?;
                    }
                    self.expect(TokenKind::OpenBrace)?;
                    let mut rec_props = Vec::new();
                    while self.peek_kind() != &TokenKind::CloseBrace && self.peek_kind() != &TokenKind::Eof {
                        if self.peek_second_kind() == Some(&TokenKind::Assign) {
                            rec_props.push(self.parse_property()?);
                        } else {
                            self.advance();
                        }
                    }
                    self.expect(TokenKind::CloseBrace)?;
                    if self.peek_kind() == &TokenKind::Semicolon {
                        self.advance();
                    }
                    records.push(rec_props);
                } else if self.peek_second_kind() == Some(&TokenKind::Colon) {
                    fields.push(self.parse_field()?);
                } else if self.peek_second_kind() == Some(&TokenKind::Assign) {
                    properties.push(self.parse_property()?);
                } else {
                    self.advance();
                }
            } else {
                self.advance();
            }
        }
        let end_tok = self.expect(TokenKind::CloseBrace)?;

        Ok(CustomMetadataDef {
            name,
            properties,
            fields,
            records,
            span: Span::new(start_tok.span.start, end_tok.span.end, start_tok.span.line, start_tok.span.column),
        })
    }


    fn dedent_block_source(raw: &str) -> String {
        let normalized = raw.replace("\r\n", "\n");
        let lines: Vec<&str> = normalized.split('\n').collect();
        let first_non_empty = lines.iter().position(|l| !l.trim().is_empty());
        let last_non_empty = lines.iter().rposition(|l| !l.trim().is_empty());
        let (Some(start), Some(end)) = (first_non_empty, last_non_empty) else {
            return String::new();
        };
        let slice = &lines[start..=end];
        if slice.len() == 1 {
            return slice[0].trim().to_string();
        }

        let leading_spaces = |s: &str| s.chars().take_while(|c| *c == ' ' || *c == '\t').count();
        let first_indent = leading_spaces(slice[0]);
        let rest_min_indent = slice[1..]
            .iter()
            .filter(|l| !l.trim().is_empty())
            .map(|l| leading_spaces(l))
            .min()
            .unwrap_or(0);

        let common_indent = if first_indent == 0 && rest_min_indent > 0 {
            rest_min_indent
        } else {
            slice
                .iter()
                .filter(|l| !l.trim().is_empty())
                .map(|l| leading_spaces(l))
                .min()
                .unwrap_or(0)
        };

        slice
            .iter()
            .enumerate()
            .map(|(idx, line)| {
                if line.trim().is_empty() {
                    String::new()
                } else if idx == 0 && first_indent == 0 {
                    line.trim().to_string()
                } else {
                    let strip = line
                        .chars()
                        .take(common_indent)
                        .take_while(|c| *c == ' ' || *c == '\t')
                        .count();
                    line.chars().skip(strip).collect::<String>().trim_end().to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    // ----------------------------------------------------
    // PARSE STATEMENTS & BLOCKS
    // ----------------------------------------------------
    fn parse_block(&mut self) -> Result<Block, ParserError> {
        let start_tok = self.expect(TokenKind::OpenBrace)?;
        let start_offset = start_tok.span.end;

        if self.source.is_some() {
            let mut depth = 1;
            let mut end_offset = start_offset;
            while depth > 0 && self.peek_kind() != &TokenKind::Eof {
                match self.peek_kind() {
                    TokenKind::OpenBrace => depth += 1,
                    TokenKind::CloseBrace => {
                        depth -= 1;
                        if depth == 0 {
                            end_offset = self.peek().span.start;
                            break;
                        }
                    }
                    _ => {}
                }
                self.advance();
            }
            let end_tok = self.expect(TokenKind::CloseBrace)?;
            let raw_source = self
                .source
                .as_ref()
                .and_then(|s| s.get(start_offset..end_offset))
                .map(Self::dedent_block_source);
            return Ok(Block {
                statements: Vec::new(),
                raw_source,
                span: Span::new(start_tok.span.start, end_tok.span.end, start_tok.span.line, start_tok.span.column),
            });
        }

        let mut statements = Vec::new();
        while self.peek_kind() != &TokenKind::CloseBrace && self.peek_kind() != &TokenKind::Eof {
            statements.push(self.parse_stmt()?);
        }

        let end_tok = self.expect(TokenKind::CloseBrace)?;
        Ok(Block {
            statements,
            raw_source: None,
            span: Span::new(start_tok.span.start, end_tok.span.end, start_tok.span.line, start_tok.span.column),
        })
    }

    fn parse_stmt(&mut self) -> Result<Stmt, ParserError> {
        match self.peek_kind() {
            TokenKind::Return => {
                self.advance();
                let ret_expr = if self.peek_kind() != &TokenKind::Semicolon {
                    Some(self.parse_expr()?)
                } else {
                    None
                };
                self.expect(TokenKind::Semicolon)?;
                Ok(Stmt::Return(ret_expr))
            }
            TokenKind::If => {
                self.advance();
                self.expect(TokenKind::OpenParen)?;
                let condition = self.parse_expr()?;
                self.expect(TokenKind::CloseParen)?;
                let then_branch = self.parse_block()?;
                let else_branch = if self.peek_kind() == &TokenKind::Else {
                    self.advance();
                    Some(self.parse_block()?)
                } else {
                    None
                };
                Ok(Stmt::If {
                    condition,
                    then_branch,
                    else_branch,
                })
            }
            _ => {
                let expr = self.parse_expr()?;
                if self.peek_kind() == &TokenKind::Assign {
                    self.advance();
                    let val = self.parse_expr()?;
                    self.expect(TokenKind::Semicolon)?;
                    Ok(Stmt::Assign {
                        target: expr,
                        value: val,
                    })
                } else {
                    if self.peek_kind() == &TokenKind::Semicolon {
                        self.advance();
                    }
                    Ok(Stmt::Expr(expr))
                }
            }
        }
    }

    // ----------------------------------------------------
    // PARSE EXPRESSIONS
    // ----------------------------------------------------
    fn parse_expr(&mut self) -> Result<Expr, ParserError> {
        self.parse_logical_or()
    }

    fn parse_logical_or(&mut self) -> Result<Expr, ParserError> {
        let mut left = self.parse_logical_and()?;
        while self.peek_kind() == &TokenKind::Or {
            self.advance();
            let right = self.parse_logical_and()?;
            left = Expr::Binary {
                left: Box::new(left),
                op: BinaryOp::Or,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_logical_and(&mut self) -> Result<Expr, ParserError> {
        let mut left = self.parse_equality()?;
        while self.peek_kind() == &TokenKind::And {
            self.advance();
            let right = self.parse_equality()?;
            left = Expr::Binary {
                left: Box::new(left),
                op: BinaryOp::And,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_equality(&mut self) -> Result<Expr, ParserError> {
        let mut left = self.parse_comparison()?;
        while let TokenKind::Eq | TokenKind::NotEq = self.peek_kind() {
            let op = match self.advance().kind {
                TokenKind::Eq => BinaryOp::Eq,
                TokenKind::NotEq => BinaryOp::NotEq,
                _ => unreachable!(),
            };
            let right = self.parse_comparison()?;
            left = Expr::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_comparison(&mut self) -> Result<Expr, ParserError> {
        let mut left = self.parse_additive()?;
        while let TokenKind::Lt | TokenKind::Lte | TokenKind::Gt | TokenKind::Gte | TokenKind::In = self.peek_kind() {
            let op = match self.advance().kind {
                TokenKind::Lt => BinaryOp::Lt,
                TokenKind::Lte => BinaryOp::Lte,
                TokenKind::Gt => BinaryOp::Gt,
                TokenKind::Gte => BinaryOp::Gte,
                TokenKind::In => BinaryOp::In,
                _ => unreachable!(),
            };
            let right = self.parse_additive()?;
            left = Expr::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_additive(&mut self) -> Result<Expr, ParserError> {
        let mut left = self.parse_multiplicative()?;
        while let TokenKind::Plus | TokenKind::Minus = self.peek_kind() {
            let op = match self.advance().kind {
                TokenKind::Plus => BinaryOp::Add,
                TokenKind::Minus => BinaryOp::Sub,
                _ => unreachable!(),
            };
            let right = self.parse_multiplicative()?;
            left = Expr::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_multiplicative(&mut self) -> Result<Expr, ParserError> {
        let mut left = self.parse_unary()?;
        while let TokenKind::Star | TokenKind::Slash = self.peek_kind() {
            let op = match self.advance().kind {
                TokenKind::Star => BinaryOp::Mul,
                TokenKind::Slash => BinaryOp::Div,
                _ => unreachable!(),
            };
            let right = self.parse_unary()?;
            left = Expr::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expr, ParserError> {
        if self.peek_kind() == &TokenKind::Not {
            self.advance();
            let operand = self.parse_unary()?;
            return Ok(Expr::Unary {
                op: UnaryOp::Not,
                operand: Box::new(operand),
            });
        } else if self.peek_kind() == &TokenKind::Minus {
            self.advance();
            let operand = self.parse_unary()?;
            return Ok(Expr::Unary {
                op: UnaryOp::Neg,
                operand: Box::new(operand),
            });
        }
        self.parse_postfix()
    }

    fn parse_postfix(&mut self) -> Result<Expr, ParserError> {
        let mut expr = self.parse_primary()?;

        loop {
            match self.peek_kind() {
                TokenKind::Dot => {
                    self.advance();
                    let (member, _) = self.expect_ident()?;
                    expr = Expr::MemberAccess {
                        object: Box::new(expr),
                        member,
                    };
                }
                TokenKind::OpenParen => {
                    self.advance();
                    let mut args = Vec::new();
                    while self.peek_kind() != &TokenKind::CloseParen {
                        args.push(self.parse_expr()?);
                        if self.peek_kind() == &TokenKind::Comma {
                            self.advance();
                        }
                    }
                    self.expect(TokenKind::CloseParen)?;
                    expr = Expr::Call {
                        callee: Box::new(expr),
                        args,
                    };
                }
                _ => break,
            }
        }

        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<Expr, ParserError> {
        match self.peek_kind().clone() {
            TokenKind::Ident(s) => {
                self.advance();
                Ok(Expr::Ident(s))
            }
            TokenKind::StringLit(s) => {
                self.advance();
                Ok(Expr::StringLit(s))
            }
            TokenKind::IntLit(n) => {
                self.advance();
                Ok(Expr::IntLit(n))
            }
            TokenKind::FloatLit(f) => {
                self.advance();
                Ok(Expr::FloatLit(f))
            }
            TokenKind::True => {
                self.advance();
                Ok(Expr::BoolLit(true))
            }
            TokenKind::False => {
                self.advance();
                Ok(Expr::BoolLit(false))
            }
            TokenKind::Null => {
                self.advance();
                Ok(Expr::NullLit)
            }
            TokenKind::OpenBracket => {
                self.advance();
                let mut elements = Vec::new();
                while self.peek_kind() != &TokenKind::CloseBracket {
                    elements.push(self.parse_expr()?);
                    if self.peek_kind() == &TokenKind::Comma {
                        self.advance();
                    }
                }
                self.expect(TokenKind::CloseBracket)?;
                Ok(Expr::ArrayLit(elements))
            }
            TokenKind::OpenParen => {
                self.advance();
                let expr = self.parse_expr()?;
                self.expect(TokenKind::CloseParen)?;
                Ok(expr)
            }
            _ => {
                if let Some(s) = self.peek_ident() {
                    self.advance();
                    Ok(Expr::Ident(s))
                } else {
                    Err(ParserError::UnexpectedToken {
                        expected: "expression".to_string(),
                        found: self.peek_kind().clone(),
                        line: self.peek().span.line,
                        col: self.peek().span.column,
                    })
                }
            }
        }
    }
}
