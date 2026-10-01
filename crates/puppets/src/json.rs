//! JSON the way Python's `json` module reads and writes it.
//!
//! The goal log is written by `puppets.py` and read back by both implementations, and the
//! read commands print parts of it verbatim, so key order, number spelling and string
//! escaping have to match `json.loads` and `json.dumps` exactly. `serde_json` keeps keys
//! sorted unless its `preserve_order` feature is on, and turning that on here would turn it
//! on for the `hydra` package too whenever the workspace builds together.

use std::fmt::Write as _;

/// A Python `str`: a sequence of code points, which unlike a Rust `String` may hold a lone
/// UTF-16 surrogate. `json.loads` reads `"\ud800"` into one and `json.dumps` writes it back,
/// so the reader has to carry it. Ordering is by code point, as Python's is.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct PyStr(Vec<u32>);

fn is_surrogate(code: u32) -> bool {
    (0xd800..0xe000).contains(&code)
}

impl PyStr {
    pub fn new() -> PyStr {
        PyStr(Vec::new())
    }

    /// The code points, each a Unicode scalar value or a surrogate.
    pub fn code_points(&self) -> &[u32] {
        &self.0
    }

    /// `len(s)`.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn push_code(&mut self, code: u32) {
        self.0.push(code);
    }

    pub fn push_str(&mut self, text: &str) {
        self.0.extend(text.chars().map(u32::from));
    }

    pub fn push(&mut self, other: &PyStr) {
        self.0.extend_from_slice(&other.0);
    }

    /// `s[:n]`.
    pub fn head(&self, n: usize) -> PyStr {
        PyStr(self.0.iter().take(n).copied().collect())
    }

    /// `s.encode("utf-8")`: `None` where Python raises `UnicodeEncodeError`, on a surrogate.
    pub fn to_utf8(&self) -> Option<String> {
        self.0.iter().map(|code| char::from_u32(*code)).collect()
    }

    /// For an error message: surrogates become U+FFFD.
    pub fn lossy(&self) -> String {
        self.0
            .iter()
            .map(|code| char::from_u32(*code).unwrap_or('\u{fffd}'))
            .collect()
    }
}

impl From<&str> for PyStr {
    fn from(text: &str) -> PyStr {
        PyStr(text.chars().map(u32::from).collect())
    }
}

impl From<String> for PyStr {
    fn from(text: String) -> PyStr {
        PyStr::from(text.as_str())
    }
}

impl PartialEq<str> for PyStr {
    fn eq(&self, other: &str) -> bool {
        self.0.iter().copied().eq(other.chars().map(u32::from))
    }
}

impl PartialEq<&str> for PyStr {
    fn eq(&self, other: &&str) -> bool {
        *self == **other
    }
}

/// One JSON value, with Python's distinction between an int and a float kept.
#[derive(Clone, Debug)]
pub enum Value {
    Null,
    Bool(bool),
    Int(i128),
    /// An integer too wide for `i128`, kept as its decimal digits with a leading `-` when
    /// negative. Never more than 4,300 digits, see `MAX_INT_DIGITS`.
    BigInt(String),
    Float(f64),
    Str(PyStr),
    List(Vec<Value>),
    Dict(Dict),
}

impl Value {
    /// A string value from Rust text.
    pub fn text(text: &str) -> Value {
        Value::Str(PyStr::from(text))
    }
}

/// A JSON object in insertion order. Setting a key that is already present keeps its
/// position, as a Python dict does.
#[derive(Clone, Debug, Default)]
pub struct Dict(Vec<(PyStr, Value)>);

impl Dict {
    pub fn new() -> Dict {
        Dict(Vec::new())
    }

    pub fn get(&self, key: &str) -> Option<&Value> {
        self.0.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }

    pub fn get_key(&self, key: &PyStr) -> Option<&Value> {
        self.0.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }

    pub fn set(&mut self, key: &str, value: Value) {
        self.set_key(PyStr::from(key), value);
    }

