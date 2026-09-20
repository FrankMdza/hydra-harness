//! `codex`, measured on 0.154.0 on 2026-09-12 in map ticket 58.
//!
//! The command line is harness spec 6.7's `codex` block as far as this build
//! implements it. A recorded run is `codex exec --json` with belt flags beside
//! the gate, `-c` overrides that point the hook at Hydra's script, and the
//! worktree named as the session root.
//!
//! **Hook trust is per-hash and a per-spawn gate never matches**, so
//! `--dangerously-bypass-hook-trust` rides every launch Hydra makes. The
//! record has to say so.
//!
//! **`--sandbox` and `--ask-for-approval never` are belt, not the gate.** They
//! narrow what the provider will do on its own; Hydra's compiled hook is still
//! what refuses under policy.
//!
//! **There is no read tool.** Every read is a `Bash` call, which is why
//! `read-only` and `judge` are refused at pre-flight until build ticket 48
//! item 4 is decided.
//!
//! **MEASURE 48.5**: on a logged-in binary, confirm whether a hook crash or
//! timeout fails open like `claude` or closed.

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};

use crate::mcp::Checkout;
use crate::policy::Dialect;
use crate::provider::{Observed, Resolved};

/// The argv after the binary name.
pub struct Launch<'a> {
    /// The preamble and the caller's task, already composed. Contract 5.1
    /// records this string verbatim.
    pub task: &'a str,
    pub model: Option<&'a str>,
    /// The run's checkout. Skills, hooks and MCP config all load from here.
    pub worktree: &'a Path,
    /// The gate command Hydra compiled, for the `-c` override beside the file.
    pub gate_command: &'a str,
}

/// 6.7's `codex` line as far as ticket 48 implements it.
pub fn argv(launch: &Launch<'_>) -> Vec<String> {
    let mut argv = vec![
        "exec".to_string(),
        "--json".to_string(),
        "-C".to_string(),
        launch.worktree.display().to_string(),
        "--sandbox".to_string(),
        "workspace-write".to_string(),
        "--ask-for-approval".to_string(),
        "never".to_string(),
        "--dangerously-bypass-hook-trust".to_string(),
    ];
    for config_override in config_overrides(launch.gate_command) {
        argv.push("-c".to_string());
        argv.push(config_override);
    }
    if let Some(model) = launch.model {
        argv.push("-m".to_string());
        argv.push(model.to_string());
    }
    argv.push(launch.task.to_string());
    argv
}

/// Harness spec 17: `codex exec resume <session-id>`, measured on 0.154.0.
pub fn resume_args(session_id: &str) -> Vec<String> {
    vec![
        "exec".to_string(),
        "resume".to_string(),
        session_id.to_string(),
    ]
}

const HOOKS: &str = ".codex/hooks.json";
const MCP: &str = ".codex/config.toml";

/// Where the hook config goes, 6.6's table: inside the worktree, a real file.
pub fn hooks_file(worktree: &Path) -> PathBuf {
    worktree.join(HOOKS)
}

/// Where the MCP config goes, 6.5's table on this provider.
pub fn mcp_file(worktree: &Path) -> PathBuf {
    worktree.join(MCP)
}

/// The `.gitignore` Hydra drops beside its two files in `.codex/`.
pub fn gitignore(worktree: &Path) -> (PathBuf, &'static str) {
    (
        worktree.join(".codex/.gitignore"),
        "/.gitignore\n/hooks.json\n/config.toml\n",
    )
}

pub fn claim(
    worktree: Option<&Path>,
    what: &str,
    file: impl Fn(&Path) -> PathBuf,
) -> Result<PathBuf> {
    super::claim(worktree, "codex", what, file, gitignore)
}

