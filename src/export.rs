//! `hydra export`, harness spec 14.
//!
//! The one path by which anything leaves this machine: default deny, allowlisted
//! by path, redacted four ways across twelve secret classes, verified before
//! anything is written. Typed by a human and reachable from no run.

use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::live::State;
use crate::normalize::paths::{PathExtraction, cwd_of, inside, paths_touched};
use crate::normalize::{Event, Run};
use crate::outcome::{Source, derive};
use crate::paths::Layout;
use crate::policy::{Capability, PolicySet};

/// Protected config #3, harness spec 15.1.
pub fn allowlist_path() -> PathBuf {
    crate::paths::hydra_home().join("config/export-allowlist.json")
}

/// The export allowlist and its sha.
pub struct Allowlist {
    pub repos: Vec<PathBuf>,
    pub sha: String,
}

impl Allowlist {
    pub fn load(path: &Path) -> Result<Self> {
        if !path.is_file() {
            bail!(
                "Write the export allowlist at config/export-allowlist.json before exporting. It is a JSON array of absolute repository paths, one per repo whose work may leave this machine; see config/export-allowlist.json.example."
            );
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
    let corpus = read_corpus(layout, policies)?;
    Ok(audit_runs(
        &corpus.runs,
        &corpus.texts,
        &corpus.live,
        allowlist,
    ))
}

pub(crate) struct Corpus {
    pub runs: BTreeMap<String, Run>,
    texts: BTreeMap<String, String>,
    live: BTreeSet<String>,
}

pub(crate) fn read_corpus(layout: &Layout, policies: &PolicySet) -> Result<Corpus> {
    let mut runs = BTreeMap::new();
    let mut texts = BTreeMap::new();
    let mut live = BTreeSet::new();
    for view in crate::live::scan(layout)? {
        if view.state == State::Live {
            live.insert(view.run_id.clone());
            continue;
        }
        let text = std::fs::read_to_string(&view.run_file)
            .with_context(|| format!("reading {}", view.run_file.display()))?;
        let run = crate::normalize::read_with(&text, policies, layout)
            .with_context(|| format!("normalizing {}", view.run_id))?;
        texts.insert(run.run_id.clone(), text);
        runs.insert(run.run_id.clone(), run);
    }
    Ok(Corpus { runs, texts, live })
}

fn audit_runs(
    runs: &BTreeMap<String, Run>,
    texts: &BTreeMap<String, String>,
    live: &BTreeSet<String>,
    allowlist: &Allowlist,
) -> Vec<Verdict> {
    let curator_passes: Vec<String> = runs
        .values()
        .filter(|run| run.policy.as_deref() == Some("curate"))
        .map(|run| run.run_id.clone())
        .collect();
    let children: BTreeMap<String, Vec<String>> = runs
        .values()
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
            run_id: run_id.clone(),
            reasons: vec![Reason::StillRunning],
        });
    }

    for (run_id, run) in runs {
        let reasons = reasons_for(run, texts, allowlist, &curator_passes, &children, runs);
        verdicts.push(Verdict {
            run_id: run_id.clone(),
            reasons,
        });
    }

    verdicts.sort_by(|a, b| a.run_id.cmp(&b.run_id));
    verdicts
}

