//! `cursor-agent`, measured on 2026.09.02-c22c1a3 on 2026-09-06.
//!
//! The command line is harness spec 6.7's `cursor-agent` block. Everything
//! provider-specific about it lives in the worktree and in one environment
//! variable rather than on the argv: the hook config at `.cursor/hooks.json`, the
//! MCP config at `.cursor/mcp.json`, the skill links at `.cursor/skills/`, all read
//! from cwd, and `CURSOR_DATA_DIR`, which moves the provider's per-project MCP
//! approval state into the run's spawn directory (6.5).
//!
//! **`--force` is safe here for the reason `bypassPermissions` is on `claude`.**
//! Cursor rejects a gated call in print mode with the flag passed, measured by
//! ticket 26, and it has no permission channel in print mode, so it can only be
//! gated rather than asked.
//!
//! **This provider gets a fatal pre-flight and the other two do not.** An
//! unapproved project server does not shadow the human's same-named one, it fails
//! to replace it, so `mcp list` reports the name as not loaded while the human's
//! server starts with the human's token. The listing is the part that lies, so
//! Hydra approves its own servers individually, disables the rest under the
//! redirect, and refuses the launch when an injected server is not `ready`.
//!
//! **Two of its files are the same as `claude`'s and two are not.** The gate
//! script and the wrapper that checks its sha are shared. The hook config is
//! Cursor's own format rather than a translation of Claude's, because only
//! Cursor's format can say `failClosed: true`, and it registers `beforeReadFile`
//! beside `preToolUse` because reads are a capability (6.6).

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
}

/// 6.7's `cursor-agent` line. The task is the last argument, after every flag.
///
/// `-p` is `--print` on this binary and the prompt is a positional, so the two
/// halves of 6.7's `-p <task>` are one boolean flag and one argument. No
/// `--workspace`: cwd is the worktree, which is where the hook config, the MCP
/// config and the skill links are all read from, and `mcp enable` resolves the
/// project from cwd and ignores the flag anyway (6.5).
pub fn argv(launch: &Launch<'_>) -> Vec<String> {
    let mut argv = vec![
        "--print".to_string(),
        "--output-format".to_string(),
        "stream-json".to_string(),
        // 6.7: safe because a hook deny beats it, and because a detached child
        // has nobody to answer a prompt. It also covers the workspace trust
        // gate, which refuses a headless run in a directory nobody trusted.
        "--force".to_string(),
    ];
    if let Some(model) = launch.model {
        argv.push("--model".to_string());
        argv.push(model.to_string());
    }
    argv.push(launch.task.to_string());
    argv
}

/// Where the hook config goes, 6.6's table: inside the worktree, a real file.
pub fn hooks_file(worktree: &Path) -> PathBuf {
    worktree.join(".cursor/hooks.json")
}

/// Where the MCP config goes, 6.5's table.
pub fn mcp_file(worktree: &Path) -> PathBuf {
    worktree.join(".cursor/mcp.json")
}

/// The `.gitignore` Hydra drops beside its two files in `.cursor/`, and what it
/// holds.
///
/// The skill root ignores itself with `*` (6.4). This directory cannot, because
/// the human's repo may track its own `.cursor/rules/` and a `*` here would hide
/// a file the child creates there from the commit `hydra reap` makes. So the
/// three names Hydra writes are listed and nothing else is.
pub fn gitignore(worktree: &Path) -> (PathBuf, &'static str) {
    (
        worktree.join(".cursor/.gitignore"),
        "/.gitignore\n/hooks.json\n/mcp.json\n",
    )
}

