//! JSON the way Python's `json` module reads and writes it.
//!
//! The Python helper wrote goal logs this one still reads, the grader's Python parses what
//! `show` prints, and the read commands print parts of a log verbatim, so key order, number
//! spelling and string
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
/// of `sys.set_int_max_str_digits`, which the Python helper left alone.
pub const MAX_INT_DIGITS: usize = 4300;

/// How deep lists and objects may nest. Python has no such number: its decoder raises
/// `RecursionError` when the C stack runs out, which on the machine this was written on was
/// 116,221 levels inside a goal log line. This is a round number below that, so the two
/// agree on every line nested less deeply than this and on every line past Python's limit.
pub const MAX_NESTING: usize = 100_000;

/// Parse a document as `json.loads(str)` does.
pub fn loads(text: &PyStr) -> Result<Value, ParseError> {
    loads_explained(text).map_err(|(err, _)| err)
}

/// `json.loads(str)`, and for a `JSONDecodeError` what `str(err)` says: the message and
/// where, as `Expecting value: line 1 column 1 (char 0)`. The messages and positions are
/// those of CPython 3.14's C scanner.
pub fn loads_explained(text: &PyStr) -> Result<Value, (ParseError, String)> {
    let mut parser = Parser::new(text);
    parser.ws();
    let parsed = parser.value().and_then(|value| {
        parser.ws();
        if parser.i != parser.s.len() {
            return parser.bad("Extra data", parser.i);
        }
        Ok(value)
    });
    parsed.map_err(|err| (err, parser.explain()))
}

/// `json.JSONDecoder().raw_decode(str)`: one value from the first character, with no
/// whitespace skipped before it and anything at all allowed after it. Returns the value and
/// where it ended, or the error as `loads_explained` gives it.
pub fn raw_decode(text: &PyStr) -> Result<(Value, usize), (ParseError, String)> {
    let mut parser = Parser::new(text);
    match parser.value() {
        Ok(value) => Ok((value, parser.i)),
        Err(err) => Err((err, parser.explain())),
    }
}

/// `json.loads(str)` on text that is already a Rust string.
pub fn loads_str(text: &str) -> Result<Value, ParseError> {
    loads(&PyStr::from(text))
}

/// Parse a document as `json.loads(bytes)` does: the encoding is the one
/// `json.detect_encoding` names, decoded with the `surrogatepass` error handler.
pub fn loads_bytes(raw: &[u8]) -> Result<Value, ParseError> {
    loads_bytes_explained(raw).map_err(|(err, _)| err)
}

/// `json.loads(bytes)`, and what `str(err)` says for the two classes a caller prints: a
/// `JSONDecodeError` as `loads_explained` gives it, and a `UnicodeDecodeError` as
/// `'utf-8' codec can't decode byte 0xff in position 0: invalid start byte`. The grader
/// names a record it skips with that text.
pub fn loads_bytes_explained(raw: &[u8]) -> Result<Value, (ParseError, String)> {
    let text = decode(raw).map_err(|message| (ParseError::Unicode, message))?;
    loads_explained(&text)
}

