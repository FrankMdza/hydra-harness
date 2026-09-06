//! The nine outcomes, and what `hydra run` returns.
//!
//! Contract section 8 owns the enum, 8.2 owns `outcome_source`, and 17 owns the
//! JSON. Harness spec 12 states the same nine from the launcher's side.
//!
//! **This is the one module that puts a derived value in a serialized shape, and
//! it says so out loud.** Contract 6 and 11 both forbid storing an outcome, and
//! `normalize` expresses that in the type system by deriving `Serialize` on
//! nothing. The projection has to live somewhere, so it lives here, behind a
//! module boundary a reader can see. Everything below serializes to stdout for an
//! orchestrator and nothing below is ever written to a run file.
//!
//! **Nothing reads the exit status to decide an outcome.** All three providers
//! exit 0 and report success on a run the gate refused, so the exit code is
//! carried through to the caller as a fact and never consulted as a signal. The
//! one thing that reads it is [`derive`]'s split between a process that died and
//! a process that never started, and that reads only whether it exists.

use serde::Serialize;

use crate::normalize::{Decider, Decision, Finding, Message, Role, Run, UsageSource};
use crate::record::Refusal;

/// Contract 8's nine. A run ends on exactly one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Completed,
    Failed,
    /// The watcher gave up and the child may still be running.
    ///
    /// **Its producer is a handle rather than a record**, and it is the only
    /// outcome in this enum that [`derive`] never returns. The second half of
    /// contract 8's row is the reason: "the child may still be running" is a
    /// sentence no `run_end` can carry, because a record with a closing line is a
    /// run that stopped. `hydra run` constructs it where the giving up happens,
    /// and the same run's record derives its own answer afterwards.
    TimedOut,
    Crashed,
    Gated,
    AuthFailed,
    /// A linked skill name the provider did not resolve as Hydra's. **No producer
    /// here**: it is decided by a launch check that needs the skill store, which
    /// is ticket 07, and it arrives through [`Refusal`] when it does.
    SkillCollision,
    /// **No producer here** either, for the same reason: the launch check is
    /// ticket 08's, and it arrives through [`Refusal`].
    McpUnavailable,
    RateLimited,
}

impl Outcome {
    pub fn as_str(self) -> &'static str {
        match self {
            Outcome::Completed => "completed",
            Outcome::Failed => "failed",
            Outcome::TimedOut => "timed_out",
            Outcome::Crashed => "crashed",
            Outcome::Gated => "gated",
            Outcome::AuthFailed => "auth_failed",
            Outcome::SkillCollision => "skill_collision",
            Outcome::McpUnavailable => "mcp_unavailable",
            Outcome::RateLimited => "rate_limited",
        }
    }

    /// Whether rung one learns from a run that ended this way, the right-hand
    /// column of contract 8's table.
    ///
    /// It says what rung one learns from and never what Hydra keeps. Every run is
    /// recorded in full whatever this returns.
    pub fn rung_one_learns(self) -> bool {
        matches!(
            self,
            Outcome::Completed | Outcome::Failed | Outcome::TimedOut | Outcome::Crashed
        )
    }
}

/// How far the outcome can be trusted, contract 8.2. Three levels of confidence
/// rather than three shades of one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// Hydra refused the launch itself, so the outcome is a fact it wrote.
    Refusal,
    /// The adapter classified the run from the stream.
    Derived,
    /// The adapter could not. The run holds a tool result it returned
    /// `unclassified` for and nothing else settles it.
    Unknown,
}

