//! Re-evaluating a run's own policy at read time, contract 7.7.
//!
//! `permission_decision` is derived rather than stored, and contract 11 spends a
//! paragraph on why: the policy is on `run_start`, the call is in `raw`, so the
//! decision is a function of two things already on disk. What that paragraph
//! leaves open is what does the evaluating. This does, by compiling the run's
//! policy into the gate of 7.5 again and asking it about each call.
//!
//! **A second implementation of the decision in Rust was the alternative, and it
//! is the wrong trade.** The gate decides with the seven path denials, the scope
//! resolution, the `mcp__` prefix rule and the pattern floor of 7.4, and the
//! floor tokenizes a shell command with `shlex`. A Rust copy of all that drifts
//! from the Python the day either side is edited, and a copy that answers
//! `allow` where the gate answered `deny` reports a gate leak that never
//! happened. Running the real script keeps that check worth reading: a
//! disagreement then means the policy directory moved or the environment
//! differs, never that two spellings of one rule fell out of step.
//!
//! Two costs, both in `IMPLEMENTATION.md`. It is a process per tool call, and
//! `/usr/bin/python3` becomes a read-time dependency as well as a run-time one.

use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};
use serde_json::{Value, json};

use crate::policy::{Dialect, PolicySet, Spawn, compile};

/// The interpreter the gate's shebang names.
///
/// Read time runs the same one run time does, on purpose. `python3` on a
/// developer's PATH is Homebrew's, the gate runs on the 3.9.6 macOS ships, and
/// a rule that behaves differently on the two would be invisible if this
/// re-evaluation used the other one.
const GATE_PYTHON: &str = "/usr/bin/python3";

/// One run's policy, compiled and ready to answer.
///
/// Build it once per run rather than once per call: compiling walks the policy
/// directory, and a corpus reader would otherwise hash it for every tool call in
/// every record.
pub struct Replay {
    script: String,
    policy: String,
    /// The shape the compiled gate insists on being asked in, which is the
    /// run's provider's. The answer is read back in the same shape.
    dialect: Dialect,
}

/// What the policy says about one call, in the gate's own words.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Verdict {
    pub allowed: bool,
    pub reason: String,
}

impl Verdict {
    /// The `hydra/<code>` prefix the gate puts on every reason.
    ///
    /// Ticket 01 made these stable so a reader keys on the code rather than on
    /// the prose after it. `allow` is one of the codes.
    pub fn code(&self) -> Option<&str> {
        reason_code(&self.reason)
    }
}

/// The machine-readable half of a gate reason: `hydra/protected-path: ...`
/// gives `protected-path`.
///
/// Returns `None` for a reason no Hydra gate wrote, which is how a reader tells
/// Hydra's `PreToolUse` hook response from the user's own guards on the same
/// event.
pub fn reason_code(reason: &str) -> Option<&str> {
    let rest = reason.strip_prefix("hydra/")?;
    let code = rest.split(':').next()?;
    if code.is_empty() { None } else { Some(code) }
}

impl Replay {
    /// Compiles the run's policy against the scopes the run had.
    ///
    /// The caller builds the [`Spawn`] out of `run_start`, which is what makes
    /// this a re-evaluation of the run's own policy rather than of today's
    /// defaults. Two fields deserve care. `gate_script` has to name the path the
    /// run's gate really lived at, because the script denies writes to itself
    /// and a placeholder would deny the wrong file. `floor_lifted` has no field
    /// on the record at all, so a caller can only pass `false`, and a run where
    /// a human typed `--allow-floor-escape` re-evaluates to a deny it did not
    /// get. `IMPLEMENTATION.md` has carried that gap since ticket 02.
    pub fn compile(set: &PolicySet, spawn: &Spawn<'_>) -> Result<Self> {
        Ok(Self {
            script: compile(set, spawn)?,
            policy: spawn.policy.to_string(),
            dialect: spawn.provider.dialect(),
        })
    }

    pub fn policy(&self) -> &str {
        &self.policy
    }

    /// The policy's answer for one recorded tool call.
    ///
    /// The payload is the shape the provider sends its tool hook, minus the
    /// envelope fields the gate ignores: `tool_name`, `tool_input` and `cwd`,
    /// which is what both routed providers send on that event. `cwd` is the
    /// run's own checkout, which is what the child sent, and it matters because
    /// a relative path in a tool argument resolves against it.
    pub fn decide(&self, tool: &str, input: &Value, cwd: &Path) -> Result<Verdict> {
        let event = match self.dialect {
            Dialect::Claude | Dialect::Agy => "PreToolUse",
            Dialect::Cursor => "preToolUse",
        };
        let payload = json!({
            "hook_event_name": event,
            "tool_name": tool,
            "tool_input": input,
            "cwd": cwd.to_string_lossy(),
        });

        // `-c` rather than a file on disk. The script is one argument and the
        // payload is stdin, so nothing is written anywhere and there is no
        // temporary gate for a concurrent reader to trip over. With `-c`,
        // `sys.argv[1]` is the dialect, which is what the script reads.
        let mut child = Command::new(GATE_PYTHON)
            .arg("-c")
            .arg(&self.script)
            .arg(self.dialect.as_str())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .with_context(|| format!("running the gate on {GATE_PYTHON}"))?;
        child
            .stdin
            .as_mut()
            .context("the gate's stdin closed")?
            .write_all(payload.to_string().as_bytes())?;
        let out = child.wait_with_output()?;

        let stdout = String::from_utf8_lossy(&out.stdout);
        let answer: Value = serde_json::from_str(stdout.trim()).with_context(|| {
            format!(
                "the gate answered something that is not JSON: {:?} (stderr {:?})",
                stdout,
                String::from_utf8_lossy(&out.stderr)
            )
        })?;
        let (decision, reason) = match self.dialect {
            Dialect::Claude => {
                let specific = &answer["hookSpecificOutput"];
                (
                    specific["permissionDecision"].as_str(),
                    specific["permissionDecisionReason"].as_str(),
                )
            }
            Dialect::Cursor => (answer["permission"].as_str(), answer["user_message"].as_str()),
            Dialect::Agy => (answer["decision"].as_str(), answer["reason"].as_str()),
        };
        let reason = reason.unwrap_or_default().to_string();
        match decision {
            Some("allow") => Ok(Verdict {
                allowed: true,
                reason,
            }),
            Some("deny") => Ok(Verdict {
                allowed: false,
                reason,
            }),
            other => bail!("the gate answered {other:?}, which is neither allow nor deny"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_code_is_read_off_the_prefix_and_nothing_else() {
        assert_eq!(
            reason_code("hydra/protected-path: the command names /x"),
            Some("protected-path")
        );
        assert_eq!(
            reason_code("hydra/allow: policy standard allows shell."),
            Some("allow")
        );
        // The user's own guards write prose in their own voice, and a reason
        // with no prefix is how a reader tells one from Hydra's.
        assert_eq!(reason_code("Rejected: t30 CUR-PROJ says allow"), None);
        assert_eq!(reason_code("hydra/"), None);
    }
}