/// `json.loads(bytes, object_pairs_hook=...)` as far as the pairs go: every object keeps
/// each pair it was written with, in order, so a key written twice is there twice and
/// `Dict::get` finds the first. `Dict::set_key` over the pairs in order gives the dict
/// Python builds from them. The errors are those of `loads_bytes_explained`.
pub fn loads_bytes_pairs_explained(raw: &[u8]) -> Result<Value, (ParseError, String)> {
    let text = decode(raw).map_err(|message| (ParseError::Unicode, message))?;
    let mut parser = Parser::new(&text);
    parser.pairs = true;
    parser.ws();
    let parsed = parser.value().and_then(|value| {
        parser.ws();
        if parser.i != parser.s.len() {
            return parser.bad("Extra data", parser.i);
        }
        Ok(value)
    });
    parsed.map_err(|err| (err, parser.explain()))
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

/// `str(UnicodeDecodeError)`: the codec, the bytes `start..end` it stopped on, and why.
fn decode_error(codec: &str, raw: &[u8], start: usize, end: usize, reason: &str) -> String {
    if end == start + 1 {
        format!(
            "'{codec}' codec can't decode byte 0x{:02x} in position {start}: {reason}",
            raw[start]
        )
    } else {
        format!(
            "'{codec}' codec can't decode bytes in position {start}-{}: {reason}",
            end - 1
        )
    }
}

/// `raw.decode(json.detect_encoding(raw), "surrogatepass")`, or `str(err)` where that
/// raises. The UTF-8 codec behind a byte order mark is handed the bytes after the mark, so
/// its positions count from there; the UTF-16 and UTF-32 codecs skip the mark themselves
/// and count from the first byte.
fn decode(raw: &[u8]) -> Result<PyStr, String> {
    let (encoding, mark) = detect_encoding(raw);
    match encoding {
        Encoding::Utf8 | Encoding::Utf8Sig => decode_utf8(&raw[mark..]),
        Encoding::Utf16 { big_endian } => decode_utf16(raw, mark, big_endian),
        Encoding::Utf32 { big_endian } => decode_utf32(raw, mark, big_endian),
    }
}

/// UTF-8 under `surrogatepass`: strict, except that the three bytes a surrogate would
/// encode to decode to that surrogate.
fn decode_utf8(raw: &[u8]) -> Result<PyStr, String> {
    let mut out = PyStr::new();
    let mut at = 0;
    loop {
        let err = match std::str::from_utf8(&raw[at..]) {
            Ok(text) => {
                out.push_str(text);
                return Ok(out);
            }
            Err(err) => err,
        };
        let start = at + err.valid_up_to();
        out.push_str(&String::from_utf8_lossy(&raw[at..start]));
        if let [a, b, c, ..] = raw[start..]
            && a & 0xf0 == 0xe0
            && b & 0xc0 == 0x80
            && c & 0xc0 == 0x80
        {
            let code =
                (u32::from(a & 0x0f) << 12) | (u32::from(b & 0x3f) << 6) | u32::from(c & 0x3f);
            if is_surrogate(code) {
                out.push_code(code);
                at = start + 3;
                continue;
            }
        }
        // CPython stops on the same bytes Rust does, the longest prefix of a sequence that
        // could still have been valid, and names three reasons for them.
        let (end, reason) = match err.error_len() {
            None => (raw.len(), "unexpected end of data"),
            Some(1) if !(0xc2..=0xf4).contains(&raw[start]) => (start + 1, "invalid start byte"),
            Some(len) => (start + len, "invalid continuation byte"),
        };
        return Err(decode_error("utf-8", raw, start, end, reason));
    }
}

/// UTF-16 under `surrogatepass`: a pair is one code point, a surrogate without its partner
/// stands as itself, and a trailing odd byte raises.
fn decode_utf16(raw: &[u8], mark: usize, big_endian: bool) -> Result<PyStr, String> {
    let (pairs, odd) = raw[mark..].as_chunks::<2>();
    if !odd.is_empty() {
        let codec = if big_endian { "utf-16-be" } else { "utf-16-le" };
        let start = raw.len() - 1;
        return Err(decode_error(codec, raw, start, raw.len(), "truncated data"));
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
    Ok(out)
}

/// UTF-32 under `surrogatepass`: a surrogate stands as itself, a unit past U+10FFFF and a
/// trailing partial unit raise, whichever comes first.
fn decode_utf32(raw: &[u8], mark: usize, big_endian: bool) -> Result<PyStr, String> {
    let codec = if big_endian { "utf-32-be" } else { "utf-32-le" };
    let (units, partial) = raw[mark..].as_chunks::<4>();
    let mut out = PyStr::new();
    for (n, unit) in units.iter().enumerate() {
        let code = if big_endian {
            u32::from_be_bytes(*unit)
        } else {
            u32::from_le_bytes(*unit)
        };
        if code > 0x10ffff {
            let start = mark + 4 * n;
            return Err(decode_error(
                codec,
                raw,
                start,
                start + 4,
                "code point not in range(0x110000)",
            ));
        }
        out.push_code(code);
    }
    if !partial.is_empty() {
        let start = raw.len() - partial.len();
        return Err(decode_error(codec, raw, start, raw.len(), "truncated data"));
    }
    Ok(out)
}

struct Parser<'a> {
    s: &'a [u32],
    i: usize,
    depth: usize,
    /// The message and position of the `JSONDecodeError`, once one is raised.
    fail: Option<(&'static str, usize)>,
    /// Keep every pair of an object, a repeated key included, as `object_pairs_hook` is
    /// handed them.
    pairs: bool,
}

impl<'a> Parser<'a> {
    fn new(text: &'a PyStr) -> Parser<'a> {
        Parser {
            s: text.code_points(),
            i: 0,
            depth: 0,
            fail: None,
            pairs: false,
        }
    }

    /// Raise `JSONDecodeError(message, doc, at)`.
    fn bad<T>(&mut self, message: &'static str, at: usize) -> Result<T, ParseError> {
        self.fail = Some((message, at));
        Err(ParseError::Decode)
    }

    /// `str(err)` for the `JSONDecodeError` raised, empty when the failure was another class.
    fn explain(&self) -> String {
        let Some((message, at)) = self.fail else {
            return String::new();
        };
        let before = &self.s[..at.min(self.s.len())];
        let line = before.iter().filter(|code| **code == 0xa).count() + 1;
        let column = match before.iter().rposition(|code| *code == 0xa) {
            Some(newline) => at - newline,
            None => at + 1,
        };
        format!("{message}: line {line} column {column} (char {at})")
    }

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
            _ => self.bad("Expecting value", self.i),
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
            _ => return self.bad("Expecting value", start),
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
            return match text.parse::<f64>() {
                Ok(float) => Ok(Value::Float(float)),
                Err(_) => self.bad("Expecting value", start),
            };
        }
        if int_digits > MAX_INT_DIGITS {
            return Err(ParseError::IntDigits);
        }
        Ok(match text.parse::<i128>() {
            Ok(n) => Value::Int(n),
            Err(_) => Value::BigInt(text),
        })
    }

    /// The four hex digits of a `\u` escape whose `u` is at `self.i - 1`. An escape the end
    /// of the document cuts short is this error, not an unterminated string.
    fn hex4(&mut self) -> Result<u32, ParseError> {
        const MESSAGE: &str = "Invalid \\uXXXX escape";
        let escape = self.i - 1;
        if self.i + 4 > self.s.len() {
            return self.bad(MESSAGE, escape);
        }
        let mut unit = 0;
        for at in self.i..self.i + 4 {
            let Some(hex) = char::from_u32(self.s[at]).and_then(|c| c.to_digit(16)) else {
                return self.bad(MESSAGE, escape);
            };
            unit = unit * 16 + hex;
        }
        self.i += 4;
        Ok(unit)
    }

    fn string(&mut self) -> Result<PyStr, ParseError> {
        const UNTERMINATED: &str = "Unterminated string starting at";
        let begin = self.i;
        self.i += 1;
        let mut out = PyStr::new();
        loop {
            let Some(&code) = self.s.get(self.i) else {
                return self.bad(UNTERMINATED, begin);
            };
            self.i += 1;
            match code {
                0x22 => return Ok(out),
                0x5c => {
                    let Some(escape) = self.peek() else {
                        return self.bad(UNTERMINATED, begin);
                    };
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
                            // A high surrogate joins a low one that follows as an escape,
                            // when a character follows that escape too; anything else
                            // leaves it standing alone, as Python does.
                            if (0xd800..0xdc00).contains(&unit)
                                && self.i + 6 < self.s.len()
                                && self.starts_with("\\u")
                            {
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
                        _ => return self.bad("Invalid \\escape", self.i - 2),
                    }
                }
                // A raw control character.
                0..0x20 => return self.bad("Invalid control character at", self.i - 1),
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
                return self.bad("Expecting property name enclosed in double quotes", self.i);
            }
            let key = self.string()?;
            self.ws();
            if self.peek() != Some(':') {
                return self.bad("Expecting ':' delimiter", self.i);
            }
            self.i += 1;
            self.ws();
            let value = self.value()?;
            if self.pairs {
                dict.push(key, value);
            } else {
                dict.set_key(key, value);
            }
            self.ws();
            match self.peek() {
                Some(',') => {
                    let comma = self.i;
                    self.i += 1;
                    self.ws();
                    if self.peek() == Some('}') {
                        return self.bad("Illegal trailing comma before end of object", comma);
                    }
                }
                Some('}') => {
                    self.i += 1;
                    return Ok(Value::Dict(dict));
                }
                _ => return self.bad("Expecting ',' delimiter", self.i),
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
                    let comma = self.i;
                    self.i += 1;
                    self.ws();
                    if self.peek() == Some(']') {
                        return self.bad("Illegal trailing comma before end of array", comma);
                    }
                }
                Some(']') => {
                    self.i += 1;
                    return Ok(Value::List(items));
                }
                _ => return self.bad("Expecting ',' delimiter", self.i),
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

/// How `json.dumps` separates what it writes.
#[derive(Clone, Copy)]
enum Style {
    /// `separators=(",", ":")`, one line.
    Compact,
    /// The default separators, `", "` and `": "`, one line.
    Spaced,
    /// `indent=n`.
    Indent(usize),
}

/// `json.dumps` with `ensure_ascii=True`: `indent` of `Some(n)` is the pretty form,
/// `None` is `separators=(",", ":")`. The output is ASCII, so it is a Rust string whatever
/// the value holds.
pub fn dumps(value: &Value, indent: Option<usize>) -> String {
    let mut out = String::new();
    write_value(
        &mut out,
        value,
        indent.map_or(Style::Compact, Style::Indent),
        0,
    );
    out
}

/// `json.dumps(value)` with no arguments: one line, `", "` between items and `": "` after
/// a key.
pub fn dumps_default(value: &Value) -> String {
    let mut out = String::new();
    write_value(&mut out, value, Style::Spaced, 0);
    out
}

/// `json.dumps(value, ensure_ascii=False)` with default separators.
pub fn dumps_unicode_default(value: &Value) -> String {
    let mut out = String::new();
    write_value_inner(&mut out, value, Style::Spaced, 0, false);
    out
}

fn write_value(out: &mut String, value: &Value, style: Style, depth: usize) {
    write_value_inner(out, value, style, depth, true);
}

fn write_value_inner(out: &mut String, value: &Value, style: Style, depth: usize, ascii: bool) {
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
        Value::Str(s) => {
            if ascii {
                write_str(out, s);
            } else {
                write_str_unicode(out, s);
            }
        }
        Value::List(items) if items.is_empty() => out.push_str("[]"),
        Value::List(items) => {
            out.push('[');
            for (n, item) in items.iter().enumerate() {
                separate(out, n, style, depth + 1);
                write_value_inner(out, item, style, depth + 1, ascii);
            }
            close(out, style, depth);
            out.push(']');
        }
        Value::Dict(dict) if dict.is_empty() => out.push_str("{}"),
        Value::Dict(dict) => {
            out.push('{');
            for (n, (key, item)) in dict.iter().enumerate() {
                separate(out, n, style, depth + 1);
                if ascii {
                    write_str(out, key);
                } else {
                    write_str_unicode(out, key);
                }
                out.push_str(if matches!(style, Style::Compact) {
                    ":"
                } else {
                    ": "
                });
                write_value_inner(out, item, style, depth + 1, ascii);
            }
            close(out, style, depth);
            out.push('}');
        }
    }
}

fn separate(out: &mut String, n: usize, style: Style, depth: usize) {
    if n > 0 {
        out.push_str(if matches!(style, Style::Spaced) {
            ", "
        } else {
            ","
        });
    }
    if let Style::Indent(width) = style {
        out.push('\n');
        out.push_str(&" ".repeat(width * depth));
    }
}

fn close(out: &mut String, style: Style, depth: usize) {
    if let Style::Indent(width) = style {
        out.push('\n');
        out.push_str(&" ".repeat(width * depth));
    }
}

fn write_str(out: &mut String, s: &PyStr) {
    write_str_impl(out, s, true);
}

fn write_str_unicode(out: &mut String, s: &PyStr) {
    write_str_impl(out, s, false);
}

fn write_str_impl(out: &mut String, s: &PyStr, ascii: bool) {
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
            code @ 0x20..=0x7e => out.push(code as u8 as char),
            code if !ascii && code >= 0x7f && char::from_u32(code).is_some() => {
                if let Some(ch) = char::from_u32(code) {
                    out.push(ch);
                }
            }
            code @ 0x10000.. => {
                let v = code - 0x10000;
                let _ = write!(
                    out,
                    "\\u{:04x}\\u{:04x}",
                    0xd800 + (v >> 10),
                    0xdc00 + (v & 0x3ff)
                );
            }
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
    fn the_pairs_of_an_object_keep_a_key_written_twice() {
        let raw = br#"{"3": {"a": 1, "a": 2}, "3": [{"b": 1, "b": 2}]}"#;
        let kept = loads_bytes_pairs_explained(raw).unwrap();
        assert_eq!(
            dumps_default(&kept),
            r#"{"3": {"a": 1, "a": 2}, "3": [{"b": 1, "b": 2}]}"#
        );
        let plain = loads_bytes_explained(raw).unwrap();
        assert_eq!(dumps_default(&plain), r#"{"3": [{"b": 2}]}"#);
        let (err, message) = loads_bytes_pairs_explained(b"{} x").unwrap_err();
        assert_eq!(err, ParseError::Decode);
        assert_eq!(message, "Extra data: line 1 column 4 (char 3)");
    }

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

    /// Each message is `str(err)` from `json.loads(text)` under Python 3.14.
    #[test]
    fn a_decode_error_says_what_python_says_and_where() {
        for (text, want) in [
            ("", "Expecting value: line 1 column 1 (char 0)"),
            ("nul", "Expecting value: line 1 column 1 (char 0)"),
            ("-", "Expecting value: line 1 column 1 (char 0)"),
            ("[", "Expecting value: line 1 column 2 (char 1)"),
            (
                "{",
                "Expecting property name enclosed in double quotes: line 1 column 2 (char 1)",
            ),
            (
                "{\"a\"",
                "Expecting ':' delimiter: line 1 column 5 (char 4)",
            ),
            ("[1 2]", "Expecting ',' delimiter: line 1 column 4 (char 3)"),
            (
                "[1,]",
                "Illegal trailing comma before end of array: line 1 column 3 (char 2)",
            ),
            (
                "{\"a\":1,}",
                "Illegal trailing comma before end of object: line 1 column 7 (char 6)",
            ),
            ("{} x", "Extra data: line 1 column 4 (char 3)"),
            (
                "\"abc",
                "Unterminated string starting at: line 1 column 1 (char 0)",
            ),
            (
                "\"\\u1234",
                "Unterminated string starting at: line 1 column 1 (char 0)",
            ),
            ("\"a\\qb\"", "Invalid \\escape: line 1 column 3 (char 2)"),
            (
                "\"a\\u12\"",
                "Invalid \\uXXXX escape: line 1 column 4 (char 3)",
            ),
            (
                "\"\\ud800\\u12",
                "Invalid \\uXXXX escape: line 1 column 9 (char 8)",
            ),
            (
                "\"a\u{1}b\"",
                "Invalid control character at: line 1 column 3 (char 2)",
            ),
            (
                "{\"a\":1,\n \"b\"}",
                "Expecting ':' delimiter: line 2 column 5 (char 12)",
            ),
        ] {
            let (class, message) = loads_explained(&PyStr::from(text)).unwrap_err();
            assert_eq!(
                (class, message.as_str()),
                (ParseError::Decode, want),
                "{text:?}"
            );
        }
        // `raw_decode` starts at the first character and stops after one value.
        let (value, end) = raw_decode(&PyStr::from("{\"a\": 1} and more")).unwrap();
        assert_eq!((dumps(&value, None).as_str(), end), ("{\"a\":1}", 8));
        let (_, message) = raw_decode(&PyStr::from(" {}")).unwrap_err();
        assert_eq!(message, "Expecting value: line 1 column 1 (char 0)");
        // An error of another class has no such message.
        let (class, message) = loads_explained(&PyStr::from("9".repeat(5000))).unwrap_err();
        assert_eq!((class, message.as_str()), (ParseError::IntDigits, ""));
    }

    #[test]
    fn the_default_separators_put_a_space_after_each_comma_and_colon() {
        let value = loads_str(r#"{"a":[1,"é",{}],"b":{"c":null}}"#).unwrap();
        assert_eq!(
            dumps_default(&value),
            "{\"a\": [1, \"\\u00e9\", {}], \"b\": {\"c\": null}}"
        );
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

    /// Each message is what `str(err)` gave under Python 3.14.7 for `json.loads` of the
    /// same bytes. `crates/grader/tests/differential.rs` compares more of them, all behind
    /// a newline, which is why the three that end a text are here.
    #[test]
    fn a_decode_error_says_what_python_says() {
        for (raw, want) in [
            (
                &b"\xff\n"[..],
                "'utf-8' codec can't decode byte 0xff in position 0: invalid start byte",
            ),
            (
                b"{\"a\":\"\xe9\"}\n",
                "'utf-8' codec can't decode byte 0xe9 in position 6: invalid continuation byte",
            ),
            (
                b"{\"a\":\"\xe2\x82\"}\n",
                "'utf-8' codec can't decode bytes in position 6-7: invalid continuation byte",
            ),
            (
                b"\"\xe2\x82",
                "'utf-8' codec can't decode bytes in position 1-2: unexpected end of data",
            ),
            (
                b"\"\xf0\x9f\x98",
                "'utf-8' codec can't decode bytes in position 1-3: unexpected end of data",
            ),
            (
                b"\"\xc3",
                "'utf-8' codec can't decode byte 0xc3 in position 1: unexpected end of data",
            ),
            (
                b"\"\xed\xa0\"",
                "'utf-8' codec can't decode byte 0xed in position 1: invalid continuation byte",
            ),
            (
                b"\"\xc0\x80\"",
                "'utf-8' codec can't decode byte 0xc0 in position 1: invalid start byte",
            ),
            // Behind a mark the UTF-8 codec counts from after it, the wide ones from 0.
            (
                b"\xef\xbb\xbf\"\xff\"",
                "'utf-8' codec can't decode byte 0xff in position 1: invalid start byte",
            ),
            (
                b"\xff\xfe\"\x00a\x00\"",
                "'utf-16-le' codec can't decode byte 0x22 in position 6: truncated data",
            ),
            (
                b"\xfe\xff\x00\"\x00a\x00\"\x00",
                "'utf-16-be' codec can't decode byte 0x00 in position 8: truncated data",
            ),
            (
                b"\"\x00\x00\x00a\x00\x00\x00\"\x00\x00\x00\x01",
                "'utf-32-le' codec can't decode byte 0x01 in position 12: truncated data",
            ),
            (
                b"\"\x00\x00\x00\x00\x00\x11\x00\"\x00\x00\x00",
                "'utf-32-le' codec can't decode bytes in position 4-7: code point not in range(0x110000)",
            ),
            (
                b"\x00\x00\xfe\xff\x00\x00\x00\"\x00\x00",
                "'utf-32-be' codec can't decode bytes in position 8-9: truncated data",
            ),
        ] {
            let (err, message) = loads_bytes_explained(raw).unwrap_err();
            assert_eq!(err, ParseError::Unicode, "{raw:?}");
            assert_eq!(message, want, "{raw:?}");
        }
        let (err, message) = loads_bytes_explained(b"\xef\xbb\xbf{\"a\":").unwrap_err();
        assert_eq!(err, ParseError::Decode);
        assert_eq!(message, "Expecting value: line 1 column 6 (char 5)");
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
