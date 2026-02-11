//! HRML Lexer
//!
//! Tokenizes `.hrml` source files into a stream of tokens.
//! Handles indentation-based structure, element names, CSS class prefixes,
//! the three HRML prefixes (`:` `@` `$`), string literals, and interpolation.
//!
//! # Example
//!
//! ```
//! use hrml_lexer::Scanner;
//!
//! let tokens = Scanner::tokenize("").unwrap();
//! assert_eq!(tokens.len(), 1); // Just EOF
//! ```

pub mod scanner;
pub mod token;

pub use scanner::Scanner;
pub use token::{Span, Token, TokenKind};

/// Lexer error with position information.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[error("Lexer error at line {line}, column {column}: {message}")]
pub struct LexerError {
    pub message: String,
    pub line: usize,
    pub column: usize,
}
