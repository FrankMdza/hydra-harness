//! The review panel: three seats that judge the proposals for `choose --judge` and a task's
//! diff for `review`, and what `review` does with their findings. Each seat is one
//! `hydra run`, waited for before the next starts. The prompts, the parsing of an answer and
//! the verdict are the Python helper's, and so is every argv handed to `hydra`, `git`,
//! `hunk` and `herdr`.

use std::ffi::{OsStr, OsString};
use std::io::{Read, Write};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

use anyhow::{Result, anyhow, bail};

use crate::fold::{self, GoalState, approval_available};
use crate::json::{self, Dict, ParseError, PyStr, Value};
use crate::launch::{
    answer_text, build_run_argv, capture, decoded, find, goal_field, herdr_pane, hydra_show,
    hydra_show_text, loads_or_raise, os_argv, pathlib, read_text, run_file_has_end, run_id_of,
    show_failed, spawn_error, spawn_run, split_pane_id, string,
};
use crate::log::Home;
use crate::py;
use crate::read::{Piece, print, text};
use crate::write::{self, append_event, arg, body, eprint, field, folded, fs_path, os_error};

/// One seat of the review panel: who sits, under which policy, in which role, and what it
/// is shown.
struct Seat {
    provider: &'static str,
    model: Option<&'static str>,
    policy: &'static str,
    role: Role,
    evidence: Evidence,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Role {
    Reviewer,
    Adversary,
    Security,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Evidence {
    DiffAndTask,
    Diff,
    DiffAndTests,
}

/// `PUPPETS_PANEL_SEATS`. Rung two's panel in `src/rungs/review.rs` keeps its own table and
/// never substitutes; only this panel may sit degraded with the human's approval.
const PANEL: [Seat; 3] = [
    Seat {
        provider: "claude",
        model: Some("opus"),
        policy: "judge",
        role: Role::Reviewer,
        evidence: Evidence::DiffAndTask,
    },
    Seat {
        provider: "codex",
        model: None,
        policy: "standard",
        role: Role::Adversary,
        evidence: Evidence::Diff,
    },
    Seat {
        provider: "cursor-agent",
        model: Some("composer-2.5"),
        policy: "judge",
        role: Role::Security,
        evidence: Evidence::DiffAndTests,
    },
];

const TIE_BREAK: [Seat; 2] = [
    Seat {
        provider: "claude",
        model: Some("claude-fable-5-1"),
        policy: "judge",
        role: Role::Reviewer,
        evidence: Evidence::DiffAndTask,
    },
    Seat {
        provider: "codex",
        model: Some("gpt-6-astra"),
        policy: "judge",
        role: Role::Reviewer,
        evidence: Evidence::DiffAndTask,
    },
];

impl Role {
    /// `PANEL_ROLE_BRIEF`.
    fn brief(self) -> &'static str {
        match self {
            Role::Reviewer => {
                "Your role is reviewer: decide whether the diff does what the task asked, \
                 naming what is missing if not."
            }
            Role::Adversary => {
                "Your role is adversary: argue the diff should be rejected; name the strongest reason."
            }
            Role::Security => {
                "Your role is security: apply the checklist items about secrets, paths, gates and the record."
            }
        }
    }
}

/// `SECURITY_CHECKLIST_ITEMS`: the rows of the checklist the security seat is given.
const SECURITY_CHECKLIST_ITEMS: [&str; 4] = [
    "security_issues",
    "baseline_test_provenance",
    "tests_present",
    "scope_creep",
];

/// `TASK_DIFF_CHECKLIST`: an item, whether it is advisory, and its definition. A finding on
/// an advisory item is never worse than minor.
const TASK_DIFF_CHECKLIST: [(&str, bool, &str); 12] = [
    (
        "scope_match",
        false,
        "The diff implements what the task asked for, naming what is missing if not",
    ),
    (
        "scope_creep",
        true,
        "Changes outside the task scope or its named paths without justification",
    ),
    (
        "tests_present",
        false,
        "A behaviour change with no test touched, naming the function and file that should have moved",
    ),
    (
        "baseline_test_provenance",
        false,
        "A weakened assertion or a test edited without comparing against the baseline",
    ),
    (
        "security_issues",
        false,
        "A credential, unsafe construct, or shell interpolation of untrusted text",
    ),
    (
        "error_handling",
        true,
        "A recoverable error path silently swallowed or misreported",
    ),
    (
        "dead_code",
        true,
        "Obvious dead code, unused imports, or unreachable branches introduced",
    ),
    (
        "naming_clarity",
        true,
        "New identifiers that are unclear or fight repo conventions",
    ),
    (
        "self_consistency",
        false,
        "The diff contradicts the task text or its own structure",
    ),
    (
        "dependencies",
        true,
        "A new dependency the task did not require",
    ),
    (
        "documentation",
        true,
        "A user-visible change with no doc update where the task touched docs",
    ),
    (
        "code_quality",
        true,
        "Unnecessary duplication or complexity beyond what the task needs",
    ),
];

/// How many times, a second apart, the findings are offered to a Hunk session.
const HUNK_TRIES: usize = 30;

/// `str.splitlines()`: every boundary Python counts as a line end, and no empty last line.
fn splitlines(text: &str) -> Vec<&str> {
    const ENDS: [char; 10] = [
        '\n', '\r', '\u{b}', '\u{c}', '\u{1c}', '\u{1d}', '\u{1e}', '\u{85}', '\u{2028}',
        '\u{2029}',
    ];
    let mut lines = Vec::new();
    let mut rest = text;
    while !rest.is_empty() {
        match rest.find(ENDS) {
            Some(at) => {
                lines.push(&rest[..at]);
                let end = if rest[at..].starts_with("\r\n") {
                    at + 2
                } else {
                    at + rest[at..].chars().next().map_or(1, char::len_utf8)
                };
                rest = &rest[end..];
            }
            None => {
                lines.push(rest);
                break;
            }
        }
    }
    lines
}

/// A seat as it sits for one command: its own provider, or the substitute a degraded
/// approval names for a provider that is down.
struct Resolved {
    seat: &'static Seat,
    provider: Value,
    model: Value,
    policy: &'static str,
    /// The provider this one stands in for.
    missing: Option<&'static str>,
}

/// The panel as it sits for one command, and the substitutions that filled it.
struct Sitting {
    seats: Vec<Resolved>,
    substitutions: Vec<Value>,
}

/// `resolve_panel_seats`: the three seats, the substitutions made, or the refusal. A seat
/// whose provider `vendors.json` marks down needs a live degraded approval that names a
/// substitute. The substitute keeps its own seat's policy, or `judge`, never the missing
/// seat's: codex sits under `standard` only because `judge` blocked its first read, and a
/// stand-in has no reason to inherit write and network access.
fn resolve_panel_seats(
    home: &Home,
    state: &GoalState,
) -> Result<std::result::Result<Sitting, String>> {
    resolve_seats(home, state, &PANEL)
}

fn resolve_seats(
    home: &Home,
    state: &GoalState,
    seats: &'static [Seat],
) -> Result<std::result::Result<Sitting, String>> {
    let mut resolved = Vec::new();
    let mut substitutions = Vec::new();
    let vendors = home.vendors_file()?;
    for seat in seats {
        if fold::provider_available(&vendors, seat.provider)? {
            resolved.push(Resolved {
                seat,
                provider: Value::text(seat.provider),
                model: seat.model.map_or(Value::Null, Value::text),
                policy: seat.policy,
                missing: None,
            });
            continue;
        }
        let missing = Value::text(seat.provider);
        let Some(approval) = approval_available(state, "degraded", &missing, &Value::Null) else {
            return Ok(Err(format!(
                "{} is unavailable; approve with: approve {} --degraded {}={}",
                seat.provider,
                state.goal_id,
                seat.provider,
                fold::DEGRADED_SUBSTITUTE_PLACEHOLDER
            )));
        };
        let substitute = field(approval, "substitute");
        if !py::truthy(&substitute) {
            return Ok(Err(format!(
                "degraded approval for {} has no substitute",
                seat.provider
            )));
        }
        let model = field(approval, "model");
        let own = PANEL
            .iter()
            .find(|other| py::eq_str(&substitute, other.provider))
            .map_or("judge", |other| other.policy);
        substitutions.push(Value::Dict(body(vec![
            ("missing", missing),
            ("substitute", substitute.clone()),
            (
                "model",
                if py::truthy(&model) {
                    model.clone()
                } else {
                    Value::text("")
                },
            ),
        ])));
        resolved.push(Resolved {
            seat,
            provider: substitute,
            model: if py::truthy(&model) {
                model
            } else {
                Value::Null
            },
            policy: own,
            missing: Some(seat.provider),
        });
    }
    Ok(Ok(Sitting {
        seats: resolved,
        substitutions,
    }))
}

/// What a panel left: one run id, one policy and one answer per seat.
struct PanelRuns {
    run_ids: Vec<String>,
    policies: Vec<&'static str>,
    texts: Vec<Value>,
}

impl PanelRuns {
    fn event_fields(&self) -> [(&'static str, Value); 2] {
        let ids = self.run_ids.iter().map(|id| Value::text(id)).collect();
        let policies = self.policies.iter().map(|p| Value::text(p)).collect();
        [
            ("judge_run_ids", Value::List(ids)),
            ("judge_policies", Value::List(policies)),
        ]
    }
}

/// `run_judge_panel`: one `hydra run` per seat, in order, each waited for before the next
/// starts. A run is over when its record holds a `run_end`; nothing here gives up on one.
fn run_judge_panel(home: &Home, seat_prompts: Vec<(&Resolved, PyStr)>) -> Result<PanelRuns> {
    let mut runs = PanelRuns {
        run_ids: Vec::new(),
        policies: Vec::new(),
        texts: Vec::new(),
    };
    for (seat, prompt) in seat_prompts {
        let argv = build_run_argv(
            &seat.provider,
            &seat.model,
            &Value::text(seat.policy),
            prompt,
        )?;
        let run_id = spawn_run(&argv)?;
        let id = PyStr::from(run_id.as_str());
        runs.run_ids.push(run_id);
        runs.policies.push(seat.policy);
        while !run_file_has_end(home, &id)? {
            std::thread::sleep(Duration::from_millis(50));
        }
        runs.texts.push(hydra_show_text(&id)?);
    }
    Ok(runs)
}

/// The `judge <provider> (<policy>) run <id>` line per seat, naming the provider a
/// substitute sits for.
fn print_judges(out: &mut dyn Write, resolved: &[Resolved], runs: &PanelRuns) -> Result<()> {
    for ((seat, policy), run_id) in resolved.iter().zip(&runs.policies).zip(&runs.run_ids) {
        let note = seat
            .missing
            .map_or(String::new(), |missing| format!(" for {missing}"));
        print(
            out,
            &text!(
                "judge ",
                py::str(&seat.provider),
                note,
                " (",
                *policy,
                ") run ",
                run_id.as_str()
            ),
        )?;
    }
    Ok(())
}

fn starts_with(codes: &[u32], prefix: &str) -> bool {
    let prefix = PyStr::from(prefix);
    codes.starts_with(prefix.code_points())
}

fn is_space(code: u32) -> bool {
    char::from_u32(code).is_some_and(py::is_space)
}

/// `strip_one_fence`: the text stripped, and when it is exactly one Markdown fence, tagged
/// `json` or not at all, what the fence holds.
fn strip_one_fence(text: &PyStr) -> PyStr {
    const FENCE: &str = "```";
    let trimmed = py::strip(text);
    let codes = trimmed.code_points();
    if !starts_with(codes, FENCE) {
        return trimmed;
    }
    let rest = &codes[3..];
    let Some(newline) = rest.iter().position(|code| *code == 0xa) else {
        return trimmed;
    };
    let tag = py::strip(&write::slice(&rest[..newline]));
    let json_tag = tag
        .to_utf8()
        .is_some_and(|tag| tag.eq_ignore_ascii_case("json"));
    if !tag.is_empty() && !json_tag {
        return trimmed;
    }
    let mut inner = &rest[newline + 1..];
    while inner.last().is_some_and(|code| is_space(*code)) {
        inner = &inner[..inner.len() - 1];
    }
    let fence = PyStr::from(FENCE);
    if !inner.ends_with(fence.code_points()) {
        return trimmed;
    }
    inner = &inner[..inner.len() - 3];
    if find(inner, fence.code_points()).is_some() {
        return trimmed;
    }
    py::strip(&write::slice(inner))
}

const SEVERITIES: [&str; 3] = ["minor", "major", "critical"];

struct Finding {
    item: PyStr,
    /// An index into `SEVERITIES`, which is in order.
    severity: usize,
    note: PyStr,
}

/// One seat's answer about a diff: its verdict and findings, or why it did not parse.
struct JudgeOutcome {
    provider: PyStr,
    blocks: bool,
    findings: Vec<Finding>,
    parse_error: Option<PyStr>,
}

/// A JSON answer, or `str(err)` for the `JSONDecodeError`. Any other failure is raised.
fn decode_answer(
    parsed: std::result::Result<Value, (ParseError, String)>,
) -> Result<std::result::Result<Value, PyStr>> {
    match parsed {
        Ok(value) => Ok(Ok(value)),
        Err((ParseError::Decode, message)) => Ok(Err(PyStr::from(message))),
        Err((err, _)) => bail!("{}: a judge's answer", err.class()),
    }
}

/// The `item`, `severity` and `note` of one raw finding, or the parse error. `ceiling`
/// gives the worst severity an item may carry, or refuses the item.
fn finding(
    raw: &Value,
    ceiling: impl Fn(&PyStr) -> std::result::Result<usize, PyStr>,
) -> std::result::Result<Finding, PyStr> {
    let Value::Dict(raw) = raw else {
        return Err("finding is not an object".into());
    };
    let (Some(Value::Str(item)), Some(Value::Str(note))) = (raw.get("item"), raw.get("note"))
    else {
        return Err("finding fields must be strings".into());
    };
    let ceiling = ceiling(item)?;
    let severity = field(raw, "severity");
    let Some(severity) = SEVERITIES
        .iter()
        .position(|name| py::eq_str(&severity, name))
    else {
        return Err(text!(
            "severity ",
            py::repr(&severity),
            " is not critical, major or minor"
        ));
    };
    Ok(Finding {
        item: item.clone(),
        severity: severity.min(ceiling),
        note: note.clone(),
    })
}

/// `parse_judge_answer`: one JSON object, `verdict` and `findings`, each finding on a
/// checklist item with a severity no worse than the item allows.
fn parse_judge_answer(provider: PyStr, answer: &PyStr) -> Result<JudgeOutcome> {
    let parsed = decode_answer(json::loads_explained(&strip_one_fence(answer)))?;
    let read = || -> std::result::Result<(bool, Vec<Finding>), PyStr> {
        let Value::Dict(parsed) = parsed? else {
            return Err("answer is not a JSON object".into());
        };
        let verdict = field(&parsed, "verdict");
        if !py::eq_str(&verdict, "pass") && !py::eq_str(&verdict, "block") {
            return Err(text!(
                "verdict ",
                py::repr(&verdict),
                " is not pass or block"
            ));
        }
        let Some(Value::List(raw)) = parsed.get("findings") else {
            return Err("findings is not an array".into());
        };
        let ceiling = |item: &PyStr| {
            TASK_DIFF_CHECKLIST
                .iter()
                .find(|(id, _, _)| item == id)
                .map(|(_, advisory, _)| if *advisory { 0 } else { 2 })
                .ok_or_else(|| {
                    text!(
                        "unknown checklist item ",
                        py::repr(&Value::Str(item.clone()))
                    )
                })
        };
        let findings = raw
            .iter()
            .map(|raw| finding(raw, ceiling))
            .collect::<std::result::Result<_, _>>()?;
        Ok((py::eq_str(&verdict, "block"), findings))
    };
    Ok(match read() {
        Ok((blocks, findings)) => JudgeOutcome {
            provider,
            blocks,
            findings,
            parse_error: None,
        },
        Err(why) => JudgeOutcome {
            provider,
            blocks: false,
            findings: Vec::new(),
            parse_error: Some(why),
        },
    })
}

/// `diff_panel_verdict`: `block` with its reasons when any seat failed to parse, said
/// block, or reported a critical finding; `pass` otherwise.
fn diff_panel_verdict(outcomes: &[JudgeOutcome]) -> (&'static str, Vec<PyStr>) {
    let mut reasons = Vec::new();
    for outcome in outcomes {
        let provider = &outcome.provider;
        if let Some(why) = &outcome.parse_error {
            reasons.push(text!(provider.clone(), ": parse error: ", why.clone()));
            continue;
        }
        if outcome.blocks {
            reasons.push(text!(provider.clone(), ": verdict block"));
        }
        for finding in &outcome.findings {
            if SEVERITIES[finding.severity] == "critical" {
                reasons.push(text!(
                    provider.clone(),
                    ": critical ",
                    finding.item.clone(),
                    ": ",
                    finding.note.clone()
                ));
            }
        }
    }
    if reasons.is_empty() {
        ("pass", reasons)
    } else {
        ("block", reasons)
    }
}

/// One seat's answer about the proposals: the name it chose, and the plan it composed when
/// it chose `composed` and its answer holds a fenced task array.
struct PlanAnswer {
    chosen: std::result::Result<PyStr, PyStr>,
    composed_plan: Option<String>,
}

/// The fenced task array of a `composed` answer as `json.dumps(tasks, indent=2)`, or `None`
/// where `extract_tasks_json` raises `ValueError`.
fn composed_plan(answer: &PyStr) -> Result<Option<String>> {
    let chars: Vec<char> = answer
        .code_points()
        .iter()
        .map(|code| char::from_u32(*code).unwrap_or('\u{fffd}'))
        .collect();
    let Some(found) = write::fenced_array(&chars) else {
        return Ok(None);
    };
    match json::loads(&write::slice(&answer.code_points()[found])) {
        Ok(tasks) => Ok(Some(json::dumps(&tasks, Some(2)))),
        Err(ParseError::Recursion) => bail!("RecursionError: the composed plan's task array"),
        Err(_) => Ok(None),
    }
}

/// `parse_plan_answer`: a JSON object at the start of the answer, with `chosen` a name and
/// `findings` well formed. What follows the object is not read as JSON.
fn parse_plan_answer(answer: &PyStr) -> Result<PlanAnswer> {
    let trimmed = strip_one_fence(&py::strip(answer));
    let parsed = decode_answer(json::raw_decode(&trimmed).map(|(value, _)| value))?;
    let read = || -> std::result::Result<PyStr, PyStr> {
        let Value::Dict(parsed) = parsed? else {
            return Err("answer is not a JSON object".into());
        };
        let chosen = match parsed.get("chosen") {
            Some(Value::Str(chosen)) if !py::strip(chosen).is_empty() => chosen.clone(),
            _ => return Err("chosen must be a non-empty string".into()),
        };
        let none = Value::List(Vec::new());
        let Value::List(raw) = parsed.get("findings").unwrap_or(&none) else {
            return Err("findings is not an array".into());
        };
        for raw in raw {
            finding(raw, |_| Ok(2))?;
        }
        Ok(chosen)
    };
    let chosen = read();
    let composed_plan = match &chosen {
        Ok(chosen) if *chosen == "composed" => composed_plan(answer)?,
        _ => None,
    };
    Ok(PlanAnswer {
        chosen,
        composed_plan,
    })
}

/// `plan_panel_verdict`: the name every seat chose and, for `composed`, the first composed
/// plan; or the reasons there is no verdict. The reasons name the panel's own seats, not a
/// substitute.
fn plan_panel_verdict(
    answers: &[PlanAnswer],
) -> std::result::Result<(PyStr, Option<String>), Vec<PyStr>> {
    let mut reasons = Vec::new();
    let mut chosen = Vec::new();
    for (seat, answer) in PANEL.iter().zip(answers) {
        match &answer.chosen {
            Ok(name) => chosen.push((seat.provider, name)),
            Err(why) => reasons.push(text!(seat.provider, ": parse error: ", why.clone())),
        }
    }
    if !reasons.is_empty() {
        return Err(reasons);
    }
    let Some(&(_, first)) = chosen.first() else {
        return Err(vec!["disagreement: ".into()]);
    };
    if chosen.iter().any(|(_, name)| *name != first) {
        let mut pairs = PyStr::from("disagreement: ");
        for (n, (provider, name)) in chosen.iter().enumerate() {
            if n > 0 {
                pairs.push_str(", ");
            }
            pairs.push(&text!(*provider, "=", (*name).clone()));
        }
        return Err(vec![pairs]);
    }
    if *first != "composed" {
        return Ok((first.clone(), None));
    }
    match answers.iter().find_map(|a| a.composed_plan.clone()) {
        Some(plan) if !plan.is_empty() => Ok((first.clone(), Some(plan))),
        _ => Err(vec!["composed chosen but no task block found".into()]),
    }
}

fn role_values(seats: &[Seat], count: usize) -> Value {
    Value::List(
        seats
            .iter()
            .take(count)
            .map(|seat| {
                Value::text(match seat.role {
                    Role::Reviewer => "reviewer",
                    Role::Adversary => "adversary",
                    Role::Security => "security",
                })
            })
            .collect(),
    )
}

fn panel_roles(runs: &PanelRuns, tie: &PanelRuns) -> Vec<(&'static str, Value)> {
    vec![
        ("judge_roles", role_values(&PANEL, runs.run_ids.len())),
        (
            "tie_break_roles",
            role_values(&TIE_BREAK, tie.run_ids.len()),
        ),
    ]
}

fn tie_break_fields(runs: &PanelRuns) -> Vec<(&'static str, Value)> {
    vec![
        (
            "tie_break_run_ids",
            Value::List(runs.run_ids.iter().map(|id| Value::text(id)).collect()),
        ),
        ("tie_break_answers", Value::List(runs.texts.clone())),
    ]
}

fn plan_verdict_fields(answers: &[PlanAnswer], tie: &[PlanAnswer]) -> Vec<(&'static str, Value)> {
    let verdict = choose_panel_verdict(answers, tie);
    let source = if verdict.is_err() {
        Value::Null
    } else {
        Value::text(if tie.is_empty() { "panel" } else { "tie_break" })
    };
    let chosen = verdict
        .as_ref()
        .map_or(Value::Null, |(chosen, _)| Value::Str(chosen.clone()));
    vec![("chosen", chosen), ("verdict_source", source)]
}

/// `build_choose_seat_prompt`: what one seat is asked about the proposals.
fn build_choose_seat_prompt(
    seat: &Seat,
    goal_text: &PyStr,
    proposal_blocks: &PyStr,
    provider_names: &PyStr,
) -> PyStr {
    let mut prompt = text!(
        "You are one judge on a three-seat panel choosing a work plan.\n\n",
        "## Role\n\n",
        seat.role.brief(),
        "\n\n"
    );
    if seat.evidence == Evidence::DiffAndTask {
        prompt.push(&text!("## Goal\n\n", goal_text.clone(), "\n\n"));
    }
    prompt.push(&text!("## Proposals\n\n", proposal_blocks.clone(), "\n"));
    if seat.evidence == Evidence::DiffAndTests {
        prompt.push_str(
            "## Note\n\nWeigh test and verification coverage in each proposal before you choose.\n\n",
        );
    }
    prompt.push(&text!(
        "## Instructions\n\n",
        "Pick exactly one proposal label (",
        provider_names.clone(),
        ") or `composed` when no single proposal is best.\n\n",
        "Answer in exactly one JSON object and nothing else:\n\n",
        "```json\n{\"chosen\": \"<label>|composed\", \"findings\": []}\n```\n"
    ));
    prompt
}

/// `", ".join(parts)`, or any other separator.
fn join(parts: &[PyStr], separator: &str) -> PyStr {
    let mut out = PyStr::new();
    for (n, part) in parts.iter().enumerate() {
        if n > 0 {
            out.push_str(separator);
        }
        out.push(part);
    }
    out
}

fn proposal_block(n: usize, plan_text: PyStr) -> PyStr {
    text!("### ", proposal_label(n), "\n\n", plan_text, "\n")
}

fn proposal_label(n: usize) -> PyStr {
    let mut n = n + 1;
    let mut letters = Vec::new();
    while n > 0 {
        n -= 1;
        letters.push((b'A' + (n % 26) as u8) as char);
        n /= 26;
    }
    PyStr::from(format!(
        "Proposal {}",
        letters.into_iter().rev().collect::<String>()
    ))
}

fn proposal_order(count: usize, seat: usize) -> Vec<usize> {
    (0..count).map(|n| (n + seat) % count).collect()
}

fn ordered_blocks(blocks: &[PyStr], order: &[usize]) -> PyStr {
    join(
        &order.iter().map(|n| blocks[*n].clone()).collect::<Vec<_>>(),
        "\n",
    )
}

fn parse_blind_answer(answer: &PyStr, providers: &[Value]) -> Result<PlanAnswer> {
    let mut parsed = parse_plan_answer(answer)?;
    if let Ok(label) = &parsed.chosen
        && *label != "composed"
    {
        parsed.chosen = providers
            .iter()
            .enumerate()
            .find(|(n, _)| *label == proposal_label(*n))
            .map(|(_, provider)| py::str(provider))
            .ok_or_else(|| text!("unknown proposal label: ", label.clone()));
    }
    Ok(parsed)
}

fn choose_panel_verdict(
    answers: &[PlanAnswer],
    tie: &[PlanAnswer],
) -> std::result::Result<(PyStr, Option<String>), Vec<PyStr>> {
    match plan_panel_verdict(answers) {
        Ok(verdict) => Ok(verdict),
        Err(reasons) if tie.len() == 2 => plan_panel_verdict(tie).map_err(|_| reasons),
        Err(reasons) => Err(reasons),
    }
}

/// `choose --judge`: the panel reads every finished proposal and names one, or `composed`
/// with a task array of its own. A split goes to two blind tie-break judges.
pub fn choose_judge(
    home: &Home,
    goal_id: &str,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> Result<i32> {
    let state = folded(home, goal_id)?;
    let Some(goal) = &state.goal else {
        writeln!(err, "unknown goal {goal_id}")?;
        return Ok(1);
    };
    let ready: Vec<&Dict> = state
        .plans
        .iter()
        .filter(|plan| py::truthy(&field(plan, "path")))
        .collect();
    if ready.len() < 2 {
        writeln!(err, "need at least two finished proposals before judging")?;
        return Ok(1);
    }
    let Sitting {
        seats: resolved,
        mut substitutions,
    } = match resolve_panel_seats(home, &state)? {
        Ok(sitting) => sitting,
        Err(refusal) => {
            writeln!(err, "{refusal}")?;
            return Ok(1);
        }
    };
    let goal_text = py::str(&goal_field(goal, "text")?);
    let (mut blocks, mut providers) = (Vec::new(), Vec::new());
    for plan in &ready {
        let provider = goal_field(plan, "provider")?;
        let path = match field(plan, "path") {
            Value::Str(path) => fs_path(&path)?,
            other => bail!(
                "TypeError: expected str, bytes or os.PathLike object, not {}",
                py::type_name(&other)
            ),
        };
        let plan_text = py::strip(&read_text(&path)?);
        blocks.push(proposal_block(blocks.len(), plan_text));
        providers.push(provider);
    }
    let labels = (0..providers.len()).map(proposal_label).collect::<Vec<_>>();
    let provider_names = join(&labels, ", ");
    let orders = (0..resolved.len())
        .map(|n| proposal_order(blocks.len(), n))
        .collect::<Vec<_>>();
    let seat_prompts = resolved
        .iter()
        .zip(&orders)
        .map(|(seat, order)| {
            (
                seat,
                build_choose_seat_prompt(
                    seat.seat,
                    &goal_text,
                    &ordered_blocks(&blocks, order),
                    &provider_names,
                ),
            )
        })
        .collect();
    let runs = run_judge_panel(home, seat_prompts)?;
    let answers = runs
        .texts
        .iter()
        .map(|answer| parse_blind_answer(answer_text(answer)?, &providers))
        .collect::<Result<Vec<_>>>()?;
    print_judges(out, &resolved, &runs)?;
    let mut tie_runs = PanelRuns {
        run_ids: Vec::new(),
        policies: Vec::new(),
        texts: Vec::new(),
    };
    let mut tie_orders = Vec::new();
    let mut tie_answers = Vec::new();
    if plan_panel_verdict(&answers).is_err() {
        let Sitting {
            mut seats,
            substitutions: extra,
        } = match resolve_seats(home, &state, &TIE_BREAK)? {
            Ok(sitting) => sitting,
            Err(refusal) => {
                writeln!(err, "{refusal}")?;
                return Ok(1);
            }
        };
        // Tie-break substitutes always stay under judge, including a codex substitute.
        for seat in &mut seats {
            seat.policy = "judge";
        }
        substitutions.extend(extra);
        tie_orders = vec![
            (0..blocks.len()).collect::<Vec<_>>(),
            (0..blocks.len()).rev().collect::<Vec<_>>(),
        ];
        tie_runs = run_judge_panel(
            home,
            seats
                .iter()
                .zip(&tie_orders)
                .map(|(seat, order)| {
                    (
                        seat,
                        build_choose_seat_prompt(
                            seat.seat,
                            &goal_text,
                            &ordered_blocks(&blocks, order),
                            &provider_names,
                        ),
                    )
                })
                .collect(),
        )?;
        print_judges(out, &seats, &tie_runs)?;
        tie_answers = tie_runs
            .texts
            .iter()
            .map(|answer| parse_blind_answer(answer_text(answer)?, &providers))
            .collect::<Result<Vec<_>>>()?;
    }
    let verdict = choose_panel_verdict(&answers, &tie_answers);
    let evidence = || {
        vec![
            (
                "label_map",
                Value::Dict({
                    let mut map = Dict::new();
                    for (label, provider) in labels.iter().zip(&providers) {
                        map.set(&label.lossy(), provider.clone());
                    }
                    map
                }),
            ),
            (
                "seat_orders",
                Value::List(
                    orders
                        .iter()
                        .map(|order| {
                            Value::List(
                                order
                                    .iter()
                                    .map(|n| Value::Str(labels[*n].clone()))
                                    .collect(),
                            )
                        })
                        .collect(),
                ),
            ),
            (
                "tie_break_orders",
                Value::List(
                    tie_orders
                        .iter()
                        .map(|order| {
                            Value::List(
                                order
                                    .iter()
                                    .map(|n| Value::Str(labels[*n].clone()))
                                    .collect(),
                            )
                        })
                        .collect(),
                ),
            ),
        ]
    };
    let (chosen, composed) = match &verdict {
        Ok(verdict) => verdict.clone(),
        Err(reasons) => {
            let [ids, policies] = runs.event_fields();
            let mut event = vec![
                ("event", Value::text("panel_verdict")),
                ids,
                policies,
                ("chosen", Value::Null),
                ("path", Value::Null),
                ("degraded", Value::Bool(!substitutions.is_empty())),
                ("substitutions", Value::List(substitutions.clone())),
            ];
            event.extend(plan_verdict_fields(&answers, &tie_answers));
            event.extend(evidence());
            event.extend(panel_roles(&runs, &tie_runs));
            event.extend(tie_break_fields(&tie_runs));
            append_event(home, goal_id, body(event))?;
            for reason in reasons {
                eprint(err, reason)?;
            }
            return Ok(1);
        }
    };
    let path = match composed {
        Some(plan) => {
            let dir = home.puppets.join(goal_id).join("plans");
            std::fs::create_dir_all(&dir)
                .map_err(|e| anyhow!("{}: {}", os_error(&e), dir.display()))?;
            let dest = pathlib(&dir.join("composed.md"));
            std::fs::write(&dest, format!("Composed plan.\n\n```json\n{plan}\n```\n"))
                .map_err(|e| anyhow!("{}: {}", os_error(&e), dest.display()))?;
            Value::Str(arg(dest.as_os_str()))
        }
        None => {
            let named = Value::Str(chosen.clone());
            let mut path = Value::Null;
            for plan in &ready {
                if py::eq(&goal_field(plan, "provider")?, &named) {
                    path = field(plan, "path");
                    break;
                }
            }
            if matches!(path, Value::Null) {
                eprint(
                    err,
                    &text!("no plan file for chosen provider ", chosen.clone()),
                )?;
                return Ok(1);
            }
            path
        }
    };
    let [ids, policies] = runs.event_fields();
    let mut event = vec![
        ("event", Value::text("panel_verdict")),
        ids,
        policies,
        ("chosen", Value::Str(chosen.clone())),
        ("path", path.clone()),
        ("degraded", Value::Bool(!substitutions.is_empty())),
        ("substitutions", Value::List(substitutions)),
    ];
    event.extend(plan_verdict_fields(&answers, &tie_answers));
    event.extend(evidence());
    event.extend(panel_roles(&runs, &tie_runs));
    event.extend(tie_break_fields(&tie_runs));
    append_event(home, goal_id, body(event))?;
    print(out, &text!("chosen ", chosen, " -> ", py::str(&path)))?;
    Ok(0)
}

/// One `git` question from the repository, as `subprocess.check_output(..., text=True,
/// stderr=DEVNULL)`. `None` where the Python catches the failure: git exits nonzero, or
/// there is no git or no such directory.
fn git(repo: &Path, args: &[&OsStr]) -> Result<Option<String>> {
    let output = Command::new("git")
        .args(args)
        .current_dir(repo)
        .stdin(Stdio::inherit())
        .stderr(Stdio::null())
        .output();
    let output = match output {
        Ok(output) => output,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(spawn_error(OsStr::new("git"), &err)),
    };
    if !output.status.success() {
        return Ok(None);
    }
    decoded(&output.stdout).map(Some)
}

/// `<base>...<branch>`, or `None` when either is empty, where the Python asks git nothing.
fn diff_range(base_sha: &Value, branch: &Value) -> Result<Option<OsString>> {
    if !py::truthy(base_sha) || !py::truthy(branch) {
        return Ok(None);
    }
    let range = text!(py::str(base_sha), "...", py::str(branch));
    Ok(Some(fs_path(&range)?.into_os_string()))
}

/// `git_diff_branch`: what the branch changed since it left the base, or nothing.
fn git_diff_branch(repo: &Path, base_sha: &Value, branch: &Value) -> Result<String> {
    let Some(range) = diff_range(base_sha, branch)? else {
        return Ok(String::new());
    };
    Ok(git(repo, &[OsStr::new("diff"), &range])?.unwrap_or_default())
}

/// `first_changed_file`: the first path the branch changed.
fn first_changed_file(repo: &Path, base_sha: &Value, branch: &Value) -> Result<Option<String>> {
    let Some(range) = diff_range(base_sha, branch)? else {
        return Ok(None);
    };
    let names = git(
        repo,
        &[OsStr::new("diff"), OsStr::new("--name-only"), &range],
    )?;
    Ok(names.and_then(|names| {
        splitlines(py::strip_str(&names))
            .first()
            .map(|first| first.to_string())
    }))
}

/// `test_paths_from_diff`: the hunks of a unified diff whose paths look like tests.
fn test_paths_from_diff(diff: &str) -> String {
    const NONE: &str = "(no test paths in diff)";
    if py::strip_str(diff).is_empty() {
        return NONE.into();
    }
    let mut keep: Vec<&str> = Vec::new();
    let mut current: Vec<&str> = Vec::new();
    for line in splitlines(diff) {
        if line.starts_with("diff --git ") || line.starts_with("--- ") || line.starts_with("+++ ") {
            let last = line
                .split(py::is_space)
                .rfind(|bit| !bit.is_empty())
                .unwrap_or("");
            let path = last.strip_prefix("b/").unwrap_or(last);
            let is_test = path.contains("/tests/")
                || path.starts_with("tests/")
                || path.contains("test_")
                || path.ends_with("_test.rs")
                || path.contains("/test/");
            if is_test {
                if !current.is_empty() {
                    keep.append(&mut current);
                    keep.push("");
                }
                current = vec![line];
            } else {
                current.clear();
            }
        } else if !current.is_empty()
            && (line.starts_with("@@") || line.starts_with(['+', '-', ' ']))
        {
            current.push(line);
        }
    }
    keep.append(&mut current);
    match py::strip_str(&keep.join("\n")) {
        "" => NONE.into(),
        kept => kept.to_string(),
    }
}

/// `build_diff_seat_prompt`: what one seat is asked about a task's diff. Each seat sees the
/// diff; the reviewer also sees the task, and the security seat the run's own account, the
/// test hunks and the base.
fn build_diff_seat_prompt(
    seat: &Seat,
    task_text: &PyStr,
    base_sha: &PyStr,
    projection: &PyStr,
    diff: &str,
) -> PyStr {
    let or = |text: &str, empty: &str| match py::strip_str(text) {
        "" => empty.to_string(),
        text => text.to_string(),
    };
    let mut prompt = text!(
        "You are one judge on a three-seat panel reviewing a task diff.\n\n## Role\n\n",
        seat.role.brief(),
        "\n"
    );
    if seat.evidence == Evidence::DiffAndTask {
        prompt.push(&text!("## Task\n\n", task_text.clone(), "\n"));
    }
    prompt.push(&text!(
        "## Diff\n\n```diff\n",
        or(diff, "(empty diff)"),
        "\n```\n"
    ));
    if seat.evidence == Evidence::DiffAndTests {
        let projection = match py::strip(projection) {
            empty if empty.is_empty() => PyStr::from("(no projection)"),
            projection => projection,
        };
        prompt.push(&text!("## Projection\n\n", projection, "\n"));
        prompt.push(&text!(
            "## Tests in diff\n\n```diff\n",
            test_paths_from_diff(diff),
            "\n```\n"
        ));
        prompt.push(&text!(
            "## Base\n\nRepository base at `",
            base_sha.clone(),
            "`.\n"
        ));
    }
    prompt.push_str(
        "## Instructions\n\nReview against the checklist for your role. Answer in exactly one JSON \
         object and nothing else:\n\n\
         ```json\n{\"verdict\": \"pass\" | \"block\", \"findings\": [{\"item\": <checklist id>, \
         \"severity\": \"critical\" | \"major\" | \"minor\", \"note\": <string>}]}\n```\n\n\
         Checklist items:\n\n",
    );
    for (item, advisory, definition) in TASK_DIFF_CHECKLIST {
        if seat.role == Role::Security && !SECURITY_CHECKLIST_ITEMS.contains(&item) {
            continue;
        }
        let ceiling = if advisory { "advisory" } else { "critical" };
        prompt.push_str(&format!("- {item} ({ceiling}): {definition}\n"));
    }
    prompt
}

/// `findings_to_hunk_comments`: every finding worse than minor as a Hunk comment on the
/// first file the branch changed.
fn findings_to_hunk_comments(outcomes: &[JudgeOutcome], default_file: &str) -> Vec<Value> {
    let mut comments = Vec::new();
    for outcome in outcomes {
        for finding in &outcome.findings {
            let severity = SEVERITIES[finding.severity];
            if severity == "minor" {
                continue;
            }
            let summary = text!(
                outcome.provider.clone(),
                ": ",
                finding.item.clone(),
                " (",
                severity,
                ") \u{2014} ",
                finding.note.clone()
            );
            comments.push(Value::Dict(body(vec![
                ("filePath", Value::text(default_file)),
                ("hunkNumber", Value::Int(1)),
                ("summary", Value::Str(summary)),
                ("author", Value::text("puppets")),
            ])));
        }
    }
    comments
}

/// `json.dumps({"comments": comments})`: what `hunk session comment apply --stdin` reads.
fn hunk_payload(comments: Vec<Value>) -> String {
    json::dumps_default(&Value::Dict(body(vec![(
        "comments",
        Value::List(comments),
    )])))
}

/// One offer of the findings to a Hunk session on `repo`. False when there is no `hunk` to
/// run or it exits nonzero, which it does until the session is up.
fn push_hunk_comments(repo: &OsStr, payload: &[u8]) -> bool {
    let argv = ["hunk", "session", "comment", "apply", "--repo"]
        .map(OsString::from)
        .into_iter()
        .chain([repo.to_os_string(), OsString::from("--stdin")])
        .collect::<Vec<_>>();
    capture(&argv, None, Some(payload)).is_ok_and(|run| run.ok())
}

/// Offer the findings until a Hunk session takes them, a second apart, at most
/// `HUNK_TRIES` times.
fn push_until_taken(repo: &OsStr, payload: &[u8]) -> bool {
    for _ in 0..HUNK_TRIES {
        if push_hunk_comments(repo, payload) {
            return true;
        }
        std::thread::sleep(Duration::from_secs(1));
    }
    false
}

/// The hidden `push-hunk-comments` command: the payload on stdin, offered to the Hunk
/// session on `repo`. Exit 0 once a session takes it and 1 when none did.
pub fn push_comments(repo: &OsStr) -> Result<i32> {
    let mut payload = Vec::new();
    std::io::stdin().lock().read_to_end(&mut payload)?;
    Ok(if push_until_taken(repo, &payload) {
        0
    } else {
        1
    })
}

/// `fork_hunk_comment_pusher`: leave a process behind that offers the findings to Hunk
/// while this one goes on to print the command or become Hunk. The Python forks; this
/// starts its own binary again as `push-hunk-comments`, with this process's stdout and
/// stderr, and does not wait for it.
fn start_comment_pusher(repo: &Path, payload: Option<&str>, err: &mut dyn Write) -> Result<()> {
    offer_comments(std::env::current_exe(), repo, payload, err)
}

/// Start the pusher from `exe`. The `review` event is in the log by now, so a binary that
/// has been moved or rebuilt away is said on stderr and the review goes on to print the
/// command or open Hunk; failing here would make a retry sit the panel a second time.
fn offer_comments(
    exe: std::io::Result<PathBuf>,
    repo: &Path,
    payload: Option<&str>,
    err: &mut dyn Write,
) -> Result<()> {
    let Some(payload) = payload else {
        return Ok(());
    };
    let started = match exe {
        Ok(exe) => pusher_from(&exe, repo, payload),
        Err(failed) => Err(anyhow!("OSError: {failed}")),
    };
    if let Err(failed) = started {
        writeln!(
            err,
            "hydra-puppets: the findings were not offered to Hunk: {failed}; \
             the review is recorded"
        )?;
    }
    Ok(())
}

/// Start `exe push-hunk-comments <repo>` and hand it the payload, without waiting for it.
fn pusher_from(exe: &Path, repo: &Path, payload: &str) -> Result<()> {
    let mut child = Command::new(exe)
        .arg("push-hunk-comments")
        .arg(repo)
        .stdin(Stdio::piped())
        .spawn()
        .map_err(|err| spawn_error(exe.as_os_str(), &err))?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(payload.as_bytes())
            .map_err(|err| anyhow!("OSError: {err}"))?;
    }
    Ok(())
}

/// `review`: launch the panel on a task's diff, record its verdict, hand its findings to
/// Hunk as comments, and open the human's review of the diff in Hunk. The diff is read from
/// the repository at the run's branch, never from a checkout the run can write.
///
/// With `no_open` the Hunk command is printed and nothing is opened. Otherwise, inside
/// Herdr the diff opens in a new pane, and outside it this process becomes Hunk.
pub fn review(
    home: &Home,
    goal_id: &str,
    task_id: &PyStr,
    no_open: bool,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> Result<i32> {
    let state = folded(home, goal_id)?;
    let reference = Value::Str(task_id.clone());
    let task = match state.task(&reference)? {
        Some(task) if py::truthy(&task.run_id) => task,
        _ => {
            eprint(err, &text!("task ", task_id.clone(), " has no run"))?;
            return Ok(1);
        }
    };
    let task_run = run_id_of(&task.run_id)?;
    let shown = hydra_show(&task_run)?;
    if !shown.ok() {
        return Err(show_failed(&shown)?);
    }
    let mut run = match loads_or_raise(&shown.stdout()?, "hydra show")? {
        Value::Dict(run) => run,
        other => bail!(
            "AttributeError: '{}' object has no attribute 'setdefault'",
            py::type_name(&other)
        ),
    };
    if run.get("run_id").is_none() {
        run.set("run_id", task.run_id.clone());
    }
    if let Some(goal) = state.goal.as_ref().filter(|goal| !goal.is_empty())
        && !py::truthy(&field(&run, "repo"))
    {
        run.set("repo", field(goal, "repo"));
    }
    let branch = field(&run, "branch");
    if !py::truthy(&branch) {
        let run_id = field(&run, "run_id");
        let run_id = if py::truthy(&run_id) {
            py::str(&run_id)
        } else {
            PyStr::new()
        };
        eprint(
            err,
            &text!(
                "run ",
                run_id,
                " has no branch; review opens Hunk on the reaped branch from the \
                 repository, not inside a run-writable checkout. If the diff exists only in an \
                 unreaped worktree, reap first."
            ),
        )?;
        return Ok(1);
    }
    let repo = match field(&run, "repo") {
        Value::Str(repo) if !repo.is_empty() => pathlib(&fs_path(&repo)?),
        other if py::truthy(&other) => bail!(
            "TypeError: expected str, bytes or os.PathLike object, not {}",
            py::type_name(&other)
        ),
        _ => std::env::current_dir().map_err(|e| anyhow!("FileNotFoundError: {e}"))?,
    };
    let base_sha = match &state.goal {
        Some(goal) if !goal.is_empty() => goal.get("base_sha").cloned(),
        _ => None,
    }
    .unwrap_or(Value::text(""));
    let result_text = field(&run, "result_text");
    let projection = if py::truthy(&result_text) {
        result_text
    } else {
        Value::text(&json::dumps(&Value::Dict(run.clone()), Some(2)))
    };
    let diff = git_diff_branch(&repo, &base_sha, &branch)?;
    let Sitting {
        seats: resolved,
        substitutions,
    } = match resolve_panel_seats(home, &state)? {
        Ok(sitting) => sitting,
        Err(refusal) => {
            writeln!(err, "{refusal}")?;
            return Ok(1);
        }
    };
    let projection = answer_text(&projection)?;
    let (task_text, base_text) = (py::str(&task.text), py::str(&base_sha));
    let seat_prompts = resolved
        .iter()
        .map(|seat| {
            let prompt =
                build_diff_seat_prompt(seat.seat, &task_text, &base_text, projection, &diff);
            (seat, prompt)
        })
        .collect();
    let runs = run_judge_panel(home, seat_prompts)?;
    let outcomes = resolved
        .iter()
        .zip(&runs.texts)
        .map(|(seat, answer)| parse_judge_answer(py::str(&seat.provider), answer_text(answer)?))
        .collect::<Result<Vec<_>>>()?;
    print_judges(out, &resolved, &runs)?;
    let (verdict, reasons) = diff_panel_verdict(&outcomes);
    for reason in &reasons {
        eprint(err, reason)?;
    }
    let [ids, policies] = runs.event_fields();
    append_event(
        home,
        goal_id,
        body(vec![
            ("event", Value::text("review")),
            ("task", reference),
            ids,
            policies,
            ("verdict", Value::text(verdict)),
            (
                "reasons",
                Value::List(reasons.into_iter().map(Value::Str).collect()),
            ),
            ("degraded", Value::Bool(!substitutions.is_empty())),
            ("substitutions", Value::List(substitutions)),
        ]),
    )?;
    let default_file = first_changed_file(&repo, &base_sha, &branch)?.unwrap_or_else(|| ".".into());
    let comments = findings_to_hunk_comments(&outcomes, &default_file);
    let payload = (!comments.is_empty()).then(|| hunk_payload(comments));
    let branch = string(&branch, "branch")?;
    let hunk = text!("hunk show ", branch.clone());
    if no_open {
        start_comment_pusher(&repo, payload.as_deref(), err)?;
        print(out, &hunk)?;
        return Ok(0);
    }
    if let Some(pane_id) = herdr_pane() {
        let split = capture(
            &os_argv(&[
                "herdr".into(),
                "pane".into(),
                "split".into(),
                pane_id,
                "--direction".into(),
                "right".into(),
                "--cwd".into(),
                arg(repo.as_os_str()),
            ])?,
            None,
            None,
        )?;
        let (stdout, _) = (split.stdout()?, split.stderr()?);
        if !split.ok() {
            bail!("CalledProcessError: herdr pane split exited nonzero");
        }
        if let Err(err) = json::loads_str(&stdout) {
            bail!("{}: herdr pane split", err.class());
        }
        let Some(new_pane) = split_pane_id(&stdout)? else {
            bail!("KeyError: herdr pane split named no pane");
        };
        let opened = capture(
            &os_argv(&[
                "herdr".into(),
                "pane".into(),
                "run".into(),
                new_pane.clone(),
                hunk,
            ])?,
            None,
            None,
        )?;
        if !opened.ok() {
            bail!("CalledProcessError: herdr pane run exited nonzero");
        }
        // With nothing to push the Python still waits out its thirty tries; there is
        // nothing for a session to take, so this prints at once.
        if let Some(payload) = &payload {
            push_until_taken(repo.as_os_str(), payload.as_bytes());
        }
        print(out, &text!(task_run, " in ", new_pane, " (", verdict, ")"))?;
        return Ok(0);
    }
    start_comment_pusher(&repo, payload.as_deref(), err)?;
    out.flush()?;
    let failed = Command::new("hunk")
        .arg("show")
        .arg(fs_path(&branch)?)
        .current_dir(&repo)
        .exec();
    Err(spawn_error(OsStr::new("hunk"), &failed))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(text: &str) -> PyStr {
        PyStr::from(text)
    }

    /// Build 196, item 0. The `review` event is in the log by the time the pusher starts,
    /// so a binary that has gone from under the process must not fail the command.
    #[test]
    fn panel_roles_follow_run_id_order() {
        let runs = PanelRuns {
            run_ids: vec!["a".into(), "b".into(), "c".into()],
            policies: vec![],
            texts: vec![],
        };
        let tie = PanelRuns {
            run_ids: vec!["d".into(), "e".into()],
            policies: vec![],
            texts: vec![],
        };
        let event = body(panel_roles(&runs, &tie));
        assert_eq!(
            py::str(&field(&event, "judge_roles")).lossy(),
            "['reviewer', 'adversary', 'security']"
        );
        assert_eq!(
            py::str(&field(&event, "tie_break_roles")).lossy(),
            "['reviewer', 'reviewer']"
        );
        let empty = PanelRuns {
            run_ids: vec![],
            policies: vec![],
            texts: vec![],
        };
        assert_eq!(
            py::str(&field(&body(panel_roles(&runs, &empty)), "tie_break_roles")).lossy(),
            "[]"
        );
    }

    #[test]
    fn a_pusher_that_cannot_start_is_reported_and_does_not_fail_the_review() {
        let gone = Path::new("/nonexistent/hydra-puppets-gone");
        let mut err = Vec::new();
        offer_comments(Ok(gone.to_path_buf()), Path::new("."), Some("{}"), &mut err).unwrap();
        let said = String::from_utf8(err).unwrap();
        assert!(
            said.contains("the findings were not offered to Hunk")
                && said.contains("/nonexistent/hydra-puppets-gone"),
            "{said}"
        );

        let mut err = Vec::new();
        let no_exe = Err(std::io::Error::other("no path"));
        offer_comments(no_exe, Path::new("."), Some("{}"), &mut err).unwrap();
        assert!(String::from_utf8(err).unwrap().contains("no path"));

        // Nothing to push starts nothing and says nothing.
        let mut err = Vec::new();
        offer_comments(Ok(gone.to_path_buf()), Path::new("."), None, &mut err).unwrap();
        assert!(err.is_empty());
    }

    fn blind_fixture() -> (Vec<Value>, Vec<PyStr>) {
        let providers = ["claude", "codex", "cursor-agent"]
            .map(Value::text)
            .to_vec();
        let blocks = (0..3).map(|n| proposal_block(n, s("task body"))).collect();
        (providers, blocks)
    }

    #[test]
    fn blind_prompts_have_no_provider_names() {
        let (providers, blocks) = blind_fixture();
        let names = join(&(0..3).map(proposal_label).collect::<Vec<_>>(), ", ");
        for (n, seat) in PANEL.iter().enumerate() {
            let prompt = build_choose_seat_prompt(
                seat,
                &s("goal"),
                &ordered_blocks(&blocks, &proposal_order(3, n)),
                &names,
            )
            .lossy();
            for provider in &providers {
                assert!(!prompt.contains(&py::str(provider).lossy()), "{prompt}");
            }
            assert!(prompt.contains("<label>|composed"));
        }
    }

    #[test]
    fn blind_choose_prompts_do_not_expose_stance_names() {
        let (_, blocks) = blind_fixture();
        for (n, seat) in PANEL.iter().enumerate() {
            let prompt = build_choose_seat_prompt(
                seat,
                &s("goal"),
                &ordered_blocks(&blocks, &proposal_order(3, n)),
                &s("Proposal A, Proposal B, Proposal C"),
            )
            .lossy();
            for stance in ["minimal", "risk", "contrarian", "builder"] {
                assert!(!prompt.contains(stance), "{prompt}");
            }
        }
    }

    #[test]
    fn blind_seats_read_three_rotations_with_stable_labels() {
        let (_, blocks) = blind_fixture();
        for (n, expected) in [[0, 1, 2], [1, 2, 0], [2, 0, 1]].iter().enumerate() {
            let order = proposal_order(3, n);
            assert_eq!(&order, expected);
            let prompt = build_choose_seat_prompt(
                &PANEL[n],
                &s("goal"),
                &ordered_blocks(&blocks, &order),
                &s("Proposal A, Proposal B, Proposal C"),
            )
            .lossy();
            let positions = expected.map(|i| {
                prompt
                    .find(&format!("### {}", proposal_label(i).lossy()))
                    .unwrap()
            });
            assert!(positions[0] < positions[1] && positions[1] < positions[2]);
        }
    }

    #[test]
    fn blind_label_answers_map_to_providers() {
        let (providers, _) = blind_fixture();
        for (n, provider) in providers.iter().enumerate() {
            let answer = s(&format!("{{\"chosen\":\"{}\"}}", proposal_label(n).lossy()));
            assert_eq!(
                parse_blind_answer(&answer, &providers)
                    .unwrap()
                    .chosen
                    .unwrap(),
                py::str(provider)
            );
        }
        assert!(
            parse_blind_answer(&s(r#"{"chosen":"codex"}"#), &providers)
                .unwrap()
                .chosen
                .is_err()
        );
    }

    struct Layout(PathBuf);
    impl Drop for Layout {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn recorded_split(name: &str, tie: [&str; 2]) -> Dict {
        let layout =
            Layout(std::env::temp_dir().join(format!("hydra-panel-{name}-{}", std::process::id())));
        let home = Home::at(&layout.0);
        let answers = plan_answers([
            r#"{"chosen":"codex"}"#,
            r#"{"chosen":"claude"}"#,
            r#"{"chosen":"codex"}"#,
        ]);
        let tie_runs = PanelRuns {
            run_ids: vec!["tie-1".into(), "tie-2".into()],
            policies: vec!["judge", "judge"],
            texts: tie.iter().map(|answer| Value::text(answer)).collect(),
        };
        let tie = tie
            .iter()
            .map(|answer| parse_plan_answer(&s(answer)).unwrap())
            .collect::<Vec<_>>();
        let mut fields = vec![
            ("event", Value::text("panel_verdict")),
            ("path", Value::Null),
        ];
        fields.extend(plan_verdict_fields(&answers, &tie));
        fields.extend(tie_break_fields(&tie_runs));
        append_event(&home, "goal", body(fields)).unwrap();
        let events = crate::log::read_log(&home, "goal").unwrap();
        match events.into_iter().last().unwrap() {
            Value::Dict(event) => event,
            _ => panic!("verdict event must be an object"),
        }
    }

    #[test]
    fn blind_split_agreed_tie_records_a_verdict() {
        let event = recorded_split(
            "agree",
            [r#"{"chosen":"claude"}"#, r#"{"chosen":"claude"}"#],
        );
        assert!(py::eq_str(&field(&event, "chosen"), "claude"));
        assert!(py::eq_str(&field(&event, "verdict_source"), "tie_break"));
    }

    #[test]
    fn blind_split_disagreed_tie_records_no_verdict() {
        let event = recorded_split(
            "disagree",
            [r#"{"chosen":"claude"}"#, r#"{"chosen":"codex"}"#],
        );
        assert!(matches!(field(&event, "chosen"), Value::Null));
        assert!(matches!(field(&event, "verdict_source"), Value::Null));
        assert_eq!(
            json::dumps_default(&field(&event, "tie_break_run_ids")),
            r#"["tie-1", "tie-2"]"#
        );
        let Value::List(recorded_answers) = field(&event, "tie_break_answers") else {
            panic!("missing tie-break answers")
        };
        assert_eq!(recorded_answers.len(), 2);
        assert!(py::eq_str(&recorded_answers[0], r#"{"chosen":"claude"}"#));
        assert!(py::eq_str(&recorded_answers[1], r#"{"chosen":"codex"}"#));
        let answers = plan_answers([
            r#"{"chosen":"codex"}"#,
            r#"{"chosen":"claude"}"#,
            r#"{"chosen":"codex"}"#,
        ]);
        let tie = plan_answers([
            r#"{"chosen":"claude"}"#,
            r#"{"chosen":"codex"}"#,
            r#"{"chosen":"codex"}"#,
        ]);
        assert!(choose_panel_verdict(&answers, &tie[..2]).is_err());
    }

    #[test]
    fn the_panel_seats_are_the_providers_the_fold_waits_on() {
        let providers: Vec<&str> = PANEL.iter().map(|seat| seat.provider).collect();
        assert_eq!(providers, fold::PANEL_PROVIDERS);
    }

    #[test]
    fn one_fence_is_stripped_and_anything_else_is_left() {
        let strip = |text: &str| strip_one_fence(&s(text)).lossy();
        assert_eq!(strip(" {\"a\": 1}\n"), "{\"a\": 1}");
        assert_eq!(strip("```json\n{}\n```"), "{}");
        assert_eq!(strip("```JSON \n {} \n```  "), "{}");
        assert_eq!(strip("```\n{}\n```"), "{}");
        assert_eq!(strip("```rust\n{}\n```"), "```rust\n{}\n```");
        assert_eq!(strip("```json {}```"), "```json {}```");
        assert_eq!(strip("```json\n{}"), "```json\n{}");
        assert_eq!(strip("```\na\n```\nb\n```"), "```\na\n```\nb\n```");
    }

    #[test]
    fn a_finding_on_an_advisory_item_is_never_worse_than_minor() {
        let answer = r#"{"verdict":"pass","findings":[
            {"item":"dead_code","severity":"critical","note":"a"},
            {"item":"scope_match","severity":"critical","note":"b"},
            {"item":"scope_match","severity":"major","note":"c"}]}"#;
        let outcome = parse_judge_answer(s("claude"), &s(answer)).unwrap();
        assert!(outcome.parse_error.is_none());
        let severities: Vec<&str> = outcome
            .findings
            .iter()
            .map(|finding| SEVERITIES[finding.severity])
            .collect();
        assert_eq!(severities, ["minor", "critical", "major"]);
        let (verdict, reasons) = diff_panel_verdict(&[outcome]);
        assert_eq!(verdict, "block");
        assert_eq!(reasons, [s("claude: critical scope_match: b")]);
    }

    #[test]
    fn an_answer_that_does_not_parse_blocks_and_says_why() {
        let why = |answer: &str| {
            parse_judge_answer(s("codex"), &s(answer))
                .unwrap()
                .parse_error
                .map(|why| why.lossy())
        };
        assert_eq!(
            why("fine by me").as_deref(),
            Some("Expecting value: line 1 column 1 (char 0)")
        );
        assert_eq!(why("[]").as_deref(), Some("answer is not a JSON object"));
        assert_eq!(
            why(r#"{"verdict":"ok"}"#).as_deref(),
            Some("verdict 'ok' is not pass or block")
        );
        assert_eq!(
            why(r#"{"verdict":"pass","findings":[{"item":"nope","severity":"minor","note":""}]}"#)
                .as_deref(),
            Some("unknown checklist item 'nope'")
        );
        assert_eq!(why(r#"{"verdict":"pass","findings":[]}"#), None);
        let (verdict, reasons) =
            diff_panel_verdict(&[parse_judge_answer(s("codex"), &s("[]")).unwrap()]);
        assert_eq!(verdict, "block");
        assert_eq!(
            reasons,
            [s("codex: parse error: answer is not a JSON object")]
        );
    }

    fn plan_answers(answers: [&str; 3]) -> Vec<PlanAnswer> {
        answers
            .iter()
            .map(|answer| parse_plan_answer(&s(answer)).unwrap())
            .collect()
    }

    #[test]
    fn the_plan_panel_needs_every_seat_to_name_the_same_plan() {
        let codex = r#"{"chosen":"codex"} and why"#;
        let agreed = plan_panel_verdict(&plan_answers([codex, codex, codex])).unwrap();
        assert_eq!(agreed, (s("codex"), None));
        let split = plan_panel_verdict(&plan_answers([codex, r#"{"chosen":"claude"}"#, codex]));
        assert_eq!(
            split.unwrap_err(),
            [s(
                "disagreement: claude=codex, codex=claude, cursor-agent=codex"
            )]
        );
        let unread = plan_panel_verdict(&plan_answers([codex, "no", r#"{"chosen":""}"#]));
        assert_eq!(
            unread.unwrap_err(),
            [
                s("codex: parse error: Expecting value: line 1 column 1 (char 0)"),
                s("cursor-agent: parse error: chosen must be a non-empty string"),
            ]
        );
    }

    #[test]
    fn a_composed_verdict_carries_the_first_task_array() {
        let bare = r#"{"chosen":"composed"}"#;
        let with_block = "{\"chosen\":\"composed\"}\n```json\n[{\"id\":\"a\"}]\n```\n";
        let (chosen, plan) = plan_panel_verdict(&plan_answers([bare, with_block, bare])).unwrap();
        assert_eq!(chosen, "composed");
        assert_eq!(plan.as_deref(), Some("[\n  {\n    \"id\": \"a\"\n  }\n]"));
        let none = plan_panel_verdict(&plan_answers([bare, bare, bare]));
        assert_eq!(
            none.unwrap_err(),
            [s("composed chosen but no task block found")]
        );
    }

    #[test]
    fn only_hunks_under_a_test_path_are_kept() {
        let diff = "diff --git a/src/a.rs b/src/a.rs\n--- a/src/a.rs\n+++ b/src/a.rs\n@@ -1 +1 @@\n-x\n+y\n\
                    diff --git a/tests/t.rs b/tests/t.rs\n--- a/tests/t.rs\n+++ b/tests/t.rs\n@@ -1 +1 @@\n-a\n+b\n";
        assert_eq!(
            test_paths_from_diff(diff),
            "diff --git a/tests/t.rs b/tests/t.rs\n\n--- a/tests/t.rs\n\n+++ b/tests/t.rs\n@@ -1 +1 @@\n-a\n+b"
        );
        assert_eq!(test_paths_from_diff(" \n"), "(no test paths in diff)");
        assert_eq!(
            test_paths_from_diff("diff --git a/src/a.rs b/src/a.rs\n+x\n"),
            "(no test paths in diff)"
        );
    }

    #[test]
    fn lines_split_where_python_splits_them() {
        assert_eq!(
            splitlines("a\nb\r\nc\rd\u{2028}e\n"),
            ["a", "b", "c", "d", "e"]
        );
        assert_eq!(splitlines("\n\na"), ["", "", "a"]);
        assert!(splitlines("").is_empty());
    }

    #[test]
    fn a_finding_worse_than_minor_becomes_a_hunk_comment() {
        let answer = r#"{"verdict":"pass","findings":[
            {"item":"scope_match","severity":"major","note":"n"},
            {"item":"scope_match","severity":"minor","note":"m"}]}"#;
        let outcome = parse_judge_answer(s("claude"), &s(answer)).unwrap();
        let payload = hunk_payload(findings_to_hunk_comments(&[outcome], "src/a.rs"));
        assert_eq!(
            payload,
            "{\"comments\": [{\"filePath\": \"src/a.rs\", \"hunkNumber\": 1, \
             \"summary\": \"claude: scope_match (major) \\u2014 n\", \"author\": \"puppets\"}]}"
        );
    }
}
