//! The two Hydra lines: `run_start` and `run_end`.
//!
//! Contract sections 5 and 6. These carry facts no provider frame holds, which is
//! why they are stored rather than derived, and they are the only two lines in a
//! run file that are.
//!
//! Two groups follow the absent-against-null rule from contract 5.5 and 5.6. A null
//! `skills.store_sha` invites a reader to ask which sha the run pinned. An absent
//! `skills` object says the question does not apply, which is the true answer when
//! nothing was injected. A null `policy` is different: the run really did have a
//! policy question and the answer really is none.

use serde::Serialize;

/// The first line of every run file, written at launch.
#[derive(Serialize)]
pub struct RunStart {
    #[serde(rename = "type")]
    pub kind: &'static str,
    pub schema_version: u32,
    pub hydra_version: String,
    /// `run`, `evolve`, `resume` or `curate`. This slice mints only `run`.
    pub command: String,
    pub launch_mode: Option<String>,
    pub provider_version: Option<String>,
    /// Sometimes late and sometimes absent. On `claude` it lands on the `system`
    /// init frame, so a reader takes it from `raw` rather than from here.
    pub provider_session_id: Option<String>,
    /// The full task string as passed, preamble included (contract 5.1). Harness
    /// spec 8's preamble rides here rather than in an injected skill, so the
    /// record shows exactly what the run was told and the preamble needs no sha.
    pub task: Option<String>,
    pub policy: Option<String>,
    pub policy_sha: Option<String>,
    /// `--allow-floor-escape`, harness spec 16. A fact about the request rather
    /// than about the gate, so it is filled on every recorded run, a refused one
    /// included, and null on a harness run and an interactive one, where no gate
    /// is compiled (contract 5.5 and 5.6).
    pub floor_lifted: Option<bool>,
    /// The absolute paths the gate resolved its scopes to, keyed by scope name
    /// (contract 5.1). Absent, never null, on a run that compiled no gate, and a
    /// key the run had no path for is absent from the object.
    ///
    /// **The sixth input to the compile, and the one nothing recovers.**
    /// `policy_sha` pins the rules and this pins what they were applied to. Two
    /// correct builds resolved `skill_store` two ways on 2026-09-06 and ten
    /// in-scope reads on one record re-derived as ten gate leaks on the other,
    /// with no field saying the two builds differed (map ticket 42).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scopes: Option<std::collections::BTreeMap<String, String>>,
    pub cwd: String,
    pub worktree: Option<WorktreeRef>,
    pub depth: u32,
    /// The policy the parent run was launched under, on a delegated run and
    /// absent on every other.
    ///
    /// **Contract 8.3 asks for it and contract 5.1 named no field for it.** A
    /// refused spawn's record has to carry "the requested policy and the parent's
    /// beside it", and `policy` is the requested one. Absent rather than null
    /// under 5.5's rule: a run a human started has no parent, so which policy the
    /// parent held is not a question with a missing answer. Contract 15 allows the
    /// addition, because a reader that ignores the field reads the record exactly
    /// as it did before.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_policy: Option<String>,
    pub pid: Option<u32>,
    pub process_start_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skills: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mcp: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth: Option<Auth>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env_allowlist: Option<Vec<String>>,
    pub subscription: Option<String>,
    /// The run id whose provider session this run continued, present only when
    /// `command` is `resume` (contract 5.1).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resumed_from: Option<String>,
}

#[derive(Serialize, Clone, Debug)]
pub struct WorktreeRef {
    pub path: String,
    pub branch: String,
    /// The sha `git write-tree` answered against the seed index the moment the
    /// worktree was seeded, before the provider started, map ticket 49's
    /// decision. Null only when `write-tree` did not answer, which is what the
    /// existing `Option` on [`crate::worktree::Worktree::seed_tree`] already
    /// models and costs the diff stat and nothing else.
    ///
    /// The tree it names is reachable from no ref, so a `git gc` on the parent
    /// repository can collect it. This field is a name rather than a guarantee
    /// that the object survives, and a reader that wants the number to keep
    /// working has to keep the object alive itself.
    pub base_tree: Option<String>,
}

#[derive(Serialize)]
pub struct Auth {
    /// `subscription-oauth` or `api-key`.
    pub mechanism: &'static str,
    /// `keychain`, `env` or `none`.
    pub source: &'static str,
    pub allow_billed: bool,
    /// True when Hydra fully controls this provider's environment. It costs nothing
    /// today and is not therefore pointless: it is what makes "a provider Hydra
    /// cannot gate is either given a config root Hydra owns, or is not routed"
    /// checkable rather than inferred from which provider ran.
    pub gated: bool,
}