/// The path one of Hydra's two `.cursor/` files will be written to, once the
/// checkout has been checked for the human's own copy and the ignore file is in
/// place.
///
/// **A file the checkout already holds is refused rather than overwritten.**
/// The worktree is seeded from the human's repo (6.3), and a `.cursor/hooks.json`
/// or `.cursor/mcp.json` in it is theirs: writing over it edits their work and
/// `hydra reap` would commit the edit, and loading theirs instead of Hydra's
/// launches an ungated run. [`crate::run`] checks the parent repo for both files
/// before a run id exists; this is the second line, on the checkout itself.
///
/// **An existing `.gitignore` is left alone**, the same rule the skill root
/// follows: a repo that tracks its own would see the overwrite as an edit to the
/// human's checkout. The cost is that Hydra's files then show in `git status`,
/// which is visible rather than silent.
pub fn claim(
    worktree: Option<&Path>,
    what: &str,
    file: impl Fn(&Path) -> PathBuf,
) -> Result<PathBuf> {
    let worktree = worktree.with_context(|| {
        format!("cursor-agent reads its {what} out of the worktree and this run has none")
    })?;
    let file = file(worktree);
    if file.exists() {
        bail!(
            "{} already exists in the checkout, and Hydra will not write over a file the \
             human's repo carries",
            file.display()
        );
    }
    if let Some(parent) = file.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    let (ignore, body) = gitignore(worktree);
    if !ignore.exists() {
        std::fs::write(&ignore, body).with_context(|| format!("writing {}", ignore.display()))?;
    }
    Ok(file)
}

/// The hook config, 6.6, in Cursor's own format.
///
/// **Two events, one script.** `preToolUse` fires for every tool this build
/// measured, reads included, with a `tool_name` and a `tool_input`.
/// `beforeReadFile` fires after a read succeeds inside the provider and before
/// the content reaches the model, carrying the path and the content and no tool
/// name; the gate maps it onto `Read`. Both are registered because 6.6 says
/// reads are a capability and one of them gating reads on this build is not a
/// promise about the next.
///
/// **`failClosed: true` on both**, which only this format can say. Cursor's
/// conversion of a Claude-format config writes `failClosed: false` into every
/// hook it translates, and a gate that fails open on the provider that
/// self-updates most is the wrong trade.
///
/// **No `matcher`.** Absent means every tool, and the gate's own table is what
/// decides per name.
pub fn hooks_config(gate_script: &Path, gate_sha: &str) -> serde_json::Value {
    let command = crate::policy::hook_command(gate_script, gate_sha, crate::policy::Dialect::Cursor);
    serde_json::json!({
        "version": 1,
        "hooks": {
            "preToolUse": [
                { "command": command, "failClosed": true }
            ],
            "beforeReadFile": [
                { "command": command, "failClosed": true }
            ]
        }
    })
}

/// Why a launch of this shape carries no MCP config at all, when it carries none.
///
/// 6.5: the redirect follows the tree, not the launch mode. A tree Hydra created
/// gets a data directory and an MCP config; a tree it did not create gets
/// neither, because approving a server there would write into `~/.cursor/`. So a
/// run with no checkout of Hydra's own cannot be delivered any server, and
/// [`crate::mcp::Plan::shortfall`] turns that into 6.1 item 6's refusal when the
/// policy admits one.
pub fn mcp_undeliverable(checkout: Checkout) -> Option<&'static str> {
    match checkout {
        Checkout::Hydra => None,
        Checkout::None => Some(
            "harness spec 6.5 gives a cursor-agent run no injected MCP config outside a \
             tree Hydra created, because approving a server there would write into \
             ~/.cursor/",
        ),
    }
}

/// The `.cursor/mcp.json` contents, 6.5.
///
/// The stdio body is byte-identical to `claude`'s. A remote server carries `url`
/// alone, where `claude` carries `type` beside it, which is the one place the
/// two files differ and the reason the canonical document is neither of them.
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

/// What one line of `cursor-agent mcp list` says about one server.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Listed {
    pub name: String,
    /// The provider's own words after the colon: `ready`, `not loaded (needs
    /// approval)`, `disabled`, or an error string. Only `ready` counts.
    pub status: String,
}

impl Listed {
    pub fn is_ready(&self) -> bool {
        self.status == "ready"
    }
}

/// `cursor-agent mcp list`'s output, parsed.
///
/// One `name: status` line per server, measured on 2026.09.02-c22c1a3 by tickets
/// 23 and 29 and again on 2026-09-06. A workspace with no server at all prints
/// one sentence, `No MCP servers configured (expected in .cursor/mcp.json or
/// ~/.cursor/mcp.json)`, which is an empty listing rather than a server called
/// `No MCP servers configured`.
pub fn parse_listing(text: &str) -> Vec<Listed> {
    text.lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() || line.starts_with("No MCP servers configured") {
                return None;
            }
            let (name, status) = line.split_once(": ")?;
            Some(Listed {
                name: name.trim().to_string(),
                status: status.trim().to_string(),
            })
        })
        .collect()
}

