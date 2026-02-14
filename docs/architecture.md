# Architecture

HRML is a compiled language. This document explains how the compiler works, from source code to running HTML.

## High-Level Overview

```
.hrml source → Compiler (Rust) → HTML + CSS + JS
                  ↓
            [Lexer → Parser → Codegen]
                  ↓
            Standalone output (no dependencies)
```

The compiler has two delivery modes:

1. **CLI** - Command-line tool for local development
2. **WASM** - Browser-based compilation for the playground

Both use the same core compiler pipeline.

## Compiler Pipeline

### Stage 1: Lexer (Tokenization)

**Crate:** `hrml-lexer`

**Input:** Raw `.hrml` source string

**Output:** Stream of tokens

**Responsibility:**
- Break source into tokens (keywords, identifiers, strings, etc.)
- Track indentation levels
- Handle two scanning modes:
  - **Html mode** - Braces `{}` mean interpolation
  - **Expression mode** - Braces `{}` are syntax

**Example:**

```hrml
state
  count: 0

div .counter
  span "{count}"
```

Becomes:

```
[Keyword(State), Newline, Indent,
 Identifier("count"), Colon, Number(0), Dedent,
 Identifier("div"), Class("counter"), Newline, Indent,
 Identifier("span"), String("{count}"), Dedent, EOF]
```

**Key decisions:**
- Token types carry data: `Identifier(String)`, not separate value field
- Indentation tracking with a stack: push on indent, pop on dedent
- Mode switching: entering `{}` in HTML mode switches to expression mode

### Stage 2: Parser (AST Construction)

**Crate:** `hrml-parser`

**Input:** Token stream from lexer

**Output:** Abstract Syntax Tree (AST)

**Responsibility:**
- Build a tree structure from flat tokens
- Validate syntax (e.g., matching indentation)
- Parse expressions (event handlers, interpolation, etc.)
- Create semantic nodes: `Document`, `Element`, `StateDeclaration`

**Two parsers:**

1. **Document Parser** - Top-level structure (state blocks, elements)
2. **Expression Parser** - JavaScript-like expressions (Pratt parsing)

**Example:**

```hrml
state
  count: 0

div @click="count++"
```

Becomes:

```
Document {
  state: StateDeclaration {
    fields: [("count", Number(0))]
  },
  elements: [
    Element {
      tag: "div",
      attributes: [
        Event("click", BinaryOp(Identifier("count"), "++"))
      ]
    }
  ]
}
```

**Key decisions:**
- Recursive descent for document structure
- Pratt parser for expressions (handles operator precedence)
- String interpolation parsed but not evaluated (stays as AST)

### Stage 3: Codegen (Code Generation)

**Crate:** `hrml-codegen`

**Input:** AST from parser

**Output:** Three strings: HTML, CSS, JS

**Responsibility:**
- Generate clean HTML with unique IDs
- Generate CSS (currently minimal, planned for future)
- Generate JavaScript runtime + reactive logic
- Embed runtime code in output

**Three generators:**

1. **HTML Generator** - Emits `<div id="hrml-0">...</div>`
2. **CSS Generator** - (Planned) Utility-first CSS with tree-shaking
3. **JS Generator** - Reactive runtime + event handlers + state updates

**Example:**

Input AST:
```
Element { tag: "div", classes: ["counter"] }
```

Output HTML:
```html
<div id="hrml-0" class="counter"></div>
```

Output JS:
```js
const state = new Proxy({ count: 0 }, {
  set(target, prop, value) {
    target[prop] = value;
    render(); // Update DOM
    return true;
  }
});

document.getElementById('hrml-1').addEventListener('click', () => {
  state.count++;
});
```

**Key decisions:**
- IDs not data-attributes (CSP-safe, no eval)
- Runtime embedded in output (~200 lines, zero deps)
- Proxy-based reactivity (native browser feature)

## Crate Structure

The compiler is a Rust workspace with 5 crates:

```
hrml/
├── crates/
│   ├── hrml-lexer/      # Tokenization
│   ├── hrml-parser/     # AST construction
│   ├── hrml-codegen/    # HTML + CSS + JS generation
│   ├── hrml-wasm/       # WebAssembly bindings
│   └── hrml-cli/        # Command-line interface
```

### hrml-lexer

**Purpose:** Tokenization with indentation tracking

**Public API:**
```rust
pub fn tokenize(source: &str) -> Result<Vec<Token>, LexerError>;
```

**Tests:** 85 tests covering:
- Basic tokens
- Indentation handling
- String interpolation
- Mode switching

### hrml-parser

**Purpose:** Parse tokens into AST

**Public API:**
```rust
pub fn parse(tokens: Vec<Token>) -> Result<Document, ParseError>;
```

**Two sub-parsers:**
- `DocumentParser` - Top-level structure
- `ExpressionParser` - JavaScript-like expressions

**Tests:** 78 tests covering:
- Element parsing
- State declarations
- Nested structures
- Expression parsing

### hrml-codegen

**Purpose:** Generate HTML + CSS + JS from AST

**Public API:**
```rust
pub fn generate(ast: Document) -> Result<CompilerOutput, CodegenError>;

pub struct CompilerOutput {
    pub html: String,
    pub css: String,
    pub js: String,
}
```

**Three generators:**
- `HtmlGenerator` - Emits HTML
- `CssGenerator` - (Planned) Emits CSS
- `JsGenerator` - Emits reactive runtime + logic

**Tests:** 61 tests covering:
- HTML generation
- JS reactive runtime
- Event handler generation

### hrml-wasm

**Purpose:** WebAssembly bindings for browser compilation

