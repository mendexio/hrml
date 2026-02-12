/// A position in source text, tracking line and column for error reporting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub line: usize,
    pub column: usize,
}

impl Span {
    pub fn new(start: usize, end: usize, line: usize, column: usize) -> Self {
        Self {
            start,
            end,
            line,
            column,
        }
    }
}

/// Token classification for HRML source.
///
/// Data-carrying variants embed their value directly (no separate `value` field on Token).
/// Data-carrying variants provide type-safe token handling.
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Structure
    Indent,
    Dedent,
    Newline,

    // Literals (carry data)
    Identifier(String),
    String(String),
    Number(f64),
    Boolean(bool),
    Null,
    Comment(String),
    Interpolation(String),

    // Prefixes
    Dot,    // .class
    Colon,  // :state
    At,     // @event
    Dollar, // $server
    Hash,   // #id (future)
    Plus,   // +element stacking (future)

    // Punctuation
    Equals,
    Comma,
    LParen,
    RParen,

    // Keywords
    State,
    Computed,
    Fn,
    Async,
    Watch,
    Props,
    Emit,
    Import,
    Page,
    Config,

    // End of input
    Eof,
}

/// A token produced by the HRML lexer.
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

impl Token {
    pub fn new(kind: TokenKind, span: Span) -> Self {
        Self { kind, span }
    }
}

/// HTML5 void elements (self-closing, no children).
pub const VOID_ELEMENTS: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param",
    "source", "track", "wbr",
];

/// Check if a tag name is an HTML5 void element.
pub fn is_void_element(tag: &str) -> bool {
    VOID_ELEMENTS.contains(&tag)
}
