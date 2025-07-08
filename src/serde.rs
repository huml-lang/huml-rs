//! Serde deserializer implementation for HUML format
//!
//! This module provides a custom Serde deserializer that allows users to deserialize
//! HUML text directly into their own Rust structs using `#[derive(Deserialize)]`.
//!
//! # Example
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
//! # Supported HUML Features
//!
//! The deserializer supports all standard HUML data types:
//! - **Scalars**: strings, numbers, booleans, null
//! - **Lists**: inline (`item1, item2, item3`) and empty (`[]`)
//! - **Dicts**: inline (`key: value, key2: value2`) and empty (`{}`)
//! - **Nested structures**: using proper HUML indentation
//! - **Enums**: unit variants, struct variants, and tuple variants

use crate::{HumlNumber, HumlValue, parse_huml};
use serde::de::{self, Deserialize, DeserializeSeed, Visitor};
use std::fmt;

/// Error type for HUML deserialization
#[derive(Debug, Clone)]
pub enum Error {
    /// Custom error message
    Message(String),
    /// Parse error from the underlying HUML parser
    ParseError(String),
    /// Type conversion error
    InvalidType(String),
    /// Missing field error
    MissingField(String),
    /// Unknown field error
    UnknownField(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Message(msg) => write!(f, "{}", msg),
            Error::ParseError(msg) => write!(f, "Parse error: {}", msg),
            Error::InvalidType(msg) => write!(f, "Invalid type: {}", msg),
            Error::MissingField(field) => write!(f, "Missing field: {}", field),
            Error::UnknownField(field) => write!(f, "Unknown field: {}", field),
        }
    }
}

impl std::error::Error for Error {}

impl de::Error for Error {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        Error::Message(msg.to_string())
    }
}

/// Result type for HUML deserialization
pub type Result<T> = std::result::Result<T, Error>;

/// HUML deserializer
pub struct Deserializer {
    value: HumlValue,
}

impl Deserializer {
    /// Create a new deserializer from a HUML value
    pub fn new(value: HumlValue) -> Self {
        Self { value }
    }

    /// Create a deserializer from HUML text
    pub fn from_str(input: &str) -> Result<Self> {
        let trimmed = input.trim();
        // Strategy: try parsing approaches in order of specificity

        // 1. First try individual value types (scalars, inline lists/dicts)
        if let Ok(value) = Self::parse_value(trimmed) {
            return Ok(value);
        }

        // 2. Then try to parse as document root (handles multiline content)
        match crate::parse_document_root(trimmed) {
            Ok(("", root)) => return Ok(Self::new(root)),
            Ok((remaining, root)) => {
                // If there's remaining input, check if it's just whitespace
                if remaining.trim().is_empty() {
                    return Ok(Self::new(root));
                }
                // Otherwise continue to next approach
            }
            Err(_) => {
                // Continue to next approach
            }
        }

        // 3. Finally try to parse as a complete document
        match parse_huml(trimmed) {
            Ok(("", document)) => Ok(Self::new(document.root)),
            Ok((remaining, document)) => {
                // If there's remaining input, check if it's just whitespace
                if remaining.trim().is_empty() {
                    Ok(Self::new(document.root))
                } else {
                    Err(Error::ParseError(format!(
                        "Unexpected remaining input: {}",
                        remaining
                    )))
                }
            }
            Err(_) => Err(Error::ParseError(format!(
                "Unable to parse HUML content: {}",
                trimmed
            ))),
        }
    }

    /// Parse individual value types (scalars, lists, inline dicts)
    fn parse_value(input: &str) -> Result<Self> {
        // Try parsing as scalar first
        if let Ok(("", value)) = crate::parse_scalar(input) {
            return Ok(Self::new(value));
        }

        // Try parsing as inline list
        if let Ok(("", value)) = crate::parse_inline_list(input) {
            return Ok(Self::new(value));
        }

        // Try parsing as inline dict
        if let Ok(("", value)) = crate::parse_inline_dict(input) {
            return Ok(Self::new(value));
        }

        // Try parsing as empty list
        if let Ok(("", value)) = crate::parse_empty_list(input) {
            return Ok(Self::new(value));
        }

        // Try parsing as empty dict
        if let Ok(("", value)) = crate::parse_empty_dict(input) {
            return Ok(Self::new(value));
        }

        Err(Error::ParseError(format!(
            "Unable to parse value: {}",
            input
        )))
    }
}