fn reasons_for(
    run: &crate::normalize::Run,
    texts: &BTreeMap<String, String>,
    allowlist: &Allowlist,
    curator_passes: &[String],
    children: &BTreeMap<String, Vec<String>>,
    runs: &BTreeMap<String, Run>,
) -> Vec<Reason> {
    let mut reasons = Vec::new();

    if run.provider.is_none() {
        reasons.push(Reason::HarnessRun);
        return reasons;
    }

    if run.run_end().is_none() {
        reasons.push(Reason::NoRunEnd);
    }

    if has_attachment(&texts[&run.run_id]) {
        reasons.push(Reason::Attachment);
    }

    if has_secret_flag(&texts[&run.run_id]) {
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
            let child = &runs[child_id];
            let child_reasons =
                reasons_for(child, texts, allowlist, curator_passes, children, runs);
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
/// After a real export it prints the entropy warnings, one line per token.
pub fn export(
    layout: &Layout,
    allowlist_path: &Path,
    out: Option<&Path>,
    dry_run: bool,
) -> Result<()> {
    let warnings = export_with_signoff(layout, allowlist_path, out, dry_run, None)?;
    if !dry_run {
        print!("{}", warning_summary(&warnings));
    }
    Ok(())
}

/// The export itself. Returns the entropy warnings, which a dry run never has.
pub fn export_with_signoff(
    layout: &Layout,
    allowlist_path: &Path,
    out: Option<&Path>,
    dry_run: bool,
    signoff_path: Option<&Path>,
) -> Result<Vec<ExportWarning>> {
    let allowlist = Allowlist::load(allowlist_path)?;
    let policies = PolicySet::discover()?;
    let corpus = read_corpus(layout, &policies)?;
    let runs = &corpus.runs;
    let verdicts = audit_runs(runs, &corpus.texts, &corpus.live, &allowlist);

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
        let signoff = crate::export_preference::SignoffManifest::load(
            signoff_path.unwrap_or(&crate::export_preference::signoff_path()),
        )?;
        let eligible_ids: Vec<String> = verdicts
            .iter()
            .filter(|v| v.eligible())
            .map(|v| v.run_id.clone())
            .collect();
        let (_, dispositions) = crate::export_preference::collect_pairs_from_runs(
            runs,
            &allowlist,
            &eligible_ids,
            &signoff,
        )?;
        for disposition in dispositions {
            if let Some((reason, pass_id)) = disposition.dry_run_reason() {
                writeln!(stdout, "{pass_id}: {reason}")?;
            }
        }
        return Ok(Vec::new());
    }

    let out = out.context("--out is required unless --dry-run is set")?;
    // The preference file is always `preference.jsonl` beside `--out`, so an
    // `--out` by that name would be truncated by its own sibling.
    if out.file_name().and_then(|n| n.to_str()) == Some("preference.jsonl") {
        bail!(
            "--out cannot be named preference.jsonl: that is the name of the preference file \
             written beside it, and the second write would truncate the first"
        );
    }
    let eligible: Vec<&Verdict> = verdicts.iter().filter(|v| v.eligible()).collect();
    let signoff = crate::export_preference::SignoffManifest::load(
        signoff_path.unwrap_or(&crate::export_preference::signoff_path()),
    )?;
    let eligible_ids: Vec<String> = eligible.iter().map(|v| v.run_id.clone()).collect();
    let (preference_pairs, _) = crate::export_preference::collect_pairs_from_runs(
        runs,
        &allowlist,
        &eligible_ids,
        &signoff,
    )?;

    let mut lines: Vec<String> = Vec::new();
    let mut warnings: Vec<ExportWarning> = Vec::new();
    for verdict in eligible {
        let run = &runs[&verdict.run_id];
        let own_repo = own_repo(run, &allowlist.repos);
        let events = export_events(run, &allowlist.repos, own_repo.as_deref());
        let line = serde_json::to_string(&json!({ "events": events }))?;
        for warning in verify_export_line(&line)? {
            warnings.push(ExportWarning {
                source: verdict.run_id.clone(),
                warning,
            });
        }
        lines.push(line);
    }
    // Rendered, and so verified, before either file is created: a secret in a
    // preference pair used to surface after the events file was already on
    // disk, which is not "aborts with nothing written".
    let (preference_lines, preference_warnings) =
        crate::export_preference::render_lines_with_warnings(&preference_pairs)?;
    warnings.extend(preference_warnings);

    let header = format!(
        "export count={} allowlist_sha={} residue=customer_data_in_fixture",
        lines.len(),
        allowlist.sha
    );

    let mut file = open_events_out(out)?;
    writeln!(file, "{header}")?;
    for line in lines {
        writeln!(file, "{line}")?;
    }
    crate::export_preference::write_preference_lines(out, &preference_lines, &signoff.sha)?;
    Ok(warnings)
}

/// Opens `--out` for the events, refusing it when it is the preference sibling
/// under another name.
///
/// The name check above catches one spelling. A case-insensitive volume, the
/// default on macOS, makes `Preference.jsonl` the same file, and so does a
/// symbolic or hard link by any name. So the file is opened without
/// truncating, and its device and inode are compared with the sibling's before
/// a byte is written. A file this call created for the comparison is removed
/// on refusal; one that was already there is left as it was.
fn open_events_out(out: &Path) -> Result<std::fs::File> {
    use std::os::unix::fs::MetadataExt;

    let existed = out.exists();
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .open(out)
        .with_context(|| format!("creating {}", out.display()))?;
    let sibling = crate::export_preference::preference_path(out);
    if let Ok(theirs) = std::fs::metadata(&sibling) {
        let ours = file.metadata()?;
        if ours.dev() == theirs.dev() && ours.ino() == theirs.ino() {
            drop(file);
            if !existed {
                let _ = std::fs::remove_file(out);
            }
            bail!(
                "--out {} is the same file as {}, the preference file written beside it, and \
                 the second write would truncate the first",
                out.display(),
                sibling.display()
            );
        }
    }
    file.set_len(0)
        .with_context(|| format!("truncating {}", out.display()))?;
    Ok(file)
}

pub(crate) fn own_repo(run: &crate::normalize::Run, repos: &[PathBuf]) -> Option<PathBuf> {
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
    // Map 113: the task is the turn the trajectory answers, so it leads the
    // line as one `user` message, preamble included, and takes the same
    // redaction as every other message.
    let task = run.run_start().and_then(|start| {
        start.body["task"].as_str().map(|text| {
            Event::Message(crate::normalize::Message {
                seq: start.seq,
                role: crate::normalize::Role::User,
                text: text.to_string(),
                block: 0,
            })
        })
    });
    task.iter()
        .chain(&run.events)
        .filter_map(|event| export_event(event, &names_to_redact))
        .collect()
}

pub(crate) fn repo_names_to_redact(repos: &[PathBuf], own_repo: Option<&Path>) -> Vec<String> {
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
        Event::Message(e) => Some(json!({
            "seq": e.seq,
            "type": "message",
            "role": e.role.as_str(),
            "text": redact_string(&e.text, repo_names),
            "block": e.block,
        })),
        Event::Thinking(e) => Some(json!({
            "seq": e.seq,
            "type": "thinking",
            "text": redact_string(&e.text, repo_names),
            "signed": e.signature.is_some(),
            "block": e.block,
        })),
        Event::ToolCall(e) => Some(json!({
            "seq": e.seq,
            "type": "tool_call",
            "call_id": redact_string(&e.call_id, repo_names),
            "name": redact_string(&e.name, repo_names),
            "arguments": redact_value(&e.arguments, repo_names),
            "capability": e.capability.map(Capability::as_str),
        })),
        Event::ToolResult(e) => Some(json!({
            "seq": e.seq,
            "type": "tool_result",
            "call_id": redact_string(&e.call_id, repo_names),
            "status": e.status.as_str(),
            "output": redact_string(&e.output, repo_names),
            "truncated": e.truncated,
        })),
        Event::Correction(e) => Some(json!({
            "type": "correction",
            "corrects": e.corrects,
            "rejected": redact_value(&e.rejected, repo_names),
            "chosen": redact_value(&e.chosen, repo_names),
            "words": e.words.as_ref().map(|w| redact_string(w, repo_names)),
        })),
        Event::RunStart(_)
        | Event::RunEnd(_)
        | Event::MessageDelta(_)
        | Event::PermissionDecision(_)
        | Event::Usage(_) => None,
    }
}

