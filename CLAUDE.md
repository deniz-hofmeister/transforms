# Commands

- Build: `cargo build` (with std) or `cargo build --no-default-features` (no_std)
- Test: `cargo test` or `cargo test --no-default-features`
- Single test: `cargo test test_name` 
- Run example: `cargo run --example std_full` or `cargo run --example no_std_minimal --no-default-features`
- Benchmark: `cargo bench`
- Format: `cargo fmt`
- Lint: `cargo clippy -- -D warnings`

# Code Style

- **Formatting**: Follow rustfmt.toml settings (vertical function params, crate-level imports)
- **Imports**: Group by crate with `imports_granularity = "Crate"`
- **Naming**: Use snake_case for functions/variables, CamelCase for types
- **Safety**: No unsafe code is allowed (`#![forbid(unsafe_code)]`)
- **Dependencies**: Minimize external dependencies, support no_std
- **Error Handling**: Use thiserror for error types, never panic in library code
- **Documentation**: Document all public APIs with examples
- **Testing**: Write unit tests for each module, use approx for float comparisons