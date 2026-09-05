//! `claude`, measured on 2.1.261.
//!
//! The command line is harness spec 6.7's, in both of its policy shapes, with the
//! two MCP flags held back and one redirect dropped.
//!
//! **`--mcp-config` and `--strict-mcp-config` are not passed**, and the two are held
//! back for different reasons. 6.7 puts both on a non-`judge` recorded run. Ticket 08
//! writes the file `--mcp-config` points at, so passing that one now would name a file
//! that does not exist. `--strict-mcp-config` is the one that costs something: 6.5 says
//! it stops the human's inherited servers starting at all, and without it a recorded
//! run's server set is the state of five directories rather than an argument Hydra
//! wrote. Measured on ticket 02's three runs, every init frame came back
//! `mcp_servers: []`, so nothing was inherited on this machine. That is an observation
//! and not the guarantee the flag would make. 6.7 pairs the two, so both go on together
//! in ticket 08.
//!
//! **`< /dev/null` is dropped, because a pty is not a pipe.** 6.7 adds it so
//! `claude` does not wait three seconds for stdin and write `no stdin data received
//! in 3s` into the record. Measured 2026-09-05 on 2.1.261 over a pty Hydra owned:
//! no wait, no warning, first frame in well under a second. The redirect was a
//! finding about a pipe, and Hydra owns a pty because it has to be the parent to
//! tee anyway.

use std::path::Path;

use crate::policy::Dialect;
use crate::provider::Observed;

/// The argv after the binary name.
pub struct Launch<'a> {
    /// The preamble and the caller's task, already composed. Contract 5.1 records
    /// this string verbatim.
    pub task: &'a str,
    pub model: Option<&'a str>,
    /// The hook config, `~/hydra/spawn/<run-id>/settings.json`.
    pub settings: &'a Path,
    /// The policy name, which picks between 6.7's two shapes.
    pub policy: &'a str,
}

/// `judge` takes `--restricted` and a shorter line than the other four.
///
/// 6.7: no `--permission-mode`, which `--restricted` refuses; no `--mcp-config`,
/// which it drops anyway; no `--forward-subagent-text`, because `judge` allows
/// nothing and spawns nothing. The cost of `--restricted` everywhere else is that
/// it strips Bash and drops MCP, which is the point for a policy that allows
/// nothing and unaffordable for the rest.
pub const RESTRICTED_POLICY: &str = "judge";

pub fn argv(launch: &Launch<'_>) -> Vec<String> {
    let restricted = launch.policy == RESTRICTED_POLICY;

    let mut argv = vec![
        "--print".to_string(),
        "--output-format".to_string(),
        "stream-json".to_string(),
        // Not optional: --print with stream-json refuses to start without it.
        "--verbose".to_string(),
        // Puts hook_started and hook_response frames on the stream, which is the
        // second, independent source on a permission decision. It also pulls the
        // stdout of the user's own hooks into every recorded run.
        "--include-hook-events".to_string(),
    ];

    if restricted {
        argv.push("--restricted".to_string());
    } else {
        argv.push("--forward-subagent-text".to_string());
    }

    // 6.6: Hydra writes the config, so the gate reaches the child on this flag.
    // The user's own settings still load and their hooks still run, which is
    // composition rather than isolation and is what 6.6 chose on purpose.
    argv.push("--settings".to_string());
    argv.push(launch.settings.display().to_string());

    if !restricted {
        // 6.7: safe because a hook deny beats it, measured, and because a detached
        // child has nobody to answer a prompt. The gate does the refusing.
        argv.push("--permission-mode".to_string());
        argv.push("bypassPermissions".to_string());
    }

    if let Some(model) = launch.model {
        argv.push("--model".to_string());
        argv.push(model.to_string());
    }
    argv.push(launch.task.to_string());
    argv
}

/// The hook config `--settings` points at, 6.6.
///
/// **One `PreToolUse` entry matching every tool.** `claude` routes reads through
/// `PreToolUse` like every other tool, so one event covers this provider. Cursor
/// is the one that needs `beforeReadFile` as well, and that is ticket 09.
///
/// **The dialect is on the command line and is never inferred.** With no argument
/// the script denies in two shapes and exits 2. `claude` ignores a response in a
/// shape it does not recognise and allows the call, so a script that guesses wrong
/// is an open door on exactly this provider.
///
/// **No `timeout` is set**, so `claude`'s own default applies. A hook that fails to
/// answer fails open here, and inventing a shorter budget than the default only
/// moves the same failure closer. The script imports nothing outside the standard
/// library and answers in milliseconds.
pub fn settings(gate_script: &Path, gate_sha: &str) -> serde_json::Value {
    let command = gate_command(gate_script, gate_sha);
    serde_json::json!({
        "hooks": {
            "PreToolUse": [
                {
                    "matcher": "*",
                    "hooks": [
                        { "type": "command", "command": command }
                    ]
                }
            ]
        }
    })
}

