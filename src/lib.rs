use nom::{
    IResult, Parser,
    branch::alt,
    bytes::complete::{tag, take_while, take_while1},
    character::complete::{char, line_ending, not_line_ending, space1},
    combinator::{map, opt, value},
    multi::{many0, separated_list1},
    sequence::preceded,
};
use std::collections::HashMap;

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

// Helper function to parse indentation (2 spaces per level)
fn parse_indent(input: &str) -> IResult<&str, usize> {
    let (input, spaces) = take_while(|c| c == ' ').parse(input)?;
    if spaces.len() % 2 != 0 {
        return Err(nom::Err::Error(nom::error::Error::new(
            input,
            nom::error::ErrorKind::Tag,
        )));
    }
    Ok((input, spaces.len() / 2))
}

// Parse comment
fn parse_comment(input: &str) -> IResult<&str, ()> {
    let (input, _) = take_while(|c| c == ' ').parse(input)?;
    let (input, _) = char('#').parse(input)?;
    let (input, _) = char(' ').parse(input)?;
    let (input, _) = not_line_ending.parse(input)?;
    Ok((input, ()))
}

// Parse empty line or comment line
fn parse_empty_or_comment(input: &str) -> IResult<&str, ()> {
    alt((
        map((take_while(|c| c == ' '), line_ending), |_| ()),
        map((parse_comment, line_ending), |_| ()),
    ))
    .parse(input)
}

// Skip empty lines and comments
fn skip_empty_and_comments(input: &str) -> IResult<&str, ()> {
    let (input, _) = many0(parse_empty_or_comment).parse(input)?;
    Ok((input, ()))
}

// Parse simple quoted string
fn parse_quoted_string(input: &str) -> IResult<&str, String> {
    let (input, _) = char('"').parse(input)?;
    let mut result = String::new();
    let mut remaining = input;

    while !remaining.is_empty() {
        if remaining.starts_with('"') {
            remaining = &remaining[1..];
            break;
        } else if remaining.starts_with('\\') {
            if remaining.len() < 2 {
                return Err(nom::Err::Error(nom::error::Error::new(
                    input,
                    nom::error::ErrorKind::Escaped,
                )));
            }
            match remaining.chars().nth(1) {
                Some('"') => {
                    result.push('"');
                    remaining = &remaining[2..];
                }
                Some('\\') => {
                    result.push('\\');
                    remaining = &remaining[2..];
                }
                Some('n') => {
                    result.push('\n');
                    remaining = &remaining[2..];
                }
                Some('t') => {
                    result.push('\t');
                    remaining = &remaining[2..];
                }
                Some('r') => {
                    result.push('\r');
                    remaining = &remaining[2..];
                }
                Some(c) => {
                    result.push(c);
                    remaining = &remaining[2..];
                }
                None => {
                    return Err(nom::Err::Error(nom::error::Error::new(
                        input,
                        nom::error::ErrorKind::Escaped,
                    )));
                }
            }
        } else {
            let c = remaining.chars().next().unwrap();
            result.push(c);
            remaining = &remaining[c.len_utf8()..];
        }
    }

    Ok((remaining, result))
}

// Parse multi-line string with ``` (preserve spaces)
fn parse_multiline_string_preserve(input: &str) -> IResult<&str, String> {
    let (input, _) = tag("```").parse(input)?;
    let (input, _) = line_ending.parse(input)?;

    let mut result = String::new();
    let mut remaining = input;
    let mut first_line = true;

    while !remaining.is_empty() {
        // Try to parse a line
        let (line_end, line_content) =
            not_line_ending::<&str, nom::error::Error<&str>>.parse(remaining)?;

        // Check if this line is the closing ```
        if line_content.trim() == "```" {
            if let Ok((after_newline, _)) =
                line_ending::<&str, nom::error::Error<&str>>.parse(line_end)
            {
                return Ok((after_newline, result));
            } else {
                return Ok((line_end, result));
            }
        }

        // Process the line content
        // If it starts with at least 2 spaces, remove them (minimum indentation)
        if line_content.len() >= 2 && line_content.starts_with("  ") {
            if !first_line {
                result.push('\n');
            }
            result.push_str(&line_content[2..]);
        } else {
            if !first_line {
                result.push('\n');
            }
            result.push_str(line_content);
        }

        first_line = false;

        // Continue to next line
        if let Ok((new_remaining, _)) = line_ending::<&str, nom::error::Error<&str>>.parse(line_end)
        {
            remaining = new_remaining;
        } else {
            break;
        }
    }

    // If we reach here, we didn't find a closing ```
    Err(nom::Err::Error(nom::error::Error::new(
        input,
        nom::error::ErrorKind::Tag,
    )))
}

// Parse multi-line string with """ (strip spaces)
fn parse_multiline_string_strip(input: &str) -> IResult<&str, String> {
    let (input, _) = tag("\"\"\"").parse(input)?;
    let (input, _) = line_ending.parse(input)?;

    let mut lines = Vec::new();
    let mut remaining = input;

    while !remaining.is_empty() {
        // Try to parse a line
        let (line_end, line_content) =
            not_line_ending::<&str, nom::error::Error<&str>>.parse(remaining)?;

        // Check if this line is the closing """
        if line_content.trim() == "\"\"\"" {
            if let Ok((after_newline, _)) =
                line_ending::<&str, nom::error::Error<&str>>.parse(line_end)
            {
                return Ok((after_newline, lines.join("\n")));
            } else {
                return Ok((line_end, lines.join("\n")));
            }
        }

        // Process the line content by stripping leading/trailing spaces
        let trimmed = line_content.trim();
        if !trimmed.is_empty() {
            lines.push(trimmed.to_string());
        }

        // Continue to next line
        if let Ok((new_remaining, _)) = line_ending::<&str, nom::error::Error<&str>>.parse(line_end)
        {
            remaining = new_remaining;
        } else {
            break;
        }
    }

    // If we reach here, we didn't find a closing """
    Err(nom::Err::Error(nom::error::Error::new(
        input,
        nom::error::ErrorKind::Tag,
    )))
}

// Parse any string
fn parse_string(input: &str) -> IResult<&str, HumlValue> {
    alt((
        map(parse_multiline_string_preserve, HumlValue::String),
        map(parse_multiline_string_strip, HumlValue::String),
        map(parse_quoted_string, HumlValue::String),
    ))
    .parse(input)
}

// Parse binary number
fn parse_binary_number(input: &str) -> IResult<&str, i64> {
    let (input, _) = tag("0b").parse(input)?;
    let (input, binary_str) = take_while1(|c| c == '0' || c == '1' || c == '_').parse(input)?;
    let clean_binary = binary_str.replace('_', "");
    let number = i64::from_str_radix(&clean_binary, 2).map_err(|_| {
        nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Digit))
    })?;
    Ok((input, number))
}

// Parse octal number
fn parse_octal_number(input: &str) -> IResult<&str, i64> {
    let (input, _) = tag("0o").parse(input)?;
    let (input, octal_str) = take_while1(|c| c >= '0' && c <= '7' || c == '_').parse(input)?;
    let clean_octal = octal_str.replace('_', "");
    let number = i64::from_str_radix(&clean_octal, 8).map_err(|_| {
        nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Digit))
    })?;
    Ok((input, number))
}

