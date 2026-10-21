use crate::token::{Span, Token, TokenKind};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum LexerError {
    #[error("Unexpected character '{0}' at line {1}, col {2}")]
    UnexpectedChar(char, usize, usize),
    #[error("Unterminated string literal at line {0}, col {1}")]
    UnterminatedString(usize, usize),
}

impl LexerError {
    pub fn line(&self) -> usize {
        match self {
            Self::UnexpectedChar(_, line, _) | Self::UnterminatedString(line, _) => *line,
        }
    }

    pub fn column(&self) -> usize {
        match self {
            Self::UnexpectedChar(_, _, col) | Self::UnterminatedString(_, col) => *col,
        }
    }

    pub fn hint(&self) -> Option<String> {
        match self {
            Self::UnexpectedChar(ch, _, _) => Some(format!(
                "Remove or escape the unsupported character '{}'.",
                ch
            )),
            Self::UnterminatedString(_, _) => Some(
                "Close the string with a matching quote (\" or ') or backtick (`).".to_string(),
            ),
        }
    }
}

pub struct Lexer<'a> {
    source: &'a str,
    chars: Vec<(usize, char)>,
    cursor: usize,
    line: usize,
    col: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str) -> Self {
        let chars: Vec<(usize, char)> = source.char_indices().collect();
        Self {
            source,
            chars,
            cursor: 0,
            line: 1,
            col: 1,
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.cursor).map(|(_, c)| *c)
    }

    fn peek_next(&self) -> Option<char> {
        self.chars.get(self.cursor + 1).map(|(_, c)| *c)
    }

    fn advance(&mut self) -> Option<char> {
        if let Some((_, c)) = self.chars.get(self.cursor) {
            self.cursor += 1;
            if *c == '\n' {
                self.line += 1;
                self.col = 1;
            } else {
                self.col += 1;
            }
            Some(*c)
        } else {
            None
        }
    }

    fn current_byte_offset(&self) -> usize {
        self.chars.get(self.cursor).map(|(i, _)| *i).unwrap_or(self.source.len())
    }

    fn save_pos(&self) -> (usize, usize, usize) {
        (self.cursor, self.line, self.col)
    }

    fn restore_pos(&mut self, pos: (usize, usize, usize)) {
        self.cursor = pos.0;
        self.line = pos.1;
        self.col = pos.2;
    }

    /// `/` starts a regex after an operator or a keyword, and divides after a value.
    fn regex_allowed(kind: &TokenKind) -> bool {
        match kind {
            TokenKind::Ident(name) => matches!(
                name.as_str(),
                "return"
                    | "throw"
                    | "case"
                    | "typeof"
                    | "void"
                    | "delete"
                    | "in"
                    | "of"
                    | "await"
                    | "yield"
                    | "instanceof"
                    | "new"
            ),
            TokenKind::Return
            | TokenKind::If
            | TokenKind::Else
            | TokenKind::OpenBrace
            | TokenKind::OpenParen
            | TokenKind::OpenBracket
            | TokenKind::Comma
            | TokenKind::Semicolon
            | TokenKind::Colon
            | TokenKind::Assign
            | TokenKind::Eq
            | TokenKind::NotEq
            | TokenKind::Lt
            | TokenKind::Lte
            | TokenKind::Gt
            | TokenKind::Gte
            | TokenKind::Plus
            | TokenKind::Minus
            | TokenKind::Star
            | TokenKind::Not
            | TokenKind::Question
            | TokenKind::And
            | TokenKind::Or
            | TokenKind::Arrow => true,
            _ => false,
        }
    }

    /// Consume `/pattern/flags` when it is closed before a newline. Restore the cursor otherwise.
    fn try_consume_regex(&mut self) -> bool {
        let saved = self.save_pos();
        self.advance();
        let mut in_class = false;
        while let Some(ch) = self.peek() {
            if ch == '\n' {
                self.restore_pos(saved);
                return false;
            }
            self.advance();
            if ch == '\\' {
                if self.peek().is_some() {
                    self.advance();
                }
                continue;
            }
            if ch == '[' && !in_class {
                in_class = true;
                continue;
            }
            if ch == ']' && in_class {
                in_class = false;
                continue;
            }
            if ch == '/' && !in_class {
                while let Some(flag) = self.peek() {
                    if flag.is_ascii_alphabetic() {
                        self.advance();
                    } else {
                        break;
                    }
                }
                return true;
            }
        }
        self.restore_pos(saved);
        false
    }

    pub fn tokenize(&mut self) -> Result<Vec<Token>, LexerError> {
        let mut tokens = Vec::new();

        while let Some(c) = self.peek() {
            // Skip whitespace
            if c.is_whitespace() {
                self.advance();
                continue;
            }

            // Skip comments
            if c == '/' {
                if let Some('/') = self.peek_next() {
                    // Single-line comment
                    while let Some(ch) = self.peek() {
                        self.advance();
                        if ch == '\n' {
                            break;
                        }
                    }
                    continue;
                } else if let Some('*') = self.peek_next() {
                    // Multi-line comment
                    self.advance(); // consume '/'
                    self.advance(); // consume '*'
                    while let Some(ch) = self.peek() {
                        if ch == '*' && self.peek_next() == Some('/') {
                            self.advance(); // consume '*'
                            self.advance(); // consume '/'
                            break;
                        }
                        self.advance();
                    }
                    continue;
                }
            }

            let start_pos = self.current_byte_offset();
            let start_line = self.line;
            let start_col = self.col;

            // Identifiers and Keywords
            if c.is_alphabetic() || c == '_' || c == '$' {
                let mut ident = String::new();
                while let Some(ch) = self.peek() {
                    if ch.is_alphanumeric() || ch == '_' || ch == '$' {
                        ident.push(ch);
                        self.advance();
                    } else {
                        break;
                    }
                }

                let kind = match ident.as_str() {
                    "table" => TokenKind::Table,
                    "tableextension" => TokenKind::TableExtension,
                    "tablelogic" => TokenKind::TableLogic,
                    "tablelogicextension" => TokenKind::TableLogicExtension,
                    "statemachine" => TokenKind::StateMachine,
                    "statemachineextension" => TokenKind::StateMachineExtension,
                    "codeunit" => TokenKind::Codeunit,
                    "interface" => TokenKind::Interface,
                    "enum" => TokenKind::Enum,
                    "enumextension" => TokenKind::EnumExtension,
                    "page" => TokenKind::Page,
                    "pageextension" => TokenKind::PageExtension,
                    "view" => TokenKind::View,
                    "query" => TokenKind::Query,
                    "report" => TokenKind::Report,
                    "custommetadata" => TokenKind::CustomMetadata,
                    "permissionset" => TokenKind::PermissionSet,
                    "elements" => TokenKind::Elements,
                    "dataitem" => TokenKind::Dataitem,
                    "select" => TokenKind::Select,
                    "from" => TokenKind::From,
                    "leftJoin" => TokenKind::LeftJoin,
                    "innerJoin" => TokenKind::InnerJoin,
                    "groupBy" => TokenKind::GroupBy,
                    "having" => TokenKind::Having,
                    "orderBy" => TokenKind::OrderBy,
                    "extends" => TokenKind::Extends,
                    "implements" => TokenKind::Implements,
                    "for" => TokenKind::For,
                    "state" => TokenKind::State,
                    "transition" => TokenKind::Transition,
                    "guard" => TokenKind::Guard,
                    "approvalMatrix" => TokenKind::ApprovalMatrix,
                    "tier" => TokenKind::Tier,
                    "params" => TokenKind::Params,
                    "timeout" => TokenKind::Timeout,
                    "escalateTo" => TokenKind::EscalateTo,
                    "primaryKey" => TokenKind::PrimaryKey,
                    "key" => TokenKind::Key,
                    "relation" => TokenKind::Relation,
                    "computed" => TokenKind::Computed,
                    "FlowFilter" => TokenKind::FlowFilter,
                    "where" => TokenKind::Where,
                    "in" => TokenKind::In,
                    "true" => TokenKind::True,
                    "false" => TokenKind::False,
                    "null" => TokenKind::Null,
                    "public" => TokenKind::Public,
                    "private" => TokenKind::Private,
                    "static" => TokenKind::Static,
                    "function" => TokenKind::Function,
                    "return" => TokenKind::Return,
                    "if" => TokenKind::If,
                    "else" => TokenKind::Else,
                    _ => TokenKind::Ident(ident),
                };

                let end_pos = self.current_byte_offset();
                tokens.push(Token::new(kind, Span::new(start_pos, end_pos, start_line, start_col)));
                continue;
            }

            // String and template literals
            if c == '"' || c == '\'' || c == '`' {
                let quote = c;
                self.advance();
                let mut string_val = String::new();
                let mut terminated = false;

                while let Some(ch) = self.peek() {
                    if ch == quote {
                        self.advance();
                        terminated = true;
                        break;
                    } else if ch == '\\' {
                        self.advance();
                        if let Some(escaped) = self.peek() {
                            match escaped {
                                'n' => string_val.push('\n'),
                                'r' => string_val.push('\r'),
                                't' => string_val.push('\t'),
                                '\\' => string_val.push('\\'),
                                '"' => string_val.push('"'),
                                '\'' => string_val.push('\''),
                                _ => string_val.push(escaped),
                            }
                            self.advance();
                        }
                    } else {
                        string_val.push(ch);
                        self.advance();
                    }
                }

                if !terminated {
                    return Err(LexerError::UnterminatedString(start_line, start_col));
                }

                let end_pos = self.current_byte_offset();
                tokens.push(Token::new(
                    TokenKind::StringLit(string_val),
                    Span::new(start_pos, end_pos, start_line, start_col),
                ));
                continue;
            }

            // Numeric literals
            if c.is_ascii_digit() {
                let mut num_str = String::new();
                let mut is_float = false;

                while let Some(ch) = self.peek() {
                    if ch.is_ascii_digit() || ch == '_' {
                        if ch != '_' {
                            num_str.push(ch);
                        }
                        self.advance();
                    } else if ch == '.' && self.peek_next().map(|p| p.is_ascii_digit()).unwrap_or(false) {
                        is_float = true;
                        num_str.push('.');
                        self.advance();
                    } else {
                        break;
                    }
                }

                let end_pos = self.current_byte_offset();
                let kind = if is_float {
                    let val: f64 = num_str.parse().unwrap_or(0.0);
                    TokenKind::FloatLit(val)
                } else {
                    let val: i64 = num_str.parse().unwrap_or(0);
                    TokenKind::IntLit(val)
                };

                tokens.push(Token::new(kind, Span::new(start_pos, end_pos, start_line, start_col)));
                continue;
            }

            // Two-character symbols
            let next = self.peek_next();
            let two_char = match (c, next) {
                ('.', Some('.')) => Some(TokenKind::DotDot),
                ('=', Some('=')) => Some(TokenKind::Eq),
                ('!', Some('=')) => Some(TokenKind::NotEq),
                ('<', Some('=')) => Some(TokenKind::Lte),
                ('>', Some('=')) => Some(TokenKind::Gte),
                ('&', Some('&')) => Some(TokenKind::And),
                ('|', Some('|')) => Some(TokenKind::Or),
                ('=', Some('>')) => Some(TokenKind::Arrow),
                _ => None,
            };

            if let Some(kind) = two_char {
                self.advance();
                self.advance();
                let end_pos = self.current_byte_offset();
                tokens.push(Token::new(kind, Span::new(start_pos, end_pos, start_line, start_col)));
                continue;
            }

            // A regex is one token, so `{` and `}` inside it do not open or close an event.
            if c == '/' {
                let allowed = tokens
                    .last()
                    .map(|token| Self::regex_allowed(&token.kind))
                    .unwrap_or(true);
                if allowed && self.try_consume_regex() {
                    let end_pos = self.current_byte_offset();
                    let lit = self.source[start_pos..end_pos].to_string();
                    tokens.push(Token::new(
                        TokenKind::StringLit(lit),
                        Span::new(start_pos, end_pos, start_line, start_col),
                    ));
                    continue;
                }
            }

            // Single-character symbols
            let single = match c {
                '{' => Some(TokenKind::OpenBrace),
                '}' => Some(TokenKind::CloseBrace),
                '(' => Some(TokenKind::OpenParen),
                ')' => Some(TokenKind::CloseParen),
                '[' => Some(TokenKind::OpenBracket),
                ']' => Some(TokenKind::CloseBracket),
                ':' => Some(TokenKind::Colon),
                ';' => Some(TokenKind::Semicolon),
                ',' => Some(TokenKind::Comma),
                '.' => Some(TokenKind::Dot),
                '=' => Some(TokenKind::Assign),
                '<' => Some(TokenKind::Lt),
                '>' => Some(TokenKind::Gt),
                '!' => Some(TokenKind::Not),
                '+' => Some(TokenKind::Plus),
                '-' => Some(TokenKind::Minus),
                '*' => Some(TokenKind::Star),
                '/' => Some(TokenKind::Slash),
                '@' => Some(TokenKind::At),
                '?' => Some(TokenKind::Question),
                '$' => Some(TokenKind::Dollar),
                '%' => Some(TokenKind::Percent),
                '|' => Some(TokenKind::Pipe),
                '&' => Some(TokenKind::Ampersand),
                '^' => Some(TokenKind::Caret),
                '~' => Some(TokenKind::Tilde),
                _ => None,
            };

            if let Some(kind) = single {
                self.advance();
                let end_pos = self.current_byte_offset();
                tokens.push(Token::new(kind, Span::new(start_pos, end_pos, start_line, start_col)));
                continue;
            }

            return Err(LexerError::UnexpectedChar(c, start_line, start_col));
        }

        let eof_pos = self.current_byte_offset();
        tokens.push(Token::new(TokenKind::Eof, Span::new(eof_pos, eof_pos, self.line, self.col)));
        Ok(tokens)
    }
}
