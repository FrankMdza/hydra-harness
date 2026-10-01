//! `baseline.py` without its label queue: the logistic baseline that trains, reports and
//! scores (build ticket 213).
//!
//! Map ticket 94 asks for CPython 3.14.7's bytes, and a model's `version` hashes its
//! weights, so one bit off after 3,000 steps of gradient descent renames the model. Every
//! float here is therefore computed in the order the Python computes it, with these four
//! things in particular:
//!
//! - `sum()` over floats is compensated since Python 3.12. Each one is
//!   `hydra_pyjson::math::fsum_like_python`, never a fold.
//! - `x ** 2` is the C library's `pow`, which is not `x * x`: `math::pow`.
//! - `max(p, eps)` keeps a NaN where `f64::max` drops it.
//! - A run's launch time is `datetime.fromtimestamp(millis / 1000)`, a float division, so the
//!   microseconds are the float's and not the id's.
//!
//! A dataset row and a model file are whatever a writer left, so a value of the wrong shape
//! is an error whose text starts with the class Python raises, as in `records.rs`. A
//! refusal the Python makes with `SystemExit` and a sentence is a [`Refusal`].
//!
//! The arguments and the two commands are in `baseline_cli.rs`.

use std::io::Write;
use std::path::Path;

use anyhow::{Result, anyhow, bail};
use hydra_pyjson::json::{self, Dict, PyStr, Value};
use hydra_pyjson::math::{self, fsum_like_python};
use hydra_pyjson::models::{Pair, canonical};
use hydra_pyjson::py;
use sha2::{Digest, Sha256};

use crate::isotime::{self, DateTime};
use crate::records;

const PREAMBLE: &str = "Hydra launched this run";
/// Outcomes the `task` label counts as the task's own failure. `completed` is 0, and every
/// other outcome says nothing about the task and leaves the row out.
const TASK_FAILURES: [&str; 3] = ["failed", "crashed", "timed_out"];
/// Below this many rows of the rarer class in either slice, `train` refuses.
pub(crate) const MIN_MINORITY: usize = 10;
const L2: f64 = 1.0;
const STEPS: usize = 3000;
const RATE: f64 = 0.5;
/// How many rows of weight the parent rate gets when a pair's or a provider's own rate is
/// estimated.
const SHRINK: f64 = 5.0;

/// What `raise SystemExit("...")` says: the sentence goes to stderr and the exit code is 1.
#[derive(Debug)]
pub struct Refusal(pub String);

impl std::fmt::Display for Refusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Refusal {}

/// The three labels, each trained, stored and scored apart.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Label {
    Outcome,
    Verdict,
    Task,
}

impl Label {
    pub const ALL: [Label; 3] = [Label::Outcome, Label::Verdict, Label::Task];

    pub fn name(self) -> &'static str {
        match self {
            Label::Outcome => "outcome",
            Label::Verdict => "verdict",
            Label::Task => "task",
        }
    }

    pub fn named(name: &str) -> Option<Label> {
        Label::ALL.into_iter().find(|label| label.name() == name)
    }

    /// What a 1 means, as the model file says it.
    pub(crate) fn positive(self) -> &'static str {
        match self {
            Label::Outcome => "not completed",
            Label::Verdict => "fixed",
            Label::Task => "failed for the task",
        }
    }
}

/// `row.get(key)`.
fn get(row: &Value, key: &str) -> Result<Value> {
    py::get(row, key, Value::Null)
}