/// Convenience function to deserialize HUML text into a type
///
/// This is the main entry point for deserializing HUML text into Rust types.
/// The type must implement `serde::Deserialize`.
///
/// # Example
///
/// ```rust
/// use serde::Deserialize;
/// use huml_rs::serde::from_str;
///
/// #[derive(Deserialize)]
/// struct Person {
///     name: String,
///     age: u32,
/// }
///
/// let huml = r#"
/// name: "Alice"
/// age: 30
/// "#;
///
/// let person: Person = from_str(huml).unwrap();
/// ```
pub fn from_str<'a, T>(input: &'a str) -> Result<T>
where
    T: Deserialize<'a>,
{
    let deserializer = Deserializer::from_str(input)?;
    T::deserialize(deserializer)
}

impl<'de> de::Deserializer<'de> for Deserializer {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.value {
            HumlValue::String(s) => visitor.visit_string(s),
            HumlValue::Number(n) => match n {
                HumlNumber::Integer(i) => visitor.visit_i64(i),
                HumlNumber::Float(f) => visitor.visit_f64(f),
                HumlNumber::Nan => visitor.visit_f64(f64::NAN),
                HumlNumber::Infinity(positive) => {
                    if positive {
                        visitor.visit_f64(f64::INFINITY)
                    } else {
                        visitor.visit_f64(f64::NEG_INFINITY)
                    }
                }
            },
            HumlValue::Boolean(b) => visitor.visit_bool(b),
            HumlValue::Null => visitor.visit_unit(),
            HumlValue::List(list) => {
                let seq = SeqDeserializer::new(list);
                visitor.visit_seq(seq)
            }
            HumlValue::Dict(dict) => {
                let map = MapDeserializer::new(dict);
                visitor.visit_map(map)
            }
        }
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.value {
            HumlValue::Boolean(b) => visitor.visit_bool(b),
            _ => Err(Error::InvalidType("Expected boolean".to_string())),
        }
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_i64(visitor)
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_i64(visitor)
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_i64(visitor)
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.value {
            HumlValue::Number(HumlNumber::Integer(i)) => visitor.visit_i64(i),
            HumlValue::Number(HumlNumber::Float(f)) => visitor.visit_i64(f as i64),
            _ => Err(Error::InvalidType("Expected integer".to_string())),
        }
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_u64(visitor)
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_u64(visitor)
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_u64(visitor)
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.value {
            HumlValue::Number(HumlNumber::Integer(i)) => {
                if i >= 0 {
                    visitor.visit_u64(i as u64)
                } else {
                    Err(Error::InvalidType("Expected positive integer".to_string()))
                }
            }
            HumlValue::Number(HumlNumber::Float(f)) => {
                if f >= 0.0 {
                    visitor.visit_u64(f as u64)
                } else {
                    Err(Error::InvalidType("Expected positive number".to_string()))
                }
            }
            _ => Err(Error::InvalidType("Expected unsigned integer".to_string())),
        }
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_f64(visitor)
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.value {
            HumlValue::Number(HumlNumber::Float(f)) => visitor.visit_f64(f),
            HumlValue::Number(HumlNumber::Integer(i)) => visitor.visit_f64(i as f64),
            HumlValue::Number(HumlNumber::Nan) => visitor.visit_f64(f64::NAN),
            HumlValue::Number(HumlNumber::Infinity(positive)) => {
                if positive {
                    visitor.visit_f64(f64::INFINITY)
                } else {
                    visitor.visit_f64(f64::NEG_INFINITY)
                }
            }
            _ => Err(Error::InvalidType("Expected float".to_string())),
        }
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.value {
            HumlValue::String(s) => {
                let mut chars = s.chars();
                match (chars.next(), chars.next()) {
                    (Some(c), None) => visitor.visit_char(c),
                    _ => Err(Error::InvalidType("Expected single character".to_string())),
                }
            }
            _ => Err(Error::InvalidType("Expected string".to_string())),
        }
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.value {
            HumlValue::String(s) => visitor.visit_string(s),
            _ => Err(Error::InvalidType("Expected string".to_string())),
        }
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.value {
            HumlValue::String(s) => visitor.visit_byte_buf(s.into_bytes()),
            _ => Err(Error::InvalidType("Expected string".to_string())),
        }
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_bytes(visitor)
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.value {
            HumlValue::Null => visitor.visit_none(),
            _ => visitor.visit_some(self),
        }
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.value {
            HumlValue::Null => visitor.visit_unit(),
            _ => Err(Error::InvalidType("Expected null".to_string())),
        }
    }

    fn deserialize_unit_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_unit(visitor)
    }

    fn deserialize_newtype_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.value {
            HumlValue::List(list) => {
                let seq = SeqDeserializer::new(list);
                visitor.visit_seq(seq)
            }
            _ => Err(Error::InvalidType("Expected list".to_string())),
        }
    }

    fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.value {
            HumlValue::Dict(dict) => {
                let map = MapDeserializer::new(dict);
                visitor.visit_map(map)
            }
            _ => Err(Error::InvalidType("Expected dict".to_string())),
        }
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_map(visitor)
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.value {
            HumlValue::String(s) => {
                visitor.visit_enum(serde::de::value::StringDeserializer::<Error>::new(s))
            }
            HumlValue::Dict(dict) => {
                if dict.len() == 1 {
                    let (key, value) = dict.into_iter().next().unwrap();
                    visitor.visit_enum(EnumDeserializer::new(key, value))
                } else {
                    Err(Error::InvalidType(
                        "Expected single-key dict for enum".to_string(),
                    ))
                }
            }
            _ => Err(Error::InvalidType(
                "Expected string or dict for enum".to_string(),
            )),
        }
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }
}

