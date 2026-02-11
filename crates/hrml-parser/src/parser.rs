//! Document parser for HRML.
//!
//! Parses a stream of source-level tokens (from `hrml-lexer`) into a `Document` AST.
//! Handles indentation-based nesting, element detection, prefix attributes,
//! and delegates inline expressions to `expr_parser`.

use crate::ast::Document;
use crate::ParseError;
use hrml_lexer::Token;

/// HRML document parser.
///
/// Converts a flat token stream from the source lexer into a hierarchical
/// `Document` AST using recursive descent.
// Fields will be used in feature/parser branch
#[allow(dead_code)]
pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    /// Create a new parser for the given tokens.
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    /// Parse source code into a document AST.
    pub fn parse(source: &str) -> Result<Document, ParseError> {
        let _tokens = hrml_lexer::Scanner::tokenize(source).map_err(|e| ParseError {
            message: e.message,
            line: e.line,
            column: e.column,
        })?;

        // TODO: Implement document parsing in feature/parser branch
        Ok(Document { nodes: Vec::new() })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_empty() {
        let doc = Parser::parse("").unwrap();
        assert!(doc.nodes.is_empty());
    }
}
