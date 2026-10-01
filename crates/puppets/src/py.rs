//! The few Python operations the fold and the read commands apply to log values.
//!
//! A log line is whatever JSON a writer put there, so a field `puppets.py` expects to be a
//! string can hold anything. These follow what Python does with such a value where that is
//! cheap to say, and return an error where Python raises, so a command stops at the same
//! point with the same stdout before it. Every error message starts with the name of the
//! exception class Python raises, which is what the differential test compares.

use std::cmp::Ordering;

use anyhow::{Result, bail};

use crate::json::{PyStr, Value, float_repr};
use crate::printable::is_printable;

/// `a == b`: `True == 1 == 1.0`, dicts equal regardless of key order. Inside a list or a
/// dict Python compares items with `same_key`'s rule, not this one.
pub fn eq(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::Str(x), Value::Str(y)) => x == y,
        (Value::List(x), Value::List(y)) => {
            x.len() == y.len() && x.iter().zip(y).all(|(p, q)| same_key(p, q))
        }
        (Value::Dict(x), Value::Dict(y)) => {
            x.len() == y.len()
                && x.iter()
                    .all(|(k, v)| y.get_key(k).is_some_and(|w| same_key(v, w)))
        }
        _ => match (number(a), number(b)) {
            (Some(x), Some(y)) => cmp_numbers(&x, &y) == Some(Ordering::Equal),
            _ => false,
        },
    }
}

/// `a is b or a == b`, which is how a dict finds a key, a set a member and a list an item.
/// The one value it treats differently from `eq` is NaN: `json.loads` hands out a single NaN
/// object for every `NaN` it reads, so two NaN task ids from the log are the same dict key,
/// while `nan == nan` is false.
pub fn same_key(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Float(x), Value::Float(y)) if x.is_nan() && y.is_nan() => true,
        _ => eq(a, b),
    }
}

pub fn eq_str(a: &Value, s: &str) -> bool {
    matches!(a, Value::Str(x) if x == s)
}

/// A Python number: an int of any width, as its decimal digits, or a float.
enum Number {
    Int(String),
    Float(f64),
}

fn number(v: &Value) -> Option<Number> {
    match v {
        Value::Bool(b) => Some(Number::Int(u8::from(*b).to_string())),
        Value::Int(n) => Some(Number::Int(n.to_string())),
        Value::BigInt(digits) => Some(Number::Int(digits.clone())),
        Value::Float(f) => Some(Number::Float(*f)),
        _ => None,
    }
}

/// Order two integers written in decimal, each with a leading `-` when negative and no
/// leading zeros.
fn cmp_decimal(a: &str, b: &str) -> Ordering {
    let magnitude = |x: &str, y: &str| x.len().cmp(&y.len()).then_with(|| x.cmp(y));
    match (a.strip_prefix('-'), b.strip_prefix('-')) {
        (None, None) => magnitude(a, b),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (Some(x), Some(y)) => magnitude(y, x),
    }
}

/// An int against a float, exactly, as Python compares them: the float is not rounded to
/// the int's width nor the int to the float's. `None` for NaN, which orders with nothing.
fn cmp_int_float(n: &str, f: f64) -> Option<Ordering> {
    if f.is_nan() {
        return None;
    }
    if f.is_infinite() {
        return Some(if f > 0.0 {
            Ordering::Less
        } else {
            Ordering::Greater
        });
    }
    // `{:.0}` prints every digit of a float that is an integer, however wide.
    let floor = f.floor();
    let mut digits = format!("{floor:.0}");
    if digits == "-0" {
        digits = "0".into();
    }
    Some(cmp_decimal(n, &digits).then(if f > floor {
        Ordering::Less
    } else {
        Ordering::Equal
    }))
}

fn cmp_numbers(a: &Number, b: &Number) -> Option<Ordering> {
    match (a, b) {
        (Number::Int(x), Number::Int(y)) => Some(cmp_decimal(x, y)),
        (Number::Float(x), Number::Float(y)) => x.partial_cmp(y),
        (Number::Int(n), Number::Float(f)) => cmp_int_float(n, *f),
        (Number::Float(f), Number::Int(n)) => cmp_int_float(n, *f).map(Ordering::reverse),
    }
}

