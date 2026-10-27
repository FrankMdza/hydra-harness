//! Corpus walks: the assertions that used to run inside `cargo test` over
//! `~/hydra/runs/`, now shared by `hydra doctor --walk` and the fixture walks
//! in `tests/record_contract.rs`, `tests/normalize.rs` and `tests/outcome.rs`.

use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
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
    pub rejected_before_hook: usize,
    /// The `rejected_before_hook` rows counted by tool and error shape, keyed
    /// by `rejected_shape_label`. Sums to `rejected_before_hook`.
    pub rejected_by_shape: BTreeMap<String, usize>,
    pub ungated: usize,
    /// How many records each host wrote, keyed `machine os/arch`, build
    /// ticket 316. A record from before the field counts under no key.
    pub hosts: BTreeMap<String, usize>,
    /// Records excluded from replay assertions, counted under their own host.
    pub foreign_hosts: BTreeMap<String, usize>,
    /// Build ticket 332: each stored `gate_refusal` row read against its call.
    pub refusals: RefusalCounts,
    /// The rows that matched no failed call. A limit of the record, printed
    /// and not failed on: the run's own shell can write the sidecar.
    pub uncorroborated: Vec<Finding>,
}

/// How the `gate_refusal` rows of a corpus read against their provider
/// streams, build ticket 332.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RefusalCounts {
    /// The row's `call_id` names a call whose result failed.
    pub corroborated: usize,
    /// The row's `call_id` names no call, a call with no result, or a call
    /// whose result is a success. Includes `no_frame`.
    pub uncorroborated: usize,
    /// Build ticket 339: the row's `call_id` names no tool call in the record's
    /// stream, so no frame for it exists. A subset of `uncorroborated`.
    pub no_frame: usize,
    /// The hook supplied no `call_id`, so there is nothing to join on.
    pub no_call_id: usize,
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
    let records_read = paths.len();
    let hosts = hosts_of(paths);
    let foreign_hosts = host_counts(paths)
        .into_iter()
        .filter(|(host, _)| host.is_other())
        .map(|(host, count)| (host.label(), count))
        .collect();
    let contract = check_contract(paths);
    let local: Vec<_> = paths
        .iter()
        .filter(|path| !host_of(path).is_some_and(|host| host.is_other()))
        .cloned()
        .collect();
    let paths = local.as_slice();
    let cache = RefCell::new(HashMap::<PathBuf, Result<Run, String>>::new());
    let read = |path: &Path| cached_read(&cache, path, read);
    // The write-nothing snapshot has to bracket the first (and, with the cache,
    // only) normalize. Findings stay in the order the checks used to run.
    let writes_nothing = check_normalizing_writes_nothing(paths, &read);
    let mut findings = Vec::new();
    findings.extend(contract);
    findings.extend(check_normalizes(paths, &read).0);
    findings.extend(check_write_denials(paths, &read).0);
    findings.extend(check_gate_leaks(paths, &read));
    findings.extend(writes_nothing);
    findings.extend(check_shell_pairs(paths, &read).0);
    findings.extend(check_outcomes(paths, &read).0);
    findings.extend(check_gated_write_denials(paths, &read).0);
    findings.extend(check_unknown_sources(paths, &read));
    let (uncorroborated, refusals) = check_gate_refusals(paths, &read);
    let mut rejected_before_hook = 0;
    let mut rejected_by_shape = BTreeMap::new();
    let mut ungated = 0;
    for path in paths {
        if let Ok(run) = read(path) {
            for finding in run.findings {
                match finding {
                    NormalizeFinding::RejectedBeforeHook { tool, error, .. } => {
                        rejected_before_hook += 1;
                        *rejected_by_shape
                            .entry(rejected_shape_label(&tool, &error))
                            .or_insert(0) += 1;
                    }
                    NormalizeFinding::Ungated { .. } => ungated += 1,
                    _ => {}
                }
            }
        }
    }
    Report {
        findings,
        records_read,
        rejected_before_hook,
        rejected_by_shape,
        ungated,
        hosts,
        foreign_hosts,
        refusals,
        uncorroborated,
    }
}

