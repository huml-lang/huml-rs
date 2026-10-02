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

/// Convenience function to serialize a value into a HUML string
///
/// The value is first collected into an ordered tree, then written out as HUML,
/// so the syntax for each value (`:` or `::`, inline or block) is chosen from
/// its type rather than from the text it produces.
pub fn to_string<T>(value: &T) -> Result<String>
where
    T: ?Sized + Serialize,
{
    let value = value.serialize(ValueSerializer)?;
    let mut output = String::new();
    write_document(&mut output, &value);
    Ok(output)
}

/// Intermediate tree built from serde calls. Maps keep insertion order.
enum Value {
    Null,
    Bool(bool),
    Int(i64),
    UInt(u64),
    Float(f64),
    Str(String),
    Seq(Vec<Value>),
    Map(Vec<(String, Value)>),
}

impl Value {
    fn is_scalar(&self) -> bool {
        !matches!(self, Value::Seq(_) | Value::Map(_))
    }
}

/// Write the document root.
fn write_document(out: &mut String, value: &Value) {
    match value {
        // A single-item inline list at the root would read back as a scalar.
        Value::Seq(items) if items.len() > 1 && items.iter().all(Value::is_scalar) => {
            write_inline_list(out, items)
        }
        Value::Seq(items) if !items.is_empty() => write_list_block(out, items, 0),
        Value::Map(entries) if !entries.is_empty() => write_dict_block(out, entries, 0),
        Value::Seq(_) => out.push_str("[]"),
        Value::Map(_) => out.push_str("{}"),
        scalar => write_scalar(out, scalar),
    }
}

/// Write the indicator and value that follow a dict key or a list item's `-`.
/// `scalar_indicator` is what precedes a scalar: `": "` after a key, `""` after `- `.
fn write_entry_value(out: &mut String, value: &Value, indent: usize, scalar_indicator: &str) {
    match value {
        Value::Seq(items) if items.is_empty() => out.push_str(":: []"),
        Value::Map(entries) if entries.is_empty() => out.push_str(":: {}"),
        Value::Seq(items) if items.iter().all(Value::is_scalar) => {
            out.push_str(":: ");
            write_inline_list(out, items);
        }
        Value::Seq(items) => {
            out.push_str("::");
            write_list_block(out, items, indent + 1);
        }
        Value::Map(entries) => {
            out.push_str("::");
            write_dict_block(out, entries, indent + 1);
        }
        scalar => {
            out.push_str(scalar_indicator);
            write_scalar(out, scalar);
        }
    }
}

fn write_dict_block(out: &mut String, entries: &[(String, Value)], indent: usize) {
    for (key, value) in entries {
        start_line(out, indent);
        write_key(out, key);
        write_entry_value(out, value, indent, ": ");
    }
}

fn write_list_block(out: &mut String, items: &[Value], indent: usize) {
    for item in items {
        start_line(out, indent);
        out.push_str("- ");
        write_entry_value(out, item, indent, "");
    }
}

fn write_inline_list(out: &mut String, items: &[Value]) {
    for (i, item) in items.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        write_scalar(out, item);
    }
}

/// Start a new line at the given indent level. The document's first line needs no newline.
fn start_line(out: &mut String, indent: usize) {
    if !out.is_empty() {
        out.push('\n');
    }
    for _ in 0..indent {
        out.push_str("  ");
    }
}

fn write_key(out: &mut String, key: &str) {
    if is_valid_unquoted_key(key) {
        out.push_str(key);
    } else {
        write_string(out, key);
    }
}

fn write_scalar(out: &mut String, value: &Value) {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Int(i) => out.push_str(&i.to_string()),
        Value::UInt(u) => out.push_str(&u.to_string()),
        Value::Float(f) if f.is_nan() => out.push_str("nan"),
        Value::Float(f) if f.is_infinite() => {
            out.push_str(if f.is_sign_positive() { "inf" } else { "-inf" })
        }
        // Debug keeps a fractional part (`1.0`, not `1`) so floats read back as floats.
        Value::Float(f) => out.push_str(&format!("{f:?}")),
        Value::Str(s) => write_string(out, s),
        Value::Seq(_) | Value::Map(_) => unreachable!("write_scalar called with a vector"),
    }
}

