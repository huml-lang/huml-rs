use crate::{HumlDocument, HumlNumber, HumlValue};
use std::collections::HashMap;
use std::fmt;

/// Result type used by all parser helpers. This mirrors the old `nom::IResult` interface
/// so that downstream code can continue to destructure `(remaining, value)` tuples.
pub type IResult<'a, T> = Result<(&'a str, T), ParseError>;

/// Rich parser error with line/column diagnostics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub line: usize,
    pub column: usize,
    pub message: String,
}

impl ParseError {
    fn new(line: usize, column: usize, message: impl Into<String>) -> Self {
        Self {
            line,
            column,
            message: message.into(),
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}:{} {}", self.line, self.column, self.message)
    }
}

impl std::error::Error for ParseError {}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum DataType {
    Scalar,
    EmptyDict,
    InlineDict,
    MultilineDict,
    EmptyList,
    InlineList,
    MultilineList,
}

/// Parse a complete HUML document, including the optional `%HUML` version line.
pub fn parse_huml(input: &str) -> IResult<'_, HumlDocument> {
    let mut parser = Parser::new(input);
    let doc = parser.parse_document()?;
    Ok((parser.remaining(), doc))
}

/// Parse just the root value from a HUML document snippet.
pub fn parse_document_root(input: &str) -> IResult<'_, HumlValue> {
    let mut parser = Parser::new(input);
    parser.skip_blank_lines()?;
    let root = parser.parse_root_value(false)?;
    parser.skip_blank_lines()?;
    if !parser.done() {
        return Err(parser.error("unexpected content after document root"));
    }
    Ok((parser.remaining(), root))
}

/// Parse an inline scalar (strings, numbers, bools, null, special floats).
pub fn parse_scalar(input: &str) -> IResult<'_, HumlValue> {
    let mut parser = Parser::new(input);
    let value = parser.parse_scalar_value(0)?;
    Ok((parser.remaining(), value))
}

/// Parse the shorthand empty list (`[]`).
pub fn parse_empty_list(input: &str) -> IResult<'_, HumlValue> {
    if input.trim_start().starts_with("[]") {
        let offset = input.len() - input.trim_start().len() + 2;
        Ok((&input[offset..], HumlValue::List(Vec::new())))
    } else {
        Err(ParseError::new(1, 1, "expected []"))
    }
}

/// Parse the shorthand empty dict (`{}`).
pub fn parse_empty_dict(input: &str) -> IResult<'_, HumlValue> {
    if input.trim_start().starts_with("{}") {
        let offset = input.len() - input.trim_start().len() + 2;
        Ok((&input[offset..], HumlValue::Dict(HashMap::new())))
    } else {
        Err(ParseError::new(1, 1, "expected {}"))
    }
}

/// Parse an inline list separated by commas.
pub fn parse_inline_list(input: &str) -> IResult<'_, HumlValue> {
    let mut parser = Parser::new(input);
    let value = parser.parse_inline_vector_contents(DataType::InlineList)?;
    Ok((parser.remaining(), value))
}

/// Parse an inline dict separated by commas.
pub fn parse_inline_dict(input: &str) -> IResult<'_, HumlValue> {
    let mut parser = Parser::new(input);
    let value = parser.parse_inline_vector_contents(DataType::InlineDict)?;
    Ok((parser.remaining(), value))
}

#[derive(Clone)]
struct Parser<'a> {
    input: &'a str,
    bytes: &'a [u8],
    len: usize,
    pos: usize,
    line: usize,
    line_start: usize,
}

