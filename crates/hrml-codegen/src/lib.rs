//! HRML Code Generator
//!
//! Compiles the HRML AST into three outputs: HTML, CSS, and JavaScript.
//! HTML generation collects reactive bindings; JS generation emits the runtime
//! and binding code. CSS is empty for the prototype (Tailwind CDN in playground).
//!
//! ```text
//! Document AST → compile() → CompilerOutput { html, css, js }
//! ```

pub mod css;
pub mod html;
pub mod js;

use hrml_parser::ast::{
    AssignOp, BinaryOp, Document, ExprKind, Expression, Node, PostfixOp, UnaryOp,
};

/// The compiled output from an HRML document.
#[derive(Debug, Clone, PartialEq)]
pub struct CompilerOutput {
    pub html: String,
    pub css: String,
    pub js: String,
}

/// Code generation error.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[error("Codegen error: {message}")]
pub struct CodegenError {
    pub message: String,
}

/// Shared context between HTML and JS generators.
/// HTML generation populates bindings; JS generation reads them.
#[derive(Default)]
pub struct CompilationContext {
    pub next_id: usize,
    pub bindings: Vec<Binding>,
    pub state_fields: Vec<(String, String)>,
    pub computed_fields: Vec<(String, String)>,
}

impl CompilationContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn assign_id(&mut self) -> String {
        let id = format!("hrml-{}", self.next_id);
        self.next_id += 1;
        id
    }

    pub fn state_names(&self) -> Vec<String> {
        self.state_fields.iter().map(|(name, _)| name.clone()).collect()
    }
}

/// A reactive binding collected during HTML generation.
pub enum Binding {
    /// `hrml.text(id, () => template)`
    Text { id: String, template: String },
    /// `hrml.on(id, event, handler)`
    Event {
        id: String,
        event: String,
        handler: String,
        modifiers: Vec<String>,
    },
    /// `hrml.show(id, () => expr)`
    Show { id: String, expr: String },
    /// `hrml.model(id, _s, 'field')`
    Model { id: String, field: String },
}

/// Compile an HRML document AST into HTML + CSS + JS.
pub fn compile(doc: &Document) -> Result<CompilerOutput, CodegenError> {
    let mut ctx = CompilationContext::new();

    // Pre-pass: collect state and computed fields
    for node in &doc.nodes {
        match node {
            Node::StateBlock(sb) => {
                for field in &sb.fields {
                    let value = expr_to_js_literal(&field.value);
                    ctx.state_fields.push((field.name.clone(), value));
                }
            }
            Node::ComputedBlock(cb) => {
                for field in &cb.fields {
                    let state_names = ctx.state_names();
                    let expr = expr_to_js(&field.body, &state_names);
                    ctx.computed_fields.push((field.name.clone(), expr));
                }
            }
            _ => {}
        }
    }

    let html_output = html::generate(doc, &mut ctx)?;
    let css_output = css::generate(doc)?;
    let js_output = js::generate(&ctx)?;

    Ok(CompilerOutput {
        html: html_output,
        css: css_output,
        js: js_output,
    })
}

// =========================================================================
// Expression → JavaScript conversion
// =========================================================================

/// Convert an expression to a JS literal (for state field initialization).
/// Does NOT prefix identifiers with `_s.`.
pub fn expr_to_js_literal(expr: &Expression) -> String {
    match &expr.kind {
        ExprKind::Number(n) => format_number(*n),
        ExprKind::String(s) => format!("'{}'", s.replace('\\', "\\\\").replace('\'', "\\'")),
        ExprKind::Boolean(b) => b.to_string(),
        ExprKind::Null => "null".into(),
        ExprKind::Undefined => "undefined".into(),
        ExprKind::Identifier(name) => name.clone(),
        ExprKind::Array(items) => {
            let parts: Vec<String> = items.iter().map(expr_to_js_literal).collect();
            format!("[{}]", parts.join(", "))
        }
        ExprKind::Object(props) => {
            let parts: Vec<String> = props
                .iter()
                .map(|p| {
                    if p.shorthand {
                        p.key.clone()
                    } else {
                        format!("{}: {}", p.key, expr_to_js_literal(&p.value))
                    }
                })
                .collect();
            format!("{{ {} }}", parts.join(", "))
        }
        _ => expr_to_js(expr, &[]),
    }
}