/// `int(text, 16)` on the first twelve characters of a run id, `None` where it raises
/// `ValueError`. `int` reads more than hex digits: whitespace around the number, a `+`, a
/// `0x` and one underscore after it, single underscores between digits, and the decimal
/// digits of any script.
fn int_base16(codes: &[u32]) -> Option<u64> {
    let mut s = Vec::with_capacity(codes.len());
    for &code in codes {
        if code < 127 {
            s.push(code as u8);
            continue;
        }
        let c = char::from_u32(code)?;
        if py::is_space(c) {
            s.push(b' ');
        } else {
            s.push(py::decimal_digit(c)? as u8);
        }
    }
    let at = |i: usize| s.get(i).copied().unwrap_or(0);
    let space = |c: u8| matches!(c, b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c);
    let mut i = 0;
    while space(at(i)) {
        i += 1;
    }
    if at(i) == b'+' {
        i += 1;
    }
    if at(i) == b'0' && matches!(at(i + 1), b'x' | b'X') {
        i += 2;
        if at(i) == b'_' {
            i += 1;
        }
    }
    if at(i) == b'_' {
        return None;
    }
    let (mut value, mut digits, mut prev) = (0_u64, 0, 0_u8);
    while i < s.len() {
        let c = s[i];
        if c == b'_' {
            if prev == b'_' {
                return None;
            }
        } else if let Some(digit) = (c as char).to_digit(16) {
            value = value * 16 + u64::from(digit);
            digits += 1;
        } else {
            break;
        }
        prev = c;
        i += 1;
    }
    if prev == b'_' || digits == 0 {
        return None;
    }
    while space(at(i)) {
        i += 1;
    }
    (i == s.len()).then_some(value)
}

/// `launched_at(row["run_id"])` as a moment: a run id is a UUIDv7, so its first 48 bits are
/// the launch time in milliseconds.
fn launched_at(row: &Value) -> Result<i128> {
    let Value::Str(run_id) = py::item(row, "run_id")? else {
        bail!(
            "AttributeError: '{}' object has no attribute 'replace'",
            py::type_name(py::item(row, "run_id")?)
        );
    };
    let head: Vec<u32> = run_id
        .code_points()
        .iter()
        .copied()
        .filter(|code| *code != u32::from('-'))
        .take(12)
        .collect();
    let Some(millis) = int_base16(&head) else {
        let mut text = PyStr::new();
        head.iter().for_each(|code| text.push_code(*code));
        bail!(
            "ValueError: invalid literal for int() with base 16: {}",
            py::repr(&Value::Str(text))
        );
    };
    DateTime::from_unix_millis(millis)
        .and_then(|at| at.instant())
        .ok_or_else(|| anyhow!("ValueError: year must be in 1..9999"))
}

/// `target`: 1 for the outcome the router wants to avoid, `None` when the row carries no
/// label.
fn target(row: &Value, label: Label) -> Result<Option<u8>> {
    match label {
        Label::Outcome => {
            // A human stop says nothing about whether the run would have completed.
            let outcome = get(row, "outcome")?;
            if matches!(outcome, Value::Null) || py::eq_str(&outcome, "stopped") {
                return Ok(None);
            }
            Ok(Some(u8::from(!py::eq_str(&outcome, "completed"))))
        }
        Label::Task => {
            let outcome = get(row, "outcome")?;
            if py::eq_str(&outcome, "completed") {
                return Ok(Some(0));
            }
            let failed = TASK_FAILURES.iter().any(|name| py::eq_str(&outcome, name));
            Ok(failed.then_some(1))
        }
        Label::Verdict => {
            let verdict = get(row, "human_verdict")?;
            py::hashable(&verdict)?;
            Ok(if py::eq_str(&verdict, "fixed") {
                Some(1)
            } else if py::eq_str(&verdict, "applied") {
                Some(0)
            } else {
                None
            })
        }
    }
}

/// `raw_features`: what a router has before a launch. Three one-hot names and two numbers.
pub(crate) struct Features {
    names: [PyStr; 3],
    pub(crate) log_task_chars: f64,
    preamble: f64,
}

fn contains(hay: &[u32], needle: &PyStr) -> bool {
    let needle = needle.code_points();
    hay.windows(needle.len()).any(|window| window == needle)
}

