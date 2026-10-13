You are proposing a work plan for the following goal.

## Goal

# s1-readme-typo

Size: small

The README of this `tally` crate says the crate "tallys" lines and words. That is a spelling
mistake. Change it to "tallies". Touch no Rust. Done means the README uses the right verb and
`cargo test` is still green.

## Repository

The goal is against this repository, `tally`. Its tree:

```
Cargo.toml
README.md
src/lib.rs
```

### Cargo.toml

```
[package]
name = "tally"
version = "0.1.0"
edition = "2024"
publish = false

[lib]
path = "src/lib.rs"
```

### README.md

```
# tally

A tiny crate that tallys lines and words in a string. Used as the scratch repository for
Hydra's orchestrator-cost goals under `integrations/bench/goals/`. Copy this directory; do
not run the goals against Hydra's own checkout.
```

### src/lib.rs

```
//! Count words and lines in a string.

/// Number of whitespace-separated words.
pub fn words(text: &str) -> usize {
    text.split_whitespace().count()
}

/// Number of lines. An empty string is zero lines.
pub fn lines(text: &str) -> usize {
    if text.is_empty() {
        0
    } else {
        text.lines().count()
    }
}

/// Add one, wrapping at 255.
pub fn bump(n: u8) -> u8 {
    n.wrapping_add(1)
}

#[cfg(test)]
mod tests {
    use super::{bump, lines, words};

    #[test]
    fn words_counts_split_tokens() {
        assert_eq!(words(""), 0);
        assert_eq!(words("  "), 0);
        assert_eq!(words("one"), 1);
    }

    #[test]
    fn lines_treats_empty_as_zero() {
        assert_eq!(lines(""), 0);
        assert_eq!(lines("a\nb"), 2);
    }

    #[test]
    fn bump_adds_one() {
        assert_eq!(bump(0), 1);
        assert_eq!(bump(1), 2);
    }
}
```


## Instructions

Break the goal into concrete tasks a coding agent can run in isolation. Each task should name
the files or areas it touches when that is knowable, and say what "done" looks like: a command
and its expected result, a file that exists, a test that passes, something a reviewer can check
without re-doing the work.

Respond with a short narrative, then end with exactly one fenced JSON block containing an array
of task objects. Use this shape for every task:

```json
[
  {
    "id": "short-id",
    "text": "what the agent should do",
    "class": "batch_scripting",
    "grade": "low",
    "done": "the check that says this task is finished",
    "deps": [],
    "paths": ["src/example.rs"]
  }
]
```

`class` must be one of: `batch_scripting`, `rust_off_launch_path`, `rust_launch_path`,
`security_review`, `research`, `docs`, `probe`.

`grade` must be one of: `low`, `medium`, `high`, `agi`.

`deps` lists task ids that must finish before this one. `paths` lists files or directories the
task is expected to touch. `done` is optional but should be filled whenever the task has a
check a reviewer can run.

Do not include any other fenced JSON blocks.
