//! `triage.py`: first matching cause, evidence, aggregate table and seeded sample.
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::monitor::Walk;
use anyhow::{Result, bail};
use hydra_pyjson::{
    json::{self, Dict, PyStr, Value},
    py,
};
use hydra_records::{isotime, records};

fn get(v: &Value, key: &str) -> Result<Value> {
    py::get(v, key, Value::Null)
}
fn hydra(v: &Value) -> Result<Value> {
    Ok(match get(v, "hydra")? {
        Value::Dict(d) => Value::Dict(d),
        _ => Value::Dict(Dict::new()),
    })
}
fn cut(v: &Value) -> String {
    json::dumps_default(v).chars().take(300).collect()
}

/// The last refused frame and whether no subsequent call opened or succeeded.
pub fn last_refusal(lines: &[Value]) -> Result<(Option<&Value>, bool)> {
    let mut walk = Walk::default();
    let (mut found, mut last) = (None, false);
    for line in lines {
        let before = walk.triage_counts();
        if let Value::Dict(d) = line {
            walk.observe(d)?;
        }
        let after = walk.triage_counts();
        if after.0 > before.0 {
            found = Some(line);
            last = true;
        } else if after.1 > before.1 || after.2 < before.2 {
            last = false;
        }
    }
    Ok((found, last))
}

/// Seconds between the last two parseable envelope timestamps.
pub fn gap_before_end(lines: &[Value]) -> Result<f64> {
    let mut stamps = Vec::new();
    for line in lines {
        let ts = get(line, "ts")?;
        if py::truthy(&ts) {
            let text = py::str(&ts).lossy().replace('Z', "+00:00");
            if let Some(stamp) = isotime::fromisoformat(&PyStr::from(text)) {
                stamps.push(stamp);
            }
        }
    }
    if stamps.len() < 2 {
        return Ok(0.0);
    }
    isotime::seconds_between(&stamps[stamps.len() - 2], &stamps[stamps.len() - 1]).ok_or_else(
        || anyhow::anyhow!("TypeError: can't subtract offset-naive and offset-aware datetimes"),
    )
}

/// First rule wins, in the reference's order.
pub fn get_cause(row: &Value, lines: &[Value]) -> Result<(&'static str, String)> {
    let outcome = get(row, "outcome")?;
    let mut ends = Vec::new();
    for line in lines {
        if py::eq_str(&get(&hydra(line)?, "type")?, "run_end") {
            ends.push(line);
        }
    }
    let end = ends
        .last()
        .map(|l| hydra(l))
        .transpose()?
        .unwrap_or_else(|| Value::Dict(Dict::new()));
    let refusal = get(&end, "refusal")?;
    if py::eq_str(&refusal, "skill_collision") || py::eq_str(&outcome, "skill_collision") {
        return Ok((
            "collision",
            ends.last()
                .map(|l| cut(l))
                .unwrap_or_else(|| "{\"outcome\": \"skill_collision\"}".into()),
        ));
    }
    for line in lines {
        let raw = get(line, "raw")?;
        let hit = matches!(raw, Value::Dict(_))
            && py::eq(&get(&raw, "api_error_status")?, &Value::Int(429));
        let text = json::dumps_default(line);
        if hit || (text.contains("You've hit your") && text.contains("limit")) {
            return Ok(("quota", cut(line)));
        }
    }
    if py::eq_str(&outcome, "rate_limited") {
        return Ok(("quota", "{\"outcome\": \"rate_limited\"}".into()));
    }
    if py::truthy(&refusal) {
        return Ok(("gate", cut(ends.last().unwrap())));
    }
    let (refused, last) = last_refusal(lines)?;
    if let Some(refused) = refused
        && (py::eq_str(&outcome, "gated") || last)
    {
        return Ok(("gate", cut(refused)));
    }
    let mut started = false;
    for line in lines {
        started |= matches!(get(line, "raw")?, Value::Dict(_));
    }
    if !started {
        return Ok((
            "never_started",
            lines
                .last()
                .map(cut)
                .unwrap_or_else(|| "empty record".into()),
        ));
    }
    if ends.is_empty() {
        return Ok(("provider_crash", "no run_end".into()));
    }
    let error = get(&end, "error")?;
    let code = get(&end, "exit_code")?;
    if matches!(error,Value::Str(ref s) if s.lossy().starts_with("the child died on signal "))
        || py::eq_str(&outcome, "crashed")
        || matches!(code,Value::Int(n) if n>=128)
        || matches!(code,Value::BigInt(ref n) if !n.starts_with('-'))
    {
        return Ok(("provider_crash", cut(ends.last().unwrap())));
    }
    if gap_before_end(lines)? > 600.0 {
        return Ok(("stall", cut(ends.last().unwrap())));
    }
    Ok(("unknown", cut(ends.last().unwrap())))
}

fn now() -> Result<String> {
    let elapsed = SystemTime::now().duration_since(UNIX_EPOCH)?;
    let base = isotime::DateTime::from_unix_millis(elapsed.as_secs() * 1000)
        .ok_or_else(|| anyhow::anyhow!("ValueError: year out of range"))?
        .isoformat();
    let base = base.trim_end_matches("+00:00");
    let micros = elapsed.subsec_micros();
    Ok(if micros == 0 {
        format!("{base}+00:00")
    } else {
        format!("{base}.{micros:06}+00:00")
    })
}

