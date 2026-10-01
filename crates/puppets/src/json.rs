//! JSON the way Python's `json` module reads and writes it.
//!
//! The goal log is written by `puppets.py` and read back by both implementations, and the
//! read commands print parts of it verbatim, so key order, number spelling and string
//! escaping have to match `json.loads` and `json.dumps` exactly. `serde_json` keeps keys
//! sorted unless its `preserve_order` feature is on, and turning that on here would turn it
//! on for the `hydra` package too whenever the workspace builds together.

use std::fmt::Write as _;

/// One JSON value, with Python's distinction between an int and a float kept.
#[derive(Clone, Debug)]
pub enum Value {
    Null,
    Bool(bool),
    Int(i128),
    /// An integer too wide for `i128`, kept as its decimal digits.
    BigInt(String),
    Float(f64),
    Str(String),
    List(Vec<Value>),
    Dict(Dict),
}

/// A JSON object in insertion order. Setting a key that is already present keeps its
/// position, as a Python dict does.
#[derive(Clone, Debug, Default)]
pub struct Dict(Vec<(String, Value)>);

impl Dict {
    pub fn new() -> Dict {
        Dict(Vec::new())
    }

    pub fn get(&self, key: &str) -> Option<&Value> {
        self.0.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }

    pub fn set(&mut self, key: &str, value: Value) {
        match self.0.iter_mut().find(|(k, _)| k == key) {
            Some(slot) => slot.1 = value,
            None => self.0.push((key.to_string(), value)),
        }
    }

    /// Append without looking for the key. Only for output built from keys that
    /// `json.dumps` would print twice, such as a task id `1` beside a task id `"1"`.
    pub fn push(&mut self, key: String, value: Value) {
        self.0.push((key, value));
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &Value)> {
        self.0.iter().map(|(k, v)| (k, v))
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// Why a document did not parse.
#[derive(Debug, PartialEq, Eq)]
pub enum ParseError {
    /// What `json.loads` raises `JSONDecodeError` for.
    Decode,
    /// Text Python accepts and this reader does not hold: a lone UTF-16 surrogate.
    Unsupported,
}

/// Parse a document as `json.loads(str)` does.
pub fn loads(text: &str) -> Result<Value, ParseError> {
    let mut parser = Parser {
        s: text.as_bytes(),
        i: 0,
    };
    parser.ws();
    let value = parser.value()?;
    parser.ws();
    if parser.i != parser.s.len() {
        return Err(ParseError::Decode);
    }
    Ok(value)
}

struct Parser<'a> {
    s: &'a [u8],
    i: usize,
}

impl Parser<'_> {
    fn peek(&self) -> Option<u8> {
        self.s.get(self.i).copied()
    }

