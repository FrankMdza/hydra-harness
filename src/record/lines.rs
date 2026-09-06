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
    pub cwd: String,
    pub worktree: Option<WorktreeRef>,
    pub depth: u32,
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
}

#[derive(Serialize, Clone)]
pub struct WorktreeRef {
    pub path: String,
    pub branch: String,
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
    /// the vocabulary, and nothing in this build writes one: all four checks
    /// belong to tickets that are not built.
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

    /// A run Hydra refused. No child ever started, so there is no exit code and
    /// no body, and contract 8.3 requires the file all the same: without it the
    /// attempt is invisible, and under D10 anything not in a run record did not
    /// happen.
    ///
    /// Nothing calls this yet. It is here rather than in each of the four tickets
    /// that will, so the codes have one home and the derivation in
    /// [`crate::outcome`] reads the same strings the writer wrote.
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
    /// A child named a policy wider than its parent's, measured as capability set
    /// containment (contract 8.3). Ticket 11.
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
