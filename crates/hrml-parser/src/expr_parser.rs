//! Expression parser for HRML.
//!
//! Parses expression token streams (from `expr_lexer`) into `Expression` AST nodes.
//! Uses recursive descent with operator precedence climbing (MOX pattern).
//!
//! Precedence (lowest to highest):
//! 1. Assignment: `=`, `+=`, `-=`, `*=`, `/=`
//! 2. Ternary: `? :`
//! 3. Nullish: `??`
//! 4. Logical OR: `||`
//! 5. Logical AND: `&&`
//! 6. Equality: `==`, `!=`, `===`, `!==`
//! 7. Comparison: `<`, `>`, `<=`, `>=`
//! 8. Additive: `+`, `-`
//! 9. Multiplicative: `*`, `/`, `%`
//! 10. Unary: `!`, `-`, `typeof`
//! 11. Postfix: `++`, `--`
//! 12. Call/Member: `.`, `[]`, `()`, `?.`
//! 13. Primary: literals, identifiers, parens, arrays, objects

use crate::ast::{
    AssignOp, BinaryOp, ExprKind, ExprSpan, Expression, ObjectProperty, PostfixOp, UnaryOp,
};
use crate::expr_lexer::{ExprLexer, Token, TokenKind, TokenValue};
use crate::ParseError;

