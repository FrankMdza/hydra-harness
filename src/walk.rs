//! Corpus walks: the assertions that used to run inside `cargo test` over
//! `~/hydra/runs/`, now shared by `hydra doctor --walk` and the fixture walks
//! in `tests/record_contract.rs`, `tests/normalize.rs` and `tests/outcome.rs`.

use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::normalize::{Decider, Decision, Finding as NormalizeFinding, Run};
use crate::outcome::{Outcome, Source, derive};
use crate::paths::Layout;
use crate::policy::PolicySet;
use crate::record;

/// One assertion the walk refused over one record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub path: PathBuf,
    pub assertion: String,
}

/// What a walk over a corpus returned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    pub findings: Vec<Finding>,
    pub records_read: usize,
}

/// Counts the three normalization buckets the walk tracks.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct NormalizeCounts {
    pub gated: usize,
    pub pre_gate: usize,
    pub harness: usize,
}

/// One closed record's derived outcome, for fixture tests that count shapes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DerivedOutcome {
    pub run_id: String,
    pub outcome: String,
    pub source: String,
}

/// Every `.jsonl` under a `runs/` directory, sorted.
pub fn records_in(runs_dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(runs_dir) else {
        return Vec::new();
    };
    let mut paths: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("jsonl"))
        .collect();
    paths.sort();
    paths
}

/// Runs every walk assertion over the layout's `runs/` directory.
pub fn walk_corpus(layout: &Layout) -> Result<Report> {
    let policies = PolicySet::discover()?;
    let paths = records_in(&layout.runs());
    let read = |path: &Path| read_record(path, &policies, layout);
    Ok(walk_paths(&paths, &read))
}

/// Runs every walk assertion over an explicit list of records.
pub fn walk_paths(paths: &[PathBuf], read: &dyn Fn(&Path) -> Result<Run>) -> Report {
    let mut findings = Vec::new();
    findings.extend(check_contract(paths));
    findings.extend(check_normalizes(paths, read).0);
    findings.extend(check_write_denials(paths, read).0);
    findings.extend(check_gate_leaks(paths, read));
    findings.extend(check_normalizing_writes_nothing(paths, read));
    findings.extend(check_shell_pairs(paths, read).0);
    findings.extend(check_outcomes(paths, read).0);
    findings.extend(check_gated_write_denials(paths, read).0);
    findings.extend(check_unknown_sources(paths, read));
    Report {
        findings,
        records_read: paths.len(),
    }
}

fn read_record(path: &Path, policies: &PolicySet, layout: &Layout) -> Result<Run> {
    let text = std::fs::read_to_string(path)
        .map_err(|err| anyhow::anyhow!("reading {}: {err}", path.display()))?;
    crate::normalize::read_with(&text, policies, layout)
        .map_err(|err| anyhow::anyhow!("{} did not normalize: {err:#}", path.display()))
}

fn push(findings: &mut Vec<Finding>, path: &Path, assertion: impl Into<String>) {
    findings.push(Finding {
        path: path.to_path_buf(),
        assertion: assertion.into(),
    });
}

/// Contract validation, one problem per finding.
pub fn check_contract(paths: &[PathBuf]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for path in paths {
        match record::validate::check_file(path) {
            Ok(report) if report.ok() => {}
            Ok(report) => {
                for problem in report.problems {
                    push(
                        &mut findings,
                        path,
                        format!("does not meet the contract: {problem}"),
                    );
                }
            }
            Err(err) => push(
                &mut findings,
                path,
                format!("does not meet the contract: {err:#}"),
            ),
        }
    }
    findings
}

/// Normalization shape checks. Returns counts for fixture tests that measure them.
pub fn check_normalizes(
    paths: &[PathBuf],
    read: &dyn Fn(&Path) -> Result<Run>,
) -> (Vec<Finding>, NormalizeCounts) {
    let mut findings = Vec::new();
    let mut counts = NormalizeCounts::default();
    for path in paths {
        let run = match read(path) {
            Ok(run) => run,
            Err(err) => {
                push(&mut findings, path, err.to_string());
                continue;
            }
        };
        if run.run_start().is_none() {
            push(&mut findings, path, "missing run_start");
            continue;
        }
        if run.provider.is_none() {
            let start = run.run_start().unwrap();
            if start.body["command"].as_str() != Some("evolve") {
                push(
                    &mut findings,
                    path,
                    format!(
                        "harness run_start.command is {:?}, expected evolve",
                        start.body["command"]
                    ),
                );
            }
            if run.tool_calls().count() != 0 {
                push(&mut findings, path, "harness run has tool calls");
            }
            counts.harness += 1;
            continue;
        }
        if !matches!(
            run.provider.as_deref(),
            Some("claude") | Some("cursor-agent") | Some("agy") | Some("codex")
        ) {
            push(
                &mut findings,
                path,
                format!("unknown provider {:?}", run.provider),
            );
            continue;
        }
        match run.policy {
            None => {
                if run.permission_decisions().count() != 0 {
                    push(
                        &mut findings,
                        path,
                        "has no policy and still produced a decision",
                    );
                }
                counts.pre_gate += 1;
            }
            Some(_) => {
                let decisions = run.permission_decisions().count();
                let calls = run.tool_calls().count();
                if decisions != calls {
                    push(
                        &mut findings,
                        path,
                        format!("owes one decision per call: {decisions} decisions, {calls} calls"),
                    );
                }
                counts.gated += 1;
            }
        }
        if run.usage().is_none() {
            push(&mut findings, path, "missing usage");
        }
    }
    (findings, counts)
}

