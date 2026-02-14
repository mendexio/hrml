# Getting Started with HRML

HRML (Hypertext Reactive Markup Language) is a compiled web language that brings reactive programming to HTML. This guide will help you get started in minutes.

## What is HRML?

HRML is a language that compiles to HTML + CSS + JavaScript. It uses three symbols:

- `:` for state and directives
- `@` for events
- `$` for server communication

Instead of learning a framework, you write HRML and the compiler generates optimized, reactive code.

## Quick Start (No Install)

The fastest way to try HRML is the [browser playground](https://hrml.dev/playground):

1. Open [hrml.dev/playground](https://hrml.dev/playground)
2. Write HRML on the left
3. See compiled HTML + JS on the right
4. Preview the result instantly

Try this example:

```hrml
state
  count: 0

div .counter
  button @click="count--" "-"
  span "{count}"
  button @click="count++" "+"
```

Click "Compile" and see it work.

## Installation

### Option 1: CLI (for development)

Install via Cargo:

```bash
cargo install hrml-cli
```

Verify installation:

```bash
hrml --version
# hrml 0.7.0
```

### Option 2: WASM (for browser compilation)

HRML compiles to WebAssembly and runs in the browser. The playground uses this approach - no backend needed.

To use WASM in your own project:

```html
<script type="module">
  import init, { compile } from './hrml_wasm.js';

  await init();
  const result = compile('div "Hello"');
  console.log(result.html); // <div>Hello</div>
</script>
```

## Your First HRML File

Create `hello.hrml`:

```hrml
state
  name: "World"

div .greeting
  h1 "Hello, {name}!"
  input :model="name" placeholder="Enter your name"
  p "You entered: {name}"
```

This creates:
- Reactive state (`name`)
- String interpolation (`{name}`)
- Two-way binding (`:model`)

## Compile Your File

Using the CLI:

```bash
# Compile to HTML + JS
hrml build hello.hrml

# This creates:
# - hello.html
# - hello.js (if state/events present)
```

Open `hello.html` in a browser - it works standalone, no server needed.

## Understanding the Output

HRML compiles your file to:

**HTML** - Clean, semantic markup:
```html
<div id="hrml-0" class="greeting">
  <h1 id="hrml-1"></h1>
  <input id="hrml-2">
  <p id="hrml-3"></p>
</div>
```

**JavaScript** - Reactive runtime (~200 lines, zero dependencies):
```js
const state = new Proxy({ name: "World" }, /* reactive handler */);
// Updates DOM when state.name changes
```

**CSS** - (In development - currently manual or use Tailwind)

## Core Concepts

### 1. State

Reactive variables that update the DOM automatically:

```hrml
state
  count: 0
  visible: true
  items: []
```

### 2. Events

Handle user interactions:

```hrml
button @click="count++"
input @input="handleInput"
form @submit.prevent="save"
```

### 3. Directives

Control rendering and binding:

```hrml
div :show="visible"        # Conditional display
input :model="username"     # Two-way binding
div :text="message"         # Text content
```

### 4. String Interpolation

Embed reactive values in text:

```hrml
p "Count: {count}"
h1 "Hello, {firstName} {lastName}!"
```

## Examples

### Counter

```hrml
state
  count: 0

div .counter
  button @click="count--" "-"
  span "{count}"
  button @click="count++" "+"
```

### Toggle

```hrml
state
  visible: true

div
  button @click="visible = !visible" "Toggle"
  p :show="visible" "Now you see me"
```

### Form Input

```hrml
state
  username: ""

div
  input :model="username" placeholder="Enter username"
  p "Username: {username}"
  p :show="username.length < 3" "Too short"
```

## Project Structure

A typical HRML project:

```
my-app/
├── src/
│   ├── index.hrml          # Main page
│   ├── components/         # Reusable components (future)
│   └── styles/             # CSS files
├── dist/                   # Compiled output
└── hrml.toml               # Config (future)
```

## CLI Commands

```bash
# Compile a file
hrml build input.hrml

# Check for errors without compiling
hrml check input.hrml

# Watch mode (future)
hrml watch src/

# Create new project (future)
hrml new my-app
```

## Current Limitations

HRML is in early development (v0.7.0). What works:

✅ State declarations
✅ String interpolation
✅ Event handlers (`@click`, `@input`)
✅ Basic directives (`:show`, `:model`)
✅ WASM compilation

Coming soon:

🚧 Computed properties
🚧 Conditional rendering (`:if` / `:else`)
🚧 List rendering (`:each`)
🚧 Server communication (`$get`, `$post`)
🚧 CSS generation
🚧 Headless components

See the [roadmap](../README.md#roadmap) for details.

## Next Steps

1. **Try the playground** - [hrml.dev/playground](https://hrml.dev/playground)
2. **Read the three symbols guide** - [three-symbols.md](three-symbols.md) (coming soon)
3. **Explore examples** - See `examples/` folder
4. **Join the conversation** - Open issues, ask questions
5. **Contribute** - See [CONTRIBUTING.md](../CONTRIBUTING.md)

## Getting Help

- **Playground** - [hrml.dev/playground](https://hrml.dev/playground)
- **Issues** - [GitHub Issues](https://github.com/mendexio/hrml/issues)
- **README** - [Main README](../README.md)

## Philosophy

HRML is designed to:

- **Compile, don't interpret** - Pre-compile for performance
- **Zero runtime dependencies** - Self-contained output
- **AI-friendly syntax** - Three-symbol pattern is learnable
- **Disappear eventually** - When browsers support native reactivity (TC39 Signals), HRML becomes unnecessary

Welcome to HRML. Start simple, build up, and enjoy reactive HTML.
