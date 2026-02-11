//! Lexer for HRML expressions.
//!
//! Tokenizes the expression language used in HRML attributes and interpolation.
//! This is the foundation of the CSP-safe expression parser —
//! no `eval()`, no `new Function()`, just structured tokenization.
//!
//! Reused from the v1.0 prototype. Renamed `Lexer` → `ExprLexer` to distinguish
//! from the source-level scanner in `hrml-lexer`.
//!
//! # Examples
//!
//! ```
//! use hrml_parser::expr_lexer::{ExprLexer, TokenKind};
//!
//! let tokens = ExprLexer::tokenize("count + 1").unwrap();
//! assert_eq!(tokens[0].kind, TokenKind::Identifier);
//! assert_eq!(tokens[1].kind, TokenKind::Plus);
//! assert_eq!(tokens[2].kind, TokenKind::Number);
//! ```

use crate::ast::Span;

/// A token produced by the expression lexer.
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
    pub value: TokenValue,
}

/// Token classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    // Literals
    Number,
    String,
    Boolean,
    Null,
    Undefined,

    // Identifiers & keywords
    Identifier,
    Typeof,

    // Arithmetic
    Plus,
    Minus,
    Star,
    Slash,
    Percent,

    // Comparison
    EqEq,
    NotEq,
    StrictEq,
    StrictNotEq,
    Lt,
    Gt,
    Lte,
    Gte,

    // Logical
    And,
    Or,
    Not,
    QuestionQuestion,

    // Assignment
    Eq,
    PlusEq,
    MinusEq,
    StarEq,
    SlashEq,

    // Postfix
    PlusPlus,
    MinusMinus,

    // Delimiters
    LParen,
    RParen,
    LBracket,
    RBracket,
    LBrace,
    RBrace,

    // Punctuation
    Dot,
    Comma,
    Colon,
    Semicolon,
    Question,
    Arrow,
    OptionalChain,

    // Interpolation
    InterpolationStart,
    InterpolationEnd,

    // End of input
    Eof,
}

/// The value carried by a token.
#[derive(Debug, Clone, PartialEq)]
pub enum TokenValue {
    None,
    Number(f64),
    String(String),
    Boolean(bool),
    Identifier(String),
}

/// Expression lexer error.
#[derive(Debug, Clone, PartialEq)]
pub struct ExprLexerError {
    pub message: String,
    pub span: Span,
}

impl std::fmt::Display for ExprLexerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Expression lexer error at position {}: {}",
            self.span.start, self.message
        )
    }
}

impl std::error::Error for ExprLexerError {}

/// HRML expression lexer.
///
/// Tokenizes expressions found in HRML attributes and interpolation blocks.
/// Operates on a single expression string (e.g. the content of `{count + 1}`).
pub struct ExprLexer<'a> {
    source: &'a str,
    chars: Vec<char>,
    pos: usize,
}

impl<'a> ExprLexer<'a> {
    /// Create a new expression lexer for the given source.
    pub fn new(source: &'a str) -> Self {
        Self {
            source,
            chars: source.chars().collect(),
            pos: 0,
        }
    }

    /// Tokenize the entire source into a vector of tokens.
    pub fn tokenize(source: &str) -> Result<Vec<Token>, ExprLexerError> {
        let mut lexer = ExprLexer::new(source);
        let mut tokens = Vec::new();

        loop {
            let token = lexer.next_token()?;
            let is_eof = token.kind == TokenKind::Eof;
            tokens.push(token);
            if is_eof {
                break;
            }
        }

        Ok(tokens)
    }

