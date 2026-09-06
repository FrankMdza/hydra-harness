//! What rung one may learn from, and what it may not.
//!
//! `docs/ouroboros-spec.md` section 3.2 names five failure classes and 3.10 names
//! the outcomes that are never one of them. The contract states the same split
//! from the other side: section 8's right-hand column per outcome, and 8.6 for the
//! whole harness-run kind.
//!
//! **Every run is read through [`crate::normalize`] and nothing here matches on
//! `raw`.** That is contract 11's whole point: an adapter fixed next month reads
//! last month's records correctly, because nothing was migrated, and a curator
//! that greps the stored frame gets none of that.
//!
//! **Skipped is not thrown away.** Every run this leaves out is on disk in full,
//! and [`Selection::skipped`] carries the reason so the brief can say what the
//! corpus holds that the curator did not read. A reader who collapses "rung one
//! never learns from this" into "throw it away" has the record backwards.

use std::path::PathBuf;

use anyhow::Result;

use crate::live::{self, State};
use crate::normalize::{self, Decider, Decision, Event, Finding, ResultStatus, Run};
use crate::outcome::{Derived, Outcome, derive};
use crate::paths::Layout;
use crate::policy::PolicySet;

/// The corpus, split by whether rung one may learn from it.
pub struct Selection {
    pub learn: Vec<Learnable>,
    pub skipped: Vec<Skipped>,
}

/// One run rung one may learn from.
pub struct Learnable {
    pub run_id: String,
    pub run_file: PathBuf,
    /// The run that spawned this one. Always null in this build, and the field
    /// the self-exclusion of section 3.5 keys on.
    pub parent_run_id: Option<String>,
    pub provider: String,
    pub policy: Option<String>,
    /// **`None` is the abandoned class rather than a missing value.** A run with
    /// no `run_end` has no outcome to derive (contract 8.6), and one whose pid is
    /// gone is the first of section 3.2's five classes. The live ones never reach
    /// here: they are skipped before the file is read.
    pub outcome: Option<Derived>,
    pub signals: Signals,
}

impl Learnable {
    pub fn outcome_name(&self) -> &'static str {
        match &self.outcome {
            Some(derived) => derived.outcome.as_str(),
            None => "abandoned",
        }
    }

    pub fn source_name(&self) -> &'static str {
        match &self.outcome {
            Some(derived) => derived.source.as_str(),
            None => "none",
        }
    }
}

/// What one learnable run holds that a curator would rank on.
#[derive(Default)]
pub struct Signals {
    pub tool_calls: usize,
    pub failed_tools: usize,
    pub unclassified_tools: usize,
    /// Calls the run's own policy refused. **A policy bug rather than a skill
    /// bug** (ticket 27), and the brief says so out loud, because contract 8's
    /// `gated` fires only when the gate refused everything: a run that had four
    /// writes denied and one read allowed comes out `completed` and would
    /// otherwise reach the curator as a skill failure.
    pub denied_by_hydra: usize,
    /// A human corrected a tool call. One producer, an interactive harvest, and
    /// this build launches no interactive runs, so it is zero everywhere today.
    pub corrections: usize,
    /// Findings where the record and the policy disagree about what was allowed:
    /// a leaked gate, subagents under a policy that denies them, a call the gate
    /// never answered, or the derivation and the gate's own answer disagreeing.
    /// Learning from such a run without reading these would teach the curator
    /// that a refused call succeeded.
    pub disagreements: Vec<String>,
    /// Findings that say the derivation could not do its job. They bound what the
    /// run can be read to mean and are not a failure of the run.
    pub limits: Vec<String>,
    pub abandoned: bool,
}

impl Signals {
    /// Section 3.2's fifth class: a run that reports success with a real failure
    /// inside it.
    ///
    /// **It earns its place because it makes the other four unreliable.** All
    /// three providers close a run reporting success with a failed tool or a
    /// denial in it, which is why the contract derives every outcome from the
    /// stream. This is the same check one level up: the derivation said the run
    /// ended cleanly and the tools inside it say otherwise.
    pub fn clean_looking_failure(&self, outcome: Option<&Derived>) -> bool {
        let clean = matches!(
            outcome.map(|derived| derived.outcome),
            Some(Outcome::Completed)
        );
        clean && (self.failed_tools > 0 || !self.disagreements.is_empty())
    }

    /// How far up the list this run goes. Signal count, not a score with a
    /// meaning: no threshold is picked in v1 and none is implied here.
    fn weight(&self) -> usize {
        self.failed_tools * 2
            + self.unclassified_tools
            + self.disagreements.len() * 2
            + self.corrections * 3
            + usize::from(self.abandoned) * 2
    }
}

/// A run the curator did not read, and why.
pub struct Skipped {
    pub run_id: String,
    pub why: Why,
}

