# HRML vs HTMX vs Alpine.js

A side-by-side comparison of three lightweight approaches to web interactivity.

## Quick Comparison

| Feature | HRML | HTMX | Alpine.js |
|---------|------|------|-----------|
| **Approach** | Compiled language | HTML attributes | Inline directives |
| **Build step** | Required | None | None |
| **Runtime size** | ~3KB embedded | ~14KB | ~15KB |
| **Client-side state** | Yes (reactive) | No | Yes (reactive) |
| **Server communication** | Planned (`$`) | Core feature | Manual (fetch) |
| **Learning curve** | Low (3 symbols) | Low (attributes) | Low (Alpine syntax) |
| **Compilation** | Pre-compiled | Runtime | Runtime |
| **Use case** | Reactive UIs | Server-driven | Sprinkles of JS |

## Example 1: Counter

### HRML

```hrml
state
  count: 0

div .counter
  button @click="count--" "-"
  span "{count}"
  button @click="count++" "+"
```

**Compiled to:** Standalone HTML + JS with embedded reactive runtime (~200 lines).

**Pros:**
- Clean, minimal syntax
- Type-safe compilation
- Zero runtime dependencies

**Cons:**
- Requires build step
- Not suitable for quick prototypes

### HTMX

```html
<div hx-ext="json-enc">
  <button hx-post="/counter/decrement">-</button>
  <span id="count">0</span>
  <button hx-post="/counter/increment">+</button>
</div>
```

**Requires:** Server endpoint that returns updated HTML.

**Pros:**
- No build step
- Server controls state
- Progressive enhancement

**Cons:**
- Requires server for every interaction
- Network latency for each click
- More server logic needed

### Alpine.js

```html
<div x-data="{ count: 0 }">
  <button @click="count--">-</button>
  <span x-text="count"></span>
  <button @click="count++">+</button>
</div>
```

**Pros:**
- No build step
- Inline and easy to read
- Client-side state

**Cons:**
- Runtime interpretation (slower than compiled)
- Attribute soup for complex UIs
- Limited to simple interactions

## Example 2: Toggle Visibility

### HRML

```hrml
state
  visible: true

button @click="visible = !visible" "Toggle"
div :show="visible" "Content"
```

### HTMX

```html
<button hx-get="/toggle" hx-target="#content">Toggle</button>
<div id="content">Content</div>
```

Server must track state and return appropriate HTML.

### Alpine.js

```html
<div x-data="{ visible: true }">
  <button @click="visible = !visible">Toggle</button>
  <div x-show="visible">Content</div>
</div>
```

## Example 3: Form Input Binding

### HRML

```hrml
state
  name: ""

input :model="name" placeholder="Your name"
p "Hello, {name}!"
```

### HTMX

```html
<input name="name"
       hx-post="/echo"
       hx-trigger="keyup changed delay:500ms"
       hx-target="#greeting">
<p id="greeting">Hello, !</p>
```

### Alpine.js

```html
<div x-data="{ name: '' }">
  <input x-model="name" placeholder="Your name">
  <p>Hello, <span x-text="name"></span>!</p>
</div>
```

## Example 4: Conditional Rendering

### HRML

```hrml
state
  role: "user"

div :show="role === 'admin'" "Admin Panel"
div :show="role === 'user'" "User Dashboard"
```

### HTMX

Not directly supported - requires server logic:

```html
<div hx-get="/content" hx-trigger="load"></div>
```

Server returns appropriate content based on role.

### Alpine.js

```html
<div x-data="{ role: 'user' }">
  <div x-show="role === 'admin'">Admin Panel</div>
  <div x-show="role === 'user'">User Dashboard</div>
</div>
```

## Example 5: Server Communication

### HRML (Planned)

```hrml
state
  users: null
  loading: false

div $get="/api/users" $data="users" $loading="loading"
  p :show="loading" "Loading..."
  div :each="user in users"
    p "{user.name}"
```

**Note:** `$` prefix is planned for v1.0.

### HTMX

```html
<div hx-get="/api/users" hx-trigger="load" hx-indicator="#spinner">
  <div id="spinner" class="htmx-indicator">Loading...</div>
  <div id="users"></div>
</div>
```

Server returns rendered HTML with user list.

### Alpine.js

```html
<div x-data="{ users: [], loading: false }" x-init="loading = true; fetch('/api/users').then(r => r.json()).then(data => { users = data; loading = false; })">
  <p x-show="loading">Loading...</p>
  <template x-for="user in users">
    <p x-text="user.name"></p>
  </template>
</div>
```

## When to Use Each

### Use HRML when:

- ✅ Building reactive client-side UIs
- ✅ You control the build step
- ✅ You want zero runtime dependencies
- ✅ You value clean, minimal syntax
- ✅ You're adding interactivity to server-rendered pages