impl Source {
    pub fn as_str(self) -> &'static str {
        match self {
            Source::Refusal => "refusal",
            Source::Derived => "derived",
            Source::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Derived {
    pub outcome: Outcome,
    pub source: Source,
}

/// The outcome of one normalized run, or `None` on a run that has not ended.
///
/// **`None` is not an outcome and must not be flattened into one.** A file with
/// no `run_end` is a run whose ending nothing wrote: either the child is still
/// going, or Hydra died and the run is ticket 14's abandoned class. Telling those
/// two apart needs the pid liveness test of contract 5.2, which is `hydra status`
/// rather than a curator's read, and contract 8.6 says so. Returning
/// `Outcome::Crashed` here would put every live run in the corpus as a dead one.
///
/// The order of the checks below is load-bearing and is roughly "how much did
/// Hydra decide, and how little did the run get to try".
pub fn derive(run: &Run) -> Option<Derived> {
    let end = run.run_end()?;

    // Contract 6: `refusal` is the one thing about an outcome a file may state,
    // because there is no stream to derive it from. It is the strongest input the
    // derivation has, so it is read first.
    if let Some(refusal) = end.body["refusal"].as_str() {
        return Some(Derived {
            outcome: Refusal::outcome_of(refusal),
            source: Source::Refusal,
        });
    }

    // Everything from here reads the stream. The two shapes where the provider
    // refused its own service come first, because both mean the run never got to
    // do its work.
    if let Some(outcome) = provider_refused(run) {
        return Some(Derived {
            outcome,
            source: Source::Derived,
        });
    }

    if refused_everything(run) {
        return Some(Derived {
            outcome: Outcome::Gated,
            source: Source::Derived,
        });
    }

    // **`run_end.error` is read on one side of this match and not the other**,
    // and the asymmetry is the whole of it. That field holds what Hydra caught
    // off the stream, which in this build is a tee that broke or a count of lines
    // stored lossily. With a terminator present the run reached its own end, so
    // the field is a fidelity note and reading it as a failure would file a run
    // whose task succeeded as a lesson about a UTF-8 byte, which is contract
    // 8.4's mistake pointed the other way. With no terminator it is the only
    // thing in the file that says which side stopped.
    let outcome = match run.result.as_ref() {
        None => {
            // `claude` writes exactly one `result` frame on every path, so
            // something stopped before the child finished. Three shapes, and
            // `crashed` is only one of them, because contract 8 defines it as
            // "the process died" and two of the three are not that.
            if end.body["exit_code"].is_null() {
                // No process ran at all. Nothing died.
                Outcome::Failed
            } else if end.body["error"].as_str().is_some() {
                // Hydra stopped rather than the child. The tee broke, and the
                // child may have gone on to finish perfectly; the record is
                // short and its own fate is not in the file. Calling that
                // `crashed` would assert a death nothing witnessed.
                Outcome::Failed
            } else {
                // The child ended without its terminator and Hydra has nothing
                // to say about why. That is the one shape where the process is
                // the thing that stopped.
                Outcome::Crashed
            }
        }
        Some(result) => {
            // `is_error` is the provider's own word and the only honest thing on
            // the frame. `subtype` is not read anywhere: it reads `success` on a
            // 429, which is the nastiest shape in the contract.
            if result["is_error"].as_bool() == Some(true) {
                Outcome::Failed
            } else {
                Outcome::Completed
            }
        }
    };

    // 8.2's `unknown`, and the only place it is produced. A `completed` reached
    // by nothing saying otherwise is the one soft outcome in the list, so an
    // unclassified tool result is enough to take the confidence off it. Every
    // other outcome above rests on something a source said out loud, which an
    // unreadable tool result does not weaken.
    let source = if outcome == Outcome::Completed && has_unclassified(run) {
        Source::Unknown
    } else {
        Source::Derived
    };
    Some(Derived { outcome, source })
}

/// The two shapes where the provider refused its own service.
///
/// Both come off `api_error_status` rather than `subtype` or `is_error`, because
/// a rate-limited `claude` run reports `subtype: "success"` with `is_error: true`
/// and `api_error_status: 429`. A recorder keying off `subtype` files an exhausted
/// run as a clean one, and one keying off `is_error` alone files it as an ordinary
/// failure, which rung one would then learn a skill lesson from.
fn provider_refused(run: &Run) -> Option<Outcome> {
    let status = run.result.as_ref()?["api_error_status"].as_u64()?;
    match status {
        // Measured on a live run, 2026-09-05, and `IMPLEMENTATION.md` has the
        // frame: this is the one status of the three that a record on this
        // machine actually carries.
        429 => Some(Outcome::RateLimited),
        // The login lapsed mid-run, contract 8's second reading of
        // `auth_failed`. The pre-flight reading is a [`Refusal`] and is not
        // built.
        //
        // **Neither status here has been seen on a record.** 401 is what an
        // expired token should produce and 403 is a guess beside it, kept because
        // an authorization refusal is not a task failure under either spelling
        // and getting it wrong the other way teaches rung one a skill lesson from
        // a lapsed login. Treat both as a hypothesis until a record shows one.
        401 | 403 => Some(Outcome::AuthFailed),
        // Anything else the provider reports is a real failure of the run and
        // falls through to `is_error`. A 500 is not a refusal.
        _ => None,
    }
}

/// Whether the gate refused everything the run tried.
///
/// **"The gate refused everything" is the design's own phrase**, in contract 8,
/// harness spec 12 and tickets 27 and 14, and this is that sentence as a
/// predicate: the run made at least one tool call, no call got through, and at
/// least one refusal was Hydra's own gate.
///
/// Three things it deliberately does not do.
///
/// It does not make a single refused call a run outcome. Contract 8.1 dropped
/// `denied` partly because it "read as a per-tool-call fact, which is
/// `permission_decision`'s job", and a run that made fifty calls, had one denied
/// and finished the task is a trajectory rung one should learn from. The denial
/// is still in the record, per call, where 7.7 puts it.
///
/// It does not count a gate leak. A `Finding::GateLeak` is the policy denying a
/// call that nothing actually refused, so the call went through and the run did
/// its work. An outcome describes what happened, and filing a leak as `gated`
/// would report the run as blocked by the hole that let it through.
///
/// It requires one refusal in Hydra's own voice. A `decider` of `external` is the
/// user's own `PreToolUse` guards, which contract 7.7 spends a section separating
/// from Hydra's gate, and `gated` is about Hydra's gate. A run blocked entirely by
/// somebody else's hooks comes out `completed`, which is a known weak spot rather
/// than a reading: nothing in the record says what that run was trying to do.
///
/// **The leaks are counted rather than matched to their calls, and the two agree.**
/// A `Finding::GateLeak` carries a `call_id`, `call_id` is the provider's to
/// supply, and a `tool_use` block without one is a shape the normalizer tolerates,
/// so two of them in a run both answer to `""`. Matching on that is the collision
/// the adapter's own decision loop switched to positional slots to avoid. It turns
/// out not to change the answer here: every leak is a call the policy denies that
/// nothing refused, so it is one of the denies below and exactly one finding is
/// pushed per call. So one leak already drops the matched count under `calls` and
/// both forms say "not gated". Counting is kept because it needs no identity and
/// takes the collision out of the reasoning, not because the other was wrong.
fn refused_everything(run: &Run) -> bool {
    let leaks = run
        .findings
        .iter()
        .filter(|finding| matches!(finding, Finding::GateLeak { .. }))
        .count();

    let mut calls = 0usize;
    let mut denied = 0usize;
    let mut by_hydra = false;
    for decision in run.permission_decisions() {
        calls += 1;
        if decision.decision == Decision::Deny {
            denied += 1;
            by_hydra |= decision.decider == Decider::HydraPolicy;
        }
    }
    calls > 0 && denied == calls && leaks == 0 && by_hydra
}

/// Whether any tool result came back in a shape the adapter could not classify.
///
/// Measured on `claude` 2.1.261: a successful `Write` returned with no `is_error`
/// field at all. Contract 7.6 forbids reading that as success, so the run holds a
/// call whose fate is unknown and 8.2 takes the confidence off the outcome.
fn has_unclassified(run: &Run) -> bool {
    run.tool_results()
        .any(|result| result.status == crate::normalize::ResultStatus::Unclassified)
}

/// The last thing the assistant said, contract 17's "final assistant text".
///
/// Taken from the `message` events rather than from `claude`'s `result.result`,
/// because the stream is the one source all three providers have and the frame is
/// one provider's spelling. A run that said nothing returns `None`, which is the
/// true answer rather than an empty string.
///
/// **This is the final turn and not the final block.** Contract 7.3 gives one
/// `message` event per content block, which is what `Message::block` numbers, and
/// an assistant frame that interleaves thinking with prose comes back as
/// `[text, thinking, text]`. Taking the last event would return the second half
/// of what the model said and drop the first, so the blocks of the last frame are
/// joined in block order. The separator is a blank line, which is how two content
/// blocks of one turn read.
pub fn final_assistant_text(run: &Run) -> Option<String> {
    let last = run
        .messages()
        .filter(|message| message.role == Role::Assistant && !message.text.trim().is_empty())
        .map(|message| message.seq)
        .max()?;

    let mut blocks: Vec<&Message> = run
        .messages()
        .filter(|message| {
            message.seq == last
                && message.role == Role::Assistant
                && !message.text.trim().is_empty()
        })
        .collect();
    blocks.sort_by_key(|message| message.block);

    Some(
        blocks
            .iter()
            .map(|message| message.text.as_str())
            .collect::<Vec<_>>()
            .join("\n\n"),
    )
}

/// What `hydra run` prints on stdout (D7).
///
/// Contract 17 makes this a projection of the record plus the diff stat, and it
/// gives the field list and then says "Nothing else". The list is the whole
/// struct, in the contract's order.
///
/// **`provider` and `provider_session_id` are gone**, and the deletion is the
/// point rather than an oversight. The first slice returned them; the contract's
/// list does not, and harness spec 9 explains the discipline: delegation exists to
/// keep a child's trajectory out of the orchestrator's context, so every field
/// that is not on the list has to argue for itself. Both are in the record, and
/// `run_file` is on the list.
#[derive(Serialize)]
pub struct RunResult {
    pub run_id: String,
    /// Null on a run with no `run_end`, and on a record Hydra could not read
    /// back. Those are the only two, `error` says which, and a blocking
    /// `hydra run` that read its own record always carries an outcome.
    pub outcome: Option<&'static str>,
    pub outcome_source: Option<&'static str>,
    pub exit_code: Option<i32>,
    pub error: Option<String>,
    pub result_text: Option<String>,
    /// The run's branch. **Absent, never null, when the run had no checkout**,
    /// following contract 5.5's rule: a null branch invites the reader to ask
    /// which branch, and outside a repo there is no such question.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    /// Absent for the same reason, and also when git could not be asked.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diff_stat: Option<DiffStat>,
    pub usage: Usage,
    pub run_file: String,
}

/// What the run changed, contract 17's one field with no record equivalent.
///
/// **The numbers are against the seed, not against `HEAD`.** Harness spec 6.3
/// seeds a run's worktree with the parent's uncommitted work, so `git diff HEAD`
/// inside it reports the human's work in progress as if the child had written it.
/// `covers` carries that sentence into the JSON, because the number is useless to
/// anyone who does not know what it is measured from.
#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub struct DiffStat {
    pub covers: &'static str,
    /// The tree the worktree held once Hydra finished seeding it. Every number
    /// beside it is a diff against this, and it is here so a reader can redo the
    /// diff by hand.
    pub base_tree: String,
    pub files_changed: usize,
    pub insertions: u64,
    pub deletions: u64,
}

/// Contract 17's `usage` with its `usage_source`, section 9's counts inside it.
#[derive(Serialize)]
pub struct Usage {
    pub usage_source: &'static str,
    /// Which tokenizer ran. Null unless the source is `estimated`, and nothing
    /// estimates.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub estimator: Option<String>,
    /// **Absent rather than zeroed** when the provider reported nothing (7.8). A
    /// null cannot be told from a provider reporting zero, and neither can a set
    /// of zeroes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub counts: Option<Counts>,
}

/// Section 9's fields, plus the two 9.1 derives.
#[derive(Serialize, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: u64,
    /// **Absent, never null, on a provider that reports no such count.** Section
    /// 9's table marks `cache_write_tokens` absent on `agy` and
    /// `thinking_tokens` absent on `cursor-agent`, and 5.5's rule is the same
    /// here as on the record: the question does not apply rather than having a
    /// missing answer. `claude` reports both, so neither is absent today.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_write_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking_tokens: Option<u64>,
    /// `input_tokens + cache_read_tokens + cache_write_tokens`, the prompt the
    /// model was actually sent. Derived here rather than left to the caller
    /// because 9.1 exists to stop a reader treating `input_tokens` as the prompt
    /// size, and a projection that ships only the parts invites exactly that.
    pub raw_input: u64,
    pub total_tokens: u64,
}