// Parse hex number
fn parse_hex_number(input: &str) -> IResult<&str, i64> {
    let (input, _) = tag("0x").parse(input)?;
    let (input, hex_str) = take_while1(|c: char| c.is_ascii_hexdigit() || c == '_').parse(input)?;
    let clean_hex = hex_str.replace('_', "");
    let number = i64::from_str_radix(&clean_hex, 16).map_err(|_| {
        nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Digit))
    })?;
    Ok((input, number))
}

// Helper to recognize a float pattern (with decimal point or exponent)
fn is_float_pattern(input: &str) -> bool {
    let mut chars = input.chars();

    // Skip optional sign
    if let Some(c) = chars.next() {
        if c == '+' || c == '-' {
            // Continue
        } else if c.is_ascii_digit() {
            // Put it back by not advancing
        } else {
            return false;
        }
    }

    // Look for digits, then decimal point or exponent
    let mut has_digits = false;
    let mut has_decimal = false;
    let mut has_exponent = false;

    for c in input.chars() {
        if c.is_ascii_digit() || c == '_' {
            has_digits = true;
        } else if c == '.' && !has_decimal && !has_exponent {
            has_decimal = true;
        } else if (c == 'e' || c == 'E') && !has_exponent && has_digits {
            has_exponent = true;
        } else if c == '+' || c == '-' {
            // Only valid after 'e' or 'E'
            continue;
        } else {
            break;
        }
    }

    has_digits && (has_decimal || has_exponent)
}

// Parse float number (only if it has decimal point or exponent)
fn parse_float(input: &str) -> IResult<&str, f64> {
    if !is_float_pattern(input) {
        return Err(nom::Err::Error(nom::error::Error::new(
            input,
            nom::error::ErrorKind::Digit,
        )));
    }

    let original_input = input;
    let (input, sign) = opt(alt((char('+'), char('-')))).parse(input)?;
    let (input, int_part) = take_while1(|c: char| c.is_ascii_digit() || c == '_').parse(input)?;

    let (input, decimal_part) = opt(preceded(
        char('.'),
        take_while1(|c: char| c.is_ascii_digit() || c == '_'),
    ))
    .parse(input)?;
    let (input, exp_part) = opt((
        alt((char('e'), char('E'))),
        opt(alt((char('+'), char('-')))),
        take_while1(|c: char| c.is_ascii_digit() || c == '_'),
    ))
    .parse(input)?;

    let mut num_str = String::new();
    if let Some(s) = sign {
        num_str.push(s);
    }
    num_str.push_str(&int_part.replace('_', ""));

    if let Some(dec) = decimal_part {
        num_str.push('.');
        num_str.push_str(&dec.replace('_', ""));
    }

    if let Some((_, exp_sign, exp_digits)) = exp_part {
        num_str.push('e');
        if let Some(s) = exp_sign {
            num_str.push(s);
        }
        num_str.push_str(&exp_digits.replace('_', ""));
    }

    let number = num_str.parse::<f64>().map_err(|_| {
        nom::Err::Error(nom::error::Error::new(
            original_input,
            nom::error::ErrorKind::Digit,
        ))
    })?;
    Ok((input, number))
}

// Parse integer
fn parse_integer(input: &str) -> IResult<&str, i64> {
    let (input, sign) = opt(alt((char('+'), char('-')))).parse(input)?;
    let (input, digits) = take_while1(|c: char| c.is_ascii_digit() || c == '_').parse(input)?;

    let mut num_str = String::new();
    if let Some(s) = sign {
        num_str.push(s);
    }
    num_str.push_str(&digits.replace('_', ""));

    let number = num_str.parse::<i64>().map_err(|_| {
        nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Digit))
    })?;
    Ok((input, number))
}

// Parse special number values
fn parse_special_numbers(input: &str) -> IResult<&str, HumlValue> {
    alt((
        value(HumlValue::Number(HumlNumber::Nan), tag("nan")),
        value(HumlValue::Number(HumlNumber::Infinity(true)), tag("inf")),
        value(HumlValue::Number(HumlNumber::Infinity(true)), tag("+inf")),
        value(HumlValue::Number(HumlNumber::Infinity(false)), tag("-inf")),
    ))
    .parse(input)
}

// Parse any number
fn parse_number(input: &str) -> IResult<&str, HumlValue> {
    alt((
        parse_special_numbers,
        map(parse_binary_number, |n| {
            HumlValue::Number(HumlNumber::Integer(n))
        }),
        map(parse_octal_number, |n| {
            HumlValue::Number(HumlNumber::Integer(n))
        }),
        map(parse_hex_number, |n| {
            HumlValue::Number(HumlNumber::Integer(n))
        }),
        map(parse_float, |n| HumlValue::Number(HumlNumber::Float(n))),
        map(parse_integer, |n| HumlValue::Number(HumlNumber::Integer(n))),
    ))
    .parse(input)
}

// Parse boolean
fn parse_boolean(input: &str) -> IResult<&str, HumlValue> {
    alt((
        value(HumlValue::Boolean(true), tag("true")),
        value(HumlValue::Boolean(false), tag("false")),
    ))
    .parse(input)
}

// Parse null
fn parse_null(input: &str) -> IResult<&str, HumlValue> {
    value(HumlValue::Null, tag("null")).parse(input)
}

// Parse scalar value
pub fn parse_scalar(input: &str) -> IResult<&str, HumlValue> {
    alt((parse_string, parse_number, parse_boolean, parse_null)).parse(input)
}

// Parse empty list
pub fn parse_empty_list(input: &str) -> IResult<&str, HumlValue> {
    value(HumlValue::List(Vec::new()), tag("[]")).parse(input)
}

// Parse empty dict
pub fn parse_empty_dict(input: &str) -> IResult<&str, HumlValue> {
    value(HumlValue::Dict(HashMap::new()), tag("{}")).parse(input)
}

// Parse inline list
pub fn parse_inline_list(input: &str) -> IResult<&str, HumlValue> {
    // First parse a scalar, then check if it's followed by a comma
    let (input, first_item) = parse_scalar(input)?;

    // If there's no comma, this is not an inline list
    if !input.starts_with(',') {
        return Err(nom::Err::Error(nom::error::Error::new(
            input,
            nom::error::ErrorKind::Tag,
        )));
    }

    // Parse the rest of the items
    let (input, mut remaining_items) =
        many0(preceded((char(','), space1), parse_scalar)).parse(input)?;

    // Combine all items
    let mut items = vec![first_item];
    items.append(&mut remaining_items);

    Ok((input, HumlValue::List(items)))
}

// Parse unquoted key
fn parse_unquoted_key(input: &str) -> IResult<&str, String> {
    map(
        take_while1(|c: char| c.is_alphanumeric() || c == '_' || c == '-'),
        |s: &str| s.to_string(),
    )
    .parse(input)
}