    fn ws(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.i += 1;
        }
    }

    fn eat(&mut self, word: &str) -> bool {
        if self.s[self.i..].starts_with(word.as_bytes()) {
            self.i += word.len();
            true
        } else {
            false
        }
    }

    fn value(&mut self) -> Result<Value, ParseError> {
        match self.peek() {
            Some(b'"') => Ok(Value::Str(self.string()?)),
            Some(b'{') => self.object(),
            Some(b'[') => self.array(),
            Some(b'n') if self.eat("null") => Ok(Value::Null),
            Some(b't') if self.eat("true") => Ok(Value::Bool(true)),
            Some(b'f') if self.eat("false") => Ok(Value::Bool(false)),
            Some(b'N') if self.eat("NaN") => Ok(Value::Float(f64::NAN)),
            Some(b'I') if self.eat("Infinity") => Ok(Value::Float(f64::INFINITY)),
            Some(b'-') if self.s[self.i..].starts_with(b"-Infinity") => {
                self.i += "-Infinity".len();
                Ok(Value::Float(f64::NEG_INFINITY))
            }
            Some(b'-' | b'0'..=b'9') => self.number(),
            _ => Err(ParseError::Decode),
        }
    }

    fn digits(&mut self) -> usize {
        let start = self.i;
        while matches!(self.peek(), Some(b'0'..=b'9')) {
            self.i += 1;
        }
        self.i - start
    }

    fn number(&mut self) -> Result<Value, ParseError> {
        let start = self.i;
        if self.peek() == Some(b'-') {
            self.i += 1;
        }
        match self.peek() {
            Some(b'0') => self.i += 1,
            Some(b'1'..=b'9') => {
                self.digits();
            }
            _ => return Err(ParseError::Decode),
        }
        let mut is_float = false;
        if self.peek() == Some(b'.') && matches!(self.s.get(self.i + 1), Some(b'0'..=b'9')) {
            self.i += 1;
            self.digits();
            is_float = true;
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            let mark = self.i;
            self.i += 1;
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.i += 1;
            }
            if self.digits() == 0 {
                // Python's number pattern stops before an `e` with no digits after it,
                // and the `e` is then extra data.
                self.i = mark;
            } else {
                is_float = true;
            }
        }
        let text = std::str::from_utf8(&self.s[start..self.i]).map_err(|_| ParseError::Decode)?;
        if is_float {
            return text
                .parse::<f64>()
                .map(Value::Float)
                .map_err(|_| ParseError::Decode);
        }
        Ok(match text.parse::<i128>() {
            Ok(n) => Value::Int(n),
            Err(_) => Value::BigInt(text.to_string()),
        })
    }

    fn hex4(&mut self) -> Result<u32, ParseError> {
        let chunk = self.s.get(self.i..self.i + 4).ok_or(ParseError::Decode)?;
        if !chunk.iter().all(u8::is_ascii_hexdigit) {
            return Err(ParseError::Decode);
        }
        let text = std::str::from_utf8(chunk).map_err(|_| ParseError::Decode)?;
        self.i += 4;
        u32::from_str_radix(text, 16).map_err(|_| ParseError::Decode)
    }

    fn string(&mut self) -> Result<String, ParseError> {
        self.i += 1;
        let mut out = String::new();
        loop {
            let start = self.i;
            while let Some(b) = self.peek() {
                if b == b'"' || b == b'\\' || b < 0x20 {
                    break;
                }
                self.i += 1;
            }
            out.push_str(
                std::str::from_utf8(&self.s[start..self.i]).map_err(|_| ParseError::Decode)?,
            );
            match self.peek() {
                Some(b'"') => {
                    self.i += 1;
                    return Ok(out);
                }
                Some(b'\\') => {
                    self.i += 1;
                    let escape = self.peek().ok_or(ParseError::Decode)?;
                    self.i += 1;
                    match escape {
                        b'"' => out.push('"'),
                        b'\\' => out.push('\\'),
                        b'/' => out.push('/'),
                        b'b' => out.push('\u{8}'),
                        b'f' => out.push('\u{c}'),
                        b'n' => out.push('\n'),
                        b'r' => out.push('\r'),
                        b't' => out.push('\t'),
                        b'u' => {
                            let unit = self.hex4()?;
                            let code = if (0xd800..0xdc00).contains(&unit)
                                && self.s[self.i..].starts_with(b"\\u")
                            {
                                let mark = self.i;
                                self.i += 2;
                                let low = self.hex4()?;
                                if (0xdc00..0xe000).contains(&low) {
                                    0x10000 + ((unit - 0xd800) << 10) + (low - 0xdc00)
                                } else {
                                    self.i = mark;
                                    unit
                                }
                            } else {
                                unit
                            };
                            out.push(char::from_u32(code).ok_or(ParseError::Unsupported)?);
                        }
                        _ => return Err(ParseError::Decode),
                    }
                }
                // A raw control character, or the end of the text.
                _ => return Err(ParseError::Decode),
            }
        }
    }

    fn object(&mut self) -> Result<Value, ParseError> {
        self.i += 1;
        let mut dict = Dict::new();
        self.ws();
        if self.peek() == Some(b'}') {
            self.i += 1;
            return Ok(Value::Dict(dict));
        }
        loop {
            if self.peek() != Some(b'"') {
                return Err(ParseError::Decode);
            }
            let key = self.string()?;
            self.ws();
            if self.peek() != Some(b':') {
                return Err(ParseError::Decode);
            }
            self.i += 1;
            self.ws();
            let value = self.value()?;
            dict.set(&key, value);
            self.ws();
            match self.peek() {
                Some(b',') => {
                    self.i += 1;
                    self.ws();
                }
                Some(b'}') => {
                    self.i += 1;
                    return Ok(Value::Dict(dict));
                }
                _ => return Err(ParseError::Decode),
            }
        }
    }

    fn array(&mut self) -> Result<Value, ParseError> {
        self.i += 1;
        let mut items = Vec::new();
        self.ws();
        if self.peek() == Some(b']') {
            self.i += 1;
            return Ok(Value::List(items));
        }
        loop {
            items.push(self.value()?);
            self.ws();
            match self.peek() {
                Some(b',') => {
                    self.i += 1;
                    self.ws();
                }
                Some(b']') => {
                    self.i += 1;
                    return Ok(Value::List(items));
                }
                _ => return Err(ParseError::Decode),
            }
        }
    }
}

/// `repr(float)`: the shortest digits that round-trip, in fixed notation while the decimal
/// point sits between 10^-4 and 10^16, and in exponent notation otherwise.
pub fn float_repr(x: f64) -> String {
    if x.is_nan() {
        return "nan".into();
    }
    if x.is_infinite() {
        return if x > 0.0 { "inf".into() } else { "-inf".into() };
    }
    if x == 0.0 {
        return if x.is_sign_negative() {
            "-0.0".into()
        } else {
            "0.0".into()
        };
    }
    let sci = format!("{:e}", x.abs());
    let (mantissa, exp) = sci.split_once('e').expect("{:e} always has an exponent");
    let exp: i32 = exp.parse().expect("{:e} exponent is an integer");
    let digits: String = mantissa.chars().filter(|c| *c != '.').collect();
    let sign = if x < 0.0 { "-" } else { "" };
    let decpt = exp + 1;
    let n = digits.len() as i32;
    if decpt > -4 && decpt <= 16 {
        if decpt <= 0 {
            format!("{sign}0.{}{digits}", "0".repeat((-decpt) as usize))
        } else if decpt >= n {
            format!("{sign}{digits}{}.0", "0".repeat((decpt - n) as usize))
        } else {
            let (head, tail) = digits.split_at(decpt as usize);
            format!("{sign}{head}.{tail}")
        }
    } else {
        let (first, rest) = digits.split_at(1);
        let fraction = if rest.is_empty() {
            String::new()
        } else {
            format!(".{rest}")
        };
        let e = decpt - 1;
        let esign = if e < 0 { '-' } else { '+' };
        format!("{sign}{first}{fraction}e{esign}{:02}", e.abs())
    }
}

