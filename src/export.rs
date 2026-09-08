//! `hydra export`, harness spec 14.
//!
//! The one path by which anything leaves this machine: default deny, allowlisted
//! by path, redacted in four fixed classes, verified before anything is written.
//! Typed by a human and reachable from no run.

use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::live::State;
use crate::normalize::paths::{PathExtraction, cwd_of, inside, paths_touched};
use crate::normalize::{Event, ResultStatus, Role};
use crate::outcome::{Source, derive};
use crate::paths::Layout;
use crate::policy::{Capability, PolicySet};
use crate::record::secrets;

/// Protected config #3, harness spec 15.1.
pub fn allowlist_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("config/export-allowlist.json")
}

/// The export allowlist and its sha.
pub struct Allowlist {
    pub repos: Vec<PathBuf>,
    pub sha: String,
}

impl Allowlist {
    pub fn load(path: &Path) -> Result<Self> {
        if !path.is_file() {
            bail!("Write the export allowlist at config/export-allowlist.json before exporting.");
        }
        let text =
            std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        let sha = format!("{:x}", Sha256::digest(text.as_bytes()));
        let repos: Vec<PathBuf> =
            serde_json::from_str(&text).with_context(|| format!("parsing {}", path.display()))?;
        for repo in &repos {
            if !repo.is_absolute() {
                bail!(
                    "every entry in {} must be an absolute path; got {}",
                    path.display(),
                    repo.display()
                );
            }
        }
        let repos: Vec<PathBuf> = repos
            .into_iter()
            .map(|repo| repo.canonicalize().unwrap_or(repo))
            .collect();
        Ok(Self { repos, sha })
    }
}

/// One named reason a run is not exportable.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Reason {
    HarnessRun,
    PathOutsideAllowlist,
    UnclassifiedPath,
    UnknownOutcome,
    Attachment,
    SecretFlag,
    NoRunEnd,
    AuthFailed,
    CuratorsChild,
    ChildIneligible,
    StillRunning,
}

impl Reason {
    pub fn as_str(&self) -> &'static str {
        match self {
            Reason::HarnessRun => "harness_run",
            Reason::PathOutsideAllowlist => "path_outside_allowlist",
            Reason::UnclassifiedPath => "unclassified_path",
            Reason::UnknownOutcome => "unknown_outcome",
            Reason::Attachment => "attachment",
            Reason::SecretFlag => "secret_flag",
            Reason::NoRunEnd => "no_run_end",
            Reason::AuthFailed => "auth_failed",
            Reason::CuratorsChild => "curators_child",
            Reason::ChildIneligible => "child_ineligible",
            Reason::StillRunning => "still_running",
        }
    }
}

/// One run's eligibility answer.
pub struct Verdict {
    pub run_id: String,
    pub reasons: Vec<Reason>,
}

impl Verdict {
    pub fn eligible(&self) -> bool {
        self.reasons.is_empty()
    }
}

/// Walks the corpus and answers export eligibility for every run.
pub fn audit(layout: &Layout, allowlist: &Allowlist, policies: &PolicySet) -> Result<Vec<Verdict>> {
    let mut curator_passes: Vec<String> = Vec::new();
    let mut runs: BTreeMap<String, (String, crate::normalize::Run)> = BTreeMap::new();
    let mut live: BTreeSet<String> = BTreeSet::new();

    for view in crate::live::scan(layout)? {
        if view.state == State::Live {
            live.insert(view.run_id.clone());
            continue;
        }
        let text = std::fs::read_to_string(&view.run_file)
            .with_context(|| format!("reading {}", view.run_file.display()))?;
        let run = crate::normalize::read_with(&text, policies, layout)
            .with_context(|| format!("normalizing {}", view.run_id))?;
        if run.policy.as_deref() == Some("curate") {
            curator_passes.push(run.run_id.clone());
        }
        runs.insert(run.run_id.clone(), (text, run));
    }

    let children: BTreeMap<String, Vec<String>> = runs
        .values()
        .map(|(_, run)| run)
        .filter_map(|run| {
            run.parent_run_id
                .as_ref()
                .map(|parent| (parent.clone(), run.run_id.clone()))
        })
        .fold(BTreeMap::new(), |mut map, (parent, child)| {
            map.entry(parent).or_default().push(child);
            map
        });

    let mut verdicts = Vec::new();
    for run_id in live {
        verdicts.push(Verdict {
            run_id,
            reasons: vec![Reason::StillRunning],
        });
    }

    for (run_id, (text, run)) in &runs {
        let reasons = reasons_for(run, text, allowlist, &curator_passes, &children, &runs);
        verdicts.push(Verdict {
            run_id: run_id.clone(),
            reasons,
        });
    }

    verdicts.sort_by(|a, b| a.run_id.cmp(&b.run_id));
    Ok(verdicts)
}

