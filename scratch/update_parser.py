import re

with open('crates/bosc-syntax/src/parser.rs', 'r', encoding='utf-8') as f:
    content = f.read()

# Replace in parse_table:
old_table_block = """                TokenKind::Ident(s)
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
                }"""

new_table_block = """                _ if self.peek_ident().is_some() => {
                    let s = self.peek_ident().unwrap();
                    if s == "fieldgroup"
                        && self.peek_second_kind() != Some(&TokenKind::Colon)
                        && self.peek_second_kind() != Some(&TokenKind::Assign)
                    {
                        field_groups.push(self.parse_field_group()?);
                    } else if s == "fieldgroups" && self.peek_second_kind() == Some(&TokenKind::OpenBrace) {
                        self.advance();
                        self.expect(TokenKind::OpenBrace)?;
                        while self.peek_kind() != &TokenKind::CloseBrace && self.peek_kind() != &TokenKind::Eof {
                            if let Some(inner) = self.peek_ident() {
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
                    } else if (s.starts_with("on") || s.starts_with("before") || s.starts_with("after"))
                        && self.peek_second_kind() == Some(&TokenKind::OpenParen)
                    {
                        // Trigger
                        triggers.push(self.parse_trigger()?);
                    } else {
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
                }"""

# Also update parse_table_ext:
old_ext_block = """                TokenKind::Ident(s) if s.starts_with("on") => {
                    triggers.push(self.parse_trigger()?);
                }
                TokenKind::Ident(_) => {
                    fields.push(self.parse_field()?);
                }"""

new_ext_block = """                _ if self.peek_ident().is_some() => {
                    let s = self.peek_ident().unwrap();
                    if s.starts_with("on") && self.peek_second_kind() == Some(&TokenKind::OpenParen) {
                        triggers.push(self.parse_trigger()?);
                    } else {
                        fields.push(self.parse_field()?);
                    }
                }"""

content_norm = content.replace("\r\n", "\n")
old_table_norm = old_table_block.replace("\r\n", "\n")
old_ext_norm = old_ext_block.replace("\r\n", "\n")

assert old_table_norm in content_norm, "old_table_block not found in parser.rs"
assert old_ext_norm in content_norm, "old_ext_block not found in parser.rs"

content_norm = content_norm.replace(old_table_norm, new_table_block.replace("\r\n", "\n"))
content_norm = content_norm.replace(old_ext_norm, new_ext_block.replace("\r\n", "\n"))

with open('crates/bosc-syntax/src/parser.rs', 'w', encoding='utf-8', newline='\r\n') as f:
    f.write(content_norm)

print("Updated parser.rs successfully!")