/// `json.dumps` with `ensure_ascii=True`: `indent` of `Some(n)` is the pretty form,
/// `None` is `separators=(",", ":")`.
pub fn dumps(value: &Value, indent: Option<usize>) -> String {
    let mut out = String::new();
    write_value(&mut out, value, indent, 0);
    out
}

fn write_value(out: &mut String, value: &Value, indent: Option<usize>, depth: usize) {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(true) => out.push_str("true"),
        Value::Bool(false) => out.push_str("false"),
        Value::Int(n) => {
            let _ = write!(out, "{n}");
        }
        Value::BigInt(digits) => out.push_str(digits),
        Value::Float(x) if x.is_nan() => out.push_str("NaN"),
        Value::Float(x) if x.is_infinite() => {
            out.push_str(if *x > 0.0 { "Infinity" } else { "-Infinity" })
        }
        Value::Float(x) => out.push_str(&float_repr(*x)),
        Value::Str(s) => write_str(out, s),
        Value::List(items) if items.is_empty() => out.push_str("[]"),
        Value::List(items) => {
            out.push('[');
            for (n, item) in items.iter().enumerate() {
                separate(out, n, indent, depth + 1);
                write_value(out, item, indent, depth + 1);
            }
            close(out, indent, depth);
            out.push(']');
        }
        Value::Dict(dict) if dict.is_empty() => out.push_str("{}"),
        Value::Dict(dict) => {
            out.push('{');
            for (n, (key, item)) in dict.iter().enumerate() {
                separate(out, n, indent, depth + 1);
                write_str(out, key);
                out.push_str(if indent.is_some() { ": " } else { ":" });
                write_value(out, item, indent, depth + 1);
            }
            close(out, indent, depth);
            out.push('}');
        }
    }
}

fn separate(out: &mut String, n: usize, indent: Option<usize>, depth: usize) {
    if n > 0 {
        out.push(',');
    }
    if let Some(width) = indent {
        out.push('\n');
        out.push_str(&" ".repeat(width * depth));
    }
}

fn close(out: &mut String, indent: Option<usize>, depth: usize) {
    if let Some(width) = indent {
        out.push('\n');
        out.push_str(&" ".repeat(width * depth));
    }
}

fn write_str(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            ' '..='~' => out.push(c),
            _ => {
                let mut units = [0u16; 2];
                for unit in c.encode_utf16(&mut units) {
                    let _ = write!(out, "\\u{unit:04x}");
                }
            }
        }
    }
    out.push('"');
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn floats_print_as_python_repr_prints_them() {
        let cases = [
            (1e16, "1e+16"),
            (1e15, "1000000000000000.0"),
            (0.0001, "0.0001"),
            (1e-5, "1e-05"),
            (1.5e-7, "1.5e-07"),
            (-2.5, "-2.5"),
            (123.456, "123.456"),
            (-0.0, "-0.0"),
            (1.7976931348623157e308, "1.7976931348623157e+308"),
        ];
        for (x, want) in cases {
            assert_eq!(float_repr(x), want, "{x}");
        }
    }

    #[test]
    fn duplicate_keys_keep_the_first_position_and_the_last_value() {
        let value = loads(r#"{"a":1,"b":2,"a":3}"#).unwrap();
        assert_eq!(dumps(&value, None), r#"{"a":3,"b":2}"#);
    }

    #[test]
    fn what_python_refuses_is_refused() {
        for bad in [
            "01",
            "1.",
            "[1,]",
            "{\"a\":1,}",
            "\"a\tb\"",
            "",
            "nul",
            "- 1",
            "1e",
        ] {
            assert_eq!(loads(bad).err(), Some(ParseError::Decode), "{bad:?}");
        }
        assert_eq!(loads("\"\\ud800\"").err(), Some(ParseError::Unsupported));
    }

    #[test]
    fn non_ascii_is_escaped_as_ensure_ascii_does() {
        let value = loads("\"\u{e9}\u{7f}\u{2028}\u{1f600}\"").unwrap();
        assert_eq!(dumps(&value, None), r#""\u00e9\u007f\u2028\ud83d\ude00""#);
    }

    #[test]
    fn indent_matches_python_layout() {
        let value = loads(r#"{"a":[],"b":{},"c":[1,{"x":null}],"n":NaN}"#).unwrap();
        let want = "{\n  \"a\": [],\n  \"b\": {},\n  \"c\": [\n    1,\n    {\n      \"x\": null\n    }\n  ],\n  \"n\": NaN\n}";
        assert_eq!(dumps(&value, Some(2)), want);
    }
}