/// Write denials under `read-only` must derive to a deny under capability.
pub fn check_write_denials(
    paths: &[PathBuf],
    read: &dyn Fn(&Path) -> Result<Run>,
) -> (Vec<Finding>, usize) {
    let mut findings = Vec::new();
    let mut seen = 0;
    for path in paths {
        let run = match read(path) {
            Ok(run) => run,
            Err(err) => {
                push(&mut findings, path, err.to_string());
                continue;
            }
        };
        if run.policy.as_deref() != Some("read-only") {
            continue;
        }
        for call in run.tool_calls().filter(|call| call.name == "Write") {
            match run
                .permission_decisions()
                .find(|decision| decision.call_id == call.call_id)
            {
                Some(decision) => {
                    if decision.decision != Decision::Deny {
                        push(
                            &mut findings,
                            path,
                            format!("Write denial expected Deny, got {:?}", decision.decision),
                        );
                    }
                    if decision.decider != Decider::HydraPolicy {
                        push(
                            &mut findings,
                            path,
                            format!(
                                "Write denial expected HydraPolicy, got {:?}",
                                decision.decider
                            ),
                        );
                    }
                    if !decision.reason.starts_with("hydra/capability:") {
                        push(
                            &mut findings,
                            path,
                            format!("Write denial reason {:?}", decision.reason),
                        );
                    }
                    seen += 1;
                }
                None => push(
                    &mut findings,
                    path,
                    "Write call missing a permission decision",
                ),
            }
        }
    }
    (findings, seen)
}

/// Run ids whose silent gate is a measured exception, not a regression.
const MEASURED_SILENT_GATES: &[&str] = &["01a0c223-9ec2-72e0-b5d4-e29782d692b0"];

/// Gate leaks, ungated calls and stale-policy disagreements.
pub fn check_gate_leaks(paths: &[PathBuf], read: &dyn Fn(&Path) -> Result<Run>) -> Vec<Finding> {
    let mut findings = Vec::new();
    for path in paths {
        let run = match read(path) {
            Ok(run) => run,
            Err(err) => {
                push(&mut findings, path, err.to_string());
                continue;
            }
        };
        if MEASURED_SILENT_GATES.iter().any(|id| run.run_id == *id) {
            let ok = run.findings.iter().any(|finding| match finding {
                NormalizeFinding::GateLeak { .. } | NormalizeFinding::StalePolicy { .. } => true,
                NormalizeFinding::Limit(text) => text.contains("not the one compiled here"),
                _ => false,
            });
            if !ok {
                push(
                    &mut findings,
                    path,
                    format!(
                        "the measured silent gate reads as neither a leak nor a moved policy: {:?}",
                        run.findings
                    ),
                );
            }
            continue;
        }
        let leaks: Vec<_> = run
            .findings
            .iter()
            .filter(|finding| {
                matches!(
                    finding,
                    NormalizeFinding::GateLeak { .. }
                        | NormalizeFinding::SubagentLeak { .. }
                        | NormalizeFinding::Ungated { .. }
                )
            })
            .collect();
        if !leaks.is_empty() {
            push(&mut findings, path, format!("gate finding: {leaks:?}"));
        }
        let rebuilt = run
            .findings
            .iter()
            .filter(|finding| rebuilt_the_scopes(finding))
            .count();
        if rebuilt > 1 {
            push(
                &mut findings,
                path,
                format!(
                    "the reader said it rebuilt the scopes {rebuilt} times, and it is once per record"
                ),
            );
        }
    }
    findings
}

