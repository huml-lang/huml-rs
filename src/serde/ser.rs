//! Serde serializer implementation for HUML format
//!
//! This module provides a custom Serde serializer that allows users to serialize
//! Rust structs into HUML format using `#[derive(Serialize)]`.
//!
//! # Example
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
//! // Output:
//! // app_name: "My Application"
//! // port: 8080
//! // debug: true
//! // features:: "auth", "logging"
//! ```

use serde::ser::{self, Serialize};
use std::fmt;
use std::io;

/// Error type for HUML serialization
#[derive(Debug, Clone)]
pub enum Error {
    /// Custom error message
    Message(String),
    /// IO error during writing
    Io(String),
    /// Unsupported type
    UnsupportedType(&'static str),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Message(msg) => f.write_str(msg),
            Error::Io(msg) => write!(f, "IO error: {msg}"),
            Error::UnsupportedType(msg) => write!(f, "Unsupported type: {msg}"),
        }
    }
}

impl std::error::Error for Error {}

impl ser::Error for Error {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        Error::Message(msg.to_string())
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Error::Io(err.to_string())
    }
}

/// Result type for HUML serialization
pub type Result<T> = std::result::Result<T, Error>;

/// HUML serializer that writes to a string
pub struct Serializer {
    output: String,
    indent_level: usize,
}

impl Serializer {
    /// Create a new serializer
    pub fn new() -> Self {
        Self {
            output: String::new(),
            indent_level: 0,
        }
    }

    /// Get the current indentation string
    fn indent(&self) -> String {
        "  ".repeat(self.indent_level)
    }

    /// Write a newline
    fn newline(&mut self) {
        self.output.push('\n');
    }

    /// Increase indentation level
    fn increase_indent(&mut self) {
        self.indent_level += 1;
    }

    /// Decrease indentation level
    fn decrease_indent(&mut self) {
        if self.indent_level > 0 {
            self.indent_level -= 1;
        }
    }

    /// Write a string value with proper HUML escaping
    fn write_string(&mut self, s: &str) -> Result<()> {
        self.output.push('"');
        for ch in s.chars() {
            match ch {
                '"' => self.output.push_str("\\\""),
                '\\' => self.output.push_str("\\\\"),
                '\n' => self.output.push_str("\\n"),
                '\t' => self.output.push_str("\\t"),
                '\r' => self.output.push_str("\\r"),
                '\x08' => self.output.push_str("\\b"),
                '\x0C' => self.output.push_str("\\f"),
                '/' => self.output.push_str("\\/"),
                c if c.is_control() => {
                    self.output.push_str(&format!("\\u{:04x}", c as u32));
                }
                c => self.output.push(c),
            }
        }
        self.output.push('"');
        Ok(())
    }

    /// Finish serialization and return the result
    pub fn into_string(self) -> String {
        self.output
    }
}

impl Default for Serializer {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience function to serialize a value into a HUML string
pub fn to_string<T>(value: &T) -> Result<String>
where
    T: Serialize,
{
    let mut serializer = Serializer::new();
    value.serialize(&mut serializer)?;
    Ok(serializer.into_string())
}

impl<'a> ser::Serializer for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    type SerializeSeq = SeqSerializer<'a>;
    type SerializeTuple = SeqSerializer<'a>;
    type SerializeTupleStruct = SeqSerializer<'a>;
    type SerializeTupleVariant = TupleVariantSerializer<'a>;
    type SerializeMap = MapSerializer<'a>;
    type SerializeStruct = MapSerializer<'a>;
    type SerializeStructVariant = StructVariantSerializer<'a>;

    fn serialize_bool(self, v: bool) -> Result<()> {
        self.output.push_str(if v { "true" } else { "false" });
        Ok(())
    }

    fn serialize_i8(self, v: i8) -> Result<()> {
        self.serialize_i64(v as i64)
    }

    fn serialize_i16(self, v: i16) -> Result<()> {
        self.serialize_i64(v as i64)
    }

    fn serialize_i32(self, v: i32) -> Result<()> {
        self.serialize_i64(v as i64)
    }

    fn serialize_i64(self, v: i64) -> Result<()> {
        self.output.push_str(&v.to_string());
        Ok(())
    }

    fn serialize_u8(self, v: u8) -> Result<()> {
        self.serialize_u64(v as u64)
    }

    fn serialize_u16(self, v: u16) -> Result<()> {
        self.serialize_u64(v as u64)
    }

    fn serialize_u32(self, v: u32) -> Result<()> {
        self.serialize_u64(v as u64)
    }

    fn serialize_u64(self, v: u64) -> Result<()> {
        self.output.push_str(&v.to_string());
        Ok(())
    }

