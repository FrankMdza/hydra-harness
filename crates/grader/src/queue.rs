//! `baseline.py` lines 279 to 370: the verdict queue and `label-import`.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use anyhow::{Result, bail};
use hydra_pyjson::json::{Dict, PyStr, Value};

use crate::jev;
use crate::records;

const QUEUE_FIELDS: [&str; 6] = [
    "ticket", "run_id", "provider", "model", "outcome", "verdict",
];
const VERDICTS: [&str; 3] = ["applied", "fixed", "unknown"];
const TICKET_VERDICTS: &str = "ticket-verdicts.json";

fn word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// The greedy, word-bounded UUID prefixes used by Python's RUN_ID pattern.
fn run_names(text: &str) -> Vec<&str> {
    let bytes = text.as_bytes();
    let hex = |slice: &[u8]| {
        slice
            .iter()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(b))
    };
    let mut names = Vec::new();
    let mut i = 0;
    while i + 8 <= bytes.len() {
        if !text.is_char_boundary(i)
            || (i > 0 && text[..i].chars().next_back().is_some_and(word))
            || !hex(&bytes[i..i + 8])
        {
            i += 1;
            continue;
        }
        let mut ends = vec![i + 8];
        let mut end = i + 8;
        for _ in 0..3 {
            if end + 5 <= bytes.len() && bytes[end] == b'-' && hex(&bytes[end + 1..end + 5]) {
                end += 5;
                ends.push(end);
            } else {
                break;
            }
        }
        let partial = ends.clone();
        for end in partial {
            if end + 13 <= bytes.len() && bytes[end] == b'-' && hex(&bytes[end + 1..end + 13]) {
                ends.push(end + 13);
            }
        }
        ends.sort_unstable();
        if let Some(end) = ends.into_iter().rev().find(|end| {
            text.is_char_boundary(*end) && !text[*end..].chars().next().is_some_and(word)
        }) {
            names.push(&text[i..end]);
            i = end;
        } else {
            i += 1;
        }
    }
    names
}

/// The text under `## Outcome` up to the next `## ` heading.
pub fn outcome_section(body: &str) -> Option<String> {
    let marker = "## Outcome";
    let start = body.find(marker)?;
    let rest = &body[start + marker.len()..];
    let end = rest.find("\n## ");
    let section = match end {
        Some(at) => &rest[..at],
        None => rest,
    };
    Some(section.trim_matches(hydra_pyjson::py::is_space).to_string())
}

#[derive(Debug, Clone)]
pub struct QueueRow {
    pub ticket: String,
    pub run_id: String,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub outcome: String,
    pub verdict: String,
}

/// One row per dataset run named in the `## Outcome` of a resolved build ticket.
pub fn label_queue(tickets: &Path, dataset: &[Value]) -> Result<(Vec<QueueRow>, Vec<String>)> {
    let mut queue = Vec::new();
    let mut skips = Vec::new();
    let mut paths: Vec<_> = match tickets.read_dir() {
        Ok(read) => read
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter(|path| path.extension().is_some_and(|ext| ext == "md"))
            .collect(),
        Err(_) => Vec::new(),
    };
    paths.sort();
    for path in paths {
        let Some(number) =
            jev::ticket_number(path.file_name().and_then(|n| n.to_str()).unwrap_or(""))
        else {
            continue;
        };
        let body = hydra_pyjson::file::read_text(&path)?
            .replace("\r\n", "\n")
            .replace('\r', "\n");
        if jev::ticket_status(&body).as_deref() != Some("resolved") {
            continue;
        }
        let Some(outcome) = outcome_section(&body) else {
            continue;
        };
        let mut seen = HashSet::new();
        for name in run_names(&outcome) {
            let matches: Vec<_> = dataset
                .iter()
                .filter(|row| row_run_id(row).is_some_and(|id| id.starts_with(name)))
                .collect();
            if matches.len() != 1 {
                skips.push(format!(
                    "{number}: {name} names {} dataset runs",
                    matches.len()
                ));
                continue;
            }
            let row = matches[0];
            let Some(run_id) = row_run_id(row) else {
                continue;
            };
            if !seen.insert(run_id.clone()) {
                continue;
            }
            let outcome_text: String = outcome.chars().take(400).collect();
            queue.push(QueueRow {
                ticket: number.to_string(),
                run_id,
                provider: row_field_str(row, "provider"),
                model: row_field_str(row, "model"),
                outcome: outcome_text,
                verdict: String::new(),
            });
        }
    }
    Ok((queue, skips))
}

fn row_run_id(row: &Value) -> Option<String> {
    match row {
        Value::Dict(dict) => match dict.get("run_id") {
            Some(Value::Str(s)) => s.to_utf8(),
            _ => None,
        },
        _ => None,
    }
}

fn row_field_str(row: &Value, key: &str) -> Option<String> {
    match row {
        Value::Dict(dict) => match dict.get(key) {
            Some(Value::Str(s)) => s.to_utf8(),
            Some(Value::Null) | None => None,
            Some(value) => hydra_pyjson::py::str(value).to_utf8(),
        },
        _ => None,
    }
}