/// The hook config in the shape `docs/hooks` documents for `hooks.json`.
///
/// One event, one handler, one script. The gate answers in `claude`'s dialect,
/// which is what map ticket 58 measured on 0.154.0.
pub fn hooks_config(gate_script: &Path, gate_sha: &str) -> serde_json::Value {
    let command = crate::policy::hook_command(gate_script, gate_sha, Dialect::Claude);
    serde_json::json!({
        "description": "Hydra gate for this run",
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

/// `-c` overrides that point the hook at Hydra's script on the command line.
///
/// MEASURE 48.5: on a logged-in binary, confirm that this override is honoured
/// as a hook source alongside or instead of `<worktree>/.codex/hooks.json`.
pub fn config_overrides(gate_command: &str) -> Vec<String> {
    let escaped = gate_command.replace('\\', "\\\\").replace('"', "\\\"");
    vec![format!(
        "hooks.PreToolUse=[{{matcher=\"*\", hooks=[{{type=\"command\", command=\"{escaped}\"}}]}}]"
    )]
}

/// Why a launch of this shape carries no MCP config at all, when it carries none.
pub fn mcp_undeliverable(checkout: Checkout) -> Option<&'static str> {
    match checkout {
        Checkout::Hydra => None,
        Checkout::None => Some(
            "harness spec 6.5 puts a codex run's MCP config inside the worktree, and this \
             launch has no checkout Hydra created to write one into",
        ),
    }
}

/// The `.codex/config.toml` fragment for admitted servers, 6.5.
pub fn mcp_config(servers: &[&crate::mcp::Server]) -> String {
    use crate::mcp::Transport;

    let mut out = String::new();
    for server in servers {
        out.push_str(&format!("\n[mcp_servers.{}]\n", server.name));
        match server.transport {
            Transport::Stdio => {
                out.push_str(&format!(
                    "command = \"{}\"\n",
                    server.command.clone().unwrap_or_default()
                ));
                if !server.args.is_empty() {
                    let args = server
                        .args
                        .iter()
                        .map(|arg| format!("\"{}\"", arg.replace('"', "\\\"")))
                        .collect::<Vec<_>>()
                        .join(", ");
                    out.push_str(&format!("args = [{args}]\n"));
                }
                if !server.env.is_empty() {
                    let pairs = server
                        .env
                        .iter()
                        .map(|(key, value)| format!("\"{key}\" = \"{value}\""))
                        .collect::<Vec<_>>()
                        .join(", ");
                    out.push_str(&format!("env = {{ {pairs} }}\n"));
                }
            }
            Transport::Http | Transport::Sse => {
                out.push_str(&format!(
                    "url = \"{}\"\n",
                    server.url.clone().unwrap_or_default()
                ));
                if !server.headers.is_empty() {
                    let pairs = server
                        .headers
                        .iter()
                        .map(|(key, value)| format!("\"{key}\" = \"{value}\""))
                        .collect::<Vec<_>>()
                        .join(", ");
                    out.push_str(&format!("http_headers = {{ {pairs} }}\n"));
                }
            }
        }
    }
    out
}

/// Whether `codex login status` says the binary is logged in.
pub fn login_status() -> Result<LoginStatus> {
    let out = Command::new("codex")
        .args(["login", "status"])
        .output()
        .with_context(|| "running codex login status")?;
    let text = String::from_utf8_lossy(&out.stdout);
    let combined = format!(
        "{}{}",
        text.trim(),
        String::from_utf8_lossy(&out.stderr).trim()
    );
    if combined.contains("Not logged in") {
        return Ok(LoginStatus::NotLoggedIn);
    }
    if out.status.success() {
        return Ok(LoginStatus::LoggedIn);
    }
    bail!(
        "codex login status exited {}: {}",
        out.status,
        combined.trim()
    );
}

/// What `codex login status` answered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoginStatus {
    LoggedIn,
    NotLoggedIn,
}

/// Hydra's remediation when auth pre-flight fails.
pub fn auth_remediation() -> &'static str {
    "codex is not logged in. Run `codex login` on a ChatGPT plan, then launch again."
}

/// Policies this build refuses for codex before spawn, per build ticket 48 item 4.
pub fn unsupported_policy(policy: &str) -> Option<String> {
    if policy == "read-only" || policy == "judge" {
        Some(format!(
            "codex has no read tool and every read is a Bash call, so policy {policy} cannot be \
             routed until build ticket 48 item 4 decides whether those policies grant a bounded \
             shell read on codex only or codex stays on standard and permissive"
        ))
    } else {
        None
    }
}

/// What codex says it resolved: contract 5.3's two `resolved` fields.
///
/// **No free probe exists on this provider yet.** Ticket 58 measured hooks and
/// the stream without logging in, and item 5 still needs a logged-in binary.
/// Both halves stay `None` until something free is measured.
pub fn resolve(_worktree: &Path, _mcp: crate::mcp::Probe<'_>) -> Result<Resolved> {
    eprintln!(
        "hydra: codex offers no free route to its resolved skill or MCP set in this build, so \
         skills.resolved and mcp.resolved are null on this run"
    );
    Ok(Resolved {
        skills: None,
        mcp: None,
        failed_mcp: Vec::new(),
    })
}

