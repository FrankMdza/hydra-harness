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