/// What the pre-flight found.
#[derive(Debug)]
pub struct Preflight {
    /// Every server the listing named, after the approvals and the disables.
    pub listing: Vec<Listed>,
}

impl Preflight {
    /// Contract 5.3's `mcp.resolved` on this provider: the servers the listing
    /// reported ready, in name order.
    ///
    /// **A free answer, and the pre-flight is what makes it exact.** The listing
    /// is taken after Hydra's servers are approved and everything else under the
    /// redirect is disabled, so a `ready` entry is one the run will start.
    pub fn resolved(&self) -> Vec<String> {
        let mut names: Vec<String> = self
            .listing
            .iter()
            .filter(|listed| listed.is_ready())
            .map(|listed| listed.name.clone())
            .collect();
        names.sort();
        names.dedup();
        names
    }

    /// 6.1 item 7's verdict: every injected server the listing did not report
    /// ready, with what it said instead.
    pub fn shortfall(&self, injected: &[String]) -> Vec<Listed> {
        injected
            .iter()
            .map(|name| {
                self.listing
                    .iter()
                    .find(|listed| &listed.name == name)
                    .cloned()
                    .unwrap_or_else(|| Listed {
                        name: name.clone(),
                        status: "not in the listing at all".to_string(),
                    })
            })
            .filter(|listed| !listed.is_ready())
            .collect()
    }

    /// What to tell a human, for `run_end.remediation`.
    pub fn remediation(&self, short: &[Listed]) -> String {
        let named: Vec<String> = short
            .iter()
            .map(|listed| format!("{}: {}", listed.name, listed.status))
            .collect();
        format!(
            "the Cursor MCP pre-flight refused this run. `cursor-agent mcp list` under the \
             run's own data directory did not report every injected server as ready: {}. \
             A server that is not ready here would leave the run with the human's \
             same-named server or with nothing, and the listing cannot tell which. Fix \
             the server's definition in {} or take this policy out of its admit list.",
            named.join("; "),
            crate::mcp::path().display(),
        )
    }
}

/// What one `cursor-agent mcp <args>` invocation came back with.
pub struct Answer {
    pub ok: bool,
    pub stdout: String,
    pub stderr: String,
}

/// The free pre-flight of 6.1 item 7, run with cwd inside the worktree and
/// `CURSOR_DATA_DIR` in the environment.
///
/// Three steps, all free, none of them a model call. `mcp enable <name>` for each
/// injected server, which resolves the project from cwd and ignores `--workspace`
/// (6.5). `mcp list`, then `mcp disable <name>` for every server it names that
/// Hydra did not inject: those are the human's user-scope servers, and under the
/// redirect the disable lands in the run's own data directory rather than in
/// `~/.cursor/` (6.5's per-run kill switch). Then `mcp list` again, which is the
/// listing the record and the verdict read.
///
/// **Each `mcp list` starts the provider's own server processes**, because the
/// listing connects to each server to report `ready` or an error, measured
/// 2026-09-06: a server whose command was `/bin/echo` listed as `Error:
/// Connection failed`. That is what makes a `ready` entry a server the run will
/// start rather than a file the run will read, and it is why 6.5 gives this
/// pre-flight to Cursor alone.
pub fn preflight(
    worktree: &Path,
    env: &crate::env::ChildEnv,
    injected: &[String],
) -> Result<Preflight> {
    preflight_with(injected, &mut |args| mcp_command(worktree, env, args))
}

/// The pre-flight over whatever runs `cursor-agent mcp`, so a test can drive the
/// sequence without the binary.
pub fn preflight_with(
    injected: &[String],
    run: &mut dyn FnMut(&[&str]) -> Result<Answer>,
) -> Result<Preflight> {
    for name in injected {
        let answer = run(&["enable", name])?;
        if !answer.ok {
            bail!(
                "cursor-agent mcp enable {name} failed: {}",
                answer.stderr.trim()
            );
        }
    }

    let first = list(run)?;
    let mut disabled_any = false;
    for listed in &first {
        if injected.iter().any(|name| name == &listed.name) {
            continue;
        }
        disabled_any = true;
        let answer = run(&["disable", &listed.name])?;
        if !answer.ok {
            // The gate refuses every call to a server Hydra did not inject
            // whatever the policy, so a disable that failed costs exposure and
            // not execution (6.5's residue), and the listing below reports the
            // server as `ready` for the record to keep.
            eprintln!(
                "hydra: cursor-agent mcp disable {} failed, so the human's server stays \
                 loaded for this run: {}",
                listed.name,
                answer.stderr.trim()
            );
        }
    }

    let listing = if disabled_any { list(run)? } else { first };
    Ok(Preflight { listing })
}

