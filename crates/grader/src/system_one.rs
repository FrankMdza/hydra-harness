//! System 1 v0 (build ticket 289): a logistic head over a task embedding plus the
//! record features `baseline.rs` already uses, with a temperature fitted on a later
//! slice in time.
//!
//! v0 embeds the task text only. Ticket 288 found the original diff for 7 of 70
//! verdicts; the rest of the branches were deleted. The head answers `outcome`.
//! Embeddings come from Ollama `/api/embed` through `loopback_http`, so nothing
//! leaves this machine. The model name is `HYDRA_EMBED_MODEL`, default
//! `nomic-embed-text`.

use std::collections::HashMap;
use std::time::Duration;

use anyhow::{Result, anyhow, bail};
use hydra_pyjson::json::{self, Dict, PyStr, Value};
use hydra_pyjson::math::{self, fsum_like_python};
use hydra_pyjson::py;
use hydra_records::isotime::DateTime;
use hydra_records::loopback_http::{self, Failure};

use crate::baseline::{self, Features, Label, Refusal};
use crate::judge;
use crate::labels::Env;

/// Floor on the rarer class in the fit slice and in the test slice. Lower than the
/// baseline's ten because v0 trains on tens of rows, not hundreds.
const MIN_MINORITY: usize = 3;
const TIMEOUT: Duration = Duration::from_secs(30);
const TEMPERATURE_STEPS: usize = 3000;
const TEMPERATURE_RATE: f64 = 0.5;
const TEMPERATURE_MIN: f64 = 0.05;
const TEMPERATURE_MAX: f64 = 20.0;

pub const DEFAULT_MODEL: &str = "nomic-embed-text";

/// `os.environ.get(name, default)`: a variable set to nothing is nothing, not the default.
fn named(env: &Env, name: &str, default: &str) -> String {
    env.get(name)
        .map_or_else(|| default.to_string(), |v| v.to_string_lossy().into_owned())
}

/// The embed URL: `HYDRA_LOCAL_LLM_URL` with `/api/generate` rewritten to `/api/embed`.
pub fn embed_url(env: &Env) -> String {
    rewrite_embed_url(&named(env, "HYDRA_LOCAL_LLM_URL", judge::DEFAULT_URL))
}

pub fn embed_model(env: &Env) -> String {
    named(env, "HYDRA_EMBED_MODEL", DEFAULT_MODEL)
}

pub(crate) fn rewrite_embed_url(url: &str) -> String {
    if url.contains("/api/embed") {
        return url.to_string();
    }
    if let Some(prefix) = url.strip_suffix("/api/generate") {
        return format!("{prefix}/api/embed");
    }
    let trimmed = url.trim_end_matches('/');
    format!("{trimmed}/api/embed")
}

fn task_text(row: &Value) -> Result<String> {
    let text = py::get(row, "task_text", Value::Null)?;
    if py::truthy(&text) {
        Ok(py::str(&text).lossy())
    } else {
        Ok(String::new())
    }
}

fn run_id_text(row: &Value) -> Result<String> {
    Ok(py::str(py::item(row, "run_id")?).lossy())
}

fn provider_key(row: &Value) -> Result<Value> {
    let provider = py::get(row, "provider", Value::Null)?;
    py::hashable(&provider)?;
    Ok(provider)
}

/// L2-normalise so the embedding's length does not dwarf the one-hot features.
pub(crate) fn l2_normalize(values: &[f64]) -> Vec<f64> {
    let squares: Vec<f64> = values.iter().map(|v| math::pow(*v, 2.0)).collect();
    let norm = math::sqrt(fsum_like_python(&squares));
    if norm == 0.0 {
        values.to_vec()
    } else {
        values.iter().map(|v| v / norm).collect()
    }
}

fn floats_of(list: &[Value]) -> Result<Vec<f64>> {
    list.iter().map(baseline::number).collect()
}

