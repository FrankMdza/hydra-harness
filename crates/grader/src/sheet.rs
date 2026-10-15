//! Corpus sheets and measured subscription budgets, ported from puppets/sheet.py.
//! Ranking remains in hydra-puppets. The sheet cache keeps the complete show answer.
use std::path::Path;

use anyhow::{Result, anyhow, bail};
use hydra_pyjson::json::{Dict, Value};
use hydra_pyjson::models::{self, Pair};
use hydra_pyjson::py;
use hydra_records::hex::hex_lower;
use sha2::{Digest, Sha256};

use crate::{collect, labels::Env};
use hydra_records::{isotime, records};

pub const POLICIES: [&str; 5] = ["judge", "read-only", "standard", "permissive", "curate"];
const CHECKLIST: &str = "secrets_check security_issues tests_affected self_consistency code_quality cross_platform changelog_accuracy approved_patch_unreachable severity_ceiling_table reviewer_failure_fails_closed panel_size_unshrunk protected_path_set admission_gate_order baseline_test_provenance env_allowlist_intact hook_config_written_fresh hook_dialect_coverage billed_spend_gate delegation_depth_cap curator_write_surface evidence_is_read_only worktree_containment record_append_only record_source_fields raw_frame_preserved export_losslessness provider_claim_dated no_new_crossing no_daemon spec_matches_code";

