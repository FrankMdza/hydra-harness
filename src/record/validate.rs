//! A contract checker for a run file.
//!
//! The claim "a valid run record per `docs/run-record-contract.md`" is worth only
//! as much as the thing that checks it, so this reads a file back and reports every
//! way it departs from sections 2 to 6. It is not the normalizer: it derives no
//! event type, reads no `raw` frame, and knows nothing about any provider. It
//! checks the shape the harness promised to write.
//!
//! The command surface in harness spec 17 has no `verify`, so this stays a library
//! function that the tests drive rather than a command nobody specced.

use std::path::Path;

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde_json::Value;

/// Everything wrong with one run file, in the order the reader found it.
#[derive(Debug, Default)]
pub struct Report {
    pub problems: Vec<String>,
    pub lines: usize,
}

impl Report {
    pub fn ok(&self) -> bool {
        self.problems.is_empty()
    }

    pub fn assert_ok(&self) {
        assert!(
            self.ok(),
            "run record does not meet the contract:\n  {}",
            self.problems.join("\n  ")
        );
    }
}

const ENVELOPE_FIELDS: &[&str] = &[
    "run_id",
    "parent_run_id",
    "seq",
    "ts",
    "provider",
    "model",
    "secret_flag",
];

const RUN_START_FIELDS: &[&str] = &[
    "type",
    "schema_version",
    "hydra_version",
    "command",
    "launch_mode",
    "provider_version",
    "provider_session_id",
    "task",
    "policy",
    "policy_sha",
    "cwd",
    "worktree",
    "depth",
    "pid",
    "process_start_time",
    "subscription",
];

const RUN_END_FIELDS: &[&str] = &[
    "type",
    "exit_code",
    "error",
    "refusal",
    "remediation",
    "body_source",
    "ended_at",
];

pub fn check_file(path: &Path) -> Result<Report> {
    let text =
        std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let stem = path.file_stem().and_then(|s| s.to_str());
    Ok(check(&text, stem))
}

pub fn check(text: &str, expected_run_id: Option<&str>) -> Report {
    let mut report = Report::default();
    let mut say = |problem: String| report.problems.push(problem);

    let lines: Vec<&str> = text.lines().collect();
    if lines.is_empty() {
        say("the file is empty; every run has at least a run_start".into());
        return report;
    }

    let mut run_id: Option<String> = None;
    let mut last_ts: Option<DateTime<Utc>> = None;
    let mut run_end_at: Option<usize> = None;
    let mut provider_run = true;

    for (index, line) in lines.iter().enumerate() {
        let at = format!("line {index}");
        let Ok(Value::Object(obj)) = serde_json::from_str::<Value>(line) else {
            say(format!("{at}: not a JSON object"));
            continue;
        };

        for field in ENVELOPE_FIELDS {
            if !obj.contains_key(*field) {
                say(format!("{at}: envelope is missing {field}"));
            }
        }

        match obj.get("seq").and_then(Value::as_u64) {
            Some(seq) if seq as usize == index => {}
            Some(seq) => say(format!("{at}: seq is {seq}, so a line was lost")),
            None => say(format!("{at}: seq is not an integer")),
        }

        match obj.get("run_id").and_then(Value::as_str) {
            Some(id) => match &run_id {
                None => {
                    run_id = Some(id.to_string());
                    if let Some(expected) = expected_run_id
                        && expected != id
                    {
                        say(format!(
                            "{at}: run_id {id} does not match the filename {expected}"
                        ));
                    }
                }
                Some(first) if first != id => {
                    say(format!("{at}: run_id {id} differs from line 0's {first}"))
                }
                Some(_) => {}
            },
            None => say(format!("{at}: run_id is not a string")),
        }

        match obj.get("ts").and_then(Value::as_str) {
            Some(ts) => match DateTime::parse_from_rfc3339(ts) {
                Ok(parsed) => {
                    let parsed = parsed.with_timezone(&Utc);
                    if last_ts.is_some_and(|prev| parsed < prev) {
                        say(format!("{at}: ts goes backwards"));
                    }
                    last_ts = Some(parsed);
                }
                Err(_) => say(format!("{at}: ts {ts} is not RFC 3339")),
            },
            None => say(format!("{at}: ts is not a string")),
        }

        match obj.get("secret_flag") {
            Some(Value::Null) => {}
            Some(Value::Array(classes)) if !classes.is_empty() => {
                if !classes.iter().all(Value::is_string) {
                    say(format!(
                        "{at}: secret_flag holds something that is not a class name"
                    ));
                }
            }
            Some(_) => say(format!(
                "{at}: secret_flag is neither null nor a non-empty array"
            )),
            None => {}
        }

        let has_raw = obj.contains_key("raw");
        let has_hydra = obj.contains_key("hydra");
        match (has_raw, has_hydra) {
            (true, true) => say(format!("{at}: carries both raw and hydra")),
            (false, false) => say(format!("{at}: carries neither raw nor hydra")),
            _ => {}
        }

        if let Some(Value::Object(body)) = obj.get("hydra") {
            let kind = body.get("type").and_then(Value::as_str).unwrap_or("");
            match kind {
                "run_start" => {
                    if index != 0 {
                        say(format!("{at}: run_start is not the first line"));
                    }
                    check_run_start(body, &at, &mut say);
                    provider_run = !body.get("provider").is_some_and(Value::is_null);
                    if obj.get("provider").is_some_and(Value::is_null) {
                        provider_run = false;
                    }
                }
                "run_end" => {
                    if run_end_at.is_some() {
                        say(format!("{at}: a second run_end"));
                    }
                    // The first one, so a run_end with anything after it is caught
                    // whether the follower is a frame or a second terminator.
                    run_end_at.get_or_insert(index);
                    for field in RUN_END_FIELDS {
                        if !body.contains_key(*field) {
                            say(format!("{at}: run_end is missing {field}"));
                        }
                    }
                }
                other => say(format!(
                    "{at}: a Hydra line of type {other:?}; the contract allows exactly two"
                )),
            }
        } else if has_hydra {
            say(format!("{at}: hydra is not an object"));
        }

        if index == 0 && !has_hydra {
            say(format!("{at}: the first line is not a Hydra run_start"));
        }

        if has_raw && obj.get("provider").is_some_and(Value::is_null) {
            say(format!("{at}: a provider line with a null provider"));
        }
    }

    if let Some(end) = run_end_at
        && end != lines.len() - 1
    {
        say(format!(
            "line {end}: run_end is followed by {} more lines",
            lines.len() - 1 - end
        ));
    }

    // Contract 5.5 says two lines, and contract 6 says a run with no `run_end`
    // is not corrupt. A harness run with one line is a pass still running, and
    // during a `hydra evolve` pass that is guaranteed: its admission gate runs
    // `cargo test`, which walks this corpus and reads the pass's own open
    // record. Measured 2026-09-08 on pass 01a08169-eb3b, which blocked its own
    // patch on it. So one line is allowed and three is not.
    if !provider_run && lines.len() > 2 {
        say(format!(
            "a harness run has {} lines; contract 5.5 says run_start, run_end and nothing between",
            lines.len()
        ));
    }

    report.lines = lines.len();
    report
}