/// The hook command: check the gate against the sha it was written with, then run
/// it.
///
/// **The script is a file the child can write and the config is not.** All three
/// providers snapshot their hook configuration at process start (6.6), while 7.5
/// says the script is "read fresh on every invocation". 7.5 closes that for
/// `judge`, `read-only` and `standard` by scoping their writes to the worktree,
/// and gives `permissive` and `curate` a base case by denying writes to the script
/// under every policy. Ticket 02's review found the base case reachable anyway:
/// the denial was applied to tools carrying a path and `Bash` carries none, so a
/// shell redirect emptied the script, and an empty script exits 0 printing nothing,
/// which 6.6 measured `claude` treating as permission to proceed.
///
/// The path check now refuses a command that names the script, and this is the
/// half that does not depend on reading argv. The sha is computed after the script
/// is written and travels in the config, which the provider has already snapshotted
/// by the time the child can run anything. A rewritten or deleted script therefore
/// fails **closed**, in Hydra's own voice, whatever spelling reached it.
///
/// The script still lives where 7.5 puts it. Only the pointer got stricter.
fn gate_command(gate_script: &Path, gate_sha: &str) -> String {
    // JSON string literals are Python string literals, so this quotes a path
    // containing anything a filesystem allows without a second escaping rule.
    let path = serde_json::to_string(&gate_script.display().to_string())
        .unwrap_or_else(|_| "\"\"".to_string());
    let sha = serde_json::to_string(gate_sha).unwrap_or_else(|_| "\"\"".to_string());
    let source = format!(
        r#"import hashlib, json, sys
GATE = {path}
SHA = {sha}
try:
    body = open(GATE, "rb").read()
except OSError:
    body = None
if body is None or hashlib.sha256(body).hexdigest() != SHA:
    sys.stdout.write(json.dumps({{"hookSpecificOutput": {{
        "hookEventName": "PreToolUse",
        "permissionDecision": "deny",
        "permissionDecisionReason": "hydra/gate-tampered: the compiled gate is gone or no longer matches the sha this run was launched with, so nothing can be decided and the call is refused."}}}}))
    sys.exit(0)
exec(compile(body, GATE, "exec"))
"#
    );
    // `python3 -c <source> claude` puts the dialect at sys.argv[1], which is where
    // the gate reads it, and leaves `__name__` as `__main__` so its own entry
    // point still fires under `exec`.
    format!(
        "/usr/bin/python3 -c {} {}",
        shell_quote(&source),
        Dialect::Claude.as_str()
    )
}

/// One shell word.
///
/// The hook `command` is a string a shell runs, and it carries both a path under a
/// home directory Hydra does not name and a block of Python. A space or a quote in
/// either would split the command and run the gate with no dialect, which denies
/// and exits 2 rather than failing open, but would deny every call on every run.
fn shell_quote(text: &str) -> String {
    format!("'{}'", text.replace('\'', r"'\''"))
}