/// Contract 11: normalizing must change no byte of a record on disk.
pub fn check_normalizing_writes_nothing(
    paths: &[PathBuf],
    read: &dyn Fn(&Path) -> Result<Run>,
) -> Vec<Finding> {
    let before: Vec<(PathBuf, Vec<u8>)> = paths
        .iter()
        .filter_map(|path| std::fs::read(path).ok().map(|bytes| (path.clone(), bytes)))
        .collect();
    if before.is_empty() {
        return Vec::new();
    }
    let mut findings = Vec::new();
    for (path, _) in &before {
        if let Err(err) = read(path) {
            push(&mut findings, path, err.to_string());
        }
    }
    let after_count = paths.len();
    if after_count < before.len() {
        findings.push(Finding {
            path: paths[0].clone(),
            assertion: "normalizing removed a file under runs/".into(),
        });
    }
    for (path, bytes) in &before {
        match std::fs::read(path) {
            Ok(after) if after.starts_with(bytes) => {}
            Ok(_) => push(
                &mut findings,
                path,
                "bytes that were there before the read are not there after it",
            ),
            Err(err) => push(
                &mut findings,
                path,
                format!("could not re-read after normalizing: {err}"),
            ),
        }
    }
    findings
}

/// Ticket 06's shape: two `Bash` calls where the first names the corpus and the second does not.
pub fn check_shell_pairs(
    paths: &[PathBuf],
    read: &dyn Fn(&Path) -> Result<Run>,
) -> (Vec<Finding>, usize) {
    let mut findings = Vec::new();
    let mut pairs = 0;
    for path in paths {
        let run = match read(path) {
            Ok(run) => run,
            Err(err) => {
                push(&mut findings, path, err.to_string());
                continue;
            }
        };
        if run.policy.as_deref() != Some("standard") {
            continue;
        }
        let shell: Vec<_> = run
            .tool_calls()
            .filter(|call| call.name == "Bash")
            .collect();
        if shell.len() != 2 {
            continue;
        }
        let names_corpus = |call: &crate::normalize::ToolCall| {
            call.arguments["command"]
                .as_str()
                .is_some_and(|command| command.contains("hydra/runs"))
        };
        if !names_corpus(shell[0]) || names_corpus(shell[1]) {
            continue;
        }
        let decision_for = |call_id: &str| {
            run.permission_decisions()
                .find(|decision| decision.call_id == call_id)
                .map(|decision| (decision.decision, decision.decider, decision.reason.clone()))
        };
        match decision_for(&shell[0].call_id) {
            Some((decision, decider, reason)) => {
                if decision != Decision::Deny {
                    push(
                        &mut findings,
                        path,
                        format!("first Bash expected Deny, got {:?}", decision),
                    );
                }
                if decider != Decider::HydraPolicy {
                    push(
                        &mut findings,
                        path,
                        format!("first Bash expected HydraPolicy, got {:?}", decider),
                    );
                }
                if !reason.starts_with("hydra/protected-path:") {
                    push(&mut findings, path, format!("first Bash reason {reason}"));
                }
            }
            None => push(
                &mut findings,
                path,
                "first Bash missing a permission decision",
            ),
        }
        match decision_for(&shell[1].call_id) {
            Some((decision, decider, reason)) => {
                if decision != Decision::Allow {
                    push(
                        &mut findings,
                        path,
                        format!("second Bash expected Allow, got {:?}", decision),
                    );
                }
                if decider != Decider::HydraPolicy {
                    push(
                        &mut findings,
                        path,
                        format!("second Bash expected HydraPolicy, got {:?}", decider),
                    );
                }
                if !reason.starts_with("hydra/allow:") {
                    push(&mut findings, path, format!("second Bash reason {reason}"));
                }
            }
            None => push(
                &mut findings,
                path,
                "second Bash missing a permission decision",
            ),
        }
        pairs += 1;
    }
    (findings, pairs)
}

/// Every closed record derives an outcome; an open one derives none.
pub fn check_outcomes(
    paths: &[PathBuf],
    read: &dyn Fn(&Path) -> Result<Run>,
) -> (Vec<Finding>, Vec<DerivedOutcome>) {
    let mut findings = Vec::new();
    let mut seen = Vec::new();
    for path in paths {
        let run = match read(path) {
            Ok(run) => run,
            Err(err) => {
                push(&mut findings, path, err.to_string());
                continue;
            }
        };
        match derive(&run) {
            Some(derived) => {
                seen.push(DerivedOutcome {
                    run_id: run.run_id.clone(),
                    outcome: derived.outcome.as_str().to_string(),
                    source: derived.source.as_str().to_string(),
                });
            }
            None if run.run_end().is_some() => {
                push(
                    &mut findings,
                    path,
                    "has a run_end and no outcome came out of it",
                );
            }
            None => {}
        }
    }
    (findings, seen)
}

