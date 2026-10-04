//! `json.dumps(value, sort_keys=True, ensure_ascii=False)`, which is how `apprentice.py`
//! writes a line of a record into a prompt.
//!
//! The text is a `PyStr` and not a Rust `String`: with `ensure_ascii=False` Python writes a
//! lone surrogate as itself, and `write_jsonl` escapes the prompt a second time when it
//! writes the bank, so a `\ud800` in a record reaches `bank.jsonl` as `\ud800` and not as
//! `\\ud800`. `hydra_pyjson::json::dumps_unicode_default` escapes it in the first pass.

use hydra_pyjson::json::{PyStr, Value, float_repr};

/// `json.dumps(value, sort_keys=True, ensure_ascii=False)`: one line, `", "` and `": "`,
/// keys in code point order, `NaN` and `Infinity` written as Python writes them.
pub fn dump_sorted(value: &Value) -> PyStr {
    let mut out = PyStr::new();
    write(&mut out, value);
    out
}

fn write(out: &mut PyStr, value: &Value) {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(true) => out.push_str("true"),
        Value::Bool(false) => out.push_str("false"),
        Value::Int(n) => out.push_str(&n.to_string()),
        Value::BigInt(digits) => out.push_str(digits),
        Value::Float(x) if x.is_nan() => out.push_str("NaN"),
        Value::Float(x) if x.is_infinite() => {
            out.push_str(if *x > 0.0 { "Infinity" } else { "-Infinity" })
        }
        Value::Float(x) => out.push_str(&float_repr(*x)),
        Value::Str(s) => write_str(out, s),
        Value::List(items) => {
            out.push_str("[");
            for (n, item) in items.iter().enumerate() {
                if n > 0 {
                    out.push_str(", ");
                }
                write(out, item);
            }
            out.push_str("]");
        }
        Value::Dict(dict) => {
            let mut items: Vec<_> = dict.iter().collect();
            items.sort_by(|a, b| a.0.cmp(b.0));
            out.push_str("{");
            for (n, (key, item)) in items.into_iter().enumerate() {
                if n > 0 {
                    out.push_str(", ");
                }
                write_str(out, key);
                out.push_str(": ");
                write(out, item);
            }
            out.push_str("}");
        }
    }
}

/// What `py_encode_basestring` escapes: the quote, the backslash and the C0 controls.
/// Everything else, DEL and a surrogate included, is written as it is.
fn write_str(out: &mut PyStr, s: &PyStr) {
    out.push_str("\"");
    for code in s.code_points() {
        match *code {
            0x22 => out.push_str("\\\""),
            0x5c => out.push_str("\\\\"),
            0xa => out.push_str("\\n"),
            0xd => out.push_str("\\r"),
            0x9 => out.push_str("\\t"),
            0x8 => out.push_str("\\b"),
            0xc => out.push_str("\\f"),
            code @ 0..0x20 => out.push_str(&format!("\\u{code:04x}")),
            code => out.push_code(code),
        }
    }
    out.push_str("\"");
}

#[cfg(test)]
mod tests {
    use super::*;
    use hydra_pyjson::json::loads_str;

    /// What CPython 3.14.8 printed for the same value on 2026-10-04.
    #[test]
    fn keys_sort_and_a_surrogate_is_written_raw() {
        let value =
            loads_str(r#"{"b": "\u007f\ud800é\u0001", "a": [NaN, 1e16, -0.0, 7]}"#).unwrap();
        let got = dump_sorted(&value);
        let mut want = PyStr::from(r#"{"a": [NaN, 1e+16, -0.0, 7], "b": ""#);
        for code in [0x7f, 0xd800, 0xe9] {
            want.push_code(code);
        }
        want.push_str(r#"\u0001"}"#);
        assert_eq!(got, want);
    }
}