/// Reads the model and the session id off a frame.
///
/// Both land on `system`/`init`, and both are on every frame after it. Hydra takes
/// them from the init frame and stops looking, because a reader that wants the
/// session id later takes it from `raw`, which is where it will always be.
pub fn observe(raw: &str) -> Observed {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(raw) else {
        return Observed::default();
    };
    if value.get("type").and_then(|v| v.as_str()) != Some("system") {
        return Observed::default();
    }
    if value.get("subtype").and_then(|v| v.as_str()) != Some("init") {
        return Observed::default();
    }
    Observed {
        model: value
            .get("model")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        session_id: value
            .get("session_id")
            .and_then(|v| v.as_str())
            .map(str::to_string),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn launch<'a>(policy: &'a str, settings: &'a Path) -> Launch<'a> {
        Launch {
            task: "hi",
            model: None,
            settings,
            policy,
        }
    }

    #[test]
    fn the_gate_reaches_the_child_on_settings() {
        let path = Path::new("/tmp/spawn/r1/settings.json");
        let argv = argv(&launch("standard", path));
        let at = argv.iter().position(|a| a == "--settings").unwrap();
        assert_eq!(argv[at + 1], "/tmp/spawn/r1/settings.json");
    }

    #[test]
    fn the_permissive_flag_rides_with_the_gate() {
        // 6.7: safe because a hook deny beats it. Measured on this wiring before
        // the flag went on, and `IMPLEMENTATION.md` carries the numbers.
        let argv = argv(&launch("standard", Path::new("/tmp/s.json")));
        let at = argv.iter().position(|a| a == "--permission-mode").unwrap();
        assert_eq!(argv[at + 1], "bypassPermissions");
        assert!(argv.iter().any(|a| a == "--settings"));
    }

    #[test]
    fn judge_is_restricted_and_takes_no_permission_mode() {
        let argv = argv(&launch("judge", Path::new("/tmp/s.json")));
        assert!(argv.iter().any(|a| a == "--restricted"));
        assert!(
            !argv.iter().any(|a| a == "--permission-mode"),
            "--restricted refuses it"
        );
        assert!(
            !argv.iter().any(|a| a == "--forward-subagent-text"),
            "judge allows nothing and spawns nothing"
        );
        assert!(
            argv.iter().any(|a| a == "--settings"),
            "--restricted ignores user, project and local settings and honours this one"
        );
    }

    #[test]
    fn the_mcp_flags_wait_for_ticket_08() {
        let argv = argv(&launch("standard", Path::new("/tmp/s.json")));
        assert!(!argv.iter().any(|a| a == "--mcp-config"));
        assert!(!argv.iter().any(|a| a == "--strict-mcp-config"));
    }

    #[test]
    fn verbose_rides_with_stream_json() {
        let argv = argv(&launch("standard", Path::new("/tmp/s.json")));
        assert!(argv.iter().any(|a| a == "--verbose"));
        assert!(argv.iter().any(|a| a == "stream-json"));
    }

    #[test]
    fn the_task_is_the_last_argument() {
        let argv = argv(&Launch {
            task: "do the thing",
            model: Some("sonnet"),
            settings: Path::new("/tmp/s.json"),
            policy: "standard",
        });
        assert_eq!(argv.last().unwrap(), "do the thing");
    }

    fn hook_command(gate: &str, sha: &str) -> String {
        settings(Path::new(gate), sha)["hooks"]["PreToolUse"][0]["hooks"][0]["command"]
            .as_str()
            .unwrap()
            .to_string()
    }

    #[test]
    fn the_hook_names_the_dialect_last() {
        let command = hook_command("/tmp/spawn/r1/gate", "abc123");
        // With no argument the script denies in two shapes and exits 2, which is
        // safe and useless. This is the typo that fails closed loudly.
        assert!(command.ends_with(" claude"), "got {command}");
        assert!(command.starts_with("/usr/bin/python3 -c "));
        assert_eq!(
            settings(Path::new("/tmp/spawn/r1/gate"), "abc123")["hooks"]["PreToolUse"][0]["matcher"],
            "*"
        );
    }

    #[test]
    fn the_hook_carries_the_sha_the_gate_was_written_with() {
        let command = hook_command("/tmp/spawn/r1/gate", "deadbeef");
        assert!(command.contains("deadbeef"), "got {command}");
        assert!(command.contains("/tmp/spawn/r1/gate"));
        assert!(
            command.contains("hydra/gate-tampered"),
            "a mismatch has to answer in the dialect, not just exit"
        );
    }

    #[test]
    fn a_home_with_a_quote_in_it_stays_one_shell_word() {
        let command = hook_command("/Users/o'brien/hydra/spawn/r1/gate", "abc");
        // One `-c` argument, so exactly two closing quotes: the escape and the end.
        assert!(command.contains(r"'\''"), "got {command}");
        assert!(command.ends_with("' claude"));
    }

    #[test]
    fn the_init_frame_names_the_model_and_the_session() {
        let observed = observe(
            r#"{"type":"system","subtype":"init","model":"claude-sonnet-5","session_id":"abc"}"#,
        );
        assert_eq!(observed.model.as_deref(), Some("claude-sonnet-5"));
        assert_eq!(observed.session_id.as_deref(), Some("abc"));
    }

    #[test]
    fn an_assistant_frame_teaches_the_envelope_nothing() {
        let observed = observe(r#"{"type":"assistant","message":{"model":"claude-sonnet-5"}}"#);
        assert!(observed.model.is_none());
    }
}