/// Per-call outcome rule and the reasons a run steps out of it.
pub fn check_gated_write_denials(
    paths: &[PathBuf],
    read: &dyn Fn(&Path) -> Result<Run>,
) -> (Vec<Finding>, usize) {
    let mut findings = Vec::new();
    let mut gated = 0;
    for path in paths {
        let run = match read(path) {
            Ok(run) => run,
            Err(err) => {
                push(&mut findings, path, err.to_string());
                continue;
            }
        };
        let Some(derived) = derive(&run) else {
            continue;
        };
        let outcome = derived.outcome;

        if derived.source == Source::Refusal {
            if outcome.rung_one_learns() {
                push(
                    &mut findings,
                    path,
                    format!("a refused launch came out {}", outcome.as_str()),
                );
            }
            continue;
        }

        let status = run
            .result
            .as_ref()
            .and_then(|result| result["api_error_status"].as_u64());
        if status.is_some() {
            if outcome.rung_one_learns() {
                push(
                    &mut findings,
                    path,
                    format!("api_error_status {status:?} came out {}", outcome.as_str()),
                );
            }
            continue;
        }

        if run
            .result
            .as_ref()
            .is_some_and(|result| result["type"] == "turn.failed")
        {
            let quota = run.result.as_ref().unwrap()["error"]["message"]
                .as_str()
                .is_some_and(|text| text.contains("usage limit"));
            let expected = if quota {
                Outcome::RateLimited
            } else {
                Outcome::Failed
            };
            if outcome != expected {
                push(
                    &mut findings,
                    path,
                    format!("expected {}, got {}", expected.as_str(), outcome.as_str()),
                );
            }
            continue;
        }

        if run.result.is_none() && !outcome.rung_one_learns() {
            continue;
        }

        if let Some(code) = run.run_end().and_then(|end| end.body["refusal"].as_str()) {
            let expected = record::Refusal::outcome_of(code);
            if outcome != expected {
                push(
                    &mut findings,
                    path,
                    format!(
                        "refusal {code}: expected {}, got {}",
                        expected.as_str(),
                        outcome.as_str()
                    ),
                );
            }
            if outcome.rung_one_learns() {
                push(&mut findings, path, "a refused launch taught a lesson");
            }
            if derive(&run).unwrap().source != Source::Refusal {
                push(&mut findings, path, "refusal source mismatch");
            }
            continue;
        }

        if outcome == Outcome::Failed {
            if run.result.is_none() {
                continue;
            }
            let exit = run.run_end().map(|end| end.body["exit_code"].clone());
            if exit == Some(serde_json::json!(0)) {
                push(&mut findings, path, "failed with exit 0");
            }
            continue;
        }

        if outcome == Outcome::Crashed {
            let error = run
                .run_end()
                .and_then(|end| end.body["error"].as_str().map(str::to_string));
            if !error.is_some_and(|said| said.contains("signal")) {
                push(
                    &mut findings,
                    path,
                    "came out crashed with no signal on run_end.error",
                );
            }
            continue;
        }

        let refused = run
            .permission_decisions()
            .filter(|d| d.decision == Decision::Deny)
            .count();
        let calls = run.permission_decisions().count();
        let expected = if calls > 0 && refused == calls {
            gated += 1;
            Outcome::Gated
        } else {
            Outcome::Completed
        };
        if outcome != expected {
            push(
                &mut findings,
                path,
                format!(
                    "expected {}, got {}: {calls} call(s), {refused} refused",
                    expected.as_str(),
                    outcome.as_str()
                ),
            );
        }
    }
    (findings, gated)
}

/// Unknown source only where a result is unclassified and the run completed.
pub fn check_unknown_sources(
    paths: &[PathBuf],
    read: &dyn Fn(&Path) -> Result<Run>,
) -> Vec<Finding> {
    let mut findings = Vec::new();
    for path in paths {
        let run = match read(path) {
            Ok(run) => run,
            Err(err) => {
                push(&mut findings, path, err.to_string());
                continue;
            }
        };
        let Some(derived) = derive(&run) else {
            continue;
        };
        let source = derived.source;
        let empty_harvest = run
            .run_end()
            .is_some_and(|end| end.body["error"].is_string())
            && run.messages().next().is_none()
            && run.tool_results().next().is_none();
        if empty_harvest {
            continue;
        }
        let unclassified = run
            .tool_results()
            .any(|r| r.status == crate::normalize::ResultStatus::Unclassified);
        let completed = derive(&run).unwrap().outcome == Outcome::Completed;
        if (source == Source::Unknown) != (unclassified && completed) {
            push(
                &mut findings,
                path,
                format!(
                    "unknown source mismatch: source={}, unclassified={}, completed={}",
                    source.as_str(),
                    unclassified,
                    completed
                ),
            );
        }
    }
    findings
}

fn rebuilt_the_scopes(finding: &NormalizeFinding) -> bool {
    matches!(finding, NormalizeFinding::Limit(said) if said.contains("carries no scopes"))
}