/// Buckets one rejected call by its tool and the first line of its error.
/// The buckets name the error shapes seen in corpora so far. An error that
/// matches none of them keeps its first 80 characters.
fn rejected_shape_label(tool: &str, error: &str) -> String {
    let shape = if error.contains("InputValidationError") {
        "InputValidationError"
    } else if error.contains("No such tool available") {
        "No such tool available"
    } else if error.starts_with("<tool_use_error>Blocked:") {
        "<tool_use_error>Blocked"
    } else if error.starts_with("<tool_use_error>File has not been read yet") {
        "<tool_use_error>File has not been read yet"
    } else if error.starts_with("<tool_use_error>File does not exist") {
        "<tool_use_error>File does not exist"
    } else if error.starts_with("<tool_use_error>String to replace not found") {
        "<tool_use_error>String to replace not found"
    } else if error.starts_with("<tool_use_error>No task found with ID") {
        "<tool_use_error>No task found with ID"
    } else if error.starts_with("<tool_use_error>Error calling tool") {
        "<tool_use_error>Error calling tool"
    } else if error.starts_with("<tool_use_error>Sibling tool call errored") {
        "<tool_use_error>Sibling tool call errored"
    } else if error.starts_with("Not run: the response that made this tool call was stopped") {
        "safety classifier stopped call"
    } else {
        return format!("{tool}: {}", error.chars().take(80).collect::<String>());
    };
    format!("{tool}: {shape}")
}

/// The `run_start.host` of each record that carries one, counted. The label is
/// `machine os/arch`; a record whose line 0 does not parse or has no host is
/// left out, and the contract walk names the first case already.
pub fn hosts_of(paths: &[PathBuf]) -> BTreeMap<String, usize> {
    host_counts(paths)
        .into_iter()
        .map(|(host, count)| (host.label(), count))
        .collect()
}

/// Typed identities keep host boundaries independent of display formatting.
pub fn host_counts(paths: &[PathBuf]) -> BTreeMap<record::Host, usize> {
    let mut hosts = BTreeMap::new();
    for path in paths {
        if let Some(host) = host_of(path) {
            *hosts.entry(host).or_insert(0) += 1;
        }
    }
    hosts
}

fn host_of(path: &Path) -> Option<record::Host> {
    use std::io::BufRead;
    let file = std::fs::File::open(path).ok()?;
    let first = std::io::BufReader::new(file).lines().next()?.ok()?;
    let envelope: serde_json::Value = serde_json::from_str(&first).ok()?;
    record::Host::of(&envelope["hydra"])
}

fn cached_read(
    cache: &RefCell<HashMap<PathBuf, Result<Run, String>>>,
    path: &Path,
    read: &dyn Fn(&Path) -> Result<Run>,
) -> Result<Run> {
    if let Some(cached) = cache.borrow().get(path).cloned() {
        return cached.map_err(|err| anyhow::anyhow!("{err}"));
    }
    let result = read(path);
    cache.borrow_mut().insert(
        path.to_path_buf(),
        match &result {
            Ok(run) => Ok(run.clone()),
            Err(err) => Err(err.to_string()),
        },
    );
    result
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

/// Validates each record, naming the file and every problem when one fails.
///
/// Includes foreign records: host separation does not relax the contract.
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

/// Counts gated, pre-gate and harness records, asserting what holds of each.
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
        // Contract 5.5: a harness run has no provider, two lines and no decision
        // to derive. The first ones landed on 2026-09-08 with `hydra evolve`.
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
            // 7.2.1 and 5.6: a run with no policy has nothing to re-evaluate.
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
                let other_host = run
                    .run_start()
                    .and_then(|start| record::Host::of(&start.body))
                    .is_some_and(|host| host.is_other());
                let calls = if other_host {
                    0
                } else {
                    run.tool_calls().count()
                };
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
        // 7.8: never a bare null, on every record.
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
        if run.policy.as_deref() != Some("read-only")
            || run
                .run_start()
                .and_then(|start| record::Host::of(&start.body))
                .is_some_and(|host| host.is_other())
        {
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
    // The three read-only runs ticket 02 measured each asked for one Write.
    (findings, seen)
}

/// The one record on this machine where the gate did stay silent, kept in the
/// corpus on purpose. Run 01a0c223-9ec2 is build ticket 48's measurement that
/// `codex` 0.154.0 reads no project hook file inside a git worktree: the file
/// was in place, `cat ~/hydra/runs/nothing.jsonl` ran, and the reader files
/// both calls as leaks, which is the reader being right. The launcher hands
/// the hook over on the command line since that day, and run 01a0c231-6583 is
/// the same probe refused. A second id here needs the same sentence.
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
            // Under the policy set it ran under, the record reads as two
            // leaks. Once the set moves, build ticket 49's rule files the same
            // disagreement as a `StalePolicy` beside a limit saying the gate
            // named is not the one compiled here, because a replay under
            // today's set cannot tell a changed gate from a leak. Either
            // reading is the reader being right about this record.
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
                        // Every gated call on this machine has a gate answer in
                        // the stream, which is what makes the fail-open check
                        // worth having rather than noise.
                        | NormalizeFinding::Ungated { .. }
                )
            })
            .collect();
        if !leaks.is_empty() {
            push(&mut findings, path, format!("gate finding: {leaks:?}"));
        }

        // Every record on this machine predates `run_start.scopes`, so every one
        // of them that had a decision to derive says once that it rebuilt the
        // paths. The walk has to keep reading those records rather than treat
        // the limit as a fault, which is what build ticket 15 asks of it.
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

