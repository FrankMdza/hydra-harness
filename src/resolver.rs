//! Resolving a run-id prefix to the run it names.
//!
//! Every command that takes a run id took the full 36 characters until build
//! ticket 39, because [`crate::paths::Layout::run_file`] builds the record path
//! straight from the string it is handed. A prefix simply missed the file and
//! the command reported the run as missing, so every session that read a record
//! copied a uuid to do it.
//!
//! **A full uuid is never resolved.** It passes through untouched, matching
//! record or not, because `hydra reap <uuid>` on a run whose record was pruned
//! is a real thing to type and predates this module. Only a shorter string is
//! looked up, and a shorter string that matches nothing is a typo rather than an
//! absent run, so it is an error. Those two inputs are different questions and
//! the asymmetry is the answer to both rather than an inconsistency.

use anyhow::{Result, bail};

use crate::paths::Layout;

/// The run id `prefix` names, or an error naming what went wrong.
///
/// A full uuid is returned as given. A shorter string matching exactly one
/// record resolves to that record's id; one matching none or several is an
/// error that says which.
pub fn resolve(layout: &Layout, prefix: &str) -> Result<String> {
    if is_uuid(prefix) {
        return Ok(prefix.to_string());
    }
    if prefix.is_empty() {
        bail!("a run id is required");
    }

    let mut matches = matching(layout, prefix)?;
    matches.sort();

    match matches.len() {
        1 => Ok(matches.remove(0)),
        0 => bail!("no run matches {prefix:?}"),
        n => {
            let list = matches
                .iter()
                .map(|id| format!("  {id}"))
                .collect::<Vec<_>>()
                .join("\n");
            bail!("{prefix:?} matches {n} runs:\n{list}")
        }
    }
}

/// Every recorded run id starting with `prefix`.
fn matching(layout: &Layout, prefix: &str) -> Result<Vec<String>> {
    let dir = match std::fs::read_dir(layout.runs()) {
        Ok(dir) => dir,
        // No corpus is not a read failure. Nothing matches, and the caller's
        // error says so in the same words a populated corpus would use.
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(err.into()),
    };

    Ok(dir
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name();
            let name = name.to_str()?;
            let run_id = name.strip_suffix(".jsonl")?;
            run_id.starts_with(prefix).then(|| run_id.to_string())
        })
        .collect())
}

/// Whether `s` is a uuid by shape rather than by length.
///
/// Counting to 36 would admit 36 characters of anything, which is the hole a
/// rung-two proposer walked through on pass `01a084d3-bdd3` when it fed an
/// invented id to a length check to make a test pass.
fn is_uuid(s: &str) -> bool {
    s.len() == 36
        && s.char_indices().all(|(i, c)| match i {
            8 | 13 | 18 | 23 => c == '-',
            _ => c.is_ascii_hexdigit(),
        })
}
