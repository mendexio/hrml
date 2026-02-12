//! JavaScript code generator.
//!
//! Generates reactive JavaScript from the compilation context.
//! Includes the HRML runtime (~50 lines) and compiled bindings.
//! No eval(), no new Function() — fully CSP-safe.

use crate::{Binding, CodegenError, CompilationContext};

/// The HRML reactive runtime.
/// Proxy-based state with batched effects via queueMicrotask.
const RUNTIME: &str = r#"const hrml = (() => {
  const _effects = [];
  let _queued = false;
  function _flush() {
    _queued = false;
    _effects.forEach(fn => fn());
  }
  function _notify() {
    if (!_queued) {
      _queued = true;
      queueMicrotask(_flush);
    }
  }
  function state(init) {
    return new Proxy(init, {
      set(target, key, value) {
        if (target[key] === value) return true;
        target[key] = value;
        _notify();
        return true;
      }
    });
  }
  function effect(fn) { _effects.push(fn); fn(); }
  function text(id, fn) {
    effect(() => {
      const el = document.getElementById(id);
      if (el) el.textContent = fn();
    });
  }
  function show(id, fn) {
    effect(() => {
      const el = document.getElementById(id);
      if (el) el.style.display = fn() ? '' : 'none';
    });
  }
  function model(id, s, key) {
    const el = document.getElementById(id);
    if (!el) return;
    el.addEventListener('input', e => { s[key] = e.target.value; });
    effect(() => { if (el.value !== String(s[key])) el.value = s[key]; });
  }
  function on(id, event, handler) {
    const el = document.getElementById(id);
    if (el) el.addEventListener(event, handler);
  }
  return { state, effect, text, show, model, on };
})();"#;

