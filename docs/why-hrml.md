# Why HRML?

HRML exists because there's a gap in web development - a space between simple HTML and complex frameworks that nothing currently fills well.

This document explains the problem, why existing tools fall short, and how HRML is different.

## The Problem

**Building reactive web UIs requires choosing between extremes:**

### Option 1: Plain HTML + JavaScript

Simple but verbose and manual:

```html
<div id="counter">
  <button id="dec">-</button>
  <span id="display">0</span>
  <button id="inc">+</button>
</div>

<script>
  let count = 0;
  const display = document.getElementById('display');

  document.getElementById('inc').addEventListener('click', () => {
    count++;
    display.textContent = count;
  });

  document.getElementById('dec').addEventListener('click', () => {
    count--;
    display.textContent = count;
  });
</script>
```

**Problems:**
- Repetitive DOM manipulation
- Manual state-to-DOM synchronization
- Brittle (IDs can break, event listeners can leak)
- Scales poorly

### Option 2: React/Vue/Svelte

Powerful but heavyweight:

```jsx
function Counter() {
  const [count, setCount] = useState(0);

  return (
    <div className="counter">
      <button onClick={() => setCount(count - 1)}>-</button>
      <span>{count}</span>
      <button onClick={() => setCount(count + 1)}>+</button>
    </div>
  );
}
```

**Problems:**
- Build step required (webpack, vite, etc.)
- Runtime overhead (React: ~140KB, Vue: ~80KB)
- Learning curve (JSX, hooks, components, lifecycle)
- Overkill for simple interactions
- Framework lock-in

### Option 3: HTMX

Server-driven, minimal JS:

```html
<div hx-get="/counter" hx-trigger="load">
  <button hx-post="/counter/dec">-</button>
  <span>0</span>
  <button hx-post="/counter/inc">+</button>
</div>
```

**Problems:**
- Requires server for every interaction
- No client-side state management
- Not suitable for rich client-side UIs
- Performance bottleneck for real-time updates

### Option 4: Alpine.js

Lightweight reactivity in HTML:

```html
<div x-data="{ count: 0 }" class="counter">
  <button @click="count--">-</button>
  <span x-text="count"></span>
  <button @click="count++">+</button>
</div>
```

**Problems:**
- Attribute soup (mixes logic and markup)
- No compilation (runtime interpretation)
- Limited to simple use cases
- Inline expressions get messy fast

## The Gap

There's a missing tool:

- **Compiled** (like React) but **standalone** (like Alpine)
- **Reactive** (like Vue) but **zero runtime** (like Svelte)
- **Simple syntax** (like HTMX) but **client-side** (no server needed)
- **File-based** (not inline) but **framework-free**

HRML fills this gap.

## How HRML is Different

### Compiled to Standalone Code

HRML compiles `.hrml` files to HTML + JS that run anywhere:

```hrml
state
  count: 0

div .counter
  button @click="count--" "-"
  span "{count}"
  button @click="count++" "+"
```

**Output:**
- `counter.html` - Standard HTML
- Embedded JS runtime (~200 lines, no dependencies)
- Works in any browser, no framework needed

### Three Symbols, Infinite Clarity

Instead of dozens of APIs, learn three prefixes:

- `:` - State and directives
- `@` - Events
- `$` - Server (planned)

Memorize the pattern once, apply everywhere.

### Zero Runtime Dependencies

The generated code has **zero npm packages**. No `node_modules`, no build tools in production, no framework to load.

The reactive runtime is ~200 lines of vanilla JavaScript embedded in your output. That's it.

### Indentation-Based Syntax

No closing tags, no angle brackets everywhere:

```hrml
# HRML
div .container
  h1 "Welcome"
  p "This is clean"

# vs HTML
<div class="container">
  <h1>Welcome</h1>
  <p>This is clean</p>
</div>
```

Less noise, more focus on structure.

### Files, Not Inline

HRML is file-based, not attribute-based:

