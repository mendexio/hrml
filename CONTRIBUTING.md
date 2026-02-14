# Contributing to HRML

Thank you for considering contributing to HRML! This document outlines how to contribute to the project.

## Ways to Contribute

- **Report bugs** - Found a bug? Open an issue with steps to reproduce
- **Suggest features** - Have an idea? Open an issue to discuss it
- **Write code** - Fix bugs, implement features, improve performance
- **Improve docs** - Fix typos, clarify explanations, add examples
- **Write tests** - Increase test coverage, add edge case tests

## Getting Started

1. **Fork the repository** on GitHub
2. **Clone your fork** locally:
   ```bash
   git clone https://github.com/mendexio/hrml.git
   cd hrml
   ```
3. **Create a branch** for your changes:
   ```bash
   git checkout -b feature/your-feature-name
   ```

## Development Setup

### Prerequisites

- Rust 1.83+ (`rustup update`)
- wasm-pack (for WASM compilation)

### Build the project

```bash
# Run all tests
cargo test

# Build the CLI
cargo build --release

# Build WASM (for playground)
cd crates/hrml-wasm && wasm-pack build --target web --release
```

### Run tests

```bash
# All tests
cargo test

# Specific crate
cargo test -p hrml-lexer
cargo test -p hrml-parser
cargo test -p hrml-codegen

# With output
cargo test -- --nocapture
```

### Lint and format

```bash
# Check for warnings
cargo clippy -- -D warnings

# Format code
cargo fmt
```

## Making Changes

1. **Write tests first** - Add tests that fail without your changes
2. **Make your changes** - Keep commits focused and atomic
3. **Run tests** - Ensure all tests pass: `cargo test`
4. **Run clippy** - Ensure no warnings: `cargo clippy`
5. **Format code** - Run `cargo fmt`

## Commit Guidelines

Use conventional commit format:

```
type: short description

Longer explanation if needed.
```

**Types:**
- `feat:` - New feature
- `fix:` - Bug fix
- `docs:` - Documentation changes
- `test:` - Adding or updating tests
- `refactor:` - Code refactoring
- `perf:` - Performance improvements
- `style:` - Code style/formatting
- `chore:` - Build, dependencies, tooling

**Examples:**
```
feat: add :each directive for list rendering
fix: handle empty state blocks correctly
docs: add getting started guide
test: add edge cases for string interpolation
```

## Pull Request Process

1. **Update tests** - Add or update tests for your changes
2. **Update docs** - Update README or docs if behavior changes
3. **Run full test suite** - `cargo test && cargo clippy`
4. **Push to your fork**
5. **Open a pull request** with:
   - Clear description of what changed and why
   - Link to related issue (if applicable)
   - Before/after examples for features or fixes

### PR Review

- Maintainers will review your PR
- Address feedback by pushing new commits
- Once approved, your PR will be merged

## Code Style

- Follow Rust conventions (`cargo fmt`)
- Document public APIs with `///` doc comments
- Keep functions focused and small
- Prefer explicit over implicit
- No `unsafe` blocks
- Zero warnings policy

## Testing

- Every public function should have tests
- Test both happy path and error cases
- Use descriptive test names: `test_parser_handles_nested_elements`
- Use `pretty_assertions` for readable test output

## Project Structure

```
hrml/
├── crates/
│   ├── hrml-lexer/      - Tokenization
│   ├── hrml-parser/     - Parsing to AST
│   ├── hrml-codegen/    - HTML/CSS/JS generation
│   ├── hrml-wasm/       - WebAssembly bindings
│   └── hrml-cli/        - Command-line interface
├── examples/            - Example .hrml files
├── playground/          - Browser playground
└── docs/                - Documentation
```

## Questions?

- Open an issue with the "question" label
- Check existing issues and discussions
- Read the [README](README.md) and [docs/](docs/)

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