### Use HTMX when:

- ✅ State lives on the server
- ✅ You can't have a build step
- ✅ You're building a traditional server-rendered app
- ✅ You want progressive enhancement
- ✅ Network latency isn't a concern for interactions

### Use Alpine.js when:

- ✅ You need quick inline interactions
- ✅ No build step is required
- ✅ You're sprinkling JavaScript into static HTML
- ✅ Interactions are simple and localized
- ✅ You don't need server communication patterns

## Performance Comparison

### Initial Load

| Library | Size (gzipped) | Parse time |
|---------|----------------|------------|
| HRML | ~3KB (embedded) | Fast (pre-compiled) |
| HTMX | ~14KB | Medium (runtime init) |
| Alpine.js | ~15KB | Medium (runtime init) |

### Runtime Performance

| Operation | HRML | HTMX | Alpine.js |
|-----------|------|------|-----------|
| State update | Fast (Proxy) | N/A (server) | Fast (Proxy) |
| DOM update | Fast (direct) | Medium (network) | Fast (direct) |
| First interaction | Instant | Network latency | Instant |

## Bundle Size Over Time

As your app grows:

- **HRML:** Size stays minimal - only generates code you use
- **HTMX:** Fixed ~14KB regardless of features used
- **Alpine.js:** Fixed ~15KB regardless of features used

## Compilation vs Runtime

### HRML (Compiled)

```
.hrml → Compiler → Optimized HTML + JS
```

**Benefits:**
- Type checking at compile time
- Dead code elimination
- Optimized output
- No runtime parsing

**Drawbacks:**
- Build step required
- Can't modify at runtime

### HTMX & Alpine.js (Runtime)

```
HTML → Browser → Runtime interpretation
```

**Benefits:**
- No build step
- Can modify via DevTools
- Quick prototyping

**Drawbacks:**
- Runtime overhead
- Larger bundle (entire library shipped)
- No compile-time checks

## Developer Experience

### HRML

**Pros:**
- Familiar indentation-based syntax
- Clear error messages at compile time
- Three-symbol pattern is easy to learn
- Generated code is readable

**Cons:**
- Requires learning new syntax
- Build step adds complexity
- Smaller ecosystem (new project)

### HTMX

**Pros:**
- Zero learning curve (just HTML)
- Works with any backend
- Extensive documentation
- Mature ecosystem

**Cons:**
- Attribute soup in complex UIs
- Server must handle all state
- Debugging requires server logs

### Alpine.js

**Pros:**
- Vue-like syntax (familiar to many)
- Excellent documentation
- Large community
- Easy to debug in browser

**Cons:**
- Can become messy in large files
- Limited to simple use cases
- Inline logic harder to test

## Migration Path

### From Alpine.js to HRML

Alpine.js and HRML share similar reactive patterns. Migration is straightforward:

```html
<!-- Alpine.js -->
<div x-data="{ count: 0 }">
  <button @click="count++">Increment</button>
  <span x-text="count"></span>
</div>
```

```hrml
# HRML
state
  count: 0

div
  button @click="count++" "Increment"
  span "{count}"
```

### From HTMX to HRML

Requires moving state from server to client:

```html
<!-- HTMX -->
<button hx-post="/increment" hx-target="#count">+</button>
<span id="count">0</span>
```

```hrml
# HRML
state
  count: 0

button @click="count++" "+"
span "{count}"
```

## Hybrid Approaches

You can combine these tools:

### HRML + HTMX

Use HRML for client-side state, HTMX for server updates:

```hrml
state
  filter: "all"

div
  button @click="filter = 'all'" "All"
  button @click="filter = 'active'" "Active"

  div hx-get="/todos?filter={filter}" hx-trigger="load"
    # Server returns filtered todos
```

### Alpine.js + HTMX

Alpine for local state, HTMX for server communication:

```html
<div x-data="{ filter: 'all' }">
  <button @click="filter = 'all'">All</button>
  <button @click="filter = 'active'">Active</button>

  <div hx-get="/todos" hx-vals='{ "filter": filter }'>
    <!-- Server returns filtered content -->
  </div>
</div>
```

## Conclusion

**Choose based on your constraints:**

- **Build step acceptable?** → HRML or framework (React/Vue)
- **No build step?** → HTMX or Alpine.js
- **State on server?** → HTMX
- **State on client?** → HRML or Alpine.js
- **Complex SPA?** → React/Vue (not these three)

All three tools solve real problems. Pick the one that matches your project's constraints and your team's preferences.

## Further Reading

- [HRML Documentation](../README.md)
- [HTMX Documentation](https://htmx.org/)
- [Alpine.js Documentation](https://alpinejs.dev/)
