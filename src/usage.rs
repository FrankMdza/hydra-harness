//! `hydra usage recount [<run-id>]`, harness spec 17.
//!
//! No provider in v1 produces `usage_source: estimated`, so this command walks
//! the corpus, tallies what each run carries, and says there is nothing to
//! recount. The surface exists so the tool is already here the day a producer
//! earns the value.

use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

use crate::normalize::{self, Run, UsageSource};
use crate::outcome::Usage;
use crate::paths::Layout;
use crate::policy::PolicySet;

/// Tallies `usage_source` across one record or the whole corpus and prints the
/// counts. `out` is stdout for the command and a buffer for the suite.
pub fn recount(layout: &Layout, run_id: Option<&str>, out: &mut dyn Write) -> Result<()> {
    let policies = PolicySet::discover()?;
    let mut provider = 0u64;
    let mut estimated = 0u64;
    let mut unavailable = 0u64;

    let paths = if let Some(id) = run_id {
        let path = layout.run_file(id);
        if !path.is_file() {
            bail!("no run record at {}", path.display());
        }
        vec![path]
    } else {
        run_files(layout)?
    };

    for path in paths {
        match read_run(&path, &policies, layout) {
            Ok(run) => tally(&run, &mut provider, &mut estimated, &mut unavailable),
            Err(err) => eprintln!("hydra: skipping {}: {err:#}", path.display()),
        }
    }

    writeln!(out, "provider: {provider}")?;
    writeln!(out, "estimated: {estimated}")?;
    writeln!(out, "unavailable: {unavailable}")?;
    if estimated == 0 {
        writeln!(
            out,
            "nothing to recount: no run carries usage_source estimated"
        )?;
    } else {
        writeln!(
            out,
            "{estimated} runs carry usage_source estimated and no tokenizer is built to recount them"
        )?;
    }
    Ok(())
}

fn run_files(layout: &Layout) -> Result<Vec<PathBuf>> {
    let dir = layout.runs();
    let entries = match std::fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(err).with_context(|| format!("reading {}", dir.display())),
    };

    let mut paths: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("jsonl"))
        .collect();
    paths.sort();
    Ok(paths)
}

fn read_run(path: &Path, policies: &PolicySet, layout: &Layout) -> Result<Run> {
    let text =
        std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    normalize::read_with(&text, policies, layout)
}

fn tally(run: &Run, provider: &mut u64, estimated: &mut u64, unavailable: &mut u64) {
    match usage_source(run) {
        UsageSource::Provider => *provider += 1,
        UsageSource::Estimated => *estimated += 1,
        UsageSource::Unavailable => *unavailable += 1,
    }
}

fn usage_source(run: &Run) -> UsageSource {
    let source = Usage::of(run).usage_source;
    match source {
        "provider" => UsageSource::Provider,
        "estimated" => UsageSource::Estimated,
        _ => UsageSource::Unavailable,
    }
}
