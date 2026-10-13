//! The ranking half of `sheet.py`: `rank` and `beta_lower`, and `ranked_assignment`, which
//! is how `assign` asks for them. The sheets under `<puppets home>/agents/` and
//! `priors.json` are read as `sheet.py` wrote them.
//!
//! The half that builds those sheets (`build`, `budget`, `corpus_records`) is not here. It
//! shells out to `hydra show` for every run, and no command of build ticket 194 reaches it.

use std::path::Path;

use anyhow::{Result, anyhow, bail};

use crate::fold::TaskState;
use crate::json::{self, PyStr, Value};
use crate::log::{self, Home};
use crate::write::read_text;
use crate::{py, pysort};

/// One agent that cleared the bar for a task, as `rank` returns it.
#[derive(Clone, Debug)]
pub struct Ranked {
    pub provider: Value,
    pub model: Value,
    /// The 10th percentile of the agent's first-pass rate.
    pub bound: f64,
    /// Median window share per run over `bound`: lower is cheaper per accepted diff.
    pub score: Option<f64>,
    /// How many verdicts stand behind `bound`. Zero means it is the prior alone.
    pub sample_size: usize,
}

/// `json.loads(path.read_text())`, with the error class Python raises.
fn load(path: &Path) -> Result<Value> {
    json::loads_str(&read_text(path)?).map_err(|err| anyhow!("{}: {}", err.class(), path.display()))
}

/// The constants of CPython's `Modules/mathmodule.c`, each written with the digits that
/// name its `f64` and no more. The C source carries about fifty.
const LANCZOS_G: f64 = 6.02468004077673;
const LANCZOS_NUM: [f64; 13] = [
    23531376880.41076,
    42919803642.6491,
    35711959237.35567,
    17921034426.03721,
    6039542586.352028,
    1439720407.3117216,
    248874557.86205417,
    31426415.585400194,
    2876370.6289353725,
    186056.26539522348,
    8071.672002365816,
    210.82427775157936,
    2.5066282746310002,
];
/// The denominator is `x * (x + 1) * ... * (x + 11)`.
const LANCZOS_DEN: [f64; 13] = [
    0.0,
    39916800.0,
    120543840.0,
    150917976.0,
    105258076.0,
    45995730.0,
    13339535.0,
    2637558.0,
    357423.0,
    32670.0,
    1925.0,
    66.0,
    1.0,
];

/// `a * b + c` as the compiler that built CPython computes it inside one C expression. On
/// arm64 clang contracts it into a fused multiply-add, which rounds once, and CPython's
/// build passes no flag against that. On x86-64 the baseline has no such instruction and the
/// product rounds before the sum. `the_ranking_arithmetic_matches_python` in
/// `tests/differential.rs` checks the choice against the `python3` on the machine.
fn c_mul_add(a: f64, b: f64, c: f64) -> f64 {
    if cfg!(target_arch = "aarch64") {
        a.mul_add(b, c)
    } else {
        a * b + c
    }
}

/// `lanczos_sum` of CPython's `Modules/mathmodule.c`: the rational function, evaluated
/// from the high coefficient down below 5 and in powers of `1 / x` from 5 up.
fn lanczos_sum(x: f64) -> f64 {
    let (mut num, mut den) = (0.0f64, 0.0f64);
    if x < 5.0 {
        for i in (0..LANCZOS_NUM.len()).rev() {
            num = c_mul_add(num, x, LANCZOS_NUM[i]);
            den = c_mul_add(den, x, LANCZOS_DEN[i]);
        }
    } else {
        for i in 0..LANCZOS_NUM.len() {
            num = num / x + LANCZOS_NUM[i];
            den = den / x + LANCZOS_DEN[i];
        }
    }
    num / den
}

/// `math.lgamma(x)` for a positive `x`, which is all `beta_lower` asks. It is `m_lgamma`
/// of CPython's `Modules/mathmodule.c`, operation for operation: CPython does not call the
/// C library's `lgamma`, and on macOS the two differ in the last bits on 196 of the integers
/// from 1 to 399, which is enough to swap two agents `rank` scores a rounding apart.
pub fn lgamma(x: f64) -> f64 {
    if x == x.floor() && x <= 2.0 {
        return 0.0;
    }
    if x < 1e-20 {
        return -x.ln();
    }
    let r = lanczos_sum(x).ln() - LANCZOS_G;
    c_mul_add(x - 0.5, (x + LANCZOS_G - 0.5).ln() - 1.0, r)
}