    pub fn set_key(&mut self, key: PyStr, value: Value) {
        match self.0.iter_mut().find(|(k, _)| *k == key) {
            Some(slot) => slot.1 = value,
            None => self.0.push((key, value)),
        }
    }

    /// Append without looking for the key. Only for output built from keys that
    /// `json.dumps` would print twice, such as a task id `1` beside a task id `"1"`.
    pub fn push(&mut self, key: PyStr, value: Value) {
        self.0.push((key, value));
    }

    pub fn iter(&self) -> impl Iterator<Item = (&PyStr, &Value)> {
        self.0.iter().map(|(k, v)| (k, v))
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// Why a document did not parse. Each names the exception `json.loads` raises.
#[derive(Debug, PartialEq, Eq)]
pub enum ParseError {
    /// `JSONDecodeError`.
    Decode,
    /// `ValueError`: an integer past `MAX_INT_DIGITS`. Not a `JSONDecodeError`, so code that
    /// catches only that class lets this one through.
    IntDigits,
    /// `UnicodeDecodeError`, from `loads_bytes` only.
    Unicode,
    /// `RecursionError`: lists and objects nested past `MAX_NESTING`. Not a `ValueError`,
    /// so in Python it escapes even the code that treats every other failure as a bad line.
    Recursion,
}

impl ParseError {
    /// The Python exception class.
    pub fn class(&self) -> &'static str {
        match self {
            ParseError::Decode => "JSONDecodeError",
            ParseError::IntDigits => "ValueError",
            ParseError::Unicode => "UnicodeDecodeError",
            ParseError::Recursion => "RecursionError",
        }
    }
}

/// Python refuses to turn a decimal string of more digits than this into an int: the default
/// of `sys.set_int_max_str_digits`, which `puppets.py` leaves alone.
pub const MAX_INT_DIGITS: usize = 4300;

/// How deep lists and objects may nest. Python has no such number: its decoder raises
/// `RecursionError` when the C stack runs out, which on the machine this was written on was
/// 116,221 levels inside a goal log line. This is a round number below that, so the two
/// agree on every line nested less deeply than this and on every line past Python's limit.
pub const MAX_NESTING: usize = 100_000;

/// Parse a document as `json.loads(str)` does.
pub fn loads(text: &PyStr) -> Result<Value, ParseError> {
    let mut parser = Parser {
        s: text.code_points(),
        i: 0,
        depth: 0,
    };
    parser.ws();
    let value = parser.value()?;
    parser.ws();
    if parser.i != parser.s.len() {
        return Err(ParseError::Decode);
    }
    Ok(value)
}

/// `json.loads(str)` on text that is already a Rust string.
pub fn loads_str(text: &str) -> Result<Value, ParseError> {
    loads(&PyStr::from(text))
}

/// Parse a document as `json.loads(bytes)` does: the encoding is the one
/// `json.detect_encoding` names, decoded with the `surrogatepass` error handler.
pub fn loads_bytes(raw: &[u8]) -> Result<Value, ParseError> {
    loads(&decode(raw).ok_or(ParseError::Unicode)?)
}

/// What `json.detect_encoding` chooses between.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Encoding {
    Utf8,
    /// UTF-8 behind a byte order mark, which the codec drops.
    Utf8Sig,
    Utf16 {
        big_endian: bool,
    },
    Utf32 {
        big_endian: bool,
    },
}

