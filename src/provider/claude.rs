//! `claude`, measured on 2.1.263.
//!
//! The command line is harness spec 6.7's, in both of its policy shapes, with one
//! redirect dropped.
//!
//! **`--mcp-config` and `--strict-mcp-config` ride a non-`judge` recorded run**,
//! together, which is how 6.7 pairs them. The first names the file
//! [`crate::mcp::write_config`] wrote and the second is what makes the run's server
//! set an argument Hydra wrote rather than the state of five directories at spawn
//! time. Without the strict flag the record could report only what Hydra intended.
//!
//! **`--restricted` does not drop MCP on 2.1.263, and the design says it does.**
//! Measured 2026-09-06 with the free probe below: `--restricted --mcp-config <file>`
//! resolved the server in the file, and 2.1.263's own `--help` now says "add
//! --strict-mcp-config to skip MCP servers too". Ticket 23 measured the opposite on
//! 2.1.257 and 6.5 and 6.7 were written from it, so `judge` gets neither flag and, on
//! this version, keeps whatever the human configured. The gate refuses every
//! `mcp__` call under `judge`, so what is left is tool descriptions reaching the
//! model's context, which is ticket 27's residue arriving on the provider that was
//! supposed to be free of it. Nothing was changed here: 6.7 fixes the `judge` argv,
//! adding a flag to it is a design decision, and `IMPLEMENTATION.md` records it.
//!
//! **`< /dev/null` is dropped, because a pty is not a pipe.** 6.7 adds it so
//! `claude` does not wait three seconds for stdin and write `no stdin data received
//! in 3s` into the record. Measured 2026-09-05 on 2.1.261 over a pty Hydra owned:
//! no wait, no warning, first frame in well under a second. The redirect was a
//! finding about a pipe, and Hydra owns a pty because it has to be the parent to
//! tee anyway.

use std::path::Path;

use crate::policy::Dialect;
use crate::provider::{Observed, Resolved};

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
    /// The run's MCP config, `~/hydra/spawn/<run-id>/mcp.json`, or `None` on a
    /// launch shape that carries none. [`mcp_undeliverable`] is the one thing
    /// that answers `None` today.
    pub mcp: Option<&'a Path>,
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

    // 6.7: both flags or neither, on a non-`judge` recorded run. The strict flag
    // is what makes the set an argument Hydra wrote, which is what lets
    // `run_start.mcp.injected` claim it. A `judge` run gets no file to point at,
    // so the branch above and this one answer the same question twice on purpose.
    if let Some(mcp) = launch.mcp
        && !restricted
    {
        argv.push("--mcp-config".to_string());
        argv.push(mcp.display().to_string());
        argv.push("--strict-mcp-config".to_string());
    }

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

/// Why a launch of this shape carries no MCP config at all, when it carries none.
///
/// One shape on this provider: 6.7's `judge` argv has no `--mcp-config` on it, so
/// nothing Hydra writes can reach that run whatever the document admits.
/// [`crate::mcp::Plan::shortfall`] turns that into 6.1 item 6's refusal.
///
/// **The reason is the argv and no longer the flag.** 6.7 justifies the omission
/// with "`--restricted` drops MCP anyway", measured by ticket 23 on 2.1.257. That
/// is false on 2.1.263, measured 2026-09-06, and the module comment above has the
/// probe. The omission still holds because 6.7 writes the argv, which is why this
/// answers on the policy name rather than on what the flag does.
pub fn mcp_undeliverable(policy: &str) -> Option<&'static str> {
    (policy == RESTRICTED_POLICY).then_some(
        "harness spec 6.7 gives a judge run --restricted and no --mcp-config, so no \
         server Hydra writes can reach it",
    )
}

