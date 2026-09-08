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
//! **`--restricted` drops what settings define and keeps what the argv passes.**
//! Ticket 23 measured 2.1.257 dropping all MCP under the flag, and 6.5 and 6.7
//! were written from that. Re-measured 2026-09-06 on 2.1.263 with the free probe
//! below, a stub server in each settings scope: a user-scope server in a scratch
//! `HOME`'s `.claude.json` and a project-scope one in the cwd's `.mcp.json` both
//! load with no flags, and neither loads under `--restricted`. A server passed on
//! `--mcp-config` loads either way. So a `judge` run, which passes no file, gets
//! an empty set on this version.
//!
//! **`judge` carries `--strict-mcp-config` anyway**, ticket 43. Not to close a
//! hole, since there is none on 2.1.263, but because the vendor's help ties
//! skipping MCP to that flag rather than to `--restricted`, so the argv names the
//! guarantee instead of resting on how `--restricted` reads settings this month.
//! `tests/drift.rs` runs the probe under both shapes and fails the day either
//! moves.
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
/// 6.7: no `--permission-mode`, which `--restricted` refuses; `--strict-mcp-config`
/// with no `--mcp-config` beside it; no `--forward-subagent-text`, because `judge`
/// allows nothing and spawns nothing. The cost of `--restricted` everywhere else is
/// that it strips Bash and drops every server settings define, which is the point
/// for a policy that allows nothing and unaffordable for the rest.
pub const RESTRICTED_POLICY: &str = "judge";

/// The three flags 6.7 gives one invocation that decide what MCP it reads.
///
/// **The launch and the probe both derive them from [`mcp_flags`]**, so the
/// probe cannot answer for a server set the run will not have. Before ticket 43
/// the probe decided its own flags from whether a config file existed, which
/// left `judge` unanswerable: the run was `--restricted` and the probe was not.
pub struct McpFlags<'a> {
    /// `--restricted`. `judge` alone takes it, and it drops every server the
    /// human's settings define.
    pub restricted: bool,
    /// The file `--mcp-config` names, when this shape carries one.
    pub config: Option<&'a Path>,
    /// `--strict-mcp-config`, which makes the server set an argument Hydra wrote
    /// rather than the state of five directories at spawn time.
    pub strict: bool,
}

/// What one launch shape does about MCP, from the policy name and the file.
///
/// One function, because two answers to one question is how a later edit to
/// either half goes unnoticed. [`argv`] writes the launch from it and
/// [`probe_argv`] writes the probe.
pub fn mcp_flags<'a>(shape: crate::mcp::Probe<'a>) -> McpFlags<'a> {
    let restricted = shape.policy == RESTRICTED_POLICY;
    McpFlags {
        restricted,
        // 6.7's `judge` argv carries no file, and this refuses one even when a
        // caller hands it one. `mcp::plan` never does, and the two halves answer
        // the same question on purpose.
        config: if restricted { None } else { shape.config },
        // `judge` gets the strict flag on its own (ticket 43). Everywhere else
        // the two ride together: `--mcp-config` alone would add Hydra's servers
        // to whatever the human has, and the record could then report only what
        // Hydra intended.
        strict: restricted || shape.config.is_some(),
    }
}

impl McpFlags<'_> {
    /// The flags as argv, in 6.7's order.
    pub fn argv(&self) -> Vec<String> {
        let mut argv = Vec::new();
        if self.restricted {
            argv.push("--restricted".to_string());
        }
        if let Some(config) = self.config {
            argv.push("--mcp-config".to_string());
            argv.push(config.display().to_string());
        }
        if self.strict {
            argv.push("--strict-mcp-config".to_string());
        }
        argv
    }

    /// Whether a probe carrying these flags answers for the run's servers.
    ///
    /// The strict flag is the whole of it. With it the set is what the argv says,
    /// and without it the answer is whatever five directories held at spawn time,
    /// which is the machine's answer rather than this run's.
    pub fn answers_for_servers(&self) -> bool {
        self.strict
    }
}