/// `bool(v)`.
pub fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Int(n) => *n != 0,
        Value::BigInt(_) => true,
        Value::Float(f) => *f != 0.0,
        Value::Str(s) => !s.is_empty(),
        Value::List(items) => !items.is_empty(),
        Value::Dict(dict) => !dict.is_empty(),
    }
}

/// Refuse what Python cannot hash, where it would use the value as a dict key or look it up.
pub fn hashable(v: &Value) -> Result<()> {
    match v {
        Value::List(_) => bail!("TypeError: unhashable type: 'list'"),
        Value::Dict(_) => bail!("TypeError: unhashable type: 'dict'"),
        _ => Ok(()),
    }
}

/// `type(v).__name__`.
pub fn type_name(v: &Value) -> &'static str {
    match v {
        Value::Null => "NoneType",
        Value::Bool(_) => "bool",
        Value::Int(_) | Value::BigInt(_) => "int",
        Value::Float(_) => "float",
        Value::Str(_) => "str",
        Value::List(_) => "list",
        Value::Dict(_) => "dict",
    }
}

/// `str(v)`, which is also what an f-string without a format spec prints.
pub fn str(v: &Value) -> PyStr {
    match v {
        Value::Str(s) => s.clone(),
        other => PyStr::from(repr(other)),
    }
}

/// `repr(v)`. Always printable text: `repr` escapes what is not, a lone surrogate included.
pub fn repr(v: &Value) -> String {
    match v {
        Value::Null => "None".into(),
        Value::Bool(true) => "True".into(),
        Value::Bool(false) => "False".into(),
        Value::Int(n) => n.to_string(),
        Value::BigInt(digits) => digits.clone(),
        Value::Float(f) => float_repr(*f),
        Value::Str(s) => repr_str(s),
        Value::List(items) => {
            let parts: Vec<String> = items.iter().map(repr).collect();
            format!("[{}]", parts.join(", "))
        }
        Value::Dict(dict) => {
            let parts: Vec<String> = dict
                .iter()
                .map(|(k, v)| format!("{}: {}", repr_str(k), repr(v)))
                .collect();
            format!("{{{}}}", parts.join(", "))
        }
    }
}

/// `repr(str)`: the quote the text does not hold, and an escape for every code point
/// `str.isprintable` rejects.
fn repr_str(s: &PyStr) -> String {
    let codes = s.code_points();
    let quote = if codes.contains(&0x27) && !codes.contains(&0x22) {
        '"'
    } else {
        '\''
    };
    let mut out = String::new();
    out.push(quote);
    for &code in codes {
        match char::from_u32(code) {
            Some('\\') => out.push_str("\\\\"),
            Some('\n') => out.push_str("\\n"),
            Some('\r') => out.push_str("\\r"),
            Some('\t') => out.push_str("\\t"),
            Some(c) if c == quote => {
                out.push('\\');
                out.push(c);
            }
            Some(c) if is_printable(code) => out.push(c),
            _ if code < 0x100 => out.push_str(&format!("\\x{code:02x}")),
            _ if code < 0x10000 => out.push_str(&format!("\\u{code:04x}")),
            _ => out.push_str(&format!("\\U{code:08x}")),
        }
    }
    out.push(quote);
    out
}

/// `format(v, "<width")`: left-aligned, padded with spaces to `width` characters.
pub fn ljust(v: &Value, width: usize) -> Result<PyStr> {
    let text = match v {
        Value::Str(s) => s.clone(),
        // bool formats through int once a spec is given.
        Value::Bool(b) => PyStr::from(i32::from(*b).to_string()),
        Value::Int(_) | Value::BigInt(_) | Value::Float(_) => PyStr::from(repr(v)),
        other => bail!(
            "TypeError: unsupported format string passed to {}.__format__",
            type_name(other)
        ),
    };
    Ok(pad(&text, width))
}

/// `format(text, "<width")` on a string: Python counts code points.
pub fn pad(text: &PyStr, width: usize) -> PyStr {
    let mut out = text.clone();
    for _ in text.len()..width {
        out.push_code(0x20);
    }
    out
}

