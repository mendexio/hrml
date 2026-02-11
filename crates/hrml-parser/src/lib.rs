//! HRML Parser
//!
//! Parses a token stream into an Abstract Syntax Tree.
//! Includes both the document parser (for `.hrml` source structure)
//! and the expression parser (for inline expressions like `count + 1`).
//!
//! The expression lexer and AST types are reused from the v1.0 prototype
//! and handle all JavaScript expression syntax that HRML supports.

pub mod ast;
pub mod expr_lexer;
pub mod expr_parser;
pub mod parser;

pub use ast::{Document, Expression, Node};
pub use parser::Parser;

/// Parser error with position information.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[error("Parse error at line {line}, column {column}: {message}")]
pub struct ParseError {
    pub message: String,
    pub line: usize,
    pub column: usize,
}
