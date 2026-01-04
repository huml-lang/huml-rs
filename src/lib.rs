use std::collections::HashMap;

mod parser;
pub mod serde;
#[cfg(test)]
pub mod standard_tests;

pub use parser::{
    parse_document_root, parse_empty_dict, parse_empty_list, parse_huml, parse_inline_dict,
    parse_inline_list, parse_scalar, IResult, ParseError, HUML_VERSION,
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

    #[test]
    fn multiline_string_preserves_dedented() {
        let input = r#"text: """
  line one
  line two
  line three
""""#;
        let (_, doc) = parse_huml(input).expect("should parse");
        if let HumlValue::Dict(map) = doc.root {
            if let Some(HumlValue::String(s)) = map.get("text") {
                assert_eq!(s, "line one\nline two\nline three");
            } else {
                panic!("expected string value");
            }
        } else {
            panic!("expected dict");
        }
    }

    #[test]
    fn multiline_string_preserves_extra_spaces() {
        let input = r#"text: """
    line with extra spaces
  line with minimal spaces
      line with many spaces
""""#;
        let (_, doc) = parse_huml(input).expect("should parse");
        if let HumlValue::Dict(map) = doc.root {
            if let Some(HumlValue::String(s)) = map.get("text") {
                assert_eq!(
                    s,
                    "  line with extra spaces\nline with minimal spaces\n    line with many spaces"
                );
            } else {
                panic!("expected string value");
            }
        } else {
            panic!("expected dict");
        }
    }

    #[test]
    fn multiline_string_with_empty_lines() {
        let input = r#"text: """
  first line

  third line
""""#;
        let (_, doc) = parse_huml(input).expect("should parse");
        if let HumlValue::Dict(map) = doc.root {
            if let Some(HumlValue::String(s)) = map.get("text") {
                assert_eq!(s, "first line\n\nthird line");
            } else {
                panic!("expected string value");
            }
        } else {
            panic!("expected dict");
        }
    }

    #[test]
    fn multiline_string_minimal_indent() {
        let input = r#"x: """
first
second
""""#;
        let (_, doc) = parse_huml(input).expect("should parse");
        if let HumlValue::Dict(map) = doc.root {
            if let Some(HumlValue::String(s)) = map.get("x") {
                assert_eq!(s, "first\nsecond");
            } else {
                panic!("expected string value");
            }
        } else {
            panic!("expected dict");
        }
    }

    #[test]
    fn backticks_multiline_string_rejected() {
        let input = r#"text: ```
  line one
```"#;
        assert!(parse_huml(input).is_err());
    }

    #[test]
    fn multiline_string_in_list() {
        let input = r#"items::
  - """
    line one
    line two
  """
  - "regular string""#;
        let (_, doc) = parse_huml(input).expect("should parse");
        if let HumlValue::Dict(map) = doc.root {
            if let Some(HumlValue::List(items)) = map.get("items") {
                assert_eq!(items.len(), 2);
                if let HumlValue::String(s) = &items[0] {
                    assert_eq!(s, "line one\nline two");
                } else {
                    panic!("expected string in list");
                }
            } else {
                panic!("expected list");
            }
        } else {
            panic!("expected dict");
        }
    }

    #[test]
    fn duplicate_key_error_before_malformed_value() {
        // This test ensures duplicate key errors are reported before parsing malformed values
        let input = r#"
key: "first"
key: [this is malformed
"#;
        let result = parse_huml(input);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        // Should get duplicate key error, not a parse error from the malformed value
        assert!(err_msg.contains("duplicate key"));
    }
}
