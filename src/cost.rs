//! `hydra cost`: orchestrator tokens against the agents of each run, map 107 / build 274.
//!
//! Given orchestrator run ids, it finds each one's agents (started after it, in
//! its repository), prints the token ratio per run, and the median and spread.
//! `--check` prints the same for real sessions, beside the set.
//!
//! **The command lives here rather than in `crates/puppets` or under
//! `integrations/bench/`.** It walks a [`Layout`] and the run records the way
//! `hydra usage` and `hydra show` do. `hydra-puppets` reads goal logs and does
//! not depend on this crate; a script would duplicate the read. The goals
//! themselves sit under `integrations/bench/goals/` and run against a scratch
//! copy of `integrations/bench/repo/`. This command launches neither.

use std::collections::BTreeSet;
use std::io::Write;
use std::path::Path;

use anyhow::{Context, Result, ensure};
use chrono::{DateTime, Utc};
use serde_json::Value;

use crate::live;
use crate::normalize;
use crate::outcome::Usage;
use crate::paths::Layout;
use crate::policy::PolicySet;
use crate::resolver;

/// What the human asked for.
pub struct CostRequest {
    /// Orchestrator runs of the fixed goal set.
    pub runs: Vec<String>,
    /// Real sessions printed beside the set.
    pub check: Vec<String>,
}

/// One named orchestrator, after resolving the id the human typed.
struct Named {
    id: String,
    started: DateTime<Utc>,
    repo: String,
    tokens: u64,
}

/// Every recorded run the scan could read far enough to match.
struct Indexed {
    id: String,
    started: Option<DateTime<Utc>>,
    repo: Option<String>,
    provider: Option<String>,
    tokens: u64,
}

/// One printed row.
struct Row {
    set: &'static str,
    id: String,
    agents: usize,
    orchestrator: u64,
    sum: u64,
    ratio: Option<f64>,
}

/// Resolve the named runs, find their agents, print the ratios and the median.
pub fn command(layout: &Layout, request: CostRequest, out: &mut dyn Write) -> Result<()> {
    ensure!(
        !request.runs.is_empty(),
        "at least one orchestrator run is required"
    );
    let policies = PolicySet::discover()?;
    let indexed = index(layout, &policies)?;
    let bench = load_named(layout, &request.runs, &indexed)?;
    let check = load_named(layout, &request.check, &indexed)?;
    let named_ids = bench
        .iter()
        .chain(check.iter())
        .map(|run| run.id.as_str())
        .collect::<BTreeSet<_>>();

    let bench_rows = rows("bench", &bench, &indexed, &named_ids);
    let check_rows = rows("check", &check, &indexed, &named_ids);

    writeln!(out, "set\trun_id\tagents\torchestrator\tsum\tratio")?;
    for row in bench_rows.iter().chain(check_rows.iter()) {
        write_row(out, row)?;
    }
    writeln!(out, "set\tn\tmedian\tmin\tmax")?;
    write_summary(out, "bench", &bench_rows)?;
    if !check.is_empty() {
        write_summary(out, "check", &check_rows)?;
    }
    Ok(())
}

fn load_named(layout: &Layout, ids: &[String], indexed: &[Indexed]) -> Result<Vec<Named>> {
    let mut named = Vec::with_capacity(ids.len());
    for raw in ids {
        let id = resolver::resolve(layout, raw)?;
        let run = indexed
            .iter()
            .find(|run| run.id == id)
            .with_context(|| format!("no run record for {id}"))?;
        let started = run
            .started
            .with_context(|| format!("run {id} carries no timestamp"))?;
        let repo = run
            .repo
            .clone()
            .with_context(|| format!("run {id} names no repository"))?;
        named.push(Named {
            id,
            started,
            repo,
            tokens: run.tokens,
        });
    }
    Ok(named)
}

fn rows(
    set: &'static str,
    named: &[Named],
    indexed: &[Indexed],
    named_ids: &BTreeSet<&str>,
) -> Vec<Row> {
    named
        .iter()
        .map(|orch| {
            let until = named
                .iter()
                .filter(|other| {
                    other.id != orch.id && other.repo == orch.repo && other.started > orch.started
                })
                .map(|other| other.started)
                .min();
            let agents: Vec<&Indexed> = indexed
                .iter()
                .filter(|run| is_agent(run, orch, until, named_ids))
                .collect();
            let sum: u64 = agents.iter().map(|run| run.tokens).sum();
            let ratio = if sum == 0 {
                None
            } else {
                Some(orch.tokens as f64 / sum as f64)
            };
            Row {
                set,
                id: orch.id.clone(),
                agents: agents.len(),
                orchestrator: orch.tokens,
                sum,
                ratio,
            }
        })
        .collect()
}

fn is_agent(
    run: &Indexed,
    orch: &Named,
    until: Option<DateTime<Utc>>,
    named_ids: &BTreeSet<&str>,
) -> bool {
    if named_ids.contains(run.id.as_str()) {
        return false;
    }
    if run.provider.is_none() {
        return false;
    }
    let Some(started) = run.started else {
        return false;
    };
    let Some(repo) = run.repo.as_deref() else {
        return false;
    };
    if repo != orch.repo {
        return false;
    }
    if started <= orch.started {
        return false;
    }
    if until.is_some_and(|end| started >= end) {
        return false;
    }
    true
}