pub fn argv(launch: &Launch<'_>) -> Vec<String> {
    let flags = mcp_flags(crate::mcp::Probe {
        policy: launch.policy,
        config: launch.mcp,
    });
    let restricted = flags.restricted;

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

    if !restricted {
        argv.push("--forward-subagent-text".to_string());
    }

    // The MCP block, and `--restricted` inside it because that flag is one of the
    // three that decide what a run reads. 6.7 prints it before `--settings` on a
    // `judge` line and after it on the other four; argv order is not semantic and
    // one block is worth more than matching both spellings.
    argv.extend(flags.argv());

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

/// The interactive argv, harness spec 3's second mode.
///
/// No `--print`, so the binary draws its own TUI instead of writing a
/// structured stream, and no `--output-format`, `--verbose` or
/// `--include-hook-events` beside it: nothing here is teed. `--session-id` is
/// the one flag every mode needs, because harvest predicts the transcript path
/// from it. No `--settings` and no `--permission-mode`: harness spec 3 gives an
/// interactive run no gate, and a human answering their own prompts is the
/// whole of the design there. `--mcp-config` rides alone, without
/// `--strict-mcp-config`, contract 5.6, so the human's own servers survive
/// beside whatever this run's file admits.
pub struct InteractiveLaunch<'a> {
    pub session: &'a str,
    /// The task string, verbatim and with no preamble: harness spec 3 gives an
    /// interactive run none. `None` opens the session with no opening prompt.
    pub task: Option<&'a str>,
    pub model: Option<&'a str>,
    pub mcp: Option<&'a Path>,
}

/// `claude` takes a positional argument as its opening prompt whether or not
/// `--print` is on the line, which is what lets [`argv`]'s trailing slot and
/// this one agree on where the task goes.
pub fn interactive_argv(launch: &InteractiveLaunch<'_>) -> Vec<String> {
    let mut argv = vec!["--session-id".to_string(), launch.session.to_string()];
    if let Some(mcp) = launch.mcp {
        argv.push("--mcp-config".to_string());
        argv.push(mcp.display().to_string());
    }
    if let Some(model) = launch.model {
        argv.push("--model".to_string());
        argv.push(model.to_string());
    }
    if let Some(task) = launch.task {
        argv.push(task.to_string());
    }
    argv
}

/// Why a launch of this shape carries no MCP config at all, when it carries none.
///
/// One shape on this provider: 6.7's `judge` argv has no `--mcp-config` on it, so
/// nothing Hydra writes can reach that run whatever the document admits.
/// [`crate::mcp::Plan::shortfall`] turns that into 6.1 item 6's refusal.
///
/// **The reason is the argv and not what a flag happens to do this month.** 6.7
/// used to justify the omission with "`--restricted` drops MCP anyway", which
/// ticket 23 measured on 2.1.257 and ticket 43 re-measured as half true on
/// 2.1.263. Either way the argv carries no file, which is why this answers on the
/// policy name.
pub fn mcp_undeliverable(policy: &str) -> Option<&'static str> {
    (policy == RESTRICTED_POLICY).then_some(
        "harness spec 6.7 gives a judge run no --mcp-config and --strict-mcp-config \
         beside its absence, so no server Hydra writes can reach it",
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
/// registers `beforeReadFile` as well, in [`crate::provider::cursor::hooks_config`].
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
///
/// **`showThinkingSummaries` puts the thinking text back on the stream.** Since
/// Claude Code 2.1.72 the binary sends a `redact-thinking-2026-02-12` beta header
/// that empties every thinking block before it is written. Measured 2026-09-07 on
/// 2.1.263 with haiku: 504 characters of thinking text with the key and 0 without,
/// and `thinking_tokens` non-zero either way. Build ticket 17.
pub fn settings(gate_script: &Path, gate_sha: &str) -> serde_json::Value {
    let command = crate::policy::hook_command(gate_script, gate_sha, Dialect::Claude);
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
        },
        "showThinkingSummaries": true
    })
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
/// **The probe carries the run's own MCP flags**, [`mcp_flags`]'s and not its
/// own. Without them it would resolve the human's servers and the record would
/// claim them for a run that will not have them. `--strict-mcp-config` on the
/// probe is chosen for that reason rather than reached for as a speed lever, and
/// it means the probe starts every server the run will start, once more per
/// launch. On `judge` that is no server at all and the answer is a measured `[]`.
///
/// **A `judge` probe answers nothing about skills.** It runs `--restricted`,
/// which drops every skill the filesystem offers: measured 2026-09-06 on 2.1.263
/// with a scratch `HOME`, the listing fell from 14 names to the binary's own 12
/// when a user-scope and a project-scope skill were both on disk. So the listing
/// a restricted probe reports cannot hold a name Hydra linked, and reporting it
/// would tell the collision check that every linked skill lost to something else.
/// Absent is the honest answer, and contract 5.3 already asks a reader to handle
/// it. What that leaves unsaid is that a `judge` run reads none of the skills
/// Hydra linked into its checkout, which no field carries today.
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
    let env = crate::env::ChildEnv::build(crate::provider::Provider::Claude)?;
    let home = crate::env::real_home()?;
    resolve_in(worktree, mcp, &home, &env)
}

