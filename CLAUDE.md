This is a parser for HUML.

The specification is available at https://huml.io/specifications/v0-1-0/

# AGENTS.md - Development Guidelines for huml-rs

## Build/Test Commands
- **Build**: `cargo build`
- **Test all**: `cargo test --verbose`
- **Test single**: `cargo test test_name`
- **Standard tests**: `./scripts/standard_tests.sh` or `cargo test standard_tests`
- **Benchmarks**: `cargo bench` or `./scripts/bench.sh`
- **Examples**: `cargo run --example serde_example` (demonstrates both serialization and deserialization)
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
- `src/serde/mod.rs`: Unified serde module with both serialization and deserialization
- `src/serde/de.rs`: Serde deserializer for HUML → Rust struct conversion
- `src/serde/ser.rs`: Serde serializer for Rust struct → HUML conversion
- `src/standard_tests.rs`: Standard HUML test suite integration
- `examples/`: Usage examples demonstrating parser features
- `benches/`: Performance benchmarks using Criterion
- `test.huml`: Kitchen sink test file for comprehensive parsing validation
- `tests/` (submodule): Official HUML test suite with 174+ assertion tests and document validation
- `scripts/standard_tests.sh`: Script to run standard tests with submodule initialization

## Serde Integration
The project provides full bidirectional serde support:

**Deserialization** (`huml_rs::serde::from_str`):
- Parse HUML text directly into Rust structs using `#[derive(Deserialize)]`
- Supports all HUML data types: scalars, lists, dicts, nested structures
- Handles enums (unit, tuple, and struct variants)
- Comprehensive error handling with descriptive messages

**Serialization** (`huml_rs::serde::to_string`):
- Convert Rust structs to canonical HUML format using `#[derive(Serialize)]`
- Produces human-readable, properly indented HUML
- Uses HUML vector syntax (`::`) for lists and nested structures
- Automatically unquotes simple keys (alphanumeric + underscore/hyphen)
- Perfect round-trip fidelity (serialize → deserialize → identical struct)

**Key Features**:
- Single canonical format (no "compact" mode - HUML has one format)
- Proper HUML syntax with `::` for vectors and nested structures
- Clean indentation and formatting following HUML specification
- Full compatibility with existing deserializer

## Standard Tests
The project includes the official HUML test suite as a git submodule. These centrally maintained tests ensure compatibility across HUML parser implementations.

**Current Status**:
- ✅ Both Document parsing and assertions test passes.

Failing tests reveal strict validation requirements like trailing whitespace detection, comment formatting rules, and precise indentation validation that need implementation.

Use context7 for any docs.