**Public API (JavaScript):**
```js
import init, { compile } from './hrml_wasm.js';

await init();
const result = compile(source);
// { html: "...", css: "...", js: "..." }
```

**Built with:**
- `wasm-bindgen` - Rust ↔ JS interop
- `wasm-pack` - Build tooling

**Size:** 68KB gzipped (including compiler!)

**Tests:** 9 tests covering:
- Compilation via WASM
- Error handling
- Type safety

### hrml-cli

**Purpose:** Command-line interface

**Commands:**
```bash
hrml build input.hrml    # Compile to HTML + JS
hrml check input.hrml    # Check for errors
```

**Tests:** Integration tests for CLI behavior

## WASM Architecture

The browser playground uses WASM compilation:

```
User types HRML
    ↓
JavaScript calls compile()
    ↓
WASM module (Rust compiler)
    ↓
Returns { html, css, js }
    ↓
Display in preview pane
```

**Benefits:**
- No server needed - compilation happens client-side
- Same compiler as CLI - consistency
- Fast - Rust compiled to WASM is faster than JS

**Build process:**
```bash
cd crates/hrml-wasm
wasm-pack build --target web --release
# Output: pkg/hrml_wasm.js (68KB gzipped)
```

## Runtime Architecture

The generated JavaScript runtime has three parts:

### 1. Reactive State (Proxy)

```js
const state = new Proxy(initialState, {
  set(target, prop, value) {
    target[prop] = value;
    updateDOM(prop, value);
    return true;
  }
});
```

**How it works:**
- Every state assignment triggers the `set` trap
- Trap updates the DOM for that property
- No manual `setState()` or `ref()` needed

### 2. Element Registry

```js
const elements = {
  'hrml-0': document.getElementById('hrml-0'),
  'hrml-1': document.getElementById('hrml-1'),
  // ...
};
```

**Why:**
- Cache DOM queries (performance)
- Unique IDs avoid collisions
- CSP-safe (no `eval` or `new Function`)

### 3. Event Handlers

```js
elements['hrml-1'].addEventListener('click', () => {
  state.count++;
});
```

**Compiled from:**
```hrml
button @click="count++"
```

## Compilation Flow Example

**Input:**
```hrml
state
  count: 0

div .counter
  button @click="count--" "-"
  span "{count}"
  button @click="count++" "+"
```

**After Lexer:**
```
[Keyword(State), Indent, Identifier("count"), Colon, Number(0), ...]
```

**After Parser:**
```rust
Document {
  state: StateDeclaration {
    fields: [("count", Literal(0))]
  },
  elements: [
    Element {
      tag: "div",
      classes: ["counter"],
      children: [...]
    }
  ]
}
```

**After Codegen:**

HTML:
```html
<div id="hrml-0" class="counter">
  <button id="hrml-1">-</button>
  <span id="hrml-2"></span>
  <button id="hrml-3">+</button>
</div>
```

JS:
```js
const state = new Proxy({ count: 0 }, { /* ... */ });
const elements = { /* cached refs */ };

elements['hrml-1'].addEventListener('click', () => state.count--);
elements['hrml-3'].addEventListener('click', () => state.count++);

function updateDOM() {
  elements['hrml-2'].textContent = state.count;
}
```

## Future Plans

### CSS Generation

Currently manual. Planned:

```hrml
div .flex .items-center .gap-4
```

Generates:
```css
.flex { display: flex; }
.items-center { align-items: center; }
.gap-4 { gap: 1rem; }
```

**Approach:**
- Utility-first (Tailwind-style)
- Tree-shaking (only emit used classes)
- Scoped styles option

### Computed Properties

```hrml
state
  firstName: "John"
  lastName: "Doe"

computed
  fullName: firstName + " " + lastName
```

Compiles to:
```js
Object.defineProperty(state, 'fullName', {
  get() { return state.firstName + " " + state.lastName; }
});
```

### Server Communication

```hrml
div $get="/api/users" $data="users"
```

Compiles to:
```js
fetch('/api/users')
  .then(r => r.json())
  .then(data => state.users = data);
```

### Component System

Planned headless components with ARIA:

```hrml
modal :show="showModal"
  h2 "Title"
  p "Content"
```

Generates accessible modal with proper ARIA attributes.

## Design Principles

1. **Compilation over Interpretation** - Pre-compile for performance
2. **Zero Runtime Dependencies** - Embed everything needed
3. **Clean Output** - Generated code should be readable
4. **CSP Safe** - No `eval`, no `new Function`, no inline scripts
5. **Progressive Enhancement** - HTML works without JS (where possible)

## Testing Strategy

- **Unit tests** - Every crate has extensive tests
- **Integration tests** - CLI behavior, end-to-end compilation
- **Example-driven** - Examples in `examples/` are tested

**Current stats:**
- 264 total tests
- Zero warnings (enforced by CI)
- Zero clippy lints

## Build Process

**Development:**
```bash
cargo build          # Build CLI
cargo test           # Run tests
cargo clippy         # Lint
```

**WASM:**
```bash
cd crates/hrml-wasm
wasm-pack build --target web --release
```

**Release:**
```bash
cargo build --release
# Binary: target/release/hrml
```

## Performance Characteristics

- **Compilation:** ~5ms for typical file (Rust is fast)
- **Runtime overhead:** ~3KB gzipped (just the runtime code)
- **DOM updates:** Proxy-based, near-native speed
- **WASM size:** 68KB gzipped (includes entire compiler!)

## Next Steps

- Read [getting-started.md](getting-started.md) for usage
- Explore `crates/` source code
- Check [CONTRIBUTING.md](../CONTRIBUTING.md) for development setup
- See the [roadmap](../README.md#roadmap) for what's next
