//! `jev_probe.py` without `ask`: build tickets into jsonl, patch rows, and score answers.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, anyhow, bail};
use hydra_pyjson::json::{Dict, PyStr, Value};
use hydra_pyjson::math::{fsum_like_python, pow};

const MIN_TICKET: i32 = 112;
const BUILD_DIR: &str = ".scratch/hydra/build";
const SCORE_QUESTION: &str = "needs_correction";

pub fn ticket_number(name: &str) -> Option<i32> {
    let stem = name.split('.').next()?;
    let (prefix, _) = stem.split_once('-')?;
    if prefix.is_empty() || !prefix.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    prefix.parse().ok()
}

pub fn ticket_status(body: &str) -> Option<String> {
    for line in body.lines() {
        if let Some(rest) = line.strip_prefix("Status:") {
            return Some(rest.trim().to_ascii_lowercase());
        }
    }
    None
}

fn word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

// Search words separated by Python whitespace. Only the reference's bounded
// expression checks the characters before and after the whole phrase.
fn phrase(text: &str, parts: &[&str], bounded: bool) -> bool {
    text.match_indices(parts[0]).any(|(start, _)| {
        if bounded && text[..start].chars().next_back().is_some_and(word) {
            return false;
        }
        let mut rest = &text[start + parts[0].len()..];
        for part in &parts[1..] {
            let trimmed = rest.trim_start_matches(hydra_pyjson::py::is_space);
            if trimmed.len() == rest.len() {
                return false;
            }
            let Some(after) = trimmed.strip_prefix(part) else {
                return false;
            };
            rest = after;
        }
        !bounded || !rest.chars().next().is_some_and(word)
    })
}

fn read_outcome(text: &str) -> Option<&'static str> {
    let lower = text.to_lowercase();
    let has_run = lower.match_indices("run").any(|(start, _)| {
        if lower[..start].chars().next_back().is_some_and(word) {
            return false;
        }
        let rest = &lower[start + 3..];
        let id = rest.trim_start_matches(hydra_pyjson::py::is_space);
        let bytes = id.as_bytes();
        rest.len() != id.len()
            && bytes.len() >= 13
            && bytes[8] == b'-'
            && bytes[..8]
                .iter()
                .chain(&bytes[9..13])
                .all(u8::is_ascii_hexdigit)
    });
    if !has_run {
        return None;
    }
    if ["changed", "fixed", "reworded"]
        .iter()
        .any(|verb| phrase(&lower, &["session", verb], false))
        || phrase(&lower, &["added", "something", "to", "the", "diff"], false)
    {
        return Some("changed");
    }
    if lower.contains("applied the diff as written")
        || phrase(&lower, &["applied", "as", "written"], true)
    {
        return Some("as_written");
    }
    None
}

pub fn extract_outcome(body: &str) -> Option<&'static str> {
    let marker = "## Outcome";
    let start = body.find(marker)?;
    let rest = &body[start + marker.len()..];
    let nxt = rest.find("\n## ");
    let section = match nxt {
        Some(at) => &rest[..at],
        None => rest,
    };
    read_outcome(section)
}

fn repo_root() -> Result<PathBuf> {
    let out = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .context("git rev-parse")?;
    if !out.status.success() {
        bail!("git rev-parse failed");
    }
    Ok(PathBuf::from(String::from_utf8(out.stdout)?.trim()))
}

fn find_build_commit(repo: &Path, number: i32) -> Result<Option<String>> {
    let prefix = format!("Build {number}:");
    let out = Command::new("git")
        .args(["log", "--format=%H", "-1", &format!("--grep=^{prefix}")])
        .current_dir(repo)
        .output()
        .context("git log")?;
    if !out.status.success() {
        bail!("git log failed");
    }
    let line = String::from_utf8(out.stdout)?.trim().to_string();
    Ok((!line.is_empty()).then_some(line))
}