impl Usage {
    /// No counts and no source to speak of. What a caller gets when the record
    /// could not be read at all, which is not the same question as a provider
    /// that reported nothing and gets the same answer, because there is no
    /// third value in 7.8's enum to distinguish them and inventing one would
    /// need a producer.
    pub fn unavailable() -> Self {
        Self {
            usage_source: UsageSource::Unavailable.as_str(),
            estimator: None,
            counts: None,
        }
    }

    /// The `usage` event of a normalized run, projected.
    pub fn of(run: &Run) -> Self {
        let Some(usage) = run.usage() else {
            // No `usage` event at all means the file has no `run_end`, since the
            // adapter emits an `unavailable` one before that line on any run
            // whose provider reported nothing. Saying `unavailable` is the same
            // answer either way.
            return Self::unavailable();
        };
        Self {
            usage_source: usage.source.as_str(),
            estimator: usage.estimator.clone(),
            counts: usage.counts.map(|counts| Counts {
                input_tokens: counts.input_tokens,
                output_tokens: counts.output_tokens,
                cache_read_tokens: counts.cache_read_tokens,
                cache_write_tokens: counts.cache_write_tokens,
                thinking_tokens: counts.thinking_tokens,
                raw_input: counts.raw_input(),
                total_tokens: counts.total_tokens(),
            }),
        }
    }
}