/// `beta_lower`: the 10th percentile of Beta(a, b) for integers, by bisection on the
/// binomial sum that equals its distribution function.
///
/// Each term is Python's, with the same `lgamma` and the operations in the same order, so
/// the result is the same float and `rank` orders a near tie as `sheet.py` does.
pub fn beta_lower(a: i128, b: i128) -> Result<f64> {
    const PROBABILITY: f64 = 0.1;
    let n = a + b - 1;
    let terms = usize::try_from((n - a + 1).max(0))
        .ok()
        .filter(|terms| *terms <= 10_000_000)
        .ok_or_else(|| anyhow!("OverflowError: prior and verdict counts are too large"))?;
    // The first term asks `math.lgamma` for `a + 1`, which raises at zero and below. With no
    // term to sum Python never asks, and a zero `a` is a term like any other.
    if a < 0 && terms > 0 {
        bail!("ValueError: expected a noninteger or positive integer");
    }
    // `sum()` over floats is compensated since Python 3.12: it carries the rounding error of
    // each addition in `lost` and adds it back once at the end.
    let cdf = |x: f64| -> f64 {
        let (mut sum, mut lost) = (0.0f64, 0.0f64);
        for j in a..a + terms as i128 {
            let term =
                (lgamma((n + 1) as f64) - lgamma((j + 1) as f64) - lgamma((n - j + 1) as f64)
                    + j as f64 * x.ln()
                    + (n - j) as f64 * (-x).ln_1p())
                .exp();
            let next = sum + term;
            lost += if sum.abs() >= term.abs() {
                (sum - next) + term
            } else {
                (term - next) + sum
            };
            sum = next;
        }
        if lost != 0.0 && lost.is_finite() {
            sum += lost;
        }
        sum
    };
    let (mut lo, mut hi) = (0.0f64, 1.0f64);
    for _ in 0..60 {
        let mid = (lo + hi) / 2.0;
        if cdf(mid) < PROBABILITY {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    Ok((lo + hi) / 2.0)
}

/// `needle in container`, for a string needle and whatever JSON the container is.
fn contains(container: &Value, needle: &str) -> Result<bool> {
    match container {
        Value::List(items) => Ok(items.iter().any(|item| py::eq_str(item, needle))),
        Value::Dict(dict) => Ok(dict.get(needle).is_some()),
        Value::Str(text) => Ok(has_substring(text, &PyStr::from(needle))),
        other => bail!(
            "TypeError: argument of type '{}' is not a container or iterable",
            py::type_name(other)
        ),
    }
}

fn has_substring(hay: &PyStr, needle: &PyStr) -> bool {
    let (hay, needle) = (hay.code_points(), needle.code_points());
    needle.is_empty() || hay.windows(needle.len()).any(|window| window == needle)
}

/// `(agent.get('model') or '').lower().replace(' ', '-')`.
fn model_name(agent: &Value) -> Result<PyStr> {
    let model = py::get(agent, "model", Value::Null)?;
    if !py::truthy(&model) {
        return Ok(PyStr::new());
    }
    let Value::Str(model) = &model else {
        bail!(
            "AttributeError: '{}' object has no attribute 'lower'",
            py::type_name(&model)
        );
    };
    let mut name = PyStr::new();
    for &code in model.code_points() {
        match char::from_u32(code) {
            Some(' ') => name.push_code(u32::from('-')),
            Some(c) => name.push_str(&c.to_lowercase().collect::<String>()),
            None => name.push_code(code),
        }
    }
    Ok(name)
}

/// The Beta prior for one agent on one task class: the first rule in `priors.json` whose
/// provider is the agent's and whose `model_contains` is in the model's name, else the
/// default. A rule's `classes` overrides its `prior` for the classes it names.
fn prior_for(
    priors: &Value,
    agent: &Value,
    name: &PyStr,
    task_class: &Value,
) -> Result<(i128, i128)> {
    let mut prior = py::item(priors, "default")?.clone();
    for rule in py::list(py::item(priors, "rules")?)? {
        if !py::eq(py::item(agent, "provider")?, py::item(&rule, "provider")?) {
            continue;
        }
        let Value::Str(part) = py::get(&rule, "model_contains", Value::text(""))? else {
            bail!("TypeError: 'in <string>' requires string as left operand");
        };
        if !has_substring(name, &part) {
            continue;
        }
        let classes = py::get(&rule, "classes", Value::Dict(json::Dict::new()))?;
        let own = py::item(&rule, "prior")?.clone();
        py::hashable(task_class)?;
        prior = match (&classes, task_class) {
            (Value::Dict(classes), Value::Str(class)) => {
                classes.get_key(class).cloned().unwrap_or(own)
            }
            (Value::Dict(_), _) => own,
            (other, _) => bail!(
                "AttributeError: '{}' object has no attribute 'get'",
                py::type_name(other)
            ),
        };
        break;
    }
    match &prior {
        Value::List(pair) => match pair.as_slice() {
            [Value::Int(a), Value::Int(b)] => Ok((*a, *b)),
            [_, _] => bail!("TypeError: a prior in priors.json is not two integers"),
            _ => bail!("ValueError: a prior in priors.json is not a pair"),
        },
        other => bail!(
            "TypeError: cannot unpack non-iterable {} object",
            py::type_name(other)
        ),
    }
}

/// A Python dict keyed by JSON values: a later equal key replaces the value in place.
struct Table<T>(Vec<(Value, T)>);

impl<T> Table<T> {
    fn get(&self, key: &Value) -> Result<Option<&T>> {
        py::hashable(key)?;
        Ok(self
            .0
            .iter()
            .find(|(k, _)| py::same_key(k, key))
            .map(|(_, v)| v))
    }

    fn insert(&mut self, key: Value, value: T) -> Result<()> {
        py::hashable(&key)?;
        match self.0.iter_mut().find(|(k, _)| py::same_key(k, &key)) {
            Some(slot) => slot.1 = value,
            None => self.0.push((key, value)),
        }
        Ok(())
    }
}

/// `x / bound`, for the JSON numbers Python divides.
fn as_float(cost: &Value) -> Result<Option<f64>> {
    match cost {
        Value::Null => Ok(None),
        Value::Bool(b) => Ok(Some(f64::from(u8::from(*b)))),
        Value::Int(n) => Ok(Some(*n as f64)),
        Value::Float(f) => Ok(Some(*f)),
        Value::BigInt(_) => bail!("OverflowError: integer division result too large for a float"),
        other => bail!(
            "TypeError: unsupported operand type(s) for /: '{}' and 'float'",
            py::type_name(other)
        ),
    }
}

/// `a < b` on two rows' sort keys, `(score is None, score or 0, -bound, provider, model
/// or '')`, the way Python compares tuples: the first position that differs decides.
fn ranked_less(a: &Ranked, b: &Ranked) -> Result<bool> {
    if a.score.is_none() != b.score.is_none() {
        return Ok(a.score.is_some());
    }
    let (x, y) = (a.score.unwrap_or(0.0), b.score.unwrap_or(0.0));
    if x != y {
        return Ok(x < y);
    }
    if a.bound != b.bound {
        return Ok(-a.bound < -b.bound);
    }
    if !py::eq(&a.provider, &b.provider) {
        return py::less(&a.provider, &b.provider);
    }
    let model = |row: &Ranked| {
        if py::truthy(&row.model) {
            row.model.clone()
        } else {
            Value::text("")
        }
    };
    let (x, y) = (model(a), model(b));
    if py::eq(&x, &y) {
        return Ok(false);
    }
    py::less(&x, &y)
}

/// `sheet.rank`: the agents that clear the bar for a task class and grade, cheapest per
/// accepted diff first.
///
/// An agent's bound is the 10th percentile of Beta(prior + its verdicts): `applied` counts
/// for it, `fixed` and `rejected` against, one verdict per task, and an `applied` on a
/// resumed run counts as `fixed`. `verdicts` is the goal's events. An `agi` task ranks
/// nobody.
pub fn rank(
    sheets: &[Value],
    task_class: &Value,
    grade: &Value,
    verdicts: &[Value],
    policy: &str,
    priors: &Path,
) -> Result<Vec<Ranked>> {
    if py::eq_str(grade, "agi") {
        return Ok(Vec::new());
    }
    py::hashable(grade)?;
    let threshold = if py::eq_str(grade, "low") {
        0.5
    } else if py::eq_str(grade, "medium") {
        0.6
    } else if py::eq_str(grade, "high") {
        0.7
    } else {
        bail!("KeyError: {}", py::repr(grade));
    };
    let priors = if priors.exists() {
        load(priors)?
    } else {
        Value::Null
    };
    let mut task_classes = Table(Vec::new());
    for event in verdicts {
        let name = py::get(event, "event", py::get(event, "type", Value::Null)?)?;
        if !py::eq_str(&name, "task") {
            continue;
        }
        if let Value::Dict(fields) = event
            && let (Some(id), Some(class)) = (fields.get("id"), fields.get("class"))
        {
            task_classes.insert(id.clone(), class.clone())?;
        }
    }
    let mut result = Vec::new();
    for agent in sheets {
        if !py::truthy(&py::get(agent, "available", Value::Bool(false))?)
            || !contains(
                &py::get(agent, "policies", Value::List(Vec::new()))?,
                policy,
            )?
        {
            continue;
        }
        let name = model_name(agent)?;
        let (a, b) = prior_for(&priors, agent, &name, task_class)?;
        let mut runs = Table(Vec::new());
        for run in py::list(&py::get(agent, "runs", Value::List(Vec::new()))?)? {
            runs.insert(py::item(&run, "run_id")?.clone(), run)?;
        }
        let mut seen: Vec<Value> = Vec::new();
        let (mut applied, mut against) = (0i128, 0i128);
        for event in verdicts {
            let name = py::get(
                event,
                "event",
                py::get(event, "type", Value::text("human_verdict"))?,
            )?;
            if !py::eq_str(&name, "human_verdict") {
                continue;
            }
            let Some(run) = runs.get(&py::get(event, "run_id", Value::Null)?)? else {
                continue;
            };
            let result = py::get(event, "result", Value::Null)?;
            if !["applied", "fixed", "rejected"]
                .iter()
                .any(|known| py::eq_str(&result, known))
            {
                continue;
            }
            let event_task = py::get(event, "task", Value::Null)?;
            let own_class = task_classes
                .get(&event_task)?
                .cloned()
                .unwrap_or_else(|| task_class.clone());
            if !py::eq(&py::get(event, "task_class", own_class)?, task_class) {
                continue;
            }
            let Some(result) = first_builder_verdict(&mut seen, event, run)? else {
                continue;
            };
            if py::eq_str(&result, "applied") {
                applied += 1;
            } else {
                against += 1;
            }
        }
        let bound = beta_lower(a + applied, b + against)?;
        if bound < threshold {
            continue;
        }
        let cost = py::item(agent, "cost")?;
        let p50 = py::item(cost, "p50")?;
        py::item(cost, "n")?;
        result.push(Ranked {
            provider: py::item(agent, "provider")?.clone(),
            model: py::get(agent, "model", Value::Null)?,
            bound,
            score: as_float(p50)?.map(|cost| cost / bound),
            sample_size: usize::try_from(applied + against).unwrap_or(usize::MAX),
        });
    }
    let order = pysort::sorted(result.len(), |i, j| ranked_less(&result[i], &result[j]))?;
    Ok(order.into_iter().map(|i| result[i].clone()).collect())
}

/// `ranked_assignment`: the cheapest capable agent for a task, as a provider and a model,
/// from the sheets under the puppets home and this goal's own verdicts. `None` when there
/// is no sheet, nobody qualifies, or the first row has no verdict behind it. In each case
/// the caller falls back to the default table, which is the human's own memory of each
/// agent and wins until this goal has taught the sheet something.
///
/// The Python also returns `None` when `sheet.py` is missing. This crate does not need
/// that file, so its absence is not consulted here.
pub fn ranked_assignment(
    home: &Home,
    task: &TaskState,
    events: &[Value],
) -> Result<Option<(Value, Value)>> {
    let agents = home.puppets.join("agents");
    if !agents.is_dir() {
        return Ok(None);
    }
    let mut names = Vec::new();
    let entries = std::fs::read_dir(&agents)
        .map_err(|err| anyhow!("OSError: {}: {err}", agents.display()))?;
    for entry in entries {
        let name = entry
            .map_err(|err| anyhow!("OSError: {}: {err}", agents.display()))?
            .file_name();
        if name.as_encoded_bytes().ends_with(b".json") {
            names.push(name);
        }
    }
    // `sorted()` over paths orders them by name, code point by code point, which for
    // UTF-8 names is byte order.
    names.sort_by(|a, b| a.as_encoded_bytes().cmp(b.as_encoded_bytes()));
    let mut sheets = Vec::new();
    for name in &names {
        sheets.push(load(&agents.join(name))?);
    }
    if sheets.is_empty() {
        return Ok(None);
    }
    let rows = rank(
        &sheets,
        &task.task_class,
        &task.grade,
        events,
        "standard",
        &log::data_dir()?.join("priors.json"),
    )?;
    let Some(best) = rows.first() else {
        return Ok(None);
    };
    if best.sample_size == 0 {
        return Ok(None);
    }
    let model = if py::truthy(&best.model) {
        best.model.clone()
    } else {
        Value::text("")
    };
    Ok(Some((best.provider.clone(), model)))
}

/// Normalize a known human verdict; accepting a resumed run needed another attempt.
pub fn builder_verdict(event: &Value, run: &Value) -> Result<Option<Value>> {
    let result = py::get(event, "result", Value::Null)?;
    if !["applied", "fixed", "rejected"]
        .iter()
        .any(|known| py::eq_str(&result, known))
    {
        return Ok(None);
    }
    if py::eq_str(&result, "applied") && py::truthy(&py::get(run, "parent_run_id", Value::Null)?) {
        Ok(Some(Value::text("fixed")))
    } else {
        Ok(Some(result))
    }
}

/// The first known verdict per task, using the same task keys as the sheet ranking.
pub fn first_builder_verdict(
    seen: &mut Vec<Value>,
    event: &Value,
    run: &Value,
) -> Result<Option<Value>> {
    let Some(result) = builder_verdict(event, run)? else {
        return Ok(None);
    };
    let event_task = py::get(event, "task", Value::Null)?;
    let run_task = py::item(run, "task_id")?.clone();
    py::hashable(&run_task)?;
    let task = Value::Str(py::str(if py::truthy(&event_task) {
        &event_task
    } else {
        &run_task
    }));
    if seen
        .iter()
        .any(|s| py::same_key(s, &task) || py::same_key(s, &run_task))
    {
        return Ok(None);
    }
    seen.push(task);
    seen.push(run_task);
    Ok(Some(result))
}
#[cfg(test)]
mod tests {
    use super::*;

    fn close(got: f64, want: f64) {
        assert!((got - want).abs() < 1e-12, "{got} is not {want}");
    }

    /// The numbers are `sheet.beta_lower` on Python 3.14, to the digits it prints.
    #[test]
    fn beta_lower_is_the_tenth_percentile() {
        // Beta(1, 1) is uniform, and Beta(a, 1) has the closed form 0.1 ** (1 / a).
        close(beta_lower(1, 1).unwrap(), 0.1);
        close(beta_lower(6, 1).unwrap(), 0.1f64.powf(1.0 / 6.0));
        close(beta_lower(1, 4).unwrap(), 1.0 - 0.9f64.powf(0.25));
        close(beta_lower(4, 2).unwrap(), 0.4161096253804709);
        close(beta_lower(5, 3).unwrap(), 0.40382027215195604);
        // A zero `a` is no error in Python, and neither is a negative one with no term to sum.
        assert_eq!(beta_lower(0, 1).unwrap(), 4.336808689942018e-19);
        assert_eq!(beta_lower(1, -3).unwrap(), 1.0);
        assert!(beta_lower(-1, 5).is_err());
    }

    /// `math.lgamma` of Python 3.14, which is CPython's own Lanczos sum. The C library's
    /// `lgamma` on this machine gives 1.7917594692280552 for 4 and 4.787491742782045 for 6.
    #[test]
    fn lgamma_is_cpythons_and_not_the_c_librarys() {
        for (x, want) in [
            (1.0, 0.0),
            (2.0, 0.0),
            (4.0, 1.791759469228055),
            (5.0, 3.1780538303479444),
            (6.0, 4.787491742782047),
            (11.0, 15.104412573075514),
            (171.0, 706.5730622457874),
            (0.5, 0.5723649429247004),
        ] {
            assert_eq!(lgamma(x), want, "lgamma({x})");
        }
    }

    /// Build ticket 207. Summing logs for `ln(k!)` scored the first agent
    /// 1.9610965296978333 and ranked it second.
    #[test]
    fn near_tie_ranks_as_python_lgamma() {
        let first = 1.0 / beta_lower(7, 3).unwrap();
        let second = 1.102805621336474 / beta_lower(4, 1).unwrap();
        assert!(first < second, "{first:?} >= {second:?}");
        assert_eq!(first, 1.9610965296978324);
        assert_eq!(second, 1.9610965296978329);
    }

    fn sheet(provider: &str, model: &str, p50: &str, runs: &str) -> Value {
        json::loads_str(&format!(
            r#"{{"provider":"{provider}","model":"{model}","available":true,"policies":["standard"],"runs":{runs},"cost":{{"p50":{p50},"n":1}}}}"#
        ))
        .unwrap()
    }

    fn events(lines: &[&str]) -> Vec<Value> {
        lines.iter().map(|l| json::loads_str(l).unwrap()).collect()
    }

    #[test]
    fn rank_orders_by_cost_per_accepted_diff_and_puts_an_unknown_cost_last() {
        let priors = log::checkout_data_dir().join("priors.json");
        let r1 = r#"[{"run_id":"r1","task_id":"r1"}]"#;
        let sheets = [
            sheet("claude", "sonnet", "0.02", r1),
            sheet("claude", "Sonnet 5", "null", r1),
            sheet("claude", "sonnet-mini", "0.01", r1),
            // Beta(4, 2) sits at 0.416, under the 0.5 a `low` task asks for.
            sheet("cursor-agent", "composer-2.5", "0.001", "[]"),
        ];
        let verdicts = events(&[
            r#"{"event":"task","id":"t1","class":"docs"}"#,
            r#"{"event":"human_verdict","task":"t1","run_id":"r1","result":"applied"}"#,
            r#"{"event":"human_verdict","task":"t1","run_id":"r1","result":"rejected"}"#,
        ]);
        let class = Value::text("docs");
        let rows = rank(
            &sheets,
            &class,
            &Value::text("low"),
            &verdicts,
            "standard",
            &priors,
        )
        .unwrap();
        let models: Vec<String> = rows.iter().map(|r| py::str(&r.model).lossy()).collect();
        assert_eq!(models, ["sonnet-mini", "sonnet", "Sonnet 5"]);
        // One verdict per task: the second on t1 is not counted.
        assert!(rows.iter().all(|r| r.sample_size == 1));
        close(rows[0].bound, 0.1f64.powf(1.0 / 6.0));
        // A verdict on a task of another class says nothing about this one.
        let probe = Value::text("probe");
        let rows = rank(
            &sheets,
            &probe,
            &Value::text("low"),
            &verdicts,
            "standard",
            &priors,
        )
        .unwrap();
        assert!(rows.iter().all(|r| r.sample_size == 0));
        // `high` asks for 0.7 and Beta(6, 1) sits at 0.681.
        let rows = rank(
            &sheets,
            &class,
            &Value::text("high"),
            &verdicts,
            "standard",
            &priors,
        )
        .unwrap();
        assert!(rows.is_empty());
        assert!(
            rank(
                &sheets,
                &class,
                &Value::text("agi"),
                &verdicts,
                "standard",
                &priors
            )
            .unwrap()
            .is_empty()
        );
        let err = rank(
            &sheets,
            &class,
            &Value::text("huge"),
            &verdicts,
            "standard",
            &priors,
        )
        .unwrap_err();
        assert_eq!(err.to_string(), "KeyError: 'huge'");
    }

    #[test]
    fn an_applied_verdict_on_a_resumed_run_counts_against() {
        let priors = log::checkout_data_dir().join("priors.json");
        let verdicts =
            events(&[r#"{"event":"human_verdict","task":"t1","run_id":"r2","result":"applied"}"#]);
        let rows = |runs: &str| {
            rank(
                &[sheet("claude", "sonnet", "0.02", runs)],
                &Value::text("docs"),
                &Value::text("low"),
                &verdicts,
                "standard",
                &priors,
            )
            .unwrap()
        };
        // A first run makes it Beta(6, 1), at 0.681. A resumed one makes it Beta(5, 2), at
        // 0.490, under the 0.5 a `low` task asks for.
        let first = rows(r#"[{"run_id":"r2","task_id":"r2"}]"#);
        close(first[0].bound, beta_lower(6, 1).unwrap());
        assert!(rows(r#"[{"run_id":"r2","task_id":"r1","parent_run_id":"r1"}]"#).is_empty());
        close(beta_lower(5, 2).unwrap(), 0.4896836934485084);
    }
}