    /// Read the next token from the source.
    pub fn next_token(&mut self) -> Result<Token, ExprLexerError> {
        self.skip_whitespace();

        if self.is_at_end() {
            return Ok(Token {
                kind: TokenKind::Eof,
                span: Span::new(self.pos, self.pos),
                value: TokenValue::None,
            });
        }

        let start = self.pos;
        let ch = self.current();

        match ch {
            // Numbers
            '0'..='9' => self.read_number(start),

            // Strings
            '\'' | '"' | '`' => self.read_string(start),

            // Identifiers and keywords
            'a'..='z' | 'A'..='Z' | '_' | '$' => self.read_identifier(start),

            // Two-character operators (check first)
            '=' if self.peek() == Some('=') => {
                if self.peek_at(2) == Some('=') {
                    self.advance_n(3);
                    Ok(self.token(TokenKind::StrictEq, start, TokenValue::None))
                } else {
                    self.advance_n(2);
                    Ok(self.token(TokenKind::EqEq, start, TokenValue::None))
                }
            }
            '!' if self.peek() == Some('=') => {
                if self.peek_at(2) == Some('=') {
                    self.advance_n(3);
                    Ok(self.token(TokenKind::StrictNotEq, start, TokenValue::None))
                } else {
                    self.advance_n(2);
                    Ok(self.token(TokenKind::NotEq, start, TokenValue::None))
                }
            }
            '&' if self.peek() == Some('&') => {
                self.advance_n(2);
                Ok(self.token(TokenKind::And, start, TokenValue::None))
            }
            '|' if self.peek() == Some('|') => {
                self.advance_n(2);
                Ok(self.token(TokenKind::Or, start, TokenValue::None))
            }
            '+' if self.peek() == Some('+') => {
                self.advance_n(2);
                Ok(self.token(TokenKind::PlusPlus, start, TokenValue::None))
            }
            '-' if self.peek() == Some('-') => {
                self.advance_n(2);
                Ok(self.token(TokenKind::MinusMinus, start, TokenValue::None))
            }
            '+' if self.peek() == Some('=') => {
                self.advance_n(2);
                Ok(self.token(TokenKind::PlusEq, start, TokenValue::None))
            }
            '-' if self.peek() == Some('=') => {
                self.advance_n(2);
                Ok(self.token(TokenKind::MinusEq, start, TokenValue::None))
            }
            '*' if self.peek() == Some('=') => {
                self.advance_n(2);
                Ok(self.token(TokenKind::StarEq, start, TokenValue::None))
            }
            '/' if self.peek() == Some('=') => {
                self.advance_n(2);
                Ok(self.token(TokenKind::SlashEq, start, TokenValue::None))
            }
            '=' if self.peek() == Some('>') => {
                self.advance_n(2);
                Ok(self.token(TokenKind::Arrow, start, TokenValue::None))
            }
            '<' if self.peek() == Some('=') => {
                self.advance_n(2);
                Ok(self.token(TokenKind::Lte, start, TokenValue::None))
            }
            '>' if self.peek() == Some('=') => {
                self.advance_n(2);
                Ok(self.token(TokenKind::Gte, start, TokenValue::None))
            }
            '?' if self.peek() == Some('?') => {
                self.advance_n(2);
                Ok(self.token(TokenKind::QuestionQuestion, start, TokenValue::None))
            }
            '?' if self.peek() == Some('.') => {
                self.advance_n(2);
                Ok(self.token(TokenKind::OptionalChain, start, TokenValue::None))
            }
            '{' if self.peek() == Some('{') => {
                self.advance_n(2);
                Ok(self.token(TokenKind::InterpolationStart, start, TokenValue::None))
            }
            '}' if self.peek() == Some('}') => {
                self.advance_n(2);
                Ok(self.token(TokenKind::InterpolationEnd, start, TokenValue::None))
            }

            // Single-character tokens
            '+' => {
                self.advance();
                Ok(self.token(TokenKind::Plus, start, TokenValue::None))
            }
            '-' => {
                self.advance();
                Ok(self.token(TokenKind::Minus, start, TokenValue::None))
            }
            '*' => {
                self.advance();
                Ok(self.token(TokenKind::Star, start, TokenValue::None))
            }
            '/' => {
                self.advance();
                Ok(self.token(TokenKind::Slash, start, TokenValue::None))
            }
            '%' => {
                self.advance();
                Ok(self.token(TokenKind::Percent, start, TokenValue::None))
            }
            '!' => {
                self.advance();
                Ok(self.token(TokenKind::Not, start, TokenValue::None))
            }
            '<' => {
                self.advance();
                Ok(self.token(TokenKind::Lt, start, TokenValue::None))
            }
            '>' => {
                self.advance();
                Ok(self.token(TokenKind::Gt, start, TokenValue::None))
            }
            '=' => {
                self.advance();
                Ok(self.token(TokenKind::Eq, start, TokenValue::None))
            }
            '(' => {
                self.advance();
                Ok(self.token(TokenKind::LParen, start, TokenValue::None))
            }
            ')' => {
                self.advance();
                Ok(self.token(TokenKind::RParen, start, TokenValue::None))
            }
            '[' => {
                self.advance();
                Ok(self.token(TokenKind::LBracket, start, TokenValue::None))
            }
            ']' => {
                self.advance();
                Ok(self.token(TokenKind::RBracket, start, TokenValue::None))
            }
            '{' => {
                self.advance();
                Ok(self.token(TokenKind::LBrace, start, TokenValue::None))
            }
            '}' => {
                self.advance();
                Ok(self.token(TokenKind::RBrace, start, TokenValue::None))
            }
            '.' => {
                self.advance();
                Ok(self.token(TokenKind::Dot, start, TokenValue::None))
            }
            ',' => {
                self.advance();
                Ok(self.token(TokenKind::Comma, start, TokenValue::None))
            }
            ':' => {
                self.advance();
                Ok(self.token(TokenKind::Colon, start, TokenValue::None))
            }
            ';' => {
                self.advance();
                Ok(self.token(TokenKind::Semicolon, start, TokenValue::None))
            }
            '?' => {
                self.advance();
                Ok(self.token(TokenKind::Question, start, TokenValue::None))
            }

            _ => Err(ExprLexerError {
                message: format!("Unexpected character: '{ch}'"),
                span: Span::new(start, start + 1),
            }),
        }
    }