/// Contract 11 forbids a normalized field on disk, and the cheapest check that
/// it stays that way is that reading the corpus changes no byte of it.
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
    // A child may be writing while this runs, and a launch may add a file, so
    // the check is that nothing this read owns changed: no file went away, and
    // every byte that was there is still there in its place. A writer appending
    // to a live record is the one other process allowed to touch the corpus.
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

/// Counts the runs of ticket 06's shape and checks how each derives.
///
/// One `standard` run asked for two `Bash` calls: one naming `~/hydra/runs`,
/// one writing inside its own worktree. 7.5 refuses the first under every
/// policy and the second is ordinary shell, so a re-evaluation that collapsed
/// them would be caught here and nowhere else in this file.
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
        // The shape is the ticket 06 measurement's and not merely "two shell
        // calls": the first names the corpus, the second does not. An `evolve`
        // proposer on 2026-09-21 made two allowed `cargo` calls under `standard`
        // and matched the count alone, which blocked a pass at admission.
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
        // A run still being written has no `run_end`, and contract 6 says it is
        // not corrupt. A walk that runs while a child works skips it rather than
        // failing on it, which it did on 2026-09-07 with two runs live.
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

/// Checks every derived outcome against the per-call rule and the reasons a
/// run steps out of it, and counts the runs the rule calls gated.
///
/// Every record here whose provider got as far as answering reports
/// `subtype: success` and `terminal_reason: completed`, and one of them exits
/// 1 on a 429. What separates them is the gate's answer per call and the
/// status on the terminator, never the exit code.
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

        // A human stop, with or without a terminator, and whatever terminator it
        // was. A provider that catches SIGTERM can write `success` or, for
        // `codex`, `turn.failed` on its way out, and the record still derives
        // `stopped` off `run_end.error` (builds 173 and 175). Every rule below
        // describes how a run ended on its own, which this one did not.
        if outcome == Outcome::Stopped {
            continue;
        }

        // A launch Hydra refused has no stream at all, so the per-call rule below
        // is about a different question. Contract 8.2 calls that outcome a fact
        // Hydra wrote rather than a reading, and this loop only checks readings.
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

        // A provider that refused its own service never reached the gate, so the
        // per-call rule has nothing to say about it.
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

        // A `codex` turn that failed ends on `turn.failed`, and build ticket 100
        // reads it. Four landed on 2026-09-22 and 2026-09-23: three on the usage
        // limit, which rung one must not learn from, and one the provider's own
        // content filter stopped, which is `failed`. A harvested session says
        // the same on `event_msg` `task_complete` with a non-null `error`
        // (build ticket 260); without this row the walk expected `completed`
        // of a record that derived `rate_limited`.
        if run.result.as_ref().is_some_and(codex_failed_turn_result) {
            let result = run.result.as_ref().unwrap();
            let error = if result["type"] == "turn.failed" {
                &result["error"]
            } else {
                &result["payload"]["error"]
            };
            let quota = error["codex_error_info"].as_str() == Some("usage_limit_exceeded")
                || error["message"]
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

        // A provider that refused in prose on the pty never wrote a terminator,
        // which is what `cursor-agent` does on its monthly limit: one sentence,
        // exit 1, no frame. Contract 8 reads that off `noise`, and the per-call
        // rule has nothing to read. Three of these landed on 2026-09-07.
        if run.result.is_none() && !outcome.rung_one_learns() {
            continue;
        }

        // A run Hydra refused before the spawn has no calls for the per-call rule
        // to read, and contract 8.2 makes `refusal` the source anyway: `derive`
        // reads it ahead of anything on the stream. Ticket 07's skill collision
        // check is what first put one of these in the corpus.
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
            if derived.source != Source::Refusal {
                push(&mut findings, path, "refusal source mismatch");
            }
            continue;
        }

        // A provider that ran the task and then died on it is `failed`, contract
        // 12's second row, and the per-call rule says nothing about it. Two
        // landed on this machine: `agy` on 2026-09-07 timed out waiting for its
        // own model at 292 seconds, exit 1, with 41 gate answers before that and
        // a terminator on the stream; `codex` on 2026-09-24 (01a0d484) stopped
        // 27 seconds in with items and one `agent_message` but no turn terminator,
        // exit 0. Contract 8 reads a streamed run that stopped without a
        // terminator as `failed` and does not consult the exit code, so exit 0
        // is allowed when `run.result` is none. Every measured `failed` run that
        // does carry a terminator exits non-zero.
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

        // A child the kernel killed. `Crashed` is derived from the signal on
        // `run_end.error`, so the gate's per-call answer says nothing about it:
        // the run was stopped from outside and its calls were all allowed up to
        // the moment it died. One landed on 2026-09-15, when six runs were live
        // on an 8 GB machine and the OOM killer took one of them.
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
        // A harvested run with no body at all, a session that exited before its
        // first turn and wrote no transcript, has nothing to read an outcome
        // from and says so on `run_end.error`. Its source is unknown for a
        // different reason than the rule below tests, so it steps out. First
        // one landed 2026-09-08, run 01a07f6b-147d.
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
        let completed = derived.outcome == Outcome::Completed;
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