impl Features {
    pub(crate) fn of(row: &Value) -> Result<Features> {
        let text = get(row, "task_text")?;
        let name = |key: &str| -> Result<PyStr> {
            let mut out = PyStr::from(format!("{key}=").as_str());
            out.push(&py::str(&get(row, key)?));
            Ok(out)
        };
        let names = [name("provider")?, name("model")?, name("policy")?];
        // `row.get("task_text") or ""`, then `len` and `in` on whatever that is.
        let preamble = PyStr::from(PREAMBLE);
        let (chars, has_preamble) = if py::truthy(&text) {
            let has = match &text {
                Value::Str(s) => contains(s.code_points(), &preamble),
                Value::List(items) => items.iter().any(|item| py::eq_str(item, PREAMBLE)),
                Value::Dict(dict) => dict.get(PREAMBLE).is_some(),
                _ => false,
            };
            (py::len(&text)?, has)
        } else {
            (0, false)
        };
        Ok(Features {
            names,
            log_task_chars: math::log1p(chars as f64),
            preamble: f64::from(u8::from(has_preamble)),
        })
    }

    /// The five names, in the order the Python's dict holds them.
    pub(crate) fn names(&self) -> Vec<PyStr> {
        let mut names = self.names.to_vec();
        names.push(PyStr::from("log_task_chars"));
        names.push(PyStr::from("preamble"));
        names
    }

    /// `feats.get(name, 0.0)`.
    pub(crate) fn value(&self, name: &PyStr) -> f64 {
        if self.names.contains(name) {
            1.0
        } else if name == "log_task_chars" {
            self.log_task_chars
        } else if name == "preamble" {
            self.preamble
        } else {
            0.0
        }
    }
}

/// A number of a model file as the float Python's arithmetic makes of it.
pub(crate) fn number(value: &Value) -> Result<f64> {
    match value {
        Value::Bool(b) => Ok(f64::from(u8::from(*b))),
        Value::Int(n) => Ok(*n as f64),
        Value::Float(x) => Ok(*x),
        Value::BigInt(digits) => match digits.parse::<f64>() {
            Ok(x) if x.is_finite() => Ok(x),
            _ => bail!("OverflowError: int too large to convert to float"),
        },
        other => bail!(
            "TypeError: unsupported operand type(s): 'float' and '{}'",
            py::type_name(other)
        ),
    }
}

/// `vector`: one row as the model reads it, a value per name of the vocabulary, scaled
/// where the model holds a scale for the name. `feats` is `feats.get(name, 0.0)`, which the
/// monitor answers from more names than the baseline has.
pub(crate) fn vector(
    feats: &dyn Fn(&PyStr) -> f64,
    vocab: &[Value],
    scale: &[(PyStr, Vec<Value>)],
) -> Result<Vec<f64>> {
    let mut out = Vec::with_capacity(vocab.len());
    for name in vocab {
        py::hashable(name)?;
        let Value::Str(name) = name else {
            out.push(0.0);
            continue;
        };
        let mut value = feats(name);
        if let Some((_, pair)) = scale.iter().find(|(key, _)| key == name) {
            let [mean, std] = pair.as_slice() else {
                bail!(
                    "ValueError: a scale of {} values where two are expected",
                    pair.len()
                );
            };
            let centered = value - number(mean)?;
            let std = number(std)?;
            if std == 0.0 {
                bail!("ZeroDivisionError: division by zero");
            }
            value = centered / std;
        }
        out.push(value);
    }
    Ok(out)
}

fn sigmoid(z: f64) -> f64 {
    if z >= 0.0 {
        1.0 / (1.0 + math::exp(-z))
    } else {
        let e = math::exp(z);
        e / (1.0 + e)
    }
}

/// `bias + sum(w * v for w, v in zip(weights, x))`. `products` is the caller's buffer, so
/// the fit does not allocate one per row per step.
fn logit(weights: &[f64], bias: f64, x: &[f64], products: &mut Vec<f64>) -> f64 {
    products.clear();
    products.extend(weights.iter().zip(x).map(|(w, v)| w * v));
    bias + fsum_like_python(products)
}

