//! The secret sweep, contract section 13.
//!
//! A prefix sweep runs on the write path. A hit puts the matched class names on the
//! envelope and leaves the frame untouched: this detects and never rewrites, so the
//! frame stays verbatim and a live credential sitting in `runs/` is visible the day
//! it lands rather than at the first export.
//!
//! This is the record's one stored derived value, and it holds as an exception
//! because a rescan reproduces the flag exactly. It is a cache, not a fact.

const CLASSES: &[(&str, &str)] = &[
    ("google_api_key", "AIza"),
    ("anthropic_api_key", "sk-ant-"),
    ("github_pat", "ghp_"),
    ("google_oauth_token", "ya29."),
    ("pem_private_key", "-----BEGIN"),
];

/// The class names matching anywhere in the line, or `None` for a clean line.
pub fn scan(line: &str) -> Option<Vec<&'static str>> {
    let hits: Vec<&'static str> = CLASSES
        .iter()
        .filter(|(_, prefix)| line.contains(prefix))
        .map(|(name, _)| *name)
        .collect();
    (!hits.is_empty()).then_some(hits)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_clean_frame_flags_nothing() {
        assert_eq!(scan(r#"{"type":"assistant","text":"pong"}"#), None);
    }

    #[test]
    fn the_key_this_shell_exports_is_caught() {
        let flags = scan(r#"{"raw":"GEMINI_API_KEY=AIzaSyExampleNotARealKey"}"#).unwrap();
        assert_eq!(flags, vec!["google_api_key"]);
    }

    #[test]
    fn a_frame_can_carry_more_than_one_class() {
        let flags = scan("sk-ant-abc and ghp_def").unwrap();
        assert_eq!(flags, vec!["anthropic_api_key", "github_pat"]);
    }
}
