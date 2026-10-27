//! The secret sweep, contract section 13.
//!
//! A prefix sweep runs on the write path. A hit puts the matched class names on the
//! envelope and leaves the frame untouched: this detects and never rewrites, so the
//! frame stays verbatim and a live credential sitting in `runs/` is visible the day
//! it lands rather than at the first export.
//!
//! This is the record's one stored derived value, and it holds as an exception
//! because a rescan reproduces the flag exactly. It is a cache, not a fact.
//!
//! The class table is `hydra_secret_classes`. This sweep reads the rows flagged
//! for the write path.

use hydra_secret_classes::CLASSES;

/// Each class's prefix, the same list the sweep scans for. The cockpit masks
/// by it (build ticket 180).
pub fn prefixes() -> impl Iterator<Item = &'static str> {
    CLASSES
        .iter()
        .filter(|class| class.write_path)
        .map(|class| class.prefix)
}

/// The class names matching anywhere in the line, or `None` for a clean line.
pub fn scan(line: &str) -> Option<Vec<&'static str>> {
    let hits: Vec<&'static str> = CLASSES
        .iter()
        .filter(|class| class.write_path && line.contains(class.prefix))
        .map(|class| class.label)
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

    #[test]
    fn scan_flags_the_same_five_labels() {
        let line = "AIza sk-ant- ghp_ ya29. -----BEGIN";
        assert_eq!(
            scan(line).unwrap(),
            vec![
                "google_api_key",
                "anthropic_api_key",
                "github_pat",
                "google_oauth_token",
                "pem_private_key",
            ]
        );
        assert_eq!(scan("gho_ github_pat_ xoxb-"), None);
    }
}
