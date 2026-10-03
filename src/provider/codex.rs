//! `codex`, measured on 0.154.0 on 2026-09-12 in map ticket 58.
//!
//! The command line is harness spec 6.7's `codex` block as far as this build
//! implements it. A recorded run is `codex exec --json` with belt flags beside
//! the gate, the worktree named as the session root, and the hook handed over
//! on the command line as a `-c hooks.PreToolUse=[...]` override that names a
//! one-line command: `/usr/bin/python3 <spawn>/hook.py claude`.
//!
//! **Why the command line and a file, measured 2026-09-21 on 0.154.0.** A
//! `<repo>/.codex/hooks.json` fires in a plain checkout and is never read
//! inside a git worktree, from the worktree or from its main checkout (probes
//! p1 to p8 in the ticket 48 outcome), and every Hydra run is a worktree. The
//! `-c` override is read in a worktree, but its value is TOML and a TOML basic
//! string cannot hold the newlines the shared hook command carries: run
//! 01a0c222-4305 died at config load, exit 1. So the Python goes to
//! `spawn/<run-id>/hook.py`, beside the gate it checks, and the override names
//! the file. Probe p10 ran that shape in a worktree and the gate's deny held.
//! The `.codex/hooks.json` in the worktree is still written, because a launch
//! with no checkout runs in a plain directory where the file is what fires.
//!
//! **Hook trust is per-hash and a per-spawn gate never matches**, so
//! `--dangerously-bypass-hook-trust` rides every launch Hydra makes. The
//! record has to say so.
//!
//! **`--sandbox workspace-write` and `-c approval_policy="never"` are belt, not
//! the gate.** They narrow what the provider will do on its own; Hydra's
//! compiled hook is still what refuses under policy. Measured 2026-09-21 on
//! 0.154.0: `exec` has no `--ask-for-approval` flag, that one lives on the
//! interactive top-level command, and the first two launches died on it with
//! exit 2 (runs 01a0c220-909f and 01a0c220-ce0e).
//!
//! **There is no read tool.** Every read is a `Bash` call, which is why
//! `read-only` is refused at pre-flight until build ticket 48 item 4 is decided.
//! `judge` is routable since build ticket 61: a judge never reads a file and the
//! diff arrives in the prompt.
//!
//! **A hook that dies without answering fails open**, measured 2026-09-21,
//! probe p3 in build ticket 48's outcome: a bare exit 2 with no JSON let the
//! call run. The sha check in `hook.py` answers a tamper with a deny in
//! `claude`'s dialect for that reason.

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
    /// The run's checkout. Skills and MCP config load from here.
    pub worktree: &'a Path,
    /// The hook file [`hook_file`] names, for the `-c` override.
    pub hook_file: &'a Path,
}

/// Where the hook's Python lives: beside the gate it checks, in the spawn
/// directory every policy denies writes to.
pub fn hook_file(layout: &crate::paths::Layout, run_id: &str) -> PathBuf {
    layout.spawn(run_id).join("hook.py")
}

