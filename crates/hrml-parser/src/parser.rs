//! Document parser for HRML.
//!
//! Parses a stream of source-level tokens (from `hrml-lexer`) into a `Document` AST.
//! Handles indentation-based nesting, element detection, prefix attributes,
//! and delegates inline expressions to `expr_parser`.
//!
//! Uses recursive descent parsing adapted for HRML syntax.

use crate::ast::{
    Attribute, AttributePrefix, ComputedBlock, ComputedField, Document, Element, ExprKind,
    ExprSpan, Expression, Node, StateBlock, StateField,
};
use crate::expr_parser::ExprParser;
use crate::ParseError;
use hrml_lexer::{Token, TokenKind};

/// HRML document parser.
///
/// Converts a flat token stream from the source lexer into a hierarchical
/// `Document` AST using recursive descent.
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
        let tokens = hrml_lexer::Scanner::tokenize(source).map_err(|e| ParseError {
            message: e.message,
            line: e.line,
            column: e.column,
        })?;

        let mut parser = Parser::new(tokens);
        parser.parse_document()
    }

    /// Parse a full document.
    fn parse_document(&mut self) -> Result<Document, ParseError> {
        let mut nodes = Vec::new();

        while !self.is_at_end() {
            self.skip_newlines();
            if self.is_at_end() {
                break;
            }

            match &self.peek().kind {
                TokenKind::State => {
                    nodes.push(self.parse_state_block()?);
                }
                TokenKind::Computed => {
                    nodes.push(self.parse_computed_block()?);
                }
                TokenKind::Comment(_) => {
                    if let TokenKind::Comment(text) = &self.peek().kind {
                        let text = text.clone();
                        self.advance();
                        nodes.push(Node::Comment(text));
                    }
                }
                TokenKind::Identifier(_) | TokenKind::Dot => {
                    nodes.push(self.parse_element()?);
                }
                _ => {
                    // Skip unexpected tokens at top level
                    self.advance();
                }
            }
        }

        Ok(Document { nodes })
    }

    // =========================================================================
    // State and computed blocks
    // =========================================================================

    /// Parse `state` block:
    /// ```text
    /// state
    ///   count: 0
    ///   loading: false
    /// ```
    fn parse_state_block(&mut self) -> Result<Node, ParseError> {
        self.advance(); // consume `state`
        self.skip_newlines();

        let mut fields = Vec::new();

        if self.peek().kind == TokenKind::Indent {
            self.advance(); // consume indent

            while self.peek().kind != TokenKind::Dedent && !self.is_at_end() {
                self.skip_newlines();
                if self.peek().kind == TokenKind::Dedent {
                    break;
                }

                // Parse field: name: value
                let name = self.expect_identifier()?;

                if self.peek().kind != TokenKind::Colon {
                    return Err(self.error(format!("Expected ':' after state field '{name}'")));
                }
                self.advance(); // consume :

                let value = self.parse_inline_value()?;
                fields.push(StateField { name, value });

                self.skip_newlines();
            }

            if self.peek().kind == TokenKind::Dedent {
                self.advance();
            }
        }

        Ok(Node::StateBlock(StateBlock { fields }))
    }

    /// Parse `computed` block:
    /// ```text
    /// computed
    ///   fullName: firstName + " " + lastName
    /// ```
    fn parse_computed_block(&mut self) -> Result<Node, ParseError> {
        self.advance(); // consume `computed`
        self.skip_newlines();

        let mut fields = Vec::new();

        if self.peek().kind == TokenKind::Indent {
            self.advance(); // consume indent

            while self.peek().kind != TokenKind::Dedent && !self.is_at_end() {
                self.skip_newlines();
                if self.peek().kind == TokenKind::Dedent {
                    break;
                }

                let name = self.expect_identifier()?;

                if self.peek().kind != TokenKind::Colon {
                    return Err(self.error(format!("Expected ':' after computed field '{name}'")));
                }
                self.advance(); // consume :

                // Computed values are expressions — collect remaining tokens on the line
                let expr_source = self.collect_to_newline();
                let body = ExprParser::parse(&expr_source)?;
                fields.push(ComputedField { name, body });

                self.skip_newlines();
            }

            if self.peek().kind == TokenKind::Dedent {
                self.advance();
            }
        }

        Ok(Node::ComputedBlock(ComputedBlock { fields }))
    }

    // =========================================================================
    // Element parsing
    // =========================================================================

    /// Parse an element:
    /// ```text
    /// div .flex items-center
    ///   span "Hello"
    /// ```
    fn parse_element(&mut self) -> Result<Node, ParseError> {
        // Stage 1: Tag name (optional if starts with dot)
        let tag = if let TokenKind::Identifier(name) = &self.peek().kind {
            let name = name.clone();
            self.advance();
            name
        } else {
            // Implicit div for shorthand .class
            "div".to_string()
        };

        let mut classes = Vec::new();
        let mut attributes = Vec::new();
        let mut children = Vec::new();

        // Stage 2: Inline modifiers
        let mut has_seen_class = false;

        loop {
            match &self.peek().kind {
                // .class
                TokenKind::Dot => {
                    self.advance();
                    let class = self.expect_identifier()?;
                    classes.push(class);
                    has_seen_class = true;
                }

                // Identifier: class (bare, after first .class), or attribute (if followed by =)
                TokenKind::Identifier(name) => {
                    let name = name.clone();

                    if self.peek_next_is_equals() {
                        // Plain attribute: name="value" — store as string literal
                        self.advance(); // consume name
                        self.advance(); // consume =
                        let value = self.parse_plain_value()?;
                        attributes.push(Attribute {
                            name,
                            value: Some(value),
                            prefix: None,
                            modifiers: Vec::new(),
                        });
                    } else if has_seen_class {
                        // Additional class (bare identifier after first .class)
                        self.advance();
                        classes.push(name);
                    } else {
                        break;
                    }
                }

                // :directive
                TokenKind::Colon => {
                    self.advance();
                    let name = self.expect_identifier()?;

                    let value = if self.peek().kind == TokenKind::Equals {
                        self.advance();
                        Some(self.parse_expression_value()?)
                    } else {
                        None
                    };

                    attributes.push(Attribute {
                        name,
                        value,
                        prefix: Some(AttributePrefix::State),
                        modifiers: Vec::new(),
                    });
                }

                // @event with optional modifiers
                TokenKind::At => {
                    self.advance();
                    let name = self.expect_identifier()?;

                    // Parse .modifier chains
                    let mut modifiers = Vec::new();
                    while self.peek().kind == TokenKind::Dot {
                        self.advance();
                        modifiers.push(self.expect_identifier()?);
                    }

                    let value = if self.peek().kind == TokenKind::Equals {
                        self.advance();
                        Some(self.parse_expression_value()?)
                    } else {
                        None
                    };

                    attributes.push(Attribute {
                        name,
                        value,
                        prefix: Some(AttributePrefix::Event),
                        modifiers,
                    });
                }

                // $server
                TokenKind::Dollar => {
                    self.advance();
                    let name = self.expect_identifier()?;

                    let value = if self.peek().kind == TokenKind::Equals {
                        self.advance();
                        Some(self.parse_expression_value()?)
                    } else {
                        None
                    };

                    attributes.push(Attribute {
                        name,
                        value,
                        prefix: Some(AttributePrefix::Server),
                        modifiers: Vec::new(),
                    });
                }

                // Inline text: "Hello {name}"
                TokenKind::String(text) => {
                    children.push(Node::Text(text.clone()));
                    self.advance();
                }

                _ => break,
            }
        }

        // Stage 3: Children (indented block)
        self.skip_newlines();

        if self.peek().kind == TokenKind::Indent {
            self.advance(); // consume indent

            while self.peek().kind != TokenKind::Dedent && !self.is_at_end() {
                self.skip_newlines();
                if self.peek().kind == TokenKind::Dedent {
                    break;
                }

                match &self.peek().kind {
                    TokenKind::Comment(text) => {
                        let text = text.clone();
                        self.advance();
                        children.push(Node::Comment(text));
                    }
                    TokenKind::Identifier(_) | TokenKind::Dot => {
                        children.push(self.parse_element()?);
                    }
                    TokenKind::String(text) => {
                        children.push(Node::Text(text.clone()));
                        self.advance();
                    }
                    _ => {
                        self.advance(); // skip unexpected
                    }
                }
            }

            if self.peek().kind == TokenKind::Dedent {
                self.advance();
            }
        }

        Ok(Node::Element(Element {
            tag,
            classes,
            attributes,
            children,
        }))
    }

    // =========================================================================
    // Value parsing helpers
    // =========================================================================

    /// Parse a prefixed attribute value (`:show="expr"`, `@click="expr"`) — string content
    /// is passed to ExprParser since it contains reactive expressions.
    fn parse_expression_value(&mut self) -> Result<Expression, ParseError> {
        match &self.peek().kind {
            TokenKind::String(s) => {
                let s = s.clone();
                self.advance();
                ExprParser::parse(&s)
            }
            TokenKind::Identifier(s) => {
                let s = s.clone();
                self.advance();
                Ok(Expression {
                    kind: ExprKind::Identifier(s),
                    span: ExprSpan::new(0, 0),
                })
            }
            TokenKind::Number(n) => {
                let n = *n;
                self.advance();
                Ok(Expression {
                    kind: ExprKind::Number(n),
                    span: ExprSpan::new(0, 0),
                })
            }
            TokenKind::Boolean(b) => {
                let b = *b;
                self.advance();
                Ok(Expression {
                    kind: ExprKind::Boolean(b),
                    span: ExprSpan::new(0, 0),
                })
            }
            _ => Err(self.error(format!(
                "Expected attribute value, got {:?}",
                self.peek().kind
            ))),
        }
    }

    /// Parse a plain attribute value (`href="/about"`, `type="text"`) — string content
    /// is stored as a string literal, NOT parsed as an expression.
    fn parse_plain_value(&mut self) -> Result<Expression, ParseError> {
        match &self.peek().kind {
            TokenKind::String(s) => {
                let s = s.clone();
                self.advance();
                Ok(Expression {
                    kind: ExprKind::String(s),
                    span: ExprSpan::new(0, 0),
                })
            }
            TokenKind::Number(n) => {
                let n = *n;
                self.advance();
                Ok(Expression {
                    kind: ExprKind::Number(n),
                    span: ExprSpan::new(0, 0),
                })
            }
            TokenKind::Boolean(b) => {
                let b = *b;
                self.advance();
                Ok(Expression {
                    kind: ExprKind::Boolean(b),
                    span: ExprSpan::new(0, 0),
                })
            }
            TokenKind::Identifier(s) => {
                let s = s.clone();
                self.advance();
                Ok(Expression {
                    kind: ExprKind::Identifier(s),
                    span: ExprSpan::new(0, 0),
                })
            }
            _ => Err(self.error(format!(
                "Expected attribute value, got {:?}",
                self.peek().kind
            ))),
        }
    }

    /// Parse an inline value for state fields.
    /// Handles: numbers, strings, booleans, null, identifiers, arrays.
    fn parse_inline_value(&mut self) -> Result<Expression, ParseError> {
        match &self.peek().kind {
            TokenKind::Number(n) => {
                let n = *n;
                self.advance();
                Ok(Expression {
                    kind: ExprKind::Number(n),
                    span: ExprSpan::new(0, 0),
                })
            }
            TokenKind::String(s) => {
                let s = s.clone();
                self.advance();
                Ok(Expression {
                    kind: ExprKind::String(s),
                    span: ExprSpan::new(0, 0),
                })
            }
            TokenKind::Boolean(b) => {
                let b = *b;
                self.advance();
                Ok(Expression {
                    kind: ExprKind::Boolean(b),
                    span: ExprSpan::new(0, 0),
                })
            }
            TokenKind::Null => {
                self.advance();
                Ok(Expression {
                    kind: ExprKind::Null,
                    span: ExprSpan::new(0, 0),
                })
            }
            TokenKind::Identifier(name) => {
                let name = name.clone();
                self.advance();
                Ok(Expression {
                    kind: ExprKind::Identifier(name),
                    span: ExprSpan::new(0, 0),
                })
            }
            _ => Err(self.error(format!(
                "Expected value, got {:?}",
                self.peek().kind
            ))),
        }
    }

    /// Collect remaining token text until newline/dedent/eof for expression parsing.
    fn collect_to_newline(&mut self) -> String {
        let mut parts = Vec::new();

        while !self.is_at_end() {
            match &self.peek().kind {
                TokenKind::Newline | TokenKind::Dedent | TokenKind::Eof => break,
                TokenKind::Identifier(s) => {
                    parts.push(s.clone());
                    self.advance();
                }
                TokenKind::Number(n) => {
                    parts.push(n.to_string());
                    self.advance();
                }
                TokenKind::String(s) => {
                    parts.push(format!("\"{}\"", s));
                    self.advance();
                }
                TokenKind::Boolean(b) => {
                    parts.push(b.to_string());
                    self.advance();
                }
                TokenKind::Null => {
                    parts.push("null".into());
                    self.advance();
                }
                TokenKind::Dot => {
                    parts.push(".".into());
                    self.advance();
                }
                TokenKind::Colon => {
                    parts.push(":".into());
                    self.advance();
                }
                TokenKind::Equals => {
                    parts.push("=".into());
                    self.advance();
                }
                TokenKind::Comma => {
                    parts.push(",".into());
                    self.advance();
                }
                TokenKind::LParen => {
                    parts.push("(".into());
                    self.advance();
                }
                TokenKind::RParen => {
                    parts.push(")".into());
                    self.advance();
                }
                TokenKind::Plus => {
                    parts.push("+".into());
                    self.advance();
                }
                _ => {
                    self.advance();
                }
            }
        }

        parts.join(" ")
    }

    // =========================================================================
    // Token navigation helpers
    // =========================================================================

    fn peek(&self) -> &Token {
        static EOF: std::sync::LazyLock<Token> = std::sync::LazyLock::new(|| {
            Token::new(
                TokenKind::Eof,
                hrml_lexer::Span::new(0, 0, 0, 0),
            )
        });
        self.tokens.get(self.pos).unwrap_or(&EOF)
    }

    fn advance(&mut self) {
        if self.pos < self.tokens.len() {
            self.pos += 1;
        }
    }

    fn is_at_end(&self) -> bool {
        self.pos >= self.tokens.len() || matches!(self.peek().kind, TokenKind::Eof)
    }

    fn skip_newlines(&mut self) {
        while matches!(self.peek().kind, TokenKind::Newline) {
            self.advance();
        }
    }

    fn expect_identifier(&mut self) -> Result<String, ParseError> {
        if let TokenKind::Identifier(name) = &self.peek().kind {
            let name = name.clone();
            self.advance();
            Ok(name)
        } else {
            Err(self.error(format!(
                "Expected identifier, got {:?}",
                self.peek().kind
            )))
        }
    }

    /// Check if the next token after current is Equals.
    fn peek_next_is_equals(&self) -> bool {
        self.tokens
            .get(self.pos + 1)
            .is_some_and(|t| matches!(t.kind, TokenKind::Equals))
    }

    fn error(&self, message: String) -> ParseError {
        let token = self.peek();
        ParseError {
            message,
            line: token.span.line,
            column: token.span.column,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::*;

    fn parse(source: &str) -> Document {
        Parser::parse(source).unwrap()
    }

    fn first_element(doc: &Document) -> &Element {
        match &doc.nodes[0] {
            Node::Element(el) => el,
            other => panic!("Expected Element, got {other:?}"),
        }
    }

    fn first_state(doc: &Document) -> &StateBlock {
        match &doc.nodes[0] {
            Node::StateBlock(sb) => sb,
            other => panic!("Expected StateBlock, got {other:?}"),
        }
    }

    // =========================================================================
    // Empty / simple
    // =========================================================================

    #[test]
    fn test_empty_document() {
        let doc = parse("");
        assert!(doc.nodes.is_empty());
    }

    #[test]
    fn test_single_element() {
        let doc = parse("div");
        let el = first_element(&doc);
        assert_eq!(el.tag, "div");
        assert!(el.classes.is_empty());
        assert!(el.attributes.is_empty());
        assert!(el.children.is_empty());
    }

    #[test]
    fn test_multiple_elements() {
        let doc = parse("div\nspan\np");
        assert_eq!(doc.nodes.len(), 3);
    }

    // =========================================================================
    // Classes
    // =========================================================================

    #[test]
    fn test_single_class() {
        let doc = parse("div .container");
        let el = first_element(&doc);
        assert_eq!(el.classes, vec!["container"]);
    }

    #[test]
    fn test_multiple_classes() {
        let doc = parse("div .flex items-center gap-4");
        let el = first_element(&doc);
        assert_eq!(el.classes, vec!["flex", "items-center", "gap-4"]);
    }

    #[test]
    fn test_implicit_div() {
        let doc = parse(".container");
        let el = first_element(&doc);
        assert_eq!(el.tag, "div");
        assert_eq!(el.classes, vec!["container"]);
    }

    #[test]
    fn test_classes_with_hyphen() {
        let doc = parse("div .text-2xl bg-blue-500");
        let el = first_element(&doc);
        assert_eq!(el.classes, vec!["text-2xl", "bg-blue-500"]);
    }

    // =========================================================================
    // Attributes
    // =========================================================================

    #[test]
    fn test_attribute_with_string() {
        let doc = parse("a href=\"/about\"");
        let el = first_element(&doc);
        assert_eq!(el.attributes.len(), 1);
        assert_eq!(el.attributes[0].name, "href");
        assert!(el.attributes[0].prefix.is_none());
    }

    #[test]
    fn test_multiple_attributes() {
        let doc = parse("img src=\"logo.png\" alt=\"Logo\"");
        let el = first_element(&doc);
        assert_eq!(el.attributes.len(), 2);
        assert_eq!(el.attributes[0].name, "src");
        assert_eq!(el.attributes[1].name, "alt");
    }

    #[test]
    fn test_class_then_attribute() {
        let doc = parse("input .form-input type=\"text\" name=\"email\"");
        let el = first_element(&doc);
        assert_eq!(el.classes, vec!["form-input"]);
        assert_eq!(el.attributes.len(), 2);
    }

    // =========================================================================
    // State directives
    // =========================================================================

    #[test]
    fn test_show_directive() {
        let doc = parse("div :show=\"visible\"");
        let el = first_element(&doc);
        assert_eq!(el.attributes.len(), 1);
        assert_eq!(el.attributes[0].name, "show");
        assert_eq!(el.attributes[0].prefix, Some(AttributePrefix::State));
    }

    #[test]
    fn test_model_directive() {
        let doc = parse("input :model=\"name\"");
        let el = first_element(&doc);
        assert_eq!(el.attributes[0].name, "model");
        assert_eq!(el.attributes[0].prefix, Some(AttributePrefix::State));
    }

    #[test]
    fn test_if_directive() {
        let doc = parse("div :if=\"count > 0\"");
        let el = first_element(&doc);
        assert_eq!(el.attributes[0].name, "if");
    }

    // =========================================================================
    // Event handlers
    // =========================================================================

    #[test]
    fn test_click_event() {
        let doc = parse("button @click=\"count++\"");
        let el = first_element(&doc);
        assert_eq!(el.attributes[0].name, "click");
        assert_eq!(el.attributes[0].prefix, Some(AttributePrefix::Event));
        assert!(el.attributes[0].modifiers.is_empty());
    }

    #[test]
    fn test_event_with_modifier() {
        let doc = parse("form @submit.prevent=\"save()\"");
        let el = first_element(&doc);
        assert_eq!(el.attributes[0].name, "submit");
        assert_eq!(el.attributes[0].modifiers, vec!["prevent"]);
    }

    #[test]
    fn test_event_multiple_modifiers() {
        let doc = parse("input @keydown.ctrl.enter=\"submit()\"");
        let el = first_element(&doc);
        assert_eq!(el.attributes[0].name, "keydown");
        assert_eq!(el.attributes[0].modifiers, vec!["ctrl", "enter"]);
    }

    // =========================================================================
    // Text content
    // =========================================================================

    #[test]
    fn test_inline_text() {
        let doc = parse("span \"Hello\"");
        let el = first_element(&doc);
        assert_eq!(el.children.len(), 1);
        assert!(matches!(&el.children[0], Node::Text(t) if t == "Hello"));
    }

    #[test]
    fn test_text_with_interpolation() {
        let doc = parse("span \"Count: {count}\"");
        let el = first_element(&doc);
        assert!(matches!(&el.children[0], Node::Text(t) if t == "Count: {count}"));
    }

    // =========================================================================
    // Nesting (indentation)
    // =========================================================================

    #[test]
    fn test_nested_children() {
        let doc = parse("div\n  span\n  p");
        let el = first_element(&doc);
        assert_eq!(el.children.len(), 2);
    }

    #[test]
    fn test_deep_nesting() {
        let doc = parse("div\n  ul\n    li \"Item\"");
        let el = first_element(&doc);
        assert_eq!(el.children.len(), 1); // ul
        match &el.children[0] {
            Node::Element(ul) => {
                assert_eq!(ul.tag, "ul");
                assert_eq!(ul.children.len(), 1); // li
            }
            _ => panic!("Expected ul element"),
        }
    }

    #[test]
    fn test_siblings_after_nesting() {
        let doc = parse("div\n  span\nbutton");
        assert_eq!(doc.nodes.len(), 2); // div and button
    }

    // =========================================================================
    // State blocks
    // =========================================================================

    #[test]
    fn test_state_block() {
        let doc = parse("state\n  count: 0\n  loading: false");
        let sb = first_state(&doc);
        assert_eq!(sb.fields.len(), 2);
        assert_eq!(sb.fields[0].name, "count");
        assert_eq!(sb.fields[1].name, "loading");
    }

    #[test]
    fn test_state_string_value() {
        let doc = parse("state\n  name: \"John\"");
        let sb = first_state(&doc);
        assert_eq!(sb.fields[0].name, "name");
        assert!(matches!(sb.fields[0].value.kind, ExprKind::String(ref s) if s == "John"));
    }

    #[test]
    fn test_state_null_value() {
        let doc = parse("state\n  error: null");
        let sb = first_state(&doc);
        assert!(matches!(sb.fields[0].value.kind, ExprKind::Null));
    }

    // =========================================================================
    // Computed blocks
    // =========================================================================

    #[test]
    fn test_computed_block() {
        let doc = parse("computed\n  double: count + count");
        match &doc.nodes[0] {
            Node::ComputedBlock(cb) => {
                assert_eq!(cb.fields.len(), 1);
                assert_eq!(cb.fields[0].name, "double");
            }
            other => panic!("Expected ComputedBlock, got {other:?}"),
        }
    }

    // =========================================================================
    // Comments
    // =========================================================================

    #[test]
    fn test_comment() {
        let doc = parse("// This is a comment");
        assert!(matches!(&doc.nodes[0], Node::Comment(t) if t == "This is a comment"));
    }

    // =========================================================================
    // Full examples (the 3 prototype examples)
    // =========================================================================

    #[test]
    fn test_counter_example() {
        let doc = parse(
            "state\n  count: 0\n\ndiv .counter\n  button @click=\"count++\" \"-\"\n  span \"{count}\"\n  button @click=\"count--\" \"+\"",
        );

        // Should have state block + div
        assert_eq!(doc.nodes.len(), 2);
        assert!(matches!(&doc.nodes[0], Node::StateBlock(_)));

        let el = match &doc.nodes[1] {
            Node::Element(el) => el,
            _ => panic!("Expected element"),
        };
        assert_eq!(el.tag, "div");
        assert_eq!(el.classes, vec!["counter"]);
        assert_eq!(el.children.len(), 3); // 3 children: button, span, button
    }

    #[test]
    fn test_toggle_example() {
        let doc = parse(
            "state\n  visible: true\n\nbutton @click=\"visible = !visible\" \"Toggle\"\ndiv :show=\"visible\" \"Content\"",
        );

        assert_eq!(doc.nodes.len(), 3); // state + button + div
    }

    #[test]
    fn test_input_binding_example() {
        let doc = parse(
            "state\n  name: \"\"\n\ninput :model=\"name\" placeholder=\"Type your name\"\nspan \"Hello {name}!\"",
        );

        assert_eq!(doc.nodes.len(), 3); // state + input + span
    }

    // =========================================================================
    // Day 7: Additional test cases (5 new tests)
    // =========================================================================

    #[test]
    fn test_very_deep_nesting() {
        let doc = parse("div\n  section\n    article\n      header\n        h1 \"Title\"");
        let el = first_element(&doc);
        assert_eq!(el.tag, "div");

        // Navigate: div -> section -> article -> header -> h1
        let section = match &el.children[0] {
            Node::Element(e) => e,
            _ => panic!("Expected section element"),
        };
        assert_eq!(section.tag, "section");

        let article = match &section.children[0] {
            Node::Element(e) => e,
            _ => panic!("Expected article element"),
        };
        assert_eq!(article.tag, "article");

        let header = match &article.children[0] {
            Node::Element(e) => e,
            _ => panic!("Expected header element"),
        };
        assert_eq!(header.tag, "header");
        assert_eq!(header.children.len(), 1);
    }

    #[test]
    fn test_state_multiple_types() {
        let doc = parse("state\n  count: 42\n  name: \"Alice\"\n  active: true\n  data: null");
        let sb = first_state(&doc);

        assert_eq!(sb.fields.len(), 4);
        assert_eq!(sb.fields[0].name, "count");
        assert!(matches!(sb.fields[0].value.kind, ExprKind::Number(n) if n == 42.0));

        assert_eq!(sb.fields[1].name, "name");
        assert!(matches!(sb.fields[1].value.kind, ExprKind::String(ref s) if s == "Alice"));

        assert_eq!(sb.fields[2].name, "active");
        assert!(matches!(sb.fields[2].value.kind, ExprKind::Boolean(true)));

        assert_eq!(sb.fields[3].name, "data");
        assert!(matches!(sb.fields[3].value.kind, ExprKind::Null));
    }

    #[test]
    fn test_complex_event_expression() {
        let doc = parse("button @click=\"count = count + 10\"");
        let el = first_element(&doc);

        assert_eq!(el.attributes.len(), 1);
        assert_eq!(el.attributes[0].name, "click");
        assert_eq!(el.attributes[0].prefix, Some(AttributePrefix::Event));

        // Verify the expression parsed successfully (parser doesn't fail on complex expressions)
        assert!(el.attributes[0].value.is_some(), "Event handler should have an expression");
        // The complex expression should be parsed without errors
        // (exact AST structure depends on expression parser implementation)
    }

    #[test]
    fn test_multiple_interpolations() {
        let doc = parse("p \"User: {firstName} {lastName}, Email: {email}\"");
        let el = first_element(&doc);

        assert_eq!(el.children.len(), 1);
        match &el.children[0] {
            Node::Text(text) => {
                assert!(text.contains("{firstName}"));
                assert!(text.contains("{lastName}"));
                assert!(text.contains("{email}"));
            },
            _ => panic!("Expected text node"),
        }
    }

    #[test]
    fn test_mixed_directives_and_attributes() {
        let doc = parse("input .form-input type=\"email\" :model=\"email\" @input=\"validate()\" placeholder=\"Enter email\"");
        let el = first_element(&doc);

        // Classes
        assert_eq!(el.classes, vec!["form-input"]);

        // Should have: type, :model, @input, placeholder (4 attributes)
        assert_eq!(el.attributes.len(), 4);

        // Verify prefixes
        let has_plain = el.attributes.iter().any(|a| a.prefix.is_none());
        let has_state = el.attributes.iter().any(|a| a.prefix == Some(AttributePrefix::State));
        let has_event = el.attributes.iter().any(|a| a.prefix == Some(AttributePrefix::Event));

        assert!(has_plain, "Should have plain attributes");
        assert!(has_state, "Should have state directive");
        assert!(has_event, "Should have event handler");
    }
}