/// `len(v)`.
pub fn len(v: &Value) -> Result<usize> {
    match v {
        Value::Str(s) => Ok(s.len()),
        Value::List(items) => Ok(items.len()),
        Value::Dict(dict) => Ok(dict.len()),
        other => bail!(
            "TypeError: object of type '{}' has no len()",
            type_name(other)
        ),
    }
}

/// `v[:n]`.
pub fn head(v: &Value, n: usize) -> Result<Value> {
    match v {
        Value::Str(s) => Ok(Value::Str(s.head(n))),
        Value::List(items) => Ok(Value::List(items.iter().take(n).cloned().collect())),
        // Slicing a dict raises KeyError in Python 3.12 and later, TypeError before.
        Value::Dict(_) => bail!("KeyError: slice(None, {n}, None)"),
        other => bail!(
            "TypeError: '{}' object is not subscriptable",
            type_name(other)
        ),
    }
}

/// `list(v)` over what a log field might hold.
pub fn list(v: &Value) -> Result<Vec<Value>> {
    match v {
        Value::List(items) => Ok(items.clone()),
        Value::Str(s) => Ok(s
            .code_points()
            .iter()
            .map(|code| {
                let mut one = PyStr::new();
                one.push_code(*code);
                Value::Str(one)
            })
            .collect()),
        Value::Dict(dict) => Ok(dict.iter().map(|(k, _)| Value::Str(k.clone())).collect()),
        other => bail!("TypeError: '{}' object is not iterable", type_name(other)),
    }
}

/// `str.isspace` for one character: Rust's whitespace plus the four information separators.
pub fn is_space(c: char) -> bool {
    c.is_whitespace() || ('\u{1c}'..='\u{1f}').contains(&c)
}

/// `str.strip()` on Rust text.
pub fn strip_str(s: &str) -> &str {
    s.trim_matches(is_space)
}

/// `str.strip()`.
pub fn strip(s: &PyStr) -> PyStr {
    let space = |code: &u32| char::from_u32(*code).is_some_and(is_space);
    let codes = s.code_points();
    let start = codes.iter().position(|c| !space(c)).unwrap_or(codes.len());
    let end = codes
        .iter()
        .rposition(|c| !space(c))
        .map_or(start, |i| i + 1);
    let mut out = PyStr::new();
    for code in &codes[start..end] {
        out.push_code(*code);
    }
    out
}

/// The first code point of each run of ten decimal digits, `0` to `9` in order: every
/// character `str.isdecimal` accepts in Unicode 16.0, which is what Python 3.14 carries.
/// `the_int_parser_matches_python_on_every_code_point` in `tests/differential.rs` checks the
/// table against the `python3` on the machine.
const DECIMAL_ZEROS: [u32; 76] = [
    0x0030, 0x0660, 0x06F0, 0x07C0, 0x0966, 0x09E6, 0x0A66, 0x0AE6, 0x0B66, 0x0BE6, 0x0C66, 0x0CE6,
    0x0D66, 0x0DE6, 0x0E50, 0x0ED0, 0x0F20, 0x1040, 0x1090, 0x17E0, 0x1810, 0x1946, 0x19D0, 0x1A80,
    0x1A90, 0x1B50, 0x1BB0, 0x1C40, 0x1C50, 0xA620, 0xA8D0, 0xA900, 0xA9D0, 0xA9F0, 0xAA50, 0xABF0,
    0xFF10, 0x104A0, 0x10D30, 0x10D40, 0x11066, 0x110F0, 0x11136, 0x111D0, 0x112F0, 0x11450,
    0x114D0, 0x11650, 0x116C0, 0x116D0, 0x116DA, 0x11730, 0x118E0, 0x11950, 0x11BF0, 0x11C50,
    0x11D50, 0x11DA0, 0x11F50, 0x16130, 0x16A60, 0x16AC0, 0x16B50, 0x16D70, 0x1CCF0, 0x1D7CE,
    0x1D7D8, 0x1D7E2, 0x1D7EC, 0x1D7F6, 0x1E140, 0x1E2F0, 0x1E4F0, 0x1E5F1, 0x1E950, 0x1FBF0,
];