/// Reads the model and the session id off one JSONL line.
///
/// MEASURE 48.5: on a logged-in recorded run, confirm which `item.*` and
/// `turn.*` keys carry `session_id`, `thread_id` and `model`.
pub fn observe(raw: &str) -> Observed {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(raw) else {
        return Observed::default();
    };
    let session_id = value["session_id"]
        .as_str()
        .or_else(|| value["thread_id"].as_str())
        .or_else(|| value["item"]["session_id"].as_str())
        .or_else(|| value["turn"]["session_id"].as_str())
        .filter(|id| !id.is_empty())
        .map(str::to_string);
    let model = value["model"]
        .as_str()
        .or_else(|| value["turn"]["model"].as_str())
        .or_else(|| value["item"]["model"].as_str())
        .map(str::to_string);
    Observed { model, session_id }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_recorded_argv_names_the_worktree_and_the_belt_flags() {
        let worktree = Path::new("/tmp/wt");
        let argv = argv(&Launch {
            task: "do the thing",
            model: Some("gpt-5.4"),
            worktree,
            gate_command: "/tmp/spawn/r1/gate abc claude",
        });
        assert_eq!(argv[0], "exec");
        assert!(argv.contains(&"--json".to_string()));
        let sandbox = argv.iter().position(|arg| arg == "--sandbox").unwrap();
        assert_eq!(argv[sandbox + 1], "workspace-write");
        let approval = argv
            .iter()
            .position(|arg| arg == "--ask-for-approval")
            .unwrap();
        assert_eq!(argv[approval + 1], "never");
        assert!(
            argv.iter()
                .any(|arg| arg == "--dangerously-bypass-hook-trust")
        );
        let cd = argv.iter().position(|arg| arg == "-C").unwrap();
        assert_eq!(argv[cd + 1], "/tmp/wt");
        assert_eq!(argv.last().unwrap(), "do the thing");
    }

    #[test]
    fn the_hook_config_matches_the_hooks_json_shape_and_names_claudes_dialect() {
        let config = hooks_config(Path::new("/tmp/spawn/r1/gate"), "abc123");
        let group = &config["hooks"]["PreToolUse"][0];
        assert_eq!(group["matcher"], "*");
        let command = group["hooks"][0]["command"].as_str().unwrap();
        assert!(command.contains("/tmp/spawn/r1/gate"), "{command}");
        assert!(command.ends_with(" claude"), "{command}");
    }

    #[test]
    fn the_command_line_override_points_at_the_same_gate_command() {
        let command = "/usr/bin/python3 -c '...' abc claude";
        let overrides = config_overrides(command);
        assert_eq!(overrides.len(), 1);
        assert!(
            overrides[0].starts_with("hooks.PreToolUse="),
            "{}",
            overrides[0]
        );
        assert!(overrides[0].contains(command), "{}", overrides[0]);
    }

    #[test]
    fn a_stdio_server_and_a_remote_one_serialize_into_config_toml() {
        use crate::mcp::{Server, Transport};
        let stdio = Server {
            name: "probe".into(),
            transport: Transport::Stdio,
            admit: vec!["standard".into()],
            command: Some("/usr/bin/python3".into()),
            args: vec!["-u".into()],
            env: Default::default(),
            url: None,
            headers: Default::default(),
        };
        let remote = Server {
            name: "remote".into(),
            transport: Transport::Http,
            admit: vec!["standard".into()],
            command: None,
            args: Vec::new(),
            env: Default::default(),
            url: Some("https://example.invalid/mcp".into()),
            headers: Default::default(),
        };
        let body = mcp_config(&[&stdio, &remote]);
        assert!(body.contains("[mcp_servers.probe]"));
        assert!(body.contains("command = \"/usr/bin/python3\""));
        assert!(body.contains("args = [\"-u\"]"));
        assert!(body.contains("[mcp_servers.remote]"));
        assert!(body.contains("url = \"https://example.invalid/mcp\""));
    }

    #[test]
    fn read_only_and_judge_are_refused_with_a_message_that_names_item_4() {
        for policy in ["read-only", "judge"] {
            let message = unsupported_policy(policy).unwrap();
            assert!(message.contains("build ticket 48 item 4"), "{message}");
            assert!(message.contains("Bash"), "{message}");
        }
        assert!(unsupported_policy("standard").is_none());
    }

    #[test]
    fn the_session_id_is_read_off_the_frame_when_it_is_there() {
        let line = serde_json::json!({
            "type": "turn.completed",
            "thread_id": "thr_123",
            "model": "gpt-5.4",
        })
        .to_string();
        let observed = observe(&line);
        assert_eq!(observed.session_id.as_deref(), Some("thr_123"));
        assert_eq!(observed.model.as_deref(), Some("gpt-5.4"));
    }
}