pub enum Why {
    /// Contract 8.6: a pass spawned four children and tried nothing itself. The
    /// rule keys on `provider: null` rather than on an outcome, so an interrupted
    /// `hydra evolve` never reads as an abandoned run worth learning from.
    HarnessRun,
    /// No `run_end` and the recorded pid still matches. Nothing is wrong with it;
    /// it has not finished.
    StillRunning,
    /// One of contract 8's five "never" outcomes. A skill does not need evolving
    /// because a token expired.
    NeverLearns(Outcome),
    /// A recorded run with no policy, written before the gate reached a provider.
    /// Contract 7.7 gives it no `permission_decision`, so a refused call inside it
    /// is invisible and the run derives `completed` whatever the gate did.
    PreGate,
    /// A run a curator pass spawned. Rung one excludes its own children or it
    /// learns from itself (section 3.5).
    CuratorsChild,
    /// The file would not read back. Said out loud rather than counted as an
    /// absence, because a corpus that silently shrinks is the failure mode
    /// section 3.4 is about.
    Unreadable(String),
}

impl Why {
    pub fn say(&self) -> String {
        match self {
            Why::HarnessRun => "a harness run, contract 8.6: no provider, nothing tried".into(),
            Why::StillRunning => "still running".into(),
            Why::NeverLearns(outcome) => {
                format!("{}, which rung one never learns from", outcome.as_str())
            }
            Why::PreGate => "recorded before the gate, so a refusal inside it is invisible".into(),
            Why::CuratorsChild => "a curator pass spawned it".into(),
            Why::Unreadable(why) => format!("it would not read back: {why}"),
        }
    }
}

/// Section 3.2's five classes, counted over the learnable set.
///
/// **This is the ranking.** Section 3.1 ranks by frequency and picks no
/// threshold, because Hydra has run few enough times that a number invented now
/// would be a guess with a decimal point on it.
#[derive(Default)]
pub struct Classes {
    pub abandoned: usize,
    pub corrections: usize,
    pub tool_errors: usize,
    pub retries: usize,
    pub clean_looking: usize,
}

impl Selection {
    /// The five classes, counted.
    pub fn classes(&self) -> Classes {
        let mut classes = Classes::default();
        for run in &self.learn {
            classes.abandoned += usize::from(run.signals.abandoned);
            classes.corrections += run.signals.corrections;
            classes.tool_errors += usize::from(run.signals.failed_tools > 0);
            classes.clean_looking +=
                usize::from(run.signals.clean_looking_failure(run.outcome.as_ref()));
            // Contract 10's tree. Delegation is not built, so every
            // `parent_run_id` on this machine is null and this is zero.
            classes.retries += usize::from(run.parent_run_id.is_some());
        }
        classes
    }

    /// How many runs were skipped for each reason, in the order a brief reads
    /// them out.
    pub fn skipped_counts(&self) -> Vec<(String, usize)> {
        let mut counts: Vec<(String, usize)> = Vec::new();
        for skipped in &self.skipped {
            let why = skipped.why.say();
            match counts.iter_mut().find(|(seen, _)| *seen == why) {
                Some((_, count)) => *count += 1,
                None => counts.push((why, 1)),
            }
        }
        counts
    }
}

/// Walks `~/hydra/runs/` and splits it.
///
/// **The liveness test comes first and costs one `sysctl`.** A run that is still
/// writing is skipped before its file is opened, which keeps the walk off the one
/// file that has a writer on it and saves normalizing a record that will look
/// different in a minute.
///
/// The rest of the cost is contract 7.7's: deriving `permission_decision` runs the
/// run's own gate script once per recorded tool call, one `/usr/bin/python3` each.
/// It scales with calls rather than with records, and if it ever bites, the fix is
/// a batch mode on the gate rather than a second implementation of the rules in
/// Rust.
pub fn select(layout: &Layout, policies: &PolicySet) -> Result<Selection> {
    let mut learn = Vec::new();
    let mut skipped = Vec::new();
    // Every run that was itself a curator pass, so the pass below can drop the
    // children of one. Delegation is not built, so nothing has a parent yet and
    // this filter takes nothing out today.
    let mut curator_passes: Vec<String> = Vec::new();

    for view in live::scan(layout)? {
        if view.state == State::Live {
            skipped.push(Skipped {
                run_id: view.run_id.clone(),
                why: Why::StillRunning,
            });
            continue;
        }

        let text = match std::fs::read_to_string(&view.run_file) {
            Ok(text) => text,
            Err(err) => {
                skipped.push(Skipped {
                    run_id: view.run_id.clone(),
                    why: Why::Unreadable(err.to_string()),
                });
                continue;
            }
        };
        let run = match normalize::read_with(&text, policies, layout) {
            Ok(run) => run,
            Err(err) => {
                skipped.push(Skipped {
                    run_id: view.run_id.clone(),
                    why: Why::Unreadable(format!("{err:#}")),
                });
                continue;
            }
        };

        if run.policy.as_deref() == Some("curate") {
            curator_passes.push(run.run_id.clone());
        }

        match judge(&run, &view.run_file) {
            Ok(item) => learn.push(item),
            Err(why) => skipped.push(Skipped {
                run_id: view.run_id.clone(),
                why,
            }),
        }
    }

    // Section 3.5, a second pass because a child can be read before its parent.
    let mut kept = Vec::with_capacity(learn.len());
    for item in learn {
        let child_of_a_pass = item
            .parent_run_id
            .as_ref()
            .is_some_and(|parent| curator_passes.iter().any(|pass| pass == parent));
        if child_of_a_pass {
            skipped.push(Skipped {
                run_id: item.run_id,
                why: Why::CuratorsChild,
            });
            continue;
        }
        kept.push(item);
    }

    // Most signal first, then oldest first, which is the run id: a UUIDv7 sorts
    // by launch time.
    kept.sort_by(|a, b| {
        b.signals
            .weight()
            .cmp(&a.signals.weight())
            .then_with(|| a.run_id.cmp(&b.run_id))
    });

    Ok(Selection {
        learn: kept,
        skipped,
    })
}

