//! Re-evaluating a run's own policy at read time, contract 7.7.
//!
//! `permission_decision` is derived rather than stored, and contract 11 spends a
//! paragraph on why: the policy is on `run_start`, the call is in `raw`, so the
//! decision is a function of two things already on disk. What that paragraph
//! leaves open is what does the evaluating. This does, by compiling the run's
//! policy into the gate of 7.5 again and asking it about each call.
//!
//! **A second implementation of the decision in Rust was the alternative, and it
//! is the wrong trade.** The gate decides with the denied paths and their
//! carve-outs, the scope resolution, the `mcp__` prefix rule and the pattern
//! floor of 7.4, and the floor tokenizes a shell command with `shlex`. A Rust
//! copy of all that drifts from the Python the day either side is edited, and a
//! copy that answers `allow` where the gate answered `deny` reports a gate leak
//! that never happened. Running the real script keeps that check worth reading:
//! a disagreement then means the policy directory moved or the environment
//! differs, never that two spellings of one rule fell out of step.
//!
//! Two costs, both in `IMPLEMENTATION.md`. It is a process per record, and one
//! per tool call when that record's batch fails, and `/usr/bin/python3` becomes
//! a read-time dependency as well as a run-time one.

use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};
use serde_json::{Value, json};

use crate::policy::{Dialect, PolicySet, Spawn, compile};
use crate::provider::Provider;

/// The interpreter the gate's shebang names.
///
/// Read time runs the same one run time does, on purpose. `python3` on a
/// developer's PATH is Homebrew's, the gate runs on the 3.9.6 macOS ships, and
/// a rule that behaves differently on the two would be invisible if this
/// re-evaluation used the other one.
const GATE_PYTHON: &str = "/usr/bin/python3";