/// The last line of a run file, written when `wait()` returns.
///
/// It is not guaranteed to exist. If Hydra dies, nothing writes it, and a dead pid
/// with no `run_end` is an abandoned run rather than a corrupt file.
#[derive(Serialize)]
pub struct RunEnd {
    #[serde(rename = "type")]
    pub kind: &'static str,
    pub exit_code: Option<i32>,
    /// Text Hydra captured that is not on the stream.
    pub error: Option<String>,
    /// The Hydra-side check that refused, or null. Four outcomes are things Hydra
    /// decided rather than read, and this names the check for them. [`Refusal`] is
    /// the vocabulary, and three of its four codes have a writer: policy
    /// narrowing, the skill collision and the MCP launch check. The auth
    /// pre-flight belongs to a ticket that is not built.
    pub refusal: Option<&'static str>,
    pub remediation: Option<String>,
    /// `stream` or `harvest`. Always `stream` here, because this slice launches no
    /// interactive runs and harvests nothing.
    pub body_source: Option<&'static str>,
    pub ended_at: String,
}

impl RunEnd {
    pub fn streamed(exit_code: Option<i32>, error: Option<String>) -> Self {
        Self {
            kind: "run_end",
            exit_code,
            error,
            refusal: None,
            remediation: None,
            body_source: Some("stream"),
            ended_at: now(),
        }
    }

    /// The closing line an interactive run gets, contract 12: the vendor's own
    /// transcript stood in for a stream nobody teed. `exit_code` is `None` when
    /// harvest ran after the fact and the process that exited was never waited
    /// on, and `error` is what says so.
    pub fn harvested(exit_code: Option<i32>, error: Option<String>) -> Self {
        Self {
            kind: "run_end",
            exit_code,
            error,
            refusal: None,
            remediation: None,
            body_source: Some("harvest"),
            ended_at: now(),
        }
    }

    /// A run Hydra refused. No child ever started, so there is no exit code and
    /// no body, and contract 8.3 requires the file all the same: without it the
    /// attempt is invisible, and under D10 anything not in a run record did not
    /// happen.
    ///
    /// The codes live in one place rather than in each check that writes one, so
    /// the derivation in [`crate::outcome`] reads the same strings the writer
    /// wrote. [`crate::run::refuse`] is the only caller.
    pub fn refused(refusal: Refusal, remediation: impl Into<String>) -> Self {
        Self {
            kind: "run_end",
            exit_code: None,
            error: None,
            refusal: Some(refusal.as_str()),
            remediation: Some(remediation.into()),
            body_source: None,
            ended_at: now(),
        }
    }
}

fn now() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

/// The four Hydra-side checks that can refuse a launch, contract 6.
///
/// **These are codes rather than prose**, in the shape ticket 01 gave the gate's
/// `hydra/` reason prefix and for the same reason: the derivation has to map one
/// back to an outcome, and a sentence is not something to match on. The
/// remediation field beside it is where the words for a human go.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// A child named a policy its parent's does not contain, measured as
    /// capability set containment (contract 8.3). [`crate::run::Lineage`] decides
    /// it and 7.3 is the rule.
    PolicyNarrowing,
    /// A linked skill name the provider did not resolve as Hydra's. Ticket 07.
    SkillCollision,
    /// The MCP launch check, or Cursor's pre-flight. Ticket 08.
    ///
    /// Contract 8's `mcp_unavailable` covers both and harness spec 12 says which
    /// one fired is a field Hydra sets rather than a second outcome, so this is
    /// one code and the remediation says which check it was.
    McpUnavailable,
    /// The auth pre-flight. Not built, and the mid-run half of `auth_failed`
    /// derives from the stream instead.
    AuthPreflight,
}

impl Refusal {
    pub fn as_str(self) -> &'static str {
        match self {
            Refusal::PolicyNarrowing => "policy_narrowing",
            Refusal::SkillCollision => "skill_collision",
            Refusal::McpUnavailable => "mcp_unavailable",
            Refusal::AuthPreflight => "auth_preflight",
        }
    }

    /// The outcome a recorded refusal derives to, contract 8.2's `refusal` source.
    ///
    /// **A code this build does not know still comes out `gated`.** All four
    /// refusals are outcomes rung one never learns from, so the class is right
    /// even when the specific check is from a version this binary predates, and
    /// `gated` is the enum's own name for "Hydra refused". The alternative,
    /// falling through to `completed`, would put a run that never started into the
    /// corpus as a finished one.
    pub fn outcome_of(code: &str) -> crate::outcome::Outcome {
        use crate::outcome::Outcome;
        match code {
            "skill_collision" => Outcome::SkillCollision,
            "mcp_unavailable" => Outcome::McpUnavailable,
            "auth_preflight" => Outcome::AuthFailed,
            _ => Outcome::Gated,
        }
    }
}
