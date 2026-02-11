//! HTML code generator.
//!
//! Transforms the HRML document AST into HTML output.

use crate::CodegenError;
use hrml_parser::ast::Document;

/// Generate HTML from a document AST.
pub fn generate(_doc: &Document) -> Result<String, CodegenError> {
    // TODO: Implement HTML generation in feature/codegen branch
    Ok(String::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_document() {
        let doc = Document { nodes: Vec::new() };
        let html = generate(&doc).unwrap();
        assert_eq!(html, "");
    }
}