/// Read Ollama's `/api/embed` body, or the older singular `embedding` field.
pub(crate) fn parse_embedding(raw: &[u8]) -> Result<Vec<f64>> {
    let text = String::from_utf8_lossy(raw).into_owned();
    let reply = json::loads_explained(&PyStr::from(text.as_str()))
        .map_err(|(err, message)| anyhow!("{}: {message}", err.class()))?;
    let embeddings = py::get(&reply, "embeddings", Value::Null)?;
    if py::truthy(&embeddings) {
        let items = py::list(&embeddings)?;
        let Some(first) = items.first() else {
            bail!("ValueError: Ollama returned no embedding");
        };
        return floats_of(&py::list(first)?);
    }
    let embedding = py::get(&reply, "embedding", Value::Null)?;
    if py::truthy(&embedding) {
        return floats_of(&py::list(&embedding)?);
    }
    bail!("ValueError: Ollama returned no embedding")
}

fn request_body(model: &str, text: &str) -> String {
    let mut body = Dict::new();
    body.set("model", Value::text(model));
    body.set("input", Value::text(text));
    json::dumps_default(&Value::Dict(body))
}

/// One `/api/embed` call. The URL must name this machine.
pub fn embed_task(text: &str, model: &str, url: &str) -> Result<Vec<f64>> {
    let body = request_body(model, text);
    let unavailable = |said: String| -> Result<Vec<f64>> { Err(Refusal(said).into()) };
    let raw = match loopback_http::post_json(url, body.as_bytes(), TIMEOUT) {
        Ok(raw) => raw,
        Err(Failure::Status(404, _)) => {
            return unavailable(format!("Ollama does not have the model {model}"));
        }
        Err(Failure::Status(code, _)) => {
            return unavailable(format!("Ollama at {url} answered HTTP {code}"));
        }
        Err(Failure::NotAnswering) => {
            return unavailable(format!("Ollama is not answering at {url}"));
        }
        Err(Failure::NotLocal(why)) => {
            return unavailable(format!(
                "the model server has to be on this machine and {why}"
            ));
        }
        Err(Failure::Raised(text)) => bail!("{text}"),
    };
    parse_embedding(&raw)
}

/// `sigmoid(z / T)` log loss, `T = exp(log_t)` so T stays positive.
pub(crate) fn fit_temperature(zs: &[f64], ys: &[u8]) -> f64 {
    if zs.is_empty() {
        return 1.0;
    }
    let n = zs.len() as f64;
    let mut log_t = 0.0;
    for _ in 0..TEMPERATURE_STEPS {
        let t = math::exp(log_t).clamp(TEMPERATURE_MIN, TEMPERATURE_MAX);
        let mut grad = 0.0;
        for (z, y) in zs.iter().zip(ys) {
            let p = baseline::sigmoid(*z / t);
            grad += (p - f64::from(*y)) * (-z / t);
        }
        log_t -= TEMPERATURE_RATE * grad / n;
    }
    math::exp(log_t).clamp(TEMPERATURE_MIN, TEMPERATURE_MAX)
}

fn minority(part: &[(&Value, u8)]) -> usize {
    let ones = baseline::positives(part);
    ones.min(part.len().saturating_sub(ones))
}

fn refuse_minority(name: &str, part: &[(&Value, u8)], labelled: usize) -> Result<()> {
    let rare = minority(part);
    if part.is_empty() || rare < MIN_MINORITY {
        return Err(Refusal(format!(
            "system-one: {labelled} labelled rows for 'outcome'; the {name} slice has {} of them \
             and {rare} of its rarer class, and {MIN_MINORITY} is the floor.",
            part.len()
        ))
        .into());
    }
    Ok(())
}

type Labelled<'a> = Vec<(&'a Value, u8)>;