/// 6.7's `codex` line as far as ticket 48 implements it.
pub fn argv(launch: &Launch<'_>) -> Vec<String> {
    let mut argv = vec![
        "exec".to_string(),
        "--json".to_string(),
        "-C".to_string(),
        launch.worktree.display().to_string(),
        // Codex's own sandbox is a seatbelt too, and a second profile cannot be
        // applied inside Hydra's unless it is identical, measured 2026-09-21:
        // under `workspace-write` every command died with `sandbox_apply:
        // Operation not permitted`, the allowed ones included. Where Hydra wraps
        // the launch, the kernel layer is Hydra's and Codex runs without its own.
        "--sandbox".to_string(),
        if crate::sandbox::wraps_launches() {
            "danger-full-access".to_string()
        } else {
            "workspace-write".to_string()
        },
        "-c".to_string(),
        "approval_policy=\"never\"".to_string(),
        "--dangerously-bypass-hook-trust".to_string(),
    ];
    for config_override in config_overrides(launch.hook_file) {
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
///
/// Takes `--json`, `-m` and `-c` and neither `-C` nor `--sandbox`. The hook
/// override names the per-spawn `hook.py`, as on a fresh `exec`. Hydra's gate
/// still rides the command line; there is no worktree pin on resume.
///
/// **The sandbox mode goes as `-c sandbox_mode=...`**, because `resume` takes no
/// `--sandbox`, and without it Codex applied its own seatbelt inside Hydra's and
/// every command died with `sandbox_apply: Operation not permitted`, measured
/// 2026-09-23 on run 01a0cfda-7012, the first `codex` resume under the seatbelt.
pub fn resume_args(session_id: &str, hook_file: &Path) -> Vec<String> {
    let sandbox_mode = if crate::sandbox::wraps_launches() {
        "danger-full-access"
    } else {
        "workspace-write"
    };
    let mut argv = vec![
        "exec".to_string(),
        "resume".to_string(),
        session_id.to_string(),
        "--json".to_string(),
        "-c".to_string(),
        format!("sandbox_mode=\"{sandbox_mode}\""),
        "-c".to_string(),
        "approval_policy=\"never\"".to_string(),
        "--dangerously-bypass-hook-trust".to_string(),
    ];
    for config_override in config_overrides(hook_file) {
        argv.push("-c".to_string());
        argv.push(config_override);
    }
    argv
}

/// The interactive argv, harness spec 3's second mode, build ticket 255.
///
/// `codex` with no subcommand is the TUI, and `codex --help` on 0.159.2 says
/// options with no subcommand go to it. No `-C`: the launch sets the child's
/// cwd to the caller's. No sandbox or approval flags either, because an
/// ungated interactive run is the human's own session and keeps whatever the
/// human's `~/.codex/config.toml` says. The task, when there is one, is the
/// positional prompt the TUI opens with.
pub fn interactive_argv(task: Option<&str>, model: Option<&str>) -> Vec<String> {
    let mut argv = Vec::new();
    if let Some(model) = model {
        argv.push("-m".to_string());
        argv.push(model.to_string());
    }
    if let Some(task) = task {
        argv.push(task.to_string());
    }
    argv
}

// What an interactive `codex` leaves on disk, measured 2026-10-03 on files
// under `~/.codex/sessions/` written by 0.154.0 and 0.159.2. Build ticket 255.
//
// The path is `~/.codex/sessions/YYYY/MM/DD/rollout-<local time>-<id>.jsonl`.
// The date directories and the time in the name are local time; the `<id>` is
// the session's own UUIDv7. `codex` takes no flag that names a session id up
// front (`codex --help` on 0.159.2 has none), so Hydra cannot predict the path
// the way it does `claude`'s, and the file is not created when the TUI starts:
// on the one interactive file measured, the first line was written 42 seconds
// after the session's own timestamp, at the first prompt.
//
// Every line is `{"timestamp", "ordinal", "type", "payload"}`. The types:
//
// - `session_meta`, line 0 and only there. `payload.id` and
//   `payload.session_id` are the session id, `payload.cwd` the directory it ran
//   in, `payload.timestamp` (UTC) when the session began. `payload.originator`
//   is `codex-tui` with `payload.source` `cli` for the TUI, and `codex_exec`
//   with `exec` for a recorded run's `codex exec`.
// - `response_item`, the conversation. `payload.type` is `message` with
//   `role` `user`, `assistant` or `developer` and `content` a list of
//   `{type: input_text | output_text, text}`; `reasoning` with a `summary`
//   list (empty on every file measured) and `encrypted_content`;
//   `custom_tool_call` with `call_id`, `name` (always `exec` on 0.159.2) and
//   `input`; `custom_tool_call_output` with `call_id` and `output`, a string
//   or a list of `{type: input_text, text}`; and once `function_call` and
//   `function_call_output`, the same pair with `arguments` in place of
//   `input`.
// - `event_msg`, the TUI's own events. `payload.type` `token_count` carries
//   `info.total_token_usage`, the session's running total, or `info: null`
//   when the turn never reached the model. `item_completed`, `task_started`,
//   `task_complete`, `thread_settings_applied` and `compacted` repeat or
//   frame what `response_item` already carries.
// - `token_usage_record`, 0.159.2 and not 0.154.0: `payload.usage` for one
//   response and `payload.thread_token_usage`, the running total again.
// - `turn_context` and `world_state`, the settings each turn ran under.
//
// Every usage object names `input_tokens`, `cached_input_tokens`,
// `cache_write_input_tokens`, `output_tokens`, `reasoning_output_tokens` and
// `total_tokens`, the names the `exec --json` stream's `turn.completed` uses.

/// Where `codex` keeps its session files under `home`.
pub fn sessions_dir(home: &Path) -> PathBuf {
    home.join(".codex/sessions")
}

/// The session file an interactive run wrote, and how sure the match is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FoundSession {
    pub path: PathBuf,
    pub session_id: String,
    /// TUI sessions in the same directory that began after the run and later
    /// than this one, which a human may have opened in another pane.
    pub later: usize,
}

/// The TUI session file `codex` wrote for a run that started at `since` in
/// `cwd`, build ticket 255.
///
/// **By the session's own start, not by id**, because `codex` takes no id to
/// predict one with. The candidates are the TUI sessions (`source: cli`) whose
/// `cwd` is the run's and whose `session_meta.payload.timestamp` is at or after
/// `since`, the process's start time: the TUI mints its session after its
/// process starts. Of those this takes the **earliest**, not the newest,
/// because the newest is the one most likely to be a second `codex` a human
/// opened in the same directory while this one ran, and a run's own session is
/// the first to begin after it did. `later` counts the rest so the record can
/// say the match was not the only one.
///
/// Only the date directories from the day before `since` to the day after now
/// are read, because the directories are named in local time and the run
/// knows only UTC.
pub fn find_interactive_session(
    home: &Path,
    cwd: &Path,
    since: chrono::DateTime<chrono::Utc>,
) -> Result<Option<FoundSession>> {
    let root = sessions_dir(home);
    let last = (chrono::Utc::now() + chrono::Duration::days(1)).date_naive();
    let mut day = (since - chrono::Duration::days(1)).date_naive();
    let mut found: Vec<(chrono::DateTime<chrono::Utc>, String, PathBuf)> = Vec::new();
    while day <= last {
        let dir = root.join(day.format("%Y/%m/%d").to_string());
        day = day.succ_opt().context("ran off the end of the calendar")?;
        let entries = match std::fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => continue,
            Err(err) => return Err(err).with_context(|| format!("reading {}", dir.display())),
        };
        for entry in entries {
            let path = entry?.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("jsonl") {
                continue;
            }
            let Some((began, id)) = tui_session_in(&path, cwd)? else {
                continue;
            };
            if began >= since {
                found.push((began, id, path));
            }
        }
    }
    found.sort();
    let later = found.len().saturating_sub(1);
    Ok(found
        .into_iter()
        .next()
        .map(|(_, session_id, path)| FoundSession {
            path,
            session_id,
            later,
        }))
}