pub(crate) fn redact_value(value: &Value, repo_names: &[String]) -> Value {
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

pub(crate) fn redact_string(text: &str, repo_names: &[String]) -> String {
    let mut out = text.to_string();
    for name in repo_names {
        if !name.is_empty() {
            out = out.replace(name, "[REDACTED:repo_name]");
        }
    }
    out = redact_secrets(&out);
    out
}

/// An OpenAI key, `sk-proj-`, `sk-svcacct-` or `sk-admin-` and at least this
/// many characters of `[A-Za-z0-9_-]`, or a legacy `sk-` and at least
/// [`OPENAI_LEGACY_BODY`] alphanumerics. Build ticket 93.
const OPENAI_PROJECT_BODY: usize = 20;
const OPENAI_LEGACY_BODY: usize = 32;

/// The byte spans of OpenAI keys in `text`, left to right. Redactor and
/// verifier both call this, so the verifier catches a redactor that skipped a
/// span and nothing the rule itself misses: finding G18 still stands.
fn openai_key_spans(text: &str) -> Vec<(usize, usize)> {
    let bytes = text.as_bytes();
    let token = |b: u8| b.is_ascii_alphanumeric() || b == b'_' || b == b'-';
    let mut spans = Vec::new();
    let mut from = 0;
    while let Some(rel) = text[from..].find("sk-") {
        let at = from + rel;
        from = at + 3;
        if at > 0 && token(bytes[at - 1]) {
            continue;
        }
        let rest = &text[at + 3..];
        let (body_start, min, allowed): (usize, usize, fn(u8) -> bool) =
            match ["proj-", "svcacct-", "admin-"]
                .iter()
                .find(|p| rest.starts_with(**p))
            {
                Some(kind) => (at + 3 + kind.len(), OPENAI_PROJECT_BODY, |b| {
                    b.is_ascii_alphanumeric() || b == b'_' || b == b'-'
                }),
                None => (at + 3, OPENAI_LEGACY_BODY, |b| b.is_ascii_alphanumeric()),
            };
        let len = bytes[body_start..]
            .iter()
            .take_while(|b| allowed(**b))
            .count();
        let end = body_start + len;
        if len >= min && (end == bytes.len() || !token(bytes[end])) {
            spans.push((at, end));
            from = end;
        }
    }
    spans
}

fn redact_secrets(text: &str) -> String {
    let mut out = text.to_string();
    for (prefix, label) in [
        ("AIza", "google_api_key"),
        ("sk-ant-", "anthropic_api_key"),
        ("ghp_", "github_pat"),
        ("github_pat_", "github_fine_grained_pat"),
        ("gho_", "github_oauth_token"),
        ("ya29.", "google_oauth_token"),
        ("AKIA", "aws_access_key"),
        ("aws_secret_access_key=", "aws_secret_access_key"),
        ("AWS_SECRET_ACCESS_KEY=", "aws_secret_access_key"),
        ("xoxb-", "slack_bot_token"),
        ("hooks.slack.com", "slack_webhook"),
        ("-----BEGIN", "pem_private_key"),
    ] {
        while let Some(at) = out.find(prefix) {
            let mut at = at;
            if prefix == "hooks.slack.com" {
                if out[..at].ends_with("https://") {
                    at -= 8;
                } else if out[..at].ends_with("http://") {
                    at -= 7;
                }
            }
            // Every class but one is a single token. The PEM header's value
            // runs past every space to a footer, so the cut is the end of the
            // `-----END ...-----` line when there is one, and the end of the
            // text when there is not. Cutting at the first space left the key
            // body on the line and the verifier, which looks for the header,
            // satisfied.
            let end = if prefix == "-----BEGIN" {
                out[at..]
                    .find("-----END")
                    .and_then(|footer| {
                        let footer = at + footer;
                        out[footer + "-----END".len()..]
                            .find("-----")
                            .map(|close| footer + "-----END".len() + close + "-----".len())
                    })
                    .unwrap_or(out.len())
            } else {
                out[at..]
                    .find(|c: char| c.is_whitespace() || c == '"' || c == '\'')
                    .map(|idx| at + idx)
                    .unwrap_or(out.len())
            };
            out.replace_range(at..end, &format!("[REDACTED:{label}]"));
        }
    }
    // OpenAI keys start with the same `sk-` as English words ending in "sk"
    // ("task-", "risk-", "disk-"), so they get a matcher of their own rather
    // than a prefix: a left boundary, and a body long enough to be a key.
    for (at, end) in openai_key_spans(&out).into_iter().rev() {
        out.replace_range(at..end, "[REDACTED:openai_api_key]");
    }
    out
}

/// A long, random-looking token the verifier found in an exported value. It
/// does not abort the export. It carries where the token sits and what it
/// looks like, and never the token or any part of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntropyWarning {
    /// JSON path to the string, such as `events[3].output`. An object key that
    /// is not a short plain identifier prints as `<key>`, since a key can be
    /// the token itself.
    pub field: String,
    /// `alphanumeric`, `base64`, `base64url` or `mixed`.
    pub class: &'static str,
    pub len: usize,
}