fn list(run: &mut dyn FnMut(&[&str]) -> Result<Answer>) -> Result<Vec<Listed>> {
    let answer = run(&["list"])?;
    if !answer.ok {
        bail!("cursor-agent mcp list failed: {}", answer.stderr.trim());
    }
    Ok(parse_listing(&answer.stdout))
}

fn mcp_command(worktree: &Path, env: &crate::env::ChildEnv, args: &[&str]) -> Result<Answer> {
    let out = std::process::Command::new("cursor-agent")
        .arg("mcp")
        .args(args)
        .current_dir(worktree)
        .env_clear()
        .envs(env.vars())
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .with_context(|| format!("running cursor-agent mcp {}", args.join(" ")))?;
    Ok(Answer {
        ok: out.status.success(),
        stdout: String::from_utf8_lossy(&out.stdout).to_string(),
        stderr: String::from_utf8_lossy(&out.stderr).to_string(),
    })
}

/// What `cursor-agent` says it resolved: nothing that is free.
///
/// Its skill listing is the paid one. Ticket 24 put it in
/// `~/.cursor/chats/<hash>/<session>/store.db`, largest blob, after a real run,
/// and a run refused for a usage limit writes no `store.db` at all. So
/// `skills.resolved` is null on this provider, which contract 5.3 makes optional
/// here for exactly that reason, and the third collision test does not run.
///
/// The MCP half has a free answer and it is not this function's: the pre-flight
/// listing of 6.1 item 7 is the provider reporting what it will start, and
/// [`Preflight::resolved`] is where `mcp.resolved` comes from on this provider.
pub fn resolve(_worktree: &Path, _mcp: crate::mcp::Probe<'_>) -> Result<Resolved> {
    eprintln!(
        "hydra: cursor-agent's skill listing is paid, so skills.resolved is null on this \
         run and the collision check compares the store against the index alone"
    );
    Ok(Resolved::default())
}

