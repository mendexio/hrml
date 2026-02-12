//! Abstract Syntax Tree for HRML.
//!
//! Contains both document-level nodes (elements, state blocks, components)
//! and expression-level nodes (binary ops, calls, literals).
//!
//! Document types are new for v2.0. Expression types are reused from the
//! v1.0 prototype's expression parser.

// ---------------------------------------------------------------------------
// Document-level AST (new for v2.0)
// ---------------------------------------------------------------------------

/// A complete HRML document.
#[derive(Debug, Clone, PartialEq)]
pub struct Document {
    pub nodes: Vec<Node>,
}

/// A top-level node in the document.
#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    /// An HTML element with optional classes, attributes, and children.
    Element(Element),

    /// A `state` block declaring reactive state.
    StateBlock(StateBlock),

    /// A `computed` block declaring derived values.
    ComputedBlock(ComputedBlock),

    /// A `fn` or `async fn` declaration.
    FnDecl(FnDecl),

    /// A component definition.
    Component(Component),

    /// Raw text content (may contain `{expr}` interpolation markers).
    Text(String),

    /// A `// comment` line.
    Comment(String),
}

/// An HTML element.
#[derive(Debug, Clone, PartialEq)]
pub struct Element {
    pub tag: String,
    pub classes: Vec<String>,
    pub attributes: Vec<Attribute>,
    pub children: Vec<Node>,
}

/// An attribute on an element.
/// For event handlers (`@click.prevent`), modifiers stores `["prevent"]`.
#[derive(Debug, Clone, PartialEq)]
pub struct Attribute {
    pub name: String,
    pub value: Option<Expression>,
    pub prefix: Option<AttributePrefix>,
    pub modifiers: Vec<String>,
}

/// The three HRML prefixes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttributePrefix {
    /// `:` — state and reactivity
    State,
    /// `@` — events and interaction
    Event,
    /// `$` — server communication
    Server,
}

/// A `:state` block.
#[derive(Debug, Clone, PartialEq)]
pub struct StateBlock {
    pub fields: Vec<StateField>,
}

/// A field inside a `:state` block.
#[derive(Debug, Clone, PartialEq)]
pub struct StateField {
    pub name: String,
    pub value: Expression,
}

/// A `:computed` block.
#[derive(Debug, Clone, PartialEq)]
pub struct ComputedBlock {
    pub fields: Vec<ComputedField>,
}

/// A field inside a `:computed` block.
#[derive(Debug, Clone, PartialEq)]
pub struct ComputedField {
    pub name: String,
    pub body: Expression,
}

/// A function declaration (`fn` or `async fn`).
#[derive(Debug, Clone, PartialEq)]
pub struct FnDecl {
    pub name: String,
    pub params: Vec<String>,
    pub body: Vec<Expression>,
    pub is_async: bool,
}

/// A component definition.
#[derive(Debug, Clone, PartialEq)]
pub struct Component {
    pub name: String,
    pub props: Vec<PropDef>,
    pub children: Vec<Node>,
}

/// A prop definition in a component.
#[derive(Debug, Clone, PartialEq)]
pub struct PropDef {
    pub name: String,
    pub default: Option<Expression>,
}

// ---------------------------------------------------------------------------
// Expression-level AST (reused from v1.0 prototype)
// ---------------------------------------------------------------------------

/// A position in expression text (relative to the expression string, not the source file).
/// Named `ExprSpan` to distinguish from `hrml_lexer::Span` which tracks source file positions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExprSpan {
    pub start: usize,
    pub end: usize,
}

impl ExprSpan {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }
}

/// A complete expression node.
#[derive(Debug, Clone, PartialEq)]
pub struct Expression {
    pub kind: ExprKind,
    pub span: ExprSpan,
}

/// Expression variants.
#[derive(Debug, Clone, PartialEq)]
pub enum ExprKind {
    /// Numeric literal: `42`, `3.14`
    Number(f64),

    /// String literal: `"hello"`, `'world'`
    String(String),

    /// Boolean literal: `true`, `false`
    Boolean(bool),

    /// Null literal
    Null,

    /// Undefined literal
    Undefined,

    /// Identifier: `count`, `isActive`
    Identifier(String),

    /// Binary operation: `a + b`, `count > 0`
    Binary {
        left: Box<Expression>,
        op: BinaryOp,
        right: Box<Expression>,
    },

    /// Unary operation: `!active`, `-count`
    Unary {
        op: UnaryOp,
        operand: Box<Expression>,
    },

    /// Postfix operation: `count++`, `count--`
    Postfix {
        operand: Box<Expression>,
        op: PostfixOp,
    },

    /// Member access: `user.name`, `items[0]`
    Member {
        object: Box<Expression>,
        property: Box<Expression>,
        computed: bool,
    },

    /// Function call: `save()`, `items.push(item)`
    Call {
        callee: Box<Expression>,
        arguments: Vec<Expression>,
    },

    /// Ternary: `count > 0 ? 'yes' : 'no'`
    Ternary {
        condition: Box<Expression>,
        consequent: Box<Expression>,
        alternate: Box<Expression>,
    },

    /// Object literal: `{ count: 0, name: 'test' }`
    Object(Vec<ObjectProperty>),

    /// Array literal: `[1, 2, 3]`
    Array(Vec<Expression>),

    /// Arrow function: `(x) => x + 1`
    Arrow {
        params: Vec<String>,
        body: Box<Expression>,
    },

    /// Assignment: `count = 5`, `count += 1`
    Assignment {
        target: Box<Expression>,
        op: AssignOp,
        value: Box<Expression>,
    },

    /// Template literal segment (from `{expr}` interpolation)
    Interpolation(Box<Expression>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ObjectProperty {
    pub key: String,
    pub value: Expression,
    pub shorthand: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    Neq,
    StrictEq,
    StrictNeq,
    Lt,
    Gt,
    Lte,
    Gte,
    And,
    Or,
    NullishCoalescing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Not,
    Neg,
    Typeof,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PostfixOp {
    Increment,
    Decrement,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssignOp {
    Assign,
    AddAssign,
    SubAssign,
    MulAssign,
    DivAssign,
}