/// Convert an expression to JS code, prefixing state variable identifiers with `_s.`.
pub fn expr_to_js(expr: &Expression, state_names: &[String]) -> String {
    match &expr.kind {
        ExprKind::Number(n) => format_number(*n),
        ExprKind::String(s) => format!("'{}'", s.replace('\\', "\\\\").replace('\'', "\\'")),
        ExprKind::Boolean(b) => b.to_string(),
        ExprKind::Null => "null".into(),
        ExprKind::Undefined => "undefined".into(),
        ExprKind::Identifier(name) => {
            if state_names.iter().any(|s| s == name) {
                format!("_s.{name}")
            } else {
                name.clone()
            }
        }
        ExprKind::Binary { left, op, right } => {
            format!(
                "{} {} {}",
                expr_to_js(left, state_names),
                binary_op_to_js(*op),
                expr_to_js(right, state_names),
            )
        }
        ExprKind::Unary { op, operand } => {
            let op_str = unary_op_to_js(*op);
            let operand_str = expr_to_js(operand, state_names);
            if matches!(op, UnaryOp::Typeof) {
                format!("{op_str} {operand_str}")
            } else {
                format!("{op_str}{operand_str}")
            }
        }
        ExprKind::Postfix { operand, op } => {
            format!(
                "{}{}",
                expr_to_js(operand, state_names),
                postfix_op_to_js(*op)
            )
        }
        ExprKind::Assignment { target, op, value } => {
            format!(
                "{} {} {}",
                expr_to_js(target, state_names),
                assign_op_to_js(*op),
                expr_to_js(value, state_names),
            )
        }
        ExprKind::Member {
            object,
            property,
            computed,
        } => {
            if *computed {
                format!(
                    "{}[{}]",
                    expr_to_js(object, state_names),
                    expr_to_js(property, state_names),
                )
            } else {
                format!(
                    "{}.{}",
                    expr_to_js(object, state_names),
                    expr_to_js(property, state_names),
                )
            }
        }
        ExprKind::Call { callee, arguments } => {
            let args: Vec<String> = arguments
                .iter()
                .map(|a| expr_to_js(a, state_names))
                .collect();
            format!(
                "{}({})",
                expr_to_js(callee, state_names),
                args.join(", ")
            )
        }
        ExprKind::Ternary {
            condition,
            consequent,
            alternate,
        } => {
            format!(
                "{} ? {} : {}",
                expr_to_js(condition, state_names),
                expr_to_js(consequent, state_names),
                expr_to_js(alternate, state_names),
            )
        }
        ExprKind::Object(props) => {
            let parts: Vec<String> = props
                .iter()
                .map(|p| {
                    if p.shorthand {
                        let ident_expr = Expression {
                            kind: ExprKind::Identifier(p.key.clone()),
                            span: expr.span,
                        };
                        format!("{}: {}", p.key, expr_to_js(&ident_expr, state_names))
                    } else {
                        format!("{}: {}", p.key, expr_to_js(&p.value, state_names))
                    }
                })
                .collect();
            format!("{{ {} }}", parts.join(", "))
        }
        ExprKind::Array(items) => {
            let parts: Vec<String> = items
                .iter()
                .map(|i| expr_to_js(i, state_names))
                .collect();
            format!("[{}]", parts.join(", "))
        }
        ExprKind::Arrow { params, body } => {
            let ps = if params.len() == 1 {
                params[0].clone()
            } else {
                format!("({})", params.join(", "))
            };
            format!("{ps} => {}", expr_to_js(body, state_names))
        }
        ExprKind::Interpolation(inner) => expr_to_js(inner, state_names),
    }
}

/// Format a number, removing `.0` for integers.
pub fn format_number(n: f64) -> String {
    if n.fract() == 0.0 && n.is_finite() {
        format!("{}", n as i64)
    } else {
        format!("{n}")
    }
}

fn binary_op_to_js(op: BinaryOp) -> &'static str {
    match op {
        BinaryOp::Add => "+",
        BinaryOp::Sub => "-",
        BinaryOp::Mul => "*",
        BinaryOp::Div => "/",
        BinaryOp::Mod => "%",
        BinaryOp::Eq => "==",
        BinaryOp::Neq => "!=",
        BinaryOp::StrictEq => "===",
        BinaryOp::StrictNeq => "!==",
        BinaryOp::Lt => "<",
        BinaryOp::Gt => ">",
        BinaryOp::Lte => "<=",
        BinaryOp::Gte => ">=",
        BinaryOp::And => "&&",
        BinaryOp::Or => "||",
        BinaryOp::NullishCoalescing => "??",
    }
}