/// Reads the model and the session id off a frame.
///
/// Both land on `system`/`init`, in the same spelling `claude` uses. The model is
/// a display name rather than an id on this provider (contract 9.3): `Composer
/// 2.5` for `--model composer-2.5`, measured 2026-09-06, and `Auto` names no
/// vendor at all.
pub fn observe(raw: &str) -> Observed {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(raw) else {
        return Observed::default();
    };
    if value.get("type").and_then(|v| v.as_str()) != Some("system")
        || value.get("subtype").and_then(|v| v.as_str()) != Some("init")
    {
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

    #[test]
    fn the_task_is_last_and_force_rides_with_print() {
        let argv = argv(&Launch {
            task: "do the thing",
            model: Some("composer-2.5"),
        });
        assert_eq!(argv.first().unwrap(), "--print");
        assert!(argv.iter().any(|a| a == "--force"));
        assert!(argv.iter().any(|a| a == "stream-json"));
        let at = argv.iter().position(|a| a == "--model").unwrap();
        assert_eq!(argv[at + 1], "composer-2.5");
        assert_eq!(argv.last().unwrap(), "do the thing");
        // Partial output stays off (6.7), and nothing here asks for a workspace
        // because cwd is the worktree.
        assert!(!argv.iter().any(|a| a == "--stream-partial-output"));
        assert!(!argv.iter().any(|a| a == "--workspace"));
    }

    #[test]
    fn no_model_means_no_model_flag() {
        let argv = argv(&Launch {
            task: "hi",
            model: None,
        });
        assert!(!argv.iter().any(|a| a == "--model"));
    }

    #[test]
    fn the_hook_config_registers_both_events_and_fails_closed() {
        let config = hooks_config(Path::new("/tmp/spawn/r1/gate"), "abc123");
        assert_eq!(config["version"], 1);
        for event in ["preToolUse", "beforeReadFile"] {
            let hook = &config["hooks"][event][0];
            assert_eq!(hook["failClosed"], true, "{event}");
            let command = hook["command"].as_str().unwrap();
            assert!(command.ends_with(" cursor"), "{event}: {command}");
            assert!(command.contains("/tmp/spawn/r1/gate"));
            assert!(command.contains("abc123"));
            assert!(hook.get("matcher").is_none(), "absent means every tool");
        }
    }

    #[test]
    fn a_tampered_gate_answers_in_cursors_shape() {
        // The wrapper's own refusal has to be readable by the provider that
        // runs it, or a tampered script would be an invalid response rather
        // than a deny. `failClosed` would still block, and the reason would be
        // lost.
        let config = hooks_config(Path::new("/tmp/spawn/r1/gate"), "abc123");
        let command = config["hooks"]["preToolUse"][0]["command"]
            .as_str()
            .unwrap();
        // The JSON is a Python string literal inside a shell word, so the
        // quotes arrive escaped once.
        assert!(command.contains(r#"\"permission\":\"deny\""#), "{command}");
        assert!(command.contains("hydra/gate-tampered"));
        assert!(!command.contains("hookSpecificOutput"));
    }

    #[test]
    fn the_gitignore_names_only_what_hydra_writes() {
        let (path, body) = gitignore(Path::new("/tmp/wt"));
        assert_eq!(path, Path::new("/tmp/wt/.cursor/.gitignore"));
        assert!(body.contains("/hooks.json"));
        assert!(body.contains("/mcp.json"));
        assert!(!body.contains('*'), "a `*` here would hide the human's own files");
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
    fn a_remote_server_carries_url_alone() {
        // 6.5: `type` plus `url` on claude, `url` alone here.
        let mut docs = server("docs", crate::mcp::Transport::Http);
        docs.url = Some("https://example.com/mcp".to_string());
        docs.headers
            .insert("Authorization".to_string(), "Bearer x".to_string());
        let config = mcp_config(&[&docs]);
        assert_eq!(config["mcpServers"]["docs"]["url"], "https://example.com/mcp");
        assert!(config["mcpServers"]["docs"].get("type").is_none());
        assert_eq!(config["mcpServers"]["docs"]["headers"]["Authorization"], "Bearer x");
    }

    #[test]
    fn a_stdio_server_is_the_body_claude_gets() {
        let mut fs = server("filesystem", crate::mcp::Transport::Stdio);
        fs.command = Some("npx".to_string());
        fs.args = vec!["-y".to_string(), "x".to_string()];
        let ours = mcp_config(&[&fs]);
        let theirs = crate::provider::claude::mcp_config(&[&fs]);
        assert_eq!(ours["mcpServers"]["filesystem"], theirs["mcpServers"]["filesystem"]);
    }

    #[test]
    fn a_tree_hydra_did_not_create_cannot_be_delivered_a_server() {
        assert!(mcp_undeliverable(Checkout::None).is_some());
        assert!(mcp_undeliverable(Checkout::Hydra).is_none());
    }

    #[test]
    fn the_listing_is_read_line_by_line_and_the_empty_sentence_is_no_server() {
        // The three statuses tickets 23 and 29 measured, plus the sentence a
        // workspace with no server prints, measured 2026-09-06.
        let listing = parse_listing(
            "t29-probe: not loaded (needs approval)\nuseronly: ready\n\
             broken: Error: Connection failed\nold: disabled\n",
        );
        assert_eq!(listing.len(), 4);
        assert_eq!(listing[0].name, "t29-probe");
        assert_eq!(listing[0].status, "not loaded (needs approval)");
        assert!(listing[1].is_ready());
        assert_eq!(listing[2].status, "Error: Connection failed");
        assert!(
            parse_listing(
                "No MCP servers configured (expected in .cursor/mcp.json or ~/.cursor/mcp.json)\n"
            )
            .is_empty()
        );
    }

    #[test]
    fn the_verdict_is_every_injected_server_that_is_not_ready() {
        let preflight = Preflight {
            listing: parse_listing(
                "fs: ready\ndocs: not loaded (needs approval)\nhuman: disabled\n",
            ),
        };
        assert_eq!(preflight.resolved(), vec!["fs".to_string()]);

        let short = preflight.shortfall(&["fs".to_string(), "docs".to_string(), "gone".to_string()]);
        assert_eq!(short.len(), 2, "{short:?}");
        assert_eq!(short[0].name, "docs");
        assert_eq!(short[1].name, "gone");
        assert!(short[1].status.contains("not in the listing"));
        let remediation = preflight.remediation(&short);
        assert!(remediation.contains("docs: not loaded"));
        assert!(remediation.contains("mcp.md"));

        assert!(preflight.shortfall(&["fs".to_string()]).is_empty());
        assert!(preflight.shortfall(&[]).is_empty());
    }

    #[test]
    fn the_pre_flight_enables_hydras_servers_disables_the_rest_and_lists_again() {
        // 6.5's sequence, driven without the binary. The human's user-scope
        // server `human` is in the first listing and not in the injected set,
        // so it is disabled under the redirect; the second listing is the one
        // the record and the verdict read.
        let mut calls: Vec<Vec<String>> = Vec::new();
        let mut lists = 0;
        let mut run = |args: &[&str]| -> Result<Answer> {
            calls.push(args.iter().map(|a| a.to_string()).collect());
            let stdout = match args {
                ["list"] => {
                    lists += 1;
                    if lists == 1 {
                        "fs: ready\nhuman: ready\n"
                    } else {
                        "fs: ready\nhuman: disabled\n"
                    }
                }
                _ => "",
            };
            Ok(Answer {
                ok: true,
                stdout: stdout.to_string(),
                stderr: String::new(),
            })
        };
        let preflight = preflight_with(&["fs".to_string()], &mut run).unwrap();
        assert_eq!(
            calls,
            vec![
                vec!["enable", "fs"],
                vec!["list"],
                vec!["disable", "human"],
                vec!["list"],
            ]
        );
        assert_eq!(preflight.resolved(), vec!["fs".to_string()]);
        assert!(preflight.shortfall(&["fs".to_string()]).is_empty());
    }

    #[test]
    fn a_listing_with_nothing_to_disable_is_taken_once() {
        let mut calls = 0;
        let mut run = |args: &[&str]| -> Result<Answer> {
            calls += 1;
            Ok(Answer {
                ok: true,
                stdout: if args == ["list"] { "fs: ready\n".into() } else { String::new() },
                stderr: String::new(),
            })
        };
        let preflight = preflight_with(&["fs".to_string()], &mut run).unwrap();
        assert_eq!(calls, 2, "one enable, one list");
        assert_eq!(preflight.resolved(), vec!["fs".to_string()]);

        // An enable that fails is the pre-flight failing, not a server that is
        // not ready: nothing spawned, and the caller refuses with the error.
        let mut run = |_: &[&str]| -> Result<Answer> {
            Ok(Answer {
                ok: false,
                stdout: String::new(),
                stderr: "MCP server 'fs' not found in configuration".into(),
            })
        };
        let err = preflight_with(&["fs".to_string()], &mut run).unwrap_err();
        assert!(err.to_string().contains("not found in configuration"), "{err:#}");
    }

    #[test]
    fn claim_refuses_the_humans_file_and_drops_the_ignore_file_beside_hydras() {
        let root = std::env::temp_dir().join(format!("hydra-cursor-claim-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();

        let file = claim(Some(&root), "hook config", hooks_file).unwrap();
        assert_eq!(file, root.join(".cursor/hooks.json"));
        assert!(!file.exists(), "claim names the path and writes nothing to it");
        let ignore = std::fs::read_to_string(root.join(".cursor/.gitignore")).unwrap();
        assert!(ignore.contains("/hooks.json"));

        std::fs::write(&file, "{}").unwrap();
        let err = claim(Some(&root), "hook config", hooks_file).unwrap_err();
        assert!(err.to_string().contains("already exists"), "{err:#}");

        let err = claim(None, "MCP config", mcp_file).unwrap_err();
        assert!(err.to_string().contains("MCP config"), "{err:#}");
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn the_init_frame_names_the_display_name_and_the_session() {
        // Measured 2026-09-06 under `--model composer-2.5`.
        let observed = observe(
            r#"{"type":"system","subtype":"init","apiKeySource":"login","cwd":"/tmp/ws","session_id":"8ccaa19c-fa4f-42d5-82a4-732806f7a312","model":"Composer 2.5","permissionMode":"default"}"#,
        );
        assert_eq!(observed.model.as_deref(), Some("Composer 2.5"));
        assert_eq!(
            observed.session_id.as_deref(),
            Some("8ccaa19c-fa4f-42d5-82a4-732806f7a312")
        );
        assert!(observe(r#"{"type":"assistant"}"#).model.is_none());
    }
}