fn commit_diff(repo: &Path, commit: &str) -> Result<String> {
    let out = Command::new("git")
        .args([
            "show",
            commit,
            "--pretty=format:",
            "--",
            "src/",
            "tests/",
            "integrations/",
        ])
        .current_dir(repo)
        .output()
        .context("git show")?;
    if !out.status.success() {
        bail!("git show failed");
    }
    Ok(String::from_utf8(out.stdout)?)
}

fn iter_build_tickets(root: &Path) -> Result<Vec<PathBuf>> {
    let build = root.join(BUILD_DIR);
    let mut paths = Vec::new();
    let Ok(read) = build.read_dir() else {
        return Ok(paths);
    };
    for entry in read.filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "md")
            && let Some(num) =
                ticket_number(path.file_name().and_then(|n| n.to_str()).unwrap_or(""))
            && num >= MIN_TICKET
        {
            paths.push(path);
        }
    }
    paths.sort();
    Ok(paths)
}

/// Label build tickets and capture committed diffs.
pub fn cmd_build(out_path: &Path) -> Result<(String, Vec<String>)> {
    let repo = repo_root()?;
    let mut rows: Vec<Dict> = Vec::new();
    let mut skips = Vec::new();
    let mut counts = [0usize, 0usize]; // changed, as_written

    for path in iter_build_tickets(&repo)? {
        let num = ticket_number(path.file_name().and_then(|n| n.to_str()).unwrap_or(""))
            .context("ticket number")?;
        let body = read_text(&path)?;
        let status = ticket_status(&body);
        if status.as_deref() != Some("resolved") {
            skips.push(format!(
                "{num}: status {}",
                status.unwrap_or_else(|| "missing".into())
            ));
            continue;
        }
        let Some(label) = extract_outcome(&body) else {
            skips.push(format!("{num}: outcome not labelled"));
            continue;
        };
        let Some(commit) = find_build_commit(&repo, num)? else {
            skips.push(format!("{num}: no Build {num}: commit"));
            continue;
        };
        let diff = commit_diff(&repo, &commit)?;
        if diff.trim().is_empty() {
            skips.push(format!(
                "{num}: empty diff under src/, tests/, integrations/"
            ));
            continue;
        }
        let mut row = Dict::new();
        row.set("ticket", Value::Int(i128::from(num)));
        row.set("label", Value::Str(hydra_pyjson::json::PyStr::from(label)));
        row.set("diff", Value::Str(hydra_pyjson::json::PyStr::from(diff)));
        row.set(
            "commit",
            Value::Str(hydra_pyjson::json::PyStr::from(commit)),
        );
        rows.push(row);
        counts[if label == "changed" { 0 } else { 1 }] += 1;
    }

    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut file = std::fs::File::create(out_path)?;
    for row in &rows {
        let value = Value::Dict(row.clone());
        let line = hydra_pyjson::json::dumps_unicode_default(&value);
        writeln!(file, "{line}")?;
    }

    let mut stdout = format!("changed: {}\n", counts[0]);
    stdout.push_str(&format!("as_written: {}\n", counts[1]));
    for line in &skips {
        stdout.push_str(&format!("skip: {line}\n"));
    }
    Ok((stdout, skips))
}

