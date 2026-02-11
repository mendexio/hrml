//! JavaScript code generator.
//!
//! Transforms the HRML document AST into JavaScript output.
//! Generates the reactive runtime code — no eval(), no new Function(),
//! fully CSP-safe.

use crate::CodegenError;
use hrml_parser::ast::Document;

/// Generate JavaScript from a document AST.
pub fn generate(_doc: &Document) -> Result<String, CodegenError> {
    // TODO: Implement JS generation in feature/codegen branch
    Ok(String::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_document() {
        let doc = Document { nodes: Vec::new() };
        let js = generate(&doc).unwrap();
        assert_eq!(js, "");
    }
}