pub(crate) fn object(fields: impl IntoIterator<Item = (&'static str, Value)>) -> Value {
    let mut dict = Dict::new();
    for (key, value) in fields {
        dict.set(key, value);
    }
    Value::Dict(dict)
}
fn empty() -> Value {
    Value::Dict(Dict::new())
}
fn get(value: &Value, key: &str) -> Result<Value> {
    py::get(value, key, Value::Null)
}
fn or_empty(value: Value) -> Value {
    if py::truthy(&value) { value } else { empty() }
}
fn set(value: &mut Value, key: &str, item: Value) -> Result<()> {
    let Value::Dict(dict) = value else {
        bail!("TypeError: object does not support item assignment")
    };
    dict.set(key, item);
    Ok(())
}
fn number(v: &Value) -> Option<f64> {
    match v {
        Value::Int(n) => Some(*n as f64),
        Value::Float(n) => Some(*n),
        Value::BigInt(n) => n.parse().ok(),
        _ => None,
    }
}
fn finite(v: &Value) -> Option<f64> {
    number(v).filter(|n| n.is_finite())
}
pub fn filename(provider: &str, model: Option<&str>) -> String {
    fn quote(text: &str, model: bool) -> String {
        text.bytes()
            .map(|b| {
                if b.is_ascii_alphanumeric() || b"-_.".contains(&b) || b == b'~' && !model {
                    (b as char).to_string()
                } else {
                    format!("%{b:02X}")
                }
            })
            .collect()
    }
    format!(
        "{}--{}.json",
        quote(provider, false),
        model.map_or_else(|| "~null".into(), |m| quote(m, true))
    )
}
fn pair(row: &Value, aliases: &[(Pair, Pair)]) -> Result<Pair> {
    models::canonical(py::item(row, "provider")?, &get(row, "model")?, aliases)
}
fn same_pair(a: &Pair, b: &Pair) -> bool {
    py::same_key(&a.0, &b.0) && py::same_key(&a.1, &b.1)
}
fn sheet_name(pair: &Pair) -> Result<String> {
    let Value::Str(provider) = &pair.0 else {
        bail!("TypeError: provider is not text")
    };
    let model = match &pair.1 {
        Value::Null => None,
        Value::Str(s) => Some(
            s.to_utf8()
                .ok_or_else(|| anyhow!("UnicodeEncodeError: model"))?,
        ),
        _ => bail!("TypeError: model is not text"),
    };
    Ok(filename(
        &provider
            .to_utf8()
            .ok_or_else(|| anyhow!("UnicodeEncodeError: provider"))?,
        model.as_deref(),
    ))
}
fn statistic(values: impl IntoIterator<Item = Value>) -> Value {
    let mut values: Vec<Value> = values.into_iter().filter(|v| finite(v).is_some()).collect();
    values.sort_by(|a, b| finite(a).unwrap().partial_cmp(&finite(b).unwrap()).unwrap());
    let n = values.len();
    let median = if n == 0 {
        Value::Null
    } else if n % 2 == 1 {
        values[n / 2].clone()
    } else {
        Value::Float((number(&values[n / 2 - 1]).unwrap() + number(&values[n / 2]).unwrap()) / 2.0)
    };
    object([("p50", median), ("n", Value::Int(n as i128))])
}
fn counter(values: impl IntoIterator<Item = Value>) -> Result<Value> {
    let mut dict = Dict::new();
    for v in values {
        py::hashable(&v)?;
        let key = py::str(&v);
        let n = match dict.get_key(&key) {
            Some(Value::Int(n)) => *n,
            _ => 0,
        };
        dict.set_key(key, Value::Int(n + 1));
    }
    Ok(Value::Dict(dict))
}
fn utilization(value: &Value) -> Option<f64> {
    let Value::Dict(dict) = value else {
        return None;
    };
    if let Some(Value::Dict(window)) = dict.get("seven_day")
        && let Some(found) = window.get("utilization").and_then(finite)
    {
        return Some(found);
    }
    dict.iter().find_map(|(_, v)| utilization(v))
}
fn judge_parsed(value: &Value) -> bool {
    let Value::Str(text) = value else {
        return false;
    };
    let mut text = py::strip(text).lossy();
    if let Some(body) = text.strip_prefix("```")
        && let Some((tag, body)) = body.split_once('\n')
        && matches!(py::strip_str(tag).to_lowercase().as_str(), "" | "json")
        && let Some(body) = py::strip_str(body).strip_suffix("```")
        && !body.contains("```")
    {
        text = py::strip_str(body).into();
    }
    let Ok(Value::Dict(obj)) = hydra_pyjson::json::loads_str(&text) else {
        return false;
    };
    let Some(Value::Str(verdict)) = obj.get("verdict") else {
        return false;
    };
    let Some(Value::List(findings)) = obj.get("findings") else {
        return false;
    };
    matches!(verdict.lossy().as_str(), "pass" | "block") && findings.iter().all(|finding| {
        let Value::Dict(f) = finding else { return false };
        matches!((f.get("item"), f.get("severity"), f.get("note")),
            (Some(Value::Str(item)), Some(Value::Str(severity)), Some(Value::Str(_)))
            if CHECKLIST.split_whitespace().any(|k| item == k) && matches!(severity.lossy().as_str(), "critical" | "major" | "minor"))
    })
}
fn stamp(v: &Value) -> Result<isotime::DateTime> {
    let Value::Str(s) = v else {
        bail!("AttributeError: timestamp has no replace")
    };
    isotime::fromisoformat(&s.lossy().replace('Z', "+00:00").into())
        .ok_or_else(|| anyhow!("ValueError: Invalid isoformat string"))
}

pub fn corpus_records(runs: &Path, home: &Path, hydra: &str, env: &Env) -> Result<Vec<Value>> {
    if !runs.is_dir() {
        bail!(
            "ValueError: runs directory does not exist: {}",
            runs.display()
        )
    }
    let cache_path = home.join("sheet-cache.json");
    let mut cache = records::read_json(&cache_path)?.unwrap_or_else(empty);
    let mut paths = std::fs::read_dir(runs)?
        .map(|e| e.map(|e| e.path()))
        .collect::<std::io::Result<Vec<_>>>()?;
    paths.retain(|p| p.extension().is_some_and(|e| e == "jsonl"));
    paths.sort();
    let mut result: Vec<Value> = Vec::new();
    for path in paths {
        let data = std::fs::read(&path)?;
        let (lines, _) = records::committed_lines(&data)
            .map_err(|err| anyhow!("{}: {}", err.error.class(), err.message))?;
        let Some(first) = lines.first() else { continue };
        let start = py::get(first, "hydra", empty())?;
        if !py::truthy(&get(first, "provider")?)
            || !matches!(
                py::str(&get(&start, "command")?).lossy().as_str(),
                "run" | "resume"
            )
        {
            continue;
        }
        let id = py::item(first, "run_id")?;
        let Value::Str(id_text) = id else {
            bail!("TypeError: run id must be text")
        };
        let fingerprint = Value::text(&hex_lower(&Sha256::digest(&data)));
        let mut entry = py::get(&cache, &id_text.lossy(), empty())?;
        if !py::eq(&get(&entry, "fingerprint")?, &fingerprint) {
            let answer = collect::refresh_full_show_answer(id, runs, hydra, env)?;
            entry = object([("fingerprint", fingerprint), ("answer", answer)]);
            set(&mut cache, &id_text.lossy(), entry.clone())?;
        }
        let answer = py::item(&entry, "answer")?;
        let projection = or_empty(get(answer, "projection")?);
        let usage = or_empty(get(&projection, "usage")?);
        let end = lines
            .iter()
            .rev()
            .find_map(|v| {
                let hydra = py::get(v, "hydra", empty());
                match hydra {
                    Ok(h) if get(&h, "type").is_ok_and(|t| py::eq_str(&t, "run_end")) => {
                        Some(Ok(h))
                    }
                    Err(e) => Some(Err(e)),
                    _ => None,
                }
            })
            .transpose()?
            .unwrap_or_else(empty);
        let ended = get(&end, "ended_at")?;
        let duration = if py::truthy(&ended) {
            Value::Float(
                isotime::seconds_between(&stamp(py::item(first, "ts")?)?, &stamp(&ended)?)
                    .ok_or_else(|| {
                        anyhow!("TypeError: can't subtract offset-naive and offset-aware datetimes")
                    })?,
            )
        } else {
            Value::Null
        };
        let rates: Vec<f64> = lines
            .iter()
            .filter_map(|v| get(v, "raw").ok())
            .filter(|v| get(v, "type").is_ok_and(|t| py::eq_str(&t, "rate_limit_event")))
            .filter_map(|v| utilization(&v))
            .collect();
        let share = if rates.len() >= 2 && rates.last() >= rates.first() {
            Value::Float(rates[rates.len() - 1] - rates[0])
        } else {
            Value::Null
        };
        let policy = get(&start, "policy")?;
        let events = py::list(&py::get(answer, "events", Value::List(vec![]))?)?;
        let denials = events
            .iter()
            .map(|v| {
                Ok(py::eq_str(&get(v, "type")?, "permission_decision")
                    && py::eq_str(&get(v, "decision")?, "deny"))
            })
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .filter(|v| *v)
            .count();
        let parent = get(first, "parent_run_id")?;
        let row = object([
            ("run_id", id.clone()),
            ("provider", py::item(first, "provider")?.clone()),
            ("model", get(first, "model")?),
            (
                "parent_run_id",
                if py::truthy(&parent) {
                    parent
                } else {
                    get(&start, "resumed_from")?
                },
            ),
            ("ts", py::item(first, "ts")?.clone()),
            ("ended_at", ended),
            ("policy", policy.clone()),
            ("subscription", get(&start, "subscription")?),
            ("provider_version", get(&start, "provider_version")?),
            ("launch_mode", get(&start, "launch_mode")?),
            ("duration_seconds", duration),
            ("outcome", get(&projection, "outcome")?),
            (
                "usage_source",
                py::get(&usage, "usage_source", Value::text("unavailable"))?,
            ),
            ("tokens", or_empty(get(&usage, "counts")?)),
            ("denials", Value::Int(denials as i128)),
            (
                "judge_parsed",
                if py::eq_str(&policy, "judge") {
                    Value::Bool(judge_parsed(&get(&projection, "result_text")?))
                } else {
                    Value::Null
                },
            ),
            ("window_share", share),
        ]);
        if let Some(slot) = result
            .iter_mut()
            .find(|r| get(r, "run_id").is_ok_and(|v| py::same_key(&v, id)))
        {
            *slot = row
        } else {
            result.push(row)
        }
    }
    records::write_json(&cache_path, &cache)?;
    Ok(result)
}

pub fn build(
    runs: &Path,
    home: &Path,
    vendors: &Value,
    hydra: &str,
    weight: f64,
    aliases: &[(Pair, Pair)],
    env: &Env,
) -> Result<Vec<Value>> {
    if !weight.is_finite() || weight < 0.0 {
        bail!("ValueError: cache_read_weight must be finite and nonnegative")
    }
    let vendors = py::list(vendors)?;
    let mut rows = corpus_records(runs, home, hydra, env)?;
    let originals = rows.clone();
    for row in &mut rows {
        let mut root = row.clone();
        let mut seen = vec![get(row, "run_id")?];
        loop {
            let parent = get(&root, "parent_run_id")?;
            if seen.iter().any(|v| py::same_key(v, &parent)) {
                break;
            }
            let Some(next) = originals
                .iter()
                .find(|r| get(r, "run_id").is_ok_and(|v| py::same_key(&v, &parent)))
            else {
                break;
            };
            seen.push(parent);
            root = next.clone();
        }
        let parent = get(&root, "parent_run_id")?;
        set(
            row,
            "task_id",
            if py::truthy(&parent) {
                parent
            } else {
                get(&root, "run_id")?
            },
        )?;
    }
    let mut groups: Vec<Pair> = Vec::new();
    for row in vendors.iter().chain(rows.iter()) {
        let key = pair(row, aliases)?;
        if !groups.iter().any(|k| same_pair(k, &key)) {
            groups.push(key)
        }
    }
    groups.sort_by_key(|(p, m)| {
        (
            py::str(p).lossy(),
            if py::truthy(m) {
                py::str(m).lossy()
            } else {
                String::new()
            },
        )
    });
    let mut sheets = Vec::new();
    for key in groups {
        let mut group = rows
            .iter()
            .filter_map(|r| match pair(r, aliases) {
                Ok(k) if same_pair(&k, &key) => Some(Ok(r.clone())),
                Err(e) => Some(Err(e)),
                _ => None,
            })
            .collect::<Result<Vec<_>>>()?;
        let vendor = vendors
            .iter()
            .find_map(|v| {
                pair(v, aliases)
                    .ok()
                    .filter(|k| same_pair(k, &key))
                    .map(|_| v.clone())
            })
            .unwrap_or_else(empty);
        let mut facts = Dict::new();
        for k in [
            "as_of",
            "source_url",
            "context_tokens",
            "thinking_levels",
            "plan",
            "windows",
            "window_budget_tokens",
        ] {
            facts.set(k, get(&vendor, k)?);
        }
        let budget = facts.get("window_budget_tokens").unwrap();
        let mut spellings = Vec::new();
        let mut tasks = Vec::new();
        for row in &mut group {
            let model = get(row, "model")?;
            if !spellings.iter().any(|m| py::same_key(m, &model)) {
                spellings.push(model)
            }
            let task = get(row, "task_id")?;
            if !tasks.iter().any(|t| py::same_key(t, &task)) {
                tasks.push(task)
            }
            if !py::eq_str(&key.0, "claude") {
                let tokens = get(row, "tokens")?;
                let share = match (
                    number(budget),
                    get(&tokens, "raw_input")?,
                    get(&tokens, "output_tokens")?,
                ) {
                    (Some(b), raw, out)
                        if b > 0.0
                            && !matches!(raw, Value::Null)
                            && !matches!(out, Value::Null) =>
                    {
                        let raw = number(&raw)
                            .ok_or_else(|| anyhow!("TypeError: token count is not numeric"))?;
                        let out = number(&out)
                            .ok_or_else(|| anyhow!("TypeError: token count is not numeric"))?;
                        Value::Float((raw * weight + out) / b)
                    }
                    _ => Value::Null,
                };
                set(row, "window_share", share)?;
            }
        }
        spellings.sort_by_key(|m| {
            if py::truthy(m) {
                py::str(m).lossy()
            } else {
                String::new()
            }
        });
        let stat = |k: &str| -> Result<Value> {
            Ok(statistic(
                group
                    .iter()
                    .map(|r| get(r, k))
                    .collect::<Result<Vec<_>>>()?,
            ))
        };
        let mut token_stats = Dict::new();
        for k in ["raw_input", "output_tokens", "cache_read_tokens"] {
            token_stats.set(
                k,
                statistic(
                    group
                        .iter()
                        .map(|r| get(&get(r, "tokens")?, k))
                        .collect::<Result<Vec<_>>>()?,
                ),
            );
        }
        let defaults = Value::List(
            POLICIES
                .iter()
                .filter(|p| !(py::eq_str(&key.0, "codex") && **p == "read-only"))
                .map(|p| Value::text(p))
                .collect(),
        );
        let sheet = object([
            ("provider", key.0.clone()),
            ("model", key.1.clone()),
            ("recorded_models", Value::List(spellings)),
            ("vendor", Value::Dict(facts)),
            (
                "available",
                py::get(&vendor, "available", Value::Bool(true))?,
            ),
            ("policies", py::get(&vendor, "policies", defaults)?),
            ("cache_read_weight", Value::Float(weight)),
            ("runs", Value::List(group.clone())),
            ("sample_size", Value::Int(group.len() as i128)),
            ("independent_tasks", Value::Int(tasks.len() as i128)),
            ("duration_seconds", stat("duration_seconds")?),
            ("cost", stat("window_share")?),
            ("denials", stat("denials")?),
            ("tokens", Value::Dict(token_stats)),
            (
                "usage_sources",
                counter(
                    group
                        .iter()
                        .map(|r| get(r, "usage_source"))
                        .collect::<Result<Vec<_>>>()?,
                )?,
            ),
            (
                "outcomes",
                counter(
                    group
                        .iter()
                        .map(|r| {
                            get(r, "outcome").map(|v| {
                                if py::truthy(&v) {
                                    v
                                } else {
                                    Value::text("unknown")
                                }
                            })
                        })
                        .collect::<Result<Vec<_>>>()?,
                )?,
            ),
            (
                "judge",
                object([
                    (
                        "n",
                        Value::Int(
                            group
                                .iter()
                                .filter(|r| {
                                    get(r, "judge_parsed").is_ok_and(|v| !matches!(v, Value::Null))
                                })
                                .count() as i128,
                        ),
                    ),
                    (
                        "parsed",
                        Value::Int(
                            group
                                .iter()
                                .filter(|r| {
                                    get(r, "judge_parsed")
                                        .is_ok_and(|v| matches!(v, Value::Bool(true)))
                                })
                                .count() as i128,
                        ),
                    ),
                ]),
            ),
        ]);
        records::write_json(&home.join("agents").join(sheet_name(&key)?), &sheet)?;
        sheets.push(sheet);
    }
    let names = sheets
        .iter()
        .map(|s| sheet_name(&pair(s, &[])?))
        .collect::<Result<Vec<_>>>()?;
    if home.join("agents").is_dir() {
        for entry in std::fs::read_dir(home.join("agents"))? {
            let path = entry?.path();
            if path.extension().is_some_and(|e| e == "json")
                && !names
                    .iter()
                    .any(|n| path.file_name().is_some_and(|p| p == n.as_str()))
            {
                std::fs::remove_file(path)?;
            }
        }
    }
    Ok(sheets)
}

fn spend(counts: &Value, weight: f64) -> Result<Option<f64>> {
    if !matches!(counts, Value::Dict(_)) {
        return Ok(None);
    }
    Ok(
        match (
            finite(&get(counts, "raw_input")?),
            finite(&get(counts, "output_tokens")?),
        ) {
            (Some(raw), Some(out)) => Some(raw * weight + out),
            _ => None,
        },
    )
}
fn window_seconds(vendor: &Value, provider: &Value) -> Result<f64> {
    let windows = get(vendor, "windows")?;
    let value = match windows {
        Value::Str(s) => {
            let text = py::strip(&s).lossy().to_lowercase();
            if let Some(n) = text.strip_suffix('d') {
                Some((Value::text(n), 86400.0))
            } else {
                text.strip_suffix('h').map(|n| (Value::text(n), 3600.0))
            }
        }
        Value::Dict(_) => {
            let days = get(&windows, "days")?;
            if !matches!(days, Value::Null) {
                Some((days, 86400.0))
            } else {
                let hours = get(&windows, "hours")?;
                (!matches!(hours, Value::Null)).then_some((hours, 3600.0))
            }
        }
        _ => None,
    };
    let Some((n, multiplier)) = value else {
        bail!(
            "ValueError: no window for {} in vendors.json; set its \"windows\" (\"5h\", \"7d\") or pass --since",
            py::str(provider).lossy()
        )
    };
    let n = match n {
        Value::Str(s) => py::float_literal(&s.lossy()),
        Value::Bool(b) => Some(if b { 1.0 } else { 0.0 }),
        v => number(&v),
    }
    .ok_or_else(|| anyhow!("ValueError: could not convert window to float"))?;
    if !n.is_finite() {
        bail!("OverflowError: cannot convert window to timedelta")
    }
    // timedelta rounds fractional microseconds half to even.
    Ok((n * multiplier * 1e6).round_ties_even() / 1e6)
}

pub struct Budget<'a> {
    pub run_id: &'a str,
    pub runs: &'a Path,
    pub home: &'a Path,
    pub vendors: &'a Path,
    pub hydra: &'a str,
    pub weight: f64,
    pub since: Option<&'a str>,
}
pub fn revise_budget(
    args: &Budget<'_>,
    aliases: &[(Pair, Pair)],
    env: &Env,
) -> Result<Option<String>> {
    if !args.weight.is_finite() || args.weight < 0.0 {
        bail!("ValueError: cache_read_weight must be finite and nonnegative")
    }
    let rows = corpus_records(args.runs, args.home, args.hydra, env)?;
    let trigger = rows
        .iter()
        .find(|r| get(r, "run_id").is_ok_and(|v| py::eq_str(&v, args.run_id)))
        .ok_or_else(|| anyhow!("ValueError: no such run in corpus: {}", args.run_id))?;
    if !py::eq_str(&get(trigger, "outcome")?, "rate_limited") {
        bail!("ValueError: run {} is not rate_limited", args.run_id)
    }
    if py::eq_str(&get(trigger, "provider")?, "claude") {
        bail!("ValueError: claude window budgets come from utilization, not token spend")
    }
    let ended = get(trigger, "ended_at")?;
    if !py::truthy(&ended) {
        bail!("ValueError: run {} has no run_end", args.run_id)
    }
    let key = pair(trigger, aliases)?;
    let mut vendors = py::list(&records::read_json(args.vendors)?.unwrap_or(Value::Null))?;
    let vendor = vendors
        .iter_mut()
        .find(|v| pair(v, aliases).is_ok_and(|k| same_pair(&k, &key)))
        .ok_or_else(|| {
            anyhow!(
                "ValueError: no vendor row for {} model {}",
                py::repr(&key.0),
                py::repr(&key.1)
            )
        })?;
    let end = stamp(&ended)?;
    let duration = match args.since {
        Some(s) => isotime::seconds_between(&stamp(&Value::text(s))?, &end).ok_or_else(|| {
            anyhow!("TypeError: can't subtract offset-naive and offset-aware datetimes")
        })?,
        None => window_seconds(vendor, &get(trigger, "provider")?)?,
    };
    let mut ids = Vec::new();
    let mut total = 0.0;
    for row in &rows {
        if !same_pair(&pair(row, aliases)?, &key) || !py::truthy(&get(row, "ended_at")?) {
            continue;
        }
        let start_delta =
            isotime::seconds_between(&end, &stamp(&get(row, "ts")?)?).ok_or_else(|| {
                anyhow!("TypeError: can't compare offset-naive and offset-aware datetimes")
            })?;
        let end_delta = isotime::seconds_between(&end, &stamp(&get(row, "ended_at")?)?)
            .ok_or_else(|| {
                anyhow!("TypeError: can't compare offset-naive and offset-aware datetimes")
            })?;
        if end_delta < -duration || start_delta > 0.0 {
            continue;
        }
        let Some(tokens) = spend(&get(row, "tokens")?, args.weight)? else {
            continue;
        };
        ids.push(get(row, "run_id")?);
        total += tokens;
    }
    ids.sort_by_key(|v| py::str(v).lossy());
    if ids.is_empty() {
        bail!("ValueError: no token spend in window for {}", args.run_id)
    }
    let mut history = py::list(&py::get(vendor, "budget_history", Value::List(vec![]))?)?;
    let previous = get(vendor, "window_budget_tokens")?;
    let new = Value::Float(total);
    if py::eq(&previous, &new)
        && let Some(last) = history.last()
    {
        let mut last_ids = py::list(&py::get(last, "run_ids", Value::List(vec![]))?)?;
        last_ids.sort_by_key(|v| py::str(v).lossy());
        if py::eq(&get(last, "new")?, &new)
            && py::eq(&Value::List(last_ids), &Value::List(ids.clone()))
        {
            return Ok(None);
        }
    }
    history.push(object([
        ("previous", previous.clone()),
        ("new", new.clone()),
        ("date", Value::text(&end.today_in_zone())),
        ("run_ids", Value::List(ids.clone())),
        ("outside_spend_possible", Value::Bool(true)),
    ]));
    set(vendor, "budget_history", Value::List(history))?;
    set(vendor, "window_budget_tokens", new.clone())?;
    records::write_json(args.vendors, &Value::List(vendors))?;
    Ok(Some(format!(
        "{} window_budget_tokens: {} -> {} (runs: {}; outside_spend_possible: true)",
        py::str(&get(trigger, "provider")?).lossy(),
        py::str(&previous).lossy(),
        py::str(&new).lossy(),
        ids.iter()
            .map(|v| py::str(v).lossy())
            .collect::<Vec<_>>()
            .join(", ")
    )))
}