/// One warning and the run or preference pair it came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportWarning {
    pub source: String,
    pub warning: EntropyWarning,
}

/// Verify decoded values independently of the record scanner and redactor.
/// A known prefix or a PEM header aborts. A high-entropy token does not: it
/// comes back as a warning for the summary the export prints.
pub fn verify_export_line(line: &str) -> Result<Vec<EntropyWarning>> {
    let value: Value = serde_json::from_str(line).context("parsing export verification input")?;
    let mut warnings = Vec::new();
    if scan_value(&value, "", &mut warnings) {
        bail!(
            "verification scan found a secret after redaction; export aborted with nothing written"
        );
    }
    Ok(warnings)
}

/// The summary `hydra export` prints once both files are written.
pub fn warning_summary(warnings: &[ExportWarning]) -> String {
    let mut out = format!(
        "entropy warnings: {} (exported anyway; each is a long random-looking token no \
         redaction rule covers, so read it before the export leaves this machine)\n",
        warnings.len()
    );
    for ExportWarning { source, warning } in warnings {
        out.push_str(&format!(
            "{source}: {} {} len={}\n",
            warning.field, warning.class, warning.len
        ));
    }
    out
}

/// True when something in `value` must abort the export. Warnings go to
/// `warnings` either way.
fn scan_value(value: &Value, path: &str, warnings: &mut Vec<EntropyWarning>) -> bool {
    match value {
        Value::String(text) => scan_text(text, path, warnings),
        Value::Array(items) => {
            let mut abort = false;
            for (index, item) in items.iter().enumerate() {
                abort |= scan_value(item, &format!("{path}[{index}]"), warnings);
            }
            abort
        }
        Value::Object(items) => {
            let mut abort = false;
            for (key, item) in items {
                let plain =
                    key.len() <= 32 && key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
                let name = if plain { key.as_str() } else { "<key>" };
                let child = if path.is_empty() {
                    name.to_string()
                } else {
                    format!("{path}.{name}")
                };
                abort |= scan_text(key, &child, warnings);
                abort |= scan_value(item, &child, warnings);
            }
            abort
        }
        _ => false,
    }
}