/// Earlier half of `fit` trains the head; the later half fits the temperature.
fn split_fit<'a>(fit: &[(&'a Value, u8)]) -> Result<(Labelled<'a>, Labelled<'a>)> {
    let mut with_time: Vec<(i128, &Value, u8)> = Vec::with_capacity(fit.len());
    for (row, y) in fit {
        with_time.push((baseline::launched_at(row)?, *row, *y));
    }
    with_time.sort_by_key(|(at, _, _)| *at);
    let mid = with_time.len() / 2;
    let mut early = Vec::new();
    let mut later = Vec::new();
    for (i, (_, row, y)) in with_time.into_iter().enumerate() {
        if i < mid {
            early.push((row, y));
        } else {
            later.push((row, y));
        }
    }
    Ok((early, later))
}

fn scale_for(feats: &[Features]) -> Result<Vec<(PyStr, Vec<Value>)>> {
    let n = feats.len() as f64;
    if n == 0.0 {
        bail!("ZeroDivisionError: division by zero");
    }
    let values: Vec<f64> = feats.iter().map(|f| f.log_task_chars).collect();
    let mean = fsum_like_python(&values) / n;
    let spread: Vec<f64> = values.iter().map(|v| math::pow(v - mean, 2.0)).collect();
    let mut std = math::sqrt(fsum_like_python(&spread) / n);
    if std == 0.0 {
        std = 1.0;
    }
    Ok(vec![(
        PyStr::from("log_task_chars"),
        vec![Value::Float(mean), Value::Float(std)],
    )])
}

fn vocab_of(feats: &[Features]) -> Vec<Value> {
    let mut vocab: Vec<PyStr> = feats.iter().flat_map(Features::names).collect();
    vocab.sort();
    vocab.dedup();
    vocab.into_iter().map(Value::Str).collect()
}

fn row_vector(
    row: &Value,
    embedding: &[f64],
    vocab: &[Value],
    scale: &[(PyStr, Vec<Value>)],
) -> Result<Vec<f64>> {
    let feats = Features::of(row)?;
    let mut x = baseline::vector(&|name| feats.value(name), vocab, scale)?;
    x.extend_from_slice(&l2_normalize(embedding));
    Ok(x)
}

fn measures(ps: &[f64], ys: &[u8]) -> Value {
    let mut out = Dict::new();
    out.set("log_loss", Value::Float(baseline::log_loss(ps, ys)));
    out.set("brier", Value::Float(baseline::brier(ps, ys)));
    out.set(
        "rank_auc",
        baseline::rank_auc(ps, ys).map_or(Value::Null, Value::Float),
    );
    Value::Dict(out)
}

struct ProviderRates {
    overall: f64,
    by_provider: Vec<(Value, f64)>,
}

impl ProviderRates {
    fn of(fit: &[(&Value, u8)]) -> Result<ProviderRates> {
        let overall = baseline::positives(fit) as f64 / fit.len() as f64;
        let mut counts: Vec<(Value, usize, usize)> = Vec::new();
        for (row, y) in fit {
            let provider = provider_key(row)?;
            match counts
                .iter()
                .position(|(known, _, _)| py::same_key(known, &provider))
            {
                Some(at) => {
                    counts[at].1 += usize::from(*y);
                    counts[at].2 += 1;
                }
                None => counts.push((provider, usize::from(*y), 1)),
            }
        }
        Ok(ProviderRates {
            overall,
            by_provider: counts
                .into_iter()
                .map(|(provider, ones, n)| (provider, ones as f64 / n as f64))
                .collect(),
        })
    }

    fn rate(&self, row: &Value) -> Result<f64> {
        let provider = provider_key(row)?;
        Ok(self
            .by_provider
            .iter()
            .find(|(known, _)| py::same_key(known, &provider))
            .map_or(self.overall, |(_, rate)| *rate))
    }
}

/// The fitted weights, for tests that check the time split did not train on the test slice.
#[allow(dead_code)]
pub(crate) struct Head {
    pub weights: Vec<f64>,
    pub bias: f64,
    pub temperature: f64,
    pub embed_dim: usize,
}