/// `json.detect_encoding`, and how many bytes of byte order mark the codec it names drops.
/// A JSON text starts with two ASCII characters, so the zero bytes among its first four say
/// which of UTF-16 and UTF-32 it is and in which byte order. The order of the tests is
/// Python's: the UTF-32 marks first, since the little-endian one starts with UTF-16's.
fn detect_encoding(b: &[u8]) -> (Encoding, usize) {
    if b.starts_with(&[0, 0, 0xfe, 0xff]) {
        return (Encoding::Utf32 { big_endian: true }, 4);
    }
    if b.starts_with(&[0xff, 0xfe, 0, 0]) {
        return (Encoding::Utf32 { big_endian: false }, 4);
    }
    if b.starts_with(&[0xfe, 0xff]) {
        return (Encoding::Utf16 { big_endian: true }, 2);
    }
    if b.starts_with(&[0xff, 0xfe]) {
        return (Encoding::Utf16 { big_endian: false }, 2);
    }
    if b.starts_with(&[0xef, 0xbb, 0xbf]) {
        return (Encoding::Utf8Sig, 3);
    }
    let encoding = if b.len() >= 4 {
        if b[0] == 0 {
            if b[1] != 0 {
                Encoding::Utf16 { big_endian: true }
            } else {
                Encoding::Utf32 { big_endian: true }
            }
        } else if b[1] == 0 {
            if b[2] != 0 || b[3] != 0 {
                Encoding::Utf16 { big_endian: false }
            } else {
                Encoding::Utf32 { big_endian: false }
            }
        } else {
            Encoding::Utf8
        }
    } else if b.len() == 2 && b[0] == 0 {
        Encoding::Utf16 { big_endian: true }
    } else if b.len() == 2 && b[1] == 0 {
        Encoding::Utf16 { big_endian: false }
    } else {
        Encoding::Utf8
    };
    (encoding, 0)
}

/// `raw.decode(json.detect_encoding(raw), "surrogatepass")`, `None` where that raises.
fn decode(raw: &[u8]) -> Option<PyStr> {
    let (encoding, mark) = detect_encoding(raw);
    let body = &raw[mark..];
    match encoding {
        Encoding::Utf8 | Encoding::Utf8Sig => decode_utf8(body),
        Encoding::Utf16 { big_endian } => decode_utf16(body, big_endian),
        Encoding::Utf32 { big_endian } => decode_utf32(body, big_endian),
    }
}

/// UTF-8 under `surrogatepass`: strict, except that the three bytes a surrogate would
/// encode to decode to that surrogate.
fn decode_utf8(mut raw: &[u8]) -> Option<PyStr> {
    let mut out = PyStr::new();
    loop {
        match std::str::from_utf8(raw) {
            Ok(text) => {
                out.push_str(text);
                return Some(out);
            }
            Err(err) => {
                let (good, bad) = raw.split_at(err.valid_up_to());
                out.push_str(std::str::from_utf8(good).ok()?);
                let [a, b, c, ..] = *bad else { return None };
                if a & 0xf0 != 0xe0 || b & 0xc0 != 0x80 || c & 0xc0 != 0x80 {
                    return None;
                }
                let code =
                    (u32::from(a & 0x0f) << 12) | (u32::from(b & 0x3f) << 6) | u32::from(c & 0x3f);
                if !is_surrogate(code) {
                    return None;
                }
                out.push_code(code);
                raw = &bad[3..];
            }
        }
    }
}

/// UTF-16 under `surrogatepass`: a pair is one code point, a surrogate without its partner
/// stands as itself, and a trailing odd byte raises.
fn decode_utf16(raw: &[u8], big_endian: bool) -> Option<PyStr> {
    let (pairs, odd) = raw.as_chunks::<2>();
    if !odd.is_empty() {
        return None;
    }
    let units: Vec<u32> = pairs
        .iter()
        .map(|pair| {
            u32::from(if big_endian {
                u16::from_be_bytes(*pair)
            } else {
                u16::from_le_bytes(*pair)
            })
        })
        .collect();
    let mut out = PyStr::new();
    let mut i = 0;
    while i < units.len() {
        let unit = units[i];
        let low = units.get(i + 1).copied();
        match low {
            Some(low) if (0xd800..0xdc00).contains(&unit) && (0xdc00..0xe000).contains(&low) => {
                out.push_code(0x10000 + ((unit - 0xd800) << 10) + (low - 0xdc00));
                i += 2;
            }
            _ => {
                out.push_code(unit);
                i += 1;
            }
        }
    }
    Some(out)
}