/// Sequence deserializer for HUML lists
struct SeqDeserializer {
    iter: std::vec::IntoIter<HumlValue>,
}

impl SeqDeserializer {
    fn new(list: Vec<HumlValue>) -> Self {
        Self {
            iter: list.into_iter(),
        }
    }
}

impl<'de> de::SeqAccess<'de> for SeqDeserializer {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
    where
        T: DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some(value) => {
                let deserializer = Deserializer::new(value);
                seed.deserialize(deserializer).map(Some)
            }
            None => Ok(None),
        }
    }
}

/// Map deserializer for HUML dicts
struct MapDeserializer {
    iter: std::collections::hash_map::IntoIter<String, HumlValue>,
    value: Option<HumlValue>,
}

impl MapDeserializer {
    fn new(dict: std::collections::HashMap<String, HumlValue>) -> Self {
        Self {
            iter: dict.into_iter(),
            value: None,
        }
    }
}

impl<'de> de::MapAccess<'de> for MapDeserializer {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some((key, value)) => {
                self.value = Some(value);
                let key_deserializer = Deserializer::new(HumlValue::String(key));
                seed.deserialize(key_deserializer).map(Some)
            }
            None => Ok(None),
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: DeserializeSeed<'de>,
    {
        match self.value.take() {
            Some(value) => {
                let deserializer = Deserializer::new(value);
                seed.deserialize(deserializer)
            }
            None => Err(Error::InvalidType("Value is missing".to_string())),
        }
    }
}

/// Enum deserializer for HUML enums
struct EnumDeserializer {
    variant: String,
    value: HumlValue,
}

impl EnumDeserializer {
    fn new(variant: String, value: HumlValue) -> Self {
        Self { variant, value }
    }
}

impl<'de> de::EnumAccess<'de> for EnumDeserializer {
    type Error = Error;
    type Variant = VariantDeserializer;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant)>
    where
        V: DeserializeSeed<'de>,
    {
        let variant_deserializer = Deserializer::new(HumlValue::String(self.variant));
        let variant = seed.deserialize(variant_deserializer)?;
        Ok((variant, VariantDeserializer::new(self.value)))
    }
}

/// Variant deserializer for HUML enum variants
struct VariantDeserializer {
    value: HumlValue,
}

impl VariantDeserializer {
    fn new(value: HumlValue) -> Self {
        Self { value }
    }
}

impl<'de> de::VariantAccess<'de> for VariantDeserializer {
    type Error = Error;