fn reasons_for(
    run: &crate::normalize::Run,
    text: &str,
    allowlist: &Allowlist,
    curator_passes: &[String],
    children: &BTreeMap<String, Vec<String>>,
    runs: &BTreeMap<String, (String, crate::normalize::Run)>,
) -> Vec<Reason> {
    let mut reasons = Vec::new();

    if run.provider.is_none() {
        reasons.push(Reason::HarnessRun);
        return reasons;
    }

    if run.run_end().is_none() {
        reasons.push(Reason::NoRunEnd);
    }

    if has_attachment(text) {
        reasons.push(Reason::Attachment);
    }

    if has_secret_flag(text) {
        reasons.push(Reason::SecretFlag);
    }

    if let Some(parent) = &run.parent_run_id
        && curator_passes.iter().any(|pass| pass == parent)
    {
        reasons.push(Reason::CuratorsChild);
    }

    if let Some(derived) = derive(run) {
        // Contract 16.2: three exclusions key on outcome, and this is the only
        // one that is an outcome. Everything else exports with its outcome
        // attached, retry loops and gated runs included; the exporter judges
        // validity and never quality.
        if derived.outcome == crate::outcome::Outcome::AuthFailed {
            reasons.push(Reason::AuthFailed);
        }
        if derived.source == Source::Unknown {
            reasons.push(Reason::UnknownOutcome);
        }
    }

    match paths_touched(run) {
        PathExtraction::Unclassified => reasons.push(Reason::UnclassifiedPath),
        PathExtraction::Paths(paths) => {
            for path in paths {
                if !allowlist.repos.iter().any(|repo| inside(&path, repo)) {
                    reasons.push(Reason::PathOutsideAllowlist);
                    break;
                }
            }
        }
    }

    if let Some(kids) = children.get(&run.run_id) {
        for child_id in kids {
            let (_, child) = &runs[child_id];
            let child_reasons = reasons_for(
                child,
                &runs[child_id].0,
                allowlist,
                curator_passes,
                children,
                runs,
            );
            if !child_reasons.is_empty() {
                reasons.push(Reason::ChildIneligible);
                break;
            }
        }
    }

    reasons.sort();
    reasons.dedup();
    reasons
}

fn has_attachment(text: &str) -> bool {
    for line in text.lines() {
        let Ok(envelope) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        let raw = envelope.get("raw").unwrap_or(&envelope);
        if raw.get("type").and_then(Value::as_str) == Some("attachment") {
            return true;
        }
    }
    false
}

fn has_secret_flag(text: &str) -> bool {
    for line in text.lines() {
        let Ok(envelope) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        if !envelope["secret_flag"].is_null() {
            return true;
        }
    }
    false
}

/// Exports eligible runs to `out`, or prints eligibility under `--dry-run`.
pub fn export(
    layout: &Layout,
    allowlist_path: &Path,
    out: Option<&Path>,
    dry_run: bool,
) -> Result<()> {
    let allowlist = Allowlist::load(allowlist_path)?;
    let policies = PolicySet::discover()?;
    let verdicts = audit(layout, &allowlist, &policies)?;

    if dry_run {
        let mut stdout = std::io::stdout().lock();
        for verdict in &verdicts {
            if verdict.eligible() {
                writeln!(stdout, "{}: eligible", verdict.run_id)?;
            } else {
                let names = verdict
                    .reasons
                    .iter()
                    .map(Reason::as_str)
                    .collect::<Vec<_>>()
                    .join(", ");
                writeln!(stdout, "{}: {}", verdict.run_id, names)?;
            }
        }
        return Ok(());
    }

    let out = out.context("--out is required unless --dry-run is set")?;
    let eligible: Vec<&Verdict> = verdicts.iter().filter(|v| v.eligible()).collect();

    let mut lines: Vec<String> = Vec::new();
    for verdict in eligible {
        let path = layout.run_file(&verdict.run_id);
        let run = crate::normalize::read_file(&path, &policies)?;
        let own_repo = own_repo(&run, &allowlist.repos);
        let events = export_events(&run, &allowlist.repos, own_repo.as_deref());
        let line = serde_json::to_string(&json!({ "events": events }))?;
        verify_export_line(&line)?;
        lines.push(line);
    }

    let header = format!(
        "export count={} allowlist_sha={} residue=customer_data_in_fixture",
        lines.len(),
        allowlist.sha
    );

    let mut file =
        std::fs::File::create(out).with_context(|| format!("creating {}", out.display()))?;
    writeln!(file, "{header}")?;
    for line in lines {
        writeln!(file, "{line}")?;
    }
    Ok(())
}

fn own_repo(run: &crate::normalize::Run, repos: &[PathBuf]) -> Option<PathBuf> {
    let PathExtraction::Paths(paths) = paths_touched(run) else {
        return None;
    };
    if paths.is_empty() {
        let cwd = cwd_of(run);
        return repos.iter().find(|repo| inside(&cwd, repo)).cloned();
    }
    repos
        .iter()
        .find(|repo| paths.iter().all(|path| inside(path, repo)))
        .cloned()
}

