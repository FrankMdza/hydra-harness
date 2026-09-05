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
    /// decided rather than read, and this names the check for them. Nothing in this
    /// slice sets it: the three refusals that would are pre-flight checks it does
    /// not run.
    pub refusal: Option<String>,
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
            ended_at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
        }
    }
}