/// Write a string value with proper HUML escaping
fn write_string(out: &mut String, s: &str) {
    out.push('"');
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            '\x08' => out.push_str("\\b"),
            '\x0C' => out.push_str("\\f"),
            c if c.is_control() => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

/// Check if a string can be used as an unquoted key in HUML.
/// Matches the parser: an ASCII letter, then ASCII letters, digits, `_` or `-`.
fn is_valid_unquoted_key(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// Serde serializer that builds a [`Value`] tree.
struct ValueSerializer;

impl ser::Serializer for ValueSerializer {
    type Ok = Value;
    type Error = Error;

    type SerializeSeq = SeqBuilder;
    type SerializeTuple = SeqBuilder;
    type SerializeTupleStruct = SeqBuilder;
    type SerializeTupleVariant = TupleVariantBuilder;
    type SerializeMap = MapBuilder;
    type SerializeStruct = MapBuilder;
    type SerializeStructVariant = StructVariantBuilder;

    fn serialize_bool(self, v: bool) -> Result<Value> {
        Ok(Value::Bool(v))
    }

    fn serialize_i8(self, v: i8) -> Result<Value> {
        Ok(Value::Int(v.into()))
    }

    fn serialize_i16(self, v: i16) -> Result<Value> {
        Ok(Value::Int(v.into()))
    }

    fn serialize_i32(self, v: i32) -> Result<Value> {
        Ok(Value::Int(v.into()))
    }

    fn serialize_i64(self, v: i64) -> Result<Value> {
        Ok(Value::Int(v))
    }

    fn serialize_u8(self, v: u8) -> Result<Value> {
        Ok(Value::UInt(v.into()))
    }

    fn serialize_u16(self, v: u16) -> Result<Value> {
        Ok(Value::UInt(v.into()))
    }

    fn serialize_u32(self, v: u32) -> Result<Value> {
        Ok(Value::UInt(v.into()))
    }

    fn serialize_u64(self, v: u64) -> Result<Value> {
        Ok(Value::UInt(v))
    }

    fn serialize_f32(self, v: f32) -> Result<Value> {
        Ok(Value::Float(v.into()))
    }

    fn serialize_f64(self, v: f64) -> Result<Value> {
        Ok(Value::Float(v))
    }

    fn serialize_char(self, v: char) -> Result<Value> {
        Ok(Value::Str(v.to_string()))
    }

    fn serialize_str(self, v: &str) -> Result<Value> {
        Ok(Value::Str(v.to_string()))
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Value> {
        Ok(Value::Seq(
            v.iter().map(|&b| Value::UInt(b.into())).collect(),
        ))
    }

    fn serialize_none(self) -> Result<Value> {
        Ok(Value::Null)
    }

    fn serialize_some<T>(self, value: &T) -> Result<Value>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Value> {
        Ok(Value::Null)
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Value> {
        Ok(Value::Null)
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Value> {
        Ok(Value::Str(variant.to_string()))
    }

    fn serialize_newtype_struct<T>(self, _name: &'static str, value: &T) -> Result<Value>
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
    ) -> Result<Value>
    where
        T: ?Sized + Serialize,
    {
        Ok(Value::Map(vec![(
            variant.to_string(),
            value.serialize(self)?,
        )]))
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<SeqBuilder> {
        Ok(SeqBuilder {
            items: Vec::with_capacity(len.unwrap_or(0)),
        })
    }

    fn serialize_tuple(self, len: usize) -> Result<SeqBuilder> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_struct(self, _name: &'static str, len: usize) -> Result<SeqBuilder> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<TupleVariantBuilder> {
        Ok(TupleVariantBuilder {
            variant,
            items: Vec::with_capacity(len),
        })
    }

    fn serialize_map(self, len: Option<usize>) -> Result<MapBuilder> {
        Ok(MapBuilder {
            entries: Vec::with_capacity(len.unwrap_or(0)),
            next_key: None,
        })
    }

    fn serialize_struct(self, _name: &'static str, len: usize) -> Result<MapBuilder> {
        self.serialize_map(Some(len))
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<StructVariantBuilder> {
        Ok(StructVariantBuilder {
            variant,
            entries: Vec::with_capacity(len),
        })
    }
}

/// Builder for sequences (lists, tuples)
struct SeqBuilder {
    items: Vec<Value>,
}

impl ser::SerializeSeq for SeqBuilder {
    type Ok = Value;
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        self.items.push(value.serialize(ValueSerializer)?);
        Ok(())
    }

    fn end(self) -> Result<Value> {
        Ok(Value::Seq(self.items))
    }
}

impl ser::SerializeTuple for SeqBuilder {
    type Ok = Value;
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        ser::SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<Value> {
        ser::SerializeSeq::end(self)
    }
}

impl ser::SerializeTupleStruct for SeqBuilder {
    type Ok = Value;
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        ser::SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<Value> {
        ser::SerializeSeq::end(self)
    }
}

/// Builder for tuple variants, written as `Variant:: a, b`
struct TupleVariantBuilder {
    variant: &'static str,
    items: Vec<Value>,
}

impl ser::SerializeTupleVariant for TupleVariantBuilder {
    type Ok = Value;
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        self.items.push(value.serialize(ValueSerializer)?);
        Ok(())
    }

    fn end(self) -> Result<Value> {
        Ok(Value::Map(vec![(
            self.variant.to_string(),
            Value::Seq(self.items),
        )]))
    }
}

/// Builder for maps and structs
struct MapBuilder {
    entries: Vec<(String, Value)>,
    next_key: Option<String>,
}

impl ser::SerializeMap for MapBuilder {
    type Ok = Value;
    type Error = Error;

    fn serialize_key<T>(&mut self, key: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        let key = match key.serialize(ValueSerializer)? {
            Value::Str(s) => s,
            Value::Int(i) => i.to_string(),
            Value::UInt(u) => u.to_string(),
            Value::Bool(b) => b.to_string(),
            _ => {
                return Err(Error::UnsupportedType(
                    "map key must be a string, integer or bool",
                ));
            }
        };
        self.next_key = Some(key);
        Ok(())
    }

    fn serialize_value<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        let key = self
            .next_key
            .take()
            .ok_or_else(|| Error::Message("serialize_value called before serialize_key".into()))?;
        self.entries.push((key, value.serialize(ValueSerializer)?));
        Ok(())
    }

    fn end(self) -> Result<Value> {
        Ok(Value::Map(self.entries))
    }
}

impl ser::SerializeStruct for MapBuilder {
    type Ok = Value;
    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        self.entries
            .push((key.to_string(), value.serialize(ValueSerializer)?));
        Ok(())
    }

    fn end(self) -> Result<Value> {
        Ok(Value::Map(self.entries))
    }
}

/// Builder for struct variants, written as `Variant::` followed by the fields
struct StructVariantBuilder {
    variant: &'static str,
    entries: Vec<(String, Value)>,
}

impl ser::SerializeStructVariant for StructVariantBuilder {
    type Ok = Value;
    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        self.entries
            .push((key.to_string(), value.serialize(ValueSerializer)?));
        Ok(())
    }

    fn end(self) -> Result<Value> {
        Ok(Value::Map(vec![(
            self.variant.to_string(),
            Value::Map(self.entries),
        )]))
    }
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
        assert_eq!(to_string(&2.5).unwrap(), "2.5");
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
        assert!(is_valid_unquoted_key("key123"));
        assert!(is_valid_unquoted_key("X-Env"));

        assert!(!is_valid_unquoted_key(""));
        assert!(!is_valid_unquoted_key("123key"));
        assert!(!is_valid_unquoted_key("with spaces"));
        assert!(!is_valid_unquoted_key("with.dot"));
        assert!(!is_valid_unquoted_key("with:colon"));
        // The parser only accepts an ASCII letter as the first character of a bare key.
        assert!(!is_valid_unquoted_key("_starts_with_underscore"));
        assert!(!is_valid_unquoted_key("café"));
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
        assert!(result.config.enabled);
        assert_eq!(result.config.timeout, 30);

        // Should use proper HUML formatting with :: syntax and indentation
        assert!(huml.contains("scores:: "));
        assert!(huml.contains("config::\n"));
        assert!(huml.contains("  enabled: true"));
        assert!(huml.contains("  timeout: 30"));
    }

    /// Serialize, parse back into the same type, and return the HUML text.
    fn round_trip<T>(value: &T) -> String
    where
        T: Serialize + for<'de> serde::Deserialize<'de> + PartialEq + std::fmt::Debug,
    {
        let huml = to_string(value).unwrap();
        let back: T = crate::serde::from_str(&huml)
            .unwrap_or_else(|e| panic!("failed to parse back {huml:?}: {e}"));
        assert_eq!(
            &back, value,
            "round trip changed the value, HUML was:\n{huml}"
        );
        huml
    }

    #[derive(Debug, PartialEq, Serialize, serde::Deserialize)]
    struct Target {
        host: String,
        port: u16,
    }

    #[test]
    fn test_list_of_structs() {
        #[derive(Debug, PartialEq, Serialize, serde::Deserialize)]
        struct Config {
            targets: Vec<Target>,
        }

        let config = Config {
            targets: vec![
                Target {
                    host: "10.0.0.1".into(),
                    port: 80,
                },
                Target {
                    host: "10.0.0.2".into(),
                    port: 81,
                },
            ],
        };
        assert_eq!(
            round_trip(&config),
            "targets::\n  - ::\n    host: \"10.0.0.1\"\n    port: 80\n  - ::\n    host: \"10.0.0.2\"\n    port: 81"
        );
    }

    #[test]
    fn test_one_item_list_stays_a_list() {
        #[derive(Debug, PartialEq, Serialize, serde::Deserialize)]
        struct Paths {
            paths: Vec<String>,
        }

        let paths = Paths {
            paths: vec!["/".into()],
        };
        assert_eq!(round_trip(&paths), "paths:: \"/\"");
    }

    #[test]
    fn test_string_with_comma_stays_a_string() {
        #[derive(Debug, PartialEq, Serialize, serde::Deserialize)]
        struct Note {
            note: String,
        }

        let note = Note {
            note: "a, b".into(),
        };
        assert_eq!(round_trip(&note), "note: \"a, b\"");
    }

    #[test]
    fn test_deeply_nested_dicts_keep_indentation() {
        #[derive(Debug, PartialEq, Serialize, serde::Deserialize)]
        struct Deeper {
            x: u32,
            y: u32,
        }
        #[derive(Debug, PartialEq, Serialize, serde::Deserialize)]
        struct Inner {
            deep: Deeper,
        }
        #[derive(Debug, PartialEq, Serialize, serde::Deserialize)]
        struct Nested {
            outer: Inner,
        }

        let nested = Nested {
            outer: Inner {
                deep: Deeper { x: 1, y: 2 },
            },
        };
        assert_eq!(round_trip(&nested), "outer::\n  deep::\n    x: 1\n    y: 2");
    }

    #[test]
    fn test_forward_slash_not_escaped() {
        assert_eq!(to_string("/healthz").unwrap(), "\"/healthz\"");
        assert_eq!(
            to_string("https://example.com").unwrap(),
            "\"https://example.com\""
        );
    }

    #[test]
    fn test_strings_with_syntax_characters_in_lists() {
        #[derive(Debug, PartialEq, Serialize, serde::Deserialize)]
        struct Urls {
            urls: Vec<String>,
            single: Vec<String>,
        }

        let urls = Urls {
            urls: vec!["https://a.example".into(), "b, c # d".into()],
            single: vec!["https://only.example".into()],
        };
        round_trip(&urls);
    }

    #[test]
    fn test_list_of_lists() {
        let lists: Vec<Vec<u32>> = vec![vec![1, 2], vec![], vec![3]];
        assert_eq!(round_trip(&lists), "- :: 1, 2\n- :: []\n- :: 3");

        let nested: Vec<Vec<Vec<u32>>> = vec![vec![vec![1], vec![2, 3]]];
        round_trip(&nested);
    }

    #[test]
    fn test_list_mixing_scalars_and_dicts() {
        let mixed = serde_json::json!({
            "items": [1, "two", {"three": 3}, [4, 5], null, {}]
        });
        let huml = to_string(&mixed).unwrap();
        let back: serde_json::Value = crate::serde::from_str(&huml).unwrap();
        assert_eq!(back, mixed, "HUML was:\n{huml}");
    }

    #[test]
    fn test_keys_that_need_quoting() {
        use std::collections::BTreeMap;

        let map: BTreeMap<String, u32> = [
            ("X-Env", 1),
            ("with space", 2),
            ("1abc", 3),
            ("_private", 4),
            ("a:b", 5),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_string(), v))
        .collect();
        let huml = round_trip(&map);
        assert!(huml.contains("X-Env: 1"));
        assert!(huml.contains("\"with space\": 2"));
        assert!(huml.contains("\"_private\": 4"));
    }

    #[test]
    fn test_escaped_newline_is_a_scalar() {
        #[derive(Debug, PartialEq, Serialize, serde::Deserialize)]
        struct Text {
            text: String,
            after: u32,
        }

        let text = Text {
            text: "line one\nline two".into(),
            after: 1,
        };
        assert_eq!(round_trip(&text), "text: \"line one\\nline two\"\nafter: 1");
    }

    #[test]
    fn test_root_values() {
        round_trip(&vec![Target {
            host: "h".into(),
            port: 1,
        }]);
        assert_eq!(round_trip(&vec!["only".to_string()]), "- \"only\"");
        assert_eq!(round_trip(&"a, b".to_string()), "\"a, b\"");
    }

    #[test]
    fn test_enum_variants_round_trip() {
        #[derive(Debug, PartialEq, Serialize, serde::Deserialize)]
        enum Action {
            Stop,
            Wait(u32),
            Move(i32, i32),
            Forward { target: Target },
        }
        #[derive(Debug, PartialEq, Serialize, serde::Deserialize)]
        struct Rule {
            actions: Vec<Action>,
            fallback: Action,
        }

        round_trip(&Rule {
            actions: vec![
                Action::Stop,
                Action::Wait(5),
                Action::Move(1, -1),
                Action::Forward {
                    target: Target {
                        host: "h".into(),
                        port: 1,
                    },
                },
            ],
            fallback: Action::Move(0, 0),
        });
    }

    #[test]
    fn test_floats_and_options_round_trip() {
        #[derive(Debug, PartialEq, Serialize, serde::Deserialize)]
        struct Numbers {
            whole: f64,
            small: f64,
            missing: Option<u32>,
            present: Option<u32>,
        }

        let huml = round_trip(&Numbers {
            whole: 1.0,
            small: 1e-7,
            missing: None,
            present: Some(3),
        });
        assert!(huml.contains("whole: 1.0"));
        assert!(huml.contains("missing: null"));

        // Untyped readers must still see a float, not an integer.
        let back: serde_json::Value = crate::serde::from_str(&huml).unwrap();
        assert!(back["whole"].is_f64());
    }
}