fn logits(weights: &[f64], bias: f64, xs: &[Vec<f64>]) -> Vec<f64> {
    let mut products = Vec::new();
    xs.iter()
        .map(|x| baseline::logit(weights, bias, x, &mut products))
        .collect()
}

fn scored(zs: &[f64], temperature: f64) -> Vec<f64> {
    zs.iter()
        .map(|z| baseline::sigmoid(*z / temperature))
        .collect()
}

/// Fit and report given an embedding for each labelled row, keyed by `run_id`.
pub(crate) fn report_with_embeddings(
    rows: &[Value],
    test_from: &DateTime,
    embed_model: &str,
    embeddings: &HashMap<String, Vec<f64>>,
) -> Result<(Head, Value)> {
    let parts = baseline::slices(rows, Label::Outcome, test_from)?;
    let (early, later) = split_fit(&parts.fit)?;
    refuse_minority("fit", &early, parts.labelled)?;
    refuse_minority("test", &parts.test, parts.labelled)?;

    let dim_of = |row: &Value| -> Result<Vec<f64>> {
        let id = run_id_text(row)?;
        embeddings
            .get(&id)
            .cloned()
            .ok_or_else(|| anyhow!("KeyError: no embedding for {id}"))
    };
    let early_emb: Vec<Vec<f64>> = early
        .iter()
        .map(|(row, _)| dim_of(row))
        .collect::<Result<_>>()?;
    let later_emb: Vec<Vec<f64>> = later
        .iter()
        .map(|(row, _)| dim_of(row))
        .collect::<Result<_>>()?;
    let test_emb: Vec<Vec<f64>> = parts
        .test
        .iter()
        .map(|(row, _)| dim_of(row))
        .collect::<Result<_>>()?;
    let dim = early_emb[0].len();
    if dim == 0 {
        bail!("ValueError: Ollama returned no embedding");
    }
    for (name, group) in [
        ("fit", early_emb.as_slice()),
        ("calib", later_emb.as_slice()),
        ("test", test_emb.as_slice()),
    ] {
        if group.iter().any(|e| e.len() != dim) {
            bail!("ValueError: embedding length changed on the {name} slice");
        }
    }

    let feats: Vec<Features> = early
        .iter()
        .map(|(row, _)| Features::of(row))
        .collect::<Result<_>>()?;
    let vocab = vocab_of(&feats);
    let scale = scale_for(&feats)?;
    let xs_early: Vec<Vec<f64>> = early
        .iter()
        .zip(&early_emb)
        .map(|((row, _), emb)| row_vector(row, emb, &vocab, &scale))
        .collect::<Result<_>>()?;
    let ys_early: Vec<u8> = early.iter().map(|(_, y)| *y).collect();
    let (weights, bias) = baseline::fit(&xs_early, &ys_early);

    let xs_later: Vec<Vec<f64>> = later
        .iter()
        .zip(&later_emb)
        .map(|((row, _), emb)| row_vector(row, emb, &vocab, &scale))
        .collect::<Result<_>>()?;
    let ys_later: Vec<u8> = later.iter().map(|(_, y)| *y).collect();
    let temperature = if minority(&later) == 0 || later.is_empty() {
        1.0
    } else {
        fit_temperature(&logits(&weights, bias, &xs_later), &ys_later)
    };

    let xs_test: Vec<Vec<f64>> = parts
        .test
        .iter()
        .zip(&test_emb)
        .map(|((row, _), emb)| row_vector(row, emb, &vocab, &scale))
        .collect::<Result<_>>()?;
    let ys: Vec<u8> = parts.test.iter().map(|(_, y)| *y).collect();
    let ps = scored(&logits(&weights, bias, &xs_test), temperature);
    let rates = ProviderRates::of(&parts.fit)?;
    let bar: Vec<f64> = parts
        .test
        .iter()
        .map(|(row, _)| rates.rate(row))
        .collect::<Result<_>>()?;
    let ours = measures(&ps, &ys);
    let bar_m = measures(&bar, &ys);
    let base = rates.overall;
    let constant = measures(&vec![base; ys.len()], &ys);

    let below = |measure: &str, invert: bool| -> Result<Value> {
        let (Value::Float(a), Value::Float(b)) =
            (py::item(&ours, measure)?, py::item(&bar_m, measure)?)
        else {
            return Ok(Value::Null);
        };
        Ok(Value::Bool(if invert { a > b } else { a < b }))
    };
    let mut beats = Dict::new();
    beats.set("brier", below("brier", false)?);
    beats.set("rank_auc", below("rank_auc", true)?);

    let mut counts = Dict::new();
    counts.set("fit", Value::Int(early.len() as i128));
    counts.set("calib", Value::Int(later.len() as i128));
    counts.set("test", Value::Int(ys.len() as i128));

    let mut out = Dict::new();
    out.set("label", Value::text(Label::Outcome.name()));
    out.set("positive", Value::text(Label::Outcome.positive()));
    out.set("embed_model", Value::text(embed_model));
    out.set("embed_dim", Value::Int(dim as i128));
    out.set("temperature", Value::Float(temperature));
    out.set("test_from", Value::text(&test_from.isoformat()));
    out.set("rows", Value::Dict(counts));
    out.set("test_rows", Value::Int(ys.len() as i128));
    out.set(
        "test_positive",
        Value::Int(baseline::positives(&parts.test) as i128),
    );
    out.set("fit_base_rate", Value::Float(base));
    out.set("constant_base_rate", constant);
    out.set("provider_base_rate", bar_m);
    out.set("model", ours);
    out.set("beats_provider_base", Value::Dict(beats));

    Ok((
        Head {
            weights,
            bias,
            temperature,
            embed_dim: dim,
        },
        Value::Dict(out),
    ))
}

