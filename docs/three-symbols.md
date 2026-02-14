# The Three Symbols

HRML uses three symbols to replace the complexity of modern web frameworks. Instead of learning hundreds of APIs, you learn three prefixes.

```
:  State and directives
@  Events
$  Server communication
```

This guide explains each symbol in depth with examples.

## Why Three Symbols?

Modern frameworks have scattered APIs:

- React: `useState`, `useEffect`, `onClick`, `className`, `value`, `onChange`
- Vue: `v-if`, `v-for`, `v-model`, `@click`, `:class`, `ref`
- Alpine: `x-data`, `x-show`, `x-on:click`, `x-bind:class`

HRML consolidates everything into three patterns:

- `:` for anything related to state
- `@` for anything triggered by user action
- `$` for anything involving the server

Learn the pattern once, apply it everywhere.

## Symbol 1: `:` (State & Directives)

The colon (`:`) represents **state-driven behavior** - anything that reacts to data changes.

### State Declarations

Define reactive variables:

```hrml
state
  count: 0
  username: ""
  visible: true
  items: []
```

State is reactive - when values change, the DOM updates automatically.

### `:show` - Conditional Display

Show or hide elements based on state:

```hrml
state
  loggedIn: false

div :show="loggedIn" "Welcome back!"
div :show="!loggedIn" "Please log in"
```

Compiles to `display: none` toggling. The element stays in the DOM.

**Use when:**
- Toggling visibility frequently
- Element needs to maintain state when hidden
- Simple show/hide logic

### `:if` - Conditional Rendering

Render elements only when condition is true:

```hrml
state
  role: "admin"

div :if="role === 'admin'" "Admin Panel"
div :if="role === 'user'" "User Dashboard"
```

Unlike `:show`, `:if` removes the element from the DOM entirely.

**Use when:**
- Element is expensive to render
- Security-sensitive content
- Mutually exclusive branches

### `:model` - Two-Way Binding

Bind input value to state bidirectionally:

```hrml
state
  username: ""

input :model="username" placeholder="Enter username"
p "You typed: {username}"
```

Changes to input update `username`. Changes to `username` update input.

**Works with:**
- `<input type="text">`
- `<input type="number">`
- `<textarea>`
- `<select>`

### `:class` - Dynamic Classes

Apply classes conditionally:

```hrml
state
  active: false
  count: 0

button :class="{ active: active, warning: count > 10 }" "Click"
```

Compiles to:
- `active` class when `active === true`
- `warning` class when `count > 10`

**Syntax:**
```hrml
# Object syntax
div :class="{ active: isActive, error: hasError }"

# String expression
div :class="isActive ? 'active' : 'inactive'"
```

### `:text` - Dynamic Text

Set text content from state:

```hrml
state
  message: "Hello"

p :text="message"
```

Equivalent to `<p>{message}</p>` but useful for dynamic content that might include HTML.

### `:html` - Render HTML

Insert HTML from state (use with caution):

```hrml
state
  content: "<strong>Bold text</strong>"

div :html="content"
```

**Warning:** Only use with trusted content. Never with user input (XSS risk).

### `:disabled` - Conditional Disable

Disable form elements conditionally:

```hrml
state
  processing: false
  username: ""

button :disabled="processing || username.length < 3" "Submit"
```

### `:each` - List Rendering

Render lists from arrays:

```hrml
state
  items: ["Apple", "Banana", "Cherry"]

ul
  li :each="item in items" "{item}"
```

**With index:**
```hrml
li :each="(item, index) in items" "{index + 1}. {item}"
```

**With objects:**
```hrml
state
  users: [
    { id: 1, name: "Alice" },
    { id: 2, name: "Bob" }
  ]

div :each="user in users"
  h3 "{user.name}"
  p "ID: {user.id}"
```

## Symbol 2: `@` (Events)

The at symbol (`@`) represents **user-triggered actions** - anything that happens when users interact.

### Basic Events

```hrml
state
  count: 0

button @click="count++" "Increment"
button @click="count--" "Decrement"
```

### Common Event Types

```hrml
# Mouse events
div @click="handleClick"
div @dblclick="handleDoubleClick"
div @mouseenter="highlight = true"
div @mouseleave="highlight = false"

# Form events
input @input="handleInput"
input @change="handleChange"
input @focus="focused = true"
input @blur="focused = false"

# Keyboard events
input @keydown="handleKeyDown"
input @keyup="handleKeyUp"
input @keypress="handleKeyPress"

# Form submission
form @submit="handleSubmit"
```

### Event Modifiers

Modify event behavior with modifiers:

#### `.prevent` - Prevent Default

```hrml
form @submit.prevent="save"
# Equivalent to: event.preventDefault()
```

