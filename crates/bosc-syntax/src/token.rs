use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub line: usize,
    pub column: usize,
}

impl Span {
    pub fn new(start: usize, end: usize, line: usize, column: usize) -> Self {
        Self { start, end, line, column }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TokenKind {
    // Keywords
    Table,
    TableExtension,
    TableLogic,
    TableLogicExtension,
    StateMachine,
    StateMachineExtension,
    Codeunit,
    Interface,
    Enum,
    EnumExtension,
    Page,
    PageExtension,
    View,
    Query,
    Report,
    CustomMetadata,
    PermissionSet,
    Elements,
    Dataitem,
    Select,
    From,
    LeftJoin,
    InnerJoin,
    GroupBy,
    Having,
    OrderBy,
    Extends,
    Implements,
    For,
    State,
    Transition,
    Guard,
    ApprovalMatrix,
    Tier,
    Params,
    Timeout,
    EscalateTo,
    PrimaryKey,
    Key,
    Relation,
    Computed,
    FlowFilter,
    Where,
    In,
    True,
    False,
    Null,
    Public,
    Private,
    Static,
    Function,
    Return,
    If,
    Else,

    // Identifiers and Literals
    Ident(String),
    StringLit(String),
    IntLit(i64),
    FloatLit(f64),

    // Symbols & Operators
    OpenBrace,      // {
    CloseBrace,     // }
    OpenParen,      // (
    CloseParen,     // )
    OpenBracket,    // [
    CloseBracket,   // ]
    Colon,          // :
    Semicolon,      // ;
    Comma,          // ,
    Dot,            // .
    DotDot,         // ..
    Assign,         // =
    Eq,             // ==
    NotEq,          // !=
    Lt,             // <
    Lte,            // <=
    Gt,             // >
    Gte,            // >=
    And,            // &&
    Or,             // ||
    Not,            // !
    Plus,           // +
    Minus,          // -
    Star,           // *
    Slash,          // /
    Arrow,          // =>
    At,             // @
    Question,       // ?
    Dollar,         // $
    Percent,        // %
    Pipe,           // |
    Ampersand,      // &
    Caret,          // ^
    Tilde,          // ~

    // End of File
    Eof,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

impl Token {
    pub fn new(kind: TokenKind, span: Span) -> Self {
        Self { kind, span }
    }
}
