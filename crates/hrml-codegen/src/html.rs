//! HTML code generator.
//!
//! Walks the HRML document AST and generates HTML output.
//! During generation, assigns auto-IDs (`hrml-N`) to reactive elements
//! and collects bindings for the JS generator.

use crate::{expr_to_js, expr_to_js_literal, Binding, CodegenError, CompilationContext};
use hrml_parser::ast::{AttributePrefix, Document, Element, ExprKind, Node};

/// Generate HTML from a document AST, populating bindings in the context.
pub fn generate(
    doc: &Document,
    ctx: &mut CompilationContext,
) -> Result<String, CodegenError> {
    let mut html = String::new();

    for node in &doc.nodes {
        generate_node(node, ctx, &mut html, 0)?;
    }

    Ok(html)
}

fn generate_node(
    node: &Node,
    ctx: &mut CompilationContext,
    out: &mut String,
    depth: usize,
) -> Result<(), CodegenError> {
    match node {
        Node::Element(el) => generate_element(el, ctx, out, depth),
        Node::Text(text) => {
            out.push_str(text);
            Ok(())
        }
        // State/Computed/Fn/Component/Comment don't generate HTML
        _ => Ok(()),
    }
}

fn generate_element(
    el: &Element,
    ctx: &mut CompilationContext,
    out: &mut String,
    depth: usize,
) -> Result<(), CodegenError> {
    let indent = "  ".repeat(depth);

    // Determine if this element needs an auto-generated ID
    let needs_id = element_needs_id(el);
    let id = if needs_id {
        Some(ctx.assign_id())
    } else {
        None
    };

    // Collect reactive bindings before generating HTML
    if let Some(ref id) = id {
        collect_bindings(el, id, ctx);
    }

    // Opening tag
    out.push_str(&indent);
    out.push('<');
    out.push_str(&el.tag);

    // ID attribute
    if let Some(ref id) = id {
        out.push_str(&format!(" id=\"{id}\""));
    }

    // Class attribute
    if !el.classes.is_empty() {
        out.push_str(&format!(" class=\"{}\"", el.classes.join(" ")));
    }

    // Plain attributes (no prefix) — rendered as HTML attributes
    for attr in &el.attributes {
        if attr.prefix.is_none() {
            out.push(' ');
            out.push_str(&attr.name);
            if let Some(ref value) = attr.value {
                let val_str = expr_to_html_attr(value);
                out.push_str(&format!("=\"{val_str}\""));
            }
        }
    }

    out.push('>');

    // Void elements — no closing tag
    if is_void_element(&el.tag) {
        out.push('\n');
        return Ok(());
    }

    // Children
    let has_element_children = el
        .children
        .iter()
        .any(|c| matches!(c, Node::Element(_)));
    let has_interpolated_text = el
        .children
        .iter()
        .any(|c| matches!(c, Node::Text(t) if t.contains('{')));

    if has_element_children {
        out.push('\n');
        for child in &el.children {
            generate_node(child, ctx, out, depth + 1)?;
        }
        out.push_str(&indent);
    } else if !has_interpolated_text {
        // Static text children — inline
        for child in &el.children {
            if let Node::Text(text) = child {
                out.push_str(text);
            }
        }
    }
    // If has interpolated text, leave element empty (JS fills it via hrml.text)

    // Closing tag
    out.push_str(&format!("</{}>", el.tag));
    out.push('\n');

    Ok(())
}

/// Check if an element needs an auto-generated ID for reactive bindings.
fn element_needs_id(el: &Element) -> bool {
    // Has event handlers
    el.attributes
        .iter()
        .any(|a| a.prefix == Some(AttributePrefix::Event))
    // Has reactive state directives
    || el.attributes.iter().any(|a| {
        a.prefix == Some(AttributePrefix::State)
            && matches!(
                a.name.as_str(),
                "show" | "if" | "model" | "class" | "text"
            )
    })
    // Has text interpolation in children
    || el
        .children
        .iter()
        .any(|c| matches!(c, Node::Text(t) if t.contains('{')))
}

