//! Expression parser for HRML.
//!
//! Parses expression token streams (from `expr_lexer`) into `Expression` AST nodes.
//! Handles operator precedence, member access, function calls, and all
//! expression forms that HRML supports.

use crate::ast::{Expression, ExprSpan};
use crate::expr_lexer::Token;
use crate::ParseError;

/// HRML expression parser.
///
/// Converts a flat token stream into a tree of `Expression` nodes
/// using recursive descent with Pratt parsing for operator precedence.
// Fields will be used in feature/parser branch
#[allow(dead_code)]
pub struct ExprParser {
    tokens: Vec<Token>,
    pos: usize,
}

impl ExprParser {
    /// Create a new expression parser for the given tokens.
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    /// Parse a complete expression from a source string.
    pub fn parse(source: &str) -> Result<Expression, ParseError> {
        let _tokens = crate::expr_lexer::ExprLexer::tokenize(source).map_err(|e| ParseError {
            message: e.message,
            line: 1,
            column: e.span.start + 1,
        })?;

        // TODO: Implement expression parsing in feature/parser branch
        Ok(Expression {
            kind: crate::ast::ExprKind::Null,
            span: ExprSpan::new(0, source.len()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_stub() {
        let expr = ExprParser::parse("null").unwrap();
        assert_eq!(expr.kind, crate::ast::ExprKind::Null);
    }
}
