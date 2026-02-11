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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    // Structure
    Indent,
    Dedent,
    Newline,

    // Elements
    Element,

    // Prefixes
    Dot,          // .class
    Colon,        // :state
    At,           // @event
    Dollar,       // $server

    // Literals
    String,
    Number,

    // Identifiers & keywords
    Identifier,
    State,
    Computed,
    Fn,
    AsyncFn,
    Props,
    Emit,
    Import,
    Page,
    Config,

    // Interpolation
    InterpolationStart, // {
    InterpolationEnd,   // }

    // Attributes
    Equals,

    // Punctuation
    Comma,
    LParen,
    RParen,

    // End of input
    Eof,
}

/// A token produced by the HRML lexer.
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
    pub value: String,
}

impl Token {
    pub fn new(kind: TokenKind, value: impl Into<String>, span: Span) -> Self {
        Self {
            kind,
            span,
            value: value.into(),
        }
    }
}