impl<'a> Parser<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            input,
            bytes: input.as_bytes(),
            len: input.len(),
            pos: 0,
            line: 1,
            line_start: 0,
        }
    }

    fn remaining(&self) -> &'a str {
        &self.input[self.pos..]
    }

    fn done(&self) -> bool {
        self.pos >= self.len
    }

    fn starts_with(&self, pat: &str) -> bool {
        self.remaining().starts_with(pat)
    }

    fn current_byte(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn current_char(&self) -> Option<char> {
        self.remaining().chars().next()
    }

    fn advance_char(&mut self) {
        if let Some(ch) = self.current_char() {
            self.advance(ch.len_utf8());
        }
    }

    fn advance(&mut self, n: usize) {
        for _ in 0..n {
            if self.done() {
                break;
            }
            if self.bytes[self.pos] == b'\n' {
                self.pos += 1;
                self.line += 1;
                self.line_start = self.pos;
            } else {
                self.pos += 1;
            }
        }
    }

    fn column(&self) -> usize {
        self.pos - self.line_start + 1
    }

    fn error(&self, msg: impl Into<String>) -> ParseError {
        ParseError::new(self.line, self.column(), msg)
    }

    fn err<T>(&self, msg: impl Into<String>) -> Result<T, ParseError> {
        Err(self.error(msg))
    }

    fn parse_document(&mut self) -> Result<HumlDocument, ParseError> {
        if self.input.is_empty() {
            return self.err("empty document is undefined");
        }

        let version = self.parse_version_header()?;
        self.skip_blank_lines()?;
        if self.done() {
            return self.err("empty document is undefined");
        }

        let root = self.parse_root_value(true)?;
        self.skip_blank_lines()?;
        if !self.done() {
            return self.err("unexpected content after document root");
        }

        Ok(HumlDocument { version, root })
    }

    fn parse_version_header(&mut self) -> Result<Option<String>, ParseError> {
        if !self.starts_with("%HUML") {
            return Ok(None);
        }

        self.advance("%HUML".len());

        let mut version = None;
        if self.current_byte() == Some(b' ') {
            self.advance(1);
            let start = self.pos;
            while !self.done() {
                match self.current_byte() {
                    Some(b' ') | Some(b'\n') | Some(b'#') => break,
                    Some(_) => self.advance(1),
                    None => break,
                }
            }
            if self.pos > start {
                let token = &self.input[start..self.pos];
                if token.starts_with('v') {
                    let trimmed = token.trim_start_matches('v').to_string();
                    if trimmed != "0.1.0" {
                        return self.err(format!(
                            "unsupported version 'v{}'. expected 'v0.1.0'",
                            trimmed
                        ));
                    }
                    version = Some(trimmed);
                } else {
                    return self.err("invalid version token");
                }
            }
        }

        self.consume_line()?;
        Ok(version)
    }

    fn parse_root_value(&mut self, allow_version_line: bool) -> Result<HumlValue, ParseError> {
        if !allow_version_line && self.starts_with("%HUML") {
            return self.err("version directive not allowed in this context");
        }

        if self.get_cur_indent() != 0 {
            return self.err("root element must not be indented");
        }

        if self.starts_with("::") {
            return self.err("'::' indicator not allowed at document root");
        }

        if self.starts_with(":") && !self.has_key_value_pair() {
            return self.err("':' indicator not allowed at document root");
        }

        match self.get_root_type() {
            DataType::InlineDict => self.parse_inline_vector_contents(DataType::InlineDict),
            DataType::MultilineDict => self.parse_multiline_dict(0),
            DataType::EmptyList => {
                self.advance(2);
                self.consume_line()?;
                Ok(HumlValue::List(Vec::new()))
            }
            DataType::EmptyDict => {
                self.advance(2);
                self.consume_line()?;
                Ok(HumlValue::Dict(HashMap::new()))
            }
            DataType::MultilineList => self.parse_multiline_list(0),
            DataType::InlineList => self.parse_inline_vector_contents(DataType::InlineList),
            DataType::Scalar => {
                let value = self.parse_scalar_value(0)?;
                self.consume_line()?;
                Ok(value)
            }
        }
    }

    fn parse_scalar_value(&mut self, key_indent: usize) -> Result<HumlValue, ParseError> {
        if self.done() {
            return self.err("unexpected end of input, expected a value");
        }

        if self.starts_with("[]") {
            self.advance(2);
            return Ok(HumlValue::List(Vec::new()));
        }
        if self.starts_with("{}") {
            self.advance(2);
            return Ok(HumlValue::Dict(HashMap::new()));
        }

        match self.current_byte().unwrap_or_default() {
            b'"' => {
                if self.starts_with("\"\"\"") {
                    let value = self.parse_multiline_string(key_indent, false)?;
                    Ok(HumlValue::String(value))
                } else {
                    let value = self.parse_string()?;
                    Ok(HumlValue::String(value))
                }
            }
            b'`' if self.starts_with("```") => {
                let value = self.parse_multiline_string(key_indent, true)?;
                Ok(HumlValue::String(value))
            }
            b't' if self.starts_with("true") => {
                self.advance(4);
                Ok(HumlValue::Boolean(true))
            }
            b'f' if self.starts_with("false") => {
                self.advance(5);
                Ok(HumlValue::Boolean(false))
            }
            b'n' if self.starts_with("null") => {
                self.advance(4);
                Ok(HumlValue::Null)
            }
            b'n' if self.starts_with("nan") => {
                self.advance(3);
                Ok(HumlValue::Number(HumlNumber::Nan))
            }
            b'i' if self.starts_with("inf") => {
                self.advance(3);
                Ok(HumlValue::Number(HumlNumber::Infinity(true)))
            }
            b'+' => {
                self.advance(1);
                if self.starts_with("inf") {
                    self.advance(3);
                    Ok(HumlValue::Number(HumlNumber::Infinity(true)))
                } else if self.current_byte().map_or(false, |c| c.is_ascii_digit()) {
                    self.pos = self.pos.saturating_sub(1);
                    let number = self.parse_number()?;
                    Ok(HumlValue::Number(number))
                } else {
                    self.err("invalid character after '+'")
                }
            }
            b'-' => {
                self.advance(1);
                if self.starts_with("inf") {
                    self.advance(3);
                    Ok(HumlValue::Number(HumlNumber::Infinity(false)))
                } else if self.current_byte().map_or(false, |c| c.is_ascii_digit()) {
                    self.pos = self.pos.saturating_sub(1);
                    let number = self.parse_number()?;
                    Ok(HumlValue::Number(number))
                } else {
                    self.err("invalid character after '-'")
                }
            }
            b if b.is_ascii_digit() => {
                let number = self.parse_number()?;
                Ok(HumlValue::Number(number))
            }
            _ => self.err(format!(
                "unexpected character '{}' when parsing value",
                self.current_byte().map(|b| b as char).unwrap_or('\u{2400}')
            )),
        }
    }

    fn parse_multiline_dict(&mut self, indent: usize) -> Result<HumlValue, ParseError> {
        let mut dict = HashMap::new();

        loop {
            self.skip_blank_lines()?;
            if self.done() {
                break;
            }

            let cur_indent = self.get_cur_indent();
            if cur_indent < indent {
                break;
            }
            if cur_indent != indent {
                return self.err(format!("bad indent {}, expected {}", cur_indent, indent));
            }

            if !self.is_key_start() {
                return self.err("expected key");
            }

            let key = self.parse_key()?;
            if dict.contains_key(&key) {
                return self.err(format!("duplicate key '{}' in dict", key));
            }

            let indicator = self.parse_indicator()?;
            let value = if indicator == ":" {
                self.assert_space("after ':'")?;
                let is_multiline_string = self.starts_with("```") || self.starts_with("\"\"\"");
                let scalar = self.parse_scalar_value(cur_indent)?;
                if !is_multiline_string {
                    self.consume_line()?;
                }
                scalar
            } else {
                self.parse_vector(indent + 2)?
            };

            dict.insert(key, value);
        }

        Ok(HumlValue::Dict(dict))
    }

    fn parse_multiline_list(&mut self, indent: usize) -> Result<HumlValue, ParseError> {
        let mut items = Vec::new();

        loop {
            self.skip_blank_lines()?;
            if self.done() {
                break;
            }

            let cur_indent = self.get_cur_indent();
            if cur_indent < indent {
                break;
            }
            if cur_indent != indent {
                return self.err(format!("bad indent {}, expected {}", cur_indent, indent));
            }

            if self.current_byte() != Some(b'-') {
                break;
            }
            self.advance(1);
            self.assert_space("after '-'")?;

            let value = if self.starts_with("::") {
                self.advance(2);
                self.parse_vector(indent + 2)?
            } else {
                let scalar = self.parse_scalar_value(indent)?;
                self.consume_line()?;
                scalar
            };

            items.push(value);
        }

        Ok(HumlValue::List(items))
    }

    fn parse_vector(&mut self, indent: usize) -> Result<HumlValue, ParseError> {
        let start_pos = self.pos;
        self.skip_spaces();

        if self.done() || self.current_byte() == Some(b'\n') || self.current_byte() == Some(b'#') {
            self.pos = start_pos;
            self.consume_line()?;
            let vector_type = self.get_multiline_vector_type(indent)?;
            let actual_indent = self.get_cur_indent();
            if actual_indent != indent {
                return self.err(format!(
                    "bad indent {} for vector, expected {}",
                    actual_indent, indent
                ));
            }
            match vector_type {
                DataType::MultilineList => self.parse_multiline_list(actual_indent),
                _ => self.parse_multiline_dict(actual_indent),
            }
        } else {
            self.pos = start_pos;
            self.assert_space("after '::'")?;

            if self.starts_with("[]") {
                self.advance(2);
                self.consume_line()?;
                return Ok(HumlValue::List(Vec::new()));
            }
            if self.starts_with("{}") {
                self.advance(2);
                self.consume_line()?;
                return Ok(HumlValue::Dict(HashMap::new()));
            }

            if self.has_inline_dict() {
                self.parse_inline_vector_contents(DataType::InlineDict)
            } else {
                self.parse_inline_vector_contents(DataType::InlineList)
            }
        }
    }

    fn get_multiline_vector_type(&mut self, indent: usize) -> Result<DataType, ParseError> {
        self.skip_blank_lines()?;
        if self.done() {
            return self.err("ambiguous empty vector after '::'. Use [] or {}.");
        }

        let cur_indent = self.get_cur_indent();
        if cur_indent < indent {
            return self.err("ambiguous empty vector after '::'. Use [] or {}.");
        }

        if self.current_byte() == Some(b'-') {
            Ok(DataType::MultilineList)
        } else {
            Ok(DataType::MultilineDict)
        }
    }

    fn parse_inline_vector_contents(&mut self, typ: DataType) -> Result<HumlValue, ParseError> {
        match typ {
            DataType::InlineDict => {
                let mut dict = HashMap::new();
                self.parse_inline_items(|parser| {
                    let key = parser.parse_key()?;
                    if parser.current_byte() != Some(b':') {
                        return parser.err("expected ':' in inline dict");
                    }
                    parser.advance(1);
                    parser.assert_space("in inline dict")?;
                    let value = parser.parse_scalar_value(0)?;
                    if dict.contains_key(&key) {
                        return parser.err(format!("duplicate key '{}' in dict", key));
                    }
                    dict.insert(key, value);
                    Ok(())
                })?;
                Ok(HumlValue::Dict(dict))
            }
            DataType::InlineList => {
                let mut items = Vec::new();
                self.parse_inline_items(|parser| {
                    let value = parser.parse_scalar_value(0)?;
                    items.push(value);
                    Ok(())
                })?;
                Ok(HumlValue::List(items))
            }
            _ => unreachable!("inline vector helper called with non-inline type"),
        }
    }

    fn parse_key(&mut self) -> Result<String, ParseError> {
        self.skip_spaces();
        if self.current_byte() == Some(b'"') {
            return self.parse_string();
        }

        let start = self.pos;
        while !self.done() {
            match self.current_byte().unwrap() {
                b if b.is_ascii_alphanumeric() || b == b'-' || b == b'_' => self.advance(1),
                _ => break,
            }
        }
        if self.pos == start {
            self.err("expected a key")
        } else {
            Ok(self.input[start..self.pos].to_string())
        }
    }

    fn parse_indicator(&mut self) -> Result<&'static str, ParseError> {
        if self.current_byte() != Some(b':') {
            return self.err("expected ':' or '::' after key");
        }
        self.advance(1);
        if self.current_byte() == Some(b':') {
            self.advance(1);
            Ok("::")
        } else {
            Ok(":")
        }
    }

    fn parse_string(&mut self) -> Result<String, ParseError> {
        if self.current_byte() != Some(b'"') {
            return self.err("expected string");
        }

        self.advance(1); // opening quote
        let mut out = String::new();
        while !self.done() {
            let ch = self
                .current_char()
                .ok_or_else(|| self.error("unexpected end of input"))?;
            match ch {
                '"' => {
                    self.advance_char();
                    return Ok(out);
                }
                '\n' => return self.err("newlines not allowed in single-line strings"),
                '\\' => {
                    self.advance_char();
                    let esc = self
                        .current_char()
                        .ok_or_else(|| self.error("incomplete escape sequence"))?;
                    match esc {
                        '"' => {
                            out.push('"');
                            self.advance_char();
                        }
                        '\\' => {
                            out.push('\\');
                            self.advance_char();
                        }
                        '/' => {
                            out.push('/');
                            self.advance_char();
                        }
                        'b' => {
                            out.push('\u{0008}');
                            self.advance_char();
                        }
                        'f' => {
                            out.push('\u{000C}');
                            self.advance_char();
                        }
                        'n' => {
                            out.push('\n');
                            self.advance_char();
                        }
                        'r' => {
                            out.push('\r');
                            self.advance_char();
                        }
                        't' => {
                            out.push('\t');
                            self.advance_char();
                        }
                        'v' => {
                            out.push('\u{000B}');
                            self.advance_char();
                        }
                        'u' => {
                            self.advance_char();
                            if self.pos + 4 > self.len {
                                return self.err("incomplete unicode escape");
                            }
                            let hex = &self.input[self.pos..self.pos + 4];
                            if !hex.chars().all(|c| c.is_ascii_hexdigit()) {
                                return self.err("invalid unicode escape digits");
                            }
                            let code_point = u32::from_str_radix(hex, 16)
                                .map_err(|_| self.error("invalid unicode escape digits"))?;
                            let decoded = std::char::from_u32(code_point)
                                .ok_or_else(|| self.error("invalid unicode scalar value"))?;
                            out.push(decoded);
                            self.advance(4);
                        }
                        _ => {
                            return Err(self.error(format!("invalid escape character '\\{}'", esc)));
                        }
                    }
                }
                _ => {
                    out.push(ch);
                    self.advance_char();
                }
            }
        }

        self.err("unclosed string")
    }

    fn parse_multiline_string(
        &mut self,
        key_indent: usize,
        preserve_spaces: bool,
    ) -> Result<String, ParseError> {
        if self.pos + 3 > self.len {
            return self.err("unterminated multiline string delimiter");
        }

        let delim = &self.input[self.pos..self.pos + 3];
        self.advance(3);
        self.consume_line()?;

        let mut out = String::new();
        loop {
            if self.done() {
                return self.err("unclosed multiline string");
            }

            let line_start = self.pos;
            let mut line_indent = 0;
            while self.current_byte() == Some(b' ') {
                line_indent += 1;
                self.advance(1);
            }

            if self.starts_with(delim) {
                if line_indent != key_indent {
                    return self.err(format!(
                        "multiline closing delimiter must be at same indentation as the key ({} spaces)",
                        key_indent
                    ));
                }
                self.advance(3);
                self.consume_line()?;

                if out.ends_with('\n') {
                    out.pop();
                }
                return Ok(out);
            }

            self.pos = line_start;
            let line_content = self.consume_line_content();

            let processed = if preserve_spaces {
                let required = key_indent + 2;
                let bytes = line_content.as_bytes();
                if bytes.len() >= required && bytes[..required].iter().all(|b| *b == b' ') {
                    line_content[required..].to_string()
                } else {
                    line_content
                }
            } else {
                line_content.trim().to_string()
            };

            out.push_str(&processed);
            out.push('\n');
        }
    }

    fn parse_number(&mut self) -> Result<HumlNumber, ParseError> {
        let start = self.pos;
        if matches!(self.current_byte(), Some(b'+') | Some(b'-')) {
            self.advance(1);
        }

        if self.starts_with("0x") {
            return self.parse_base_number(start, 16, "0x");
        }
        if self.starts_with("0o") {
            return self.parse_base_number(start, 8, "0o");
        }
        if self.starts_with("0b") {
            return self.parse_base_number(start, 2, "0b");
        }

        let mut is_float = false;
        loop {
            if self.done() {
                break;
            }
            match self.current_byte().unwrap() {
                b if b.is_ascii_digit() || b == b'_' => self.advance(1),
                b'.' => {
                    is_float = true;
                    self.advance(1);
                }
                b'e' | b'E' => {
                    is_float = true;
                    self.advance(1);
                    if matches!(self.current_byte(), Some(b'+') | Some(b'-')) {
                        self.advance(1);
                    }
                }
                _ => break,
            }
        }

        if self.pos == start
            || (self.pos == start + 1 && matches!(self.input.as_bytes()[start], b'+' | b'-'))
        {
            return self.err("invalid number literal, missing digits");
        }

        let literal = self.input[start..self.pos].replace('_', "");
        if is_float {
            literal
                .parse::<f64>()
                .map(HumlNumber::Float)
                .map_err(|_| self.error("invalid float literal"))
        } else {
            literal
                .parse::<i64>()
                .map(HumlNumber::Integer)
                .map_err(|_| self.error("invalid integer literal"))
        }
    }

    fn parse_base_number(
        &mut self,
        start: usize,
        base: u32,
        prefix: &str,
    ) -> Result<HumlNumber, ParseError> {
        self.advance(prefix.len());
        let num_start = self.pos;
        while !self.done() {
            let byte = self.current_byte().unwrap();
            let valid = match base {
                16 => byte.is_ascii_hexdigit() || byte == b'_',
                8 => (b'0'..=b'7').contains(&byte) || byte == b'_',
                2 => byte == b'0' || byte == b'1' || byte == b'_',
                _ => false,
            };
            if !valid {
                break;
            }
            self.advance(1);
        }

        if self.pos == num_start {
            return self.err("invalid number literal, requires digits after prefix");
        }

        let sign = match self.input.as_bytes()[start] {
            b'-' => -1,
            _ => 1,
        };
        let digits = self.input[num_start..self.pos].replace('_', "");
        let parsed = i64::from_str_radix(&digits, base)
            .map_err(|_| self.error("invalid digits for number literal"))?;
        Ok(HumlNumber::Integer(parsed * sign))
    }

    fn skip_blank_lines(&mut self) -> Result<(), ParseError> {
        loop {
            if self.done() {
                return Ok(());
            }

            let line_start = self.pos;
            self.skip_spaces();

            if self.done() {
                if self.pos > line_start {
                    return self.err("trailing spaces are not allowed");
                }
                return Ok(());
            }

            match self.current_byte() {
                Some(b'\n') => {
                    if self.pos > line_start {
                        return self.err("trailing spaces are not allowed");
                    }
                    self.advance(1);
                }
                Some(b'#') => {
                    self.pos = line_start;
                    self.consume_line()?;
                }
                _ => {
                    return Ok(());
                }
            }
        }
    }

    fn consume_line(&mut self) -> Result<(), ParseError> {
        let content_start = self.pos;
        self.skip_spaces();

        if self.done() || self.current_byte() == Some(b'\n') {
            if self.pos > content_start {
                return self.err("trailing spaces are not allowed");
            }
        } else if self.current_byte() == Some(b'#') {
            if self.pos == content_start
                && self.get_cur_indent() != self.pos.saturating_sub(self.line_start)
            {
                return self.err("a value must be separated from an inline comment by a space");
            }
            self.advance(1);
            match self.current_byte() {
                Some(b' ') | Some(b'\n') | None => {}
                _ => return self.err("comment hash '#' must be followed by a space"),
            }
        } else {
            return self.err("unexpected content at end of line");
        }

        let comment_end = self.pos;
        while !self.done() && self.current_byte() != Some(b'\n') {
            self.advance(1);
        }

        if self.pos > 0
            && self.bytes[self.pos.saturating_sub(1)] == b' '
            && self.pos - 1 > comment_end
        {
            return self.err("trailing spaces are not allowed");
        }

        if self.current_byte() == Some(b'\n') {
            self.advance(1);
        }

        Ok(())
    }

    fn consume_line_content(&mut self) -> String {
        let start = self.pos;
        while !self.done() && self.current_byte() != Some(b'\n') {
            self.advance(1);
        }
        let content = self.input[start..self.pos].to_string();
        if self.current_byte() == Some(b'\n') {
            self.advance(1);
        }
        content
    }

    fn assert_space(&mut self, context: &str) -> Result<(), ParseError> {
        if self.current_byte() != Some(b' ') {
            return self.err(format!("expected single space {}", context));
        }
        self.advance(1);
        if self.current_byte() == Some(b' ') {
            return self.err(format!("expected single space {}, found multiple", context));
        }
        Ok(())
    }

    fn expect_comma(&mut self) -> Result<(), ParseError> {
        self.skip_spaces();
        if self.current_byte() != Some(b',') {
            return self.err("expected a comma in inline collection");
        }
        if self.pos > 0 && self.bytes[self.pos - 1] == b' ' {
            return self.err("no spaces allowed before comma");
        }
        self.advance(1);
        self.assert_space("after comma")
    }

    fn get_cur_indent(&self) -> usize {
        let mut indent = 0;
        let mut idx = self.line_start;
        while idx < self.len && self.bytes[idx] == b' ' {
            indent += 1;
            idx += 1;
        }
        indent
    }

    fn get_root_type(&self) -> DataType {
        if self.has_key_value_pair() {
            if self.has_inline_dict_at_root() {
                return DataType::InlineDict;
            }
            return DataType::MultilineDict;
        }
        if self.starts_with("[]") {
            return DataType::EmptyList;
        }
        if self.starts_with("{}") {
            return DataType::EmptyDict;
        }
        if self.current_byte() == Some(b'-') {
            return DataType::MultilineList;
        }
        if self.has_inline_list_at_root() {
            return DataType::InlineList;
        }
        DataType::Scalar
    }

    fn has_key_value_pair(&self) -> bool {
        let mut clone = self.clone();
        clone.parse_key().is_ok() && clone.current_byte() == Some(b':')
    }

    fn has_inline_list_at_root(&self) -> bool {
        let mut pos = self.pos;
        while pos < self.len && self.bytes[pos] != b'\n' && self.bytes[pos] != b'#' {
            match self.bytes[pos] {
                b',' => return true,
                b':' => return false,
                _ => pos += 1,
            }
        }
        false
    }

    fn has_inline_dict_at_root(&self) -> bool {
        let mut pos = self.pos;
        let mut has_colon = false;
        let mut has_comma = false;
        let mut has_double_colon = false;

        while pos < self.len && self.bytes[pos] != b'\n' && self.bytes[pos] != b'#' {
            match self.bytes[pos] {
                b':' => {
                    if pos + 1 < self.len && self.bytes[pos + 1] == b':' {
                        has_double_colon = true;
                    } else {
                        has_colon = true;
                    }
                }
                b',' => has_comma = true,
                _ => {}
            }
            pos += 1;
        }

        if !(has_colon && has_comma && !has_double_colon) {
            return false;
        }

        while pos < self.len {
            while pos < self.len && self.bytes[pos] == b' ' {
                pos += 1;
            }
            if pos >= self.len {
                break;
            }
            match self.bytes[pos] {
                b'\n' => {
                    pos += 1;
                }
                b'#' => {
                    while pos < self.len && self.bytes[pos] != b'\n' {
                        pos += 1;
                    }
                    if pos < self.len && self.bytes[pos] == b'\n' {
                        pos += 1;
                    }
                }
                _ => return false,
            }
        }

        true
    }

    fn has_inline_dict(&self) -> bool {
        let mut pos = self.pos;
        while pos < self.len && self.bytes[pos] != b'\n' && self.bytes[pos] != b'#' {
            if self.bytes[pos] == b':' {
                if pos + 1 < self.len && self.bytes[pos + 1] != b':' {
                    return true;
                }
            }
            pos += 1;
        }
        false
    }

    fn is_key_start(&self) -> bool {
        matches!(self.current_byte(), Some(b'"'))
            || self
                .current_byte()
                .map_or(false, |b| b.is_ascii_alphabetic())
    }

    fn skip_spaces(&mut self) {
        while self.current_byte() == Some(b' ') {
            self.advance(1);
        }
    }

    fn parse_inline_items<F>(&mut self, mut parse_item: F) -> Result<(), ParseError>
    where
        F: FnMut(&mut Parser<'a>) -> Result<(), ParseError>,
    {
        let mut first = true;
        while !self.done()
            && self.current_byte() != Some(b'\n')
            && self.current_byte() != Some(b'#')
        {
            if !first {
                self.expect_comma()?;
            }
            first = false;
            parse_item(self)?;

            if !self.done() && self.current_byte() == Some(b' ') {
                let mut next = self.pos + 1;
                while next < self.len && self.bytes[next] == b' ' {
                    next += 1;
                }
                if next < self.len && self.bytes[next] == b',' {
                    self.skip_spaces();
                } else {
                    break;
                }
            }
        }

        self.consume_line()
    }
}