fn export_events(
    run: &crate::normalize::Run,
    repos: &[PathBuf],
    own_repo: Option<&Path>,
) -> Vec<Value> {
    let names_to_redact = repo_names_to_redact(repos, own_repo);
    run.events
        .iter()
        .filter_map(|event| export_event(event, &names_to_redact))
        .collect()
}

fn repo_names_to_redact(repos: &[PathBuf], own_repo: Option<&Path>) -> Vec<String> {
    repos
        .iter()
        .filter(|repo| own_repo != Some(repo.as_path()))
        .filter_map(|repo| {
            repo.file_name()
                .map(|name| name.to_string_lossy().to_string())
        })
        .collect()
}

fn export_event(event: &Event, repo_names: &[String]) -> Option<Value> {
    match event {
        Event::RunStart(_) | Event::RunEnd(_) | Event::PermissionDecision(_) | Event::Usage(_) => {
            None
        }
        Event::MessageDelta(_) => None,
        other => Some(match other {
            Event::Message(e) => json!({
                "seq": e.seq,
                "type": "message",
                "role": role_str(e.role),
                "text": redact_string(&e.text, repo_names),
                "block": e.block,
            }),
            Event::Thinking(e) => json!({
                "seq": e.seq,
                "type": "thinking",
                "text": redact_string(&e.text, repo_names),
                "signed": e.signature.is_some(),
                "block": e.block,
            }),
            Event::ToolCall(e) => json!({
                "seq": e.seq,
                "type": "tool_call",
                "call_id": redact_string(&e.call_id, repo_names),
                "name": redact_string(&e.name, repo_names),
                "arguments": redact_value(&e.arguments, repo_names),
                "capability": e.capability.map(Capability::as_str),
            }),
            Event::ToolResult(e) => json!({
                "seq": e.seq,
                "type": "tool_result",
                "call_id": redact_string(&e.call_id, repo_names),
                "status": status_str(e.status),
                "output": redact_string(&e.output, repo_names),
                "truncated": e.truncated,
            }),
            Event::Correction(e) => json!({
                "type": "correction",
                "corrects": e.corrects,
                "rejected": redact_value(&e.rejected, repo_names),
                "chosen": redact_value(&e.chosen, repo_names),
                "words": e.words.as_ref().map(|w| redact_string(w, repo_names)),
            }),
            Event::RunStart(_)
            | Event::RunEnd(_)
            | Event::MessageDelta(_)
            | Event::PermissionDecision(_)
            | Event::Usage(_) => return None,
        }),
    }
}

fn redact_value(value: &Value, repo_names: &[String]) -> Value {
    match value {
        Value::String(text) => Value::String(redact_string(text, repo_names)),
        Value::Array(items) => Value::Array(
            items
                .iter()
                .map(|item| redact_value(item, repo_names))
                .collect(),
        ),
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(key, val)| (key.clone(), redact_value(val, repo_names)))
                .collect(),
        ),
        other => other.clone(),
    }
}

fn redact_string(text: &str, repo_names: &[String]) -> String {
    let mut out = text.to_string();
    for name in repo_names {
        if !name.is_empty() {
            out = out.replace(name, "[REDACTED:repo_name]");
        }
    }
    out = redact_secrets(&out);
    out
}

fn redact_secrets(text: &str) -> String {
    let mut out = text.to_string();
    for (prefix, label) in [
        ("AIza", "google_api_key"),
        ("sk-ant-", "anthropic_api_key"),
        ("ghp_", "github_pat"),
        ("ya29.", "google_oauth_token"),
        ("-----BEGIN", "pem_private_key"),
    ] {
        while let Some(at) = out.find(prefix) {
            let end = out[at..]
                .find(|c: char| c.is_whitespace() || c == '"' || c == '\'')
                .map(|idx| at + idx)
                .unwrap_or(out.len());
            out.replace_range(at..end, &format!("[REDACTED:{label}]"));
        }
    }
    out
}

fn role_str(role: Role) -> &'static str {
    match role {
        Role::Assistant => "assistant",
        Role::User => "user",
    }
}

fn status_str(status: ResultStatus) -> &'static str {
    match status {
        ResultStatus::Ok => "ok",
        ResultStatus::Error => "error",
        ResultStatus::Unclassified => "unclassified",
    }
}

/// Harness spec 14's second prefix sweep. A hit means the redactor is broken.
pub fn verify_export_line(line: &str) -> Result<()> {
    if secrets::scan(line).is_some() {
        bail!(
            "verification scan found a secret after redaction; export aborted with nothing written"
        );
    }
    Ok(())
}

/// Prints the refusal when the allowlist is missing.
pub fn refuse_missing_allowlist() -> String {
    "Write the export allowlist at config/export-allowlist.json before exporting.".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redaction_strips_a_planted_prefix() {
        let text = "here is sk-ant-secret-key-value";
        let redacted = redact_string(text, &[]);
        assert!(!redacted.contains("sk-ant-"));
        assert!(redacted.contains("[REDACTED:anthropic_api_key]"));
    }
}