/// [`resolve`], with the home and the environment named rather than discovered.
///
/// **One caller passes something other than the machine's own, and it is a
/// test.** `tests/drift.rs` has to run this under a scratch `HOME` holding stub
/// servers, because the alternative is writing a server into the human's
/// `~/.claude.json`. [`crate::env::ChildEnv::build`] drops a `HOME` that
/// disagrees with the passwd entry, which is the right rule for a launch and the
/// wrong one for measuring the binary, so that test sets the variable afterwards
/// through [`crate::env::ChildEnv::set`] and passes the same path here.
///
/// The two arguments travel together on purpose. A transcript is read out of the
/// `HOME` the child was given, so a caller that moved one and not the other would
/// look for the file in the wrong place.
pub fn resolve_in(
    worktree: &Path,
    mcp: crate::mcp::Probe<'_>,
    home: &Path,
    env: &crate::env::ChildEnv,
) -> anyhow::Result<Resolved> {
    use anyhow::Context;

    let session = uuid::Uuid::now_v7().to_string();
    let flags = mcp_flags(mcp);

    let out = std::process::Command::new("claude")
        .args(probe_argv(&session, mcp))
        .current_dir(worktree)
        .env_clear()
        .envs(env.vars())
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .context("running the skill listing probe")?;

    let transcript = transcript_path(home, worktree, &session);

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

    // **A null this side produces says so on stderr, like the other two.** A
    // failed probe already prints, and without this line a `judge` run's null
    // `skills.resolved` reads identically to a probe that fell over. The record
    // cannot tell them apart yet, which is the map question the section on
    // `--restricted` in `IMPLEMENTATION.md` writes up.
    if flags.restricted {
        eprintln!(
            "hydra: the probe ran --restricted, which lists no skill the filesystem \
             offers, so skills.resolved is null on this run by design"
        );
    }

    // Two passes over the same transcript, because the servers come back through
    // the reader `tests/drift.rs` uses too and the failures do not.
    Ok(Resolved {
        skills: if flags.restricted {
            None
        } else {
            skill_listing(&text)
        },
        // A probe that was not asked the run's own question answers nothing about
        // servers, whatever its own transcript says.
        mcp: if flags.answers_for_servers() {
            resolved_servers(&text)
        } else {
            None
        },
        failed_mcp: tool_listing(&text)
            .map(|listing| listing.failed)
            .unwrap_or_default(),
    })
}

/// The whole free probe's argv, which [`resolve`] runs and a drift test repeats.
///
/// **One caller matters and it is not `resolve`.** The probe's flags have to be
/// the launch's, and the only way to check that against the binary is to run this
/// argv under a scratch `HOME` where stub servers can be written without touching
/// the human's own config. `tests/drift.rs` does exactly that, so this is public.
pub fn probe_argv(session: &str, mcp: crate::mcp::Probe<'_>) -> Vec<String> {
    let mut argv: Vec<String> = ["-p", "x", "--model", PROBE_MODEL, "--session-id", session]
        .iter()
        .map(|arg| (*arg).to_string())
        .collect();
    argv.extend(mcp_flags(mcp).argv());
    argv
}

/// Where `--session-id` puts a probe's transcript.
///
/// **The cwd is canonicalized first.** Claude Code slugs the path it resolved,
/// not the one it was handed: a probe run in `/tmp/x` on macOS writes to
/// `-private-tmp-x`, measured 2026-09-06 on 2.1.263. A run's own worktree lives
/// under `~/hydra/` and crosses no link, so this changes nothing there and makes
/// the scratch directories a test uses reachable.
pub fn transcript_path(home: &Path, cwd: &Path, session: &str) -> std::path::PathBuf {
    let resolved = std::fs::canonicalize(cwd).unwrap_or_else(|_| cwd.to_path_buf());
    home.join(".claude/projects")
        .join(project_slug(&resolved))
        .join(format!("{session}.jsonl"))
}