/// Rows from the agents' own diffs and hand labels.
pub fn cmd_patches(labels_path: &Path, out_path: &Path) -> Result<String> {
    let text = read_text(labels_path)?;
    let labels = hydra_pyjson::json::loads_str(&text).map_err(|err| anyhow!("{err:?}"))?;
    let Value::List(items) = labels else {
        bail!("ValueError: labels must be a list");
    };
    let mut counts = [0usize, 0usize];
    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut file = std::fs::File::create(out_path)?;
    for item in &items {
        let Value::Dict(row) = item else {
            continue;
        };
        let ticket = row.get("ticket").cloned().unwrap_or(Value::Null);
        let label = row
            .get("label")
            .and_then(|v| match v {
                Value::Str(s) => s.to_utf8(),
                _ => None,
            })
            .unwrap_or_default();
        let patch = row
            .get("patch")
            .and_then(|v| match v {
                Value::Str(s) => s.to_utf8(),
                _ => None,
            })
            .context("patch path")?;
        let diff = read_text(&expand_user(&patch))?;
        let mut out_row = Dict::new();
        out_row.set("ticket", ticket);
        out_row.set("label", Value::Str(PyStr::from(label.as_str())));
        out_row.set("diff", Value::Str(PyStr::from(diff)));
        let line = hydra_pyjson::json::dumps_unicode_default(&Value::Dict(out_row));
        writeln!(file, "{line}")?;
        counts[if label == "changed" { 0 } else { 1 }] += 1;
    }
    Ok(format!(
        "changed: {}\nas_written: {}\n",
        counts[0], counts[1]
    ))
}

fn label_yes(label: &str) -> f64 {
    if label == "changed" { 1.0 } else { 0.0 }
}

fn brier_score(pairs: &[(f64, f64)]) -> f64 {
    if pairs.is_empty() {
        return f64::NAN;
    }
    let sum: f64 = fsum_like_python(
        &pairs
            .iter()
            .map(|(p, y)| pow(p - y, 2.0))
            .collect::<Vec<_>>(),
    );
    sum / pairs.len() as f64
}

fn format_fixed6(x: f64) -> String {
    if x.is_nan() {
        "nan".to_string()
    } else if x.is_infinite() {
        if x > 0.0 {
            "inf".to_string()
        } else {
            "-inf".to_string()
        }
    } else {
        format!("{x:.6}")
    }
}

fn accuracy_at_threshold(pairs: &[(f64, f64)], threshold: f64) -> f64 {
    if pairs.is_empty() {
        return f64::NAN;
    }
    let correct = pairs
        .iter()
        .filter(|(p, y)| (*p >= threshold) == (*y >= 0.5))
        .count();
    correct as f64 / pairs.len() as f64
}

pub fn rank_auc(pairs: &[(f64, f64)]) -> f64 {
    let pos: Vec<f64> = pairs
        .iter()
        .filter(|(_, y)| *y >= 0.5)
        .map(|(p, _)| *p)
        .collect();
    let neg: Vec<f64> = pairs
        .iter()
        .filter(|(_, y)| *y < 0.5)
        .map(|(p, _)| *p)
        .collect();
    if pos.is_empty() || neg.is_empty() {
        return f64::NAN;
    }
    let mut wins = 0.0;
    for a in &pos {
        for b in &neg {
            wins += if a > b {
                1.0
            } else if a == b {
                0.5
            } else {
                0.0
            };
        }
    }
    wins / (pos.len() as f64 * neg.len() as f64)
}

fn reliability_bins(
    pairs: &[(f64, f64)],
    bins: usize,
) -> Vec<(String, usize, Option<f64>, Option<f64>)> {
    let width = 1.0 / bins as f64;
    let mut table = Vec::new();
    for i in 0..bins {
        let lo = i as f64 * width;
        let hi = if i == bins - 1 {
            1.0
        } else {
            (i + 1) as f64 * width
        };
        let bucket: Vec<_> = pairs
            .iter()
            .copied()
            .filter(|(p, _)| *p >= lo && (*p < hi || (i == bins - 1 && *p == 1.0)))
            .collect();
        if bucket.is_empty() {
            table.push((format!("[{lo:.1},{hi:.1})"), 0, None, None));
            continue;
        }
        let mean_pred = fsum_like_python(&bucket.iter().map(|(p, _)| *p).collect::<Vec<_>>())
            / bucket.len() as f64;
        let frac = fsum_like_python(&bucket.iter().map(|(_, y)| *y).collect::<Vec<_>>())
            / bucket.len() as f64;
        let label = if i < bins - 1 {
            format!("[{lo:.1},{hi:.1})")
        } else {
            format!("[{lo:.1},1.0]")
        };
        table.push((label, bucket.len(), Some(mean_pred), Some(frac)));
    }
    table
}