// Parse quoted key
fn parse_quoted_key(input: &str) -> IResult<&str, String> {
    parse_quoted_string(input)
}

// Parse any key
fn parse_key(input: &str) -> IResult<&str, String> {
    alt((parse_quoted_key, parse_unquoted_key)).parse(input)
}

// Parse key-value pair for dict
fn parse_dict_pair(input: &str) -> IResult<&str, (String, HumlValue)> {
    let (input, key) = parse_key(input)?;
    let (input, _) = char(':').parse(input)?;
    let (input, _) = space1.parse(input)?;
    let (input, value) = parse_scalar(input)?;
    Ok((input, (key, value)))
}

// Parse inline dict
pub fn parse_inline_dict(input: &str) -> IResult<&str, HumlValue> {
    map(
        separated_list1((char(','), space1), parse_dict_pair),
        |pairs| {
            let mut dict = HashMap::new();
            for (key, value) in pairs {
                dict.insert(key, value);
            }
            HumlValue::Dict(dict)
        },
    )
    .parse(input)
}

// Parse multi-line list item
fn parse_list_item(input: &str, expected_indent: usize) -> IResult<&str, HumlValue> {
    let (input, indent) = parse_indent(input)?;

    if indent != expected_indent {
        return Err(nom::Err::Error(nom::error::Error::new(
            input,
            nom::error::ErrorKind::Tag,
        )));
    }

    let (input, _) = char('-').parse(input)?;
    let (input, _) = space1.parse(input)?;

    // Check if this is a nested vector
    if input.starts_with("::") {
        let (input, _) = tag("::").parse(input)?;

        if let Ok((input, _)) = line_ending::<&str, nom::error::Error<&str>>.parse(input) {
            parse_multiline_value(input, expected_indent + 1)
        } else {
            // Check if we have a space followed by content
            if let Ok((input, _)) = space1::<&str, nom::error::Error<&str>>.parse(input) {
                // Check if it's a comment after the space
                if input.starts_with('#') {
                    let (input, _) = opt(parse_comment).parse(input)?;
                    let (input, _) = line_ending.parse(input)?;
                    parse_multiline_value(input, expected_indent + 1)
                } else {
                    alt((
                        parse_inline_list,
                        parse_inline_dict,
                        parse_empty_list,
                        parse_empty_dict,
                    ))
                    .parse(input)
                }
            } else if input.starts_with('#') {
                let (input, _) = opt(parse_comment).parse(input)?;
                let (input, _) = line_ending.parse(input)?;
                parse_multiline_value(input, expected_indent + 1)
            } else {
                parse_multiline_value(input, expected_indent + 1)
            }
        }
    } else {
        // Parse scalar value or inline structures
        alt((
            parse_string,
            parse_scalar,
            parse_inline_list,
            parse_inline_dict,
            parse_empty_list,
            parse_empty_dict,
        ))
        .parse(input)
    }
}

// Parse multi-line list
fn parse_multiline_list(input: &str, expected_indent: usize) -> IResult<&str, HumlValue> {
    let mut items = Vec::new();
    let mut remaining = input;

    loop {
        let (input, _) = skip_empty_and_comments(remaining)?;

        // Check if we're at end of input or at a different indentation level
        if input.is_empty() {
            break;
        }

        // Check indentation to see if we should stop
        if let Ok((_, indent)) = parse_indent(input) {
            if indent < expected_indent {
                // We've reached content at a lower indentation level, stop here
                remaining = input;
                break;
            }
        }

        // Check if we have a list item at the expected indentation
        if let Ok((new_input, item)) = parse_list_item(input, expected_indent) {
            items.push(item);
            let (new_input, _) = opt(line_ending).parse(new_input)?;
            remaining = new_input;
        } else {
            break;
        }
    }

    Ok((remaining, HumlValue::List(items)))
}

// Parse multi-line dict entry
fn parse_dict_entry(input: &str, expected_indent: usize) -> IResult<&str, (String, HumlValue)> {
    let (input, indent) = parse_indent(input)?;

    if indent != expected_indent {
        return Err(nom::Err::Error(nom::error::Error::new(
            input,
            nom::error::ErrorKind::Tag,
        )));
    }

    let (input, key) = parse_key(input)?;

    // Check if this is a scalar or vector
    if input.starts_with("::") {
        let (input, _) = tag("::").parse(input)?;

        // Check for optional space and inline content
        if let Ok((input, _)) = space1::<&str, nom::error::Error<&str>>.parse(input) {
            // Check if inline or multiline
            if input.starts_with('\n') || input.starts_with('\r') {
                let (input, _) = line_ending.parse(input)?;
                let (input, value) = parse_multiline_value(input, expected_indent + 1)?;
                Ok((input, (key, value)))
            } else if input.starts_with('#') {
                // Comment after ::, treat as multiline
                let (input, _) = opt(parse_comment).parse(input)?;
                let (input, _) = line_ending.parse(input)?;
                let (input, value) = parse_multiline_value(input, expected_indent + 1)?;
                Ok((input, (key, value)))
            } else {
                // Inline vector
                let (input, value) = alt((
                    parse_inline_list,
                    parse_inline_dict,
                    parse_empty_list,
                    parse_empty_dict,
                ))
                .parse(input)?;
                Ok((input, (key, value)))
            }
        } else if input.starts_with('#') {
            // Comment immediately after ::, treat as multiline
            let (input, _) = opt(parse_comment).parse(input)?;
            let (input, _) = line_ending.parse(input)?;
            let (input, value) = parse_multiline_value(input, expected_indent + 1)?;
            Ok((input, (key, value)))
        } else {
            // Must be followed by newline for multiline
            let (input, _) = line_ending.parse(input)?;
            let (input, value) = parse_multiline_value(input, expected_indent + 1)?;
            Ok((input, (key, value)))
        }
    } else {
        let (input, _) = char(':').parse(input)?;
        let (input, _) = space1.parse(input)?;

        // Parse the value (can be scalar, inline list, inline dict, or multiline string)
        let (input, value) = alt((
            parse_string, // This handles both regular and multiline strings
            parse_empty_list,
            parse_empty_dict,
            parse_scalar,
            parse_inline_list,
            parse_inline_dict,
        ))
        .parse(input)?;
        Ok((input, (key, value)))
    }
}

// Parse multi-line dict
fn parse_multiline_dict(input: &str, expected_indent: usize) -> IResult<&str, HumlValue> {
    let mut dict = HashMap::new();
    let mut remaining = input;

    loop {
        // Skip empty lines and comments
        let (new_input, _) = skip_empty_and_comments(remaining)?;

        // Check if we're at the end of input
        if new_input.is_empty() {
            remaining = new_input;
            break;
        }

        // Check indentation to see if we should stop
        if let Ok((_, indent)) = parse_indent(new_input) {
            if indent < expected_indent {
                // We've reached content at a lower indentation level, stop here
                remaining = new_input;
                break;
            }
        }

        // Try to parse a dict entry
        match parse_dict_entry(new_input, expected_indent) {
            Ok((new_input, (key, value))) => {
                dict.insert(key, value);
                let (new_input, _) = opt(line_ending).parse(new_input)?;
                remaining = new_input;
            }
            Err(_) => break,
        }
    }

    Ok((remaining, HumlValue::Dict(dict)))
}