fn scan_text(text: &str, path: &str, warnings: &mut Vec<EntropyWarning>) -> bool {
    // Header only. The redactor replaces everything from `-----BEGIN` to the
    // end of its footer, or to the end of the text when there is no footer,
    // so a redacted value never holds this string and any hit means the
    // redactor missed a block. Certificates and public keys abort too, as
    // they do in the redactor.
    //
    // The prefixes are the redactor's own, matched anywhere in the text as
    // the redactor matches them, so a hit also means the redactor missed.
    if text.contains("-----BEGIN")
        || [
            "AIza",
            "sk-ant-",
            "ghp_",
            "github_pat_",
            "gho_",
            "ya29.",
            "AKIA",
            "aws_secret_access_key=",
            "AWS_SECRET_ACCESS_KEY=",
            "xoxb-",
            "hooks.slack.com",
        ]
        .iter()
        .any(|prefix| text.contains(prefix))
        || !openai_key_spans(text).is_empty()
    {
        return true;
    }

    // Tokens split on any character outside the token set. A secret broken
    // by a space, or URL-encoded so its `/` and `+` become `%2F` and `%2B`,
    // reaches this check as short pieces and is missed. This is a heuristic
    // that stands behind the redactor, and it only warns.
    for token in mask_routine_values(text).split(|c: char| !is_token_char(c)) {
        if opaque_export_token(token) {
            warnings.push(EntropyWarning {
                field: path.to_string(),
                class: token_class(token),
                len: token.len(),
            });
        }
    }
    false
}

fn token_class(token: &str) -> &'static str {
    let only = |extra: &str| {
        token
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || extra.contains(c))
    };
    if only("") {
        "alphanumeric"
    } else if only("+/=") {
        "base64"
    } else if only("-_=") {
        "base64url"
    } else {
        "mixed"
    }
}

fn is_token_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || "_-/+.=".contains(c)
}

/// Blanks the routine values that look random but are not secrets: UUIDs,
/// which Hydra writes into every worktree path, and the base64 payload of a
/// data URI. Each becomes a space, so a path around a UUID splits into two
/// ordinary tokens.
fn mask_routine_values(text: &str) -> String {
    const DATA_URI: &str = ";base64,";
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    let mut at = 0;
    while at < bytes.len() {
        if is_uuid(&bytes[at..]) {
            out.push(' ');
            at += 36;
        } else if bytes[at..].starts_with(DATA_URI.as_bytes()) {
            out.push(' ');
            at += DATA_URI.len();
            while at < bytes.len()
                && (bytes[at].is_ascii_alphanumeric() || b"+/=".contains(&bytes[at]))
            {
                at += 1;
            }
        } else {
            // Every skip above ends on an ASCII byte, so `at` is always a
            // char boundary here.
            let c = text[at..].chars().next().unwrap_or(' ');
            out.push(c);
            at += c.len_utf8();
        }
    }
    out
}

/// True when `bytes` starts with an 8-4-4-4-12 hex UUID.
fn is_uuid(bytes: &[u8]) -> bool {
    bytes.len() >= 36
        && bytes[..36].iter().enumerate().all(|(i, &b)| match i {
            8 | 13 | 18 | 23 => b == b'-',
            _ => b.is_ascii_hexdigit(),
        })
}

