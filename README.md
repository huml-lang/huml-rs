# HUML Parser for Rust

This is a Rust library for the [HUML (Human-Usable Markup Language)](https://huml.io/) data serialization format.

## Features
*   **Serde Support:** Full bidirectional support - serialize Rust structs to HUML and deserialize HUML into Rust structs.
*   **Fully compliant with HUML specification**
    *   Supports all HUML data types (string, number, boolean, array, object)
    *   Handles comments and whitespace correctly

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
huml-rs = "0.2.0"
serde = { version = "1.0", features = ["derive"] }
```

You can find the crate on [crates.io](https://crates.io/crates/huml-rs).

### Low-level Parsing

Then, you can parse a HUML string like this:

```rust
use huml_rs::parse_huml;

fn main() {
    let huml_string = r#"
%HUML v0.2.0

app_name: "My Awesome App"
version: "1.0"
debug_mode: true
    "#;

    match parse_huml(huml_string) {
        Ok((remaining, document)) => {
            if !remaining.trim().is_empty() {
                eprintln!("Warning: Unparsed input remains: {}", remaining);
            }
            println!("Successfully parsed HUML document!");
            println!("Version: {:?}", document.version);
            println!("Root value: {:?}", document.root);
        }
        Err(e) => {
            eprintln!("Failed to parse HUML: {:?}", e);
        }
    }
}
```

### Serde Integration (Serialization & Deserialization)

HUML-rs provides full bidirectional serde support for seamless integration with Rust structs.

#### Deserializing HUML into Rust Structs

```rust
use huml_rs::serde::from_str;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
struct Config {
    app_name: String,
    port: u16,
    debug_mode: bool,
    features: Vec<String>,
    database: DatabaseConfig,
}

#[derive(Deserialize, Debug)]
struct DatabaseConfig {
    host: String,
    port: u16,
    ssl: bool,
}

fn main() {
    let huml_string = r#"
app_name: "My Awesome App"
port: 8080
debug_mode: true
features:: "auth", "logging", "metrics"
database::
  host: "localhost"
  port: 5432
  ssl: true
    "#;

    match from_str::<Config>(huml_string) {
        Ok(config) => {
            println!("Successfully deserialized config: {:#?}", config);
        }
        Err(e) => {
            eprintln!("Failed to deserialize HUML: {}", e);
        }
    }
}
```

#### Serializing Rust Structs to HUML

```rust
use huml_rs::serde::to_string;
use serde::Serialize;

#[derive(Serialize)]
struct Config {
    app_name: String,
    port: u16,
    debug_mode: bool,
    features: Vec<String>,
    database: DatabaseConfig,
}

#[derive(Serialize)]
struct DatabaseConfig {
    host: String,
    port: u16,
    ssl: bool,
}

fn main() {
    let config = Config {
        app_name: "My Awesome App".to_string(),
        port: 8080,
        debug_mode: true,
        features: vec!["auth".to_string(), "logging".to_string(), "metrics".to_string()],
        database: DatabaseConfig {
            host: "localhost".to_string(),
            port: 5432,
            ssl: true,
        },
    };

    match to_string(&config) {
        Ok(huml) => {
            println!("Serialized HUML:\n{}", huml);
            // Output:
            // app_name: "My Awesome App"
            // port: 8080
            // debug_mode: true
            // features:: "auth", "logging", "metrics"
            // database::
            //   host: "localhost"
            //   port: 5432
            //   ssl: true
        }
        Err(e) => {
            eprintln!("Failed to serialize to HUML: {}", e);
        }
    }
}
```

#### Round-trip Serialization

HUML maintains perfect round-trip fidelity:

```rust
use huml_rs::serde::{to_string, from_str};
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct Data {
    name: String,
    values: Vec<i32>,
}

let original = Data {
    name: "test".to_string(),
    values: vec![1, 2, 3],
};

// Serialize to HUML
let huml = to_string(&original).unwrap();

// Deserialize back to struct
let restored: Data = from_str(&huml).unwrap();

assert_eq!(original, restored); // Perfect round-trip!
```

## Development

This project is built with Rust using a hand-written recursive descent parser inspired by Go's parsing style.

To build the project:
```sh
cargo build
```

To run tests:
```sh
cargo test
```

### Standard HUML Test Suite

This parser includes the official HUML test suite as a git submodule, which contains centrally maintained test cases that all HUML parser implementations should pass. These tests help ensure compatibility and correctness across different parsers.

To initialize the test submodule:
```sh
git submodule init
git submodule update
```

The standard tests include:
- **Assertion Tests**: 174+ test cases covering valid and invalid HUML syntax
- **Document Tests**: Complete HUML documents with expected JSON output for validation

To run only the standard tests:
```sh
cargo test standard_tests
```

**Current Status**:
- ✅ All document parsing tests pass
- ✅ All assertion tests pass (174+ test cases)

To run benchmarks:
```sh
cargo bench
```

## Benchmarking

This project includes comprehensive benchmarks using [Criterion.rs](https://github.com/bheisler/criterion.rs) to measure parsing performance across different scenarios:

### Benchmark Categories

- **Full Document Parsing**: Measures performance parsing the complete `test.huml` file
- **Component Parsing**: Tests individual parsing functions (strings, numbers, booleans, etc.)
- **Collection Parsing**: Benchmarks inline and multiline lists/dictionaries
- **Multiline Strings**: Tests multiline string parsing with preserved formatting
- **Document Sizes**: Compares performance across small, medium, and large documents
- **Edge Cases**: Tests long strings, deep nesting, and large collections
- **Memory Usage**: Measures allocation patterns and repeated parsing

### Running Benchmarks

```sh
# Run all benchmarks (HTML reports automatically generated)
cargo bench

# Run specific benchmark group
cargo bench parse_components

# View HTML reports in target/criterion/reports/index.html
```

## Contributing

Contributions are welcome! Please feel free to open an issue or submit a pull request.