/// The ASCII digit a decimal digit of any script stands for, as `int()` reads it.
fn decimal_digit(c: char) -> Option<char> {
    let code = u32::from(c);
    let at = DECIMAL_ZEROS
        .partition_point(|zero| *zero <= code)
        .checked_sub(1)?;
    char::from_digit(code - DECIMAL_ZEROS[at], 10)
}

/// The whitespace `int()` strips from a string: `str.isspace` without the four information
/// separators U+001C to U+001F. `int('\x1c7')` raises where `'\x1c7'.strip()` is `'7'`.
fn is_int_space(c: char) -> bool {
    c.is_whitespace()
}

/// `sys.get_int_max_str_digits()`, which nothing in `puppets.py` changes. `int()` counts
/// leading zeros towards it, and not underscores, the sign or the whitespace.
const INT_MAX_STR_DIGITS: usize = 4300;

/// The integer `int(text)` reads, as decimal digits with a `-` when negative: whitespace
/// around the number, a sign, and single underscores between decimal digits of any script.
/// The digits come back at whatever width `int()` takes. `None` where Python raises
/// `ValueError`.
pub fn int_literal(text: &str) -> Option<String> {
    let body = text.trim_matches(is_int_space);
    let (negative, number) = match body.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, body.strip_prefix('+').unwrap_or(body)),
    };
    let mut digits = String::new();
    for run in number.split('_') {
        if run.is_empty() {
            return None;
        }
        for c in run.chars() {
            digits.push(decimal_digit(c)?);
        }
    }
    if digits.len() > INT_MAX_STR_DIGITS {
        return None;
    }
    let digits = match digits.trim_start_matches('0') {
        "" => "0",
        digits => digits,
    };
    Some(format!("{}{digits}", if negative { "-" } else { "" }))
}

/// `int(text)` as a value: an `Int` where it fits `i128`, a `BigInt` where it does not.
pub fn int_value(text: &str) -> Option<Value> {
    let digits = int_literal(text)?;
    Some(match digits.parse() {
        Ok(n) => Value::Int(n),
        Err(_) => Value::BigInt(digits),
    })
}

/// `d[key]` on a value that must be a dict holding `key`.
pub fn item<'a>(v: &'a Value, key: &str) -> Result<&'a Value> {
    match v {
        Value::Dict(dict) => match dict.get(key) {
            Some(found) => Ok(found),
            None => bail!("KeyError: '{key}'"),
        },
        other => bail!(
            "TypeError: '{}' object is not subscriptable by '{key}'",
            type_name(other)
        ),
    }
}

/// `d.get(key, default)` on a value that must be a dict.
pub fn get(v: &Value, key: &str, default: Value) -> Result<Value> {
    match v {
        Value::Dict(dict) => Ok(dict.get(key).cloned().unwrap_or(default)),
        other => bail!(
            "AttributeError: '{}' object has no attribute 'get'",
            type_name(other)
        ),
    }
}

