use crate::token::{Span, Token, TokenKind};
use crate::LexerError;

/// Scanner mode determines how braces are interpreted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScannerMode {
    /// Default mode: `{expr}` is interpolation.
    Html,
    /// Inside expression context: `{` and `}` are regular braces.
    Expression,
}

/// HRML source scanner.
///
/// Tokenizes `.hrml` source files into a stream of tokens.
/// Handles indentation tracking, prefix symbols, string literals
/// with inline interpolation, and keyword detection.
///
/// Follows patterns from the MOX compiler's ink-lexer:
/// - `Vec<char>` source for index-based navigation
/// - Stack-based indentation tracking
/// - Mode-aware brace handling
/// - Position tracking on every token
pub struct Scanner<'a> {
    source: &'a str,
    chars: Vec<char>,
    pos: usize,
    line: usize,
    column: usize,
    tokens: Vec<Token>,
    indent_stack: Vec<usize>,
    at_line_start: bool,
    mode: ScannerMode,
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
            at_line_start: true,
            mode: ScannerMode::Html,
        }
    }

    /// Create a scanner with a specific mode.
    pub fn with_mode(source: &'a str, mode: ScannerMode) -> Self {
        let mut scanner = Self::new(source);
        scanner.mode = mode;
        scanner
    }

    /// Tokenize the entire source into a vector of tokens.
    pub fn tokenize(source: &str) -> Result<Vec<Token>, LexerError> {
        let mut scanner = Scanner::new(source);
        scanner.scan_tokens()?;
        Ok(scanner.tokens)
    }

    /// Scan all tokens from the source.
    fn scan_tokens(&mut self) -> Result<(), LexerError> {
        while !self.is_at_end() {
            self.scan_token()?;
        }

        // Close all pending indents at EOF (MOX pattern)
        while self.indent_stack.len() > 1 {
            self.indent_stack.pop();
            self.emit(TokenKind::Dedent);
        }

        self.emit(TokenKind::Eof);
        Ok(())
    }

    /// Scan the next token.
    fn scan_token(&mut self) -> Result<(), LexerError> {
        if self.at_line_start {
            self.handle_indentation()?;
            self.at_line_start = false;
            if self.is_at_end() {
                return Ok(());
            }
        }

        let ch = self.peek();

        match ch {
            // Whitespace (mid-line, skip)
            ' ' | '\t' => {
                self.advance();
                Ok(())
            }

            // Newlines
            '\n' => {
                self.emit(TokenKind::Newline);
                self.advance();
                self.line += 1;
                self.column = 1;
                self.at_line_start = true;
                Ok(())
            }
            '\r' => {
                self.advance();
                // Handle \r\n as single newline
                if !self.is_at_end() && self.peek() == '\n' {
                    self.advance();
                }
                self.emit(TokenKind::Newline);
                self.line += 1;
                self.column = 1;
                self.at_line_start = true;
                Ok(())
            }

            // Comments
            '/' if self.peek_next() == '/' => self.scan_comment(),

            // Strings
            '"' | '\'' => self.scan_string(),

            // Interpolation in HTML mode
            '{' if self.mode == ScannerMode::Html => self.scan_interpolation(),

            // Numbers
            '0'..='9' => self.scan_number(),

            // Prefixes
            '.' => {
                self.emit(TokenKind::Dot);
                self.advance();
                Ok(())
            }
            ':' => {
                self.emit(TokenKind::Colon);
                self.advance();
                Ok(())
            }
            '@' => {
                self.emit(TokenKind::At);
                self.advance();
                Ok(())
            }
            '$' => {
                self.emit(TokenKind::Dollar);
                self.advance();
                Ok(())
            }
            '#' => {
                self.emit(TokenKind::Hash);
                self.advance();
                Ok(())
            }
            '+' => {
                self.emit(TokenKind::Plus);
                self.advance();
                Ok(())
            }

            // Punctuation
            '=' => {
                self.emit(TokenKind::Equals);
                self.advance();
                Ok(())
            }
            ',' => {
                self.emit(TokenKind::Comma);
                self.advance();
                Ok(())
            }
            '(' => {
                self.emit(TokenKind::LParen);
                self.advance();
                Ok(())
            }
            ')' => {
                self.emit(TokenKind::RParen);
                self.advance();
                Ok(())
            }

            // Identifiers and keywords
            c if c.is_alphabetic() || c == '_' => self.scan_identifier(),

            _ => Err(self.error(format!("Unexpected character: '{ch}'"))),
        }
    }

    // --- Indentation ---

    /// Handle indentation at the start of a line.
    /// Counts leading spaces, compares with indent stack, emits Indent/Dedent.
    fn handle_indentation(&mut self) -> Result<(), LexerError> {
        let mut spaces = 0;

        while !self.is_at_end() && self.peek() == ' ' {
            self.advance();
            spaces += 1;
        }

        // Skip tabs (count as error for now, HRML uses spaces)
        if !self.is_at_end() && self.peek() == '\t' {
            return Err(self.error("Tabs are not allowed for indentation, use spaces".into()));
        }

        // Skip blank lines (just whitespace then newline or EOF)
        if self.is_at_end() || self.peek() == '\n' || self.peek() == '\r' {
            return Ok(());
        }

        // Skip comment-only lines (don't affect indentation)
        if self.peek() == '/' && self.peek_next() == '/' {
            return Ok(());
        }

        let current_indent = *self.indent_stack.last().expect("indent stack never empty");

        if spaces > current_indent {
            self.indent_stack.push(spaces);
            self.emit(TokenKind::Indent);
        } else if spaces < current_indent {
            // Pop multiple levels if needed
            while self.indent_stack.len() > 1
                && *self.indent_stack.last().expect("indent stack never empty") > spaces
            {
                self.indent_stack.pop();
                self.emit(TokenKind::Dedent);
            }

            // Validate alignment
            if *self.indent_stack.last().expect("indent stack never empty") != spaces {
                return Err(self.error(format!(
                    "Indentation does not match any outer level (got {spaces} spaces)"
                )));
            }
        }

        Ok(())
    }

    // --- Scanners ---

    /// Scan a string literal. Strings carry raw content including `{expr}` markers.
    /// The parser is responsible for splitting interpolation segments.
    fn scan_string(&mut self) -> Result<(), LexerError> {
        let quote = self.peek();
        let start_line = self.line;
        let start_col = self.column;
        let start_pos = self.pos;
        self.advance(); // consume opening quote

        let mut value = std::string::String::new();

        while !self.is_at_end() && self.peek() != quote {
            if self.peek() == '\\' {
                self.advance(); // consume backslash
                if self.is_at_end() {
                    return Err(LexerError {
                        message: "Unterminated escape sequence".into(),
                        line: self.line,
                        column: self.column,
                    });
                }
                match self.peek() {
                    'n' => value.push('\n'),
                    't' => value.push('\t'),
                    'r' => value.push('\r'),
                    '\\' => value.push('\\'),
                    '{' => value.push('{'),
                    '}' => value.push('}'),
                    c if c == quote => value.push(c),
                    c => {
                        value.push('\\');
                        value.push(c);
                    }
                }
                self.advance();
            } else {
                value.push(self.peek());
                self.advance();
            }
        }

        if self.is_at_end() {
            return Err(LexerError {
                message: "Unterminated string".into(),
                line: start_line,
                column: start_col,
            });
        }

        self.advance(); // consume closing quote

        let span = Span::new(start_pos, self.pos, start_line, start_col);
        self.tokens.push(Token::new(TokenKind::String(value), span));
        Ok(())
    }

    /// Scan interpolation `{expr}` in HTML mode. Tracks brace depth for nesting.
    fn scan_interpolation(&mut self) -> Result<(), LexerError> {
        let start_line = self.line;
        let start_col = self.column;
        let start_pos = self.pos;
        self.advance(); // consume opening `{`

        let mut content = std::string::String::new();
        let mut depth = 1;

        while !self.is_at_end() && depth > 0 {
            let c = self.peek();
            match c {
                '{' => {
                    depth += 1;
                    content.push(c);
                    self.advance();
                }
                '}' => {
                    depth -= 1;
                    if depth > 0 {
                        content.push(c);
                    }
                    self.advance();
                }
                '\n' => {
                    content.push(c);
                    self.advance();
                    self.line += 1;
                    self.column = 1;
                }
                _ => {
                    content.push(c);
                    self.advance();
                }
            }
        }

        if depth > 0 {
            return Err(LexerError {
                message: "Unterminated interpolation".into(),
                line: start_line,
                column: start_col,
            });
        }

        let span = Span::new(start_pos, self.pos, start_line, start_col);
        self.tokens
            .push(Token::new(TokenKind::Interpolation(content.trim().to_string()), span));
        Ok(())
    }

    /// Scan an identifier or keyword. Supports hyphens when followed by alphanumeric
    /// (for CSS class names like `text-2xl`, `bg-blue-500`).
    fn scan_identifier(&mut self) -> Result<(), LexerError> {
        let start_line = self.line;
        let start_col = self.column;
        let start_pos = self.pos;

        let mut ident = std::string::String::new();
        ident.push(self.peek());
        self.advance();

        while !self.is_at_end()
            && (self.peek().is_alphanumeric()
                || self.peek() == '_'
                || (self.peek() == '-' && self.peek_next().is_alphanumeric()))
        {
            ident.push(self.peek());
            self.advance();
        }

        let span = Span::new(start_pos, self.pos, start_line, start_col);
        // After a prefix (., :, @, $, #), suppress HRML keywords —
        // they're class names, state names, event names, etc.
        // Literals (true, false, null) are always recognized.
        let after_prefix = self.tokens.last().is_some_and(|t| {
            matches!(
                t.kind,
                TokenKind::Dot
                    | TokenKind::Colon
                    | TokenKind::At
                    | TokenKind::Dollar
                    | TokenKind::Hash
            )
        });
        let kind = if after_prefix {
            match ident.as_str() {
                "true" => TokenKind::Boolean(true),
                "false" => TokenKind::Boolean(false),
                "null" => TokenKind::Null,
                _ => TokenKind::Identifier(ident),
            }
        } else {
            Self::keyword_or_ident(ident)
        };
        self.tokens.push(Token::new(kind, span));
        Ok(())
    }

    /// Scan a number literal (integer or float).
    fn scan_number(&mut self) -> Result<(), LexerError> {
        let start_line = self.line;
        let start_col = self.column;
        let start_pos = self.pos;

        while !self.is_at_end() && (self.peek().is_ascii_digit() || self.peek() == '.') {
            self.advance();
        }

        let text = &self.source[start_pos..self.pos];
        let value: f64 = text.parse().map_err(|_| LexerError {
            message: format!("Invalid number: '{text}'"),
            line: start_line,
            column: start_col,
        })?;

        let span = Span::new(start_pos, self.pos, start_line, start_col);
        self.tokens.push(Token::new(TokenKind::Number(value), span));
        Ok(())
    }

    /// Scan a line comment (`// ...`).
    fn scan_comment(&mut self) -> Result<(), LexerError> {
        let start_line = self.line;
        let start_col = self.column;
        let start_pos = self.pos;

        // Skip the two `/` characters
        self.advance();
        self.advance();

        // Skip optional space after //
        if !self.is_at_end() && self.peek() == ' ' {
            self.advance();
        }

        let mut content = std::string::String::new();
        while !self.is_at_end() && self.peek() != '\n' && self.peek() != '\r' {
            content.push(self.peek());
            self.advance();
        }

        let span = Span::new(start_pos, self.pos, start_line, start_col);
        self.tokens
            .push(Token::new(TokenKind::Comment(content), span));
        Ok(())
    }

    // --- Keyword detection ---

    /// Determine if an identifier is a keyword or remains an identifier.
    fn keyword_or_ident(ident: std::string::String) -> TokenKind {
        match ident.as_str() {
            "state" => TokenKind::State,
            "computed" => TokenKind::Computed,
            "fn" => TokenKind::Fn,
            "async" => TokenKind::Async,
            "watch" => TokenKind::Watch,
            "props" => TokenKind::Props,
            "emit" => TokenKind::Emit,
            "import" => TokenKind::Import,
            "page" => TokenKind::Page,
            "config" => TokenKind::Config,
            "true" => TokenKind::Boolean(true),
            "false" => TokenKind::Boolean(false),
            "null" => TokenKind::Null,
            _ => TokenKind::Identifier(ident),
        }
    }

    // --- Helpers ---

    fn emit(&mut self, kind: TokenKind) {
        let span = Span::new(self.pos, self.pos, self.line, self.column);
        self.tokens.push(Token::new(kind, span));
    }

    fn peek(&self) -> char {
        if self.is_at_end() {
            '\0'
        } else {
            self.chars[self.pos]
        }
    }

    fn peek_next(&self) -> char {
        if self.pos + 1 >= self.chars.len() {
            '\0'
        } else {
            self.chars[self.pos + 1]
        }
    }

    fn advance(&mut self) {
        if !self.is_at_end() {
            self.pos += 1;
            self.column += 1;
        }
    }

    fn is_at_end(&self) -> bool {
        self.pos >= self.chars.len()
    }

    fn error(&self, message: std::string::String) -> LexerError {
        LexerError {
            message,
            line: self.line,
            column: self.column,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: tokenize and return token kinds (ignoring spans).
    fn kinds(source: &str) -> Vec<TokenKind> {
        Scanner::tokenize(source)
            .unwrap()
            .into_iter()
            .map(|t| t.kind)
            .collect()
    }

    /// Helper: tokenize and panic on error.
    fn tokens(source: &str) -> Vec<Token> {
        Scanner::tokenize(source).unwrap()
    }

    // =========================================================================
    // Structure: empty, newlines, EOF
    // =========================================================================

    #[test]
    fn test_empty_source() {
        let toks = tokens("");
        assert_eq!(toks.len(), 1);
        assert_eq!(toks[0].kind, TokenKind::Eof);
    }

    #[test]
    fn test_single_newline() {
        assert_eq!(kinds("\n"), vec![TokenKind::Newline, TokenKind::Eof]);
    }

    #[test]
    fn test_multiple_newlines() {
        assert_eq!(
            kinds("\n\n\n"),
            vec![
                TokenKind::Newline,
                TokenKind::Newline,
                TokenKind::Newline,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_windows_line_endings() {
        assert_eq!(kinds("\r\n"), vec![TokenKind::Newline, TokenKind::Eof]);
    }

    #[test]
    fn test_carriage_return_only() {
        assert_eq!(kinds("\r"), vec![TokenKind::Newline, TokenKind::Eof]);
    }

    // =========================================================================
    // Structure: indentation
    // =========================================================================

    #[test]
    fn test_indent_simple() {
        let k = kinds("a\n  b");
        assert_eq!(
            k,
            vec![
                TokenKind::Identifier("a".into()),
                TokenKind::Newline,
                TokenKind::Indent,
                TokenKind::Identifier("b".into()),
                TokenKind::Dedent,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_indent_multiple_levels() {
        let k = kinds("a\n  b\n    c");
        assert_eq!(
            k,
            vec![
                TokenKind::Identifier("a".into()),
                TokenKind::Newline,
                TokenKind::Indent,
                TokenKind::Identifier("b".into()),
                TokenKind::Newline,
                TokenKind::Indent,
                TokenKind::Identifier("c".into()),
                TokenKind::Dedent,
                TokenKind::Dedent,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_dedent_multiple_levels() {
        let k = kinds("a\n  b\n    c\nd");
        assert_eq!(
            k,
            vec![
                TokenKind::Identifier("a".into()),
                TokenKind::Newline,
                TokenKind::Indent,
                TokenKind::Identifier("b".into()),
                TokenKind::Newline,
                TokenKind::Indent,
                TokenKind::Identifier("c".into()),
                TokenKind::Newline,
                TokenKind::Dedent,
                TokenKind::Dedent,
                TokenKind::Identifier("d".into()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_eof_auto_closes_indents() {
        let k = kinds("a\n  b\n    c");
        // Should have 2 dedents before EOF
        assert_eq!(k.iter().filter(|t| **t == TokenKind::Dedent).count(), 2);
        assert_eq!(*k.last().unwrap(), TokenKind::Eof);
    }

    #[test]
    fn test_blank_lines_ignored_for_indent() {
        let k = kinds("a\n\n  b");
        assert_eq!(
            k,
            vec![
                TokenKind::Identifier("a".into()),
                TokenKind::Newline,
                TokenKind::Newline,
                TokenKind::Indent,
                TokenKind::Identifier("b".into()),
                TokenKind::Dedent,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_same_indent_no_token() {
        let k = kinds("a\nb");
        assert_eq!(
            k,
            vec![
                TokenKind::Identifier("a".into()),
                TokenKind::Newline,
                TokenKind::Identifier("b".into()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_indent_error_misaligned() {
        let result = Scanner::tokenize("a\n  b\n c");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.message.contains("does not match"));
    }

    #[test]
    fn test_tabs_rejected() {
        let result = Scanner::tokenize("\ta");
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("Tabs"));
    }

    // =========================================================================
    // Identifiers and hyphens
    // =========================================================================

    #[test]
    fn test_simple_identifier() {
        assert_eq!(
            kinds("div"),
            vec![TokenKind::Identifier("div".into()), TokenKind::Eof]
        );
    }

    #[test]
    fn test_hyphenated_identifier() {
        assert_eq!(
            kinds("text-2xl"),
            vec![TokenKind::Identifier("text-2xl".into()), TokenKind::Eof]
        );
    }

    #[test]
    fn test_multi_hyphen_identifier() {
        assert_eq!(
            kinds("bg-blue-500"),
            vec![TokenKind::Identifier("bg-blue-500".into()), TokenKind::Eof]
        );
    }

    #[test]
    fn test_hover_variant_identifier() {
        // hover:bg-blue-600 — colon splits it, so we get identifier + colon + identifier
        let k = kinds("hover:bg-blue-600");
        assert_eq!(
            k,
            vec![
                TokenKind::Identifier("hover".into()),
                TokenKind::Colon,
                TokenKind::Identifier("bg-blue-600".into()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_multiple_identifiers() {
        assert_eq!(
            kinds("div span"),
            vec![
                TokenKind::Identifier("div".into()),
                TokenKind::Identifier("span".into()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_identifier_with_underscore() {
        assert_eq!(
            kinds("my_var"),
            vec![TokenKind::Identifier("my_var".into()), TokenKind::Eof]
        );
    }

    // =========================================================================
    // Prefixes
    // =========================================================================

    #[test]
    fn test_dot_prefix() {
        assert_eq!(
            kinds(".container"),
            vec![
                TokenKind::Dot,
                TokenKind::Identifier("container".into()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_dot_class_chain() {
        assert_eq!(
            kinds("div .flex items-center gap-4"),
            vec![
                TokenKind::Identifier("div".into()),
                TokenKind::Dot,
                TokenKind::Identifier("flex".into()),
                TokenKind::Identifier("items-center".into()),
                TokenKind::Identifier("gap-4".into()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_colon_prefix() {
        let k = kinds(":show");
        assert_eq!(
            k,
            vec![
                TokenKind::Colon,
                TokenKind::Identifier("show".into()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_at_prefix() {
        let k = kinds("@click");
        assert_eq!(
            k,
            vec![
                TokenKind::At,
                TokenKind::Identifier("click".into()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_dollar_prefix() {
        let k = kinds("$get");
        assert_eq!(
            k,
            vec![
                TokenKind::Dollar,
                TokenKind::Identifier("get".into()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_hash_prefix() {
        let k = kinds("#main");
        assert_eq!(
            k,
            vec![
                TokenKind::Hash,
                TokenKind::Identifier("main".into()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_plus_prefix() {
        let k = kinds("+th");
        assert_eq!(
            k,
            vec![
                TokenKind::Plus,
                TokenKind::Identifier("th".into()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_state_directive_with_value() {
        let k = kinds(":show=\"visible\"");
        assert_eq!(
            k,
            vec![
                TokenKind::Colon,
                TokenKind::Identifier("show".into()),
                TokenKind::Equals,
                TokenKind::String("visible".into()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_event_with_value() {
        let k = kinds("@click=\"count++\"");
        assert_eq!(
            k,
            vec![
                TokenKind::At,
                TokenKind::Identifier("click".into()),
                TokenKind::Equals,
                TokenKind::String("count++".into()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_event_modifier() {
        let k = kinds("@submit.prevent");
        assert_eq!(
            k,
            vec![
                TokenKind::At,
                TokenKind::Identifier("submit".into()),
                TokenKind::Dot,
                TokenKind::Identifier("prevent".into()),
                TokenKind::Eof,
            ]
        );
    }

    // =========================================================================
    // Strings
    // =========================================================================

    #[test]
    fn test_double_quoted_string() {
        assert_eq!(
            kinds("\"hello\""),
            vec![TokenKind::String("hello".into()), TokenKind::Eof]
        );
    }

    #[test]
    fn test_single_quoted_string() {
        assert_eq!(
            kinds("'hello'"),
            vec![TokenKind::String("hello".into()), TokenKind::Eof]
        );
    }

    #[test]
    fn test_empty_string() {
        assert_eq!(
            kinds("\"\""),
            vec![TokenKind::String("".into()), TokenKind::Eof]
        );
    }

    #[test]
    fn test_string_escape_newline() {
        assert_eq!(
            kinds("\"hello\\nworld\""),
            vec![
                TokenKind::String("hello\nworld".into()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_string_escape_tab() {
        assert_eq!(
            kinds("\"col1\\tcol2\""),
            vec![
                TokenKind::String("col1\tcol2".into()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_string_escape_braces() {
        assert_eq!(
            kinds("\"\\{not interpolation\\}\""),
            vec![
                TokenKind::String("{not interpolation}".into()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_string_with_interpolation_marker() {
        // String carries raw content; parser splits later
        assert_eq!(
            kinds("\"Hello, {name}!\""),
            vec![
                TokenKind::String("Hello, {name}!".into()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_string_unterminated() {
        let result = Scanner::tokenize("\"hello");
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("Unterminated string"));
    }

    #[test]
    fn test_string_escaped_quote() {
        assert_eq!(
            kinds("\"say \\\"hi\\\"\""),
            vec![
                TokenKind::String("say \"hi\"".into()),
                TokenKind::Eof,
            ]
        );
    }

    // =========================================================================
    // Interpolation (HTML mode)
    // =========================================================================

    #[test]
    fn test_simple_interpolation() {
        assert_eq!(
            kinds("{count}"),
            vec![TokenKind::Interpolation("count".into()), TokenKind::Eof]
        );
    }

    #[test]
    fn test_interpolation_expression() {
        assert_eq!(
            kinds("{count + 1}"),
            vec![
                TokenKind::Interpolation("count + 1".into()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_interpolation_nested_braces() {
        assert_eq!(
            kinds("{items[0]}"),
            vec![
                TokenKind::Interpolation("items[0]".into()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_interpolation_object() {
        assert_eq!(
            kinds("{{ active: isActive }}"),
            vec![
                TokenKind::Interpolation("{ active: isActive }".into()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_interpolation_unterminated() {
        let result = Scanner::tokenize("{count");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .message
            .contains("Unterminated interpolation"));
    }

    // =========================================================================
    // Keywords and booleans
    // =========================================================================

    #[test]
    fn test_state_keyword() {
        assert_eq!(kinds("state"), vec![TokenKind::State, TokenKind::Eof]);
    }

    #[test]
    fn test_computed_keyword() {
        assert_eq!(kinds("computed"), vec![TokenKind::Computed, TokenKind::Eof]);
    }

    #[test]
    fn test_fn_keyword() {
        assert_eq!(kinds("fn"), vec![TokenKind::Fn, TokenKind::Eof]);
    }

    #[test]
    fn test_async_keyword() {
        assert_eq!(kinds("async"), vec![TokenKind::Async, TokenKind::Eof]);
    }

    #[test]
    fn test_watch_keyword() {
        assert_eq!(kinds("watch"), vec![TokenKind::Watch, TokenKind::Eof]);
    }

    #[test]
    fn test_boolean_true() {
        assert_eq!(
            kinds("true"),
            vec![TokenKind::Boolean(true), TokenKind::Eof]
        );
    }

    #[test]
    fn test_boolean_false() {
        assert_eq!(
            kinds("false"),
            vec![TokenKind::Boolean(false), TokenKind::Eof]
        );
    }

    #[test]
    fn test_null_keyword() {
        assert_eq!(kinds("null"), vec![TokenKind::Null, TokenKind::Eof]);
    }

    // =========================================================================
    // Numbers
    // =========================================================================

    #[test]
    fn test_integer() {
        assert_eq!(
            kinds("42"),
            vec![TokenKind::Number(42.0), TokenKind::Eof]
        );
    }

    #[test]
    fn test_float() {
        assert_eq!(
            kinds("2.75"),
            vec![TokenKind::Number(2.75), TokenKind::Eof]
        );
    }

    #[test]
    fn test_zero() {
        assert_eq!(
            kinds("0"),
            vec![TokenKind::Number(0.0), TokenKind::Eof]
        );
    }

    // =========================================================================
    // Comments
    // =========================================================================

    #[test]
    fn test_line_comment() {
        assert_eq!(
            kinds("// this is a comment"),
            vec![
                TokenKind::Comment("this is a comment".into()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_comment_after_code() {
        let k = kinds("div // element");
        assert_eq!(
            k,
            vec![
                TokenKind::Identifier("div".into()),
                TokenKind::Comment("element".into()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_empty_comment() {
        assert_eq!(
            kinds("//"),
            vec![TokenKind::Comment("".into()), TokenKind::Eof]
        );
    }

    #[test]
    fn test_comment_lines_dont_affect_indent() {
        let k = kinds("a\n  // comment\n  b");
        assert_eq!(
            k,
            vec![
                TokenKind::Identifier("a".into()),
                TokenKind::Newline,
                TokenKind::Comment("comment".into()),
                TokenKind::Newline,
                TokenKind::Indent,
                TokenKind::Identifier("b".into()),
                TokenKind::Dedent,
                TokenKind::Eof,
            ]
        );
    }

    // =========================================================================
    // Punctuation
    // =========================================================================

    #[test]
    fn test_equals() {
        assert_eq!(kinds("="), vec![TokenKind::Equals, TokenKind::Eof]);
    }

    #[test]
    fn test_comma() {
        assert_eq!(kinds(","), vec![TokenKind::Comma, TokenKind::Eof]);
    }

    #[test]
    fn test_parens() {
        assert_eq!(
            kinds("()"),
            vec![TokenKind::LParen, TokenKind::RParen, TokenKind::Eof]
        );
    }

    // =========================================================================
    // Error handling
    // =========================================================================

    #[test]
    fn test_unexpected_character() {
        let result = Scanner::tokenize("~");
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("Unexpected character"));
    }

    // =========================================================================
    // Span tracking
    // =========================================================================

    #[test]
    fn test_span_line_column() {
        let toks = tokens("div\n  span");
        // "div" at line 1, col 1
        assert_eq!(toks[0].span.line, 1);
        assert_eq!(toks[0].span.column, 1);
        // "span" at line 2, col 3 (after 2 spaces)
        let span_tok = toks.iter().find(|t| t.kind == TokenKind::Identifier("span".into())).unwrap();
        assert_eq!(span_tok.span.line, 2);
        assert_eq!(span_tok.span.column, 3);
    }

    // =========================================================================
    // Full HRML snippets — the 3 required examples
    // =========================================================================

    #[test]
    fn test_counter_example() {
        let source = r#"state
  count: 0

div .flex items-center gap-4 p-6
  button @click="count--" "-"
  span .text-2xl "{count}"
  button @click="count++" "+""#;

        let toks = tokens(source);
        // Should parse without error
        assert!(!toks.is_empty());
        assert_eq!(*toks.last().unwrap(), Token::new(TokenKind::Eof, toks.last().unwrap().span));
        // Check key tokens exist
        let k: Vec<_> = toks.iter().map(|t| &t.kind).collect();
        assert!(k.contains(&&TokenKind::State));
        assert!(k.contains(&&TokenKind::Identifier("count".into())));
        assert!(k.contains(&&TokenKind::Number(0.0)));
        assert!(k.contains(&&TokenKind::Identifier("div".into())));
        assert!(k.contains(&&TokenKind::Dot));
        assert!(k.contains(&&TokenKind::At));
        assert!(k.contains(&&TokenKind::Indent));
    }

    #[test]
    fn test_toggle_example() {
        let source = r#"state
  visible: true

button @click="visible = !visible" "Toggle"
div :show="visible"
  p "Now you see me""#;

        let toks = tokens(source);
        assert!(!toks.is_empty());
        let k: Vec<_> = toks.iter().map(|t| &t.kind).collect();
        assert!(k.contains(&&TokenKind::State));
        assert!(k.contains(&&TokenKind::Boolean(true)));
        assert!(k.contains(&&TokenKind::At));
        assert!(k.contains(&&TokenKind::Colon));
        assert!(k.contains(&&TokenKind::String("Toggle".into())));
    }

    #[test]
    fn test_input_binding_example() {
        let source = r#"state
  name: ""

div .p-4
  input :model="name" placeholder="Your name"
  p "Hello, {name}!""#;

        let toks = tokens(source);
        assert!(!toks.is_empty());
        let k: Vec<_> = toks.iter().map(|t| &t.kind).collect();
        assert!(k.contains(&&TokenKind::State));
        assert!(k.contains(&&TokenKind::Identifier("input".into())));
        assert!(k.contains(&&TokenKind::Colon));
        assert!(k.contains(&&TokenKind::String("Hello, {name}!".into())));
    }

    // =========================================================================
    // State block tokenization
    // =========================================================================

    #[test]
    fn test_state_block() {
        let k = kinds("state\n  count: 0\n  loading: false");
        assert_eq!(
            k,
            vec![
                TokenKind::State,
                TokenKind::Newline,
                TokenKind::Indent,
                TokenKind::Identifier("count".into()),
                TokenKind::Colon,
                TokenKind::Number(0.0),
                TokenKind::Newline,
                TokenKind::Identifier("loading".into()),
                TokenKind::Colon,
                TokenKind::Boolean(false),
                TokenKind::Dedent,
                TokenKind::Eof,
            ]
        );
    }

    // =========================================================================
    // Element with attributes
    // =========================================================================

    #[test]
    fn test_element_with_attribute() {
        let k = kinds("a href=\"/about\" \"About\"");
        assert_eq!(
            k,
            vec![
                TokenKind::Identifier("a".into()),
                TokenKind::Identifier("href".into()),
                TokenKind::Equals,
                TokenKind::String("/about".into()),
                TokenKind::String("About".into()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_img_with_attributes() {
        let k = kinds("img src=\"/photo.jpg\" alt=\"Photo\" .rounded");
        assert_eq!(
            k,
            vec![
                TokenKind::Identifier("img".into()),
                TokenKind::Identifier("src".into()),
                TokenKind::Equals,
                TokenKind::String("/photo.jpg".into()),
                TokenKind::Identifier("alt".into()),
                TokenKind::Equals,
                TokenKind::String("Photo".into()),
                TokenKind::Dot,
                TokenKind::Identifier("rounded".into()),
                TokenKind::Eof,
            ]
        );
    }

    // =========================================================================
    // Nested structure
    // =========================================================================

    #[test]
    fn test_nested_elements() {
        let source = "div .page\n  header\n    span \"Logo\"\n  main\n    h1 \"Welcome\"";
        let k = kinds(source);
        assert_eq!(
            k,
            vec![
                TokenKind::Identifier("div".into()),
                TokenKind::Dot,
                TokenKind::Identifier("page".into()),
                TokenKind::Newline,
                TokenKind::Indent,
                TokenKind::Identifier("header".into()),
                TokenKind::Newline,
                TokenKind::Indent,
                TokenKind::Identifier("span".into()),
                TokenKind::String("Logo".into()),
                TokenKind::Newline,
                TokenKind::Dedent,
                TokenKind::Identifier("main".into()),
                TokenKind::Newline,
                TokenKind::Indent,
                TokenKind::Identifier("h1".into()),
                TokenKind::String("Welcome".into()),
                TokenKind::Dedent,
                TokenKind::Dedent,
                TokenKind::Eof,
            ]
        );
    }

    // =========================================================================
    // Prefix + keyword suppression
    // =========================================================================

    #[test]
    fn test_dot_state_is_identifier() {
        // .state is a CSS class, not the State keyword
        assert_eq!(
            kinds(".state"),
            vec![TokenKind::Dot, TokenKind::Identifier("state".into()), TokenKind::Eof]
        );
    }

    #[test]
    fn test_at_fn_is_identifier() {
        // @fn is an event name, not the Fn keyword
        assert_eq!(
            kinds("@fn"),
            vec![TokenKind::At, TokenKind::Identifier("fn".into()), TokenKind::Eof]
        );
    }

    #[test]
    fn test_colon_computed_is_identifier() {
        // :computed is a directive name, not the Computed keyword
        assert_eq!(
            kinds(":computed"),
            vec![TokenKind::Colon, TokenKind::Identifier("computed".into()), TokenKind::Eof]
        );
    }

    #[test]
    fn test_dollar_import_is_identifier() {
        // $import is a server name, not the Import keyword
        assert_eq!(
            kinds("$import"),
            vec![TokenKind::Dollar, TokenKind::Identifier("import".into()), TokenKind::Eof]
        );
    }

    #[test]
    fn test_colon_true_is_boolean() {
        // Literals always recognized even after prefix
        assert_eq!(
            kinds(":true"),
            vec![TokenKind::Colon, TokenKind::Boolean(true), TokenKind::Eof]
        );
    }

    #[test]
    fn test_colon_null_is_null() {
        assert_eq!(
            kinds(":null"),
            vec![TokenKind::Colon, TokenKind::Null, TokenKind::Eof]
        );
    }

    // =========================================================================
    // Edge cases
    // =========================================================================

    #[test]
    fn test_number_then_identifier() {
        assert_eq!(
            kinds("42 items"),
            vec![TokenKind::Number(42.0), TokenKind::Identifier("items".into()), TokenKind::Eof]
        );
    }

    #[test]
    fn test_identifier_starting_with_keyword() {
        // "stateful" starts with "state" but is an identifier
        assert_eq!(
            kinds("stateful"),
            vec![TokenKind::Identifier("stateful".into()), TokenKind::Eof]
        );
    }

    #[test]
    fn test_identifier_starting_with_true() {
        // "truthy" starts with "true" but is an identifier
        assert_eq!(
            kinds("truthy"),
            vec![TokenKind::Identifier("truthy".into()), TokenKind::Eof]
        );
    }

    #[test]
    fn test_consecutive_strings() {
        assert_eq!(
            kinds("\"hello\" \"world\""),
            vec![
                TokenKind::String("hello".into()),
                TokenKind::String("world".into()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_all_keywords() {
        assert_eq!(
            kinds("state computed fn async watch props emit import page config"),
            vec![
                TokenKind::State, TokenKind::Computed, TokenKind::Fn,
                TokenKind::Async, TokenKind::Watch, TokenKind::Props,
                TokenKind::Emit, TokenKind::Import, TokenKind::Page,
                TokenKind::Config, TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_all_prefixes_in_sequence() {
        assert_eq!(
            kinds(".a :b @c $d #e +f"),
            vec![
                TokenKind::Dot, TokenKind::Identifier("a".into()),
                TokenKind::Colon, TokenKind::Identifier("b".into()),
                TokenKind::At, TokenKind::Identifier("c".into()),
                TokenKind::Dollar, TokenKind::Identifier("d".into()),
                TokenKind::Hash, TokenKind::Identifier("e".into()),
                TokenKind::Plus, TokenKind::Identifier("f".into()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_empty_lines_between_elements() {
        // Each \n produces a Newline token; blank lines don't affect indentation
        assert_eq!(
            kinds("div\n\n\nspan"),
            vec![
                TokenKind::Identifier("div".into()),
                TokenKind::Newline,
                TokenKind::Newline,
                TokenKind::Newline,
                TokenKind::Identifier("span".into()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_trailing_whitespace() {
        assert_eq!(
            kinds("div   \nspan"),
            vec![
                TokenKind::Identifier("div".into()),
                TokenKind::Newline,
                TokenKind::Identifier("span".into()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_string_with_spaces() {
        assert_eq!(
            kinds("\"hello world\""),
            vec![TokenKind::String("hello world".into()), TokenKind::Eof]
        );
    }
}