/// `fit`: full-batch gradient descent on L2-penalised log loss. Nothing is shuffled.
pub(crate) fn fit(xs: &[Vec<f64>], ys: &[u8]) -> (Vec<f64>, f64) {
    let n = xs.len() as f64;
    let d = xs[0].len();
    let (mut weights, mut bias) = (vec![0.0; d], 0.0);
    let mut products = Vec::with_capacity(d);
    for _ in 0..STEPS {
        let (mut grad_w, mut grad_b) = (vec![0.0; d], 0.0);
        for (x, y) in xs.iter().zip(ys) {
            let err = sigmoid(logit(&weights, bias, x, &mut products)) - f64::from(*y);
            grad_b += err;
            for (g, v) in grad_w.iter_mut().zip(x) {
                if *v != 0.0 {
                    *g += err * v;
                }
            }
        }
        bias -= RATE * grad_b / n;
        for (w, g) in weights.iter_mut().zip(&grad_w) {
            *w -= RATE * (g / n + L2 * *w / n);
        }
    }
    (weights, bias)
}

/// Python's `max(a, b)`: the first unless the second is greater, so a NaN first stays.
fn py_max(a: f64, b: f64) -> f64 {
    if b > a { b } else { a }
}

fn log_loss(ps: &[f64], ys: &[u8]) -> f64 {
    let eps = 1e-12;
    let terms: Vec<f64> = ps
        .iter()
        .zip(ys)
        .map(|(p, y)| {
            let y = f64::from(*y);
            y * math::log(py_max(*p, eps)) + (1.0 - y) * math::log(py_max(1.0 - p, eps))
        })
        .collect();
    -fsum_like_python(&terms) / ys.len() as f64
}

fn brier(ps: &[f64], ys: &[u8]) -> f64 {
    let terms: Vec<f64> = ps
        .iter()
        .zip(ys)
        .map(|(p, y)| math::pow(p - f64::from(*y), 2.0))
        .collect();
    fsum_like_python(&terms) / ys.len() as f64
}

fn rank_auc(ps: &[f64], ys: &[u8]) -> Option<f64> {
    let of = |wanted: bool| -> Vec<f64> {
        ps.iter()
            .zip(ys)
            .filter(|(_, y)| (**y != 0) == wanted)
            .map(|(p, _)| *p)
            .collect()
    };
    let (pos, neg) = (of(true), of(false));
    if pos.is_empty() || neg.is_empty() {
        return None;
    }
    let mut wins = Vec::with_capacity(pos.len() * neg.len());
    for a in &pos {
        for b in &neg {
            wins.push(if a > b {
                1.0
            } else if a == b {
                0.5
            } else {
                0.0
            });
        }
    }
    Some(fsum_like_python(&wins) / (pos.len() * neg.len()) as f64)
}

fn reliability(ps: &[f64], ys: &[u8]) -> Value {
    const BINS: usize = 5;
    let mut table = Vec::new();
    for k in 0..BINS {
        let (lo, hi) = (k as f64 / BINS as f64, (k + 1) as f64 / BINS as f64);
        let members: Vec<(f64, u8)> = ps
            .iter()
            .zip(ys)
            .filter(|(p, _)| (lo <= **p && **p < hi) || (k == BINS - 1 && **p == 1.0))
            .map(|(p, y)| (*p, *y))
            .collect();
        let n = members.len();
        let mut row = Dict::new();
        row.set("bin", Value::text(&format!("{lo:.1}-{hi:.1}")));
        row.set("n", Value::Int(n as i128));
        let (mean_p, observed) = if n == 0 {
            (Value::Null, Value::Null)
        } else {
            let scores: Vec<f64> = members.iter().map(|(p, _)| *p).collect();
            let positive: u64 = members.iter().map(|(_, y)| u64::from(*y)).sum();
            (
                Value::Float(fsum_like_python(&scores) / n as f64),
                Value::Float(positive as f64 / n as f64),
            )
        };
        row.set("mean_p", mean_p);
        row.set("observed", observed);
        table.push(Value::Dict(row));
    }
    Value::List(table)
}

