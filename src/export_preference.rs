//! Preference pairs for `hydra export`, contract 16.3.
//!
//! Corrections from eligible interactive runs and judge verdicts from rung-two
//! blocked passes each have a reader here, beside [`crate::export`], not inside
//! rung one's five classes.

use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::export::{
    Allowlist, EntropyWarning, ExportWarning, own_repo, redact_string, redact_value,
    repo_names_to_redact, verify_export_line,
};
use crate::normalize::{Event, Run};
use crate::paths::Layout;
use crate::policy::PolicySet;
use crate::rungs::evolve::{blocked_branch_names, branch_exists, offerable_branch_names};
use crate::rungs::review;
use crate::worktree;

/// Protected config the human writes. Build ticket 65.
pub fn signoff_path() -> PathBuf {
    crate::paths::hydra_home().join("config/export-signoff.json")
}

/// One human confirmation before a judge-derived pair may leave.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct SignoffEntry {
    pub pass_id: String,
    pub judge_run_id: String,
    pub finding_item: String,
}

/// The sign-off manifest and its sha.
pub struct SignoffManifest {
    pub entries: Vec<SignoffEntry>,
    pub sha: String,
}

impl SignoffManifest {
    pub fn load(path: &Path) -> Result<Self> {
        if !path.is_file() {
            return Ok(Self {
                entries: Vec::new(),
                sha: sha_of(""),
            });
        }
        let text =
            std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        let entries: Vec<SignoffEntry> =
            serde_json::from_str(&text).with_context(|| format!("parsing {}", path.display()))?;
        Ok(Self {
            entries,
            sha: sha_of(&text),
        })
    }

    pub fn contains(&self, pass_id: &str, judge_run_id: &str, finding_item: &str) -> bool {
        self.entries.iter().any(|entry| {
            entry.pass_id == pass_id
                && entry.judge_run_id == judge_run_id
                && entry.finding_item == finding_item
        })
    }
}

fn sha_of(text: &str) -> String {
    use sha2::{Digest, Sha256};
    format!("{:x}", Sha256::digest(text.as_bytes()))
}

/// One TRL explicit-prompt preference line, contract 16.1 and 16.3.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreferencePair {
    pub prompt: Vec<Value>,
    pub rejected: Vec<Value>,
    pub chosen: Vec<Value>,
    pub source: &'static str,
}

impl PreferencePair {
    /// The verified line and the entropy warnings the verifier raised on it.
    pub fn to_line(&self) -> Result<(String, Vec<EntropyWarning>)> {
        let line = serde_json::to_string(&json!({
            "prompt": self.prompt,
            "rejected": self.rejected,
            "chosen": self.chosen,
            "source": self.source,
        }))?;
        let warnings = verify_export_line(&line)?;
        Ok((line, warnings))
    }
}

/// Why a judge-derived pair did not export.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JudgeDisposition {
    Exported(PreferencePair),
    Withheld {
        pass_id: String,
    },
    Refused {
        pass_id: String,
    },
    /// The pass branch or the proposer's `base_tree` cannot be read. The rest
    /// of the export continues; `--dry-run` prints `pass_unreadable`.
    Unreadable {
        pass_id: String,
    },
}

impl JudgeDisposition {
    /// The reason `--dry-run` prints for a pass that did not export, if any.
    pub fn dry_run_reason(&self) -> Option<(&'static str, &str)> {
        match self {
            Self::Unreadable { pass_id } => Some(("pass_unreadable", pass_id)),
            Self::Exported(_) | Self::Withheld { .. } | Self::Refused { .. } => None,
        }
    }
}

/// Corrections from one eligible run.
pub fn correction_pairs(run: &Run, repo_names: &[String]) -> Vec<PreferencePair> {
    let mut pairs = Vec::new();
    for (index, event) in run.events.iter().enumerate() {
        if !matches!(event, Event::Correction(_)) {
            continue;
        }
        let Event::Correction(correction) = event else {
            continue;
        };
        let prompt = prompt_before(&run.events[..index], repo_names);
        pairs.push(PreferencePair {
            prompt,
            rejected: vec![json!({
                "role": "assistant",
                "content": redact_value(&correction.rejected, repo_names),
            })],
            chosen: vec![json!({
                "role": "assistant",
                "content": redact_value(&correction.chosen, repo_names),
            })],
            source: "correction",
        });
    }
    pairs
}

/// Walks evolve passes and reads judge-derived pairs.
pub fn read_judge_pairs(
    layout: &Layout,
    policies: &PolicySet,
    repo: &Path,
    signoff: &SignoffManifest,
) -> Result<Vec<JudgeDisposition>> {
    let corpus = crate::export::read_corpus(layout, policies)?;
    read_judge_pairs_from_runs(&corpus.runs, repo, signoff)
}

