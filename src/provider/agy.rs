//! `agy`, measured on 1.1.26 on 2026-09-06 and confirmed on 1.1.27 the same evening,
//! which is this provider self-updating in the middle of the ticket that routed it.
//! Nothing below changed across the move.
//!
//! The command line is harness spec 6.7's `agy` block. Three things ride into
//! this provider on one flag: the skill links at `.agents/skills/`, the hook
//! config at `.agents/hooks.json` and the MCP config at
//! `.agents/mcp_config.json`, none of which loads unless the worktree arrives as
//! `--add-dir`. Measured again on 1.1.26: the same `hooks.json`, read from cwd,
//! listed as `hooks: []`, and listed in full the moment the directory was passed
//! on the flag.
//!
//! **`-p=<prompt>` with the equals sign.** A bare `-p` swallows the next argument
//! as the prompt and silently drops the real one.
//!
//! **`--dangerously-skip-permissions` is what makes the gate the only thing
//! refusing.** Without it the provider auto-denies acting tools mid-run and
//! cannot be asked, because its stream-json input accepts one message shape and
//! answers a `permission_response` with `warning: ignoring unsupported stream
//! input message event`. So the permission mode is a launch decision. Measured
//! without the flag on 1.1.26: `permission_mode: request-review` on the init
//! frame, a `write_to_file` refused with `is not a valid artifact path`, and a
//! `run_command` refused with `user denied permission to run command`, both
//! carrying `error.type: TOOL_ERROR`, both mentioning permissions, and meaning
//! two different things. That is contract 7.6's `unclassified` case in one run.
//! With the flag, on a recorded 1.1.27 run, the same field reads
//! `always-proceed` and the gate is what refuses.
//!
//! **Its hooks fail closed and there is no flag to change that**, which is the
//! opposite of `cursor-agent`'s converted-Claude default. A gate script that
//! crashes stops the run rather than opening it.
//!
//! **The gate mount is checked before the spawn, free.** `agy -p="/hooks"` is one
//! of the read-only slash commands that answer without starting an agent turn:
//! `num_turns: 0`, `total_tokens: 0`, measured. So the worst failure mode on this
//! provider, a hook config nothing loaded and no error anywhere, costs one process
//! to rule out. [`hooks_mounted`] is that check and [`crate::run`] refuses a
//! launch it fails.
//!
//! **The file format is the binary's own, and research 26 recorded the wrong one.**
//! That ticket read `/hooks` output and took it for the input shape. The embedded
//! `Lifecycle Hooks (hooks.json)` reference in the 1.1.26 binary says the file is a
//! JSON object keyed by hook name, each name carrying event arrays, and
//! `PreToolUse` entries wrapping their handlers in a `matcher` group. The output
//! shape has `name`, `enabled`, `source` and a flat `actions` list. Written in the
//! output shape, on 1.1.26, the file loads as nothing: `hooks: []`, no error, which
//! is exactly the silence this module's pre-flight exists for.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

use crate::mcp::Checkout;
use crate::provider::{Observed, Resolved};

/// The argv after the binary name.
pub struct Launch<'a> {
    /// The preamble and the caller's task, already composed. Contract 5.1
    /// records this string verbatim.
    pub task: &'a str,
    pub model: Option<&'a str>,
    /// The run's checkout. Everything Hydra injects is read out of it and none of
    /// it loads without this flag, so a launch with no worktree is refused before
    /// a run id exists rather than launched blind ([`crate::run`]).
    pub worktree: &'a Path,
}

/// 6.7's `agy` line.
///
/// `-p=<task>` is one argument, not two. The prompt rides inside the flag because
/// a bare `-p` takes the next argv element as the prompt and drops whatever was
/// meant to be it, silently.
pub fn argv(launch: &Launch<'_>) -> Vec<String> {
    let mut argv = vec![
        format!("-p={}", launch.task),
        "--output-format".to_string(),
        "stream-json".to_string(),
        "--add-dir".to_string(),
        launch.worktree.display().to_string(),
        // 6.7: the gate is the only thing refusing, and a detached child has
        // nobody to answer a prompt to.
        "--dangerously-skip-permissions".to_string(),
    ];
    if let Some(model) = launch.model {
        argv.push("--model".to_string());
        argv.push(model.to_string());
    }
    argv
}

