//! Ticket Outcomes, with normalized harness attribution.
use crate::json::Value;
use crate::{launch, py};
use anyhow::Result;
use std::collections::{BTreeMap, BTreeSet};
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};

// A heuristic over prose that will be wrong sometimes. First class wins;
// rejection and manual completion precede correction and acceptance.
const PHRASES: &[(&str, &[&str])] = &[
    ("rejected", &["rejected", "not applied", "discarded"]),
    (
        "finished by hand",
        &[
            "the session finished",
            "died",
            "capacity",
            "completed the mechanical rest",
        ],
    ),
    (
        "corrected",
        &[
            "needed correction",
            "the session fixed",
            "fixed by the session",
            "with one fix",
            "with one change",
            "with one sentence",
            "amended by the session",
            "the session corrected",
            "corrected there",
            "adjusted by hand",
            "fixes by the session",
            "added one change",
        ],
    ),
    (
        "applied as is",
        &[
            "applied as it was",
            "applied it as it was",
            "entered without correction",
            "aplicado tal cual",
            "applied to master",
            "applied as written",
            "applied by the session",
            "applied unchanged",
            "the session applied",
            "built in this run",
            "applied without changes",
        ],
    ),
];
struct Ticket {
    number: u64,
    ids: Vec<String>,
    class: &'static str,
    phrase: String,
}
#[derive(Clone)]
struct Record {
    id: String,
    provider: String,
    model: String,
    policy: String,
    outcome: String,
    tokens: Option<i128>,
}
#[derive(Default)]
struct Group {
    tickets: BTreeSet<u64>,
    runs: BTreeMap<String, Record>,
    classes: BTreeMap<&'static str, BTreeSet<u64>>,
}
fn classify(text: &str) -> (&'static str, String) {
    // Newlines become spaces so a wrapped Markdown sentence still matches.
    // Length is unchanged, including Spanish phrases.
    let folded = text.replace(['\t', '\n', '\r'], " ");
    let lower = folded.to_ascii_lowercase();
    for (class, phrases) in PHRASES {
        for phrase in *phrases {
            if let Some(at) = lower.find(phrase) {
                return (class, folded[at..at + phrase.len()].into());
            }
        }
    }
    ("unsaid", String::new())
}
fn outcome_prose(outcome: &str) -> String {
    // A pasted report is a fence or a pipe table, not the ticket's own runs.
    outcome
        .lines()
        .take_while(|l| !l.trim_start().starts_with("```") && !l.starts_with('|'))
        .collect::<Vec<_>>()
        .join("\n")
}
fn run_ids(text: &str) -> Vec<String> {
    let mut ids = Vec::new();
    for (at, _) in text.match_indices("01a") {
        if at > 0 && text.as_bytes()[at - 1].is_ascii_alphanumeric() {
            continue;
        }
        let id: String = text[at..]
            .chars()
            .take_while(|c| "0123456789abcdefABCDEF-".contains(*c))
            .collect();
        let id = id.trim_end_matches('-');
        if id.len() >= 13 && !ids.iter().any(|known| known == id) {
            ids.push(id.to_string());
        }
    }
    ids
}
fn load(dir: &Path) -> Result<Vec<Ticket>> {
    let mut tickets = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        let name = entry.file_name();
        let Some((number, _)) = name
            .to_str()
            .and_then(|n| n.strip_suffix(".md"))
            .and_then(|n| n.split_once('-'))
        else {
            continue;
        };
        let Ok(number) = number.parse::<u64>() else {
            continue;
        };
        let text = std::fs::read_to_string(entry.path())?;
        if !text
            .split("\n## ")
            .next()
            .unwrap_or("")
            .lines()
            .any(|l| l.trim() == "Type: build")
        {
            continue;
        }
        let outcome = outcome_prose(
            &text
                .lines()
                .skip_while(|l| l.trim() != "## Outcome")
                .skip(1)
                .take_while(|l| !l.starts_with("## ") || l.trim() == "## Outcome")
                .collect::<Vec<_>>()
                .join("\n"),
        );
        let (class, phrase) = classify(&outcome);
        tickets.push(Ticket {
            number,
            ids: run_ids(&outcome),
            class,
            phrase,
        });
    }
    tickets.sort_by_key(|t| t.number);
    Ok(tickets)
}
fn field(v: &Value, key: &str) -> Value {
    match v {
        Value::Dict(d) => d.get(key).cloned().unwrap_or(Value::Null),
        _ => Value::Null,
    }
}
fn display(v: Value) -> String {
    if matches!(v, Value::Null) {
        "unavailable".into()
    } else {
        py::str(&v).lossy()
    }
}
fn clean(text: &str) -> String {
    text.replace(['\t', '\n', '\r'], " ")
}
fn tokens(n: Option<i128>) -> String {
    n.map_or_else(|| "unavailable".into(), |n| n.to_string())
}