/// HRML expression parser.
///
/// Converts a flat token stream into a tree of `Expression` nodes
/// using recursive descent with operator precedence climbing.
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
        let tokens = ExprLexer::tokenize(source).map_err(|e| ParseError {
            message: e.message,
            line: 1,
            column: e.span.start + 1,
        })?;

        let mut parser = ExprParser::new(tokens);
        let expr = parser.parse_expression()?;

        // Ensure we consumed everything (except Eof)
        if parser.peek().kind != TokenKind::Eof {
            return Err(parser.error(format!(
                "Unexpected token: {:?}",
                parser.peek().kind
            )));
        }

        Ok(expr)
    }

    // =========================================================================
    // Precedence levels (lowest to highest)
    // =========================================================================

    /// Entry point: parse a full expression.
    fn parse_expression(&mut self) -> Result<Expression, ParseError> {
        self.parse_assignment()
    }

    /// Level 1: Assignment (`=`, `+=`, `-=`, `*=`, `/=`) — right-associative.
    fn parse_assignment(&mut self) -> Result<Expression, ParseError> {
        let expr = self.parse_ternary()?;

        let op = match self.peek().kind {
            TokenKind::Eq => Some(AssignOp::Assign),
            TokenKind::PlusEq => Some(AssignOp::AddAssign),
            TokenKind::MinusEq => Some(AssignOp::SubAssign),
            TokenKind::StarEq => Some(AssignOp::MulAssign),
            TokenKind::SlashEq => Some(AssignOp::DivAssign),
            _ => None,
        };

        if let Some(op) = op {
            self.advance();
            let value = self.parse_assignment()?; // Right-associative
            let span = ExprSpan::new(expr.span.start, value.span.end);
            Ok(Expression {
                kind: ExprKind::Assignment {
                    target: Box::new(expr),
                    op,
                    value: Box::new(value),
                },
                span,
            })
        } else {
            Ok(expr)
        }
    }

    /// Level 2: Ternary (`? :`)
    fn parse_ternary(&mut self) -> Result<Expression, ParseError> {
        let expr = self.parse_nullish()?;

        if self.peek().kind == TokenKind::Question {
            self.advance();
            let consequent = self.parse_assignment()?;
            if self.peek().kind != TokenKind::Colon {
                return Err(self.error("Expected ':' in ternary expression".into()));
            }
            self.advance();
            let alternate = self.parse_assignment()?;
            let span = ExprSpan::new(expr.span.start, alternate.span.end);
            Ok(Expression {
                kind: ExprKind::Ternary {
                    condition: Box::new(expr),
                    consequent: Box::new(consequent),
                    alternate: Box::new(alternate),
                },
                span,
            })
        } else {
            Ok(expr)
        }
    }

    /// Level 3: Nullish coalescing (`??`)
    fn parse_nullish(&mut self) -> Result<Expression, ParseError> {
        let mut left = self.parse_or()?;

        while self.peek().kind == TokenKind::QuestionQuestion {
            self.advance();
            let right = self.parse_or()?;
            let span = ExprSpan::new(left.span.start, right.span.end);
            left = Expression {
                kind: ExprKind::Binary {
                    left: Box::new(left),
                    op: BinaryOp::NullishCoalescing,
                    right: Box::new(right),
                },
                span,
            };
        }

        Ok(left)
    }

    /// Level 4: Logical OR (`||`)
    fn parse_or(&mut self) -> Result<Expression, ParseError> {
        let mut left = self.parse_and()?;

        while self.peek().kind == TokenKind::Or {
            self.advance();
            let right = self.parse_and()?;
            let span = ExprSpan::new(left.span.start, right.span.end);
            left = Expression {
                kind: ExprKind::Binary {
                    left: Box::new(left),
                    op: BinaryOp::Or,
                    right: Box::new(right),
                },
                span,
            };
        }

        Ok(left)
    }

    /// Level 5: Logical AND (`&&`)
    fn parse_and(&mut self) -> Result<Expression, ParseError> {
        let mut left = self.parse_equality()?;

        while self.peek().kind == TokenKind::And {
            self.advance();
            let right = self.parse_equality()?;
            let span = ExprSpan::new(left.span.start, right.span.end);
            left = Expression {
                kind: ExprKind::Binary {
                    left: Box::new(left),
                    op: BinaryOp::And,
                    right: Box::new(right),
                },
                span,
            };
        }

        Ok(left)
    }

    /// Level 6: Equality (`==`, `!=`, `===`, `!==`)
    fn parse_equality(&mut self) -> Result<Expression, ParseError> {
        let mut left = self.parse_comparison()?;

        loop {
            let op = match self.peek().kind {
                TokenKind::EqEq => BinaryOp::Eq,
                TokenKind::NotEq => BinaryOp::Neq,
                TokenKind::StrictEq => BinaryOp::StrictEq,
                TokenKind::StrictNotEq => BinaryOp::StrictNeq,
                _ => break,
            };
            self.advance();
            let right = self.parse_comparison()?;
            let span = ExprSpan::new(left.span.start, right.span.end);
            left = Expression {
                kind: ExprKind::Binary {
                    left: Box::new(left),
                    op,
                    right: Box::new(right),
                },
                span,
            };
        }

        Ok(left)
    }

    /// Level 7: Comparison (`<`, `>`, `<=`, `>=`)
    fn parse_comparison(&mut self) -> Result<Expression, ParseError> {
        let mut left = self.parse_additive()?;

        loop {
            let op = match self.peek().kind {
                TokenKind::Lt => BinaryOp::Lt,
                TokenKind::Gt => BinaryOp::Gt,
                TokenKind::Lte => BinaryOp::Lte,
                TokenKind::Gte => BinaryOp::Gte,
                _ => break,
            };
            self.advance();
            let right = self.parse_additive()?;
            let span = ExprSpan::new(left.span.start, right.span.end);
            left = Expression {
                kind: ExprKind::Binary {
                    left: Box::new(left),
                    op,
                    right: Box::new(right),
                },
                span,
            };
        }

        Ok(left)
    }

    /// Level 8: Additive (`+`, `-`)
    fn parse_additive(&mut self) -> Result<Expression, ParseError> {
        let mut left = self.parse_multiplicative()?;

        loop {
            let op = match self.peek().kind {
                TokenKind::Plus => BinaryOp::Add,
                TokenKind::Minus => BinaryOp::Sub,
                _ => break,
            };
            self.advance();
            let right = self.parse_multiplicative()?;
            let span = ExprSpan::new(left.span.start, right.span.end);
            left = Expression {
                kind: ExprKind::Binary {
                    left: Box::new(left),
                    op,
                    right: Box::new(right),
                },
                span,
            };
        }

        Ok(left)
    }

    /// Level 9: Multiplicative (`*`, `/`, `%`)
    fn parse_multiplicative(&mut self) -> Result<Expression, ParseError> {
        let mut left = self.parse_unary()?;

        loop {
            let op = match self.peek().kind {
                TokenKind::Star => BinaryOp::Mul,
                TokenKind::Slash => BinaryOp::Div,
                TokenKind::Percent => BinaryOp::Mod,
                _ => break,
            };
            self.advance();
            let right = self.parse_unary()?;
            let span = ExprSpan::new(left.span.start, right.span.end);
            left = Expression {
                kind: ExprKind::Binary {
                    left: Box::new(left),
                    op,
                    right: Box::new(right),
                },
                span,
            };
        }

        Ok(left)
    }

    /// Level 10: Unary (`!`, `-`, `typeof`)
    fn parse_unary(&mut self) -> Result<Expression, ParseError> {
        let start = self.peek().span.start;
        let op = match self.peek().kind {
            TokenKind::Not => Some(UnaryOp::Not),
            TokenKind::Minus => Some(UnaryOp::Neg),
            TokenKind::Typeof => Some(UnaryOp::Typeof),
            _ => None,
        };

        if let Some(op) = op {
            self.advance();
            let operand = self.parse_unary()?; // Right-recursive for chaining: !!x
            let span = ExprSpan::new(start, operand.span.end);
            Ok(Expression {
                kind: ExprKind::Unary {
                    op,
                    operand: Box::new(operand),
                },
                span,
            })
        } else {
            self.parse_postfix()
        }
    }

    /// Level 11: Postfix (`++`, `--`)
    fn parse_postfix(&mut self) -> Result<Expression, ParseError> {
        let mut expr = self.parse_call_member()?;

        loop {
            let op = match self.peek().kind {
                TokenKind::PlusPlus => PostfixOp::Increment,
                TokenKind::MinusMinus => PostfixOp::Decrement,
                _ => break,
            };
            let end = self.peek().span.end;
            self.advance();
            let span = ExprSpan::new(expr.span.start, end);
            expr = Expression {
                kind: ExprKind::Postfix {
                    operand: Box::new(expr),
                    op,
                },
                span,
            };
        }

        Ok(expr)
    }

    /// Level 12: Call and member access (`.`, `[]`, `()`, `?.`)
    fn parse_call_member(&mut self) -> Result<Expression, ParseError> {
        let mut expr = self.parse_primary()?;

        loop {
            match self.peek().kind {
                // Member access: expr.prop
                TokenKind::Dot => {
                    self.advance();
                    let prop_start = self.peek().span.start;
                    let prop_end = self.peek().span.end;
                    let name = self.expect_identifier()?;
                    let span = ExprSpan::new(expr.span.start, prop_end);
                    expr = Expression {
                        kind: ExprKind::Member {
                            object: Box::new(expr),
                            property: Box::new(Expression {
                                kind: ExprKind::Identifier(name),
                                span: ExprSpan::new(prop_start, prop_end),
                            }),
                            computed: false,
                        },
                        span,
                    };
                }
                // Optional chaining: expr?.prop
                TokenKind::OptionalChain => {
                    self.advance();
                    let prop_start = self.peek().span.start;
                    let prop_end = self.peek().span.end;
                    let name = self.expect_identifier()?;
                    let span = ExprSpan::new(expr.span.start, prop_end);
                    expr = Expression {
                        kind: ExprKind::Member {
                            object: Box::new(expr),
                            property: Box::new(Expression {
                                kind: ExprKind::Identifier(name),
                                span: ExprSpan::new(prop_start, prop_end),
                            }),
                            computed: false,
                        },
                        span,
                    };
                }
                // Computed member: expr[index]
                TokenKind::LBracket => {
                    self.advance();
                    let index = self.parse_expression()?;
                    if self.peek().kind != TokenKind::RBracket {
                        return Err(self.error("Expected ']'".into()));
                    }
                    let end = self.peek().span.end;
                    self.advance();
                    let span = ExprSpan::new(expr.span.start, end);
                    expr = Expression {
                        kind: ExprKind::Member {
                            object: Box::new(expr),
                            property: Box::new(index),
                            computed: true,
                        },
                        span,
                    };
                }
                // Function call: expr(args)
                TokenKind::LParen => {
                    self.advance();
                    let args = self.parse_arguments()?;
                    if self.peek().kind != TokenKind::RParen {
                        return Err(self.error("Expected ')'".into()));
                    }
                    let end = self.peek().span.end;
                    self.advance();
                    let span = ExprSpan::new(expr.span.start, end);
                    expr = Expression {
                        kind: ExprKind::Call {
                            callee: Box::new(expr),
                            arguments: args,
                        },
                        span,
                    };
                }
                _ => break,
            }
        }

        Ok(expr)
    }

    /// Level 13: Primary (literals, identifiers, parens, arrays, objects, arrow functions)
    fn parse_primary(&mut self) -> Result<Expression, ParseError> {
        let token = self.peek().clone();

        match &token.kind {
            // Number literal
            TokenKind::Number => {
                let value = match &token.value {
                    TokenValue::Number(n) => *n,
                    _ => 0.0,
                };
                self.advance();
                Ok(Expression {
                    kind: ExprKind::Number(value),
                    span: token.span,
                })
            }

            // String literal
            TokenKind::String => {
                let value = match &token.value {
                    TokenValue::String(s) => s.clone(),
                    _ => String::new(),
                };
                self.advance();
                Ok(Expression {
                    kind: ExprKind::String(value),
                    span: token.span,
                })
            }

            // Boolean literal
            TokenKind::Boolean => {
                let value = match &token.value {
                    TokenValue::Boolean(b) => *b,
                    _ => false,
                };
                self.advance();
                Ok(Expression {
                    kind: ExprKind::Boolean(value),
                    span: token.span,
                })
            }

            // Null
            TokenKind::Null => {
                self.advance();
                Ok(Expression {
                    kind: ExprKind::Null,
                    span: token.span,
                })
            }

            // Undefined
            TokenKind::Undefined => {
                self.advance();
                Ok(Expression {
                    kind: ExprKind::Undefined,
                    span: token.span,
                })
            }

            // Identifier — or start of arrow function
            TokenKind::Identifier => {
                let name = match &token.value {
                    TokenValue::Identifier(s) => s.clone(),
                    _ => String::new(),
                };

                // Check for single-param arrow: `x => x + 1`
                if self.peek_at(1).map(|t| &t.kind) == Some(&TokenKind::Arrow) {
                    self.advance(); // consume identifier
                    self.advance(); // consume =>
                    let body = self.parse_assignment()?;
                    let span = ExprSpan::new(token.span.start, body.span.end);
                    return Ok(Expression {
                        kind: ExprKind::Arrow {
                            params: vec![name],
                            body: Box::new(body),
                        },
                        span,
                    });
                }

                self.advance();
                Ok(Expression {
                    kind: ExprKind::Identifier(name),
                    span: token.span,
                })
            }

            // Parenthesized expression or arrow function params
            TokenKind::LParen => {
                // Try arrow function: (params) => body
                if self.is_arrow_params() {
                    return self.parse_arrow_function();
                }

                self.advance(); // consume (
                let expr = self.parse_expression()?;
                if self.peek().kind != TokenKind::RParen {
                    return Err(self.error("Expected ')'".into()));
                }
                self.advance(); // consume )

                // Check for arrow after parenthesized expression
                if self.peek().kind == TokenKind::Arrow {
                    // Re-interpret expr as parameter list
                    let params = self.expr_to_params(&expr)?;
                    self.advance(); // consume =>
                    let body = self.parse_assignment()?;
                    let span = ExprSpan::new(token.span.start, body.span.end);
                    return Ok(Expression {
                        kind: ExprKind::Arrow {
                            params,
                            body: Box::new(body),
                        },
                        span,
                    });
                }

                Ok(expr)
            }

            // Array literal: [1, 2, 3]
            TokenKind::LBracket => {
                self.advance(); // consume [
                let mut elements = Vec::new();

                while self.peek().kind != TokenKind::RBracket
                    && self.peek().kind != TokenKind::Eof
                {
                    elements.push(self.parse_assignment()?);
                    if self.peek().kind == TokenKind::Comma {
                        self.advance();
                    }
                }

                if self.peek().kind != TokenKind::RBracket {
                    return Err(self.error("Expected ']'".into()));
                }
                let end = self.peek().span.end;
                self.advance();

                Ok(Expression {
                    kind: ExprKind::Array(elements),
                    span: ExprSpan::new(token.span.start, end),
                })
            }

            // Object literal: { key: value }
            TokenKind::LBrace => {
                self.advance(); // consume {
                let mut properties = Vec::new();

                while self.peek().kind != TokenKind::RBrace
                    && self.peek().kind != TokenKind::Eof
                {
                    let key_token = self.peek().clone();
                    let key = self.expect_identifier()?;

                    if self.peek().kind == TokenKind::Colon {
                        // Full property: key: value
                        self.advance();
                        let value = self.parse_assignment()?;
                        properties.push(ObjectProperty {
                            key,
                            value,
                            shorthand: false,
                        });
                    } else {
                        // Shorthand: { key } means { key: key }
                        properties.push(ObjectProperty {
                            key: key.clone(),
                            value: Expression {
                                kind: ExprKind::Identifier(key),
                                span: key_token.span,
                            },
                            shorthand: true,
                        });
                    }

                    if self.peek().kind == TokenKind::Comma {
                        self.advance();
                    }
                }

                if self.peek().kind != TokenKind::RBrace {
                    return Err(self.error("Expected '}'".into()));
                }
                let end = self.peek().span.end;
                self.advance();

                Ok(Expression {
                    kind: ExprKind::Object(properties),
                    span: ExprSpan::new(token.span.start, end),
                })
            }

            _ => Err(self.error(format!("Unexpected token: {:?}", token.kind))),
        }
    }

    // =========================================================================
    // Helpers
    // =========================================================================

    fn peek(&self) -> &Token {
        static EOF: std::sync::LazyLock<Token> = std::sync::LazyLock::new(|| Token {
            kind: TokenKind::Eof,
            span: ExprSpan::new(0, 0),
            value: TokenValue::None,
        });
        self.tokens.get(self.pos).unwrap_or(&EOF)
    }

    fn peek_at(&self, offset: usize) -> Option<&Token> {
        self.tokens.get(self.pos + offset)
    }

    fn advance(&mut self) {
        if self.pos < self.tokens.len() {
            self.pos += 1;
        }
    }

    fn expect_identifier(&mut self) -> Result<String, ParseError> {
        if let TokenValue::Identifier(name) = &self.peek().value {
            let name = name.clone();
            self.advance();
            Ok(name)
        } else {
            Err(self.error(format!("Expected identifier, got {:?}", self.peek().kind)))
        }
    }

    /// Parse comma-separated argument list.
    fn parse_arguments(&mut self) -> Result<Vec<Expression>, ParseError> {
        let mut args = Vec::new();

        if self.peek().kind == TokenKind::RParen {
            return Ok(args);
        }

        args.push(self.parse_assignment()?);
        while self.peek().kind == TokenKind::Comma {
            self.advance();
            args.push(self.parse_assignment()?);
        }

        Ok(args)
    }

    /// Check if current position starts an arrow function parameter list.
    /// Heuristic: `(` followed eventually by `)` then `=>`.
    fn is_arrow_params(&self) -> bool {
        let mut depth = 0;
        let mut i = self.pos;

        while i < self.tokens.len() {
            match self.tokens[i].kind {
                TokenKind::LParen => depth += 1,
                TokenKind::RParen => {
                    depth -= 1;
                    if depth == 0 {
                        // Check if next token is =>
                        return i + 1 < self.tokens.len()
                            && self.tokens[i + 1].kind == TokenKind::Arrow;
                    }
                }
                TokenKind::Eof => return false,
                _ => {}
            }
            i += 1;
        }

        false
    }

    /// Parse arrow function: `(a, b) => expr`
    fn parse_arrow_function(&mut self) -> Result<Expression, ParseError> {
        let start = self.peek().span.start;
        self.advance(); // consume (
        let mut params = Vec::new();

        while self.peek().kind != TokenKind::RParen && self.peek().kind != TokenKind::Eof {
            params.push(self.expect_identifier()?);
            if self.peek().kind == TokenKind::Comma {
                self.advance();
            }
        }

        if self.peek().kind != TokenKind::RParen {
            return Err(self.error("Expected ')' after arrow parameters".into()));
        }
        self.advance(); // consume )

        if self.peek().kind != TokenKind::Arrow {
            return Err(self.error("Expected '=>'".into()));
        }
        self.advance(); // consume =>

        let body = self.parse_assignment()?;
        let span = ExprSpan::new(start, body.span.end);

        Ok(Expression {
            kind: ExprKind::Arrow {
                params,
                body: Box::new(body),
            },
            span,
        })
    }

    /// Convert a parsed expression back into a parameter list for arrow functions.
    /// Handles: `(x)`, `(x, y)` where the expression was parsed as identifier or comma sequence.
    fn expr_to_params(&self, expr: &Expression) -> Result<Vec<String>, ParseError> {
        match &expr.kind {
            ExprKind::Identifier(name) => Ok(vec![name.clone()]),
            _ => Err(ParseError {
                message: "Invalid arrow function parameters".into(),
                line: 1,
                column: expr.span.start + 1,
            }),
        }
    }

    fn error(&self, message: String) -> ParseError {
        let span = self.peek().span;
        ParseError {
            message,
            line: 1,
            column: span.start + 1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(source: &str) -> Expression {
        ExprParser::parse(source).unwrap()
    }

    fn parse_kind(source: &str) -> ExprKind {
        parse(source).kind
    }

    // =========================================================================
    // Literals
    // =========================================================================

    #[test]
    fn test_number_integer() {
        assert_eq!(parse_kind("42"), ExprKind::Number(42.0));
    }

    #[test]
    fn test_number_float() {
        assert_eq!(parse_kind("3.14"), ExprKind::Number(3.14));
    }

    #[test]
    fn test_string_double() {
        assert_eq!(parse_kind("\"hello\""), ExprKind::String("hello".into()));
    }

    #[test]
    fn test_string_single() {
        assert_eq!(parse_kind("'world'"), ExprKind::String("world".into()));
    }

    #[test]
    fn test_boolean_true() {
        assert_eq!(parse_kind("true"), ExprKind::Boolean(true));
    }

    #[test]
    fn test_boolean_false() {
        assert_eq!(parse_kind("false"), ExprKind::Boolean(false));
    }

    #[test]
    fn test_null() {
        assert_eq!(parse_kind("null"), ExprKind::Null);
    }

    #[test]
    fn test_undefined() {
        assert_eq!(parse_kind("undefined"), ExprKind::Undefined);
    }

    #[test]
    fn test_identifier() {
        assert_eq!(parse_kind("count"), ExprKind::Identifier("count".into()));
    }

    // =========================================================================
    // Binary operators
    // =========================================================================

    #[test]
    fn test_addition() {
        let expr = parse("a + b");
        assert!(matches!(expr.kind, ExprKind::Binary { op: BinaryOp::Add, .. }));
    }

    #[test]
    fn test_subtraction() {
        let expr = parse("a - b");
        assert!(matches!(expr.kind, ExprKind::Binary { op: BinaryOp::Sub, .. }));
    }

    #[test]
    fn test_multiplication() {
        let expr = parse("a * b");
        assert!(matches!(expr.kind, ExprKind::Binary { op: BinaryOp::Mul, .. }));
    }

    #[test]
    fn test_division() {
        let expr = parse("a / b");
        assert!(matches!(expr.kind, ExprKind::Binary { op: BinaryOp::Div, .. }));
    }

    #[test]
    fn test_modulo() {
        let expr = parse("a % b");
        assert!(matches!(expr.kind, ExprKind::Binary { op: BinaryOp::Mod, .. }));
    }

    #[test]
    fn test_precedence_mul_over_add() {
        // a + b * c should be a + (b * c)
        let expr = parse("a + b * c");
        match &expr.kind {
            ExprKind::Binary { op: BinaryOp::Add, right, .. } => {
                assert!(matches!(right.kind, ExprKind::Binary { op: BinaryOp::Mul, .. }));
            }
            _ => panic!("Expected addition at top level"),
        }
    }

    #[test]
    fn test_left_associativity() {
        // a - b - c should be (a - b) - c
        let expr = parse("a - b - c");
        match &expr.kind {
            ExprKind::Binary { op: BinaryOp::Sub, left, .. } => {
                assert!(matches!(left.kind, ExprKind::Binary { op: BinaryOp::Sub, .. }));
            }
            _ => panic!("Expected subtraction at top level"),
        }
    }

    // =========================================================================
    // Comparison and equality
    // =========================================================================

    #[test]
    fn test_less_than() {
        let expr = parse("a < b");
        assert!(matches!(expr.kind, ExprKind::Binary { op: BinaryOp::Lt, .. }));
    }

    #[test]
    fn test_strict_equality() {
        let expr = parse("a === b");
        assert!(matches!(expr.kind, ExprKind::Binary { op: BinaryOp::StrictEq, .. }));
    }

    #[test]
    fn test_inequality() {
        let expr = parse("a != b");
        assert!(matches!(expr.kind, ExprKind::Binary { op: BinaryOp::Neq, .. }));
    }

    // =========================================================================
    // Logical operators
    // =========================================================================

    #[test]
    fn test_logical_and() {
        let expr = parse("a && b");
        assert!(matches!(expr.kind, ExprKind::Binary { op: BinaryOp::And, .. }));
    }

    #[test]
    fn test_logical_or() {
        let expr = parse("a || b");
        assert!(matches!(expr.kind, ExprKind::Binary { op: BinaryOp::Or, .. }));
    }

    #[test]
    fn test_nullish_coalescing() {
        let expr = parse("a ?? b");
        assert!(matches!(expr.kind, ExprKind::Binary { op: BinaryOp::NullishCoalescing, .. }));
    }

    // =========================================================================
    // Unary and postfix
    // =========================================================================

    #[test]
    fn test_unary_not() {
        let expr = parse("!active");
        assert!(matches!(expr.kind, ExprKind::Unary { op: UnaryOp::Not, .. }));
    }

    #[test]
    fn test_unary_neg() {
        let expr = parse("-count");
        assert!(matches!(expr.kind, ExprKind::Unary { op: UnaryOp::Neg, .. }));
    }

    #[test]
    fn test_postfix_increment() {
        let expr = parse("count++");
        assert!(matches!(expr.kind, ExprKind::Postfix { op: PostfixOp::Increment, .. }));
    }

    #[test]
    fn test_postfix_decrement() {
        let expr = parse("count--");
        assert!(matches!(expr.kind, ExprKind::Postfix { op: PostfixOp::Decrement, .. }));
    }

    // =========================================================================
    // Assignment
    // =========================================================================

    #[test]
    fn test_assignment() {
        let expr = parse("count = 5");
        assert!(matches!(expr.kind, ExprKind::Assignment { op: AssignOp::Assign, .. }));
    }

    #[test]
    fn test_add_assign() {
        let expr = parse("count += 1");
        assert!(matches!(expr.kind, ExprKind::Assignment { op: AssignOp::AddAssign, .. }));
    }

    // =========================================================================
    // Ternary
    // =========================================================================

    #[test]
    fn test_ternary() {
        let expr = parse("a > 0 ? 'yes' : 'no'");
        assert!(matches!(expr.kind, ExprKind::Ternary { .. }));
    }

    // =========================================================================
    // Member access and calls
    // =========================================================================

    #[test]
    fn test_member_access() {
        let expr = parse("user.name");
        assert!(matches!(
            expr.kind,
            ExprKind::Member { computed: false, .. }
        ));
    }

    #[test]
    fn test_computed_member() {
        let expr = parse("items[0]");
        assert!(matches!(
            expr.kind,
            ExprKind::Member { computed: true, .. }
        ));
    }

    #[test]
    fn test_function_call() {
        let expr = parse("save()");
        match &expr.kind {
            ExprKind::Call { arguments, .. } => assert!(arguments.is_empty()),
            _ => panic!("Expected call"),
        }
    }

    #[test]
    fn test_call_with_args() {
        let expr = parse("add(1, 2)");
        match &expr.kind {
            ExprKind::Call { arguments, .. } => assert_eq!(arguments.len(), 2),
            _ => panic!("Expected call"),
        }
    }

    #[test]
    fn test_method_call() {
        let expr = parse("items.push(x)");
        assert!(matches!(expr.kind, ExprKind::Call { .. }));
    }

    #[test]
    fn test_chained_member() {
        let expr = parse("a.b.c");
        match &expr.kind {
            ExprKind::Member { object, .. } => {
                assert!(matches!(object.kind, ExprKind::Member { .. }));
            }
            _ => panic!("Expected member"),
        }
    }

    #[test]
    fn test_optional_chaining() {
        let expr = parse("user?.name");
        assert!(matches!(expr.kind, ExprKind::Member { .. }));
    }

    // =========================================================================
    // Arrow functions
    // =========================================================================

    #[test]
    fn test_arrow_single_param() {
        let expr = parse("x => x + 1");
        match &expr.kind {
            ExprKind::Arrow { params, .. } => assert_eq!(params, &["x"]),
            _ => panic!("Expected arrow"),
        }
    }

    #[test]
    fn test_arrow_multi_params() {
        let expr = parse("(a, b) => a + b");
        match &expr.kind {
            ExprKind::Arrow { params, .. } => assert_eq!(params, &["a", "b"]),
            _ => panic!("Expected arrow"),
        }
    }

    #[test]
    fn test_arrow_no_params() {
        let expr = parse("() => 42");
        match &expr.kind {
            ExprKind::Arrow { params, .. } => assert!(params.is_empty()),
            _ => panic!("Expected arrow"),
        }
    }

    // =========================================================================
    // Object and array literals
    // =========================================================================

    #[test]
    fn test_array_literal() {
        let expr = parse("[1, 2, 3]");
        match &expr.kind {
            ExprKind::Array(elements) => assert_eq!(elements.len(), 3),
            _ => panic!("Expected array"),
        }
    }

    #[test]
    fn test_empty_array() {
        let expr = parse("[]");
        match &expr.kind {
            ExprKind::Array(elements) => assert!(elements.is_empty()),
            _ => panic!("Expected array"),
        }
    }

    #[test]
    fn test_object_literal() {
        let expr = parse("{ count: 0, name: 'test' }");
        match &expr.kind {
            ExprKind::Object(props) => {
                assert_eq!(props.len(), 2);
                assert_eq!(props[0].key, "count");
                assert!(!props[0].shorthand);
            }
            _ => panic!("Expected object"),
        }
    }

    #[test]
    fn test_object_shorthand() {
        let expr = parse("{ count }");
        match &expr.kind {
            ExprKind::Object(props) => {
                assert_eq!(props.len(), 1);
                assert!(props[0].shorthand);
            }
            _ => panic!("Expected object"),
        }
    }

    // =========================================================================
    // Parenthesized expressions
    // =========================================================================

    #[test]
    fn test_parens_override_precedence() {
        // (a + b) * c should have mul at top level
        let expr = parse("(a + b) * c");
        assert!(matches!(expr.kind, ExprKind::Binary { op: BinaryOp::Mul, .. }));
    }

    // =========================================================================
    // Complex expressions (real HRML patterns)
    // =========================================================================

    #[test]
    fn test_counter_increment() {
        let expr = parse("count++");
        assert!(matches!(expr.kind, ExprKind::Postfix { op: PostfixOp::Increment, .. }));
    }

    #[test]
    fn test_toggle_expression() {
        // visible = !visible
        let expr = parse("visible = !visible");
        assert!(matches!(expr.kind, ExprKind::Assignment { .. }));
    }

    #[test]
    fn test_reduce_expression() {
        let expr = parse("items.reduce((sum, i) => sum + i.price, 0)");
        assert!(matches!(expr.kind, ExprKind::Call { .. }));
    }

    // =========================================================================
    // Errors
    // =========================================================================

    #[test]
    fn test_error_unclosed_paren() {
        assert!(ExprParser::parse("(a + b").is_err());
    }

    #[test]
    fn test_error_unclosed_bracket() {
        assert!(ExprParser::parse("[1, 2").is_err());
    }

    #[test]
    fn test_error_unexpected_token() {
        assert!(ExprParser::parse("+ +").is_err());
    }
}