#### `.stop` - Stop Propagation

```hrml
button @click.stop="handleClick"
# Equivalent to: event.stopPropagation()
```

#### `.once` - Fire Once

```hrml
button @click.once="trackFirstClick"
# Event listener removed after first trigger
```

#### Combined Modifiers

```hrml
form @submit.prevent.stop="save"
```

### Key Modifiers

Respond to specific keys:

```hrml
# Enter key
input @keydown.enter="submit"

# Escape key
input @keydown.esc="cancel"

# Arrow keys
div @keydown.up="moveUp"
div @keydown.down="moveDown"
div @keydown.left="moveLeft"
div @keydown.right="moveRight"

# Letter keys
input @keydown.ctrl.s="save"
```

### Inline Expressions vs Functions

**Simple expressions (inline):**
```hrml
button @click="count++"
button @click="visible = !visible"
button @click="items.push('new')"
```

**Complex logic (functions):**
```hrml
state
  count: 0

fn increment() {
  count = count + 1
  console.log('Count:', count)
}

button @click="increment()" "Increment"
```

### Event Object

Access event data:

```hrml
fn handleInput(event) {
  console.log(event.target.value)
}

input @input="handleInput($event)"
```

## Symbol 3: `$` (Server Communication)

The dollar sign (`$`) represents **server interactions** - anything that communicates with APIs.

> **Note:** Server directives are planned for a future release. This section describes the intended API.

### `$get` - Fetch Data

Load data from an endpoint:

```hrml
state
  users: []
  loading: false

div $get="/api/users" $data="users" $loading="loading"
  p :show="loading" "Loading..."
  div :each="user in users"
    p "{user.name}"
```

**How it works:**
- Component mounts → triggers GET request
- Sets `loading = true`
- On success → sets `users = response`
- Sets `loading = false`

### `$post` - Send Data

Submit data to the server:

```hrml
state
  username: ""
  email: ""
  submitting: false

form @submit.prevent $post="/api/users" $body="{ username, email }" $loading="submitting"
  input :model="username"
  input :model="email"
  button :disabled="submitting" "Submit"
```

### `$trigger` - Manual Control

Trigger requests programmatically:

```hrml
state
  query: ""
  results: []

fn search() {
  // Trigger $get manually
}

input :model="query"
button @click="search" "Search"

div $get="/api/search?q={query}" $trigger="manual" $data="results"
```

### `$error` - Error Handling

Handle request failures:

```hrml
state
  users: []
  error: null

div $get="/api/users" $data="users" $error="error"
  div :show="error" .error
    p "Failed to load: {error.message}"
```

### Response Transformation

Transform response before setting state:

```hrml
state
  users: []

fn transform(response) {
  return response.data.users
}

div $get="/api/users" $data="users" $transform="transform"
```

## Combining Symbols

The power of HRML comes from combining symbols:

### Search with Debounce

```hrml
state
  query: ""
  results: []
  loading: false

input :model="query" @input.debounce="search"

div $get="/api/search?q={query}" $data="results" $loading="loading"
  p :show="loading" "Searching..."
  div :each="result in results"
    p "{result.title}"
```

### Form with Validation

```hrml
state
  email: ""
  password: ""
  errors: {}
  submitting: false

fn validate() {
  errors = {}
  if (!email.includes('@')) errors.email = "Invalid email"
  if (password.length < 8) errors.password = "Too short"
  return Object.keys(errors).length === 0
}

fn submit() {
  if (validate()) {
    // Submit
  }
}

form @submit.prevent="submit"
  input :model="email" :class="{ error: errors.email }"
  p :show="errors.email" .error "{errors.email}"

  input :model="password" type="password"
  p :show="errors.password" .error "{errors.password}"

  button :disabled="submitting" "Log In"
```

### Real-Time Data

```hrml
state
  messages: []
  newMessage: ""

# Load initial messages
div $get="/api/messages" $data="messages"

# Render messages
div :each="message in messages"
  p "{message.text}"

# Send new message
form @submit.prevent $post="/api/messages" $body="{ text: newMessage }"
  input :model="newMessage"
  button "Send"
```

## Summary

| Symbol | Purpose | Example |
|--------|---------|---------|
| `:` | State-driven behavior | `:show="visible"` |
| `@` | User-triggered actions | `@click="submit"` |
| `$` | Server communication | `$get="/api/data"` |

**Three symbols. Infinite possibilities.**

## Next Steps

- Read [getting-started.md](getting-started.md) for installation
- Explore `examples/` folder for complete files
- Try the [playground](https://hrml.dev/playground)
- Check the [roadmap](../README.md#roadmap) for upcoming features