    fn serialize_f32(self, v: f32) -> Result<()> {
        self.serialize_f64(v as f64)
    }

    fn serialize_f64(self, v: f64) -> Result<()> {
        if v.is_nan() {
            self.output.push_str("nan");
        } else if v.is_infinite() {
            if v.is_sign_positive() {
                self.output.push_str("inf");
            } else {
                self.output.push_str("-inf");
            }
        } else {
            self.output.push_str(&v.to_string());
        }
        Ok(())
    }

    fn serialize_char(self, v: char) -> Result<()> {
        self.write_string(&v.to_string())
    }

    fn serialize_str(self, v: &str) -> Result<()> {
        self.write_string(v)
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<()> {
        use ser::SerializeSeq;
        let mut seq = self.serialize_seq(Some(v.len()))?;
        for byte in v {
            seq.serialize_element(byte)?;
        }
        seq.end()
    }

    fn serialize_none(self) -> Result<()> {
        self.serialize_unit()
    }

    fn serialize_some<T>(self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<()> {
        self.output.push_str("null");
        Ok(())
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<()> {
        self.serialize_unit()
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<()> {
        self.serialize_str(variant)
    }

    fn serialize_newtype_struct<T>(self, _name: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        self.output.push_str(variant);
        self.output.push_str(": ");
        value.serialize(self)?;
        Ok(())
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq> {
        if len == Some(0) {
            self.output.push_str("[]");
            Ok(SeqSerializer::empty(self))
        } else {
            Ok(SeqSerializer::new(self))
        }
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        self.output.push_str(variant);
        self.output.push_str(": ");
        Ok(TupleVariantSerializer::new(self))
    }

    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap> {
        if len == Some(0) {
            self.output.push_str("{}");
            Ok(MapSerializer::empty(self))
        } else {
            Ok(MapSerializer::new(self, false))
        }
    }

    fn serialize_struct(self, _name: &'static str, len: usize) -> Result<Self::SerializeStruct> {
        self.serialize_map(Some(len))
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        self.output.push_str(variant);
        self.output.push_str("::");
        self.output.push('\n');
        Ok(StructVariantSerializer::new(self))
    }
}

/// Serializer for sequences (lists, tuples)
pub struct SeqSerializer<'a> {
    ser: &'a mut Serializer,
    first: bool,
    empty: bool,
}

impl<'a> SeqSerializer<'a> {
    fn new(ser: &'a mut Serializer) -> Self {
        Self {
            ser,
            first: true,
            empty: false,
        }
    }

    fn empty(ser: &'a mut Serializer) -> Self {
        Self {
            ser,
            first: true,
            empty: true,
        }
    }
}

impl<'a> ser::SerializeSeq for SeqSerializer<'a> {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        if self.empty {
            return Ok(());
        }

        if self.first {
            self.first = false;
        } else {
            self.ser.output.push_str(", ");
        }

        value.serialize(&mut *self.ser)?;
        Ok(())
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl<'a> ser::SerializeTuple for SeqSerializer<'a> {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        ser::SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<()> {
        ser::SerializeSeq::end(self)
    }
}

impl<'a> ser::SerializeTupleStruct for SeqSerializer<'a> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        ser::SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<()> {
        ser::SerializeSeq::end(self)
    }
}

/// Serializer for tuple variants
pub struct TupleVariantSerializer<'a> {
    ser: &'a mut Serializer,
    first: bool,
}

impl<'a> TupleVariantSerializer<'a> {
    fn new(ser: &'a mut Serializer) -> Self {
        Self { ser, first: true }
    }
}

impl<'a> ser::SerializeTupleVariant for TupleVariantSerializer<'a> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        if self.first {
            self.first = false;
        } else {
            self.ser.output.push_str(", ");
        }
        value.serialize(&mut *self.ser)?;
        Ok(())
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

/// Serializer for maps and structs
pub struct MapSerializer<'a> {
    ser: &'a mut Serializer,
    first: bool,
    empty: bool,
    inline: bool,
}

impl<'a> MapSerializer<'a> {
    fn new(ser: &'a mut Serializer, inline: bool) -> Self {
        Self {
            ser,
            first: true,
            empty: false,
            inline,
        }
    }

    fn empty(ser: &'a mut Serializer) -> Self {
        Self {
            ser,
            first: true,
            empty: true,
            inline: false,
        }
    }
}

impl<'a> ser::SerializeMap for MapSerializer<'a> {
    type Ok = ();
    type Error = Error;

    fn serialize_key<T>(&mut self, key: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        if self.empty {
            return Ok(());
        }

        if self.first {
            self.first = false;
        } else if self.inline {
            self.ser.output.push_str(", ");
        } else {
            self.ser.newline();
        }

        if !self.inline {
            self.ser.output.push_str(&self.ser.indent());
        }

        // Serialize the key - for HUML, keys should be unquoted if possible
        let start_pos = self.ser.output.len();
        key.serialize(&mut *self.ser)?;

        // Check if we need to unquote the key (if it's a simple string)
        let key_str = self.ser.output[start_pos..].to_string();
        if key_str.starts_with('"') && key_str.ends_with('"') {
            let unquoted = &key_str[1..key_str.len() - 1];
            if is_valid_unquoted_key(unquoted) {
                // Replace the quoted key with unquoted version
                self.ser.output.truncate(start_pos);
                self.ser.output.push_str(unquoted);
            }
        }

        Ok(())
    }

    fn serialize_value<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        if self.empty {
            return Ok(());
        }

        // Check what kind of value we're serializing
        let start_pos = self.ser.output.len();

        // Serialize the value to see what it looks like
        let value_start = self.ser.output.len();
        value.serialize(&mut *self.ser)?;
        let value_str = self.ser.output[value_start..].to_string();

        // Determine if we need special HUML syntax
        if value_str.contains('\n') {
            // Multi-line value - use :: syntax
            self.ser.output.insert_str(start_pos, "::");
            self.ser.output.insert(start_pos + 2, '\n');
            // Re-indent all lines in the value
            let lines: Vec<&str> = value_str.lines().collect();
            if lines.len() > 1 {
                self.ser.output.truncate(value_start + 3); // Keep "::\n"
                self.ser.increase_indent();
                for (i, line) in lines.iter().enumerate() {
                    if i > 0 {
                        self.ser.newline();
                    }
                    if !line.trim().is_empty() {
                        self.ser.output.push_str(&self.ser.indent());
                        self.ser.output.push_str(line.trim());
                    }
                }
                self.ser.decrease_indent();
            }
        } else if value_str.contains(", ")
            && !value_str.starts_with('{')
            && !value_str.is_empty()
            && value_str != "[]"
            && value_str != "{}"
        {
            // Inline list - use :: syntax
            self.ser.output.insert_str(start_pos, ":: ");
        } else {
            // Regular scalar value - use : syntax
            self.ser.output.insert_str(start_pos, ": ");
        }

        Ok(())
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl<'a> ser::SerializeStruct for MapSerializer<'a> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        ser::SerializeMap::serialize_entry(self, key, value)
    }

    fn end(self) -> Result<()> {
        ser::SerializeMap::end(self)
    }
}

/// Serializer for struct variants
pub struct StructVariantSerializer<'a> {
    ser: &'a mut Serializer,
    first: bool,
}