fn unary_op_to_js(op: UnaryOp) -> &'static str {
    match op {
        UnaryOp::Not => "!",
        UnaryOp::Neg => "-",
        UnaryOp::Typeof => "typeof",
    }
}

fn postfix_op_to_js(op: PostfixOp) -> &'static str {
    match op {
        PostfixOp::Increment => "++",
        PostfixOp::Decrement => "--",
    }
}

fn assign_op_to_js(op: AssignOp) -> &'static str {
    match op {
        AssignOp::Assign => "=",
        AssignOp::AddAssign => "+=",
        AssignOp::SubAssign => "-=",
        AssignOp::MulAssign => "*=",
        AssignOp::DivAssign => "/=",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hrml_parser::ast::ExprSpan;

    fn make_expr(kind: ExprKind) -> Expression {
        Expression {
            kind,
            span: ExprSpan::new(0, 0),
        }
    }

    // =========================================================================
    // expr_to_js_literal
    // =========================================================================

    #[test]
    fn test_literal_number_integer() {
        assert_eq!(expr_to_js_literal(&make_expr(ExprKind::Number(42.0))), "42");
    }

    #[test]
    fn test_literal_number_float() {
        assert_eq!(
            expr_to_js_literal(&make_expr(ExprKind::Number(3.14))),
            "3.14"
        );
    }

    #[test]
    fn test_literal_number_zero() {
        assert_eq!(expr_to_js_literal(&make_expr(ExprKind::Number(0.0))), "0");
    }

    #[test]
    fn test_literal_string() {
        assert_eq!(
            expr_to_js_literal(&make_expr(ExprKind::String("hello".into()))),
            "'hello'"
        );
    }

    #[test]
    fn test_literal_empty_string() {
        assert_eq!(
            expr_to_js_literal(&make_expr(ExprKind::String(String::new()))),
            "''"
        );
    }

    #[test]
    fn test_literal_boolean_true() {
        assert_eq!(
            expr_to_js_literal(&make_expr(ExprKind::Boolean(true))),
            "true"
        );
    }

    #[test]
    fn test_literal_boolean_false() {
        assert_eq!(
            expr_to_js_literal(&make_expr(ExprKind::Boolean(false))),
            "false"
        );
    }

    #[test]
    fn test_literal_null() {
        assert_eq!(expr_to_js_literal(&make_expr(ExprKind::Null)), "null");
    }

    // =========================================================================
    // expr_to_js (with state prefixing)
    // =========================================================================

    #[test]
    fn test_js_identifier_state() {
        let state = vec!["count".into()];
        assert_eq!(
            expr_to_js(&make_expr(ExprKind::Identifier("count".into())), &state),
            "_s.count"
        );
    }

    #[test]
    fn test_js_identifier_non_state() {
        let state = vec!["count".into()];
        assert_eq!(
            expr_to_js(
                &make_expr(ExprKind::Identifier("console".into())),
                &state
            ),
            "console"
        );
    }

    #[test]
    fn test_js_binary() {
        let state = vec!["count".into()];
        let expr = make_expr(ExprKind::Binary {
            left: Box::new(make_expr(ExprKind::Identifier("count".into()))),
            op: BinaryOp::Gt,
            right: Box::new(make_expr(ExprKind::Number(0.0))),
        });
        assert_eq!(expr_to_js(&expr, &state), "_s.count > 0");
    }

    #[test]
    fn test_js_unary_not() {
        let state = vec!["visible".into()];
        let expr = make_expr(ExprKind::Unary {
            op: UnaryOp::Not,
            operand: Box::new(make_expr(ExprKind::Identifier("visible".into()))),
        });
        assert_eq!(expr_to_js(&expr, &state), "!_s.visible");
    }

    #[test]
    fn test_js_postfix_increment() {
        let state = vec!["count".into()];
        let expr = make_expr(ExprKind::Postfix {
            operand: Box::new(make_expr(ExprKind::Identifier("count".into()))),
            op: PostfixOp::Increment,
        });
        assert_eq!(expr_to_js(&expr, &state), "_s.count++");
    }

    #[test]
    fn test_js_assignment() {
        let state = vec!["visible".into()];
        let expr = make_expr(ExprKind::Assignment {
            target: Box::new(make_expr(ExprKind::Identifier("visible".into()))),
            op: AssignOp::Assign,
            value: Box::new(make_expr(ExprKind::Unary {
                op: UnaryOp::Not,
                operand: Box::new(make_expr(ExprKind::Identifier("visible".into()))),
            })),
        });
        assert_eq!(expr_to_js(&expr, &state), "_s.visible = !_s.visible");
    }

    #[test]
    fn test_js_member_access() {
        let state = vec!["items".into()];
        let expr = make_expr(ExprKind::Member {
            object: Box::new(make_expr(ExprKind::Identifier("items".into()))),
            property: Box::new(make_expr(ExprKind::Identifier("length".into()))),
            computed: false,
        });
        assert_eq!(expr_to_js(&expr, &state), "_s.items.length");
    }

    #[test]
    fn test_js_call() {
        let state = vec!["count".into()];
        let expr = make_expr(ExprKind::Call {
            callee: Box::new(make_expr(ExprKind::Identifier("save".into()))),
            arguments: vec![make_expr(ExprKind::Identifier("count".into()))],
        });
        assert_eq!(expr_to_js(&expr, &state), "save(_s.count)");
    }

    #[test]
    fn test_js_ternary() {
        let state = vec!["count".into()];
        let expr = make_expr(ExprKind::Ternary {
            condition: Box::new(make_expr(ExprKind::Binary {
                left: Box::new(make_expr(ExprKind::Identifier("count".into()))),
                op: BinaryOp::Gt,
                right: Box::new(make_expr(ExprKind::Number(0.0))),
            })),
            consequent: Box::new(make_expr(ExprKind::String("yes".into()))),
            alternate: Box::new(make_expr(ExprKind::String("no".into()))),
        });
        assert_eq!(
            expr_to_js(&expr, &state),
            "_s.count > 0 ? 'yes' : 'no'"
        );
    }

    // =========================================================================
    // Integration: compile()
    // =========================================================================

    fn parse(source: &str) -> Document {
        hrml_parser::Parser::parse(source).unwrap()
    }

    #[test]
    fn test_compile_empty() {
        let doc = parse("");
        let output = compile(&doc).unwrap();
        assert_eq!(output.html, "");
        assert_eq!(output.css, "");
        assert_eq!(output.js, "");
    }

    #[test]
    fn test_compile_counter() {
        let doc = parse(
            "state\n  count: 0\n\ndiv .counter\n  button @click=\"count++\" \"-\"\n  span \"{count}\"\n  button @click=\"count--\" \"+\"",
        );
        let output = compile(&doc).unwrap();

        // HTML has structure
        assert!(output.html.contains("<div class=\"counter\">"));
        assert!(output.html.contains("<button id=\"hrml-0\">-</button>"));
        assert!(output.html.contains("<span id=\"hrml-1\"></span>"));
        assert!(output.html.contains("<button id=\"hrml-2\">+</button>"));

        // JS has state and bindings
        assert!(output.js.contains("hrml.state({ count: 0 })"));
        assert!(output.js.contains("_s.count++"));
        assert!(output.js.contains("_s.count--"));
        assert!(output.js.contains("${_s.count}"));
    }

    #[test]
    fn test_compile_toggle() {
        let doc = parse(
            "state\n  visible: true\n\nbutton @click=\"visible = !visible\" \"Toggle\"\ndiv :show=\"visible\" \"Content\"",
        );
        let output = compile(&doc).unwrap();

        assert!(output.html.contains("<button id=\"hrml-0\">Toggle</button>"));
        assert!(output.html.contains("<div id=\"hrml-1\">Content</div>"));
        assert!(output.js.contains("hrml.state({ visible: true })"));
        assert!(output.js.contains("_s.visible = !_s.visible"));
        assert!(output.js.contains("hrml.show("));
    }

    #[test]
    fn test_compile_input_binding() {
        let doc = parse(
            "state\n  name: \"\"\n\ninput :model=\"name\" placeholder=\"Type your name\"\nspan \"Hello {name}!\"",
        );
        let output = compile(&doc).unwrap();

        assert!(output.html.contains("<input id=\"hrml-0\""));
        assert!(output.html.contains("placeholder=\"Type your name\""));
        assert!(output.html.contains("<span id=\"hrml-1\"></span>"));
        assert!(output.js.contains("hrml.state({ name: '' })"));
        assert!(output.js.contains("hrml.model("));
        assert!(output.js.contains("Hello ${_s.name}!"));
    }
}