/// The `--mcp-config` file's contents, 6.5.
///
/// `claude`'s own shape: `mcpServers` keyed by name, a stdio body of `command`,
/// `args` and `env`, and a remote body of `type` plus `url`. The remote key is
/// where the three providers part company, so this is the one place the shape is
/// written down for this one.
///
/// **An empty object is a real answer.** With `--strict-mcp-config` beside it, an
/// empty `mcpServers` is what stops the human's own servers starting, so a run
/// that admits nothing still gets a file.
pub fn mcp_config(servers: &[&crate::mcp::Server]) -> serde_json::Value {
    use crate::mcp::Transport;

    let mut map = serde_json::Map::new();
    for server in servers {
        let mut body = serde_json::Map::new();
        match server.transport {
            Transport::Stdio => {
                body.insert(
                    "command".into(),
                    server.command.clone().unwrap_or_default().into(),
                );
                if !server.args.is_empty() {
                    body.insert("args".into(), server.args.clone().into());
                }
                if !server.env.is_empty() {
                    body.insert("env".into(), serde_json::json!(server.env));
                }
            }
            Transport::Http | Transport::Sse => {
                body.insert("type".into(), server.transport.as_str().into());
                body.insert("url".into(), server.url.clone().unwrap_or_default().into());
                if !server.headers.is_empty() {
                    body.insert("headers".into(), serde_json::json!(server.headers));
                }
            }
        }
        map.insert(server.name.clone(), serde_json::Value::Object(body));
    }
    serde_json::json!({ "mcpServers": serde_json::Value::Object(map) })
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

/// The model name the skill probe passes, which no catalog will ever hold.
///
/// Claude Code rejects an unknown model before it calls the API, so the probe is
/// free. That is a measured property of the binary rather than a promise: 2.1.263
/// prints `[claude-code:unrecognized_model]`, exits without a request, and still
/// writes the session transcript. A build that started resolving unknown names
/// would turn this into a paid call, which is why the name says what it is.
const PROBE_MODEL: &str = "hydra-skill-preflight-not-a-model";

/// What `claude` says it resolved, for contract 5.3's `skills.resolved` and
/// `mcp.resolved`.
///
/// **The record cannot claim what the model saw without this.** Claude Code fits
/// its whole skill listing into a character budget and renders the overflow as
/// bare names, so nine injected links moved bare entries from 8 to 14 and three of
/// the six newly stripped were the user's own plugin skills, with no warning.
/// A sha and a list of what Hydra linked do not say that; the provider's own
/// listing does.
///
/// **It is free and it is not instant.** Measured 2026-09-05 on 2.1.263: 1.8s
/// wall clock, no API call, no tokens. Re-measured 2026-09-06 with both MCP flags:
/// 1.8s with an empty config and 1.9s with one stdio server in it.
///
/// **The MCP half comes off the tool names.** Measured 2026-09-06 on 2.1.263
/// against a stdio server written for it, `tests/fixtures/mcp-probe-server.py`:
/// the transcript's `deferred_tools_delta` attachment gained
/// `mcp__hydra-probe__ping` in `addedNames`, and the same attachment carries
/// `pendingMcpServers`, `needsAuthMcpServers` and `failedMcpServers`, the last as
/// objects with a `name` and an error string. **A server that connects and offers
/// no tool is invisible to this reading**, which is the one thing it cannot see.
///
/// **The probe carries the run's own MCP flags.** Without them it would resolve
/// the human's servers and the record would claim them for a run that will not
/// have them. `--strict-mcp-config` on the probe is chosen for that reason rather
/// than reached for as a speed lever, and it means the probe starts every server
/// the run will start, once more per launch.
///
/// Three details are load-bearing. `--session-id` names the transcript, so this
/// reads a file it can predict rather than the newest one in a directory a
/// concurrent session may also be writing to. Stdin is `/dev/null`, or the binary
/// waits three seconds for input that is not coming. And the environment is the
/// child's own allowlist, because a wrong `HOME` moves both the user-scope skill
/// root and the transcript, and the probe has to be looking at the same machine
/// the run will.
///
/// **The probe runs the human's own `UserPromptSubmit` hooks**, because it is an
/// ordinary `claude` invocation with the user's settings. That is the same
/// exposure a recorded run already has and is worth knowing about here, where
/// nothing was asked of a model.
pub fn resolve(worktree: &Path, mcp: crate::mcp::Probe<'_>) -> anyhow::Result<Resolved> {
    use anyhow::Context;

    let env = crate::env::ChildEnv::build(crate::provider::Provider::Claude)?;
    let home = crate::env::real_home()?;
    let session = uuid::Uuid::now_v7().to_string();

    let mut args: Vec<String> = ["-p", "x", "--model", PROBE_MODEL, "--session-id", &session]
        .iter()
        .map(|arg| (*arg).to_string())
        .collect();
    if let crate::mcp::Probe::Strict(config) = mcp {
        args.push("--mcp-config".to_string());
        args.push(config.display().to_string());
        args.push("--strict-mcp-config".to_string());
    }

    let out = std::process::Command::new("claude")
        .args(&args)
        .current_dir(worktree)
        .env_clear()
        .envs(env.vars())
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .context("running the skill listing probe")?;

    let transcript = home
        .join(".claude/projects")
        .join(project_slug(worktree))
        .join(format!("{session}.jsonl"));

    // **The exit status says nothing and the output says everything.** The probe
    // is a run the binary is meant to refuse, so it exits however it likes and a
    // status check would only encode today's spelling. What matters is whether the
    // transcript arrived, and when it did not, what the binary said instead: a
    // lapsed login and a renamed transcript directory both land here and read
    // identically without it.
    let text = std::fs::read_to_string(&transcript).with_context(|| {
        format!(
            "the skill probe left no transcript at {}. It exited {} and said: {}",
            transcript.display(),
            out.status,
            said(&out).unwrap_or_else(|| "nothing".to_string())
        )
    })?;

    let tools = tool_listing(&text);
    Ok(Resolved {
        skills: skill_listing(&text),
        // A probe that was not asked the run's own question answers nothing about
        // servers, whatever its own transcript says.
        mcp: match mcp {
            crate::mcp::Probe::Strict(_) => {
                tools.as_ref().map(|listing| mcp_servers(&listing.added))
            }
            crate::mcp::Probe::Unanswerable => None,
        },
        failed_mcp: tools.map(|listing| listing.failed).unwrap_or_default(),
    })
}

/// The first line the probe printed on either stream, for an error message.
fn said(out: &std::process::Output) -> Option<String> {
    [&out.stderr, &out.stdout]
        .into_iter()
        .flat_map(|stream| {
            String::from_utf8_lossy(stream)
                .to_string()
                .lines()
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .find(|line| !line.trim().is_empty())
}

/// Claude Code's name for a project directory under `~/.claude/projects/`.
///
/// Measured on 2.1.263 against three paths, one of them holding a dot and an
/// underscore: every character that is not a letter, a digit or a hyphen becomes a
/// hyphen, and the leading `/` becomes the leading hyphen that way too.
///
/// A wrong answer here is a missing file rather than a wrong listing, because
/// [`resolved_skills`] names the transcript with `--session-id` and reads that
/// exact path.
fn project_slug(path: &Path) -> String {
    path.to_string_lossy()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' {
                c
            } else {
                '-'
            }
        })
        .collect()
}