/// Harness spec 17: `--conversation <id>`, measured on 1.1.26.
pub fn resume_args(session_id: &str) -> Vec<String> {
    vec!["--conversation".to_string(), session_id.to_string()]
}

const HOOKS: &str = ".agents/hooks.json";
const MCP: &str = ".agents/mcp_config.json";

/// Where the hook config goes, 6.6's table: inside the worktree, a real file.
pub fn hooks_file(worktree: &Path) -> PathBuf {
    worktree.join(HOOKS)
}

/// Where the MCP config goes, 6.5's table.
pub fn mcp_file(worktree: &Path) -> PathBuf {
    worktree.join(MCP)
}

/// The `.gitignore` Hydra drops beside its two files in `.agents/`, and what it
/// holds.
///
/// **By name, for the reason Cursor's is by name.** The skill root under
/// `.agents/skills/` ignores itself with `*` (6.4), and a `*` one level up would
/// hide anything the child wrote in a `.agents/` the human's repo tracks: this
/// provider reads `rules/`, `plugins/` and `workflows/` out of the same directory,
/// and `hydra reap` commits the checkout. So the three names Hydra writes are
/// listed and nothing else is.
pub fn gitignore(worktree: &Path) -> (PathBuf, &'static str) {
    (
        worktree.join(".agents/.gitignore"),
        "/.gitignore\n/hooks.json\n/mcp_config.json\n",
    )
}

/// The path one of Hydra's two `.agents/` files will be written to, once the
/// checkout has been checked for the human's own copy.
///
/// [`super::claim`] holds the rule and the reasons; this names the two files it
/// applies to on this provider.
pub fn claim(
    worktree: Option<&Path>,
    what: &str,
    file: impl Fn(&Path) -> PathBuf,
) -> Result<PathBuf> {
    super::claim(worktree, "agy", what, file, gitignore)
}

/// The hook config, 6.6, in the shape the 1.1.26 binary's own reference documents.
///
/// One event, one handler, one script. `PreToolUse` is where every tool this
/// provider offers arrives, reads included, so unlike Cursor there is no second
/// event to register: 6.6 says `claude` and `agy` route reads through the tool
/// hook like everything else.
///
/// **No `failClosed` field, because there is none to write.** Failing closed is
/// this provider's only behaviour and the config schema has no switch for it,
/// measured by ticket 26 the hard way: a hook whose command exited 127 denied
/// every tool in the run.
///
/// **`matcher: "*"` is every tool**, and the gate's own table is what decides per
/// name. `timeout` is seconds and defaults to 30; 30 is left as it is, since the
/// gate is a local Python process and a run that has to wait longer than that for
/// it has something worse wrong.
pub fn hooks_config(gate_script: &Path, gate_sha: &str) -> serde_json::Value {
    let command = crate::policy::hook_command(gate_script, gate_sha, crate::policy::Dialect::Agy);
    serde_json::json!({
        "hydra-gate": {
            "enabled": true,
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

/// Why a launch of this shape carries no MCP config at all, when it carries none.
///
/// The same answer Cursor gives and for a narrower reason: this provider reads its
/// config out of the worktree too, so a run with no checkout of Hydra's own has
/// nowhere to put one. [`crate::run`] refuses such a launch before a run id
/// exists, because the hook config has the same problem and a provider Hydra
/// cannot gate is not launched, so this branch is the second line rather than the
/// first.
pub fn mcp_undeliverable(checkout: Checkout) -> Option<&'static str> {
    match checkout {
        Checkout::Hydra => None,
        Checkout::None => Some(
            "harness spec 6.5 puts an agy run's MCP config inside the worktree, and this \
             launch has no checkout Hydra created to write one into",
        ),
    }
}

/// The `.agents/mcp_config.json` contents, 6.5.
///
/// The stdio body is byte-identical to the other two. A remote server carries
/// `serverUrl`, which is this provider's own spelling of what `cursor-agent`
/// calls `url` and `claude` calls `type` plus `url`, and it is the reason the
/// canonical document is none of the three files.
///
/// **A server naming both a command and a url cannot reach here.** This provider
/// refuses that pair with `MCP server %q cannot have both command and serverUrl`,
/// and [`crate::mcp::Document`] refuses it at parse time with `agy` in the error,
/// so no document can ask for one.
///
/// **A relative path in `args` breaks under `--add-dir`, silently.** Research 23
/// measured the server process starting with the launch cwd rather than the
/// worktree. Nothing here can fix that; the document is where an absolute path
/// belongs.
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
                body.insert(
                    "serverUrl".into(),
                    server.url.clone().unwrap_or_default().into(),
                );
                if !server.headers.is_empty() {
                    body.insert("headers".into(), serde_json::json!(server.headers));
                }
            }
        }
        map.insert(server.name.clone(), serde_json::Value::Object(body));
    }
    serde_json::json!({ "mcpServers": serde_json::Value::Object(map) })
}