/// Reads each stored `gate_refusal` row's `call_id` against the provider
/// stream of the same record, build ticket 332.
///
/// Contract 7.7 calls the row hook evidence and not tamper-proof provenance:
/// the run's shell can append to the sidecar the hook writes. So a row only
/// corroborates when the provider's own stream shows the call it names and
/// that call failing. An `unclassified` result counts as failed, since that is
/// how `agy` reports a denial. Everything else comes back as an
/// `Uncorroborated` note naming the call id and the command, or the arguments
/// hash on a non-shell row, which the caller
/// prints and does not fail on, the way a `Foreign` gate leaves a finding
/// unbacked rather than wrong. A record that does not read is the other
/// checks' to report.
pub fn check_gate_refusals(
    paths: &[PathBuf],
    read: &dyn Fn(&Path) -> Result<Run>,
) -> (Vec<Finding>, RefusalCounts) {
    use crate::normalize::{Event, ResultStatus};
    let mut notes = Vec::new();
    let mut counts = RefusalCounts::default();
    for path in paths {
        let Ok(run) = read(path) else {
            continue;
        };
        let known: std::collections::HashSet<&str> =
            run.tool_calls().map(|call| call.call_id.as_str()).collect();
        let results: HashMap<&str, ResultStatus> = run
            .tool_results()
            .map(|result| (result.call_id.as_str(), result.status))
            .collect();
        for event in &run.events {
            let Event::GateRefusal(row) = event else {
                continue;
            };
            let Some(call_id) = row.body["call_id"].as_str() else {
                counts.no_call_id += 1;
                continue;
            };
            let why = if !known.contains(call_id) {
                counts.no_frame += 1;
                "matches no tool call"
            } else {
                match results.get(call_id) {
                    Some(ResultStatus::Error | ResultStatus::Unclassified) => {
                        counts.corroborated += 1;
                        continue;
                    }
                    Some(ResultStatus::Ok) => "matches a call whose result is a success",
                    None => "matches a call with no result",
                }
            };
            counts.uncorroborated += 1;
            // A shell row names its command. Any other row carries the hash of
            // its arguments, and the note prints that instead.
            let named = match row.body["command"].as_str() {
                Some(command) => format!("command {command:?}"),
                None => format!(
                    "arguments_sha256 {:?}",
                    row.body["arguments_sha256"].as_str().unwrap_or_default()
                ),
            };
            push(
                &mut notes,
                path,
                format!(
                    "Uncorroborated gate_refusal at seq {}: call {call_id:?} {why}; {named}",
                    row.seq,
                ),
            );
        }
    }
    (notes, counts)
}

fn rebuilt_the_scopes(finding: &NormalizeFinding) -> bool {
    matches!(finding, NormalizeFinding::Limit(said) if said.contains("carries no scopes"))
}

/// Streamed `turn.failed`, or the harvested session spelling of the same
/// ending: `event_msg` `task_complete` with a non-null `error`.
fn codex_failed_turn_result(result: &serde_json::Value) -> bool {
    result["type"] == "turn.failed"
        || (result["type"] == "event_msg"
            && result["payload"]["type"] == "task_complete"
            && !result["payload"]["error"].is_null())
}