/// The rows of a dataset that carry `label`, each with its target, split by launch time so
/// that nothing launched later can train what launched earlier.
pub(crate) struct Slices<'a> {
    pub(crate) labelled: usize,
    pub(crate) fit: Vec<(&'a Value, u8)>,
    pub(crate) test: Vec<(&'a Value, u8)>,
}

pub(crate) fn slices<'a>(
    rows: &'a [Value],
    label: Label,
    test_from: &DateTime,
) -> Result<Slices<'a>> {
    let mut labelled = Vec::new();
    for row in rows {
        if let Some(y) = target(row, label)? {
            labelled.push((row, y));
        }
    }
    let (mut fit, mut test) = (Vec::new(), Vec::new());
    for (row, y) in &labelled {
        let at = launched_at(row)?;
        let Some(from) = test_from.instant() else {
            bail!("TypeError: can't compare offset-naive and offset-aware datetimes");
        };
        if at < from {
            fit.push((*row, *y));
        } else {
            test.push((*row, *y));
        }
    }
    Ok(Slices {
        labelled: labelled.len(),
        fit,
        test,
    })
}

pub(crate) fn positives(part: &[(&Value, u8)]) -> usize {
    part.iter().map(|(_, y)| usize::from(*y)).sum()
}

/// `json.dumps(value, sort_keys=True)`.
pub(crate) fn dumps_sorted(value: &Value) -> String {
    fn sorted(value: &Value) -> Value {
        match value {
            Value::List(items) => Value::List(items.iter().map(sorted).collect()),
            Value::Dict(dict) => {
                let mut entries: Vec<(&PyStr, &Value)> = dict.iter().collect();
                entries.sort_by(|a, b| a.0.cmp(b.0));
                let mut out = Dict::new();
                for (key, item) in entries {
                    out.push(key.clone(), sorted(item));
                }
                Value::Dict(out)
            }
            other => other.clone(),
        }
    }
    json::dumps_default(&sorted(value))
}

pub(crate) fn floats(values: &[f64]) -> Value {
    Value::List(values.iter().map(|x| Value::Float(*x)).collect())
}