/// What the transcript's `deferred_tools_delta` attachment says about tools and
/// about the servers behind them.
struct ToolListing {
    /// `addedNames`, which holds one entry per MCP tool as
    /// `mcp__<server>__<tool>`.
    added: Vec<String>,
    /// `failedMcpServers`, rendered as `name: reason`.
    failed: Vec<String>,
}

/// The first `deferred_tools_delta` attachment in a transcript.
///
/// **The first, because the probe writes exactly one.** The name says delta and a
/// long session would carry several; the probe never reaches a second turn, so
/// merging them would be code no run of this build can exercise. A session that
/// did carry more would need the removals applied too, and that belongs to
/// whoever first has one.
fn tool_listing(transcript: &str) -> Option<ToolListing> {
    for line in transcript.lines() {
        let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        if value["attachment"]["type"].as_str() != Some("deferred_tools_delta") {
            continue;
        }
        let added = value["attachment"]["addedNames"]
            .as_array()
            .map(|names| {
                names
                    .iter()
                    .filter_map(|name| name.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default();
        let failed = value["attachment"]["failedMcpServers"]
            .as_array()
            .map(|servers| {
                servers
                    .iter()
                    .map(|server| {
                        let name = server["name"].as_str().unwrap_or("?");
                        let why = server["error"]
                            .as_str()
                            .or_else(|| server["errorCode"].as_str())
                            .unwrap_or("no reason given");
                        format!("{name}: {why}")
                    })
                    .collect()
            })
            .unwrap_or_default();
        return Some(ToolListing { added, failed });
    }
    None
}

/// The servers behind a list of tool names, sorted and deduplicated.
///
/// `claude` names an MCP tool `mcp__<server>__<tool>`, so the server is what sits
/// between the prefix and the next `__`. That split is exact because
/// [`crate::mcp::Document`] refuses a server name carrying `__`, which is the
/// rule that keeps this from being a guess.
fn mcp_servers(tools: &[String]) -> Vec<String> {
    let mut names: Vec<String> = tools
        .iter()
        .filter_map(|tool| tool.strip_prefix("mcp__"))
        .filter_map(|rest| rest.split("__").next())
        .filter(|name| !name.is_empty())
        .map(str::to_string)
        .collect();
    names.sort();
    names.dedup();
    names
}

/// The `names` array off the transcript's `skill_listing` attachment.
///
/// **Names and never descriptions.** On the probe's path every
/// filesystem-discovered skill renders as a bare name, so the listing answers
/// identity and precedence, which is what the collision check asks, and cannot
/// answer whether a description survived. A real run renders descriptions for all
/// of them and is where that question goes if anyone ever asks it.
fn skill_listing(transcript: &str) -> Option<Vec<String>> {
    for line in transcript.lines() {
        let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        if value["attachment"]["type"].as_str() != Some("skill_listing") {
            continue;
        }
        return Some(
            value["attachment"]["names"]
                .as_array()?
                .iter()
                .filter_map(|name| name.as_str().map(str::to_string))
                .collect(),
        );
    }
    None
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

    const MCP: &str = "/tmp/spawn/r1/mcp.json";

    fn launch<'a>(policy: &'a str, settings: &'a Path) -> Launch<'a> {
        Launch {
            task: "hi",
            model: None,
            settings,
            policy,
            mcp: Some(Path::new(MCP)),
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
    fn the_two_mcp_flags_ride_together() {
        // 6.7 pairs them on a non-`judge` recorded run, and the pairing is the
        // point: `--mcp-config` alone would add Hydra's servers to whatever the
        // human has, and the record could then only report what Hydra intended.
        let argv = argv(&launch("standard", Path::new("/tmp/s.json")));
        let at = argv.iter().position(|a| a == "--mcp-config").unwrap();
        assert_eq!(argv[at + 1], MCP);
        assert!(argv.iter().any(|a| a == "--strict-mcp-config"));
    }

    #[test]
    fn judge_carries_neither_mcp_flag_even_when_a_file_exists() {
        // 6.7's `judge` shape. `mcp::plan` never hands this policy a file, and
        // the argv refuses one anyway, because two answers to one question is
        // how a later edit to either half goes unnoticed.
        let argv = argv(&launch("judge", Path::new("/tmp/s.json")));
        assert!(!argv.iter().any(|a| a == "--mcp-config"));
        assert!(!argv.iter().any(|a| a == "--strict-mcp-config"));
    }

    #[test]
    fn a_launch_with_no_mcp_file_passes_neither_flag() {
        let argv = argv(&Launch {
            task: "hi",
            model: None,
            settings: Path::new("/tmp/s.json"),
            policy: "standard",
            mcp: None,
        });
        assert!(!argv.iter().any(|a| a == "--mcp-config"));
        assert!(
            !argv.iter().any(|a| a == "--strict-mcp-config"),
            "the strict flag without a config would strip the human's servers and \
             add none, which is not a shape 6.7 asks for"
        );
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
            mcp: Some(Path::new(MCP)),
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

    #[test]
    fn the_project_slug_flattens_everything_that_is_not_a_letter_or_a_digit() {
        // Measured on 2.1.263 by running the probe in a directory called
        // `slug.probe_1` and reading the name it created under
        // `~/.claude/projects/`.
        assert_eq!(
            project_slug(Path::new("/tmp/scratch/slug.probe_1")),
            "-tmp-scratch-slug-probe-1"
        );
        assert_eq!(
            project_slug(Path::new(
                "/Users/USER/hydra/worktrees/01a07484-1667-7500-b299-2889144b84c1"
            )),
            "-Users-USER-hydra-worktrees-01a07484-1667-7500-b299-2889144b84c1"
        );
    }

    #[test]
    fn the_listing_comes_off_the_attachment_line() {
        let transcript = concat!(
            "{\"type\":\"user\"}\n",
            "{\"type\":\"attachment\",\"attachment\":{\"type\":\"agent_listing_delta\"}}\n",
            "{\"type\":\"attachment\",\"attachment\":{\"type\":\"skill_listing\",\
             \"skillCount\":2,\"names\":[\"alpha\",\"mattpocock-skills:tdd\"]}}\n",
            "not json at all\n",
        );
        assert_eq!(
            skill_listing(transcript).unwrap(),
            vec!["alpha".to_string(), "mattpocock-skills:tdd".to_string()]
        );
    }

    fn server(name: &str, transport: crate::mcp::Transport) -> crate::mcp::Server {
        crate::mcp::Server {
            name: name.to_string(),
            transport,
            admit: vec!["standard".to_string()],
            command: None,
            args: Vec::new(),
            env: Default::default(),
            url: None,
            headers: Default::default(),
        }
    }

    #[test]
    fn a_stdio_server_generates_the_body_all_three_providers_share() {
        let mut fs = server("filesystem", crate::mcp::Transport::Stdio);
        fs.command = Some("npx".to_string());
        fs.args = vec!["-y".to_string(), "server-filesystem".to_string()];
        fs.env.insert("TOKEN".to_string(), "x".to_string());

        let config = mcp_config(&[&fs]);
        assert_eq!(config["mcpServers"]["filesystem"]["command"], "npx");
        assert_eq!(
            config["mcpServers"]["filesystem"]["args"][1],
            "server-filesystem"
        );
        assert_eq!(config["mcpServers"]["filesystem"]["env"]["TOKEN"], "x");
        assert!(
            config["mcpServers"]["filesystem"].get("type").is_none(),
            "a stdio server carries no type on this provider"
        );
    }

    #[test]
    fn a_remote_server_carries_the_key_this_provider_reads() {
        // 6.5: `type` plus `url` here, `url` alone on `cursor-agent`,
        // `serverUrl` on `agy`. This is the difference the canonical document
        // exists to absorb.
        let mut docs = server("docs", crate::mcp::Transport::Http);
        docs.url = Some("https://example.com/mcp".to_string());
        docs.headers
            .insert("Authorization".to_string(), "Bearer x".to_string());

        let config = mcp_config(&[&docs]);
        assert_eq!(config["mcpServers"]["docs"]["type"], "http");
        assert_eq!(
            config["mcpServers"]["docs"]["url"],
            "https://example.com/mcp"
        );
        assert_eq!(
            config["mcpServers"]["docs"]["headers"]["Authorization"],
            "Bearer x"
        );
        assert!(config["mcpServers"]["docs"].get("command").is_none());
    }

    #[test]
    fn a_run_that_admits_nothing_still_gets_a_file() {
        // With `--strict-mcp-config` beside it, an empty object is what stops the
        // human's own servers starting. It is a claim rather than an omission.
        let config = mcp_config(&[]);
        assert!(config["mcpServers"].is_object());
        assert_eq!(config["mcpServers"].as_object().unwrap().len(), 0);
    }

    #[test]
    fn judge_is_the_one_shape_that_cannot_be_delivered_a_server() {
        assert!(mcp_undeliverable("judge").is_some());
        for policy in ["standard", "read-only", "curate", "permissive"] {
            assert!(mcp_undeliverable(policy).is_none(), "{policy}");
        }
    }

    #[test]
    fn the_resolved_servers_come_off_the_tool_names() {
        // Measured on 2.1.263, 2026-09-06: the probe's transcript carried
        // `mcp__hydra-probe__ping` in `addedNames` for a stdio server passed on
        // `--mcp-config`, and named a server that would not start in
        // `failedMcpServers`.
        let transcript = concat!(
            "{\"type\":\"attachment\",\"attachment\":{\"type\":\"deferred_tools_delta\",",
            "\"addedNames\":[\"WebSearch\",\"mcp__hydra-probe__ping\",",
            "\"mcp__hydra-probe__pong\",\"mcp__docs__search_the_docs\"],",
            "\"failedMcpServers\":[{\"name\":\"broken\",\"errorCode\":\"CONNECTION_CLOSED\",",
            "\"error\":\"Connection closed\"}]}}\n",
        );
        let listing = tool_listing(transcript).unwrap();
        assert_eq!(
            mcp_servers(&listing.added),
            vec!["docs".to_string(), "hydra-probe".to_string()],
            "one entry per server, not per tool"
        );
        assert_eq!(
            listing.failed,
            vec!["broken: Connection closed".to_string()]
        );
    }

    #[test]
    fn a_transcript_with_no_tool_listing_answers_nothing() {
        // The same rule `skills.resolved` follows: absent is unanswered, and an
        // empty array would claim the run got no servers.
        assert!(tool_listing("{\"type\":\"user\"}\n").is_none());
    }

    #[test]
    fn a_transcript_with_no_listing_answers_nothing_rather_than_an_empty_list() {
        // Contract 5.3: a reader must handle the absence rather than treating it
        // as a listing with nothing in it.
        assert!(skill_listing("{\"type\":\"user\"}\n").is_none());
    }
}
