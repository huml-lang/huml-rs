This is a parser for HUML.

The specification is available at https://huml.pages.dev/specifications/v0-1-0/

# AGENTS.md - Development Guidelines for huml-rs

## Build/Test Commands
- **Build**: `cargo build`
- **Test all**: `cargo test --verbose`
- **Test single**: `cargo test test_name`
- **Benchmarks**: `cargo bench` or `./scripts/bench.sh`
- **Examples**: `cargo run --example serde_example`
- **Lint**: `cargo clippy`
- **Format**: `cargo fmt`

## Code Style Guidelines
- **Edition**: Rust 2024
- **Dependencies**: Nom 8 for parsing, Serde for serialization
- **Imports**: Group std imports first, then external crates, then local modules
- **Naming**: snake_case for functions/variables, PascalCase for types/enums
- **Error handling**: Use Result<T, Error> pattern, custom error types with Display/Error traits
- **Documentation**: Use `///` for public APIs, include examples in doc comments
- **Testing**: Comprehensive unit tests in `#[cfg(test)]` modules, use descriptive test names
- **Performance**: Pre-allocate collections with capacity hints, use `&str` over `String` where possible

## Project Structure
- `src/lib.rs`: Main parser implementation using Nom combinators
- `src/serde.rs`: Serde deserializer for direct struct deserialization
- `examples/`: Usage examples demonstrating parser features
- `benches/`: Performance benchmarks using Criterion
- `test.huml`: Kitchen sink test file for comprehensive parsing validation

Use context7 for any docs.