/// The servers a probe's transcript says resolved, contract 5.3's `mcp.resolved`.
///
/// `None` when the transcript carries no tool listing at all, which is unanswered
/// rather than empty. An empty list is a real answer and the one a `judge` probe
/// gives.
pub fn resolved_servers(transcript: &str) -> Option<Vec<String>> {
    tool_listing(transcript).map(|listing| mcp_servers(&listing.added))
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
    fn thinking_text_is_turned_back_on_in_the_settings() {
        let value = settings(Path::new("/tmp/spawn/r1/gate.py"), "abc");
        assert_eq!(
            value["showThinkingSummaries"],
            serde_json::Value::Bool(true)
        );
        assert_eq!(value["hooks"]["PreToolUse"].as_array().unwrap().len(), 1);
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
    fn judge_carries_the_strict_flag_and_never_a_file() {
        // 6.7's `judge` shape since ticket 43. `mcp::plan` never hands this
        // policy a file, and the argv refuses one anyway, because two answers to
        // one question is how a later edit to either half goes unnoticed. The
        // strict flag rides alone, which is the one shape that means "no servers
        // at all" rather than "these servers".
        let argv = argv(&launch("judge", Path::new("/tmp/s.json")));
        assert!(!argv.iter().any(|a| a == "--mcp-config"));
        assert!(argv.iter().any(|a| a == "--strict-mcp-config"));
        assert!(argv.iter().any(|a| a == "--restricted"));
    }

    #[test]
    fn the_probe_carries_the_launchs_own_mcp_flags() {
        // Ticket 43. The probe's answer is the run's answer only when the two
        // argvs agree about MCP, so they come off one function and this is where
        // that is checked.
        for (policy, config) in [("judge", None), ("standard", Some(Path::new(MCP)))] {
            let shape = crate::mcp::Probe { policy, config };
            let probe = probe_argv("01a07600-0000-7000-8000-000000000001", shape);
            let launched = argv(&Launch {
                task: "hi",
                model: None,
                settings: Path::new("/tmp/s.json"),
                policy,
                mcp: config,
            });
            for flag in ["--restricted", "--mcp-config", "--strict-mcp-config"] {
                assert_eq!(
                    probe.iter().any(|a| a == flag),
                    launched.iter().any(|a| a == flag),
                    "{policy} passes {flag} on one of the two argvs and not the other"
                );
            }
        }
    }

    #[test]
    fn only_a_strict_probe_may_claim_the_runs_server_set() {
        // Without the flag the set is whatever five directories held at spawn
        // time, so an answer would be the machine's rather than this run's.
        let judge = mcp_flags(crate::mcp::Probe {
            policy: "judge",
            config: None,
        });
        assert!(
            judge.answers_for_servers(),
            "an empty set is still a measurement"
        );

        let injected = mcp_flags(crate::mcp::Probe {
            policy: "standard",
            config: Some(Path::new(MCP)),
        });
        assert!(injected.answers_for_servers());

        let nothing = mcp_flags(crate::mcp::Probe {
            policy: "standard",
            config: None,
        });
        assert!(!nothing.answers_for_servers());
    }

    #[test]
    fn a_judge_probe_reads_no_servers_off_a_listing_that_holds_none() {
        // A trimmed copy of what `tests/drift.rs` gets from the real binary, read
        // through the same function `resolve` uses. That probe, 2026-09-06 on
        // 2.1.263 under `--restricted --strict-mcp-config` with a stub server in
        // each settings scope, listed fourteen deferred built-ins and no `mcp__`
        // name at all.
        let transcript = concat!(
            "{\"type\":\"attachment\",\"attachment\":{\"type\":\"deferred_tools_delta\",",
            "\"addedNames\":[\"WebSearch\",\"TaskCreate\"],\"failedMcpServers\":[]}}\n",
        );
        assert_eq!(resolved_servers(transcript), Some(Vec::new()));
        assert_eq!(
            resolved_servers("{\"type\":\"user\"}\n"),
            None,
            "no listing is unanswered, which is not the same as an empty set"
        );
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
    fn the_interactive_argv_carries_no_gate_and_no_stream_flags() {
        let argv = interactive_argv(&InteractiveLaunch {
            session: "s1",
            task: Some("do the thing"),
            model: Some("sonnet"),
            mcp: Some(Path::new("/tmp/spawn/r1/mcp.json")),
        });
        assert_eq!(argv[0], "--session-id");
        assert_eq!(argv[1], "s1");
        assert!(argv.iter().any(|a| a == "--mcp-config"));
        assert!(
            !argv.iter().any(|a| a == "--strict-mcp-config"),
            "the human's own servers have to survive beside this run's"
        );
        for flag in [
            "--print",
            "--output-format",
            "--include-hook-events",
            "--settings",
            "--permission-mode",
        ] {
            assert!(!argv.iter().any(|a| a == flag), "{flag} rides no gate");
        }
        assert_eq!(argv.last().unwrap(), "do the thing");
    }

    #[test]
    fn an_interactive_launch_with_no_task_opens_with_no_opening_prompt() {
        let argv = interactive_argv(&InteractiveLaunch {
            session: "s1",
            task: None,
            model: None,
            mcp: None,
        });
        assert_eq!(argv, vec!["--session-id".to_string(), "s1".to_string()]);
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