/// UTF-32 under `surrogatepass`: a surrogate stands as itself, a unit past U+10FFFF and a
/// trailing partial unit raise.
fn decode_utf32(raw: &[u8], big_endian: bool) -> Option<PyStr> {
    let (units, partial) = raw.as_chunks::<4>();
    if !partial.is_empty() {
        return None;
    }
    let mut out = PyStr::new();
    for unit in units {
        let code = if big_endian {
            u32::from_be_bytes(*unit)
        } else {
            u32::from_le_bytes(*unit)
        };
        if code > 0x10ffff {
            return None;
        }
        out.push_code(code);
    }
    Some(out)
}

struct Parser<'a> {
    s: &'a [u32],
    i: usize,
    depth: usize,
}

impl Parser<'_> {
    fn peek(&self) -> Option<char> {
        // A surrogate is no character the grammar names, so U+FFFD stands for it here; inside
        // a string the code point itself is copied, not this.
        self.s
            .get(self.i)
            .map(|code| char::from_u32(*code).unwrap_or('\u{fffd}'))
    }

    fn ws(&mut self) {
        while matches!(self.peek(), Some(' ' | '\t' | '\n' | '\r')) {
            self.i += 1;
        }
    }

    fn starts_with(&self, word: &str) -> bool {
        let rest = &self.s[self.i..];
        rest.len() >= word.len()
            && word
                .chars()
                .map(u32::from)
                .eq(rest[..word.len()].iter().copied())
    }

    fn eat(&mut self, word: &str) -> bool {
        if self.starts_with(word) {
            self.i += word.len();
            true
        } else {
            false
        }
    }

    fn value(&mut self) -> Result<Value, ParseError> {
        match self.peek() {
            Some('"') => Ok(Value::Str(self.string()?)),
            Some('{') => self.nested(Parser::object),
            Some('[') => self.nested(Parser::array),
            Some('n') if self.eat("null") => Ok(Value::Null),
            Some('t') if self.eat("true") => Ok(Value::Bool(true)),
            Some('f') if self.eat("false") => Ok(Value::Bool(false)),
            Some('N') if self.eat("NaN") => Ok(Value::Float(f64::NAN)),
            Some('I') if self.eat("Infinity") => Ok(Value::Float(f64::INFINITY)),
            Some('-') if self.eat("-Infinity") => Ok(Value::Float(f64::NEG_INFINITY)),
            Some('-' | '0'..='9') => self.number(),
            _ => Err(ParseError::Decode),
        }
    }

    fn nested(
        &mut self,
        container: fn(&mut Self) -> Result<Value, ParseError>,
    ) -> Result<Value, ParseError> {
        if self.depth == MAX_NESTING {
            return Err(ParseError::Recursion);
        }
        self.depth += 1;
        let value = container(self);
        self.depth -= 1;
        value
    }

    fn digits(&mut self) -> usize {
        let start = self.i;
        while matches!(self.peek(), Some('0'..='9')) {
            self.i += 1;
        }
        self.i - start
    }

    fn number(&mut self) -> Result<Value, ParseError> {
        let start = self.i;
        if self.peek() == Some('-') {
            self.i += 1;
        }
        let int_digits = match self.peek() {
            Some('0') => {
                self.i += 1;
                1
            }
            Some('1'..='9') => self.digits(),
            _ => return Err(ParseError::Decode),
        };
        let mut is_float = false;
        if self.peek() == Some('.')
            && matches!(self.s.get(self.i + 1), Some(d) if (0x30..=0x39).contains(d))
        {
            self.i += 1;
            self.digits();
            is_float = true;
        }
        if matches!(self.peek(), Some('e' | 'E')) {
            let mark = self.i;
            self.i += 1;
            if matches!(self.peek(), Some('+' | '-')) {
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
        // Every code point of a number is ASCII.
        let text: String = self.s[start..self.i]
            .iter()
            .filter_map(|code| char::from_u32(*code))
            .collect();
        if is_float {
            return text
                .parse::<f64>()
                .map(Value::Float)
                .map_err(|_| ParseError::Decode);
        }
        if int_digits > MAX_INT_DIGITS {
            return Err(ParseError::IntDigits);
        }
        Ok(match text.parse::<i128>() {
            Ok(n) => Value::Int(n),
            Err(_) => Value::BigInt(text),
        })
    }

    fn hex4(&mut self) -> Result<u32, ParseError> {
        let chunk = self.s.get(self.i..self.i + 4).ok_or(ParseError::Decode)?;
        let mut unit = 0;
        for code in chunk {
            let digit = char::from_u32(*code)
                .and_then(|c| c.to_digit(16))
                .ok_or(ParseError::Decode)?;
            unit = unit * 16 + digit;
        }
        self.i += 4;
        Ok(unit)
    }

    fn string(&mut self) -> Result<PyStr, ParseError> {
        self.i += 1;
        let mut out = PyStr::new();
        loop {
            let code = *self.s.get(self.i).ok_or(ParseError::Decode)?;
            self.i += 1;
            match code {
                0x22 => return Ok(out),
                0x5c => {
                    let escape = self.peek().ok_or(ParseError::Decode)?;
                    self.i += 1;
                    match escape {
                        '"' => out.push_code(0x22),
                        '\\' => out.push_code(0x5c),
                        '/' => out.push_code(0x2f),
                        'b' => out.push_code(0x8),
                        'f' => out.push_code(0xc),
                        'n' => out.push_code(0xa),
                        'r' => out.push_code(0xd),
                        't' => out.push_code(0x9),
                        'u' => {
                            let unit = self.hex4()?;
                            // A high surrogate joins a low one that follows as an escape;
                            // anything else leaves it standing alone, as Python does.
                            if (0xd800..0xdc00).contains(&unit) && self.starts_with("\\u") {
                                let mark = self.i;
                                self.i += 2;
                                let low = self.hex4()?;
                                if (0xdc00..0xe000).contains(&low) {
                                    out.push_code(
                                        0x10000 + ((unit - 0xd800) << 10) + (low - 0xdc00),
                                    );
                                    continue;
                                }
                                self.i = mark;
                            }
                            out.push_code(unit);
                        }
                        _ => return Err(ParseError::Decode),
                    }
                }
                // A raw control character.
                0..0x20 => return Err(ParseError::Decode),
                other => out.push_code(other),
            }
        }
    }

    fn object(&mut self) -> Result<Value, ParseError> {
        self.i += 1;
        let mut dict = Dict::new();
        self.ws();
        if self.peek() == Some('}') {
            self.i += 1;
            return Ok(Value::Dict(dict));
        }
        loop {
            if self.peek() != Some('"') {
                return Err(ParseError::Decode);
            }
            let key = self.string()?;
            self.ws();
            if self.peek() != Some(':') {
                return Err(ParseError::Decode);
            }
            self.i += 1;
            self.ws();
            let value = self.value()?;
            dict.set_key(key, value);
            self.ws();
            match self.peek() {
                Some(',') => {
                    self.i += 1;
                    self.ws();
                }
                Some('}') => {
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
        if self.peek() == Some(']') {
            self.i += 1;
            return Ok(Value::List(items));
        }
        loop {
            items.push(self.value()?);
            self.ws();
            match self.peek() {
                Some(',') => {
                    self.i += 1;
                    self.ws();
                }
                Some(']') => {
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
/// `None` is `separators=(",", ":")`. The output is ASCII, so it is a Rust string whatever
/// the value holds.
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

fn write_str(out: &mut String, s: &PyStr) {
    out.push('"');
    for code in s.code_points() {
        match *code {
            0x22 => out.push_str("\\\""),
            0x5c => out.push_str("\\\\"),
            0xa => out.push_str("\\n"),
            0xd => out.push_str("\\r"),
            0x9 => out.push_str("\\t"),
            0x8 => out.push_str("\\b"),
            0xc => out.push_str("\\f"),
            // Printable ASCII; the arms above took the quote and the backslash.
            code @ 0x20..=0x7e => out.push(code as u8 as char),
            code @ 0x10000.. => {
                let v = code - 0x10000;
                let _ = write!(
                    out,
                    "\\u{:04x}\\u{:04x}",
                    0xd800 + (v >> 10),
                    0xdc00 + (v & 0x3ff)
                );
            }
            // The rest of the BMP, a lone surrogate included: its own escape.
            code => {
                let _ = write!(out, "\\u{code:04x}");
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
        let value = loads_str(r#"{"a":1,"b":2,"a":3}"#).unwrap();
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
            "-NaN",
            "+1",
            ".5",
            "\u{b}1",
            "\"\\ud800\\uzzzz\"",
            "\"\\u12\"",
        ] {
            assert_eq!(loads_str(bad).err(), Some(ParseError::Decode), "{bad:?}");
        }
    }

    /// Each expectation is what `json.dumps(json.loads(text))` prints under Python 3.14.
    #[test]
    fn a_lone_surrogate_is_read_and_written_back() {
        for (text, want) in [
            (r#""\ud800""#, r#""\ud800""#),
            (r#""\ud800\u00e9""#, r#""\ud800\u00e9""#),
            (r#""\udc00\ud800""#, r#""\udc00\ud800""#),
            (r#""\ud800\ud800\udc00""#, r#""\ud800\ud800\udc00""#),
            (r#""\uD83d\uDE00""#, r#""\ud83d\ude00""#),
            (r#"{"\udfff":["\ud800"]}"#, r#"{"\udfff":["\ud800"]}"#),
        ] {
            let value = loads_str(text).unwrap();
            assert_eq!(dumps(&value, None), want, "{text}");
        }
        let Value::Str(pair) = loads_str(r#""\ud83d\ude00""#).unwrap() else {
            panic!("not a string");
        };
        assert_eq!(pair.code_points(), [0x1f600]);
        assert_eq!(pair.to_utf8().as_deref(), Some("\u{1f600}"));
        let Value::Str(lone) = loads_str(r#""a\ud800""#).unwrap() else {
            panic!("not a string");
        };
        assert_eq!(lone.len(), 2);
        assert_eq!(lone.to_utf8(), None);
    }

    /// Each expectation is `json.dumps(json.loads(raw))` under Python 3.14, `None` where
    /// `json.loads` raises `UnicodeDecodeError`.
    #[test]
    fn bytes_decode_as_detect_encoding_says() {
        let cases: [(&[u8], Option<&str>); 20] = [
            (b"\"\xed\xa0\x80\"", Some(r#""\ud800""#)),
            (b"\"\xed\xbf\xbf\xed\xa0\x80\"", Some(r#""\udfff\ud800""#)),
            (b"\"\xed\xa0\"", None),
            (b"\"\xc0\x80\"", None),
            (b"\"\xe0\x80\x80\"", None),
            (b"\"\xf4\x90\x80\x80\"", None),
            (b"\x00\"\xd8\x00\x00\"", Some(r#""\ud800""#)),
            (b"\x00\"\xdc\x00\xd8\x00\x00\"", Some(r#""\udc00\ud800""#)),
            (b"\x00\"\xd8\x3d\xde\x00\x00\"", Some(r#""\ud83d\ude00""#)),
            (b"\x00\"\x00a\x00\"\x00", None),
            (
                b"\x00\x00\x00\"\x00\x00\xd8\x00\x00\x00\x00\"",
                Some(r#""\ud800""#),
            ),
            (b"\x00\x00\x00\"\x00\x11\x00\x00\x00\x00\x00\"", None),
            (b"\x00\x00\x00\"\x00\x00\x00\"\x00", None),
            (b"\"\x00\x00\x00a\x00\x00\x00\"\x00\x00\x00", Some(r#""a""#)),
            (b"\"\x00a\x00\"\x00", Some(r#""a""#)),
            (b"\xff\xfe\"\x00\"\x00", Some(r#""""#)),
            (
                b"\xff\xfe\x00\x00\"\x00\x00\x00\"\x00\x00\x00",
                Some(r#""""#),
            ),
            (b"\xef\xbb\xbf\"\"", Some(r#""""#)),
            (b"1\x00", Some("1")),
            (b"\x001", Some("1")),
        ];
        for (raw, want) in cases {
            match want {
                Some(want) => assert_eq!(dumps(&loads_bytes(raw).unwrap(), None), want, "{raw:?}"),
                None => assert_eq!(loads_bytes(raw).err(), Some(ParseError::Unicode), "{raw:?}"),
            }
        }
        for raw in [
            &b"\xef\xbb\xbf\xef\xbb\xbf\"\""[..],
            b"\xfe\xff\xfe\xff\x00\"\x00\"",
            b"\x00",
            b"",
            b"\xfe\xff",
            b"\xff\xfe\x00\x00",
        ] {
            assert_eq!(loads_bytes(raw).err(), Some(ParseError::Decode), "{raw:?}");
        }
    }

    #[test]
    fn an_integer_past_4300_digits_is_refused_and_a_float_is_not() {
        let at = "1".repeat(MAX_INT_DIGITS);
        let past = "1".repeat(MAX_INT_DIGITS + 1);
        assert!(matches!(loads_str(&at), Ok(Value::BigInt(_))));
        assert!(matches!(loads_str(&format!("-{at}")), Ok(Value::BigInt(_))));
        assert_eq!(loads_str(&past).err(), Some(ParseError::IntDigits));
        assert_eq!(
            loads_str(&format!("[-{past}]")).err(),
            Some(ParseError::IntDigits)
        );
        assert!(matches!(
            loads_str(&format!("{past}.5")),
            Ok(Value::Float(f)) if f.is_infinite()
        ));
        assert!(matches!(
            loads_str(&format!("{past}e1")),
            Ok(Value::Float(f)) if f.is_infinite()
        ));
    }

    /// Run on a thread with room for it, as `main` runs every command.
    #[test]
    fn nesting_past_the_cap_is_a_recursion_error() {
        let nested = |depth: usize| format!("{}{}", "[".repeat(depth), "]".repeat(depth));
        let checks = std::thread::Builder::new()
            .stack_size(1 << 30)
            .spawn(move || {
                assert!(loads_str(&nested(MAX_NESTING)).is_ok());
                assert_eq!(
                    loads_str(&nested(MAX_NESTING + 1)).err(),
                    Some(ParseError::Recursion)
                );
                let objects = format!(
                    "{}1{}",
                    r#"{"a":"#.repeat(MAX_NESTING + 1),
                    "}".repeat(MAX_NESTING + 1)
                );
                assert_eq!(loads_str(&objects).err(), Some(ParseError::Recursion));
                // Depth is how deep, not how many: siblings do not add up.
                assert!(loads_str(&format!("[{}]", "[],".repeat(1000) + "[]")).is_ok());
            })
            .unwrap();
        checks.join().unwrap();
    }

    #[test]
    fn non_ascii_is_escaped_as_ensure_ascii_does() {
        let value = loads_str("\"\u{e9}\u{7f}\u{2028}\u{1f600}\"").unwrap();
        assert_eq!(dumps(&value, None), r#""\u00e9\u007f\u2028\ud83d\ude00""#);
    }

    #[test]
    fn indent_matches_python_layout() {
        let value = loads_str(r#"{"a":[],"b":{},"c":[1,{"x":null}],"n":NaN}"#).unwrap();
        let want = "{\n  \"a\": [],\n  \"b\": {},\n  \"c\": [\n    1,\n    {\n      \"x\": null\n    }\n  ],\n  \"n\": NaN\n}";
        assert_eq!(dumps(&value, Some(2)), want);
    }
}