    // --- Private helpers ---

    fn read_number(&mut self, start: usize) -> Result<Token, ExprLexerError> {
        while !self.is_at_end() && (self.current().is_ascii_digit() || self.current() == '.') {
            self.advance();
        }

        let text = &self.source[start..self.pos];
        let value: f64 = text.parse().map_err(|_| ExprLexerError {
            message: format!("Invalid number: '{text}'"),
            span: Span::new(start, self.pos),
        })?;

        Ok(self.token(TokenKind::Number, start, TokenValue::Number(value)))
    }

    fn read_string(&mut self, start: usize) -> Result<Token, ExprLexerError> {
        let quote = self.current();
        self.advance(); // skip opening quote

        let mut value = String::new();

        while !self.is_at_end() && self.current() != quote {
            if self.current() == '\\' {
                self.advance();
                if self.is_at_end() {
                    return Err(ExprLexerError {
                        message: "Unterminated escape sequence".into(),
                        span: Span::new(start, self.pos),
                    });
                }
                match self.current() {
                    'n' => value.push('\n'),
                    't' => value.push('\t'),
                    'r' => value.push('\r'),
                    '\\' => value.push('\\'),
                    c if c == quote => value.push(c),
                    c => {
                        value.push('\\');
                        value.push(c);
                    }
                }
            } else {
                value.push(self.current());
            }
            self.advance();
        }

        if self.is_at_end() {
            return Err(ExprLexerError {
                message: "Unterminated string".into(),
                span: Span::new(start, self.pos),
            });
        }

        self.advance(); // skip closing quote

        Ok(self.token(
            TokenKind::String,
            start,
            TokenValue::String(value),
        ))
    }

    fn read_identifier(&mut self, start: usize) -> Result<Token, ExprLexerError> {
        while !self.is_at_end()
            && (self.current().is_alphanumeric() || self.current() == '_' || self.current() == '$')
        {
            self.advance();
        }

        let text = &self.source[start..self.pos];

        match text {
            "true" => Ok(self.token(
                TokenKind::Boolean,
                start,
                TokenValue::Boolean(true),
            )),
            "false" => Ok(self.token(
                TokenKind::Boolean,
                start,
                TokenValue::Boolean(false),
            )),
            "null" => Ok(self.token(TokenKind::Null, start, TokenValue::None)),
            "undefined" => Ok(self.token(TokenKind::Undefined, start, TokenValue::None)),
            "typeof" => Ok(self.token(TokenKind::Typeof, start, TokenValue::None)),
            _ => Ok(self.token(
                TokenKind::Identifier,
                start,
                TokenValue::Identifier(text.to_string()),
            )),
        }
    }

