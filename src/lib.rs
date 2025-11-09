use std::collections::HashMap;

mod parser;
pub mod serde;
#[cfg(test)]
pub mod standard_tests;

pub use parser::{
    IResult, ParseError, parse_document_root, parse_empty_dict, parse_empty_list, parse_huml,
    parse_inline_dict, parse_inline_list, parse_scalar,
};

#[derive(Debug, Clone, PartialEq)]
pub enum HumlValue {
    String(String),
    Number(HumlNumber),
    Boolean(bool),
    Null,
    List(Vec<HumlValue>),
    Dict(HashMap<String, HumlValue>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum HumlNumber {
    Integer(i64),
    Float(f64),
    Nan,
    Infinity(bool), // true = positive, false = negative
}

#[derive(Debug, Clone, PartialEq)]
pub struct HumlDocument {
    pub version: Option<String>,
    pub root: HumlValue,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple_scalar_document() {
        let (_, doc) = parse_huml("\"hello\"").expect("should parse");
        assert_eq!(doc.root, HumlValue::String("hello".into()));
    }

    #[test]
    fn parses_inline_list() {
        if let HumlValue::List(values) = parse_inline_list("1, 2, 3").unwrap().1 {
            assert_eq!(values.len(), 3);
        } else {
            panic!("expected list");
        }
    }

    #[test]
    fn parses_multiline_dict_document() {
        let input = r#"
key1: "value"
key2::
  nested: 1
"#;
        let (_, doc) = parse_huml(input).expect("should parse");
        if let HumlValue::Dict(map) = doc.root {
            assert!(map.contains_key("key1"));
            assert!(map.contains_key("key2"));
        } else {
            panic!("expected dict");
        }
    }
}