/// Collect reactive bindings from an element into the compilation context.
fn collect_bindings(el: &Element, id: &str, ctx: &mut CompilationContext) {
    let state_names = ctx.state_names();

    for attr in &el.attributes {
        match attr.prefix {
            Some(AttributePrefix::Event) => {
                let handler = attr
                    .value
                    .as_ref()
                    .map(|v| expr_to_js(v, &state_names))
                    .unwrap_or_default();
                ctx.bindings.push(Binding::Event {
                    id: id.to_string(),
                    event: attr.name.clone(),
                    handler,
                    modifiers: attr.modifiers.clone(),
                });
            }
            Some(AttributePrefix::State) => match attr.name.as_str() {
                "show" | "if" => {
                    let expr = attr
                        .value
                        .as_ref()
                        .map(|v| expr_to_js(v, &state_names))
                        .unwrap_or_default();
                    ctx.bindings.push(Binding::Show {
                        id: id.to_string(),
                        expr,
                    });
                }
                "model" => {
                    let field = attr
                        .value
                        .as_ref()
                        .map(|v| match &v.kind {
                            ExprKind::Identifier(name) => name.clone(),
                            _ => expr_to_js(v, &[]),
                        })
                        .unwrap_or_default();
                    ctx.bindings.push(Binding::Model {
                        id: id.to_string(),
                        field,
                    });
                }
                _ => {}
            },
            _ => {}
        }
    }

    // Text interpolation binding
    let has_interpolation = el
        .children
        .iter()
        .any(|c| matches!(c, Node::Text(t) if t.contains('{')));
    if has_interpolation {
        let text: String = el
            .children
            .iter()
            .filter_map(|c| {
                if let Node::Text(t) = c {
                    Some(t.as_str())
                } else {
                    None
                }
            })
            .collect();
        let template = interpolate_text(&text, &state_names);
        ctx.bindings.push(Binding::Text {
            id: id.to_string(),
            template,
        });
    }
}

/// Convert an expression to an HTML attribute value string.
/// For strings, returns the raw content (no JS quoting).
fn expr_to_html_attr(expr: &hrml_parser::ast::Expression) -> String {
    match &expr.kind {
        ExprKind::String(s) => s.clone(),
        ExprKind::Number(n) => crate::format_number(*n),
        ExprKind::Boolean(b) => b.to_string(),
        ExprKind::Identifier(s) => s.clone(),
        _ => expr_to_js_literal(expr),
    }
}

/// Transform text with `{expr}` into a JS template literal body.
/// `"Count: {count}"` → `Count: ${_s.count}`
fn interpolate_text(text: &str, state_names: &[String]) -> String {
    let mut result = String::new();
    let mut chars = text.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '{' {
            let mut expr = String::new();
            let mut depth = 1;
            for next in chars.by_ref() {
                if next == '{' {
                    depth += 1;
                }
                if next == '}' {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                expr.push(next);
            }
            // Prefix first identifier with _s. if it's a state variable
            let first_ident: String = expr
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            if state_names.iter().any(|s| s == &first_ident) {
                result.push_str(&format!("${{_s.{expr}}}"));
            } else {
                result.push_str(&format!("${{{expr}}}"));
            }
        } else {
            result.push(ch);
        }
    }

    result
}