fn read_judge_pairs_from_runs(
    runs: &BTreeMap<String, Run>,
    repo: &Path,
    signoff: &SignoffManifest,
) -> Result<Vec<JudgeDisposition>> {
    let mut passes = BTreeMap::new();
    let mut children: BTreeMap<String, Vec<(String, &Run)>> = BTreeMap::new();
    for run in runs.values() {
        if is_evolve_pass(run) {
            passes.insert(run.run_id.clone(), run);
            continue;
        }
        if let Some(parent) = &run.parent_run_id {
            children
                .entry(parent.clone())
                .or_default()
                .push((run.run_id.clone(), run));
        }
    }

    let mut out = Vec::new();
    for (pass_id, _pass) in passes {
        let kids = children.remove(&pass_id).unwrap_or_default();
        let Some(proposer) = kids
            .iter()
            .find(|(_, run)| run.policy.as_deref() == Some("standard"))
        else {
            continue;
        };
        let blocked = match blocked_branch(repo, &pass_id) {
            Ok(branch) => branch,
            Err(_) => continue,
        };
        let (rejected, chosen) = match readable_halves(repo, proposer.1, &blocked, &pass_id) {
            Ok(halves) => halves,
            Err(_) => {
                out.push(JudgeDisposition::Unreadable { pass_id });
                continue;
            }
        };
        if rejected.trim().is_empty() {
            continue;
        }
        let Some(chosen) = chosen else {
            out.push(JudgeDisposition::Withheld { pass_id });
            continue;
        };
        let judges: Vec<_> = kids
            .iter()
            .filter(|(_, run)| run.policy.as_deref() == Some("judge"))
            .collect();
        if judges.is_empty() {
            out.push(JudgeDisposition::Withheld { pass_id });
            continue;
        }
        let signed = judges.iter().any(|(judge_id, judge)| {
            let text = crate::outcome::final_assistant_text(judge).unwrap_or_default();
            let provider = judge_provider(judge.provider.as_deref());
            let outcome = review::parse_judge_answer(provider, &text);
            outcome
                .findings
                .iter()
                .any(|finding| signoff.contains(pass_id.as_str(), judge_id, &finding.item))
        });
        if !signed {
            out.push(JudgeDisposition::Refused { pass_id });
            continue;
        }
        let prompt = proposer_prompt(proposer.1);
        out.push(JudgeDisposition::Exported(PreferencePair {
            prompt,
            rejected: diff_message(&rejected),
            chosen: diff_message(&chosen),
            source: "judge_verdict",
        }));
    }
    Ok(out)
}

/// Every pair as a verified line, in order. Fails on the first secret, and
/// the caller runs it before any file exists.
pub fn render_lines(pairs: &[PreferencePair]) -> Result<Vec<String>> {
    Ok(render_lines_with_warnings(pairs)?.0)
}

/// [`render_lines`], plus the entropy warnings. A pair has no run id, so each
/// warning names the pair by its 1-based place in `preference.jsonl` and its
/// source.
pub fn render_lines_with_warnings(
    pairs: &[PreferencePair],
) -> Result<(Vec<String>, Vec<ExportWarning>)> {
    let mut lines = Vec::new();
    let mut warnings = Vec::new();
    for (index, pair) in pairs.iter().enumerate() {
        let (line, found) = pair.to_line()?;
        for warning in found {
            warnings.push(ExportWarning {
                source: format!("preference pair {} ({})", index + 1, pair.source),
                warning,
            });
        }
        lines.push(line);
    }
    Ok((lines, warnings))
}

/// Writes every pair to `preference.jsonl` beside `events_out`.
pub fn write_preference_jsonl(
    events_out: &Path,
    pairs: &[PreferencePair],
    signoff_sha: &str,
) -> Result<()> {
    let lines = render_lines(pairs)?;
    write_preference_lines(events_out, &lines, signoff_sha)
}

/// Writes lines [`render_lines`] already verified to `preference.jsonl` beside
/// `events_out`.
pub fn write_preference_lines(
    events_out: &Path,
    lines: &[String],
    signoff_sha: &str,
) -> Result<()> {
    let path = preference_path(events_out);
    let header = format!(
        "preference count={} signoff_sha={} residue=customer_data_in_fixture",
        lines.len(),
        signoff_sha
    );
    let mut file =
        std::fs::File::create(&path).with_context(|| format!("creating {}", path.display()))?;
    writeln!(file, "{header}")?;
    for line in lines {
        writeln!(file, "{line}")?;
    }
    Ok(())
}

pub fn preference_path(events_out: &Path) -> PathBuf {
    events_out
        .parent()
        .map(|dir| dir.join("preference.jsonl"))
        .unwrap_or_else(|| PathBuf::from("preference.jsonl"))
}

/// Collects correction pairs from eligible runs and signed-off judge pairs.
pub fn collect_pairs(
    layout: &Layout,
    policies: &PolicySet,
    allowlist: &Allowlist,
    eligible_run_ids: &[String],
    signoff: &SignoffManifest,
) -> Result<(Vec<PreferencePair>, Vec<JudgeDisposition>)> {
    let corpus = crate::export::read_corpus(layout, policies)?;
    collect_pairs_from_runs(&corpus.runs, allowlist, eligible_run_ids, signoff)
}

