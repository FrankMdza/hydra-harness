//! The routable providers.
//!
//! Three in the design: `claude`, `cursor-agent`, `agy`. This slice routes one.
//! Every provider-specific fact is a property of the binary rather than of the
//! model, so a model is a flag inside a provider and never a provider of its own.

pub mod claude;

use std::fmt;
use std::str::FromStr;

use anyhow::{Result, bail};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Provider {
    Claude,
}

impl Provider {
    pub fn binary(self) -> &'static str {
        match self {
            Provider::Claude => "claude",
        }
    }

    /// The string the binary printed, read at launch.
    ///
    /// Contract 5.1 keeps this on every `run_start` because two of the three
    /// providers self-update without asking, and a claim measured on one build must
    /// not silently become a claim about another.
    pub fn version(self) -> Result<String> {
        let out = std::process::Command::new(self.binary())
            .arg("--version")
            .output();
        match out {
            Ok(out) if out.status.success() => {
                Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
            }
            Ok(out) => bail!(
                "{} --version exited {}: {}",
                self.binary(),
                out.status,
                String::from_utf8_lossy(&out.stderr).trim()
            ),
            Err(err) => bail!("{} is not on PATH: {err}", self.binary()),
        }
    }

    /// Where Hydra links this run's skills, relative to the worktree (6.4).
    ///
    /// The other two rows of 6.4's table are `.cursor/skills/` and
    /// `.agents/skills/`, and both come with a launch flag this build does not
    /// pass, so they arrive with their providers in tickets 09 and 10.
    pub fn skill_root(self) -> &'static str {
        match self {
            Provider::Claude => ".claude/skills",
        }
    }

    /// The user-scope roots this provider reads, which the collision check
    /// compares the store against.
    ///
    /// **`claude` reads one and `cursor-agent` reads six**, so this is a
    /// per-provider list rather than the index's own. Ticket 24 fired probes into
    /// all of them at both scopes; `crate::skills::index` scans the union and this
    /// is the filter over it.
    pub fn user_skill_roots(self, home: &std::path::Path) -> Vec<std::path::PathBuf> {
        match self {
            Provider::Claude => vec![home.join(".claude/skills")],
        }
    }

    /// What this provider says it resolved: contract 5.3's `skills.resolved` and
    /// `mcp.resolved`, from one probe.
    ///
    /// **One question and not two, because it is one process.** The skill listing
    /// and the MCP tool list arrive in the same transcript, so asking twice would
    /// pay twice for an answer that is already there.
    ///
    /// Free on `claude` and on `agy`, paid on `cursor-agent`. See
    /// [`claude::resolve`] for what free costs in wall-clock time.
    pub fn resolve(
        self,
        worktree: &std::path::Path,
        mcp: crate::mcp::Probe<'_>,
    ) -> Result<Resolved> {
        match self {
            Provider::Claude => claude::resolve(worktree, mcp),
        }
    }

    /// Which subscription pays for this binary's runs.
    ///
    /// Hydra authors this at spawn. `agy` serves Claude models on the Google
    /// subscription, so `provider` stopped answering which vendor's model replied
    /// and the two questions came apart. Hydra knows which binary it spawned and
    /// which subscription that binary is logged into, which is the only version
    /// that answers identically on all three.
    pub fn subscription(self) -> &'static str {
        match self {
            Provider::Claude => "anthropic",
        }
    }

    /// What the provider reported about this frame that the envelope needs.
    pub fn observe(self, raw: &str) -> Observed {
        match self {
            Provider::Claude => claude::observe(raw),
        }
    }
}

/// What one free pre-flight probe found, for contract 5.3's two `resolved`
/// fields.
///
/// Each half is `None` when the probe could not answer it. Contract 5.3 already
/// requires a reader to treat that as unanswered rather than as an empty listing,
/// and the two halves fail independently: a transcript can carry a tool list and
/// no skill listing.
#[derive(Default)]
pub struct Resolved {
    pub skills: Option<Vec<String>>,
    pub mcp: Option<Vec<String>>,
    /// Servers the provider named as failed, each as `name: reason`. Not a
    /// refusal: 6.5 gives the fatal pre-flight to Cursor alone and says why, so
    /// this reaches stderr and the record shows the server as injected and not
    /// resolved.
    pub failed_mcp: Vec<String>,
}

/// The two fields a frame can teach the envelope. Contract 4 and 5.1 both backfill
/// in memory rather than rewriting line 0.
#[derive(Default)]
pub struct Observed {
    pub model: Option<String>,
    pub session_id: Option<String>,
}

impl fmt::Display for Provider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.binary())
    }
}

impl FromStr for Provider {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "claude" => Ok(Provider::Claude),
            "cursor-agent" | "agy" => bail!(
                "{s} is a routable provider in the design and is not built yet; \
                 this slice routes claude only"
            ),
            other => bail!("{other} is not a routable provider"),
        }
    }
}