/// Check if an HTML tag is a void element (self-closing, no children).
fn is_void_element(tag: &str) -> bool {
    matches!(
        tag,
        "area"
            | "base"
            | "br"
            | "col"
            | "embed"
            | "hr"
            | "img"
            | "input"
            | "link"
            | "meta"
            | "param"
            | "source"
            | "track"
            | "wbr"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CompilationContext;
    use hrml_parser::ast::Document;

    fn parse(source: &str) -> Document {
        hrml_parser::Parser::parse(source).unwrap()
    }

    fn gen(source: &str) -> (String, CompilationContext) {
        let doc = parse(source);
        let mut ctx = CompilationContext::new();
        // Pre-collect state fields (same as compile())
        for node in &doc.nodes {
            if let Node::StateBlock(sb) = node {
                for field in &sb.fields {
                    let value = crate::expr_to_js_literal(&field.value);
                    ctx.state_fields.push((field.name.clone(), value));
                }
            }
        }
        let html = generate(&doc, &mut ctx).unwrap();
        (html, ctx)
    }

    // =========================================================================
    // Basic elements
    // =========================================================================

    #[test]
    fn test_empty_document() {
        let (html, _) = gen("");
        assert_eq!(html, "");
    }

    #[test]
    fn test_single_element() {
        let (html, _) = gen("div");
        assert_eq!(html, "<div></div>\n");
    }

    #[test]
    fn test_element_with_classes() {
        let (html, _) = gen("div .flex items-center");
        assert_eq!(html, "<div class=\"flex items-center\"></div>\n");
    }

    #[test]
    fn test_element_with_attribute() {
        let (html, _) = gen("a href=\"/about\"");
        assert_eq!(html, "<a href=\"/about\"></a>\n");
    }

    #[test]
    fn test_element_multiple_attributes() {
        let (html, _) = gen("img src=\"logo.png\" alt=\"Logo\"");
        assert_eq!(html, "<img src=\"logo.png\" alt=\"Logo\">\n");
    }

    #[test]
    fn test_element_with_static_text() {
        let (html, _) = gen("span \"Hello\"");
        assert_eq!(html, "<span>Hello</span>\n");
    }

    #[test]
    fn test_void_element() {
        let (html, _) = gen("br");
        assert_eq!(html, "<br>\n");
    }

    #[test]
    fn test_void_element_with_attributes() {
        let (html, _) = gen("input type=\"text\" name=\"email\"");
        assert_eq!(html, "<input type=\"text\" name=\"email\">\n");
    }

    #[test]
    fn test_implicit_div() {
        let (html, _) = gen(".container");
        assert_eq!(html, "<div class=\"container\"></div>\n");
    }

    // =========================================================================
    // Nesting
    // =========================================================================

    #[test]
    fn test_nested_elements() {
        let (html, _) = gen("div\n  span \"Hello\"");
        assert_eq!(html, "<div>\n  <span>Hello</span>\n</div>\n");
    }

    #[test]
    fn test_deep_nesting() {
        let (html, _) = gen("div\n  ul\n    li \"Item\"");
        assert_eq!(
            html,
            "<div>\n  <ul>\n    <li>Item</li>\n  </ul>\n</div>\n"
        );
    }

    #[test]
    fn test_multiple_children() {
        let (html, _) = gen("div\n  span \"A\"\n  span \"B\"");
        assert_eq!(
            html,
            "<div>\n  <span>A</span>\n  <span>B</span>\n</div>\n"
        );
    }

    // =========================================================================
    // Reactive elements (ID assignment)
    // =========================================================================

    #[test]
    fn test_event_handler_gets_id() {
        let (html, ctx) = gen("state\n  count: 0\n\nbutton @click=\"count++\" \"Click\"");
        assert!(html.contains("<button id=\"hrml-0\">Click</button>"));
        assert_eq!(ctx.bindings.len(), 1);
    }

    #[test]
    fn test_show_directive_gets_id() {
        let (html, ctx) = gen("state\n  visible: true\n\ndiv :show=\"visible\" \"Content\"");
        assert!(html.contains("<div id=\"hrml-0\">Content</div>"));
        assert_eq!(ctx.bindings.len(), 1);
    }

    #[test]
    fn test_model_directive_gets_id() {
        let (html, ctx) = gen("state\n  name: \"\"\n\ninput :model=\"name\"");
        assert!(html.contains("<input id=\"hrml-0\">"));
        assert_eq!(ctx.bindings.len(), 1);
    }

    #[test]
    fn test_interpolated_text_gets_id() {
        let (html, ctx) = gen("state\n  count: 0\n\nspan \"{count}\"");
        assert!(html.contains("<span id=\"hrml-0\"></span>"));
        assert_eq!(ctx.bindings.len(), 1);
    }

    #[test]
    fn test_static_element_no_id() {
        let (html, _) = gen("div .container\n  span \"Hello\"");
        assert!(!html.contains("id="));
    }

    // =========================================================================
    // Text interpolation
    // =========================================================================

    #[test]
    fn test_interpolate_simple() {
        let result = interpolate_text("{count}", &["count".into()]);
        assert_eq!(result, "${_s.count}");
    }

    #[test]
    fn test_interpolate_with_text() {
        let result = interpolate_text("Hello {name}!", &["name".into()]);
        assert_eq!(result, "Hello ${_s.name}!");
    }

    #[test]
    fn test_interpolate_multiple() {
        let result =
            interpolate_text("{first} {last}", &["first".into(), "last".into()]);
        assert_eq!(result, "${_s.first} ${_s.last}");
    }

    #[test]
    fn test_interpolate_non_state() {
        let result = interpolate_text("{foo}", &["count".into()]);
        assert_eq!(result, "${foo}");
    }

    // =========================================================================
    // Full examples
    // =========================================================================

    #[test]
    fn test_counter_html() {
        let (html, _) = gen(
            "state\n  count: 0\n\ndiv .counter\n  button @click=\"count++\" \"-\"\n  span \"{count}\"\n  button @click=\"count--\" \"+\"",
        );
        assert!(html.contains("<div class=\"counter\">"));
        assert!(html.contains("<button id=\"hrml-0\">-</button>"));
        assert!(html.contains("<span id=\"hrml-1\"></span>"));
        assert!(html.contains("<button id=\"hrml-2\">+</button>"));
        assert!(html.contains("</div>"));
    }

    #[test]
    fn test_toggle_html() {
        let (html, _) = gen(
            "state\n  visible: true\n\nbutton @click=\"visible = !visible\" \"Toggle\"\ndiv :show=\"visible\" \"Content\"",
        );
        assert!(html.contains("<button id=\"hrml-0\">Toggle</button>"));
        assert!(html.contains("<div id=\"hrml-1\">Content</div>"));
    }

    #[test]
    fn test_input_binding_html() {
        let (html, _) = gen(
            "state\n  name: \"\"\n\ninput :model=\"name\" placeholder=\"Type your name\"\nspan \"Hello {name}!\"",
        );
        assert!(html.contains("<input id=\"hrml-0\" placeholder=\"Type your name\">"));
        assert!(html.contains("<span id=\"hrml-1\"></span>"));
    }
}