/// How long a free probe waits, with the provider's own flag.
///
/// **This bounds the print-mode wait and not the process.** Both probes sit on
/// the foreground launch path after the worktree and the branch exist, so an
/// `agy` that never answers would otherwise hang `hydra run` with a checkout on
/// disk and no run to `hydra logs`. Thirty seconds is far outside the measured
/// cost of a command that starts no agent turn, which came back in about a
/// second. A hang before print mode begins is still unbounded, the same gap
/// `cursor-agent`'s pre-flight has, and `IMPLEMENTATION.md` says so.
const PROBE_TIMEOUT: &str = "30s";

/// What one free read-only slash command answered.
struct Command {
    /// The `command.data` object, or `Value::Null` when the run did not carry one.
    data: serde_json::Value,
}

/// Runs one of the free read-only slash commands and returns its `command.data`.
///
/// Free is measured rather than assumed: `/hooks` and `/skills` both come back
/// with `num_turns: 0` and every count in `usage` at zero on 1.1.26. `/mcp` is
/// not one of them any more, which is a change since research 23 ran it for four
/// paid turns: on 1.1.26 it answers `/mcp is not available in print mode (it
/// opens the interactive MCP server view)`.
///
/// **The probe runs under the environment the run will get**, built from 6.2's
/// allowlist here the way [`super::claude::resolve`] builds its own. Inheriting
/// Hydra's instead would hand the probe the `GEMINI_API_KEY` on line 1 of
/// `~/.zshrc`, so it could authenticate by a route the run cannot and answer a
/// question the run never asked. It also carries `--add-dir` for the same
/// reason: a listing taken without the flag is a listing of a workspace this
/// provider was never told about.
fn slash(worktree: &Path, command: &str) -> Result<Command> {
    let env = crate::env::ChildEnv::build(crate::provider::Provider::Agy)?;
    let out = std::process::Command::new("agy")
        .arg("--add-dir")
        .arg(worktree)
        .arg(format!("-p=/{command}"))
        .arg("--output-format")
        .arg("json")
        .arg("--print-timeout")
        .arg(PROBE_TIMEOUT)
        .current_dir(worktree)
        .env_clear()
        .envs(env.vars())
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .with_context(|| format!("running agy -p=/{command}"))?;
    if !out.status.success() {
        bail!(
            "agy -p=/{command} exited {}: {}",
            out.status,
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let answer: serde_json::Value = serde_json::from_str(text.trim())
        .with_context(|| format!("agy -p=/{command} answered something that is not JSON"))?;
    if let Some(error) = answer["error"].as_str() {
        bail!("agy -p=/{command} refused: {error}");
    }
    Ok(Command {
        data: answer["command"]["data"].clone(),
    })
}

/// One hook the provider says it loaded, from the free `/hooks` listing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mounted {
    pub name: String,
    pub enabled: bool,
    /// The absolute path of the file the hook came from, which is what makes this
    /// a check rather than a guess: Hydra compares it against the file it wrote.
    pub source: String,
    pub events: Vec<String>,
}

/// Every hook `agy` reports loading for a run in this worktree.
///
/// Free, and it is the only thing standing between Hydra and 6.6's worst failure
/// mode on this provider: without `--add-dir` the hook config is not an error, it
/// is `hooks: []`.
pub fn hooks_mounted(worktree: &Path) -> Result<Vec<Mounted>> {
    let answer = slash(worktree, "hooks")?;
    Ok(parse_hooks(&answer.data))
}

/// The `/hooks` listing, parsed.
pub fn parse_hooks(data: &serde_json::Value) -> Vec<Mounted> {
    let Some(hooks) = data["hooks"].as_array() else {
        return Vec::new();
    };
    hooks
        .iter()
        .map(|hook| Mounted {
            name: hook["name"].as_str().unwrap_or_default().to_string(),
            // Absent means loaded, per the embedded reference: `enabled` defaults
            // to true and is written only to turn a hook off.
            enabled: hook["enabled"].as_bool().unwrap_or(true),
            source: hook["source"].as_str().unwrap_or_default().to_string(),
            events: hook["actions"]
                .as_array()
                .map(|actions| {
                    actions
                        .iter()
                        .filter_map(|action| action["event"].as_str())
                        .map(str::to_string)
                        .collect()
                })
                .unwrap_or_default(),
        })
        .collect()
}

/// Whether the gate Hydra wrote is loaded and answering `PreToolUse`.
///
/// The check is on the source path rather than on the hook's name, because a
/// name is something the config asked for and a source is where the provider says
/// it read the config from. A hook of Hydra's name coming out of somebody else's
/// file would pass a name check and gate nothing.
pub fn gate_is_mounted(mounted: &[Mounted], hooks_file: &Path) -> bool {
    let wanted = std::fs::canonicalize(hooks_file).unwrap_or_else(|_| hooks_file.to_path_buf());
    mounted.iter().any(|hook| {
        hook.enabled
            && hook.events.iter().any(|event| event == "PreToolUse")
            && std::fs::canonicalize(&hook.source)
                .map(|source| source == wanted)
                .unwrap_or_else(|_| Path::new(&hook.source) == wanted)
    })
}

/// What to tell a human when the gate did not mount.
pub fn unmounted_remediation(mounted: &[Mounted], hooks_file: &Path) -> String {
    let listed: Vec<String> = mounted
        .iter()
        .map(|hook| format!("{} from {}", hook.name, hook.source))
        .collect();
    format!(
        "agy did not load the gate Hydra wrote at {}. The free `/hooks` listing for this \
         worktree reported {}. Without the gate every tool call in the run would be \
         decided by the provider rather than by the policy, so the launch stops here. \
         This provider reads that file only when the worktree arrives as `--add-dir`, and \
         it reports a config it did not load as an empty listing rather than as an error.",
        hooks_file.display(),
        if listed.is_empty() {
            "no hooks at all".to_string()
        } else {
            listed.join("; ")
        }
    )
}

/// What `agy` says it resolved: the skills, free, and nothing about MCP.
///
/// **The skill listing costs nothing.** `agy -p="/skills" --output-format json`
/// answers with one record per surviving skill and `num_turns: 0`, measured on
/// 1.1.26 on 2026-09-06 with an injected skill in the listing under its
/// frontmatter `name`. That is the name this provider keys on, and it is why the
/// store's validator requires the frontmatter name to equal the directory name.
///
/// **The MCP half has no free answer and this one is honest about it.** Ticket 23
/// paid four turns for it on 1.1.25 through `/mcp`, and on 1.1.26 that command
/// refuses in print mode. Contract 5.3 requires a reader to treat `None` as
/// unanswered rather than as an empty listing, which is the whole reason the field
/// is an `Option`.
pub fn resolve(worktree: &Path, _mcp: crate::mcp::Probe<'_>) -> Result<Resolved> {
    let answer = slash(worktree, "skills")?;
    let skills = answer.data["skills"].as_array().map(|listed| {
        listed
            .iter()
            .filter_map(|skill| skill["name"].as_str())
            .map(str::to_string)
            .collect()
    });
    // The version is the one the behaviour was measured on, not the one running,
    // which a reader on 1.2.2 read as Hydra being three releases behind.
    eprintln!(
        "hydra: agy offers no free route to its resolved MCP set (measured on 1.1.26 and \
         unchanged since), so mcp.resolved is null on this run"
    );
    Ok(Resolved {
        skills,
        mcp: None,
        failed_mcp: Vec::new(),
    })
}

/// Reads the model and the session id off a frame.
///
/// **The two come off different frames and the wider one is not the `init`
/// frame.** The model is echoed once, in `init.model`, exactly as it was passed,
/// and the result frame has no model field at all, so a recorder that does not
/// latch it there loses it. The conversation id is on every frame this provider
/// writes, including the `step_update`s and the terminator, so it is read off
/// whichever arrives first rather than off `init` alone: a record whose init
/// frame did not parse still names its session.
pub fn observe(raw: &str) -> Observed {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(raw) else {
        return Observed::default();
    };
    let session_id = value["conversation_id"]
        .as_str()
        // The terminator nests it, and a free slash command leaves it empty.
        .or_else(|| value["result"]["conversation_id"].as_str())
        .or_else(|| value["step_update"]["conversation_id"].as_str())
        .filter(|id| !id.is_empty())
        .map(str::to_string);
    let model = match value.get("event").and_then(|v| v.as_str()) {
        Some("init") => value["init"]["model"].as_str().map(str::to_string),
        _ => None,
    };
    Observed { model, session_id }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_prompt_rides_inside_the_flag_and_the_worktree_is_named() {
        let worktree = Path::new("/tmp/wt");
        let argv = argv(&Launch {
            task: "do the thing",
            model: Some("gemini-3.8-flash-low"),
            worktree,
        });
        // 6.7: a bare `-p` swallows the next argument, so the prompt is inside
        // the flag and never beside it.
        assert_eq!(argv[0], "-p=do the thing");
        assert!(
            !argv.iter().any(|arg| arg == "-p"),
            "a bare -p drops the prompt: {argv:?}"
        );
        // The one flag three separate things ride in on.
        let add_dir = argv.iter().position(|arg| arg == "--add-dir").unwrap();
        assert_eq!(argv[add_dir + 1], "/tmp/wt");
        assert!(
            argv.iter()
                .any(|arg| arg == "--dangerously-skip-permissions")
        );
        assert_eq!(argv.last().unwrap(), "gemini-3.8-flash-low");
    }

    #[test]
    fn the_hook_config_is_keyed_by_name_and_wraps_its_handlers_in_a_matcher() {
        // The shape the 1.1.26 binary's own embedded reference documents. The
        // output shape research 26 recorded loads as nothing, with no error.
        let config = hooks_config(Path::new("/tmp/spawn/r1/gate"), "abc123");
        let gate = &config["hydra-gate"];
        assert_eq!(gate["enabled"], serde_json::json!(true));
        let group = &gate["PreToolUse"][0];
        assert_eq!(group["matcher"], serde_json::json!("*"));
        let handler = &group["hooks"][0];
        assert_eq!(handler["type"], serde_json::json!("command"));
        let command = handler["command"].as_str().unwrap();
        assert!(command.contains("/tmp/spawn/r1/gate"), "{command}");
        assert!(command.contains("abc123"), "{command}");
        // The dialect is the last argument, never inferred (6.6).
        assert!(command.ends_with(" agy"), "{command}");
        // Nothing in the schema turns failing closed off, so nothing here tries.
        assert!(!config.to_string().contains("failClosed"));
    }

    #[test]
    fn a_remote_server_carries_serverurl_and_a_stdio_one_is_the_shared_body() {
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
        let config = mcp_config(&[&stdio, &remote]);
        let servers = &config["mcpServers"];
        assert_eq!(servers["probe"]["command"], "/usr/bin/python3");
        assert_eq!(servers["probe"]["args"][0], "-u");
        assert_eq!(
            servers["remote"]["serverUrl"],
            "https://example.invalid/mcp"
        );
        assert!(
            servers["remote"]["url"].is_null(),
            "`url` is the other two providers' spelling"
        );
        assert!(servers["remote"]["type"].is_null());
    }

    #[test]
    fn the_hooks_listing_is_read_off_the_source_path() {
        // The `/hooks` answer measured on 1.1.26, with the paths shortened.
        let data = serde_json::json!({
            "hooks": [{
                "name": "hydra-gate",
                "enabled": true,
                "source": "/tmp/wt/.agents/hooks.json",
                "actions": [{
                    "event": "PreToolUse",
                    "matcher": "*",
                    "type": "command",
                    "command": "/usr/bin/python3 -c ... agy",
                    "timeout_seconds": 30,
                }],
            }]
        });
        let mounted = parse_hooks(&data);
        assert_eq!(mounted.len(), 1);
        assert_eq!(mounted[0].events, vec!["PreToolUse".to_string()]);
        assert!(gate_is_mounted(
            &mounted,
            Path::new("/tmp/wt/.agents/hooks.json")
        ));
        // A hook of Hydra's own name out of somebody else's file gates nothing.
        assert!(!gate_is_mounted(
            &mounted,
            Path::new("/tmp/other/.agents/hooks.json")
        ));
        // The silence this check exists for.
        assert!(parse_hooks(&serde_json::json!({ "hooks": [] })).is_empty());
        assert!(!gate_is_mounted(
            &[],
            Path::new("/tmp/wt/.agents/hooks.json")
        ));
        let remediation = unmounted_remediation(&[], Path::new("/tmp/wt/.agents/hooks.json"));
        assert!(remediation.contains("no hooks at all"), "{remediation}");
    }

    #[test]
    fn a_disabled_hook_is_not_a_mounted_gate() {
        let data = serde_json::json!({
            "hooks": [{
                "name": "hydra-gate",
                "enabled": false,
                "source": "/tmp/wt/.agents/hooks.json",
                "actions": [{ "event": "PreToolUse" }],
            }]
        });
        assert!(!gate_is_mounted(
            &parse_hooks(&data),
            Path::new("/tmp/wt/.agents/hooks.json")
        ));
    }

    #[test]
    fn the_model_and_the_session_come_off_the_init_frame_and_nowhere_else() {
        // The init frame measured on 1.1.26, cut to the two fields.
        let init = serde_json::json!({
            "event": "init",
            "conversation_id": "b631e11b-4a99-43db-9f37-11a373cdd98a",
            "init": { "model": "gemini-3.8-flash-low", "permission_mode": "request-review" },
        })
        .to_string();
        let observed = observe(&init);
        assert_eq!(observed.model.as_deref(), Some("gemini-3.8-flash-low"));
        assert_eq!(
            observed.session_id.as_deref(),
            Some("b631e11b-4a99-43db-9f37-11a373cdd98a")
        );
        // The result frame carries the conversation id and no model, so a
        // recorder that read it here would report the model as absent.
        let result = serde_json::json!({
            "event": "result",
            "result": { "conversation_id": "b631e11b", "status": "SUCCESS" },
        })
        .to_string();
        assert!(observe(&result).model.is_none());
    }
}