/// `train`: the fitted model as the file holds it, or the refusal when either slice has
/// fewer than ten rows of its rarer class.
pub fn train(rows: &[Value], label: Label, test_from: &DateTime) -> Result<Value> {
    let parts = slices(rows, label, test_from)?;
    for (name, part) in [("fit", &parts.fit), ("test", &parts.test)] {
        let ones = positives(part);
        let minority = ones.min(part.len() - ones);
        if minority < MIN_MINORITY {
            let label = label.name();
            return Err(Refusal(format!(
                "baseline: {} labelled rows for '{label}'; the {name} slice has {} of them and \
                 {minority} of its rarer class, and {MIN_MINORITY} is the floor. Label more runs \
                 before training on '{label}'.",
                parts.labelled,
                part.len()
            ))
            .into());
        }
    }

    let feats: Vec<Features> = parts
        .fit
        .iter()
        .map(|(row, _)| Features::of(row))
        .collect::<Result<_>>()?;
    let mut vocab: Vec<PyStr> = feats.iter().flat_map(Features::names).collect();
    vocab.sort();
    vocab.dedup();
    let vocab: Vec<Value> = vocab.into_iter().map(Value::Str).collect();

    let n = feats.len() as f64;
    let values: Vec<f64> = feats.iter().map(|f| f.log_task_chars).collect();
    let mean = fsum_like_python(&values) / n;
    let spread: Vec<f64> = values.iter().map(|v| math::pow(v - mean, 2.0)).collect();
    let mut std = math::sqrt(fsum_like_python(&spread) / n);
    if std == 0.0 {
        std = 1.0;
    }
    let scale = vec![(
        PyStr::from("log_task_chars"),
        vec![Value::Float(mean), Value::Float(std)],
    )];

    let xs: Vec<Vec<f64>> = feats
        .iter()
        .map(|f| vector(&|name| f.value(name), &vocab, &scale))
        .collect::<Result<_>>()?;
    let ys: Vec<u8> = parts.fit.iter().map(|(_, y)| *y).collect();
    let (weights, bias) = fit(&xs, &ys);

    let mut body = Dict::new();
    body.set("label", Value::text(label.name()));
    body.set("positive", Value::text(label.positive()));
    body.set("vocab", Value::List(vocab));
    let mut scales = Dict::new();
    for (name, pair) in scale {
        scales.set_key(name, Value::List(pair));
    }
    body.set("scale", Value::Dict(scales));
    body.set("weights", floats(&weights));
    body.set("bias", Value::Float(bias));
    body.set("test_from", Value::text(&test_from.isoformat()));
    let mut counts = Dict::new();
    counts.set("fit", Value::Int(parts.fit.len() as i128));
    counts.set("test", Value::Int(parts.test.len() as i128));
    body.set("rows", Value::Dict(counts));

    let body = Value::Dict(body);
    let digest: String = Sha256::digest(dumps_sorted(&body).as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    let Value::Dict(mut model) = body else {
        unreachable!("the body is a dict");
    };
    model.set(
        "version",
        Value::text(&format!("baseline-{}-{}", label.name(), &digest[..12])),
    );
    Ok(Value::Dict(model))
}

/// `predict`: the model's probability for one row. The model is a file's JSON, read here
/// the way the Python reads it: only as far as the row's vector reaches.
pub fn predict(model: &Value, row: &Value) -> Result<f64> {
    predict_with(model, || {
        let feats = Features::of(row)?;
        Ok(move |name: &PyStr| feats.value(name))
    })
}

/// `predict` over whatever answers `feats.get(name, 0.0)` for the row. The row's features
/// are asked for after the model's scale, weights, bias and vocabulary are read, which is
/// the order the Python raises in.
pub(crate) fn predict_with<F: Fn(&PyStr) -> f64>(
    model: &Value,
    feats: impl FnOnce() -> Result<F>,
) -> Result<f64> {
    let Value::Dict(scales) = py::item(model, "scale")? else {
        bail!(
            "AttributeError: '{}' object has no attribute 'items'",
            py::type_name(py::item(model, "scale")?)
        );
    };
    let mut scale = Vec::new();
    for (name, pair) in scales.iter() {
        scale.push((name.clone(), py::list(pair)?));
    }
    let weights = py::item(model, "weights")?;
    let bias = py::item(model, "bias")?;
    let vocab = py::item(model, "vocab")?;
    let feats = feats()?;
    let x = vector(&feats, &py::list(vocab)?, &scale)?;
    let mut read = Vec::with_capacity(x.len());
    for (w, _) in py::list(weights)?.iter().zip(&x) {
        read.push(number(w)?);
    }
    Ok(sigmoid(logit(&read, number(bias)?, &x, &mut Vec::new())))
}

/// `provider_model_rates`: the fit slice's positive rate for a row's (provider, model),
/// shrunk toward its provider's, which is shrunk toward the whole slice's. This is the bar
/// a model has to beat: most positives are rate limits, so one that learned only which
/// provider runs out of quota should not pass.
struct Rates {
    pairs: Vec<(Pair, (usize, usize))>,
    providers: Vec<(Value, (usize, usize))>,
    overall: f64,
}

impl Rates {
    fn of(fit: &[(&Value, u8)]) -> Result<Rates> {
        let mut rates = Rates {
            pairs: Vec::new(),
            providers: Vec::new(),
            overall: positives(fit) as f64 / fit.len() as f64,
        };
        for (row, y) in fit {
            let (provider, model) = key(row)?;
            let at = match rates
                .pairs
                .iter()
                .position(|(known, _)| same(known, &(provider.clone(), model.clone())))
            {
                Some(at) => at,
                None => {
                    rates.pairs.push(((provider.clone(), model), (0, 0)));
                    rates.pairs.len() - 1
                }
            };
            rates.pairs[at].1.0 += usize::from(*y);
            rates.pairs[at].1.1 += 1;
            let at = match rates
                .providers
                .iter()
                .position(|(known, _)| py::same_key(known, &provider))
            {
                Some(at) => at,
                None => {
                    rates.providers.push((provider, (0, 0)));
                    rates.providers.len() - 1
                }
            };
            rates.providers[at].1.0 += usize::from(*y);
            rates.providers[at].1.1 += 1;
        }
        Ok(rates)
    }

    fn rate(&self, row: &Value) -> Result<f64> {
        let shrunk = |(ones, n): (usize, usize), prior: f64| {
            (ones as f64 + SHRINK * prior) / (n as f64 + SHRINK)
        };
        let pair = key(row)?;
        let provider = self
            .providers
            .iter()
            .find(|(known, _)| py::same_key(known, &pair.0))
            .map_or((0, 0), |(_, seen)| *seen);
        let seen = self
            .pairs
            .iter()
            .find(|(known, _)| same(known, &pair))
            .map_or((0, 0), |(_, seen)| *seen);
        Ok(shrunk(seen, shrunk(provider, self.overall)))
    }
}

/// A row's `(provider, model)` as a dict key, which a list or an object cannot be.
fn key(row: &Value) -> Result<Pair> {
    let (provider, model) = (get(row, "provider")?, get(row, "model")?);
    py::hashable(&provider)?;
    py::hashable(&model)?;
    Ok((provider, model))
}

fn same(a: &Pair, b: &Pair) -> bool {
    py::same_key(&a.0, &b.0) && py::same_key(&a.1, &b.1)
}

fn measures(ps: &[f64], ys: &[u8]) -> Value {
    let mut out = Dict::new();
    out.set("log_loss", Value::Float(log_loss(ps, ys)));
    out.set("brier", Value::Float(brier(ps, ys)));
    out.set(
        "rank_auc",
        rank_auc(ps, ys).map_or(Value::Null, Value::Float),
    );
    Value::Dict(out)
}

/// `report`: how a model `train` just fitted does on its test slice, beside a constant and
/// beside the provider and model rates.
pub fn report(model: &Value, rows: &[Value], label: Label) -> Result<Value> {
    report_with(model, rows, label, &|row| predict(model, row))
}

/// `report` with the caller's `predict`, which is the one line `monitor.py`'s differs by.
pub(crate) fn report_with(
    model: &Value,
    rows: &[Value],
    label: Label,
    predict: &dyn Fn(&Value) -> Result<f64>,
) -> Result<Value> {
    let Value::Str(written) = py::item(model, "test_from")? else {
        bail!("TypeError: fromisoformat: argument must be str");
    };
    let test_from = isotime::fromisoformat(written)
        .ok_or_else(|| anyhow!("ValueError: Invalid isoformat string"))?;
    let parts = slices(rows, label, &test_from)?;
    let ys: Vec<u8> = parts.test.iter().map(|(_, y)| *y).collect();
    if parts.fit.is_empty() {
        bail!("ZeroDivisionError: division by zero");
    }
    let base = positives(&parts.fit) as f64 / parts.fit.len() as f64;
    let scored: Vec<f64> = parts
        .test
        .iter()
        .map(|(row, _)| predict(row))
        .collect::<Result<_>>()?;
    let rates = Rates::of(&parts.fit)?;
    if ys.is_empty() {
        bail!("ZeroDivisionError: division by zero");
    }

    let ours = measures(&scored, &ys);
    let bar_scores: Vec<f64> = parts
        .test
        .iter()
        .map(|(row, _)| rates.rate(row))
        .collect::<Result<_>>()?;
    let bar = measures(&bar_scores, &ys);
    let below = |measure: &str| -> Result<Value> {
        let (Value::Float(a), Value::Float(b)) =
            (py::item(&ours, measure)?, py::item(&bar, measure)?)
        else {
            unreachable!("a measure is a float");
        };
        Ok(Value::Bool(a < b))
    };
    let mut beats = Dict::new();
    beats.set("log_loss", below("log_loss")?);
    beats.set("brier", below("brier")?);

    let mut out = Dict::new();
    out.set("version", py::item(model, "version")?.clone());
    out.set("label", Value::text(label.name()));
    out.set("test_rows", Value::Int(ys.len() as i128));
    out.set("test_positive", Value::Int(positives(&parts.test) as i128));
    out.set("fit_base_rate", Value::Float(base));
    out.set("constant_base_rate", measures(&vec![base; ys.len()], &ys));
    out.set("provider_model_base_rate", bar);
    out.set("model", ours);
    out.set("beats_provider_model_base", Value::Dict(beats));
    out.set("reliability", reliability(&scored, &ys));
    Ok(Value::Dict(out))
}

/// `row_from_record`: the pre-launch fields of one record, spelled the way a dataset row
/// spells them. `aliases` is asked for only once the record is known to be a run.
pub fn row_from_record(
    path: &Path,
    shown: &str,
    aliases: &dyn Fn() -> Result<Vec<(Pair, Pair)>>,
    stderr: &mut dyn Write,
) -> Result<Value> {
    let Some(item) = records::scan_record(path, stderr)? else {
        return Err(Refusal(format!("baseline: {shown} is not a run or resume record")).into());
    };
    row_of(&item, aliases)
}

/// `row_from_record` once the record is scanned.
pub(crate) fn row_of(
    item: &records::Scanned,
    aliases: &dyn Fn() -> Result<Vec<(Pair, Pair)>>,
) -> Result<Value> {
    let first = &item.first;
    let start = get(first, "hydra")?;
    let start = if py::truthy(&start) {
        start
    } else {
        Value::Dict(Dict::new())
    };
    let provider = py::item(first, "provider")?;
    let (_, model) = canonical(provider, &get(first, "model")?, &aliases()?)?;
    let task = get(&start, "task")?;
    let mut row = Dict::new();
    row.set("run_id", py::item(first, "run_id")?.clone());
    row.set("provider", provider.clone());
    row.set("model", model);
    row.set("policy", get(&start, "policy")?);
    row.set(
        "task_text",
        if py::truthy(&task) {
            task
        } else {
            Value::text("")
        },
    );
    Ok(Value::Dict(row))
}

/// What `score` prints: one line of JSON with the model's probability for the row.
pub fn score_line(model: &Value, row: &Value) -> Result<String> {
    let mut out = Dict::new();
    out.set("run_id", py::item(row, "run_id")?.clone());
    out.set("label", py::item(model, "label")?.clone());
    out.set("positive", py::item(model, "positive")?.clone());
    out.set("p", Value::Float(predict(model, row)?));
    out.set("model_version", py::item(model, "version")?.clone());
    Ok(json::dumps_default(&Value::Dict(out)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(text: &str) -> Option<u64> {
        int_base16(PyStr::from(text).code_points())
    }

    #[test]
    fn a_run_id_reads_as_python_reads_a_base_16_int() {
        assert_eq!(hex("0199d3f2a1b0"), Some(0x0199_d3f2_a1b0));
        assert_eq!(hex("0x_1f"), Some(31));
        assert_eq!(hex(" 1f"), Some(31));
        assert_eq!(hex("+1f"), Some(31));
        assert_eq!(hex("1_f"), Some(31));
        assert_eq!(hex("\u{ff11}\u{ff12}"), Some(18));
        assert_eq!(hex("1f\u{2003}"), Some(31));
        for refused in ["", "_1", "1_", "1__2", "0x", "\u{1c}1", "g", "1 f"] {
            assert_eq!(hex(refused), None, "{refused:?}");
        }
    }

    #[test]
    fn a_nan_score_stays_a_nan_in_the_log_loss() {
        assert!(py_max(f64::NAN, 1e-12).is_nan());
        assert_eq!(py_max(0.0, 1e-12), 1e-12);
    }

    #[test]
    fn keys_are_sorted_at_every_depth_for_the_digest() {
        let value =
            json::loads_str(r#"{"b": {"z": 1, "a": [{"d": 1, "c": 2}]}, "a": 0.5}"#).unwrap();
        assert_eq!(
            dumps_sorted(&value),
            r#"{"a": 0.5, "b": {"a": [{"c": 2, "d": 1}], "z": 1}}"#
        );
    }
}