/// The filters one run passes, in the order they can be answered.
///
/// Kind first, because it needs no outcome and settles a whole population.
/// Contract 8's column second. The pre-gate test last, because it only matters
/// for a run that would otherwise be learned from. The fourth, a run a curator
/// pass spawned, needs the whole corpus and is in [`select`].
fn judge(run: &Run, run_file: &std::path::Path) -> Result<Learnable, Why> {
    // 8.6. A pass tried nothing, and a pass killed with Ctrl-C is a dead pid with
    // no `run_end`, which is exactly the abandoned class. Keying on the outcome
    // would file every interrupted `hydra evolve` as a failure signal about
    // nothing.
    let Some(provider) = run.provider.clone() else {
        return Err(Why::HarnessRun);
    };

    // Contract 8's right-hand column, read off the outcome rather than guessed
    // at. `None` is the abandoned class and is learnable.
    let outcome = derive(run);
    if let Some(derived) = &outcome
        && !derived.outcome.rung_one_learns()
    {
        return Err(Why::NeverLearns(derived.outcome));
    }

    // A recorded run with a null policy predates the gate. Contract 7.7 derives
    // no `permission_decision` without a policy, so the record cannot say whether
    // a call was refused, and the outcome comes out `completed` either way. One
    // such record on this machine had a `Write` refused and nothing in it says so.
    //
    // **The test is the launch mode beside the null policy, and dropping either
    // half breaks it.** An interactive run also carries a null policy, legitimately
    // (contract 5.6), and it is the only population a human correction can come
    // from, so a rule that keyed on the null alone would throw away half the
    // corpus and the whole of section 3.2's second class with it.
    let recorded = run
        .run_start()
        .map(|start| start.body["launch_mode"].as_str() == Some("recorded"))
        .unwrap_or(false);
    if recorded && run.policy.is_none() {
        return Err(Why::PreGate);
    }

    let signals = signals(run, outcome.is_none());
    Ok(Learnable {
        run_id: run.run_id.clone(),
        run_file: run_file.to_path_buf(),
        parent_run_id: run.parent_run_id.clone(),
        provider,
        policy: run.policy.clone(),
        outcome,
        signals,
    })
}

fn signals(run: &Run, abandoned: bool) -> Signals {
    let mut signals = Signals {
        abandoned,
        ..Signals::default()
    };

    signals.tool_calls = run.tool_calls().count();
    for result in run.tool_results() {
        match result.status {
            ResultStatus::Error => signals.failed_tools += 1,
            ResultStatus::Unclassified => signals.unclassified_tools += 1,
            ResultStatus::Ok => {}
        }
    }
    signals.denied_by_hydra = run
        .permission_decisions()
        .filter(|decision| {
            decision.decision == Decision::Deny && decision.decider == Decider::HydraPolicy
        })
        .count();
    signals.corrections = run
        .events
        .iter()
        .filter(|event| matches!(event, Event::Correction(_)))
        .count();

    for finding in &run.findings {
        match finding {
            Finding::GateLeak { tool, reason, .. } => signals
                .disagreements
                .push(format!("the gate leaked on {tool}: {reason}")),
            Finding::SubagentLeak { spawned } => signals.disagreements.push(format!(
                "{spawned} in-process subagent(s) ran under a policy that grants none"
            )),
            Finding::Disagreement { call_id, .. } => signals.disagreements.push(format!(
                "the derivation and the gate's own answer disagree about call {call_id}"
            )),
            Finding::Ungated { call_id, tool } => signals.disagreements.push(format!(
                "{tool} (call {call_id}) reached the model with no gate answer behind it"
            )),
            Finding::UnknownTool { name } => signals.limits.push(format!(
                "the capability table does not know the tool {name}"
            )),
            Finding::StalePolicy { recorded, .. } => signals.limits.push(format!(
                "policy_sha {} is not today's, so the decisions answer for another policy",
                &recorded[..12.min(recorded.len())]
            )),
            Finding::Limit(said) => signals.limits.push(said.clone()),
        }
    }
    signals
}
