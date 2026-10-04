//! `redact.py`: the secret classes `hydra export` redacts, applied to what the grader keeps.
//!
//! Known key prefixes, PEM blocks and the OpenAI key span rule. Every index is a code point
//! of a Python `str`, never a UTF-8 byte: the Python's first version mixed the two and
//! mis-scanned any text with an accent before a key. A secret found by its prefix runs to
//! the first quote or the first character `str.isspace` accepts, and that set is wider than
//! Rust's by U+001C to U+001F, so it is `hydra_pyjson::py::is_space` that ends one here.

use hydra_pyjson::json::{Dict, PyStr, Value};
use hydra_pyjson::py;

const OPENAI_PROJECT_BODY: usize = 20;
const OPENAI_LEGACY_BODY: usize = 32;

/// The prefixes, in the order the Python replaces them, and the label each leaves.
const PREFIXES: [(&str, &str); 12] = [
    ("AIza", "google_api_key"),
    ("sk-ant-", "anthropic_api_key"),
    ("ghp_", "github_pat"),
    ("github_pat_", "github_fine_grained_pat"),
    ("gho_", "github_oauth_token"),
    ("ya29.", "google_oauth_token"),
    ("AKIA", "aws_access_key"),
    ("aws_secret_access_key=", "aws_secret_access_key"),
    ("AWS_SECRET_ACCESS_KEY=", "aws_secret_access_key"),
    ("xoxb-", "slack_bot_token"),
    ("hooks.slack.com", "slack_webhook"),
    ("-----BEGIN", "pem_private_key"),
];

const SLACK: &str = "hooks.slack.com";
const PEM: &str = "-----BEGIN";

fn codes(text: &str) -> Vec<u32> {
    text.chars().map(u32::from).collect()
}

fn pystr(codes: &[u32]) -> PyStr {
    let mut out = PyStr::new();
    for code in codes {
        out.push_code(*code);
    }
    out
}

/// `hay.find(needle, from)`.
fn find(hay: &[u32], needle: &[u32], from: usize) -> Option<usize> {
    if from > hay.len() {
        return None;
    }
    hay[from..]
        .windows(needle.len())
        .position(|window| window == needle)
        .map(|at| from + at)
}

fn ascii(code: u32) -> Option<char> {
    char::from_u32(code).filter(char::is_ascii)
}

/// `c.isascii() and c.isalnum()`.
fn alnum(code: u32) -> bool {
    ascii(code).is_some_and(|c| c.is_ascii_alphanumeric())
}

/// `_token_char`: what an OpenAI key and the word around one are made of.
fn token_char(code: u32) -> bool {
    alnum(code) || code == u32::from('_') || code == u32::from('-')
}

fn is_space(code: u32) -> bool {
    char::from_u32(code).is_some_and(py::is_space)
}

fn key_spans(text: &[u32]) -> Vec<(usize, usize)> {
    let needle = codes("sk-");
    let kinds = ["proj-", "svcacct-", "admin-"].map(codes);
    let mut spans = Vec::new();
    let mut from = 0;
    while let Some(at) = find(text, &needle, from) {
        from = at + 3;
        if at > 0 && token_char(text[at - 1]) {
            continue;
        }
        let rest = &text[at + 3..];
        let kind = kinds.iter().find(|kind| rest.starts_with(kind));
        let (body, allowed, least): (usize, fn(u32) -> bool, usize) = match kind {
            Some(kind) => (at + 3 + kind.len(), token_char, OPENAI_PROJECT_BODY),
            None => (at + 3, alnum, OPENAI_LEGACY_BODY),
        };
        let mut pos = body;
        while pos < text.len() && allowed(text[pos]) {
            pos += 1;
        }
        if pos - body >= least && (pos == text.len() || !token_char(text[pos])) {
            spans.push((at, pos));
            from = pos;
        }
    }
    spans
}

/// `openai_key_spans`: where the OpenAI keys of `text` start and end, left to right, in
/// code points.
pub fn openai_key_spans(text: &PyStr) -> Vec<(usize, usize)> {
    key_spans(text.code_points())
}

fn label(name: &str) -> Vec<u32> {
    codes(&format!("[REDACTED:{name}]"))
}

/// `redact_string`: `text` with every secret replaced by `[REDACTED:<class>]`.
pub fn redact_string(text: &PyStr) -> PyStr {
    let mut out = text.code_points().to_vec();
    let (footer, dashes) = (codes("-----END"), codes("-----"));
    let (secure, plain) = (codes("https://"), codes("http://"));
    for (prefix, name) in PREFIXES {
        let needle = codes(prefix);
        while let Some(mut at) = find(&out, &needle, 0) {
            if prefix == SLACK {
                if out[..at].ends_with(&secure) {
                    at -= secure.len();
                } else if out[..at].ends_with(&plain) {
                    at -= plain.len();
                }
            }
            let end = if prefix == PEM {
                // To the dashes that close the footer, or to the end of the text.
                find(&out, &footer, at)
                    .and_then(|footer_at| find(&out, &dashes, footer_at + footer.len()))
                    .map_or(out.len(), |close| close + dashes.len())
            } else {
                out[at..]
                    .iter()
                    .position(|c| is_space(*c) || *c == u32::from('"') || *c == u32::from('\''))
                    .map_or(out.len(), |i| at + i)
            };
            out.splice(at..end, label(name));
        }
    }
    for (at, end) in key_spans(&out).into_iter().rev() {
        out.splice(at..end, label("openai_api_key"));
    }
    pystr(&out)
}

/// `redact_value`: `redact_string` over every string of a JSON value, keys included. Two
/// keys that redact to the same text become one, at the first one's place with the last
/// one's value, as the Python's dict comprehension leaves them.
pub fn redact_value(value: &Value) -> Value {
    match value {
        Value::Str(s) => Value::Str(redact_string(s)),
        Value::List(items) => Value::List(items.iter().map(redact_value).collect()),
        Value::Dict(dict) => {
            let mut out = Dict::new();
            for (key, item) in dict.iter() {
                out.set_key(redact_string(key), redact_value(item));
            }
            Value::Dict(out)
        }
        other => other.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn redacted(text: &str) -> String {
        redact_string(&PyStr::from(text)).lossy()
    }

    #[test]
    fn a_legacy_key_after_an_accent_is_redacted_whole() {
        assert_eq!(
            redacted("\u{e9} sk-abcdefghijklmnopqrstuvwxyz0123456789ABCD"),
            "\u{e9} [REDACTED:openai_api_key]"
        );
    }

    #[test]
    fn an_information_separator_ends_a_secret() {
        assert_eq!(
            redacted("ghp_planted\u{1c}tail"),
            "[REDACTED:github_pat]\u{1c}tail"
        );
    }

    #[test]
    fn prose_about_a_prefix_is_left_alone() {
        for words in ["task-management-system-v2", "the `sk-proj-` prefix"] {
            assert_eq!(redacted(words), words);
        }
    }
}