/// Generate JavaScript from the compilation context.
pub fn generate(ctx: &CompilationContext) -> Result<String, CodegenError> {
    // No state, no bindings → no JS needed
    if ctx.state_fields.is_empty() && ctx.bindings.is_empty() {
        return Ok(String::new());
    }

    let mut js = String::new();

    // Runtime
    js.push_str(RUNTIME);
    js.push_str("\n\n");

    // User code in IIFE
    js.push_str("(function() {\n");

    // State initialization
    if !ctx.state_fields.is_empty() {
        js.push_str("  const _s = hrml.state({ ");
        let fields: Vec<String> = ctx
            .state_fields
            .iter()
            .map(|(name, value)| format!("{name}: {value}"))
            .collect();
        js.push_str(&fields.join(", "));
        js.push_str(" });\n");
    }

    // Bindings
    for binding in &ctx.bindings {
        match binding {
            Binding::Text { id, template } => {
                js.push_str(&format!(
                    "  hrml.text('{id}', () => `{template}`);\n"
                ));
            }
            Binding::Event {
                id,
                event,
                handler,
                modifiers,
            } => {
                if modifiers.contains(&"prevent".to_string()) {
                    js.push_str(&format!(
                        "  hrml.on('{id}', '{event}', (e) => {{ e.preventDefault(); {handler}; }});\n"
                    ));
                } else {
                    js.push_str(&format!(
                        "  hrml.on('{id}', '{event}', () => {{ {handler}; }});\n"
                    ));
                }
            }
            Binding::Show { id, expr } => {
                js.push_str(&format!("  hrml.show('{id}', () => {expr});\n"));
            }
            Binding::Model { id, field } => {
                js.push_str(&format!("  hrml.model('{id}', _s, '{field}');\n"));
            }
        }
    }

    // Computed fields as effects
    for (name, expr) in &ctx.computed_fields {
        js.push_str(&format!(
            "  hrml.effect(() => {{ _s.{name} = {expr}; }});\n"
        ));
    }

    js.push_str("})();\n");

    Ok(js)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CompilationContext;

    // =========================================================================
    // Empty context
    // =========================================================================

    #[test]
    fn test_empty_context() {
        let ctx = CompilationContext::new();
        let js = generate(&ctx).unwrap();
        assert_eq!(js, "");
    }

    // =========================================================================
    // State initialization
    // =========================================================================

    #[test]
    fn test_state_single_field() {
        let mut ctx = CompilationContext::new();
        ctx.state_fields.push(("count".into(), "0".into()));
        let js = generate(&ctx).unwrap();
        assert!(js.contains("hrml.state({ count: 0 })"));
    }

    #[test]
    fn test_state_multiple_fields() {
        let mut ctx = CompilationContext::new();
        ctx.state_fields.push(("count".into(), "0".into()));
        ctx.state_fields
            .push(("loading".into(), "false".into()));
        let js = generate(&ctx).unwrap();
        assert!(js.contains("hrml.state({ count: 0, loading: false })"));
    }

    #[test]
    fn test_state_string_field() {
        let mut ctx = CompilationContext::new();
        ctx.state_fields.push(("name".into(), "''".into()));
        let js = generate(&ctx).unwrap();
        assert!(js.contains("name: ''"));
    }

    // =========================================================================
    // Bindings
    // =========================================================================

    #[test]
    fn test_text_binding() {
        let mut ctx = CompilationContext::new();
        ctx.state_fields.push(("count".into(), "0".into()));
        ctx.bindings.push(Binding::Text {
            id: "hrml-0".into(),
            template: "${_s.count}".into(),
        });
        let js = generate(&ctx).unwrap();
        assert!(js.contains("hrml.text('hrml-0', () => `${_s.count}`)"));
    }

    #[test]
    fn test_event_binding() {
        let mut ctx = CompilationContext::new();
        ctx.state_fields.push(("count".into(), "0".into()));
        ctx.bindings.push(Binding::Event {
            id: "hrml-0".into(),
            event: "click".into(),
            handler: "_s.count++".into(),
            modifiers: Vec::new(),
        });
        let js = generate(&ctx).unwrap();
        assert!(js.contains("hrml.on('hrml-0', 'click', () => { _s.count++; })"));
    }

    #[test]
    fn test_event_binding_with_prevent() {
        let mut ctx = CompilationContext::new();
        ctx.state_fields.push(("data".into(), "null".into()));
        ctx.bindings.push(Binding::Event {
            id: "hrml-0".into(),
            event: "submit".into(),
            handler: "save()".into(),
            modifiers: vec!["prevent".into()],
        });
        let js = generate(&ctx).unwrap();
        assert!(js.contains("e.preventDefault()"));
        assert!(js.contains("save()"));
    }

    #[test]
    fn test_show_binding() {
        let mut ctx = CompilationContext::new();
        ctx.state_fields
            .push(("visible".into(), "true".into()));
        ctx.bindings.push(Binding::Show {
            id: "hrml-0".into(),
            expr: "_s.visible".into(),
        });
        let js = generate(&ctx).unwrap();
        assert!(js.contains("hrml.show('hrml-0', () => _s.visible)"));
    }

    #[test]
    fn test_model_binding() {
        let mut ctx = CompilationContext::new();
        ctx.state_fields.push(("name".into(), "''".into()));
        ctx.bindings.push(Binding::Model {
            id: "hrml-0".into(),
            field: "name".into(),
        });
        let js = generate(&ctx).unwrap();
        assert!(js.contains("hrml.model('hrml-0', _s, 'name')"));
    }

    // =========================================================================
    // Runtime
    // =========================================================================

    #[test]
    fn test_runtime_included() {
        let mut ctx = CompilationContext::new();
        ctx.state_fields.push(("x".into(), "0".into()));
        let js = generate(&ctx).unwrap();
        assert!(js.contains("const hrml = (() => {"));
        assert!(js.contains("new Proxy("));
        assert!(js.contains("queueMicrotask"));
        assert!(js.contains("return { state, effect, text, show, model, on };"));
    }

    #[test]
    fn test_iife_wrapper() {
        let mut ctx = CompilationContext::new();
        ctx.state_fields.push(("x".into(), "0".into()));
        let js = generate(&ctx).unwrap();
        assert!(js.contains("(function() {"));
        assert!(js.contains("})();"));
    }

    // =========================================================================
    // Computed fields
    // =========================================================================

    #[test]
    fn test_computed_field() {
        let mut ctx = CompilationContext::new();
        ctx.state_fields.push(("count".into(), "0".into()));
        ctx.computed_fields
            .push(("double".into(), "_s.count * 2".into()));
        let js = generate(&ctx).unwrap();
        assert!(js.contains("hrml.effect(() => { _s.double = _s.count * 2; })"));
    }

    // =========================================================================
    // Full examples via compile()
    // =========================================================================

    fn parse(source: &str) -> hrml_parser::ast::Document {
        hrml_parser::Parser::parse(source).unwrap()
    }

    #[test]
    fn test_counter_js() {
        let doc = parse(
            "state\n  count: 0\n\ndiv .counter\n  button @click=\"count++\" \"-\"\n  span \"{count}\"\n  button @click=\"count--\" \"+\"",
        );
        let output = crate::compile(&doc).unwrap();
        assert!(output.js.contains("hrml.state({ count: 0 })"));
        assert!(output.js.contains("_s.count++"));
        assert!(output.js.contains("_s.count--"));
        assert!(output.js.contains("${_s.count}"));
    }

    #[test]
    fn test_toggle_js() {
        let doc = parse(
            "state\n  visible: true\n\nbutton @click=\"visible = !visible\" \"Toggle\"\ndiv :show=\"visible\" \"Content\"",
        );
        let output = crate::compile(&doc).unwrap();
        assert!(output.js.contains("hrml.state({ visible: true })"));
        assert!(output.js.contains("_s.visible = !_s.visible"));
        assert!(output.js.contains("hrml.show("));
    }

    #[test]
    fn test_input_binding_js() {
        let doc = parse(
            "state\n  name: \"\"\n\ninput :model=\"name\" placeholder=\"Type your name\"\nspan \"Hello {name}!\"",
        );
        let output = crate::compile(&doc).unwrap();
        assert!(output.js.contains("hrml.state({ name: '' })"));
        assert!(output.js.contains("hrml.model("));
        assert!(output.js.contains("Hello ${_s.name}!"));
    }
}