```hrml
# Everything in one .hrml file
state
  users: []

div
  h1 "Users"
  div :each="user in users"
    p "{user.name}"
```

No mixing of logic in HTML attributes. Clean separation.

## HRML vs The Alternatives

### vs React/Vue/Angular

| Aspect | React/Vue/Angular | HRML |
|--------|-------------------|------|
| Build required | Yes (webpack, vite) | Yes (hrml compiler) |
| Runtime size | 80-140KB | ~3KB (embedded) |
| Dependencies | Many | Zero |
| Learning curve | High | Low (3 symbols) |
| Use case | Full apps | Pages + interactions |

**When to use React:** Building a complex SPA with routing, state management, and many components.

**When to use HRML:** Adding reactivity to server-rendered pages or building small interactive UIs.

### vs HTMX

| Aspect | HTMX | HRML |
|--------|------|------|
| Approach | Server-driven | Client-driven |
| State location | Server | Client |
| Network required | Yes | No |
| Real-time updates | Polling/SSE | Native |
| Build step | No | Yes |

**When to use HTMX:** Server-rendered apps where state lives on the backend.

**When to use HRML:** Client-side state and interactions without server round-trips.

### vs Alpine.js

| Aspect | Alpine.js | HRML |
|--------|-----------|------|
| Compilation | No (runtime) | Yes (build-time) |
| Syntax | Inline attributes | Separate files |
| Performance | Interpreted | Compiled |
| Complexity limit | Simple | Medium |

**When to use Alpine:** Quick inline interactions without a build step.

**When to use HRML:** Structured, maintainable reactive UIs with compilation.

### vs Plain JavaScript

| Aspect | Plain JS | HRML |
|--------|----------|------|
| Reactivity | Manual | Automatic |
| Boilerplate | High | Low |
| Learning curve | Low | Low |
| Maintainability | Poor | Good |

**When to use Plain JS:** Tiny scripts with no state management needs.

**When to use HRML:** Any UI with reactive state.

## Who HRML is For

HRML is designed for:

- **Backend developers** who need client-side reactivity without learning React
- **Teams** building server-rendered apps with interactive components
- **Solo developers** who want simple, maintainable reactive UIs
- **Projects** where bundle size and dependencies matter
- **Situations** where you control the build step but not the runtime

## Who HRML is NOT For

HRML is **not** ideal for:

- Large SPAs with complex routing (use React/Vue)
- Projects that need a massive ecosystem (use React)
- Teams already invested in a framework (stay with it)
- No-build-step requirements (use Alpine/HTMX)
- Mobile apps (use React Native/Flutter)

## The Philosophy

HRML believes:

1. **Compilation beats interpretation** - Pre-compile for performance and size
2. **Three symbols beat hundreds** - Simplicity scales better than features
3. **Zero dependencies is a feature** - Less to break, less to maintain
4. **Files beat attributes** - Separation of concerns matters
5. **Reactive should be default** - Don't fight the DOM, make it reactive

## The Long-Term Vision

HRML is **designed to disappear**.

When browsers natively support reactivity (TC39 Signals proposal), HRML's compilation target will change from "HTML + runtime" to "HTML + native signals".

Eventually, HRML becomes a syntax layer over native browser features. No runtime, no framework, just HTML that happens to be reactive.

## Try HRML If...

- You're tired of `npm install` for simple interactions
- You want reactivity without a framework
- You're building server-rendered pages with interactive parts
- You value small, dependency-free output
- You like the idea of compiled reactive HTML

## Don't Try HRML If...

- You need a mature ecosystem **now**
- You're building a complex SPA
- Your team is happy with React/Vue
- You can't have a build step
- You need mobile/desktop/native support

## Next Steps

1. **Try the playground** - [hrml.dev/playground](https://hrml.dev/playground)
2. **Read getting started** - [getting-started.md](getting-started.md)
3. **Learn the symbols** - [three-symbols.md](three-symbols.md)
4. **See examples** - `examples/` folder

HRML isn't trying to replace React. It's filling a gap - reactive HTML for the rest of us.