/// Score calibration for `needs_correction`.
pub fn cmd_score(path: &Path) -> Result<String> {
    let text = read_text(path)?;
    let mut pairs = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let row = hydra_pyjson::json::loads_str(line).map_err(|err| anyhow!("{err:?}"))?;
        let Value::Dict(dict) = row else {
            continue;
        };
        let label = dict.get("label").and_then(|v| match v {
            Value::Str(s) => s.to_utf8(),
            _ => None,
        });
        if !matches!(label.as_deref(), Some("changed") | Some("as_written")) {
            continue;
        }
        let answers = dict.get("answers").and_then(|v| match v {
            Value::Dict(d) => Some(d),
            _ => None,
        });
        let Some(value) = answers.and_then(|d| d.get(SCORE_QUESTION)) else {
            continue;
        };
        let p = match value {
            Value::Float(p) => *p,
            Value::Int(n) => *n as f64,
            Value::Bool(b) => f64::from(*b),
            Value::Str(s) => s
                .lossy()
                .trim_matches(hydra_pyjson::py::is_space)
                .parse::<f64>()
                .map_err(|_| anyhow!("ValueError: could not convert string to float"))?,
            _ => bail!("TypeError: float() argument must be a string or a real number"),
        };
        pairs.push((p, label_yes(label.as_deref().unwrap())));
    }

    let mut out = String::from(
        "About thirty labelled build diffs make every number below loose; treat them as directional.\n",
    );
    out.push_str(&format!("question: {SCORE_QUESTION}\n"));
    out.push_str(&format!("n: {}\n", pairs.len()));
    out.push_str(&format!("brier: {}\n", format_fixed6(brier_score(&pairs))));
    out.push_str(&format!(
        "accuracy_at_0.5: {}\n",
        format_fixed6(accuracy_at_threshold(&pairs, 0.5))
    ));
    out.push_str(&format!(
        "rank_auc: {}  (0.5 is chance; with few as_written rows it is loose)\n",
        format_fixed6(rank_auc(&pairs))
    ));
    if !pairs.is_empty() {
        let base = fsum_like_python(&pairs.iter().map(|(_, y)| *y).collect::<Vec<_>>())
            / pairs.len() as f64;
        let majority = base.max(1.0 - base);
        out.push_str(&format!(
            "base_rate_yes: {}  (always answering the majority label scores {})\n",
            format_fixed6(base),
            format_fixed6(majority)
        ));
        let brier_base = fsum_like_python(
            &pairs
                .iter()
                .map(|(_, y)| pow(base - y, 2.0))
                .collect::<Vec<_>>(),
        ) / pairs.len() as f64;
        out.push_str(&format!(
            "brier_of_the_base_rate: {}\n",
            format_fixed6(brier_base)
        ));
    }
    out.push_str("reliability:\n");
    for (bin, n, mean_pred, frac) in reliability_bins(&pairs, 5) {
        let mean_s = mean_pred
            .map(|v| format!("{v:.4}"))
            .unwrap_or_else(|| "-".into());
        let frac_s = frac
            .map(|v| format!("{v:.4}"))
            .unwrap_or_else(|| "-".into());
        out.push_str(&format!(
            "  {bin} n={n} mean_pred={mean_s} fraction_yes={frac_s}\n"
        ));
    }
    Ok(out)
}

fn expand_user(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/")
        && let Some(home) = std::env::var_os("HOME").filter(|h| !h.is_empty())
    {
        return PathBuf::from(home).join(rest);
    }
    PathBuf::from(path)
}

fn read_text(path: &Path) -> Result<String> {
    Ok(hydra_pyjson::file::read_text(path)?
        .replace("\r\n", "\n")
        .replace('\r', "\n"))
}
