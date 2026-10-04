//! Join each human verdict with the unified diff the agent left on `hydra/<run-id>`.

use std::collections::HashMap;
use std::path::Path;
use std::process::{Command, Output, Stdio};

use anyhow::{Context, Result, bail};
use hydra_pyjson::json::{Dict, Value};

use crate::queue;
use hydra_records::records;

/// Paths a call of `hydra-grader originals` is told. Tests pass a scratch copy of each.
pub struct Paths<'a> {
    pub home: &'a Path,
    pub dataset: &'a Path,
    pub tickets: &'a Path,
    pub repo: &'a Path,
    pub runs: &'a Path,
    pub out: &'a Path,
}

/// One jsonl line per verdict. A gone branch or base tree is a `missing` reason, never a skip.
pub fn write_originals(paths: &Paths<'_>) -> Result<usize> {
    let verdicts = queue::ticket_verdicts(paths.home)?;
    let dataset = records::read_jsonl(paths.dataset)?;
    let (queue, _) = queue::label_queue(paths.tickets, &dataset)?;
    let mut ticket_of = HashMap::new();
    for row in queue {
        ticket_of.entry(row.run_id.clone()).or_insert(row.ticket);
    }
    let mut lines = Vec::with_capacity(verdicts.len());
    for (run_id, verdict) in &verdicts {
        let (provider, model) = dataset_fields(&dataset, run_id);
        lines.push(one_line(
            run_id,
            verdict,
            ticket_of.get(run_id).cloned(),
            provider,
            model,
            paths.repo,
            paths.runs,
        )?);
    }
    records::write_jsonl(paths.out, &lines)?;
    Ok(lines.len())
}

fn dataset_fields(dataset: &[Value], run_id: &str) -> (Option<String>, Option<String>) {
    for row in dataset {
        let Value::Dict(dict) = row else {
            continue;
        };
        let Some(Value::Str(id)) = dict.get("run_id") else {
            continue;
        };
        let Some(id) = id.to_utf8() else {
            continue;
        };
        if id != run_id {
            continue;
        }
        return (field_str(dict, "provider"), field_str(dict, "model"));
    }
    (None, None)
}

fn field_str(dict: &Dict, key: &str) -> Option<String> {
    match dict.get(key) {
        Some(Value::Str(s)) => s.to_utf8(),
        _ => None,
    }
}

fn nested_str(value: &Value, keys: &[&str]) -> Option<String> {
    let mut cur = value;
    for key in keys {
        let Value::Dict(dict) = cur else {
            return None;
        };
        cur = dict.get(key)?;
    }
    match cur {
        Value::Str(s) => s.to_utf8().filter(|text| !text.is_empty()),
        _ => None,
    }
}

/// The sha the projection prints as `diff_stat.base_tree`, stored on the record as
/// `worktree.base_tree`.
fn base_tree(lines: &[Value]) -> Option<String> {
    for line in lines {
        if let Some(sha) = nested_str(line, &["hydra", "worktree", "base_tree"]) {
            return Some(sha);
        }
        if let Some(sha) = nested_str(line, &["diff_stat", "base_tree"]) {
            return Some(sha);
        }
        if let Some(sha) = nested_str(line, &["hydra", "diff_stat", "base_tree"]) {
            return Some(sha);
        }
    }
    None
}

fn git(repo: &Path, args: &[&str]) -> Result<Output> {
    Command::new("git")
        .args(args)
        .current_dir(repo)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .env_remove("GIT_OBJECT_DIRECTORY")
        .env_remove("GIT_ALTERNATE_OBJECT_DIRECTORIES")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .stdin(Stdio::null())
        .output()
        .with_context(|| format!("git {}", args.join(" ")))
}

fn ok(output: &Output) -> bool {
    output.status.success()
}

fn diff_ran(output: &Output) -> bool {
    matches!(output.status.code(), Some(0) | Some(1))
}

fn one_line(
    run_id: &str,
    verdict: &str,
    ticket: Option<String>,
    provider: Option<String>,
    model: Option<String>,
    repo: &Path,
    runs: &Path,
) -> Result<Value> {
    let mut row = Dict::new();
    row.set("run_id", Value::text(run_id));
    row.set(
        "ticket",
        ticket.map_or(Value::Null, |text| Value::text(&text)),
    );
    row.set("verdict", Value::text(verdict));
    row.set(
        "provider",
        provider.map_or(Value::Null, |text| Value::text(&text)),
    );
    row.set(
        "model",
        model.map_or(Value::Null, |text| Value::text(&text)),
    );

    let named = format!("refs/heads/hydra/{run_id}");
    let parsed = git(repo, &["rev-parse", "--verify", "--quiet", &named])?;
    if !ok(&parsed) {
        row.set("missing", Value::text("branch"));
        return Ok(Value::Dict(row));
    }

    let record = runs.join(format!("{run_id}.jsonl"));
    let lines = records::read_jsonl(&record)?;
    let Some(base) = base_tree(&lines) else {
        row.set("missing", Value::text("base_tree"));
        return Ok(Value::Dict(row));
    };
    let exists = git(repo, &["cat-file", "-e", &base])?;
    if !ok(&exists) {
        row.set("missing", Value::text("base_tree"));
        return Ok(Value::Dict(row));
    }

    let flags = [
        "--no-pager",
        "diff",
        "--no-color",
        "--no-ext-diff",
        "--no-renames",
        base.as_str(),
        named.as_str(),
    ];
    let diff = git(repo, &flags)?;
    if !diff_ran(&diff) {
        bail!(
            "git diff failed for {run_id}: {}",
            String::from_utf8_lossy(&diff.stderr)
        );
    }
    let names = git(
        repo,
        &[
            "--no-pager",
            "diff",
            "--name-only",
            "--no-color",
            "--no-ext-diff",
            "--no-renames",
            base.as_str(),
            named.as_str(),
        ],
    )?;
    if !diff_ran(&names) {
        bail!(
            "git diff --name-only failed for {run_id}: {}",
            String::from_utf8_lossy(&names.stderr)
        );
    }
    let diff_text = String::from_utf8(diff.stdout)
        .with_context(|| format!("git diff for {run_id} was not utf-8"))?;
    let files = String::from_utf8(names.stdout)
        .with_context(|| format!("git diff --name-only for {run_id} was not utf-8"))?
        .lines()
        .filter(|line| !line.is_empty())
        .count();
    row.set("diff", Value::text(&diff_text));
    row.set(
        "files_changed",
        Value::Int(i128::try_from(files).unwrap_or(i128::MAX)),
    );
    Ok(Value::Dict(row))
}