fn write_row(out: &mut dyn Write, row: &Row) -> Result<()> {
    let ratio = row
        .ratio
        .map(|value| format!("{value:.3}"))
        .unwrap_or_default();
    writeln!(
        out,
        "{}\t{}\t{}\t{}\t{}\t{ratio}",
        row.set, row.id, row.agents, row.orchestrator, row.sum
    )?;
    Ok(())
}

fn write_summary(out: &mut dyn Write, set: &str, rows: &[Row]) -> Result<()> {
    let ratios: Vec<f64> = rows.iter().filter_map(|row| row.ratio).collect();
    let n = ratios.len();
    match median_f64(&ratios) {
        Some(median) => {
            let min = ratios.iter().copied().fold(f64::INFINITY, f64::min);
            let max = ratios.iter().copied().fold(f64::NEG_INFINITY, f64::max);
            writeln!(out, "{set}\t{n}\t{median:.3}\t{min:.3}\t{max:.3}")?;
        }
        None => writeln!(out, "{set}\t0\t\t\t")?,
    }
    Ok(())
}

fn index(layout: &Layout, policies: &PolicySet) -> Result<Vec<Indexed>> {
    let views = live::scan(layout)?;
    let mut runs = Vec::with_capacity(views.len());
    for view in views {
        match load_one(layout, policies, &view.run_file, &view.run_id) {
            Ok(run) => runs.push(run),
            Err(err) => eprintln!("hydra: skipping {}: {err:#}", view.run_file.display()),
        }
    }
    Ok(runs)
}

fn load_one(layout: &Layout, policies: &PolicySet, path: &Path, run_id: &str) -> Result<Indexed> {
    let text =
        std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let first = text.lines().next().unwrap_or("");
    let envelope: Value = serde_json::from_str(first)
        .with_context(|| format!("parsing line 0 of {}", path.display()))?;
    let body = &envelope["hydra"];
    let started = envelope["ts"].as_str().and_then(parse_time);
    let repo = repository(body);
    let provider = envelope["provider"].as_str().map(str::to_string);
    let tokens = match normalize::read_with(&text, policies, layout) {
        Ok(run) => Usage::of(&run)
            .counts
            .map(|counts| counts.total_tokens)
            .unwrap_or(0),
        Err(_) => 0,
    };
    Ok(Indexed {
        id: envelope["run_id"]
            .as_str()
            .filter(|id| !id.is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| run_id.to_string()),
        started,
        repo,
        provider,
        tokens,
    })
}

/// The repository a run was launched from: `scopes.parent_git_dir` stripped of
/// `/.git` when the gate recorded it, otherwise `cwd`. An interactive session
/// without a worktree has only `cwd`; a recorded agent has the git dir.
fn repository(start: &Value) -> Option<String> {
    if let Some(git_dir) = start["scopes"]["parent_git_dir"].as_str() {
        return Some(repo_from_git_dir(git_dir));
    }
    start["cwd"]
        .as_str()
        .map(|cwd| cwd.trim_end_matches('/').to_string())
}

fn repo_from_git_dir(git_dir: &str) -> String {
    let trimmed = git_dir.trim_end_matches('/');
    trimmed.strip_suffix("/.git").unwrap_or(trimmed).to_string()
}

fn parse_time(text: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(text)
        .ok()
        .map(|t| t.with_timezone(&Utc))
}

fn median_f64(samples: &[f64]) -> Option<f64> {
    if samples.is_empty() {
        return None;
    }
    let mut xs = samples.to_vec();
    xs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = xs.len();
    if n % 2 == 1 {
        Some(xs[n / 2])
    } else {
        Some((xs[n / 2 - 1] + xs[n / 2]) / 2.0)
    }
}

#[cfg(test)]
mod tests {
    use super::{median_f64, repo_from_git_dir, repository};
    use serde_json::json;

    #[test]
    fn a_parent_git_dir_strips_the_git_suffix() {
        assert_eq!(repo_from_git_dir("/tmp/tally/.git"), "/tmp/tally");
        assert_eq!(repo_from_git_dir("/tmp/tally/.git/"), "/tmp/tally");
        assert_eq!(repo_from_git_dir("/tmp/tally"), "/tmp/tally");
    }

    #[test]
    fn an_interactive_cwd_is_the_repository_when_no_git_dir_is_recorded() {
        let start = json!({"cwd": "/tmp/tally", "scopes": {}});
        assert_eq!(repository(&start).as_deref(), Some("/tmp/tally"));
    }

    #[test]
    fn a_recorded_agent_matches_the_interactive_cwd() {
        let orch = json!({"cwd": "/tmp/tally"});
        let agent = json!({
            "cwd": "/hydra/worktrees/agent",
            "scopes": {"parent_git_dir": "/tmp/tally/.git"},
        });
        assert_eq!(repository(&orch), repository(&agent));
    }

    #[test]
    fn median_is_the_middle_or_the_mean_of_the_two_middle() {
        assert_eq!(median_f64(&[]), None);
        assert_eq!(median_f64(&[0.4]), Some(0.4));
        assert_eq!(median_f64(&[0.2, 0.6]), Some(0.4));
        assert_eq!(median_f64(&[0.2, 0.4, 0.6]), Some(0.4));
    }
}
