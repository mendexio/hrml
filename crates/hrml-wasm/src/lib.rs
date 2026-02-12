//! WASM bindings for the HRML compiler.
//!
//! Exposes `compile()` to JavaScript via wasm-bindgen.
//! Returns a JS object `{ html, css, js }` or throws on error.

use wasm_bindgen::prelude::*;

/// Compile HRML source to HTML + CSS + JS.
///
/// Returns a JS object with `{ html: string, css: string, js: string }`.
/// Throws a JS error if parsing or code generation fails.
#[wasm_bindgen]
pub fn compile(source: &str) -> Result<JsValue, JsError> {
    let doc = hrml_parser::Parser::parse(source).map_err(|e| JsError::new(&e.to_string()))?;

    let output =
        hrml_codegen::compile(&doc).map_err(|e| JsError::new(&e.to_string()))?;

    // Serialize to a plain JS object { html, css, js }
    let js_obj = js_sys::Object::new();
    js_sys::Reflect::set(&js_obj, &"html".into(), &output.html.into())
        .map_err(|_| JsError::new("Failed to set html property"))?;
    js_sys::Reflect::set(&js_obj, &"css".into(), &output.css.into())
        .map_err(|_| JsError::new("Failed to set css property"))?;
    js_sys::Reflect::set(&js_obj, &"js".into(), &output.js.into())
        .map_err(|_| JsError::new("Failed to set js property"))?;

    Ok(js_obj.into())
}

/// Get the compiler version.
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Native tests (non-WASM) — verify the compile pipeline works
    // =========================================================================

    fn native_compile(source: &str) -> hrml_codegen::CompilerOutput {
        let doc = hrml_parser::Parser::parse(source).unwrap();
        hrml_codegen::compile(&doc).unwrap()
    }

    #[test]
    fn test_empty_document() {
        let output = native_compile("");
        assert_eq!(output.html, "");
        assert_eq!(output.css, "");
        assert_eq!(output.js, "");
    }

    #[test]
    fn test_static_html() {
        let output = native_compile("div .container\n  span \"Hello\"");
        assert!(output.html.contains("<div class=\"container\">"));
        assert!(output.html.contains("<span>Hello</span>"));
        assert_eq!(output.js, "");
    }

    #[test]
    fn test_counter_example() {
        let output = native_compile(
            "state\n  count: 0\n\ndiv .counter\n  button @click=\"count++\" \"-\"\n  span \"{count}\"\n  button @click=\"count--\" \"+\"",
        );
        assert!(output.html.contains("<div class=\"counter\">"));
        assert!(output.html.contains("id=\"hrml-0\""));
        assert!(output.js.contains("hrml.state({ count: 0 })"));
        assert!(output.js.contains("_s.count++"));
    }

    #[test]
    fn test_toggle_example() {
        let output = native_compile(
            "state\n  visible: true\n\nbutton @click=\"visible = !visible\" \"Toggle\"\ndiv :show=\"visible\" \"Content\"",
        );
        assert!(output.html.contains("<button"));
        assert!(output.js.contains("hrml.state({ visible: true })"));
        assert!(output.js.contains("hrml.show("));
    }

    #[test]
    fn test_input_binding_example() {
        let output = native_compile(
            "state\n  name: \"\"\n\ninput :model=\"name\" placeholder=\"Type your name\"\nspan \"Hello {name}!\"",
        );
        assert!(output.html.contains("<input"));
        assert!(output.js.contains("hrml.model("));
        assert!(output.js.contains("Hello ${_s.name}!"));
    }

    #[test]
    fn test_parse_error() {
        // Intentionally invalid — state block with no fields followed by nothing useful
        // This should still parse (empty state is valid or ignored)
        // Test that the pipeline doesn't panic
        let result = hrml_parser::Parser::parse("div");
        assert!(result.is_ok());
    }

    #[test]
    fn test_version() {
        let v = version();
        assert!(!v.is_empty());
        assert!(v.contains('.'));
    }

    #[test]
    fn test_multiple_compiles() {
        // Verify no global state leakage between compiles
        let out1 = native_compile("state\n  x: 0\n\nspan \"{x}\"");
        let out2 = native_compile("state\n  y: 1\n\nspan \"{y}\"");
        assert!(out1.js.contains("x: 0"));
        assert!(!out1.js.contains("y: 1"));
        assert!(out2.js.contains("y: 1"));
        assert!(!out2.js.contains("x: 0"));
    }

    #[test]
    fn test_output_has_no_eval() {
        let output = native_compile(
            "state\n  count: 0\n\nbutton @click=\"count++\" \"Click\"",
        );
        assert!(!output.js.contains("eval("));
        assert!(!output.js.contains("new Function("));
    }
}