// Parse multi-line value (list or dict)
fn parse_multiline_value(input: &str, expected_indent: usize) -> IResult<&str, HumlValue> {
    let (input, _) = skip_empty_and_comments(input)?;

    // If input is empty, return empty dict
    if input.is_empty() {
        return Ok((input, HumlValue::Dict(HashMap::new())));
    }

    // Peek at the next non-empty line to determine if it's a list or dict
    let mut peek_input = input;
    loop {
        let (new_input, _) = skip_empty_and_comments(peek_input)?;
        if new_input.is_empty() {
            break;
        }
        let (new_input, indent) = parse_indent(new_input)?;

        if indent == expected_indent {
            if new_input.starts_with('-') {
                return parse_multiline_list(input, expected_indent);
            } else {
                return parse_multiline_dict(input, expected_indent);
            }
        } else if indent < expected_indent {
            // We've reached content at a lower indentation level
            break;
        } else {
            // Continue looking for content at the expected level
            peek_input = new_input;
        }
    }

    // If we can't determine, default to empty dict
    Ok((input, HumlValue::Dict(HashMap::new())))
}

// Parse version header
fn parse_version(input: &str) -> IResult<&str, Option<String>> {
    let (input, _) = skip_empty_and_comments(input)?;

    opt(map(
        (
            tag("%HUML"),
            space1::<&str, nom::error::Error<&str>>,
            preceded(
                char('v'),
                take_while1(|c: char| c.is_alphanumeric() || c == '.' || c == '-'),
            ),
            line_ending,
        ),
        |(_, _, version, _)| version.to_string(),
    ))
    .parse(input)
}

// Parse document root
pub fn parse_document_root(input: &str) -> IResult<&str, HumlValue> {
    let (input, _) = skip_empty_and_comments(input)?;

    // Check if the document starts with a list item
    if input.starts_with('-') {
        return parse_multiline_list(input, 0);
    }

    // Check if it's a multiline string at root
    if input.starts_with("```") || input.starts_with("\"\"\"") {
        return parse_string(input);
    }

    // Check if it's a simple scalar value (single line)
    if !input.contains('\n') {
        // Try inline dict/list first
        if let Ok((remaining, value)) = alt((
            parse_inline_dict,
            parse_inline_list,
            parse_empty_dict,
            parse_empty_list,
        ))
        .parse(input)
        {
            return Ok((remaining, value));
        }

        // Then try scalar
        if let Ok((remaining, value)) = parse_scalar(input) {
            return Ok((remaining, value));
        }
    }

    // Otherwise, it's a multi-line dict at root level
    parse_multiline_dict(input, 0)
}

// Main parser function
pub fn parse_huml(input: &str) -> IResult<&str, HumlDocument> {
    let (input, version) = parse_version(input)?;
    let (input, root) = parse_document_root(input)?;
    let (input, _) = skip_empty_and_comments(input)?;
    Ok((input, HumlDocument { version, root }))
}