fn opaque_export_token(token: &str) -> bool {
    // Subresource integrity strings in lockfiles are public digests.
    if ["sha256-", "sha384-", "sha512-"]
        .iter()
        .any(|prefix| token.starts_with(prefix))
    {
        return false;
    }
    if token.len() < 32 {
        return false;
    }
    // Paths, URLs and joined identifiers clear the entropy bar at 32 to 44
    // bytes, because their pieces are words. A token must also hold 24
    // characters in a row with no `/`, `.`, `_` or `-` between them. This
    // costs a little on base64url secrets, which use `_` and `-`.
    let longest_run = token
        .split(|c: char| "/._-".contains(c))
        .map(str::len)
        .max()
        .unwrap_or(0);
    if longest_run < 24 {
        return false;
    }
    let mut counts = [0usize; 128];
    for byte in token.bytes() {
        counts[usize::from(byte)] += 1;
    }
    let entropy: f64 = counts
        .iter()
        .filter(|&&count| count != 0)
        .map(|&count| {
            let probability = count as f64 / token.len() as f64;
            -probability * probability.log2()
        })
        .sum();
    // A token of n bytes holds at most log2(n) bits per byte, so a fixed bar
    // cannot fire on short tokens. The bar is 80% of that ceiling, capped at
    // 64 bytes. Random base64 of 32 to 48 bytes clears it about 99% of the
    // time; hex tops out at 4 bits per byte and clears it almost never, and
    // hex digests are not secrets here.
    let ceiling = (token.len().min(64) as f64).log2();
    entropy >= 0.80 * ceiling
}

/// Prints the refusal when the allowlist is missing.
pub fn refuse_missing_allowlist() -> String {
    "Write the export allowlist at config/export-allowlist.json before exporting. It is a JSON array of absolute repository paths, one per repo whose work may leave this machine; see config/export-allowlist.json.example."
        .to_string()
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

    #[test]
    fn openai_keys_are_redacted_and_short_sk_in_prose_is_not() {
        let prose = "the sk- prefix in prose is not a key";
        let redacted = redact_string(prose, &[]);
        assert!(redacted.contains("sk-"), "{redacted}");
        assert!(
            !redacted.contains("[REDACTED:openai_api_key]"),
            "{redacted}"
        );
        for words in [
            "task-management-system-v2",
            "risk-assessment-notes",
            "flask-sqlalchemy==3.1.1",
            "the `sk-proj-` prefix",
        ] {
            assert_eq!(redact_string(words, &[]), words);
        }
        for key in [
            "sk-abcdefghijklmnopqrstuvwxyz0123456789ABCD",
            "sk-proj-planted_openai-key-value-0123",
            "sk-svcacct-planted_openai-key-value-0123",
        ] {
            let redacted = redact_string(&format!("key `{key}`, done"), &[]);
            assert_eq!(redacted, "key `[REDACTED:openai_api_key]`, done");
        }
    }

    #[test]
    fn widened_secret_classes_are_redacted() {
        let hook = concat!("https://", "hooks.slack.com/services/T00/B00/planted");
        for (text, label, leaked) in [
            ("AKIAPlantedSecretKey", "aws_access_key", "PlantedSecretKey"),
            (
                "aws_secret_access_key=planted-aws-secret-value",
                "aws_secret_access_key",
                "planted-aws-secret-value",
            ),
            (
                "xoxb-planted-slack-bot-token",
                "slack_bot_token",
                "planted-slack-bot-token",
            ),
            (
                "AWS_SECRET_ACCESS_KEY=planted-aws-env-secret",
                "aws_secret_access_key",
                "planted-aws-env-secret",
            ),
            (hook, "slack_webhook", "hooks.slack.com"),
            (
                "github_pat_plantedtokenvalue",
                "github_fine_grained_pat",
                "plantedtokenvalue",
            ),
            (
                "gho_plantedoauthtokenvalue",
                "github_oauth_token",
                "plantedoauthtokenvalue",
            ),
        ] {
            let redacted = redact_string(text, &[]);
            assert!(
                redacted.contains(&format!("[REDACTED:{label}]")),
                "{text} -> {redacted}"
            );
            assert!(!redacted.contains(leaked), "{text} -> {redacted}");
        }
    }

    #[test]
    fn a_redacted_pem_block_passes_the_header_check() {
        for text in [
            "before -----BEGIN RSA PRIVATE KEY-----\nMIIE body\n-----END RSA PRIVATE KEY----- after",
            "before -----BEGIN PRIVATE KEY-----\nno footer",
        ] {
            let redacted = redact_string(text, &[]);
            assert!(!redacted.contains("-----BEGIN"), "{redacted}");
            let line = serde_json::json!({ "output": redacted }).to_string();
            verify_export_line(&line).unwrap();
        }
    }
}