    fn unit_variant(self) -> Result<()> {
        match self.value {
            HumlValue::Null => Ok(()),
            _ => Err(Error::InvalidType(
                "Expected null for unit variant".to_string(),
            )),
        }
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value>
    where
        T: DeserializeSeed<'de>,
    {
        let deserializer = Deserializer::new(self.value);
        seed.deserialize(deserializer)
    }

    fn tuple_variant<V>(self, _len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.value {
            HumlValue::List(list) => {
                let seq = SeqDeserializer::new(list);
                visitor.visit_seq(seq)
            }
            _ => Err(Error::InvalidType(
                "Expected list for tuple variant".to_string(),
            )),
        }
    }

    fn struct_variant<V>(self, _fields: &'static [&'static str], visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.value {
            HumlValue::Dict(dict) => {
                let map = MapDeserializer::new(dict);
                visitor.visit_map(map)
            }
            _ => Err(Error::InvalidType(
                "Expected dict for struct variant".to_string(),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;
    use std::collections::HashMap;

    #[derive(Debug, Deserialize, PartialEq)]
    struct Person {
        name: String,
        age: u32,
        active: bool,
    }

    #[derive(Debug, Deserialize, PartialEq)]
    struct PersonWithOptional {
        name: String,
        age: Option<u32>,
        email: Option<String>,
    }

    #[derive(Debug, Deserialize, PartialEq)]
    struct PersonWithList {
        name: String,
        hobbies: Vec<String>,
    }

    #[derive(Debug, Deserialize, PartialEq)]
    struct Nested {
        person: Person,
        metadata: HashMap<String, String>,
    }

    #[derive(Debug, Deserialize, PartialEq)]
    enum Status {
        Active,
        Inactive { reason: String },
        Pending(u32),
    }

    #[test]
    fn test_deserialize_simple_struct() {
        let huml = r#"
name: "Alice"
age: 30
active: true
"#;

        let person: Person = from_str(huml).unwrap();
        assert_eq!(
            person,
            Person {
                name: "Alice".to_string(),
                age: 30,
                active: true,
            }
        );
    }

    #[test]
    fn test_deserialize_with_optional() {
        let huml = r#"
name: "Bob"
age: 25
"#;

        let person: PersonWithOptional = from_str(huml).unwrap();
        assert_eq!(
            person,
            PersonWithOptional {
                name: "Bob".to_string(),
                age: Some(25),
                email: None,
            }
        );
    }

    #[test]
    fn test_deserialize_with_list() {
        let huml = r#"
name: "Charlie"
hobbies:: "reading", "coding", "gaming"
"#;

        let person: PersonWithList = from_str(huml).unwrap();
        assert_eq!(
            person,
            PersonWithList {
                name: "Charlie".to_string(),
                hobbies: vec![
                    "reading".to_string(),
                    "coding".to_string(),
                    "gaming".to_string()
                ],
            }
        );
    }

    #[test]
    fn test_deserialize_nested() {
        let huml = r#"
person:: name: "David", age: 35, active: false
metadata:: role: "admin", department: "engineering"
"#;

        let nested: Nested = from_str(huml).unwrap();
        let mut expected_metadata = HashMap::new();
        expected_metadata.insert("role".to_string(), "admin".to_string());
        expected_metadata.insert("department".to_string(), "engineering".to_string());

        assert_eq!(
            nested,
            Nested {
                person: Person {
                    name: "David".to_string(),
                    age: 35,
                    active: false,
                },
                metadata: expected_metadata,
            }
        );
    }

    #[test]
    fn test_deserialize_enum_unit_variant() {
        let huml = r#""Active""#;
        let status: Status = from_str(huml).unwrap();
        assert_eq!(status, Status::Active);
    }

    #[test]
    fn test_deserialize_enum_struct_variant() {
        let huml = r#"
Inactive:: reason: "maintenance"
"#;
        let status: Status = from_str(huml).unwrap();
        assert_eq!(
            status,
            Status::Inactive {
                reason: "maintenance".to_string()
            }
        );
    }

    #[test]
    fn test_deserialize_enum_tuple_variant() {
        let huml = r#"
Pending: 42
"#;
        let status: Status = from_str(huml).unwrap();
        assert_eq!(status, Status::Pending(42));
    }

    #[test]
    fn test_deserialize_primitive_types() {
        // Test string
        let s: String = from_str(r#""hello""#).unwrap();
        assert_eq!(s, "hello");

        // Test integer
        let i: i32 = from_str("42").unwrap();
        assert_eq!(i, 42);

        // Test float
        let f: f64 = from_str("3.14").unwrap();
        assert_eq!(f, 3.14);

        // Test boolean
        let b: bool = from_str("true").unwrap();
        assert_eq!(b, true);

        // Test list
        let list: Vec<i32> = from_str("1, 2, 3").unwrap();
        assert_eq!(list, vec![1, 2, 3]);
    }

    #[test]
    fn test_deserialize_error_cases() {
        // Test invalid type
        let result: Result<i32> = from_str(r#""not a number""#);
        assert!(result.is_err());

        // Test missing field
        let result: Result<Person> = from_str(r#"name: "Alice""#);
        assert!(result.is_err());

        // Test parse error
        let result: Result<Person> = from_str(r#"invalid huml syntax {"#);
        assert!(result.is_err());
    }

    #[test]
    fn test_serde_integration_example() {
        // Example demonstrating the serde deserializer in action
        #[derive(Debug, Deserialize, PartialEq)]
        struct Config {
            app_name: String,
            port: u16,
            debug: bool,
            features: Vec<String>,
        }

        let huml = r#"
app_name: "My App"
port: 8080
debug: true
features:: "auth", "logging", "metrics"
"#;

        let config: Config = from_str(huml).unwrap();

        assert_eq!(config.app_name, "My App");
        assert_eq!(config.port, 8080);
        assert_eq!(config.debug, true);
        assert_eq!(config.features, vec!["auth", "logging", "metrics"]);
    }
}