// Reuse wait's normalized show reader. The temporary layout lets --runs name
// any directory, without copying or modifying a record. It is removed on exit.
struct ShowRoot(PathBuf);
impl ShowRoot {
    fn new(runs: &Path) -> Result<Self> {
        let runs = runs.canonicalize()?;
        for serial in 0..1000 {
            let path = PathBuf::from(format!(".puppets-report-{}-{serial}", std::process::id()));
            match std::fs::create_dir(&path) {
                Ok(()) => {
                    let root = Self(path);
                    std::os::unix::fs::symlink(&runs, root.0.join("runs"))?;
                    return Ok(root);
                }
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(e) => return Err(e.into()),
            }
        }
        anyhow::bail!("could not create temporary report layout")
    }
}
impl Drop for ShowRoot {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(self.0.join("runs"));
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn record(root: &ShowRoot, id: &str) -> Result<Record> {
    // Envelopes carry the provider and a model backfilled after line zero.
    let mut provider = Value::Null;
    let mut model = Value::Null;
    let path = root.0.join("runs").join(format!("{id}.jsonl"));
    for line in std::io::BufReader::new(std::fs::File::open(path)?).split(b'\n') {
        let line = line?;
        let Ok(envelope) = crate::json::loads_bytes(&line) else {
            continue;
        };
        let p = field(&envelope, "provider");
        let m = field(&envelope, "model");
        if !matches!(p, Value::Null) {
            provider = p;
        }
        if !matches!(m, Value::Null) {
            model = m;
        }
    }
    let home = crate::log::Home {
        root: root.0.clone(),
        puppets: PathBuf::new(),
        grader: PathBuf::new(),
        vendors: None,
    };
    if !launch::run_file_has_end(&home, &id.into())? {
        return Ok(Record {
            id: id.into(),
            provider: display(provider),
            model: display(model),
            policy: "unavailable".into(),
            outcome: "unfinished".into(),
            tokens: None,
        });
    }
    let output =
        std::process::Command::new(std::env::var_os("HYDRA_BIN").unwrap_or_else(|| "hydra".into()))
            .args(["show", id, "--events"])
            .env("HYDRA_ROOT", &root.0)
            .output()?;
    anyhow::ensure!(
        output.status.success(),
        "hydra show {id}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let shown = launch::loads_show(std::str::from_utf8(&output.stdout)?)?;
    let projection = field(&shown, "projection");
    let mut start = Value::Null;
    if let Value::List(events) = field(&shown, "events") {
        for event in events {
            if py::eq_str(&field(&event, "type"), "run_start") {
                start = field(&event, "body");
                break;
            }
        }
    }
    let counts = field(&field(&projection, "usage"), "counts");
    Ok(Record {
        id: id.into(),
        provider: display(provider),
        model: display(model),
        policy: display(field(&start, "policy")),
        outcome: display(field(&projection, "outcome")),
        tokens: match field(&counts, "total_tokens") {
            Value::Int(n) => Some(n),
            _ => None,
        },
    })
}
/// Two TSV tables. Omitting runs produces an explicitly unavailable record view.
pub fn run(dir: &Path, runs: Option<&Path>, out: &mut dyn Write) -> Result<i32> {
    let tickets = load(dir)?;
    let mut names = Vec::new();
    if let Some(runs) = runs {
        for entry in std::fs::read_dir(runs)? {
            let entry = entry?;
            if entry.file_type()?.is_file()
                && let Some(id) = entry
                    .file_name()
                    .to_str()
                    .and_then(|n| n.strip_suffix(".jsonl"))
            {
                names.push(id.to_string());
            }
        }
        names.sort();
    }
    let root = runs.map(ShowRoot::new).transpose()?;
    let mut cache = BTreeMap::new();
    let mut groups: BTreeMap<(String, String), Group> = BTreeMap::new();
    let mut rows = Vec::new();
    for t in &tickets {
        let mut seen = BTreeSet::new();
        let (mut ids, mut models, mut policies, mut outcomes) =
            (Vec::new(), Vec::new(), Vec::new(), Vec::new());
        let mut sum = Some(0_i128);
        for prefix in &t.ids {
            let found: Vec<_> = names.iter().filter(|id| id.starts_with(prefix)).collect();
            let state = match (runs, found.as_slice()) {
                (None, _) => "unavailable",
                (_, []) => "missing",
                (_, [_]) => "resolved",
                _ => "ambiguous",
            };
            if state != "resolved" {
                ids.push(format!("{prefix} ({state})"));
                models.push("unavailable".into());
                policies.push("unavailable".into());
                outcomes.push(state.into());
                sum = None;
                continue;
            }
            let id = found[0];
            if !seen.insert(id.clone()) {
                continue;
            }
            if !cache.contains_key(id) {
                cache.insert(id.clone(), record(root.as_ref().unwrap(), id)?);
            }
            let r: &Record = &cache[id];
            ids.push(r.id.clone());
            models.push(r.model.clone());
            policies.push(r.policy.clone());
            outcomes.push(r.outcome.clone());
            sum = sum.zip(r.tokens).map(|(a, b)| a + b);
            let group = groups
                .entry((r.provider.clone(), r.model.clone()))
                .or_default();
            group.tickets.insert(t.number);
            group.runs.insert(r.id.clone(), r.clone());
            group.classes.entry(t.class).or_default().insert(t.number);
        }
        if t.ids.is_empty() {
            sum = None;
        }
        let joined = |v: Vec<String>| {
            if v.is_empty() {
                "unavailable".into()
            } else {
                v.join("; ")
            }
        };
        rows.push(format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            t.number,
            if ids.is_empty() {
                "-".into()
            } else {
                ids.join("; ")
            },
            joined(models),
            tokens(sum),
            joined(outcomes),
            t.class,
            clean(&t.phrase),
            joined(policies)
        ));
    }
    writeln!(
        out,
        "provider\tmodel\ttickets\truns\ttotal_tokens\tmedian_tokens\tcompleted\tfailed\trest\tapplied as is\tcorrected\tfinished by hand\trejected"
    )?;
    for ((provider, model), g) in groups {
        let mut counts: Vec<_> = g.runs.values().filter_map(|r| r.tokens).collect();
        counts.sort();
        let complete = counts.len() == g.runs.len();
        let median = if !complete || counts.is_empty() {
            "unavailable".into()
        } else {
            let mid = counts.len() / 2;
            if counts.len() % 2 == 1 {
                counts[mid].to_string()
            } else {
                let sum = counts[mid - 1] + counts[mid];
                if sum % 2 == 0 {
                    (sum / 2).to_string()
                } else {
                    format!("{}.5", sum / 2)
                }
            }
        };
        let completed = g.runs.values().filter(|r| r.outcome == "completed").count();
        let failed = g.runs.values().filter(|r| r.outcome == "failed").count();
        writeln!(
            out,
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            clean(&provider),
            clean(&model),
            g.tickets.len(),
            g.runs.len(),
            tokens(complete.then(|| counts.iter().sum())),
            median,
            completed,
            failed,
            g.runs.len() - completed - failed,
            g.classes.get("applied as is").map_or(0, BTreeSet::len),
            g.classes.get("corrected").map_or(0, BTreeSet::len),
            g.classes.get("finished by hand").map_or(0, BTreeSet::len),
            g.classes.get("rejected").map_or(0, BTreeSet::len)
        )?;
    }
    if runs.is_none() {
        writeln!(
            out,
            "unavailable\tunavailable\t{}\tunavailable\tunavailable\tunavailable\tunavailable\tunavailable\tunavailable\t{}\t{}\t{}\t{}",
            tickets.len(),
            tickets
                .iter()
                .filter(|t| t.class == "applied as is")
                .count(),
            tickets.iter().filter(|t| t.class == "corrected").count(),
            tickets
                .iter()
                .filter(|t| t.class == "finished by hand")
                .count(),
            tickets.iter().filter(|t| t.class == "rejected").count()
        )?;
    }
    writeln!(
        out,
        "\nticket\truns\tmodel\ttokens\trecord outcome\tprose class\tmatched phrase\tpolicy"
    )?;
    for row in rows {
        writeln!(out, "{row}")?;
    }
    writeln!(
        out,
        "\nunsaid: {} of {} tickets",
        tickets.iter().filter(|t| t.class == "unsaid").count(),
        tickets.len()
    )?;
    Ok(0)
}