/// Rewrite triage.jsonl, skipping completed and live rows.
pub fn run(home: &Path, runs: &Path) -> Result<()> {
    let mut out = Vec::new();
    for row in records::read_jsonl(&home.join("dataset.jsonl"))? {
        let outcome = get(&row, "outcome")?;
        if matches!(outcome, Value::Null) || py::eq_str(&outcome, "completed") {
            continue;
        }
        let id = py::item(&row, "run_id")?.clone();
        let path = runs.join(format!("{}.jsonl", py::str(&id).lossy()));
        let lines = if path.exists() {
            let bytes = std::fs::read(&path)
                .map_err(|e| anyhow::anyhow!("{}: {e}", hydra_pyjson::file::os_error(&e)))?;
            records::committed_lines(&bytes).map_err(|e| e.raised())?.0
        } else {
            Vec::new()
        };
        let (cause, evidence) = get_cause(&row, &lines)?;
        let mut result = Dict::new();
        result.set("run_id", id);
        result.set("outcome", outcome);
        result.set("cause", Value::Str(PyStr::from(cause)));
        result.set("evidence", Value::Str(PyStr::from(evidence)));
        result.set("triaged_at", Value::Str(PyStr::from(now()?)));
        out.push(Value::Dict(result));
    }
    records::write_jsonl(&home.join("triage.jsonl"), &out)
}

/// Outcome/cause and provider/cause counts, sorted by Python's string forms.
pub fn table(home: &Path) -> Result<String> {
    if !home.join("triage.jsonl").exists() {
        return Ok(String::new());
    }
    let rows = records::read_jsonl(&home.join("triage.jsonl"))?;
    let mut providers: Vec<(Value, Value)> = Vec::new();
    for row in records::read_jsonl(&home.join("dataset.jsonl"))? {
        let id = py::item(&row, "run_id")?.clone();
        py::hashable(&id)?;
        let provider = py::get(&row, "provider", Value::Str(PyStr::from("unknown")))?;
        if let Some((_, p)) = providers.iter_mut().find(|(k, _)| py::same_key(k, &id)) {
            *p = provider;
        } else {
            providers.push((id, provider));
        }
    }
    let (mut oc, mut pc) = (Counts::default(), Counts::default());
    for r in rows {
        let outcome = get(&r, "outcome")?;
        let cause = get(&r, "cause")?;
        py::hashable(&outcome)?;
        py::hashable(&cause)?;
        let id = py::item(&r, "run_id")?;
        py::hashable(id)?;
        let provider = providers
            .iter()
            .find(|(k, _)| py::same_key(k, id))
            .map(|(_, p)| p.clone())
            .unwrap_or(Value::Str(PyStr::from("unknown")));
        py::hashable(&provider)?;
        oc.add(outcome, cause.clone());
        pc.add(provider, cause);
    }
    let mut out = String::from("Outcome and cause:\n");
    out.push_str(&oc.render());
    out.push_str("\nProvider and cause:\n");
    out.push_str(&pc.render());
    Ok(out)
}

/// Python groups by tuple equality before sorting by the string forms. Distinct keys
/// can print alike, and numeric keys can compare equal while printing differently.
#[derive(Default)]
struct Counts(Vec<(Value, Value, usize)>);

impl Counts {
    fn add(&mut self, first: Value, second: Value) {
        if let Some((_, _, n)) = self
            .0
            .iter_mut()
            .find(|(a, b, _)| py::same_key(a, &first) && py::same_key(b, &second))
        {
            *n += 1;
        } else {
            self.0.push((first, second, 1));
        }
    }

    fn render(mut self) -> String {
        self.0
            .sort_by_cached_key(|(a, b, _)| (py::str(a), py::str(b)));
        self.0
            .into_iter()
            .map(|(a, b, n)| format!("{} {}: {n}\n", py::str(&a).lossy(), py::str(&b).lossy()))
            .collect()
    }
}

/// FNV-1a UTF-8 seed, SplitMix64 draws, partial Fisher-Yates without replacement.
/// Rejection sampling avoids modulo bias.
pub fn sample(home: &Path, n: &Value, seed: &str) -> Result<String> {
    if !home.join("triage.jsonl").exists() {
        return Ok(String::new());
    }
    let mut rows = records::read_jsonl(&home.join("triage.jsonl"))?;
    let n = match n {
        Value::Int(n) if *n >= 0 => (*n as u128).min(rows.len() as u128) as usize,
        Value::BigInt(n) if !n.starts_with('-') => rows.len(),
        _ => bail!("ValueError: Sample larger than population or is negative"),
    };
    let mut state = 0xcbf29ce484222325u64;
    for b in seed.bytes() {
        state = (state ^ u64::from(b)).wrapping_mul(0x100000001b3);
    }
    let mut out = String::new();
    for i in 0..n {
        let bound = (rows.len() - i) as u64;
        let threshold = bound.wrapping_neg() % bound;
        let draw = loop {
            state = state.wrapping_add(0x9e3779b97f4a7c15);
            let mut z = state;
            z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
            z ^= z >> 31;
            if z >= threshold {
                break z % bound;
            }
        };
        rows.swap(i, i + draw as usize);
        let r = &rows[i];
        out.push_str(&format!(
            "Run {} - Cause: {}\nEvidence: {}\n\n",
            py::str(py::item(r, "run_id")?).lossy(),
            py::str(&get(r, "cause")?).lossy(),
            py::str(&get(r, "evidence")?).lossy()
        ));
    }
    Ok(out)
}