pub(crate) fn collect_pairs_from_runs(
    runs: &BTreeMap<String, Run>,
    allowlist: &Allowlist,
    eligible_run_ids: &[String],
    signoff: &SignoffManifest,
) -> Result<(Vec<PreferencePair>, Vec<JudgeDisposition>)> {
    let mut pairs = Vec::new();
    for run_id in eligible_run_ids {
        let run = runs
            .get(run_id)
            .with_context(|| format!("missing eligible run {run_id}"))?;
        let own_repo = own_repo(run, &allowlist.repos);
        let names = repo_names_to_redact(&allowlist.repos, own_repo.as_deref());
        pairs.extend(correction_pairs(run, &names));
    }

    let mut judge_dispositions = Vec::new();
    for repo in &allowlist.repos {
        judge_dispositions.extend(read_judge_pairs_from_runs(runs, repo, signoff)?);
    }
    for disposition in &judge_dispositions {
        if let JudgeDisposition::Exported(pair) = disposition {
            pairs.push(pair.clone());
        }
    }
    Ok((pairs, judge_dispositions))
}

fn judge_provider(provider: Option<&str>) -> &'static str {
    match provider {
        Some("claude") => "claude",
        Some("cursor-agent") => "cursor-agent",
        Some("agy") => "agy",
        Some("codex") => "codex",
        _ => "unknown",
    }
}

fn is_evolve_pass(run: &Run) -> bool {
    run.provider.is_none()
        && run
            .run_start()
            .and_then(|start| start.body["command"].as_str())
            == Some("evolve")
}

fn readable_halves(
    repo: &Path,
    proposer: &Run,
    blocked: &str,
    pass_id: &str,
) -> Result<(String, Option<String>)> {
    let base_tree = base_tree_of(proposer)?;
    let rejected = branch_diff(repo, blocked, &base_tree)?;
    let chosen = chosen_half(repo, pass_id, &base_tree)?;
    Ok((rejected, chosen))
}

fn blocked_branch(repo: &Path, pass_id: &str) -> Result<String> {
    blocked_branch_names(pass_id)
        .into_iter()
        .find(|branch| branch_exists(repo, branch))
        .ok_or_else(|| anyhow::anyhow!("no blocked branch for pass {pass_id}"))
}

fn offerable_branch(repo: &Path, pass_id: &str) -> Option<String> {
    offerable_branch_names(pass_id)
        .into_iter()
        .find(|branch| branch_exists(repo, branch))
}

fn branch_diff(repo: &Path, branch: &str, base: &str) -> Result<String> {
    worktree::git(repo, &["diff", base, branch])
}

fn chosen_half(repo: &Path, pass_id: &str, base: &str) -> Result<Option<String>> {
    if let Some(offer) = offerable_branch(repo, pass_id) {
        let diff = branch_diff(repo, &offer, base)?;
        if !diff.trim().is_empty() {
            return Ok(Some(diff));
        }
    }
    for main in ["master", "main"] {
        if !branch_exists(repo, main) {
            continue;
        }
        let log = worktree::git(repo, &["log", main, "--grep", pass_id, "-1", "--format=%H"])?;
        let commit = log.trim();
        if commit.is_empty() {
            continue;
        }
        let parent = format!("{commit}^");
        let diff = worktree::git(repo, &["diff", &parent, commit])?;
        if !diff.trim().is_empty() {
            return Ok(Some(diff));
        }
    }
    Ok(None)
}

fn base_tree_of(run: &Run) -> Result<String> {
    run.run_start()
        .and_then(|start| start.body["worktree"]["base_tree"].as_str())
        .map(str::to_string)
        .context("proposer record has no base_tree")
}

fn proposer_prompt(run: &Run) -> Vec<Value> {
    let text = run
        .run_start()
        .and_then(|start| start.body["task"].as_str())
        .unwrap_or("");
    vec![json!({"role": "user", "content": text})]
}

fn diff_message(diff: &str) -> Vec<Value> {
    vec![json!({"role": "assistant", "content": diff})]
}

fn prompt_before(events: &[Event], repo_names: &[String]) -> Vec<Value> {
    events
        .iter()
        .filter_map(|event| prompt_event(event, repo_names))
        .collect()
}

fn prompt_event(event: &Event, repo_names: &[String]) -> Option<Value> {
    match event {
        Event::Message(message) => Some(json!({
            "role": message.role.as_str(),
            "content": redact_string(&message.text, repo_names),
        })),
        Event::Thinking(thinking) => Some(json!({
            "role": "assistant",
            "content": redact_string(&thinking.text, repo_names),
        })),
        Event::ToolCall(call) => Some(json!({
            "role": "assistant",
            "tool_calls": [{
                "id": redact_string(&call.call_id, repo_names),
                "type": "function",
                "function": {
                    "name": redact_string(&call.name, repo_names),
                    "arguments": redact_value(&call.arguments, repo_names),
                }
            }],
        })),
        Event::ToolResult(result) => Some(json!({
            "role": "tool",
            "tool_call_id": redact_string(&result.call_id, repo_names),
            "content": redact_string(&result.output, repo_names),
        })),
        Event::RunStart(_)
        | Event::GateRefusal(_)
        | Event::RunEnd(_)
        | Event::MessageDelta(_)
        | Event::PermissionDecision(_)
        | Event::Usage(_)
        | Event::Correction(_) => None,
    }
}
