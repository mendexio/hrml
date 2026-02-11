use crate::token::{Span, Token, TokenKind};
use crate::LexerError;

/// HRML source scanner.
///
/// Tokenizes `.hrml` source files into a stream of tokens.
/// Handles indentation tracking, element detection, prefix symbols,
/// string literals, and interpolation.
// Fields will be used in feature/lexer branch
#[allow(dead_code)]
pub struct Scanner<'a> {
    source: &'a str,
    chars: Vec<char>,
    pos: usize,
    line: usize,
    column: usize,
    tokens: Vec<Token>,
    indent_stack: Vec<usize>,
}

impl<'a> Scanner<'a> {
    /// Create a new scanner for the given source.
    pub fn new(source: &'a str) -> Self {
        Self {
            source,
            chars: source.chars().collect(),
            pos: 0,
            line: 1,
            column: 1,
            tokens: Vec::new(),
            indent_stack: vec![0],
        }
    }

    /// Tokenize the entire source into a vector of tokens.
    pub fn tokenize(source: &str) -> Result<Vec<Token>, LexerError> {
        let mut scanner = Scanner::new(source);
        scanner.scan_tokens()?;
        Ok(scanner.tokens)
    }

    /// Scan all tokens from the source.
    fn scan_tokens(&mut self) -> Result<(), LexerError> {
        // TODO: Implement full scanner in feature/lexer branch
        self.tokens.push(Token::new(
            TokenKind::Eof,
            "",
            Span::new(self.pos, self.pos, self.line, self.column),
        ));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_source() {
        let tokens = Scanner::tokenize("").unwrap();
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].kind, TokenKind::Eof);
    }
}
