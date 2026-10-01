//! The few Python operations the fold and the read commands apply to log values.
//!
//! A log line is whatever JSON a writer put there, so a field `puppets.py` expects to be a
//! string can hold anything. These follow what Python does with such a value where that is
//! cheap to say, and return an error where Python raises, so a command stops at the same
//! point with the same stdout before it.

use anyhow::{Result, bail};

use crate::json::{Value, float_repr};

/// `a == b`: `True == 1 == 1.0`, dicts equal regardless of key order.
pub fn eq(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::Str(x), Value::Str(y)) => x == y,
        (Value::List(x), Value::List(y)) => {
            x.len() == y.len() && x.iter().zip(y).all(|(p, q)| eq(p, q))
        }
        (Value::Dict(x), Value::Dict(y)) => {
            x.len() == y.len() && x.iter().all(|(k, v)| y.get(k).is_some_and(|w| eq(v, w)))
        }
        (Value::BigInt(x), Value::BigInt(y)) => x == y,
        _ => match (number(a), number(b)) {
            (Some(Number::Int(x)), Some(Number::Int(y))) => x == y,
            (Some(Number::Float(x)), Some(Number::Float(y))) => x == y,
            (Some(Number::Int(n)), Some(Number::Float(f)))
            | (Some(Number::Float(f)), Some(Number::Int(n))) => int_eq_float(n, f),
            _ => false,
        },
    }
}

pub fn eq_str(a: &Value, s: &str) -> bool {
    matches!(a, Value::Str(x) if x == s)
}

enum Number {
    Int(i128),
    Float(f64),
}

fn number(v: &Value) -> Option<Number> {
    match v {
        Value::Bool(b) => Some(Number::Int(i128::from(*b))),
        Value::Int(n) => Some(Number::Int(*n)),
        Value::Float(f) => Some(Number::Float(*f)),
        _ => None,
    }
}

fn int_eq_float(n: i128, f: f64) -> bool {
    // Exact, as Python compares: a float equals an int only when it is that integer.
    f.is_finite() && f.fract() == 0.0 && f.abs() < 1.7e38 && f as i128 == n
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

fn type_name(v: &Value) -> &'static str {
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
pub fn str(v: &Value) -> String {
    match v {
        Value::Str(s) => s.clone(),
        other => repr(other),
    }
}

/// `repr(v)`.
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

/// `repr(str)`. Printable is approximated as "not a C0 or C1 control, DEL or NBSP", which
/// covers what a goal log holds; Python's own table is the Unicode database.
fn repr_str(s: &str) -> String {
    let quote = if s.contains('\'') && !s.contains('"') {
        '"'
    } else {
        '\''
    };
    let mut out = String::new();
    out.push(quote);
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c == quote => {
                out.push('\\');
                out.push(c);
            }
            c if (c as u32) < 0x20 || (0x7f..=0xa0).contains(&(c as u32)) => {
                out.push_str(&format!("\\x{:02x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out.push(quote);
    out
}

/// `format(v, "<width")`: left-aligned, padded with spaces to `width` characters.
pub fn ljust(v: &Value, width: usize) -> Result<String> {
    let text = match v {
        Value::Str(s) => s.clone(),
        // bool formats through int once a spec is given.
        Value::Bool(b) => i32::from(*b).to_string(),
        Value::Int(_) | Value::BigInt(_) | Value::Float(_) => repr(v),
        other => bail!(
            "TypeError: unsupported format string passed to {}.__format__",
            type_name(other)
        ),
    };
    Ok(pad(&text, width))
}

pub fn pad(text: &str, width: usize) -> String {
    format!("{text:<width$}")
}

/// `len(v)`.
pub fn len(v: &Value) -> Result<usize> {
    match v {
        Value::Str(s) => Ok(s.chars().count()),
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
        Value::Str(s) => Ok(Value::Str(s.chars().take(n).collect())),
        Value::List(items) => Ok(Value::List(items.iter().take(n).cloned().collect())),
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
        Value::Str(s) => Ok(s.chars().map(|c| Value::Str(c.to_string())).collect()),
        Value::Dict(dict) => Ok(dict.iter().map(|(k, _)| Value::Str(k.clone())).collect()),
        other => bail!("TypeError: '{}' object is not iterable", type_name(other)),
    }
}

/// `str.isspace` for one character: Rust's whitespace plus the four information separators.
pub fn is_space(c: char) -> bool {
    c.is_whitespace() || ('\u{1c}'..='\u{1f}').contains(&c)
}

/// `str.strip()`.
pub fn strip(s: &str) -> &str {
    s.trim_matches(is_space)
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
            (Some(Number::Int(x)), Some(Number::Int(y))) => Ok(x < y),
            (Some(Number::Float(x)), Some(Number::Float(y))) => Ok(x < y),
            (Some(Number::Int(x)), Some(Number::Float(y))) => Ok((x as f64) < y),
            (Some(Number::Float(x)), Some(Number::Int(y))) => Ok(x < y as f64),
            _ => bail!(
                "TypeError: '<' not supported between instances of '{}' and '{}'",
                type_name(a),
                type_name(b)
            ),
        },
    }
}