fn collect_embeddings(
    parts: &baseline::Slices<'_>,
    model: &str,
    url: &str,
) -> Result<HashMap<String, Vec<f64>>> {
    let mut found = HashMap::new();
    for (row, _) in parts.fit.iter().chain(parts.test.iter()) {
        let id = run_id_text(row)?;
        if found.contains_key(&id) {
            continue;
        }
        // A run with no task text has nothing to embed, and Ollama answers an empty input with
        // no vector; it gets zeros below, so the head leans on its record features alone.
        let text = task_text(row)?;
        let embedding = if text.trim().is_empty() {
            Vec::new()
        } else {
            embed_task(&text, model, url)?
        };
        found.insert(id, embedding);
    }
    let dim = found.values().map(Vec::len).max().unwrap_or(0);
    for embedding in found.values_mut() {
        if embedding.is_empty() {
            *embedding = vec![0.0; dim];
        }
    }
    Ok(found)
}

/// Embed each labelled task, fit, and return the printed report.
pub fn report(rows: &[Value], test_from: &DateTime, env: &Env) -> Result<Value> {
    let model = embed_model(env);
    let url = embed_url(env);
    let parts = baseline::slices(rows, Label::Outcome, test_from)?;
    let embeddings = collect_embeddings(&parts, &model, &url)?;
    let (_, printed) = report_with_embeddings(rows, test_from, &model, &embeddings)?;
    Ok(printed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::baseline_cli::day;
    use hydra_pyjson::json;

    fn row(run_id: &str, provider: &str, task: &str, outcome: &str) -> Value {
        json::loads_str(&format!(
            r#"{{"run_id":"{run_id}","provider":"{provider}","model":"gpt","policy":"standard","task_text":"{task}","outcome":"{outcome}"}}"#
        ))
        .unwrap()
    }

    fn uuid(millis: u64, n: u32) -> String {
        format!(
            "{:08x}-{:04x}-7{:03x}-8{:03x}-{:012x}",
            (millis >> 16) as u32,
            millis as u16,
            0u16,
            0u16,
            u64::from(n)
        )
    }

    #[test]
    fn generate_urls_are_rewritten_to_embed() {
        assert_eq!(
            rewrite_embed_url("http://127.0.0.1:11434/api/generate"),
            "http://127.0.0.1:11434/api/embed"
        );
        assert_eq!(
            rewrite_embed_url("http://127.0.0.1:9/api/embed"),
            "http://127.0.0.1:9/api/embed"
        );
        assert_eq!(
            rewrite_embed_url("http://localhost:11434"),
            "http://localhost:11434/api/embed"
        );
    }

    #[test]
    fn both_ollama_embedding_shapes_read() {
        let nested = parse_embedding(br#"{"embeddings":[[1.0, 0.0, 0.5]]}"#).unwrap();
        assert_eq!(nested, [1.0, 0.0, 0.5]);
        let single = parse_embedding(br#"{"embedding":[0.25, 0.75]}"#).unwrap();
        assert_eq!(single, [0.25, 0.75]);
        assert!(parse_embedding(br#"{"model":"x"}"#).is_err());
    }

    #[test]
    fn a_zero_vector_stays_zero_after_normalising() {
        assert_eq!(l2_normalize(&[0.0, 0.0]), [0.0, 0.0]);
        let n = l2_normalize(&[3.0, 4.0]);
        assert!((n[0] - 0.6).abs() < 1e-12);
        assert!((n[1] - 0.8).abs() < 1e-12);
    }

    #[test]
    fn mixed_labels_on_the_same_logit_raise_the_temperature() {
        let zs = [8.0, 8.0, 8.0, 8.0, 8.0, 8.0];
        let ys = [1, 1, 1, 0, 0, 0];
        let t = fit_temperature(&zs, &ys);
        assert!(t > 1.5, "{t}");
    }

    #[test]
    fn the_head_does_not_train_on_the_later_test_slice() {
        let test_from = day("2026-09-23").unwrap();
        let early_ms = 1_788_220_800_000;
        let calib_ms = 1_789_430_400_000;
        let test_ms = 1_790_208_000_000;
        let mut rows = Vec::new();
        let mut embeddings = HashMap::new();
        let mut n = 0u32;
        let mut push = |millis: u64, fail: bool, trap: bool| {
            n += 1;
            let id = uuid(millis + u64::from(n), n);
            let outcome = if fail { "failed" } else { "completed" };
            let task = if fail { "fail later" } else { "ok later" };
            rows.push(row(&id, "codex", task, outcome));
            let embedding = if trap {
                if fail {
                    vec![0.0, 0.0, 1.0]
                } else {
                    vec![0.0, 0.0, -1.0]
                }
            } else if fail {
                vec![1.0, 0.0, 0.0]
            } else {
                vec![0.0, 1.0, 0.0]
            };
            embeddings.insert(id, embedding);
        };
        for i in 0..8u32 {
            push(early_ms + u64::from(i) * 1000, i % 2 == 0, false);
        }
        for i in 0..8u32 {
            push(calib_ms + u64::from(i) * 1000, i % 2 == 0, false);
        }
        for i in 0..8u32 {
            push(test_ms + u64::from(i) * 1000, i % 2 == 0, true);
        }
        let (head, report) =
            report_with_embeddings(&rows, &test_from, "nomic-embed-text", &embeddings).unwrap();
        assert_eq!(head.embed_dim, 3);
        assert!(head.bias.is_finite());
        let record = match &head.weights[head.weights.len() - 3..] {
            [w0, w1, w2] => (*w0, *w1, *w2),
            _ => panic!("expected three embedding weights"),
        };
        assert!(
            record.0.abs() > record.2.abs() * 4.0,
            "fit used the test trap dim: {record:?}"
        );
        assert!(
            record.1.abs() > record.2.abs() * 4.0,
            "fit used the test trap dim: {record:?}"
        );
        let Value::Dict(body) = report else {
            panic!("report is not an object");
        };
        match body.get("temperature") {
            Some(Value::Float(t)) => assert!(*t > 0.0 && t.is_finite(), "{t}"),
            other => panic!("{other:?}"),
        }
    }
}