    fn token(&self, kind: TokenKind, start: usize, value: TokenValue) -> Token {
        Token {
            kind,
            span: Span::new(start, self.pos),
            value,
        }
    }

    fn current(&self) -> char {
        self.chars[self.pos]
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos + 1).copied()
    }

    fn peek_at(&self, offset: usize) -> Option<char> {
        self.chars.get(self.pos + offset).copied()
    }

    fn advance(&mut self) {
        self.pos += 1;
    }

    fn advance_n(&mut self, n: usize) {
        self.pos += n;
    }

    fn is_at_end(&self) -> bool {
        self.pos >= self.chars.len()
    }

    fn skip_whitespace(&mut self) {
        while !self.is_at_end() && self.current().is_whitespace() {
            self.advance();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokenize(source: &str) -> Vec<Token> {
        ExprLexer::tokenize(source).unwrap()
    }

    fn kinds(source: &str) -> Vec<TokenKind> {
        tokenize(source).into_iter().map(|t| t.kind).collect()
    }

    // --- Basic tokens ---

    #[test]
    fn test_number() {
        let tokens = tokenize("42");
        assert_eq!(tokens[0].kind, TokenKind::Number);
        assert_eq!(tokens[0].value, TokenValue::Number(42.0));
    }

    #[test]
    fn test_float() {
        let tokens = tokenize("2.75");
        assert_eq!(tokens[0].value, TokenValue::Number(2.75));
    }

    #[test]
    fn test_string_single_quotes() {
        let tokens = tokenize("'hello'");
        assert_eq!(tokens[0].kind, TokenKind::String);
        assert_eq!(tokens[0].value, TokenValue::String("hello".into()));
    }

    #[test]
    fn test_string_double_quotes() {
        let tokens = tokenize("\"world\"");
        assert_eq!(tokens[0].value, TokenValue::String("world".into()));
    }

    #[test]
    fn test_string_escape() {
        let tokens = tokenize("'hello\\nworld'");
        assert_eq!(tokens[0].value, TokenValue::String("hello\nworld".into()));
    }

    #[test]
    fn test_boolean_true() {
        let tokens = tokenize("true");
        assert_eq!(tokens[0].kind, TokenKind::Boolean);
        assert_eq!(tokens[0].value, TokenValue::Boolean(true));
    }

    #[test]
    fn test_boolean_false() {
        let tokens = tokenize("false");
        assert_eq!(tokens[0].value, TokenValue::Boolean(false));
    }

    #[test]
    fn test_null() {
        assert_eq!(tokenize("null")[0].kind, TokenKind::Null);
    }

    #[test]
    fn test_undefined() {
        assert_eq!(tokenize("undefined")[0].kind, TokenKind::Undefined);
    }

    #[test]
    fn test_identifier() {
        let tokens = tokenize("count");
        assert_eq!(tokens[0].kind, TokenKind::Identifier);
        assert_eq!(tokens[0].value, TokenValue::Identifier("count".into()));
    }

    // --- Operators ---

    #[test]
    fn test_arithmetic() {
        assert_eq!(
            kinds("a + b - c * d / e % f"),
            vec![
                TokenKind::Identifier,
                TokenKind::Plus,
                TokenKind::Identifier,
                TokenKind::Minus,
                TokenKind::Identifier,
                TokenKind::Star,
                TokenKind::Identifier,
                TokenKind::Slash,
                TokenKind::Identifier,
                TokenKind::Percent,
                TokenKind::Identifier,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_comparison() {
        assert_eq!(
            kinds("a == b != c === d !== e"),
            vec![
                TokenKind::Identifier,
                TokenKind::EqEq,
                TokenKind::Identifier,
                TokenKind::NotEq,
                TokenKind::Identifier,
                TokenKind::StrictEq,
                TokenKind::Identifier,
                TokenKind::StrictNotEq,
                TokenKind::Identifier,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_logical() {
        assert_eq!(
            kinds("a && b || !c ?? d"),
            vec![
                TokenKind::Identifier,
                TokenKind::And,
                TokenKind::Identifier,
                TokenKind::Or,
                TokenKind::Not,
                TokenKind::Identifier,
                TokenKind::QuestionQuestion,
                TokenKind::Identifier,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_increment_decrement() {
        assert_eq!(
            kinds("count++ count--"),
            vec![
                TokenKind::Identifier,
                TokenKind::PlusPlus,
                TokenKind::Identifier,
                TokenKind::MinusMinus,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_assignment() {
        assert_eq!(
            kinds("a = b += c -= d"),
            vec![
                TokenKind::Identifier,
                TokenKind::Eq,
                TokenKind::Identifier,
                TokenKind::PlusEq,
                TokenKind::Identifier,
                TokenKind::MinusEq,
                TokenKind::Identifier,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_arrow() {
        assert_eq!(
            kinds("(x) => x + 1"),
            vec![
                TokenKind::LParen,
                TokenKind::Identifier,
                TokenKind::RParen,
                TokenKind::Arrow,
                TokenKind::Identifier,
                TokenKind::Plus,
                TokenKind::Number,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_optional_chaining() {
        assert_eq!(
            kinds("user?.name"),
            vec![
                TokenKind::Identifier,
                TokenKind::OptionalChain,
                TokenKind::Identifier,
                TokenKind::Eof,
            ]
        );
    }

    // --- Real HRML expressions ---

    #[test]
    fn test_counter_increment() {
        assert_eq!(
            kinds("count++"),
            vec![
                TokenKind::Identifier,
                TokenKind::PlusPlus,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_state_object() {
        assert_eq!(
            kinds("{ count: 0 }"),
            vec![
                TokenKind::LBrace,
                TokenKind::Identifier,
                TokenKind::Colon,
                TokenKind::Number,
                TokenKind::RBrace,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_ternary() {
        assert_eq!(
            kinds("count > 0 ? 'positive' : 'zero'"),
            vec![
                TokenKind::Identifier,
                TokenKind::Gt,
                TokenKind::Number,
                TokenKind::Question,
                TokenKind::String,
                TokenKind::Colon,
                TokenKind::String,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_member_access() {
        assert_eq!(
            kinds("user.profile.name"),
            vec![
                TokenKind::Identifier,
                TokenKind::Dot,
                TokenKind::Identifier,
                TokenKind::Dot,
                TokenKind::Identifier,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_function_call() {
        assert_eq!(
            kinds("items.push(newItem)"),
            vec![
                TokenKind::Identifier,
                TokenKind::Dot,
                TokenKind::Identifier,
                TokenKind::LParen,
                TokenKind::Identifier,
                TokenKind::RParen,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_complex_state() {
        assert_eq!(
            kinds("{ items: [], loading: false, error: null }"),
            vec![
                TokenKind::LBrace,
                TokenKind::Identifier,
                TokenKind::Colon,
                TokenKind::LBracket,
                TokenKind::RBracket,
                TokenKind::Comma,
                TokenKind::Identifier,
                TokenKind::Colon,
                TokenKind::Boolean,
                TokenKind::Comma,
                TokenKind::Identifier,
                TokenKind::Colon,
                TokenKind::Null,
                TokenKind::RBrace,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_interpolation_markers() {
        assert_eq!(
            kinds("{{ count }}"),
            vec![
                TokenKind::InterpolationStart,
                TokenKind::Identifier,
                TokenKind::InterpolationEnd,
                TokenKind::Eof,
            ]
        );
    }

    // --- Error handling ---

    #[test]
    fn test_unterminated_string() {
        let result = ExprLexer::tokenize("'hello");
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("Unterminated string"));
    }

    #[test]
    fn test_unexpected_character() {
        let result = ExprLexer::tokenize("count # 5");
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("Unexpected character"));
    }

    #[test]
    fn test_empty_input() {
        let tokens = tokenize("");
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].kind, TokenKind::Eof);
    }

    // --- Span tracking ---

    #[test]
    fn test_span_tracking() {
        let tokens = tokenize("a + b");
        assert_eq!(tokens[0].span, Span::new(0, 1)); // 'a'
        assert_eq!(tokens[1].span, Span::new(2, 3)); // '+'
        assert_eq!(tokens[2].span, Span::new(4, 5)); // 'b'
    }
}
