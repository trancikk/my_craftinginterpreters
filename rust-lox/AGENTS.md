# AGENTS.md — rust-lox

## Project

**rust-lox** — A Rust implementation of the Lox programming language from [Crafting Interpreters](https://craftinginterpreters.com/) by Robert Nystrom.

This is a tree-walking interpreter implementing:
- Lexical scanning (scanner)
- Recursive descent parsing (parser) with error recovery
- Tree-walking evaluation (eval) with runtime type checking
- Variable declarations, assignments, and scoping
- Control flow: if/else, while loops, for loops
- Functions and closures (planned)

**Current Status**: Core language features implemented; parser has known bug in assignment parsing; environment lacks nested scope resolution.

## Stack & Conventions

- **Language**: Rust (edition 2021)
- **Build**: Cargo (`cargo build`, `cargo test`, `cargo run`)
- **Testing**: Built-in `#[cfg(test)]` modules in each source file
- **Error Handling**: `thiserror` for error types, `Result<T, E>` throughout
- **Architecture**: Modular — `scanner`, `parser`, `eval`, `environment`, `interpreter`, `token`
- **No external runtime deps** beyond `thiserror` and `log`

## Key Constraints

| Constraint | Detail |
|------------|--------|
| **No parser changes** | User requested not to modify implementation — only add tests |
| **Parser bug** | `assignment()` in `parser.rs` doesn't consume `=` token — assignment expressions fail to parse |
| **Environment limitation** | `Env::get()` and `Env::assign()` don't traverse `parent_env` — nested scopes don't work |
| **Assignment semantics** | `assign()` returns `bool` but doesn't error on undefined variable — silently succeeds |
| **Single-threaded** | No concurrency; `RefCell`/`Rc` used for environment sharing |

## Architecture Overview

```
src/
├── main.rs           # Entry point, REPL, file runner
├── rlox.rs           # Module declarations
├── rlox/
│   ├── scanner.rs    # Lexical analysis → Tokens
│   ├── parser.rs     # Recursive descent → AST (Expr, Statement)
│   ├── eval.rs       # Tree-walking interpreter → Value
│   ├── environment.rs # Variable storage (HashMap + parent_env)
│   ├── interpreter.rs # High-level driver (parse + eval loop)
│   └── token.rs      # Token types and Token struct
```

### Data Flow

```
Source Code → Scanner (Tokens) → Parser (AST) → Evaluator (Value) → Output
                    ↑                                    ↓
              Error Recovery ←────────────────────── Runtime Errors
```

## Test Strategy

- **461 tests** total across all modules
- Parser tests: AST structure verification (`as_binary`, `as_ternary`, `as_assignment` helpers)
- Evaluator tests: Value semantics, type errors, operator behavior
- Integration tests: Full parse+eval via `interpreter.rs`
- Run with: `cargo test` (all) or `cargo test <module>::<test>`

## Known Issues (Not To Fix Per Constraints)

1. **Parser: Assignment missing `=` consumption** (`parser.rs:200`)
   - `if self.check(&Equal)` → should `self.advance()` before parsing RHS
   - All 19 parser assignment tests fail

2. **Environment: No parent scope traversal** (`environment.rs`)
   - `get()` and `assign()` only check local `names` HashMap
   - 6 nested environment tests fail

3. **Assignment: Silent undefined variable** (`eval.rs:83`)
   - `env.assign()` returns `false` but evaluation continues
   - Should return `RuntimeError::IdentifierNotFound` per Lox spec

## Commands

```bash
# Build
cargo build

# Run all tests
cargo test

# Run specific test module
cargo test rlox::parser::tests::assignment
cargo test rlox::eval::tests::identifiers::assignment
cargo test rlox::eval::tests::identifiers::nested_environment

# Run REPL
cargo run

# Run file
cargo run -- path/to/script.lox

# Format
cargo fmt

# Clippy
cargo clippy
```

## Development Notes

- Tests are co-located in `#[cfg(test)]` modules within each source file
- Parser uses "panic mode" synchronization for error recovery
- Evaluator uses `Value` enum: `Number(f64)`, `Str(String)`, `Boolean(bool)`, `Nil`
- Type errors at runtime (not compile time) — Lox is dynamically typed
- Operator precedence: unary > factor > term > comparison > equality > ternary > assignment > comma

## References

- [Crafting Interpreters - Statements and State](https://craftinginterpreters.com/statements-and-state.html)
- [Crafting Interpreters - Control Flow](https://craftinginterpreters.com/control-flow.html)
- [Crafting Interpreters - Functions](https://craftinginterpreters.com/functions.html)