pub mod serde;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_string() {
        assert_eq!(
            parse_string("\"hello\""),
            Ok(("", HumlValue::String("hello".to_string())))
        );
        assert_eq!(
            parse_string("\"hello \\\"world\\\"\""),
            Ok(("", HumlValue::String("hello \"world\"".to_string())))
        );
    }

    #[test]
    fn test_parse_number() {
        assert_eq!(
            parse_number("123"),
            Ok(("", HumlValue::Number(HumlNumber::Integer(123))))
        );
        assert_eq!(
            parse_number("1_234_567"),
            Ok(("", HumlValue::Number(HumlNumber::Integer(1234567))))
        );
        assert_eq!(
            parse_number("3.14"),
            Ok(("", HumlValue::Number(HumlNumber::Float(3.14))))
        );
        assert_eq!(
            parse_number("1e10"),
            Ok(("", HumlValue::Number(HumlNumber::Float(1e10))))
        );
        assert_eq!(
            parse_number("0x1A"),
            Ok(("", HumlValue::Number(HumlNumber::Integer(26))))
        );
        assert_eq!(
            parse_number("0o12"),
            Ok(("", HumlValue::Number(HumlNumber::Integer(10))))
        );
        assert_eq!(
            parse_number("0b1010"),
            Ok(("", HumlValue::Number(HumlNumber::Integer(10))))
        );
        assert_eq!(
            parse_number("nan"),
            Ok(("", HumlValue::Number(HumlNumber::Nan)))
        );
        assert_eq!(
            parse_number("inf"),
            Ok(("", HumlValue::Number(HumlNumber::Infinity(true))))
        );
        assert_eq!(
            parse_number("-inf"),
            Ok(("", HumlValue::Number(HumlNumber::Infinity(false))))
        );
    }

    #[test]
    fn test_parse_boolean() {
        assert_eq!(parse_boolean("true"), Ok(("", HumlValue::Boolean(true))));
        assert_eq!(parse_boolean("false"), Ok(("", HumlValue::Boolean(false))));
    }

    #[test]
    fn test_parse_null() {
        assert_eq!(parse_null("null"), Ok(("", HumlValue::Null)));
    }

    #[test]
    fn test_parse_inline_list() {
        if let Ok((_, HumlValue::List(items))) = parse_inline_list("1, 2, \"three\"") {
            assert_eq!(items.len(), 3);
            assert_eq!(items[0], HumlValue::Number(HumlNumber::Integer(1)));
            assert_eq!(items[1], HumlValue::Number(HumlNumber::Integer(2)));
            assert_eq!(items[2], HumlValue::String("three".to_string()));
        } else {
            panic!("Failed to parse inline list");
        }
    }

    #[test]
    fn test_parse_inline_dict() {
        if let Ok((_, HumlValue::Dict(dict))) = parse_inline_dict("foo: 1, bar: \"two\"") {
            assert_eq!(dict.len(), 2);
            assert_eq!(
                dict.get("foo"),
                Some(&HumlValue::Number(HumlNumber::Integer(1)))
            );
            assert_eq!(dict.get("bar"), Some(&HumlValue::String("two".to_string())));
        } else {
            panic!("Failed to parse inline dict");
        }
    }

    #[test]
    fn test_empty_containers() {
        assert_eq!(
            parse_empty_list("[]"),
            Ok(("", HumlValue::List(Vec::new())))
        );
        assert_eq!(
            parse_empty_dict("{}"),
            Ok(("", HumlValue::Dict(HashMap::new())))
        );
    }

    #[test]
    fn test_version_parsing() {
        let input = "%HUML v0.1.0\n\"hello\"";
        if let Ok((_, doc)) = parse_huml(input) {
            assert_eq!(doc.version, Some("0.1.0".to_string()));
            assert_eq!(doc.root, HumlValue::String("hello".to_string()));
        } else {
            panic!("Failed to parse document with version");
        }
    }

    #[test]
    fn test_simple_document() {
        let input = r#""hello world""#;
        if let Ok((_, doc)) = parse_huml(input) {
            assert_eq!(doc.root, HumlValue::String("hello world".to_string()));
        } else {
            panic!("Failed to parse simple document");
        }
    }

    #[test]
    fn test_multiline_list() {
        let input = "- 1\n- 2\n- \"three\"";
        match parse_huml(input) {
            Ok((_, doc)) => {
                if let HumlValue::List(items) = doc.root {
                    assert_eq!(items.len(), 3);
                    assert_eq!(items[0], HumlValue::Number(HumlNumber::Integer(1)));
                    assert_eq!(items[1], HumlValue::Number(HumlNumber::Integer(2)));
                    assert_eq!(items[2], HumlValue::String("three".to_string()));
                } else {
                    panic!("Expected list, got {:?}", doc.root);
                }
            }
            Err(e) => panic!("Failed to parse multiline list: {:?}", e),
        }
    }

    #[test]
    fn test_multiline_dict() {
        let input = "foo: 1\nbar: \"two\"";
        match parse_huml(input) {
            Ok((_, doc)) => {
                if let HumlValue::Dict(dict) = doc.root {
                    assert_eq!(dict.len(), 2);
                    assert_eq!(
                        dict.get("foo"),
                        Some(&HumlValue::Number(HumlNumber::Integer(1)))
                    );
                    assert_eq!(dict.get("bar"), Some(&HumlValue::String("two".to_string())));
                } else {
                    panic!("Expected dict, got {:?}", doc.root);
                }
            }
            Err(e) => panic!("Failed to parse multiline dict: {:?}", e),
        }
    }

    #[test]
    fn test_multiline_string_preserve() {
        let input = "description: ```\n  Line 1\n   Line 2\n```";
        match parse_huml(input) {
            Ok((_, doc)) => {
                if let HumlValue::Dict(dict) = doc.root {
                    if let Some(HumlValue::String(s)) = dict.get("description") {
                        assert_eq!(s, "Line 1\n Line 2");
                    } else {
                        panic!("Expected string value, got {:?}", dict.get("description"));
                    }
                } else {
                    panic!("Expected dict, got {:?}", doc.root);
                }
            }
            Err(e) => panic!("Failed to parse multiline string: {:?}", e),
        }
    }

    #[test]
    fn test_comprehensive_huml() {
        let input = r#"%HUML v0.1.0
# Root configuration
app_name: "HUML Parser Test"
version: "1.0.0"
debug: true
timeout: 30.5
retry_count: 5
infinity_val: +inf
nan_val: nan
null_val: null

# Numbers in different formats
numbers::
  decimal: 123
  negative: -456
  float: 3.14159
  scientific: 1.23e10
  hex: 0xFF
  octal: 0o755
  binary: 0b1010
  with_underscores: 1_000_000

# Strings
strings::
  simple: "Hello, World!"
  escaped: "Line with \"quotes\" and \\backslash"
  multiline_preserve: ```
    First line with  extra spaces
      Second line indented
    Third line
  ```
  multiline_strip: """
    First line
      Second line
    Third line
  """

# Collections
lists::
  inline:: 1, 2, 3, "four"
  multiline::
    - "first"
    - 42
    - true
    - null
    - ::
      nested: "value"

empty_list:: []
empty_dict:: {}

# Nested structures
database::
  host: "localhost"
  port: 5432
  credentials::
    username: "admin"
    password: "secret"
  settings::
    timeout: 30
    pool_size: 10
    ssl: true
"#;

        match parse_huml(input) {
            Ok((remaining, doc)) => {
                println!("Remaining input: {:?}", remaining);
                println!("Remaining length: {}", remaining.len());
                if !remaining.trim().is_empty() {
                    println!(
                        "First 100 chars of remaining: {:?}",
                        &remaining[..remaining.len().min(100)]
                    );
                }
                assert!(remaining.trim().is_empty());
                assert_eq!(doc.version, Some("0.1.0".to_string()));

                if let HumlValue::Dict(root) = doc.root {
                    // Test basic scalars
                    assert_eq!(
                        root.get("app_name"),
                        Some(&HumlValue::String("HUML Parser Test".to_string()))
                    );
                    assert_eq!(root.get("debug"), Some(&HumlValue::Boolean(true)));
                    assert_eq!(
                        root.get("timeout"),
                        Some(&HumlValue::Number(HumlNumber::Float(30.5)))
                    );
                    assert_eq!(
                        root.get("retry_count"),
                        Some(&HumlValue::Number(HumlNumber::Integer(5)))
                    );
                    assert_eq!(
                        root.get("infinity_val"),
                        Some(&HumlValue::Number(HumlNumber::Infinity(true)))
                    );
                    assert_eq!(
                        root.get("nan_val"),
                        Some(&HumlValue::Number(HumlNumber::Nan))
                    );
                    assert_eq!(root.get("null_val"), Some(&HumlValue::Null));

                    // Test numbers section
                    if let Some(HumlValue::Dict(numbers)) = root.get("numbers") {
                        assert_eq!(
                            numbers.get("decimal"),
                            Some(&HumlValue::Number(HumlNumber::Integer(123)))
                        );
                        assert_eq!(
                            numbers.get("negative"),
                            Some(&HumlValue::Number(HumlNumber::Integer(-456)))
                        );
                        assert_eq!(
                            numbers.get("float"),
                            Some(&HumlValue::Number(HumlNumber::Float(3.14159)))
                        );
                        assert_eq!(
                            numbers.get("hex"),
                            Some(&HumlValue::Number(HumlNumber::Integer(255)))
                        );
                        assert_eq!(
                            numbers.get("octal"),
                            Some(&HumlValue::Number(HumlNumber::Integer(493)))
                        );
                        assert_eq!(
                            numbers.get("binary"),
                            Some(&HumlValue::Number(HumlNumber::Integer(10)))
                        );
                        assert_eq!(
                            numbers.get("with_underscores"),
                            Some(&HumlValue::Number(HumlNumber::Integer(1000000)))
                        );
                    } else {
                        panic!("Expected numbers dict");
                    }

                    // Test strings section
                    if let Some(HumlValue::Dict(strings)) = root.get("strings") {
                        assert_eq!(
                            strings.get("simple"),
                            Some(&HumlValue::String("Hello, World!".to_string()))
                        );
                        assert_eq!(
                            strings.get("escaped"),
                            Some(&HumlValue::String(
                                "Line with \"quotes\" and \\backslash".to_string()
                            ))
                        );

                        if let Some(HumlValue::String(preserve)) = strings.get("multiline_preserve")
                        {
                            assert!(preserve.contains("  extra spaces"));
                            assert!(preserve.contains("  Second line indented"));
                        } else {
                            panic!("Expected multiline_preserve string");
                        }

                        if let Some(HumlValue::String(strip)) = strings.get("multiline_strip") {
                            assert_eq!(strip, "First line\nSecond line\nThird line");
                        } else {
                            panic!("Expected multiline_strip string");
                        }
                    } else {
                        panic!("Expected strings dict");
                    }

                    // Test collections
                    if let Some(HumlValue::Dict(lists)) = root.get("lists") {
                        if let Some(HumlValue::List(inline)) = lists.get("inline") {
                            assert_eq!(inline.len(), 4);
                            assert_eq!(inline[0], HumlValue::Number(HumlNumber::Integer(1)));
                            assert_eq!(inline[3], HumlValue::String("four".to_string()));
                        } else {
                            panic!("Expected inline list");
                        }

                        if let Some(HumlValue::List(multiline)) = lists.get("multiline") {
                            assert_eq!(multiline.len(), 5);
                            assert_eq!(multiline[0], HumlValue::String("first".to_string()));
                            assert_eq!(multiline[1], HumlValue::Number(HumlNumber::Integer(42)));
                            assert_eq!(multiline[2], HumlValue::Boolean(true));
                            assert_eq!(multiline[3], HumlValue::Null);

                            if let HumlValue::Dict(nested) = &multiline[4] {
                                assert_eq!(
                                    nested.get("nested"),
                                    Some(&HumlValue::String("value".to_string()))
                                );
                            } else {
                                panic!("Expected nested dict in list");
                            }
                        } else {
                            panic!("Expected multiline list");
                        }
                    } else {
                        panic!("Expected lists dict");
                    }

                    // Test empty containers
                    assert_eq!(root.get("empty_list"), Some(&HumlValue::List(Vec::new())));
                    assert_eq!(
                        root.get("empty_dict"),
                        Some(&HumlValue::Dict(HashMap::new()))
                    );

                    // Test nested structures
                    if let Some(HumlValue::Dict(database)) = root.get("database") {
                        assert_eq!(
                            database.get("host"),
                            Some(&HumlValue::String("localhost".to_string()))
                        );
                        assert_eq!(
                            database.get("port"),
                            Some(&HumlValue::Number(HumlNumber::Integer(5432)))
                        );

                        if let Some(HumlValue::Dict(credentials)) = database.get("credentials") {
                            assert_eq!(
                                credentials.get("username"),
                                Some(&HumlValue::String("admin".to_string()))
                            );
                            assert_eq!(
                                credentials.get("password"),
                                Some(&HumlValue::String("secret".to_string()))
                            );
                        } else {
                            panic!("Expected credentials dict");
                        }

                        if let Some(HumlValue::Dict(settings)) = database.get("settings") {
                            assert_eq!(
                                settings.get("timeout"),
                                Some(&HumlValue::Number(HumlNumber::Integer(30)))
                            );
                            assert_eq!(
                                settings.get("pool_size"),
                                Some(&HumlValue::Number(HumlNumber::Integer(10)))
                            );
                            assert_eq!(settings.get("ssl"), Some(&HumlValue::Boolean(true)));
                        } else {
                            panic!("Expected settings dict");
                        }
                    } else {
                        panic!("Expected database dict");
                    }
                } else {
                    panic!("Expected root dict, got {:?}", doc.root);
                }
            }
            Err(e) => panic!("Failed to parse comprehensive HUML: {:?}", e),
        }
    }

    #[test]
    fn test_kitchensink_huml() {
        let input = include_str!("../test.huml");

        match parse_huml(input) {
            Ok((remaining, doc)) => {
                println!("Remaining input: {:?}", remaining);
                println!("Remaining length: {}", remaining.len());
                if !remaining.trim().is_empty() {
                    println!(
                        "First 100 chars of remaining: {:?}",
                        &remaining[..remaining.len().min(100)]
                    );
                }
                assert!(remaining.trim().is_empty());
                assert_eq!(doc.version, None); // test.huml has no version header

                if let HumlValue::Dict(root) = doc.root {
                    // Test foo_one section
                    if let Some(HumlValue::Dict(foo_one)) = root.get("foo_one") {
                        // Test basic scalars
                        assert_eq!(
                            foo_one.get("foo_string"),
                            Some(&HumlValue::String("bar_value".to_string()))
                        );
                        assert_eq!(
                            foo_one.get("bar_string"),
                            Some(&HumlValue::String("baz with spaces".to_string()))
                        );
                        assert_eq!(
                            foo_one.get("baz_int"),
                            Some(&HumlValue::Number(HumlNumber::Integer(42)))
                        );
                        assert_eq!(
                            foo_one.get("qux_float"),
                            Some(&HumlValue::Number(HumlNumber::Float(3.14159)))
                        );
                        assert_eq!(foo_one.get("quux_bool"), Some(&HumlValue::Boolean(true)));
                        assert_eq!(foo_one.get("corge_bool"), Some(&HumlValue::Boolean(false)));
                        assert_eq!(foo_one.get("grault_null"), Some(&HumlValue::Null));

                        // Test foo_integers section
                        if let Some(HumlValue::Dict(foo_integers)) = foo_one.get("foo_integers") {
                            assert_eq!(
                                foo_integers.get("bar_positive"),
                                Some(&HumlValue::Number(HumlNumber::Integer(1234567)))
                            );
                            assert_eq!(
                                foo_integers.get("baz_negative"),
                                Some(&HumlValue::Number(HumlNumber::Integer(-987654)))
                            );
                            assert_eq!(
                                foo_integers.get("qux_zero"),
                                Some(&HumlValue::Number(HumlNumber::Integer(0)))
                            );
                            assert_eq!(
                                foo_integers.get("quux_underscore"),
                                Some(&HumlValue::Number(HumlNumber::Integer(1_000_000)))
                            );
                            assert_eq!(
                                foo_integers.get("corge_hex"),
                                Some(&HumlValue::Number(HumlNumber::Integer(0xDEADBEEF)))
                            );
                            assert_eq!(
                                foo_integers.get("grault_octal"),
                                Some(&HumlValue::Number(HumlNumber::Integer(0o777)))
                            );
                            assert_eq!(
                                foo_integers.get("garply_binary"),
                                Some(&HumlValue::Number(HumlNumber::Integer(0b1010101)))
                            );
                        } else {
                            panic!("Expected foo_integers dict");
                        }

                        // Test foo_floats section
                        if let Some(HumlValue::Dict(foo_floats)) = foo_one.get("foo_floats") {
                            assert_eq!(
                                foo_floats.get("bar_simple"),
                                Some(&HumlValue::Number(HumlNumber::Float(123.456)))
                            );
                            assert_eq!(
                                foo_floats.get("baz_negative"),
                                Some(&HumlValue::Number(HumlNumber::Float(-78.90)))
                            );
                            assert_eq!(
                                foo_floats.get("qux_scientific"),
                                Some(&HumlValue::Number(HumlNumber::Float(1.23e10)))
                            );
                            assert_eq!(
                                foo_floats.get("quux_scientific_neg"),
                                Some(&HumlValue::Number(HumlNumber::Float(-4.56e-7)))
                            );
                            assert_eq!(
                                foo_floats.get("corge_zero"),
                                Some(&HumlValue::Number(HumlNumber::Float(0.0)))
                            );
                        } else {
                            panic!("Expected foo_floats dict");
                        }

                        // Test foo_strings section
                        if let Some(HumlValue::Dict(foo_strings)) = foo_one.get("foo_strings") {
                            assert_eq!(
                                foo_strings.get("bar_empty"),
                                Some(&HumlValue::String("".to_string()))
                            );
                            assert_eq!(
                                foo_strings.get("baz_spaces"),
                                Some(&HumlValue::String("   spaces   ".to_string()))
                            );
                            assert_eq!(
                                foo_strings.get("qux_escaped"),
                                Some(&HumlValue::String(
                                    "Hello \"World\" with 'quotes'".to_string()
                                ))
                            );
                            assert_eq!(
                                foo_strings.get("quux_path"),
                                Some(&HumlValue::String("C:\\path\\to\\file.txt".to_string()))
                            );
                            assert_eq!(
                                foo_strings.get("corge_unicode"),
                                Some(&HumlValue::String("Unicode: αβγδε 中文 🚀".to_string()))
                            );
                        } else {
                            panic!("Expected foo_strings dict");
                        }
                    } else {
                        panic!("Expected foo_one dict");
                    }

                    // Test foo_two section
                    if let Some(HumlValue::Dict(foo_two)) = root.get("foo_two") {
                        // Test inline lists
                        if let Some(HumlValue::List(foo_inline_list)) =
                            foo_two.get("foo_inline_list")
                        {
                            assert_eq!(foo_inline_list.len(), 5);
                            assert_eq!(
                                foo_inline_list[0],
                                HumlValue::Number(HumlNumber::Integer(1))
                            );
                            assert_eq!(
                                foo_inline_list[4],
                                HumlValue::Number(HumlNumber::Integer(5))
                            );
                        } else {
                            panic!("Expected foo_inline_list");
                        }

                        if let Some(HumlValue::List(bar_inline_list)) =
                            foo_two.get("bar_inline_list")
                        {
                            assert_eq!(bar_inline_list.len(), 3);
                            assert_eq!(bar_inline_list[0], HumlValue::String("alpha".to_string()));
                            assert_eq!(bar_inline_list[1], HumlValue::String("beta".to_string()));
                            assert_eq!(bar_inline_list[2], HumlValue::String("gamma".to_string()));
                        } else {
                            panic!("Expected bar_inline_list");
                        }

                        // Test inline dicts
                        if let Some(HumlValue::Dict(quux_inline_dict)) =
                            foo_two.get("quux_inline_dict")
                        {
                            assert_eq!(
                                quux_inline_dict.get("foo"),
                                Some(&HumlValue::String("bar".to_string()))
                            );
                            assert_eq!(
                                quux_inline_dict.get("baz"),
                                Some(&HumlValue::Number(HumlNumber::Integer(123)))
                            );
                            assert_eq!(
                                quux_inline_dict.get("qux"),
                                Some(&HumlValue::Boolean(true))
                            );
                        } else {
                            panic!("Expected quux_inline_dict");
                        }

                        // Test empty collections
                        assert_eq!(
                            foo_two.get("foo_empty_list"),
                            Some(&HumlValue::List(Vec::new()))
                        );
                        assert_eq!(
                            foo_two.get("bar_empty_dict"),
                            Some(&HumlValue::Dict(HashMap::new()))
                        );

                        // Test multiline list
                        if let Some(HumlValue::List(foo_list)) = foo_two.get("foo_list") {
                            assert_eq!(foo_list.len(), 7);
                            assert_eq!(foo_list[0], HumlValue::String("first_item".to_string()));
                            assert_eq!(foo_list[3], HumlValue::Null);
                            assert_eq!(foo_list[4], HumlValue::Number(HumlNumber::Integer(42)));
                            assert_eq!(foo_list[5], HumlValue::Boolean(true));
                            assert_eq!(foo_list[6], HumlValue::Boolean(false));
                        } else {
                            panic!("Expected foo_list");
                        }

                        // Test special keys
                        if let Some(HumlValue::Dict(foo_special_keys)) =
                            foo_two.get("foo_special_keys")
                        {
                            assert_eq!(
                                foo_special_keys.get("quoted-key"),
                                Some(&HumlValue::String("quoted_value".to_string()))
                            );
                            assert_eq!(
                                foo_special_keys.get("key with spaces"),
                                Some(&HumlValue::String("spaced_value".to_string()))
                            );
                            assert_eq!(
                                foo_special_keys.get("key.with.dots"),
                                Some(&HumlValue::String("dotted_value".to_string()))
                            );
                        } else {
                            panic!("Expected foo_special_keys dict");
                        }
                    } else {
                        panic!("Expected foo_two dict");
                    }

                    // Test foo_three section
                    if let Some(HumlValue::Dict(foo_three)) = root.get("foo_three") {
                        // Test multiline strings
                        if let Some(HumlValue::String(multiline_preserved)) =
                            foo_three.get("foo_multiline_preserved")
                        {
                            assert!(multiline_preserved.contains("Preserved formatting"));
                            assert!(multiline_preserved.contains("  With different indentation"));
                        } else {
                            panic!("Expected foo_multiline_preserved");
                        }

                        if let Some(HumlValue::String(multiline_stripped)) =
                            foo_three.get("bar_multiline_stripped")
                        {
                            assert!(multiline_stripped.contains("Stripped formatting"));
                            assert!(multiline_stripped.contains("This will be normalized"));
                        } else {
                            panic!("Expected bar_multiline_stripped");
                        }

                        // Test boolean variations
                        if let Some(HumlValue::Dict(foo_booleans)) = foo_three.get("foo_booleans") {
                            assert_eq!(
                                foo_booleans.get("bar_true"),
                                Some(&HumlValue::Boolean(true))
                            );
                            assert_eq!(
                                foo_booleans.get("baz_false"),
                                Some(&HumlValue::Boolean(false))
                            );
                            assert_eq!(
                                foo_booleans.get("qux_TRUE"),
                                Some(&HumlValue::Boolean(true))
                            );
                            assert_eq!(
                                foo_booleans.get("quux_FALSE"),
                                Some(&HumlValue::Boolean(false))
                            );
                        } else {
                            panic!("Expected foo_booleans dict");
                        }

                        // Test null variations
                        if let Some(HumlValue::Dict(foo_nulls)) = foo_three.get("foo_nulls") {
                            assert_eq!(foo_nulls.get("bar_null"), Some(&HumlValue::Null));
                            assert_eq!(foo_nulls.get("baz_NULL"), Some(&HumlValue::Null));
                            assert_eq!(foo_nulls.get("qux_Null"), Some(&HumlValue::Null));
                        } else {
                            panic!("Expected foo_nulls dict");
                        }

                        // Test complex nesting
                        if let Some(HumlValue::Dict(foo_complex_nesting)) =
                            foo_three.get("foo_complex_nesting")
                        {
                            if let Some(HumlValue::Dict(bar_level1)) =
                                foo_complex_nesting.get("bar_level1")
                            {
                                if let Some(HumlValue::Dict(baz_level2)) =
                                    bar_level1.get("baz_level2")
                                {
                                    if let Some(HumlValue::Dict(qux_level3)) =
                                        baz_level2.get("qux_level3")
                                    {
                                        if let Some(HumlValue::Dict(quux_level4)) =
                                            qux_level3.get("quux_level4")
                                        {
                                            assert_eq!(
                                                quux_level4.get("corge_deep_value"),
                                                Some(&HumlValue::String("very_deep".to_string()))
                                            );
                                        } else {
                                            panic!("Expected quux_level4 dict");
                                        }
                                    } else {
                                        panic!("Expected qux_level3 dict");
                                    }
                                } else {
                                    panic!("Expected baz_level2 dict");
                                }
                            } else {
                                panic!("Expected bar_level1 dict");
                            }
                        } else {
                            panic!("Expected foo_complex_nesting dict");
                        }

                        // Test edge cases
                        if let Some(HumlValue::Dict(foo_edge_cases)) =
                            foo_three.get("foo_edge_cases")
                        {
                            assert_eq!(
                                foo_edge_cases.get(""),
                                Some(&HumlValue::String("empty_key".to_string()))
                            );
                            assert_eq!(
                                foo_edge_cases.get(" "),
                                Some(&HumlValue::String("space_key".to_string()))
                            );
                            assert_eq!(
                                foo_edge_cases.get("123"),
                                Some(&HumlValue::String("numeric_string_key".to_string()))
                            );
                            assert_eq!(
                                foo_edge_cases.get("true"),
                                Some(&HumlValue::String("boolean_string_key".to_string()))
                            );
                            assert_eq!(
                                foo_edge_cases.get("null"),
                                Some(&HumlValue::String("null_string_key".to_string()))
                            );
                        } else {
                            panic!("Expected foo_edge_cases dict");
                        }
                    } else {
                        panic!("Expected foo_three dict");
                    }

                    // Test foo_final section
                    if let Some(HumlValue::Dict(foo_final)) = root.get("foo_final") {
                        if let Some(HumlValue::Dict(foo_final_test)) =
                            foo_final.get("foo_final_test")
                        {
                            if let Some(HumlValue::List(bar_everything)) =
                                foo_final_test.get("bar_everything")
                            {
                                assert!(bar_everything.len() >= 3);

                                // Check first complex item
                                if let HumlValue::Dict(first_item) = &bar_everything[0] {
                                    assert_eq!(
                                        first_item.get("string_val"),
                                        Some(&HumlValue::String("test".to_string()))
                                    );
                                    assert_eq!(
                                        first_item.get("int_val"),
                                        Some(&HumlValue::Number(HumlNumber::Integer(42)))
                                    );
                                    assert_eq!(
                                        first_item.get("bool_val"),
                                        Some(&HumlValue::Boolean(true))
                                    );
                                    assert_eq!(first_item.get("null_val"), Some(&HumlValue::Null));
                                } else {
                                    panic!("Expected first item to be dict");
                                }

                                // Check simple string item
                                assert_eq!(
                                    bar_everything[1],
                                    HumlValue::String("simple_string_item".to_string())
                                );

                                // Check number item
                                assert_eq!(
                                    bar_everything[2],
                                    HumlValue::Number(HumlNumber::Integer(999))
                                );
                            } else {
                                panic!("Expected bar_everything list");
                            }
                        } else {
                            panic!("Expected foo_final_test dict");
                        }
                    } else {
                        panic!("Expected foo_final dict");
                    }
                } else {
                    panic!("Expected root dict, got {:?}", doc.root);
                }
            }
            Err(e) => panic!("Failed to parse kitchensink HUML: {:?}", e),
        }
    }

    #[test]
    fn test_nested_dict_format() {
        // Test the exact format that's failing in the serde example
        let input = r#"database::
  host: "localhost"
  port: 5432
  name: "myapp_db"
  ssl: false"#;

        match parse_huml(input) {
            Ok((remaining, doc)) => {
                println!("Remaining: '{}'", remaining);
                println!("Document: {:#?}", doc);

                if let HumlValue::Dict(dict) = doc.root {
                    assert_eq!(dict.len(), 1);

                    if let Some(HumlValue::Dict(db)) = dict.get("database") {
                        assert_eq!(
                            db.get("host"),
                            Some(&HumlValue::String("localhost".to_string()))
                        );
                        assert_eq!(
                            db.get("port"),
                            Some(&HumlValue::Number(HumlNumber::Integer(5432)))
                        );
                        assert_eq!(
                            db.get("name"),
                            Some(&HumlValue::String("myapp_db".to_string()))
                        );
                        assert_eq!(db.get("ssl"), Some(&HumlValue::Boolean(false)));
                    } else {
                        panic!("Expected database dict, got {:?}", dict.get("database"));
                    }
                } else {
                    panic!("Expected root dict, got {:?}", doc.root);
                }
            }
            Err(e) => panic!("Failed to parse nested dict: {:?}", e),
        }
    }

    #[test]
    fn test_full_serde_example() {
        // Test the full example from serde_example.rs that's failing
        let input = r#"app_name: "My Awesome App"
port: 8080
debug: true
features:: "auth", "logging", "metrics", "caching"
database::
  host: "localhost"
  port: 5432
  name: "myapp_db"
  ssl: false"#;

        match parse_huml(input) {
            Ok((remaining, doc)) => {
                println!("Remaining: '{}'", remaining);
                println!("Document: {:#?}", doc);

                if let HumlValue::Dict(dict) = doc.root {
                    // Should have 5 keys: app_name, port, debug, features, database
                    println!("Dict keys: {:?}", dict.keys().collect::<Vec<_>>());

                    assert_eq!(
                        dict.get("app_name"),
                        Some(&HumlValue::String("My Awesome App".to_string()))
                    );
                    assert_eq!(
                        dict.get("port"),
                        Some(&HumlValue::Number(HumlNumber::Integer(8080)))
                    );
                    assert_eq!(dict.get("debug"), Some(&HumlValue::Boolean(true)));

                    // Check features list
                    if let Some(HumlValue::List(features)) = dict.get("features") {
                        assert_eq!(features.len(), 4);
                        assert_eq!(features[0], HumlValue::String("auth".to_string()));
                        assert_eq!(features[1], HumlValue::String("logging".to_string()));
                        assert_eq!(features[2], HumlValue::String("metrics".to_string()));
                        assert_eq!(features[3], HumlValue::String("caching".to_string()));
                    } else {
                        panic!("Expected features list, got {:?}", dict.get("features"));
                    }

                    // Check database dict
                    if let Some(HumlValue::Dict(db)) = dict.get("database") {
                        assert_eq!(
                            db.get("host"),
                            Some(&HumlValue::String("localhost".to_string()))
                        );
                        assert_eq!(
                            db.get("port"),
                            Some(&HumlValue::Number(HumlNumber::Integer(5432)))
                        );
                        assert_eq!(
                            db.get("name"),
                            Some(&HumlValue::String("myapp_db".to_string()))
                        );
                        assert_eq!(db.get("ssl"), Some(&HumlValue::Boolean(false)));
                    } else {
                        panic!("Expected database dict, got {:?}", dict.get("database"));
                    }
                } else {
                    panic!("Expected root dict, got {:?}", doc.root);
                }
            }
            Err(e) => panic!("Failed to parse full serde example: {:?}", e),
        }
    }
}
