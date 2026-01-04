//! Serde integration for HUML format
//!
//! This module provides both serialization and deserialization support for HUML format
//! using the Serde framework. It allows you to serialize Rust structs to HUML text
//! and deserialize HUML text into Rust structs.
//!
//! # Examples
//!
//! ## Deserialization
//!
//! ```rust
//! use serde::Deserialize;
//! use huml_rs::serde::from_str;
//!
//! #[derive(Deserialize, Debug)]
//! struct Config {
//!     app_name: String,
//!     port: u16,
//!     debug: bool,
//!     features: Vec<String>,
//! }
//!
//! let huml = r#"
//! app_name: "My Application"
//! port: 8080
//! debug: true
//! features:: "auth", "logging", "metrics"
//! "#;
//!
//! let config: Config = from_str(huml).unwrap();
//! println!("{:?}", config);
//! ```
//!
//! ## Serialization
//!
//! ```rust
//! use serde::Serialize;
//! use huml_rs::serde::to_string;
//!
//! #[derive(Serialize)]
//! struct Config {
//!     app_name: String,
//!     port: u16,
//!     debug: bool,
//!     features: Vec<String>,
//! }
//!
//! let config = Config {
//!     app_name: "My Application".to_string(),
//!     port: 8080,
//!     debug: true,
//!     features: vec!["auth".to_string(), "logging".to_string()],
//! };
//!
//! let huml = to_string(&config).unwrap();
//! println!("{}", huml);
//! ```

pub mod de;
pub mod ser;

// Re-export common functions for convenience
pub use de::{from_str, Deserializer, Error as DeError};
pub use ser::{to_string, Error as SerError, Serializer};

pub use de::Result as DeResult;

/// Combined error type for both serialization and deserialization
#[derive(Debug)]
pub enum Error {
    /// Deserialization error
    De(de::Error),
    /// Serialization error
    Ser(ser::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::De(e) => write!(f, "Deserialization error: {e}"),
            Error::Ser(e) => write!(f, "Serialization error: {e}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::De(e) => Some(e),
            Error::Ser(e) => Some(e),
        }
    }
}

impl From<de::Error> for Error {
    fn from(err: de::Error) -> Self {
        Error::De(err)
    }
}

impl From<ser::Error> for Error {
    fn from(err: ser::Error) -> Self {
        Error::Ser(err)
    }
}

/// Convenience function to serialize a value to HUML and then deserialize it back
///
/// This is useful for testing round-trip serialization/deserialization.
pub fn round_trip<T>(value: &T) -> Result<T, Error>
where
    T: serde::Serialize + for<'de> serde::Deserialize<'de>,
{
    let huml = to_string(value)?;
    let result = from_str(&huml)?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    use std::collections::HashMap;

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct TestStruct {
        name: String,
        age: u32,
        active: bool,
        scores: Vec<i32>,
        metadata: HashMap<String, String>,
    }

    #[test]
    fn test_round_trip_serialization() {
        let mut metadata = HashMap::new();
        metadata.insert("role".to_string(), "admin".to_string());
        metadata.insert("department".to_string(), "engineering".to_string());

        let original = TestStruct {
            name: "Alice".to_string(),
            age: 30,
            active: true,
            scores: vec![85, 92, 78],
            metadata,
        };

        // Debug the serialization
        let serialized = to_string(&original).unwrap();
        println!("Serialized:\n{}", serialized);

        let deserialized: TestStruct = from_str(&serialized).unwrap();
        println!("Original metadata: {:?}", original.metadata);
        println!("Deserialized metadata: {:?}", deserialized.metadata);

        assert_eq!(original, deserialized);
    }

    #[test]
    fn test_serialization_then_manual_deserialization() {
        let original = TestStruct {
            name: "Bob".to_string(),
            age: 25,
            active: false,
            scores: vec![90, 88],
            metadata: HashMap::new(),
        };

        let huml = to_string(&original).unwrap();
        println!("Serialized HUML:\n{}", huml);

        let deserialized: TestStruct = from_str(&huml).unwrap();
        assert_eq!(original, deserialized);
    }
}
