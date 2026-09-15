//! The instruction preamble, harness spec 8.
//!
//! **It rides the task string rather than an injected skill.** An always-injected
//! skill would spend 6.4's measured listing cost on every run forever and would
//! carry frontmatter for the `: ` hazard to break. A preamble costs no listing
//! slot, rides the one input that is unambiguously Hydra's, and lands in
//! `run_start.task`, so the record shows exactly what the run was told.
//!
//! **The text is protected config #4** (15.1) and lives at `config/preamble.md`.
//! The gate denies writes to it under every policy, because a run that can edit
//! this is a run told a different rule than the one Hydra enforces.
//!
//! **It asserts three things and no more**, and the file is where that is
//! enforced by a human reading it rather than by code. Nothing here validates the
//! wording; a checker that counted assertions would be theatre.

use std::path::PathBuf;

use anyhow::{Context, Result};

/// Protected config #4's path, the same one [`crate::policy::denied_write_paths`]
/// denies every policy from writing to.
pub fn path() -> PathBuf {
    crate::paths::hydra_home().join("config/preamble.md")
}

/// The preamble text, trailing newline trimmed.
pub fn load() -> Result<String> {
    let file = path();
    let text = std::fs::read_to_string(&file)
        .with_context(|| format!("reading the instruction preamble at {}", file.display()))?;
    let text = text.trim_end().to_string();
    if text.is_empty() {
        anyhow::bail!(
            "{} is empty, and an empty preamble would launch a run told nothing \
             while the record claimed it was told something",
            file.display()
        );
    }
    Ok(text)
}

/// What the provider is passed: the preamble, a blank line, then the caller's task
/// verbatim.
///
/// Contract 5.1 records this whole string as `task`, preamble included, so the
/// record needs no separate field and the preamble needs no sha.
///
/// **No marker line separates the two.** Section 8 allows the preamble three
/// assertions and no more, and a "your task follows" line is a fourth sentence in
/// Hydra's voice. The blank line is the boundary.
pub fn compose(preamble: &str, task: &str) -> String {
    format!("{preamble}\n\n{task}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_shipped_preamble_loads_and_says_the_three_things() {
        let text = load().unwrap();
        // Not a wording check. These are the three assertions section 8 permits,
        // and the test fails if an edit drops one of the words they turn on.
        assert!(text.contains("instructions"), "assertion one");
        assert!(text.contains("data"), "assertion two");
        assert!(text.contains("widen"), "assertion three");
    }

    #[test]
    fn the_shipped_preamble_promises_nothing() {
        let text = load().unwrap().to_lowercase();
        for word in [
            "guarantee",
            "guaranteed",
            "will catch",
            "prevents",
            "ensures",
        ] {
            assert!(
                !text.contains(word),
                "the preamble says {word:?}, and section 8 forbids anything shaped \
                 like a guarantee: a run that trusts the gate is the Hermes failure \
                 with the roles swapped"
            );
        }
    }

    #[test]
    fn the_task_arrives_verbatim_after_the_preamble() {
        let composed = compose("PREAMBLE", "do the thing");
        assert_eq!(composed, "PREAMBLE\n\ndo the thing");
        assert!(composed.ends_with("do the thing"));
    }
}
