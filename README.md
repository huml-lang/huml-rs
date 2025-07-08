# HUML Parser for Rust (Experimental)

This is an **experimental** Rust parser for the [HUML (Human-Usable Markup Language)](https://huml.pages.dev) data serialization format. It is built using the powerful `nom` parser-combinator library.

## Specification

This parser aims to implement the [HUML specification v0.1.0](https://huml.pages.dev/specifications/v0-1-0/).

## Features

*   **Version Declaration:** Parses the optional `%HUML v...` header.
*   **Comments:** Supports `#` prefixed comments.
*   **Data Types:**
    *   **Strings:**
        *   Single-quoted: `"Hello, World!"`
        *   Multi-line (preserving whitespace): ```` ``...`` ````
        *   Multi-line (stripping whitespace): `"""..."""`
    *   **Numbers:**
        *   Integers: `42`, `-1_000`
        *   Floats: `3.14`, `1.23e10`
        *   Hexadecimal: `0xFF`
        *   Octal: `0o755`
        *   Binary: `0b1010`
        *   Special values: `inf`, `-inf`, `nan`
    *   **Booleans:** `true`, `false`
    *   **Null:** `null`
*   **Collections:**
    *   **Dictionaries (Maps):**
        *   Inline: `key: "value", another: 123`
        *   Multi-line:
            ```huml
            key: "value"
            another: 123
            ```
    *   **Lists:**
        *   Inline: `1, 2, "three"`
        *   Multi-line:
            ```huml
            - item1
            - item2
            ```
*   **Nested Structures:** Supports deeply nested dictionaries and lists.
*   **Complex Keys:** Keys can be unquoted, or quoted to include spaces and special characters.
*   **Serde Support:** Deserialize HUML directly into Rust structs.

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
huml-rs = "0.1.0" # Replace with the desired version
serde = { version = "1.0", features = ["derive"] }
```

### Low-level Parsing

Then, you can parse a HUML string like this:

```rust
use huml_rs::parse_huml;

fn main() {
    let huml_string = r#"
%HUML v0.1.0

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

### Deserializing with Serde

You can also deserialize HUML into your own Rust structs using `serde`.

```rust
use huml_rs::serde::from_str;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
struct Config {
    app_name: String,
    version: String,
    debug_mode: bool,
}

fn main() {
    let huml_string = r#"
app_name: "My Awesome App"
version: "1.0"
debug_mode: true
    "#;

    match from_str::<Config>(huml_string) {
        Ok(config) => {
            println!("Successfully deserialized config: {:?}", config);
        }
        Err(e) => {
            eprintln!("Failed to deserialize HUML: {}", e);
        }
    }
}
```

## Development

This project is built with Rust and `nom`.

To build the project:
```sh
cargo build
```

To run tests:
```sh
cargo test
```

## Contributing

Contributions are welcome! As this is an experimental project, there is much to do. Please feel free to open an issue or submit a pull request.