impl<'a> StructVariantSerializer<'a> {
    fn new(ser: &'a mut Serializer) -> Self {
        ser.increase_indent();
        Self { ser, first: true }
    }
}

impl<'a> ser::SerializeStructVariant for StructVariantSerializer<'a> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        if self.first {
            self.first = false;
        } else {
            self.ser.newline();
        }

        self.ser.output.push_str(&self.ser.indent());
        self.ser.output.push_str(key);
        self.ser.output.push_str(": ");
        value.serialize(&mut *self.ser)?;
        Ok(())
    }

    fn end(self) -> Result<()> {
        self.ser.decrease_indent();
        Ok(())
    }
}

/// Check if a string can be used as an unquoted key in HUML
fn is_valid_unquoted_key(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }

    // First character must be alphabetic or underscore
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_alphabetic() || c == '_' => {}
        _ => return false,
    }

    // Remaining characters must be alphanumeric, underscore, or hyphen
    chars.all(|c| c.is_alphanumeric() || c == '_' || c == '-')
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Serialize;
    use std::collections::HashMap;

    #[derive(Serialize)]
    struct Person {
        name: String,
        age: u32,
        active: bool,
    }

    #[derive(Serialize)]
    struct PersonWithList {
        name: String,
        hobbies: Vec<String>,
    }

    #[derive(Serialize)]
    enum Status {
        Active,
        Inactive { reason: String },
        Pending(u32),
    }

    #[test]
    fn test_serialize_simple_struct() {
        let person = Person {
            name: "Alice".to_string(),
            age: 30,
            active: true,
        };

        let huml = to_string(&person).unwrap();
        println!("Serialized: {}", huml);

        // Should contain the fields
        assert!(huml.contains("name: \"Alice\""));
        assert!(huml.contains("age: 30"));
        assert!(huml.contains("active: true"));
    }

    #[test]
    fn test_serialize_with_list() {
        let person = PersonWithList {
            name: "Bob".to_string(),
            hobbies: vec!["reading".to_string(), "coding".to_string()],
        };

        let huml = to_string(&person).unwrap();
        println!("Serialized: {}", huml);

        assert!(huml.contains("name: \"Bob\""));
        assert!(huml.contains("hobbies:: \"reading\", \"coding\""));
    }

    #[test]
    fn test_serialize_enum_variants() {
        let active = Status::Active;
        let huml = to_string(&active).unwrap();
        assert_eq!(huml, "\"Active\"");

        let inactive = Status::Inactive {
            reason: "maintenance".to_string(),
        };
        let huml = to_string(&inactive).unwrap();
        assert!(huml.contains("Inactive::"));
        assert!(huml.contains("reason: \"maintenance\""));

        let pending = Status::Pending(42);
        let huml = to_string(&pending).unwrap();
        assert!(huml.contains("Pending: 42"));
    }

    #[test]
    fn test_serialize_primitive_types() {
        assert_eq!(to_string(&"hello").unwrap(), "\"hello\"");
        assert_eq!(to_string(&42).unwrap(), "42");
        assert_eq!(to_string(&3.14).unwrap(), "3.14");
        assert_eq!(to_string(&true).unwrap(), "true");
        assert_eq!(to_string(&false).unwrap(), "false");

        let empty_list: Vec<i32> = vec![];
        assert_eq!(to_string(&empty_list).unwrap(), "[]");

        let list = vec![1, 2, 3];
        assert_eq!(to_string(&list).unwrap(), "1, 2, 3");
    }

    #[test]
    fn test_serialize_special_numbers() {
        assert_eq!(to_string(&f64::NAN).unwrap(), "nan");
        assert_eq!(to_string(&f64::INFINITY).unwrap(), "inf");
        assert_eq!(to_string(&f64::NEG_INFINITY).unwrap(), "-inf");
    }

    #[test]
    fn test_serialize_empty_containers() {
        let empty_map: HashMap<String, String> = HashMap::new();
        assert_eq!(to_string(&empty_map).unwrap(), "{}");

        let empty_vec: Vec<String> = Vec::new();
        assert_eq!(to_string(&empty_vec).unwrap(), "[]");
    }

    #[test]
    fn test_unquoted_keys() {
        assert!(is_valid_unquoted_key("simple"));
        assert!(is_valid_unquoted_key("with_underscore"));
        assert!(is_valid_unquoted_key("with-hyphen"));
        assert!(is_valid_unquoted_key("_starts_with_underscore"));
        assert!(is_valid_unquoted_key("key123"));

        assert!(!is_valid_unquoted_key(""));
        assert!(!is_valid_unquoted_key("123key"));
        assert!(!is_valid_unquoted_key("with spaces"));
        assert!(!is_valid_unquoted_key("with.dot"));
        assert!(!is_valid_unquoted_key("with:colon"));
    }

    #[test]
    fn test_serialize_hashmap() {
        use std::collections::HashMap;
        let mut map = HashMap::new();
        map.insert("key1".to_string(), "value1".to_string());
        map.insert("key2".to_string(), "value2".to_string());

        let result = to_string(&map).unwrap();
        println!("HashMap serialized: {}", result);

        // Should contain both keys
        assert!(result.contains("key1"));
        assert!(result.contains("key2"));
        assert!(result.contains("value1"));
        assert!(result.contains("value2"));
    }

    #[test]
    fn test_canonical_huml_formatting() {
        #[derive(Serialize, serde::Deserialize)]
        struct NestedExample {
            name: String,
            scores: Vec<i32>,
            config: InnerConfig,
        }

        #[derive(Serialize, serde::Deserialize)]
        struct InnerConfig {
            enabled: bool,
            timeout: u32,
        }

        let data = NestedExample {
            name: "test".to_string(),
            scores: vec![1, 2, 3],
            config: InnerConfig {
                enabled: true,
                timeout: 30,
            },
        };

        let huml = to_string(&data).unwrap();

        println!("=== CANONICAL HUML FORMAT ===");
        println!("{}", huml);

        // Should be parseable and round-trip correctly
        let result: NestedExample = crate::serde::from_str(&huml).unwrap();
        assert_eq!(result.name, "test");
        assert_eq!(result.scores, vec![1, 2, 3]);
        assert_eq!(result.config.enabled, true);
        assert_eq!(result.config.timeout, 30);

        // Should use proper HUML formatting with :: syntax and indentation
        assert!(huml.contains("scores:: "));
        assert!(huml.contains("config::\n"));
        assert!(huml.contains("  enabled: true"));
        assert!(huml.contains("  timeout: 30"));
    }
}
