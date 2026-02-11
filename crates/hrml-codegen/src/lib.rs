//! HRML Code Generator
//!
//! Compiles the HRML AST into three outputs: HTML, CSS, and JavaScript.
//! Each generator is a separate module with a clean interface.
//!
//! ```text
//! Document AST → compile() → CompilerOutput { html, css, js }
//! ```

pub mod css;
pub mod html;
pub mod js;

use hrml_parser::ast::Document;

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

/// Compile an HRML document AST into HTML + CSS + JS.
pub fn compile(doc: &Document) -> Result<CompilerOutput, CodegenError> {
    let html = html::generate(doc)?;
    let css = css::generate(doc)?;
    let js = js::generate(doc)?;

    Ok(CompilerOutput { html, css, js })
}