/// `a < b` where the sort needs it: strings by code point, numbers by value.
pub fn less(a: &Value, b: &Value) -> Result<bool> {
    match (a, b) {
        (Value::Str(x), Value::Str(y)) => Ok(x < y),
        _ => match (number(a), number(b)) {
            (Some(x), Some(y)) => Ok(cmp_numbers(&x, &y) == Some(Ordering::Less)),
            _ => bail!(
                "TypeError: '<' not supported between instances of '{}' and '{}'",
                type_name(a),
                type_name(b)
            ),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::json::loads_str;

    fn v(text: &str) -> Value {
        loads_str(text).unwrap()
    }

    /// Each row is `(a, b, a == b, a < b, b < a)` as Python 3.14 answers it.
    #[test]
    fn ints_and_floats_compare_exactly_at_any_width() {
        let two_140 = "1393796574908163946345982392040522594123776";
        let min_i128 = "-170141183460469231731687303715884105728";
        let rows = [
            (two_140, "1.393796574908164e42", true, false, false),
            (
                "1393796574908163946345982392040522594123777",
                "1.393796574908164e42",
                false,
                false,
                true,
            ),
            (
                "1393796574908163946345982392040522594123775",
                "1.393796574908164e42",
                false,
                true,
                false,
            ),
            (min_i128, "-1.7014118346046923e38", true, false, false),
            (
                "-170141183460469231731687303715884105729",
                "-1.7014118346046923e38",
                false,
                true,
                false,
            ),
            (
                "680564733841876926926749214863536422912",
                "6.80564733841877e38",
                true,
                false,
                false,
            ),
            (
                "2722258935367507707706996859454145691648",
                "1361129467683753853853498429727072845824",
                false,
                false,
                true,
            ),
            (
                "-2722258935367507707706996859454145691648",
                "-1361129467683753853853498429727072845824",
                false,
                true,
                false,
            ),
            (
                "-2722258935367507707706996859454145691648",
                "7",
                false,
                true,
                false,
            ),
            (two_140, "Infinity", false, true, false),
            (two_140, "-Infinity", false, false, true),
            (two_140, "NaN", false, false, false),
            ("NaN", "NaN", false, false, false),
            ("3", "3.5", false, true, false),
            ("4", "3.5", false, false, true),
            ("-3", "-3.5", false, false, true),
            ("-4", "-3.5", false, true, false),
            ("0", "-0.0", true, false, false),
            ("true", "1.0", true, false, false),
            ("false", "0.5", false, true, false),
            ("9007199254740993", "9007199254740992.0", false, false, true),
            ("1e400", two_140, false, false, true),
        ];
        for (a, b, equal, a_less, b_less) in rows {
            let (x, y) = (v(a), v(b));
            assert_eq!(eq(&x, &y), equal, "{a} == {b}");
            assert_eq!(eq(&y, &x), equal, "{b} == {a}");
            assert_eq!(less(&x, &y).unwrap(), a_less, "{a} < {b}");
            assert_eq!(less(&y, &x).unwrap(), b_less, "{b} < {a}");
        }
    }

    /// Each expectation is `repr(json.loads(text))` under Python 3.14.
    #[test]
    fn repr_escapes_what_isprintable_rejects() {
        for (text, want) in [
            (r#""\u2028""#, r#"'\u2028'"#),
            (r#""\u200b""#, r#"'\u200b'"#),
            (r#""\u00ad""#, r#"'\xad'"#),
            (r#""\u00a0""#, r#"'\xa0'"#),
            (r#""\u3000""#, r#"'\u3000'"#),
            (r#""\ud800""#, r#"'\ud800'"#),
            (r#""\ue000""#, r#"'\ue000'"#),
            (r#""\u0378""#, r#"'\u0378'"#),
            (r#""\udb40\udc01""#, r#"'\U000e0001'"#),
            (r#""\u007f\u0000\t\n\r\\""#, r"'\x7f\x00\t\n\r\\'"),
            (r#""a'b""#, r#""a'b""#),
            (r#""a'b\"c""#, r#"'a\'b"c'"#),
            (r#""é ☃ 😀""#, "'é ☃ 😀'"),
            (
                r#"{"\u2028":["\ud800",1.0,null,true]}"#,
                r#"{'\u2028': ['\ud800', 1.0, None, True]}"#,
            ),
        ] {
            assert_eq!(repr(&v(text)), want, "{text}");
        }
    }

    /// As Python answers `a == b` and `a in [b]` on values from one `json.loads`.
    #[test]
    fn nan_is_a_key_that_finds_itself_and_a_value_equal_to_nothing() {
        let nan = v("NaN");
        assert!(!eq(&nan, &nan));
        assert!(same_key(&nan, &nan));
        assert!(eq(&v("[NaN]"), &v("[NaN]")));
        assert!(eq(&v(r#"{"a":NaN}"#), &v(r#"{"a":NaN}"#)));
        assert!(!same_key(&nan, &v("1.0")));
        assert!(same_key(&v("1"), &v("1.0")));
    }

    #[test]
    fn strings_pad_and_strip_by_code_point() {
        let Value::Str(lone) = v(r#""\ud800\u00e9""#) else {
            panic!("not a string");
        };
        assert_eq!(pad(&lone, 4).len(), 4);
        assert_eq!(pad(&lone, 1), lone);
        assert_eq!(strip(&PyStr::from(" \u{1f}a b\u{3000}\n")), "a b");
        assert_eq!(strip(&PyStr::from(" \t")), "");
    }
}
