with open('crates/bosc-syntax/src/parser.rs', 'r', encoding='utf-8') as f:
    content = f.read()

# 1. Add format_with_source to ParserError
old_err_def = """#[derive(Debug, Error)]
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
}"""

new_err_def = """#[derive(Debug, Error)]
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
    pub fn format_with_source(&self, source: &str, file_name: &str) -> String {
        match self {
            ParserError::UnexpectedToken { expected, found, line, col } => {
                let lines: Vec<&str> = source.lines().collect();
                let line_idx = if *line > 0 { line - 1 } else { 0 };
                let mut out = format!("error: expected {}, found {:?}\\n  --> {}:{}:{}\\n", expected, found, file_name, line, col);
                if line_idx < lines.len() {
                    let source_line = lines[line_idx];
                    out.push_str(&format!("{:>5} | {}\\n", line, source_line));
                    let indent = " ".repeat(col.saturating_sub(1));
                    out.push_str(&format!("      | {}^ expected {}\\n", indent, expected));
                }
                out
            }
            ParserError::UnexpectedEof => {
                format!("error: unexpected end of file\\n  --> {}:EOF\\n", file_name)
            }
        }
    }
}"""

# 2. Update parse_table_logic to support validate(name) and validate name(val)
old_val_logic = """                if s == "validate" {
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
                }"""

new_val_logic = """                if s == "validate" {
                    self.advance();
                    let (field_name, val_span, param_name, old_param_name) = if self.peek_kind() == &TokenKind::OpenParen {
                        self.advance();
                        let (f_name, f_span) = self.expect_ident()?;
                        let (p_name, o_name) = if self.peek_kind() == &TokenKind::Comma {
                            self.advance();
                            let (p, _) = self.expect_ident()?;
                            let o = if self.peek_kind() == &TokenKind::Comma {
                                self.advance();
                                Some(self.expect_ident()?.0)
                            } else {
                                None
                            };
                            (p, o)
                        } else {
                            ("rec".to_string(), None)
                        };
                        self.expect(TokenKind::CloseParen)?;
                        (f_name, f_span, p_name, o_name)
                    } else {
                        let (f_name, f_span) = self.expect_ident()?;
                        self.expect(TokenKind::OpenParen)?;
                        let (param, _) = self.expect_ident()?;
                        if self.peek_kind() == &TokenKind::Colon {
                            self.advance();
                            self.parse_type_annotation_str()?;
                        }
                        let old_param = if self.peek_kind() == &TokenKind::Comma {
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
                        (f_name, f_span, param, old_param)
                    };
                    let body = self.parse_block()?;
                    field_validations.push(FieldValidationDef {
                        field_name,
                        param_name,
                        old_param_name,
                        body,
                        span: val_span,
                    });
                    continue;
                }"""

# 3. Update parse_state_machine_ext to support states: [...] and parse_state to support semicolon
old_sm_ext_states = """                TokenKind::Ident(s) if s == "states" => {
                    self.advance();
                    self.expect(TokenKind::OpenBrace)?;
                    while self.peek_kind() != &TokenKind::CloseBrace {
                        states.push(self.parse_state()?);
                    }
                    self.expect(TokenKind::CloseBrace)?;
                }"""

new_sm_ext_states = """                TokenKind::Ident(s) if s == "states" => {
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
                        while self.peek_kind() != &TokenKind::CloseBrace {
                            states.push(self.parse_state()?);
                        }
                        self.expect(TokenKind::CloseBrace)?;
                    }
                }"""

old_parse_state = """        self.expect(TokenKind::OpenBrace)?;
        let mut properties = Vec::new();
        let mut timeout = None;"""

new_parse_state = """        let mut properties = Vec::new();
        let mut timeout = None;

        if self.peek_kind() == &TokenKind::Semicolon {
            self.advance();
            return Ok(StateDef {
                name,
                properties,
                timeout,
                span: start_span,
            });
        }

        self.expect(TokenKind::OpenBrace)?;"""

# 4. Update parse_permission_set to accept either semicolon or comma
old_perm_semi = """                        self.expect(TokenKind::Semicolon)?;"""
new_perm_semi = """                        if self.peek_kind() == &TokenKind::Comma || self.peek_kind() == &TokenKind::Semicolon {
                            self.advance();
                        }"""

# 5. Update parse_view where filter to support where { ... }
old_view_where = """                TokenKind::Where => {
                    self.advance();
                    where_filter = Some(self.parse_expr()?);
                    if self.peek_kind() == &TokenKind::Semicolon {
                        self.advance();
                    }
                }"""

new_view_where = """                TokenKind::Where => {
                    self.advance();
                    let has_brace = if self.peek_kind() == &TokenKind::OpenBrace {
                        self.advance();
                        true
                    } else {
                        false
                    };
                    where_filter = Some(self.parse_expr()?);
                    if has_brace {
                        self.expect(TokenKind::CloseBrace)?;
                    }
                    if self.peek_kind() == &TokenKind::Semicolon {
                        self.advance();
                    }
                }"""

content_norm = content.replace("\r\n", "\n")

assert old_err_def.replace("\r\n", "\n") in content_norm, "old_err_def not found"
assert old_val_logic.replace("\r\n", "\n") in content_norm, "old_val_logic not found"
assert old_sm_ext_states.replace("\r\n", "\n") in content_norm, "old_sm_ext_states not found"
assert old_parse_state.replace("\r\n", "\n") in content_norm, "old_parse_state not found"
assert old_perm_semi.replace("\r\n", "\n") in content_norm, "old_perm_semi not found"
assert old_view_where.replace("\r\n", "\n") in content_norm, "old_view_where not found"

content_norm = content_norm.replace(old_err_def.replace("\r\n", "\n"), new_err_def.replace("\r\n", "\n"), 1)
content_norm = content_norm.replace(old_val_logic.replace("\r\n", "\n"), new_val_logic.replace("\r\n", "\n"))
content_norm = content_norm.replace(old_sm_ext_states.replace("\r\n", "\n"), new_sm_ext_states.replace("\r\n", "\n"), 1)
content_norm = content_norm.replace(old_parse_state.replace("\r\n", "\n"), new_parse_state.replace("\r\n", "\n"), 1)
content_norm = content_norm.replace(old_perm_semi.replace("\r\n", "\n"), new_perm_semi.replace("\r\n", "\n"), 1)
content_norm = content_norm.replace(old_view_where.replace("\r\n", "\n"), new_view_where.replace("\r\n", "\n"), 1)

with open('crates/bosc-syntax/src/parser.rs', 'w', encoding='utf-8', newline='\r\n') as f:
    f.write(content_norm)

print("Enhanced crates/bosc-syntax/src/parser.rs successfully!")