/// When the TUI session in `path` began and its id, when line 0 says it is a
/// TUI session that ran in `cwd`. A file whose first line does not parse is
/// skipped rather than fatal: it is someone else's, mid-write.
fn tui_session_in(
    path: &Path,
    cwd: &Path,
) -> Result<Option<(chrono::DateTime<chrono::Utc>, String)>> {
    use std::io::BufRead;

    let file = std::fs::File::open(path).with_context(|| format!("opening {}", path.display()))?;
    let mut first = String::new();
    std::io::BufReader::new(file)
        .read_line(&mut first)
        .with_context(|| format!("reading {}", path.display()))?;
    let Ok(line) = serde_json::from_str::<serde_json::Value>(&first) else {
        return Ok(None);
    };
    let meta = &line["payload"];
    if line["type"].as_str() != Some("session_meta") || meta["source"].as_str() != Some("cli") {
        return Ok(None);
    }
    let Some(ran_in) = meta["cwd"].as_str().map(Path::new) else {
        return Ok(None);
    };
    let same_dir = ran_in == cwd
        || matches!(
            (ran_in.canonicalize(), cwd.canonicalize()),
            (Ok(a), Ok(b)) if a == b
        );
    if !same_dir {
        return Ok(None);
    }
    let began = meta["timestamp"]
        .as_str()
        .and_then(|t| chrono::DateTime::parse_from_rfc3339(t).ok())
        .map(|t| t.with_timezone(&chrono::Utc));
    let id = meta["id"].as_str().or_else(|| meta["session_id"].as_str());
    Ok(began.zip(id).map(|(began, id)| (began, id.to_string())))
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

/// The `-c` override that points the hook at the file, one line, in the
/// dialect the gate insists on. The value is TOML: an inline array of one
/// inline table, and the command is a basic string, so a backslash or a quote
/// in the path is escaped and a newline would be refused, which is why the
/// Python is in a file.
pub fn config_overrides(hook_file: &Path) -> Vec<String> {
    // Codex hands the command to a shell, so the path is quoted for one before
    // the TOML escaping below. Unquoted, a space in the data root split the
    // path, Python opened the wrong file, and the hook died without answering,
    // which on this provider is a call that runs (SECURITY.md).
    let command = format!(
        "/usr/bin/python3 {} {}",
        crate::policy::shell_quote(&hook_file.display().to_string()),
        Dialect::Claude.as_str()
    );
    let escaped = command.replace('\\', "\\\\").replace('"', "\\\"");
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
///
/// **Serialized by the `toml` crate, not by `format!`.** The values come from
/// `config/mcp.md`, which the human writes, and a quote, a backslash or a
/// newline in a command, an argument, an env value, a URL, a header or a server
/// name has to reach `codex` as the same string. The hand-built version escaped
/// quotes in `args` and nothing else. The third hiring panel found it, H43.
pub fn mcp_config(servers: &[&crate::mcp::Server]) -> Result<String> {
    use crate::mcp::Transport;
    use toml::{Table, Value};

    let strings = |map: &std::collections::BTreeMap<String, String>| -> Table {
        map.iter()
            .map(|(key, value)| (key.clone(), Value::String(value.clone())))
            .collect()
    };
    let mut table = Table::new();
    for server in servers {
        let mut entry = Table::new();
        match server.transport {
            Transport::Stdio => {
                entry.insert(
                    "command".into(),
                    Value::String(server.command.clone().unwrap_or_default()),
                );
                if !server.args.is_empty() {
                    entry.insert(
                        "args".into(),
                        Value::Array(server.args.iter().cloned().map(Value::String).collect()),
                    );
                }
                if !server.env.is_empty() {
                    entry.insert("env".into(), Value::Table(strings(&server.env)));
                }
            }
            Transport::Http | Transport::Sse => {
                entry.insert(
                    "url".into(),
                    Value::String(server.url.clone().unwrap_or_default()),
                );
                if !server.headers.is_empty() {
                    entry.insert(
                        "http_headers".into(),
                        Value::Table(strings(&server.headers)),
                    );
                }
            }
        }
        table.insert(server.name.clone(), Value::Table(entry));
    }
    let mut root = Table::new();
    if !table.is_empty() {
        root.insert("mcp_servers".into(), Value::Table(table));
    }
    toml::to_string(&root).context("serializing the codex MCP config")
}

/// Whether `codex login status` says the binary is logged in.
///
/// **The probe gets the run's allowlist, like every other probe.** It used to
/// inherit the launcher's environment whole, so a `CODEX_HOME` in the shell
/// made it answer about a login the run would never see, and the vendor binary
/// got `GEMINI_API_KEY` along the way. The third hiring panel found it, H46.
pub fn login_status() -> Result<LoginStatus> {
    login_status_of("codex")
}

fn login_status_of(binary: &str) -> Result<LoginStatus> {
    let env = crate::env::ChildEnv::build(crate::provider::Provider::Codex)?;
    let out = Command::new(binary)
        .args(["login", "status"])
        .env_clear()
        .envs(env.vars())
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
    if policy == "read-only" {
        Some(
            "codex has no read tool and every read is a Bash call, so policy read-only cannot be \
             routed until build ticket 48 item 4 decides whether that policy grants a bounded \
             shell read on codex only or codex stays on standard and permissive; judge is \
             routable since build ticket 61"
                .to_string(),
        )
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
/// Measured 2026-09-21 on 0.154.0, run 01a0c223-4432: the session id is
/// `thread_id` on a `thread.started` frame, and no frame names the model; the
/// hook payload does, under `model`, but the stream never sees it, so the
/// record's `model` stays null unless `--model` was passed.
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

    /// Build ticket 255: the TUI is `codex` with no subcommand, so no `exec`,
    /// no `--json` and none of a recorded run's belt flags.
    #[test]
    fn the_interactive_argv_is_the_tui_with_a_model_and_a_prompt() {
        assert!(interactive_argv(None, None).is_empty());
        assert_eq!(
            interactive_argv(Some("look around"), Some("gpt-6-sol")),
            ["-m", "gpt-6-sol", "look around"]
        );
    }

    #[test]
    fn the_login_probe_gets_the_allowlist_and_nothing_the_launcher_carries() {
        use std::os::unix::fs::PermissionsExt;

        // A stub `codex` that writes down the environment it was handed and
        // answers logged in, so the probe runs and no vendor binary does.
        let dir = std::env::temp_dir().join(format!("hydra-codex-login-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let dump = dir.join("env");
        let stub = dir.join("codex");
        std::fs::write(
            &stub,
            format!(
                "#!/bin/sh\n/usr/bin/env > '{}'\necho 'Logged in using ChatGPT'\n",
                dump.display()
            ),
        )
        .unwrap();
        std::fs::set_permissions(&stub, std::fs::Permissions::from_mode(0o755)).unwrap();

        let allowed: std::collections::BTreeSet<String> =
            crate::env::ChildEnv::build(crate::provider::Provider::Codex)
                .unwrap()
                .names()
                .into_iter()
                // What `/bin/sh` exports on its own, whatever it was handed.
                .chain(["PWD", "SHLVL", "_", "OLDPWD"].map(String::from))
                .collect();
        // Without a name outside the allowlist in this process, an inherited
        // environment and a cleared one look the same and the test proves nothing.
        // `cargo test` sets `CARGO_MANIFEST_DIR` and its neighbours.
        assert!(
            std::env::vars().any(|(name, _)| !allowed.contains(&name)),
            "the test process carries nothing outside the allowlist"
        );

        assert_eq!(
            login_status_of(stub.to_str().unwrap()).unwrap(),
            LoginStatus::LoggedIn
        );
        let seen = std::fs::read_to_string(&dump).unwrap();
        let leaked: Vec<&str> = seen
            .lines()
            .filter_map(|line| line.split_once('=').map(|(name, _)| name))
            .filter(|name| !allowed.contains(*name))
            .collect();
        let _ = std::fs::remove_dir_all(&dir);
        assert!(
            leaked.is_empty(),
            "the login probe inherited {leaked:?} from the launcher"
        );
    }

    #[test]
    fn the_recorded_argv_names_the_worktree_and_the_belt_flags() {
        let worktree = Path::new("/tmp/wt");
        let argv = argv(&Launch {
            task: "do the thing",
            model: Some("gpt-5.4"),
            worktree,
            hook_file: Path::new("/tmp/spawn/r1/hook.py"),
        });
        assert_eq!(argv[0], "exec");
        assert!(argv.contains(&"--json".to_string()));
        let sandbox = argv.iter().position(|arg| arg == "--sandbox").unwrap();
        let expected = if crate::sandbox::wraps_launches() {
            "danger-full-access"
        } else {
            "workspace-write"
        };
        assert_eq!(argv[sandbox + 1], expected);
        assert!(
            !argv.iter().any(|arg| arg == "--ask-for-approval"),
            "exec has no such flag on 0.154.0"
        );
        assert!(
            argv.iter().any(|arg| arg == "approval_policy=\"never\""),
            "{argv:?}"
        );
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
    fn the_override_names_the_hook_file_on_one_line_with_the_dialect() {
        let argv = argv(&Launch {
            task: "t",
            model: None,
            worktree: Path::new("/tmp/wt"),
            hook_file: Path::new("/tmp/spawn/r1/hook.py"),
        });
        let value = argv
            .iter()
            .find(|arg| arg.starts_with("hooks.PreToolUse=["))
            .expect("the hook override rides a -c of its own");
        let at = argv.iter().position(|arg| arg == value).unwrap();
        assert_eq!(argv[at - 1], "-c");
        assert!(
            value.contains("command=\"/usr/bin/python3 '/tmp/spawn/r1/hook.py' claude\""),
            "{value}"
        );
        assert!(
            !value.contains('\n'),
            "a TOML basic string cannot hold a newline"
        );
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
        let body = mcp_config(&[&stdio, &remote]).unwrap();
        assert!(body.contains("[mcp_servers.probe]"));
        assert!(body.contains("command = \"/usr/bin/python3\""));
        assert!(body.contains("args = [\"-u\"]"));
        assert!(body.contains("[mcp_servers.remote]"));
        assert!(body.contains("url = \"https://example.invalid/mcp\""));
    }

    #[test]
    fn quotes_backslashes_and_newlines_round_trip_through_the_mcp_fragment() {
        // Third panel, H43: every string the document can put in the fragment
        // parses back as the same string, the server name included.
        use crate::mcp::{Server, Transport};
        let hostile = "a \"quoted\" C:\\path\\ and\na newline, a ' and a ]";
        let pair = |key: &str| [(format!("{key} \"k\""), hostile.to_string())].into();
        let stdio = Server {
            name: "odd.name \"x\"".into(),
            transport: Transport::Stdio,
            admit: vec!["standard".into()],
            command: Some(format!("/bin/{hostile}")),
            args: vec![hostile.into(), "\\".into()],
            env: pair("ENV"),
            url: None,
            headers: Default::default(),
        };
        let remote = Server {
            name: "remote".into(),
            transport: Transport::Sse,
            admit: vec!["standard".into()],
            command: None,
            args: Vec::new(),
            env: Default::default(),
            url: Some(format!("https://example.invalid/{hostile}")),
            headers: pair("Header"),
        };
        let body = mcp_config(&[&stdio, &remote]).unwrap();
        let parsed: toml::Table =
            toml::from_str(&body).unwrap_or_else(|err| panic!("{err}\n{body}"));
        let servers = parsed["mcp_servers"].as_table().unwrap();
        assert_eq!(servers.len(), 2, "{body}");

        let odd = servers["odd.name \"x\""].as_table().unwrap();
        assert_eq!(odd["command"].as_str(), stdio.command.as_deref());
        let args: Vec<_> = odd["args"]
            .as_array()
            .unwrap()
            .iter()
            .map(|arg| arg.as_str().unwrap())
            .collect();
        assert_eq!(args, [hostile, "\\"]);
        assert_eq!(odd["env"]["ENV \"k\""].as_str(), Some(hostile));

        let remote_table = servers["remote"].as_table().unwrap();
        assert_eq!(remote_table["url"].as_str(), remote.url.as_deref());
        assert_eq!(
            remote_table["http_headers"]["Header \"k\""].as_str(),
            Some(hostile)
        );
        assert!(mcp_config(&[]).unwrap().trim().is_empty());
    }

    #[test]
    fn read_only_is_refused_with_a_message_that_names_item_4_and_judge_is_routable() {
        let message = unsupported_policy("read-only").unwrap();
        assert!(message.contains("build ticket 48 item 4"), "{message}");
        assert!(message.contains("Bash"), "{message}");
        assert!(message.contains("judge"), "{message}");
        assert!(message.contains("build ticket 61"), "{message}");
        assert!(unsupported_policy("judge").is_none());
        assert!(unsupported_policy("standard").is_none());
    }

    #[test]
    fn resume_argv_names_the_thread_and_the_belt_flags_without_a_worktree_pin() {
        let hook_file = Path::new("/tmp/spawn/r1/hook.py");
        let argv = resume_args("thr_123", hook_file);
        assert_eq!(argv[0..3], ["exec", "resume", "thr_123"]);
        assert!(argv.contains(&"--json".to_string()));
        assert!(!argv.iter().any(|arg| arg == "-C"));
        assert!(!argv.iter().any(|arg| arg == "--sandbox"));
        assert!(argv.iter().any(|arg| arg.starts_with("sandbox_mode=")));
        assert!(
            argv.iter()
                .any(|arg| arg == "--dangerously-bypass-hook-trust")
        );
        assert!(argv.iter().any(|arg| arg.starts_with("hooks.PreToolUse=[")));
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