fn check_run_start(body: &serde_json::Map<String, Value>, at: &str, say: &mut impl FnMut(String)) {
    for field in RUN_START_FIELDS {
        if !body.contains_key(*field) {
            say(format!("{at}: run_start is missing {field}"));
        }
    }

    // Contract 5.5 and 5.6: these groups are absent when the question does not
    // apply, never null. A null store_sha invites a reader to ask which sha the run
    // pinned, and no sha was pinned. `scopes` joined them under ticket 42, and for
    // the same reason: a harness run compiled no gate, so which paths it resolved
    // is not a question with a missing answer.
    for group in ["skills", "mcp", "auth", "scopes"] {
        if body.get(group).is_some_and(Value::is_null) {
            say(format!(
                "{at}: {group} is null; the contract says absent when it does not apply"
            ));
        }
    }

    // Contract 5.3: each of the two injection groups carries a sha and two name
    // lists, and the reason is the same both times. A sha without names does not
    // say what was selected and names without a sha do not say what bytes they
    // resolved to, so a group missing either half is a group that answers half
    // the question.
    for (group, fields) in [
        ("skills", ["store_sha", "linked", "resolved"]),
        ("mcp", ["doc_sha", "injected", "resolved"]),
    ] {
        let Some(Value::Object(inner)) = body.get(group) else {
            continue;
        };
        for field in fields {
            match inner.get(field) {
                None => say(format!("{at}: {group} is missing {field}")),
                // `resolved` is null when the provider could not be asked, which
                // 5.3 requires a reader to tell apart from an empty listing. The
                // other two are always answerable.
                Some(Value::Null) if field != "resolved" => {
                    say(format!("{at}: {group}.{field} is null"))
                }
                Some(Value::Null) => {}
                Some(value) if field.ends_with("_sha") && !value.is_string() => {
                    say(format!("{at}: {group}.{field} is not a sha"))
                }
                Some(value) if !field.ends_with("_sha") && !value.is_array() => {
                    say(format!("{at}: {group}.{field} is not a list of names"))
                }
                Some(_) => {}
            }
        }
    }

    match (body.get("pid"), body.get("process_start_time")) {
        (Some(Value::Null), Some(Value::Null)) => {}
        (Some(pid), Some(start)) if pid.is_u64() && start.is_string() => {}
        _ => say(format!(
            "{at}: pid and process_start_time disagree about whether a process ran"
        )),
    }

    if let Some(mode) = body.get("launch_mode").and_then(Value::as_str)
        && !matches!(mode, "recorded" | "interactive")
    {
        say(format!(
            "{at}: launch_mode {mode:?} is neither recorded nor interactive"
        ));
    }

    if let Some(command) = body.get("command").and_then(Value::as_str)
        && !matches!(command, "run" | "evolve" | "resume" | "curate")
    {
        say(format!(
            "{at}: command {command:?} is not one of the four that mint a run id"
        ));
    }
}