/// Write the queue CSV the way `csv.DictWriter` does on CPython 3.14.
pub fn write_label_queue_csv(path: &Path, queue: &[QueueRow]) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut text = csv_line(&QUEUE_FIELDS);
    for row in queue {
        text.push_str(&csv_line(&[
            &row.ticket,
            &row.run_id,
            row.provider.as_deref().unwrap_or(""),
            row.model.as_deref().unwrap_or(""),
            &row.outcome,
            &row.verdict,
        ]));
    }
    std::fs::write(path, text)?;
    Ok(())
}

fn csv_line(fields: &[&str]) -> String {
    let fields: Vec<String> = fields
        .iter()
        .map(|field| {
            if field.contains([',', '"', '\r', '\n']) {
                format!("\"{}\"", field.replace('"', "\"\""))
            } else {
                field.to_string()
            }
        })
        .collect();
    format!("{}\r\n", fields.join(","))
}

// csv's default dialect is permissive: doubled quotes, embedded line ends, blank
// records skipped, and text after a closing quote retained.
fn read_csv_rows(path: &Path) -> Result<Vec<Dict>> {
    let text = hydra_pyjson::file::read_text(path)?;
    let mut chars = text.chars().peekable();
    let mut records = Vec::new();
    let mut row = Vec::new();
    let mut field = String::new();
    let mut quoted = false;
    let mut start = true;
    let mut active = false;
    while let Some(c) = chars.next() {
        if quoted {
            if c == '"' {
                if chars.peek() == Some(&'"') {
                    chars.next();
                    field.push('"');
                } else {
                    quoted = false;
                }
            } else {
                field.push(c);
            }
        } else {
            match c {
                '"' if start => {
                    quoted = true;
                    active = true;
                    start = false;
                }
                ',' => {
                    row.push(std::mem::take(&mut field));
                    start = true;
                    active = true;
                }
                '\r' | '\n' => {
                    if c == '\r' && chars.peek() == Some(&'\n') {
                        chars.next();
                    }
                    if active {
                        row.push(std::mem::take(&mut field));
                        records.push(std::mem::take(&mut row));
                    }
                    active = false;
                    start = true;
                }
                _ => {
                    field.push(c);
                    start = false;
                    active = true;
                }
            }
        }
    }
    if active {
        row.push(field);
        records.push(row);
    }
    let mut records = records.into_iter();
    let Some(headers) = records.next() else {
        return Ok(Vec::new());
    };
    Ok(records
        .map(|record| {
            let mut dict = Dict::new();
            for (i, key) in headers.iter().enumerate() {
                dict.set(
                    key.as_str(),
                    record.get(i).map_or(Value::Null, |v| Value::text(v)),
                );
            }
            dict
        })
        .collect())
}

/// Fold a filled-in queue into `ticket-verdicts.json` and the dataset's `human_verdict`.
pub fn label_import(csv_path: &Path, dataset_path: &Path, home: &Path) -> Result<(usize, usize)> {
    let filled = read_csv_rows(csv_path)?;
    let mut verdicts: Dict = Dict::new();
    let mut unknown = 0usize;
    for row in &filled {
        let run_id = dict_str(row, "run_id").unwrap_or_else(|| "None".into());
        let verdict = dict_str(row, "verdict")
            .unwrap_or_default()
            .trim_matches(hydra_pyjson::py::is_space)
            .to_lowercase();
        if !verdict.is_empty() && !VERDICTS.contains(&verdict.as_str()) {
            bail!(
                "baseline: run {run_id} has verdict '{verdict}'; use one of {} or leave it empty",
                VERDICTS.join(", ")
            );
        }
        if verdict.is_empty() || verdict == "unknown" {
            unknown += 1;
            continue;
        }
        if row.get("run_id").is_none() {
            bail!("KeyError: 'run_id'");
        }
        verdicts.set(run_id.as_str(), Value::Str(PyStr::from(verdict)));
    }
    let kept_path = home.join(TICKET_VERDICTS);
    let mut kept = records::read_json(&kept_path)?.unwrap_or(Value::Dict(Dict::new()));
    let Value::Dict(ref mut kept_dict) = kept else {
        bail!("ValueError: ticket-verdicts.json is not an object");
    };
    for (key, value) in verdicts.iter() {
        kept_dict.set_key(key.clone(), value.clone());
    }
    records::write_json(&kept_path, &kept)?;
    let merged = match &kept {
        Value::Dict(dict) => dict,
        _ => bail!("ValueError: ticket-verdicts.json is not an object"),
    };
    let mut rows = records::read_jsonl(dataset_path)?;
    for row in &mut rows {
        let Some(run_id) = row_run_id(row) else {
            continue;
        };
        if let Value::Dict(dict) = row
            && let Some(v) = merged.get(run_id.as_str())
        {
            dict.set("human_verdict", v.clone());
        }
    }
    records::write_jsonl(dataset_path, &rows)?;
    Ok((verdicts.len(), unknown))
}

fn dict_str(dict: &Dict, key: &str) -> Option<String> {
    match dict.get(key) {
        Some(Value::Str(s)) => s.to_utf8(),
        Some(Value::Null) | None => None,
        Some(_) => Some(String::new()),
    }
}

/// Default `--tickets` beside the repo root.
pub fn default_tickets_dir() -> PathBuf {
    // Like baseline.py's HERE.parents[1], independent of the caller's cwd.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("grader lives in crates/grader")
        .join(".scratch/hydra/build")
}
