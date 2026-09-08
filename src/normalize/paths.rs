//! Every path a run's tool calls read or wrote, contract 16.2.
//!
//! Export eligibility keys on this adapter more than anything else does. A tool
//! shape it cannot classify is a path nobody sees, so [`PathExtraction::Unclassified`]
//! is a refusal rather than an empty answer.

use std::path::{Path, PathBuf};

use crate::normalize::ToolCall;
use crate::policy::{Capability, table};
use crate::provider::Provider;

/// What the adapter could read off a run's calls.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathExtraction {
    Paths(Vec<PathBuf>),
    /// A call the adapter cannot resolve, contract 16.2's first rule.
    Unclassified,
}

/// Every path the run's tool calls touched, or [`PathExtraction::Unclassified`].
pub fn paths_touched(run: &crate::normalize::Run) -> PathExtraction {
    let provider = run.provider.as_deref().unwrap_or("");
    let cwd = cwd_of(run);
    let mut paths = Vec::new();
    for call in run.tool_calls() {
        match paths_from_call(provider, call, &cwd) {
            PathExtraction::Paths(found) => paths.extend(found),
            PathExtraction::Unclassified => return PathExtraction::Unclassified,
        }
    }
    PathExtraction::Paths(paths)
}

/// The directory tool arguments resolve against, read off `run_start`.
pub fn cwd_of(run: &crate::normalize::Run) -> PathBuf {
    run.run_start()
        .and_then(|start| start.body["cwd"].as_str())
        .map(PathBuf::from)
        .or_else(|| {
            run.run_start()
                .and_then(|start| start.body["worktree"]["path"].as_str())
                .map(PathBuf::from)
        })
        .unwrap_or_else(|| PathBuf::from("/tmp"))
}

/// One call's paths, or [`PathExtraction::Unclassified`].
pub fn paths_from_call(provider: &str, call: &ToolCall, cwd: &Path) -> PathExtraction {
    if provider == "cursor-agent" && call.name.ends_with("ToolCall") {
        return PathExtraction::Unclassified;
    }

    let table = match provider {
        "claude" => table::Table::of(Provider::Claude),
        "cursor-agent" => table::Table::of(Provider::CursorAgent),
        "agy" => table::Table::of(Provider::Agy),
        _ => return PathExtraction::Unclassified,
    };

    if table
        .mcp_prefix
        .is_some_and(|prefix| call.name.starts_with(prefix))
    {
        return PathExtraction::Paths(vec![]);
    }

    let entry = table.tools.iter().find(|tool| tool.name == call.name);
    if let Some(entry) = entry {
        if let Some(field) = entry.path_field {
            let raw = call.arguments.get(field).and_then(|value| value.as_str());
            return PathExtraction::Paths(match raw {
                Some(path) if !path.is_empty() => vec![resolve(path, cwd)],
                _ => vec![],
            });
        }
        if entry.capability == Capability::Shell {
            let command = call
                .arguments
                .get(table.command_field)
                .and_then(|value| value.as_str());
            return PathExtraction::Paths(match command {
                Some(command) => paths_from_shell(command, cwd),
                None => vec![],
            });
        }
        return PathExtraction::Paths(vec![]);
    }

    PathExtraction::Paths(vec![])
}

/// The gate's `resolve`, without importing Python: join to `cwd`, then canonicalize
/// when the path exists.
fn resolve(raw: &str, cwd: &Path) -> PathBuf {
    let path = PathBuf::from(raw);
    let joined = if path.is_absolute() {
        path
    } else {
        cwd.join(path)
    };
    joined.canonicalize().unwrap_or(joined)
}

/// Paths a shell command names, coarse in the same direction as the gate's floor.
fn paths_from_shell(command: &str, cwd: &Path) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    for segment in split_shell_segments(command) {
        for token in segment {
            for candidate in token_path_candidates(&token) {
                paths.push(resolve(&candidate, cwd));
            }
        }
    }
    paths
}

fn split_shell_segments(command: &str) -> Vec<Vec<String>> {
    let mut segments: Vec<Vec<String>> = Vec::new();
    let mut current: Vec<String> = Vec::new();
    let mut word = String::new();
    let mut in_single = false;
    let mut in_double = false;
    let mut i = 0;
    let bytes = command.as_bytes();
    while i < bytes.len() {
        let ch = bytes[i] as char;
        if in_single {
            if ch == '\'' {
                in_single = false;
            } else {
                word.push(ch);
            }
            i += 1;
            continue;
        }
        if in_double {
            if ch == '"' {
                in_double = false;
            } else {
                word.push(ch);
            }
            i += 1;
            continue;
        }
        if ch == '\'' {
            in_single = true;
            i += 1;
            continue;
        }
        if ch == '"' {
            in_double = true;
            i += 1;
            continue;
        }
        if ch.is_whitespace() {
            if !word.is_empty() {
                current.push(word.clone());
                word.clear();
            }
            i += 1;
            continue;
        }
        if matches!(ch, ';' | '|' | '&' | '\n') {
            let doubled =
                matches!(ch, '|' | '&') && i + 1 < bytes.len() && bytes[i + 1] as char == ch;
            i += if doubled { 2 } else { 1 };
            if !word.is_empty() {
                current.push(word.clone());
                word.clear();
            }
            if !current.is_empty() {
                segments.push(current);
                current = Vec::new();
            }
            continue;
        }
        word.push(ch);
        i += 1;
    }
    if !word.is_empty() {
        current.push(word);
    }
    if !current.is_empty() {
        segments.push(current);
    }
    segments
}

fn token_path_candidates(token: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for part in [Some(token), token.split_once('=').map(|(_, tail)| tail)] {
        let Some(part) = part else { continue };
        if part.is_empty() || !seen.insert(part) {
            continue;
        }
        out.push(expand_token(part));
    }
    out
}

fn expand_token(token: &str) -> String {
    if let Some(rest) = token.strip_prefix("~/")
        && let Some(home) = dirs::home_dir()
    {
        return home.join(rest).to_string_lossy().to_string();
    }
    if token == "~"
        && let Some(home) = dirs::home_dir()
    {
        return home.to_string_lossy().to_string();
    }
    token.to_string()
}

/// Case-folded containment, harness spec 15.2.
pub fn inside(path: &Path, root: &Path) -> bool {
    let path = casefold_string(path);
    let root = casefold_string(root);
    if path == root {
        return true;
    }
    let mut prefix = root;
    if !prefix.ends_with('/') {
        prefix.push('/');
    }
    path.starts_with(&prefix)
}

fn casefold_string(path: &Path) -> String {
    path.to_string_lossy()
        .chars()
        .flat_map(|c| c.to_lowercase())
        .collect()
}

mod dirs {
    use std::path::PathBuf;

    pub fn home_dir() -> Option<PathBuf> {
        crate::env::real_home().ok()
    }
}