/// The variables a replayed gate is handed from the reader's environment.
///
/// The live gate ran in the provider's environment, which harness spec 6.2
/// builds from an allowlist, and `path_candidates` expands `$VAR` out of it.
/// A replay that inherits the reader's whole environment expands names the
/// run never had: measured on run 01a10445-e17a, a command spelling
/// `"$OLDPWD/docs/costos/..."` that the live gate allowed, and that the walk
/// replayed under the human's own `OLDPWD` into `~/hydra` and filed as a leak.
/// These are the names `src/env.rs` passes to every provider, plus the two it
/// gives `claude` alone, with the reader's values: the per-run names Hydra sets
/// itself, `HYDRA_RUN_ID` and the git object directories, stay unset, so a
/// command naming one is judged with the name unexpanded.
fn gate_env_names(provider: Provider) -> &'static [&'static str] {
    match provider {
        Provider::Claude => &["HOME", "USER", "PATH", "TERM", "SHELL", "TMPDIR"],
        Provider::CursorAgent | Provider::Agy | Provider::Codex => {
            &["HOME", "USER", "PATH", "TERM"]
        }
    }
}

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
    /// Which environment names the gate is handed, [`gate_env_names`].
    provider: Provider,
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
    /// defaults. Three fields deserve care. `gate_script` has to name the path
    /// the run's gate really lived at, because the script denies writes to
    /// itself and a placeholder would deny the wrong file, and `run_start.scopes`
    /// carries it since build ticket 15. `skill_snapshot` is recomputed from
    /// `hydra.skills.store_sha` through
    /// [`crate::paths::Layout::skill_snapshot_scope`], the same function the
    /// launch called, so a read through a skill link reads back the way the gate
    /// answered rather than as a leak. `floor_lifted` is read off line 0 too, and
    /// a record written before that field re-evaluates as if the floor was in
    /// force, so a run a human lifted it for derives a `hydra/floor` deny it did
    /// not get.
    pub fn compile(set: &PolicySet, spawn: &Spawn<'_>) -> Result<Self> {
        Ok(Self {
            script: compile(set, spawn)?,
            policy: spawn.policy.to_string(),
            dialect: spawn.provider.dialect(),
            provider: spawn.provider,
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
    ///
    /// This is the single-call entry the live hook uses, one interpreter per
    /// call. [`Replay::decide_all`] falls back to it when a batch fails.
    pub fn decide(&self, tool: &str, input: &Value, cwd: &Path) -> Result<Verdict> {
        let payload = self.payload(tool, input, cwd);
        // The gate expands `$VAR` out of its own environment, and the live
        // hook's environment is the provider's, where `PWD` is the run's
        // working directory. A replay that inherits this process's `PWD`
        // resolves `$PWD/x` under the reader's own cwd, which since build
        // ticket 74 is a denied path, and reports a leak the gate never
        // made; measured 2026-09-22 on run 01a0c9d9-6652, five commands
        // of the shape `HYDRA_ROOT=$PWD/.scratch-root cargo test`.
        let answer = self.run_gate(false, Some(cwd), &payload)?;
        verdict_of(self.dialect, &answer)
    }

    /// The policy's answers for every tool call in one record, one result each.
    ///
    /// One interpreter for the whole record when the batch answers. When the
    /// batch fails as a whole, an interpreter that died or an answer count
    /// that does not match, each call is asked on its own through
    /// [`Replay::decide`], so one call the batch could not carry costs that
    /// call a `Limit` and not every call in the record.
    pub fn decide_all(&self, calls: &[(&str, &Value, &Path)]) -> Vec<Result<Verdict>> {
        match self.decide_batch(calls) {
            Ok(verdicts) => verdicts.into_iter().map(Ok).collect(),
            Err(_) => calls
                .iter()
                .map(|(tool, input, cwd)| self.decide(tool, input, cwd))
                .collect(),
        }
    }

    /// The policy's answers for every tool call in one record, in one interpreter.
    ///
    /// The batch entry reads a JSON array of hook payloads on stdin and writes
    /// one answer per payload, in order. PWD cannot be set once on the process:
    /// two calls in one record can carry different `cwd`s, so each payload's
    /// `cwd` field is handed to the gate and the batch entry sets `os.environ`
    /// from it before evaluating that payload, which is the rule
    /// [`Replay::decide`] gets by setting the child's environment.
    pub fn decide_batch(&self, calls: &[(&str, &Value, &Path)]) -> Result<Vec<Verdict>> {
        if calls.is_empty() {
            return Ok(Vec::new());
        }
        let payloads = Value::Array(
            calls
                .iter()
                .map(|(tool, input, cwd)| self.payload(tool, input, cwd))
                .collect(),
        );
        let answers = self.run_gate(true, None, &payloads)?;
        let answers = answers.as_array().with_context(|| {
            format!("the gate batch entry answered {:?}, not an array", answers)
        })?;
        if answers.len() != calls.len() {
            bail!(
                "the gate answered {} payloads for {} calls",
                answers.len(),
                calls.len()
            );
        }
        answers
            .iter()
            .map(|answer| verdict_of(self.dialect, answer))
            .collect()
    }

    fn payload(&self, tool: &str, input: &Value, cwd: &Path) -> Value {
        let event = match self.dialect {
            Dialect::Claude | Dialect::Agy => "PreToolUse",
            Dialect::Cursor => "preToolUse",
        };
        let mut payload = json!({
            "hook_event_name": event,
            "tool_name": tool,
            "cwd": cwd.to_string_lossy(),
        });
        // A recorded call with no arguments reads back as null: every
        // normalizer clones a missing field, which yields `Value::Null`. The
        // gate refuses a `tool_input` that is present and null, so a null here
        // leaves the key out, which the gate reads as a call with no arguments.
        if !input.is_null() {
            payload["tool_input"] = input.clone();
        }
        payload
    }

    /// Runs the compiled gate once and reads its stdout as JSON.
    fn run_gate(&self, batch: bool, pwd: Option<&Path>, stdin: &Value) -> Result<Value> {
        // `-c` rather than a file on disk. The script is one argument and the
        // payload is stdin, so nothing is written anywhere and there is no
        // temporary gate for a concurrent reader to trip over. With `-c`,
        // `sys.argv[1]` is the dialect and `sys.argv[2]`, when present, is
        // `batch`.
        let mut command = Command::new(GATE_PYTHON);
        command
            .arg("-c")
            .arg(&self.script)
            .arg(self.dialect.as_str());
        if batch {
            command.arg("batch");
        }
        command.env_clear();
        for name in gate_env_names(self.provider) {
            if let Some(value) = std::env::var_os(name) {
                command.env(name, value);
            }
        }
        if let Some(pwd) = pwd {
            command.env("PWD", pwd);
        }
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .with_context(|| format!("running the gate on {GATE_PYTHON}"))?;
        child
            .stdin
            .as_mut()
            .context("the gate's stdin closed")?
            .write_all(serde_json::to_string(stdin)?.as_bytes())?;
        let out = child.wait_with_output()?;

        let stdout = String::from_utf8_lossy(&out.stdout);
        serde_json::from_str(stdout.trim()).with_context(|| {
            format!(
                "the gate answered something that is not JSON: {:?} (stderr {:?})",
                stdout,
                String::from_utf8_lossy(&out.stderr)
            )
        })
    }
}

fn verdict_of(dialect: Dialect, answer: &Value) -> Result<Verdict> {
    let (decision, reason) = match dialect {
        Dialect::Claude => {
            let specific = &answer["hookSpecificOutput"];
            (
                specific["permissionDecision"].as_str(),
                specific["permissionDecisionReason"].as_str(),
            )
        }
        Dialect::Cursor => (
            answer["permission"].as_str(),
            answer["user_message"].as_str(),
        ),
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

    /// A stand-in gate whose batch entry dies and whose single-call entry
    /// answers with the PWD it was handed, or dies on a tool named `broken`.
    const BATCH_DIES: &str = r#"
import json, os, sys
if len(sys.argv) > 2:
    sys.exit(1)
payload = json.loads(sys.stdin.read())
if payload["tool_name"] == "broken":
    sys.exit(1)
print(json.dumps({"decision": "allow", "reason": "hydra/allow: " + os.environ["PWD"]}))
"#;

    #[test]
    fn a_batch_that_fails_as_a_whole_is_asked_again_one_call_at_a_time() {
        let replay = Replay {
            script: BATCH_DIES.to_string(),
            policy: "standard".to_string(),
            dialect: Dialect::Agy,
            provider: Provider::Agy,
        };
        let input = json!({});
        let first = Path::new("/tmp/first");
        let second = Path::new("/tmp/second");
        let calls = [
            ("read_file", &input, first),
            ("broken", &input, second),
            ("read_file", &input, second),
        ];
        assert!(replay.decide_batch(&calls).is_err());

        let answers = replay.decide_all(&calls);
        assert_eq!(answers.len(), 3);
        // The single-call entry gets PWD from the child's environment, as the
        // live hook does, and each call gets its own.
        assert_eq!(
            answers[0].as_ref().unwrap().reason,
            "hydra/allow: /tmp/first"
        );
        assert!(answers[1].is_err(), "{:?}", answers[1]);
        assert_eq!(
            answers[2].as_ref().unwrap().reason,
            "hydra/allow: /tmp/second"
        );
    }

    #[test]
    fn a_recorded_call_with_null_arguments_is_sent_without_tool_input() {
        // The gate refuses a `tool_input` that is present and null, and a
        // record with no arguments reads back as null, so the replay leaves
        // the key out rather than report a refusal the live hook never made.
        let replay = Replay {
            script: String::new(),
            policy: "permissive".to_string(),
            dialect: Dialect::Claude,
            provider: Provider::Claude,
        };
        let cwd = Path::new("/tmp/run");
        for tool in ["Task", "Agent"] {
            let absent = replay.payload(tool, &Value::Null, cwd);
            assert!(absent.get("tool_input").is_none(), "{tool}: {absent}");
            let present = replay.payload(tool, &json!({ "prompt": "x" }), cwd);
            assert_eq!(present["tool_input"], json!({ "prompt": "x" }), "{tool}");
        }
    }

    /// A stand-in gate that answers with the names in its own environment.
    const ECHOES_ITS_ENV: &str = r#"
import json, os, sys
payload = json.loads(sys.stdin.read())
names = ",".join(sorted(os.environ))
print(json.dumps({"decision": "allow", "reason": "hydra/allow: " + names}))
"#;

    #[test]
    fn the_gate_is_replayed_without_the_readers_own_environment() {
        // Run 01a10445-e17a: `"$OLDPWD/docs/costos/x.xlsx"` resolved under the
        // reader's `OLDPWD` rather than the provider's, which never had one.
        // The names cargo sets for this test stand in for it: 6.2 passes none
        // of them to a child. Not every name off the list, because macOS's
        // `/usr/bin/python3` adds `__CF_USER_TEXT_ENCODING` and a few SDK
        // paths to its own environment whatever it was handed.
        let readers_own: Vec<String> = std::env::vars_os()
            .filter_map(|(name, _)| name.into_string().ok())
            .filter(|name| name.starts_with("CARGO_"))
            .collect();
        assert!(
            readers_own.iter().any(|name| name == "CARGO_MANIFEST_DIR"),
            "the test needs a name off the list in its own environment"
        );
        let replay = Replay {
            script: ECHOES_ITS_ENV.to_string(),
            policy: "standard".to_string(),
            dialect: Dialect::Agy,
            provider: Provider::Agy,
        };
        let verdict = replay
            .decide("run_command", &json!({}), Path::new("/tmp/run"))
            .unwrap();
        let seen: Vec<&str> = verdict
            .reason
            .trim_start_matches("hydra/allow: ")
            .split(',')
            .collect();
        for name in &readers_own {
            assert!(!seen.contains(&name.as_str()), "{name} reached the gate");
        }
        assert!(seen.contains(&"PWD"), "{seen:?}");
        if std::env::var_os("HOME").is_some() {
            assert!(seen.contains(&"HOME"), "{seen:?}");
        }
    }
}
