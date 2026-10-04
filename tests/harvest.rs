//! `hydra run --interactive`'s harvest, harness spec 3 and 10, contract 7.9.
//!
//! Every test here builds a vendor transcript by hand, in the shape map ticket
//! 38 measured on real `~/.claude/projects/` files, and points
//! [`hydra::interactive::harvest_in`] at a scratch home directory rather than
//! the developer's own. No provider is launched.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use hydra::interactive::{HarvestOutcome, harvest_in};
use hydra::normalize::{Event, Finding, Role};
use hydra::paths::Layout;
use hydra::provider::claude::transcript_path;
use hydra::record::{Auth, RunStart, SCHEMA_VERSION, WorktreeRef, Writer};
use serde_json::{Value, json};

fn scratch(name: &str) -> (Layout, PathBuf, PathBuf) {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let root = std::env::temp_dir().join(format!(
        "hydra-harvest-{}-{name}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&root);
    let layout = Layout::at(root.join("hydra"));
    layout.ensure().unwrap();
    let home = root.join("home");
    let cwd = root.join("cwd");
    std::fs::create_dir_all(&home).unwrap();
    std::fs::create_dir_all(&cwd).unwrap();
    (layout, home, cwd)
}

/// The `run_start` an interactive launch writes, contract 5.6's shape: no
/// policy, no worktree, the session id known up front rather than backfilled
/// off an init frame.
fn open_interactive_run(layout: &Layout, run_id: &str, session: &str, cwd: &Path) {
    let pid = std::process::id();
    let start_time = hydra::procinfo::start_time(pid).unwrap();
    let start = RunStart {
        kind: "run_start",
        schema_version: SCHEMA_VERSION,
        hydra_version: "0.2.0".into(),
        command: "run".into(),
        launch_mode: Some("interactive".into()),
        provider_version: Some("2.1.261 (Claude Code)".into()),
        provider_session_id: Some(session.to_string()),
        task: Some("probe".into()),
        policy: None,
        policy_sha: None,
        floor_lifted: None,
        gate_sha: None,
        sandbox: None,
        scopes: None,
        cwd: cwd.display().to_string(),
        worktree: None,
        depth: 0,
        parent_policy: None,
        pid: Some(pid),
        process_start_time: start_time
            .map(|t| t.to_rfc3339_opts(chrono::SecondsFormat::Micros, true)),
        skills: None,
        mcp: Some(json!({"doc_sha": "abc", "injected": [], "resolved": null})),
        auth: Some(Auth {
            mechanism: "subscription-oauth",
            source: "keychain",
            allow_billed: false,
            gated: true,
        }),
        env_allowlist: Some(vec!["HOME".into(), "PATH".into()]),
        subscription: Some("anthropic".into()),
        resumed_from: None,
        may_edit: None,
    };
    let mut writer =
        Writer::create(layout.run_file(run_id), run_id, None, Some("claude"), None).unwrap();
    writer.hydra_line(&start).unwrap();
}

/// The `run_start` a gated interactive launch writes: policy, worktree and
/// scopes on the envelope, with the session id known up front.
fn open_gated_interactive_run(
    layout: &Layout,
    run_id: &str,
    session: &str,
    cwd: &Path,
    policy: &str,
) {
    let pid = std::process::id();
    let start_time = hydra::procinfo::start_time(pid).unwrap();
    let policies = hydra::policy::PolicySet::discover().unwrap();
    let start = RunStart {
        kind: "run_start",
        schema_version: SCHEMA_VERSION,
        hydra_version: "0.2.0".into(),
        command: "run".into(),
        launch_mode: Some("interactive".into()),
        provider_version: Some("2.1.261 (Claude Code)".into()),
        provider_session_id: Some(session.to_string()),
        task: Some("probe".into()),
        policy: Some(policy.to_string()),
        policy_sha: Some(policies.sha().to_string()),
        floor_lifted: Some(false),
        gate_sha: None,
        sandbox: None,
        scopes: Some(std::collections::BTreeMap::from([(
            "worktree".to_string(),
            cwd.display().to_string(),
        )])),
        cwd: cwd.display().to_string(),
        worktree: Some(WorktreeRef {
            path: cwd.display().to_string(),
            branch: format!("hydra/{run_id}"),
            base_tree: Some("abc123".to_string()),
        }),
        depth: 0,
        parent_policy: None,
        pid: Some(pid),
        process_start_time: start_time
            .map(|t| t.to_rfc3339_opts(chrono::SecondsFormat::Micros, true)),
        skills: None,
        mcp: Some(json!({"doc_sha": "abc", "injected": [], "resolved": null})),
        auth: Some(Auth {
            mechanism: "subscription-oauth",
            source: "keychain",
            allow_billed: false,
            gated: true,
        }),
        env_allowlist: Some(vec!["HOME".into(), "PATH".into()]),
        subscription: Some("anthropic".into()),
        resumed_from: None,
        may_edit: None,
    };
    let mut writer =
        Writer::create(layout.run_file(run_id), run_id, None, Some("claude"), None).unwrap();
    writer.hydra_line(&start).unwrap();
}

/// A gate refusal in the vendor transcript: an error `tool_result` whose text
/// starts with `hydra/`, map ticket 50's measured shape.
fn gate_denial(uuid: &str, parent: &str, tool_use_id: &str, reason: &str) -> String {
    record(
        uuid,
        Some(parent),
        "user",
        json!({
            "message": {
                "role": "user",
                "content": [{
                    "type": "tool_result",
                    "tool_use_id": tool_use_id,
                    "content": reason,
                    "is_error": true,
                }],
            },
        }),
    )
}

/// One record of the vendor's own transcript, in the shape research 34 and 35
/// measured: `type`, `uuid`, `parentUuid`, and `message`.
fn record(uuid: &str, parent: Option<&str>, kind: &str, extra: Value) -> String {
    let mut body = json!({
        "type": kind,
        "uuid": uuid,
        "parentUuid": parent,
    });
    for (key, value) in extra.as_object().unwrap() {
        body[key] = value.clone();
    }
    body.to_string()
}

fn assistant_tool_use(
    uuid: &str,
    parent: &str,
    tool_use_id: &str,
    name: &str,
    input: Value,
) -> String {
    record(
        uuid,
        Some(parent),
        "assistant",
        json!({
            "message": {
                "role": "assistant",
                "content": [{"type": "tool_use", "id": tool_use_id, "name": name, "input": input}],
                "usage": {
                    "input_tokens": 4,
                    "output_tokens": 12,
                    "cache_read_input_tokens": 0,
                    "cache_creation_input_tokens": 0,
                },
            },
        }),
    )
}

/// A human's rejection at an interactive prompt: `toolDenialKind`,
/// `entrypoint` and `sourceToolAssistantUUID` on a `user` record whose own
/// content is the `tool_result` the denial produced.
fn human_denial(uuid: &str, parent: &str, source: &str, tool_use_id: &str) -> String {
    record(
        uuid,
        Some(parent),
        "user",
        json!({
            "toolDenialKind": "user-rejected",
            "entrypoint": "cli",
            "sourceToolAssistantUUID": source,
            "message": {
                "role": "user",
                "content": [{
                    "type": "tool_result",
                    "tool_use_id": tool_use_id,
                    "content": "The user doesn't want to proceed with this tool use.",
                    "is_error": true,
                }],
            },
        }),
    )
}

/// The interrupt marker every denial also emits, ticket 35's finding: a
/// `user` record carrying a text block, which is a block array and therefore
/// not the plain-string shape a human's own words arrive in.
fn interrupt_marker(uuid: &str, parent: &str) -> String {
    record(
        uuid,
        Some(parent),
        "user",
        json!({
            "message": {
                "role": "user",
                "content": [{"type": "text", "text": "[Request interrupted by user for tool use]"}],
            },
        }),
    )
}

/// The human's typed redirection, a bare string rather than a block list.
fn human_words(uuid: &str, parent: &str, text: &str) -> String {
    record(
        uuid,
        Some(parent),
        "user",
        json!({ "message": { "role": "user", "content": text } }),
    )
}

fn write_transcript(home: &Path, cwd: &Path, session: &str, lines: &[String]) {
    let path = transcript_path(home, cwd, session);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, format!("{}\n", lines.join("\n"))).unwrap();
}

#[test]
fn a_human_rejection_and_its_retry_become_a_correction_with_the_words_kept() {
    let (layout, home, cwd) = scratch("bash");
    let run_id = "01a07900-0000-7000-8000-0000000000a1";
    let session = "01a07900-0000-7000-8000-0000000000a2";
    open_interactive_run(&layout, run_id, session, &cwd);

    let lines = vec![
        assistant_tool_use(
            "a1",
            "root",
            "toolu_1",
            "Bash",
            json!({"command": "touch t35probe.txt"}),
        ),
        human_denial("a2", "a1", "a1", "toolu_1"),
        interrupt_marker("a3", "a2"),
        record("a4", Some("a3"), "file-history-snapshot", json!({})),
        human_words(
            "a5",
            "a4",
            "use printf to write the word hello into it instead",
        ),
        assistant_tool_use(
            "a6",
            "a5",
            "toolu_2",
            "Bash",
            json!({"command": "printf 'hello' > t35probe.txt"}),
        ),
    ];
    write_transcript(&home, &cwd, session, &lines);

    let outcome = harvest_in(&layout, run_id, Some(0), &home).unwrap();
    assert!(
        matches!(outcome, HarvestOutcome::Harvested { records: 6 }),
        "{outcome:?}"
    );

    let text = std::fs::read_to_string(layout.run_file(run_id)).unwrap();
    hydra::record::validate::check(&text, Some(run_id)).assert_ok();

    let policies = hydra::policy::PolicySet::discover().unwrap();
    let run = hydra::normalize::read_file(&layout.run_file(run_id), &policies).unwrap();

    assert_eq!(run.policy, None, "no gate was compiled for this run");
    assert_eq!(
        run.permission_decisions().count(),
        0,
        "an interactive run has no policy to re-evaluate a call against"
    );

    let correction = run
        .events
        .iter()
        .find_map(|event| match event {
            Event::Correction(c) => Some(c),
            _ => None,
        })
        .expect("the retry should have produced a correction");
    assert_eq!(
        correction.rejected,
        json!({"command": "touch t35probe.txt"})
    );
    assert_eq!(
        correction.chosen,
        json!({"command": "printf 'hello' > t35probe.txt"})
    );
    assert_eq!(
        correction.words.as_deref(),
        Some("use printf to write the word hello into it instead")
    );

    // The two calls and the two results all still normalize on their own,
    // beside the correction that pairs them (contract 7.2.1: every other
    // derived row keeps its type).
    assert_eq!(run.tool_calls().count(), 2);
    assert_eq!(run.tool_results().count(), 1);
    assert_eq!(
        run.messages()
            .filter(|m| m.role == Role::User && m.text.contains("printf"))
            .count(),
        1
    );

    // `message.usage` rode two assistant turns; the harvested `usage` event
    // is their sum (contract 7.2.1).
    let usage = run.usage().expect("usage should have a producer here");
    let counts = usage.counts.expect("both assistant turns carried counts");
    assert_eq!(counts.input_tokens, 8);
    assert_eq!(counts.output_tokens, 24);
}

#[test]
fn a_rejected_edit_needs_no_special_handling() {
    // Contract 7.9: "A rejected `Edit` needs no special handling. The diff is
    // `old_string` and `new_string` in the ordinary tool input."
    let (layout, home, cwd) = scratch("edit");
    let run_id = "01a07900-0000-7000-8000-0000000000b1";
    let session = "01a07900-0000-7000-8000-0000000000b2";
    open_interactive_run(&layout, run_id, session, &cwd);

    let rejected_input = json!({
        "file_path": "/tmp/edit-me.txt",
        "old_string": "beta",
        "new_string": "delta",
        "replace_all": false,
    });
    let chosen_input = json!({
        "file_path": "/tmp/edit-me.txt",
        "old_string": "gamma",
        "new_string": "delta",
        "replace_all": false,
    });
    let lines = vec![
        assistant_tool_use("e1", "root", "toolu_9", "Edit", rejected_input.clone()),
        human_denial("e2", "e1", "e1", "toolu_9"),
        human_words("e3", "e2", "change gamma instead, leave beta alone"),
        assistant_tool_use("e4", "e3", "toolu_10", "Edit", chosen_input.clone()),
    ];
    write_transcript(&home, &cwd, session, &lines);

    harvest_in(&layout, run_id, Some(0), &home).unwrap();
    let policies = hydra::policy::PolicySet::discover().unwrap();
    let run = hydra::normalize::read_file(&layout.run_file(run_id), &policies).unwrap();

    let correction = run
        .events
        .iter()
        .find_map(|event| match event {
            Event::Correction(c) => Some(c),
            _ => None,
        })
        .expect("a rejected Edit should pair with its retry");
    assert_eq!(correction.rejected, rejected_input);
    assert_eq!(correction.chosen, chosen_input);
}

#[test]
fn a_second_harvest_of_the_same_run_writes_nothing() {
    let (layout, home, cwd) = scratch("idempotent");
    let run_id = "01a07900-0000-7000-8000-0000000000c1";
    let session = "01a07900-0000-7000-8000-0000000000c2";
    open_interactive_run(&layout, run_id, session, &cwd);
    write_transcript(
        &home,
        &cwd,
        session,
        &[assistant_tool_use(
            "f1",
            "root",
            "toolu_1",
            "Bash",
            json!({"command": "echo hi"}),
        )],
    );

    let first = harvest_in(&layout, run_id, Some(0), &home).unwrap();
    assert!(matches!(first, HarvestOutcome::Harvested { .. }));
    let text_after_first = std::fs::read_to_string(layout.run_file(run_id)).unwrap();

    let second = harvest_in(&layout, run_id, Some(0), &home).unwrap();
    assert_eq!(second, HarvestOutcome::AlreadyDone);
    let text_after_second = std::fs::read_to_string(layout.run_file(run_id)).unwrap();

    assert_eq!(
        text_after_first, text_after_second,
        "a second harvest must not append anything"
    );
}

#[test]
fn an_agent_that_argued_instead_of_retrying_produces_no_pair() {
    // Step 6 of the harvest rule: a text reply after a rejection is a
    // rejection without a correction, not a pair with an empty chosen half.
    let (layout, home, cwd) = scratch("argued");
    let run_id = "01a07900-0000-7000-8000-0000000000d1";
    let session = "01a07900-0000-7000-8000-0000000000d2";
    open_interactive_run(&layout, run_id, session, &cwd);

    let lines = vec![
        assistant_tool_use(
            "g1",
            "root",
            "toolu_1",
            "Bash",
            json!({"command": "rm -rf /tmp/x"}),
        ),
        human_denial("g2", "g1", "g1", "toolu_1"),
        human_words("g3", "g2", "never mind, forget it"),
        record(
            "g4",
            Some("g3"),
            "assistant",
            json!({
                "message": {
                    "role": "assistant",
                    "content": [{"type": "text", "text": "Understood, I will not run that."}],
                },
            }),
        ),
    ];
    write_transcript(&home, &cwd, session, &lines);

    harvest_in(&layout, run_id, Some(0), &home).unwrap();
    let policies = hydra::policy::PolicySet::discover().unwrap();
    let run = hydra::normalize::read_file(&layout.run_file(run_id), &policies).unwrap();

    assert!(
        !run.events
            .iter()
            .any(|event| matches!(event, Event::Correction(_))),
        "an argued reply is a rejection without a correction"
    );
}

#[test]
fn a_gated_harvest_derives_one_denial_and_one_limit_finding() {
    let (layout, home, cwd) = scratch("gated-deny");
    let run_id = "01a07900-0000-7000-8000-0000000000e1";
    let session = "01a07900-0000-7000-8000-0000000000e2";
    open_gated_interactive_run(&layout, run_id, session, &cwd, "read-only");

    let reason = "hydra/capability: policy read-only does not allow file_write (Write).";
    let lines = vec![
        assistant_tool_use(
            "h1",
            "root",
            "toolu_1",
            "Write",
            json!({"file_path": "/tmp/x.txt", "content": "x"}),
        ),
        gate_denial("h2", "h1", "toolu_1", reason),
    ];
    write_transcript(&home, &cwd, session, &lines);

    harvest_in(&layout, run_id, Some(0), &home).unwrap();
    let policies = hydra::policy::PolicySet::discover().unwrap();
    let run = hydra::normalize::read_file(&layout.run_file(run_id), &policies).unwrap();

    assert_eq!(run.policy.as_deref(), Some("read-only"));
    assert_eq!(run.permission_decisions().count(), 1);
    let decision = run.permission_decisions().next().unwrap();
    assert_eq!(decision.decision, hydra::normalize::Decision::Deny);
    assert_eq!(decision.decider, hydra::normalize::Decider::HydraPolicy);
    assert_eq!(decision.reason, reason);
    assert!(
        run.findings
            .iter()
            .any(|finding| matches!(finding, Finding::Limit(text) if text == "allows were not recorded on this launch mode")),
        "{:?}",
        run.findings
    );
}

#[test]
fn an_ungated_harvest_derives_no_denial_and_no_limit_finding() {
    let (layout, home, cwd) = scratch("ungated");
    let run_id = "01a07900-0000-7000-8000-0000000000f1";
    let session = "01a07900-0000-7000-8000-0000000000f2";
    open_interactive_run(&layout, run_id, session, &cwd);

    let lines = vec![
        assistant_tool_use(
            "i1",
            "root",
            "toolu_1",
            "Write",
            json!({"file_path": "/tmp/x.txt", "content": "x"}),
        ),
        gate_denial(
            "i2",
            "i1",
            "toolu_1",
            "hydra/capability: policy read-only does not allow file_write (Write).",
        ),
    ];
    write_transcript(&home, &cwd, session, &lines);

    harvest_in(&layout, run_id, Some(0), &home).unwrap();
    let policies = hydra::policy::PolicySet::discover().unwrap();
    let run = hydra::normalize::read_file(&layout.run_file(run_id), &policies).unwrap();

    assert_eq!(run.policy, None);
    assert_eq!(run.permission_decisions().count(), 0);
    assert!(
        !run.findings
            .iter()
            .any(|finding| matches!(finding, Finding::Limit(_))),
        "{:?}",
        run.findings
    );
}

#[test]
fn a_harvested_session_limit_is_rate_limited() {
    let (layout, home, cwd) = scratch("rate-limit");
    let run_id = "01a07900-0000-7000-8000-0000000000g1";
    let session = "01a07900-0000-7000-8000-0000000000g2";
    open_interactive_run(&layout, run_id, session, &cwd);

    let limit_text = "You've hit your session limit · resets 1:50am (America/Bogota)";
    let lines = vec![record(
        "j1",
        Some("root"),
        "assistant",
        json!({
            "message": {
                "role": "assistant",
                "content": [{"type": "text", "text": limit_text}],
            },
        }),
    )];
    write_transcript(&home, &cwd, session, &lines);

    harvest_in(&layout, run_id, Some(0), &home).unwrap();
    let policies = hydra::policy::PolicySet::discover().unwrap();
    let run = hydra::normalize::read_file(&layout.run_file(run_id), &policies).unwrap();

    let derived = hydra::outcome::derive(&run).unwrap();
    assert_eq!(derived.outcome, hydra::outcome::Outcome::RateLimited);
    assert_eq!(derived.source, hydra::outcome::Source::Derived);
    assert_eq!(
        hydra::outcome::final_assistant_text(&run).as_deref(),
        Some(limit_text)
    );
}

#[test]
fn a_harvested_run_with_ordinary_assistant_text_is_completed() {
    let (layout, home, cwd) = scratch("completed-harvest");
    let run_id = "01a07900-0000-7000-8000-0000000000h1";
    let session = "01a07900-0000-7000-8000-0000000000h2";
    open_interactive_run(&layout, run_id, session, &cwd);

    let lines = vec![record(
        "k1",
        Some("root"),
        "assistant",
        json!({
            "message": {
                "role": "assistant",
                "content": [{"type": "text", "text": "Done. Here's what I found."}],
            },
        }),
    )];
    write_transcript(&home, &cwd, session, &lines);

    harvest_in(&layout, run_id, Some(0), &home).unwrap();
    let policies = hydra::policy::PolicySet::discover().unwrap();
    let run = hydra::normalize::read_file(&layout.run_file(run_id), &policies).unwrap();

    let derived = hydra::outcome::derive(&run).unwrap();
    assert_eq!(derived.outcome, hydra::outcome::Outcome::Completed);
    assert_eq!(derived.source, hydra::outcome::Source::Derived);
}

/// Line 0 of an ungated interactive run with its liveness pair left for
/// `record_then_wait` to fill.
fn unfilled_start(cwd: &Path) -> RunStart {
    RunStart {
        kind: "run_start",
        schema_version: SCHEMA_VERSION,
        hydra_version: "0.2.0".into(),
        command: "run".into(),
        launch_mode: Some("interactive".into()),
        provider_version: Some("2.1.268 (Claude Code)".into()),
        provider_session_id: Some("00000000-0000-7000-8000-000000000107".into()),
        task: Some("probe".into()),
        policy: None,
        policy_sha: None,
        floor_lifted: None,
        gate_sha: None,
        sandbox: None,
        scopes: None,
        cwd: cwd.display().to_string(),
        worktree: None,
        depth: 0,
        parent_policy: None,
        pid: None,
        process_start_time: None,
        skills: None,
        mcp: Some(json!({"doc_sha": "abc", "injected": [], "resolved": null})),
        auth: Some(Auth {
            mechanism: "subscription-oauth",
            source: "keychain",
            allow_billed: false,
            gated: true,
        }),
        env_allowlist: Some(vec!["HOME".into(), "PATH".into()]),
        subscription: Some("anthropic".into()),
        resumed_from: None,
        may_edit: None,
    }
}

/// A stand-in for the session: a shell that sleeps and then leaves a file, so
/// the file says whether the caller waited for it.
fn session_that_leaves(marker: &Path) -> std::process::Child {
    std::process::Command::new("/bin/sh")
        .arg("-c")
        .arg(format!("sleep 0.5; : > '{}'", marker.display()))
        .spawn()
        .unwrap()
}

/// Build ticket 112, item 1. The start time is read after `claude` is
/// running, so a failure there cannot roll the launch back. It closes the
/// record with the pid in the error and still waits for the session.
#[test]
fn a_start_time_that_will_not_read_leaves_a_failure_record_naming_the_pid() {
    let (layout, home, cwd) = scratch("no-start-time");
    let run_id = "00000000-0000-7000-8000-000000000171";
    let run_file = layout.run_file(run_id);
    let marker = cwd.join("session-exited");
    let mut child = session_that_leaves(&marker);
    let pid = child.id();
    let checkout = WorktreeRef {
        path: cwd.display().to_string(),
        branch: format!("hydra/{run_id}"),
        base_tree: None,
    };

    let err = hydra::interactive::record_then_wait(
        hydra::provider::Provider::Claude,
        &run_file,
        run_id,
        None,
        unfilled_start(&cwd),
        &mut child,
        Err(anyhow::anyhow!("sysctl said no")),
        Some(&checkout),
    )
    .expect_err("a start time that did not read is a failed launch");

    assert!(
        marker.exists(),
        "record_then_wait returned before the session exited"
    );
    let message = format!("{err:#}");
    assert!(message.contains(&format!("pid {pid}")), "{message}");
    assert!(message.contains(&checkout.path), "{message}");

    let text = std::fs::read_to_string(&run_file).expect("no record was written");
    let lines: Vec<Value> = text
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(lines.len(), 2, "{text}");
    assert_eq!(lines[0]["hydra"]["pid"], Value::Null);
    assert_eq!(lines[0]["hydra"]["process_start_time"], Value::Null);
    assert_eq!(lines[1]["hydra"]["type"], "run_end");
    let error = lines[1]["hydra"]["error"].as_str().unwrap();
    assert!(error.contains(&format!("pid {pid}")), "{error}");
    assert!(error.contains("sysctl said no"), "{error}");
    hydra::record::validate::check_file(&run_file)
        .unwrap()
        .assert_ok();

    let policies = hydra::policy::PolicySet::discover().unwrap();
    let run = hydra::normalize::read_file(&run_file, &policies).unwrap();
    let derived = hydra::outcome::derive(&run).unwrap();
    assert_eq!(derived.outcome, hydra::outcome::Outcome::Failed);
    assert_eq!(derived.source, hydra::outcome::Source::Derived);

    // Closed, so `hydra status` never harvests it as abandoned mid-session.
    assert_eq!(
        harvest_in(&layout, run_id, None, &home).unwrap(),
        HarvestOutcome::AlreadyDone
    );
}

/// A session that exits before its first turn writes no vendor transcript.
/// Harvest closes the record with an error naming the path it looked for.
#[test]
fn a_session_that_wrote_no_transcript_derives_failed() {
    let (layout, home, cwd) = scratch("no-transcript");
    let run_id = "00000000-0000-7000-8000-000000000174";
    let session = "00000000-0000-7000-8000-000000000174";
    open_interactive_run(&layout, run_id, session, &cwd);

    let outcome = harvest_in(&layout, run_id, Some(0), &home).unwrap();
    assert_eq!(outcome, HarvestOutcome::Harvested { records: 0 });

    let policies = hydra::policy::PolicySet::discover().unwrap();
    let run = hydra::normalize::read_file(&layout.run_file(run_id), &policies).unwrap();
    let error = run.run_end().unwrap().body["error"].as_str().unwrap();
    assert!(error.contains("wrote no transcript"), "{error}");

    let derived = hydra::outcome::derive(&run).unwrap();
    assert_eq!(derived.outcome, hydra::outcome::Outcome::Failed);
    assert_eq!(derived.source, hydra::outcome::Source::Derived);
}

/// Build ticket 112, item 1. A record that cannot be written at all is said
/// on stderr, and the launch still waits for the session it started.
#[test]
fn a_record_that_cannot_be_written_still_waits_for_the_session() {
    let (layout, _home, cwd) = scratch("no-record");
    let run_id = "00000000-0000-7000-8000-000000000172";
    let run_file = layout.run_file(run_id);
    // A directory where the record goes, so `Writer::create` fails.
    std::fs::create_dir_all(&run_file).unwrap();
    let marker = cwd.join("session-exited");
    let mut child = session_that_leaves(&marker);
    let pid = child.id();
    let start_time = hydra::procinfo::start_time(pid);

    let err = hydra::interactive::record_then_wait(
        hydra::provider::Provider::Claude,
        &run_file,
        run_id,
        None,
        unfilled_start(&cwd),
        &mut child,
        start_time,
        None,
    )
    .expect_err("a record that could not be written is a failed launch");

    assert!(
        marker.exists(),
        "record_then_wait returned before the session exited"
    );
    let message = format!("{err:#}");
    assert!(message.contains(&format!("pid {pid}")), "{message}");
    assert!(run_file.is_dir());
}

/// The path every launch takes: the pair lands on line 0 and the call returns
/// the session's own exit status.
#[test]
fn a_recorded_start_carries_the_pid_and_its_start_time() {
    let (layout, _home, cwd) = scratch("recorded-start");
    let run_id = "00000000-0000-7000-8000-000000000173";
    let run_file = layout.run_file(run_id);
    let marker = cwd.join("session-exited");
    let mut child = session_that_leaves(&marker);
    let pid = child.id();
    let start_time = hydra::procinfo::start_time(pid);

    let status = hydra::interactive::record_then_wait(
        hydra::provider::Provider::Claude,
        &run_file,
        run_id,
        None,
        unfilled_start(&cwd),
        &mut child,
        start_time,
        None,
    )
    .unwrap();

    assert!(status.success());
    assert!(marker.exists());
    let text = std::fs::read_to_string(&run_file).unwrap();
    let line: Value = serde_json::from_str(text.lines().next().unwrap()).unwrap();
    assert_eq!(line["hydra"]["pid"], json!(pid));
    assert!(line["hydra"]["process_start_time"].is_string(), "{text}");
    assert_eq!(text.lines().count(), 1);
}

/// Line 0 of an ungated interactive `codex` run: no session id, because
/// `codex` takes none, and the liveness pair of this test's own process, which
/// started before any session file below began.
fn open_codex_interactive_run(layout: &Layout, run_id: &str, cwd: &Path) {
    let mut start = unfilled_start(cwd);
    let pid = std::process::id();
    start.pid = Some(pid);
    start.process_start_time = hydra::procinfo::start_time(pid)
        .unwrap()
        .map(|t| t.to_rfc3339_opts(chrono::SecondsFormat::Micros, true));
    start.provider_version = Some("codex-cli 0.159.2".into());
    start.provider_session_id = None;
    start.subscription = Some("openai".into());
    let mut writer =
        Writer::create(layout.run_file(run_id), run_id, None, Some("codex"), None).unwrap();
    writer.hydra_line(&start).unwrap();
}

/// A session file in the shape `provider::codex` measured, every line
/// synthetic. `source` is `cli` for the TUI and `exec` for `codex exec`.
fn write_codex_session(
    home: &Path,
    cwd: &Path,
    id: &str,
    source: &str,
    began: chrono::DateTime<chrono::Utc>,
    lines: &[Value],
) -> PathBuf {
    let dir = hydra::provider::codex::sessions_dir(home)
        .join(chrono::Utc::now().format("%Y/%m/%d").to_string());
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(format!(
        "rollout-{}-{id}.jsonl",
        began.format("%Y-%m-%dT%H-%M-%S")
    ));
    let line = |ordinal: usize, kind: &str, payload: &Value| {
        json!({
            "timestamp": began.to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            "ordinal": ordinal,
            "type": kind,
            "payload": payload,
        })
        .to_string()
    };
    let meta = json!({
        "id": id,
        "session_id": id,
        "timestamp": began.to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
        "cwd": cwd.display().to_string(),
        "originator": if source == "exec" { "codex_exec" } else { "codex-tui" },
        "cli_version": "0.159.2",
        "source": source,
    });
    let mut text = vec![line(0, "session_meta", &meta)];
    for (at, body) in lines.iter().enumerate() {
        text.push(line(
            at + 1,
            body["type"].as_str().unwrap(),
            &body["payload"],
        ));
    }
    std::fs::write(&path, format!("{}\n", text.join("\n"))).unwrap();
    path
}

fn usage_of(input: u64, cached: u64, output: u64, reasoning: u64) -> Value {
    json!({
        "input_tokens": input,
        "cached_input_tokens": cached,
        "cache_write_input_tokens": 0,
        "output_tokens": output,
        "reasoning_output_tokens": reasoning,
        "total_tokens": input + output,
    })
}

/// Build ticket 255. A TUI session that began after the run, in its cwd, is
/// the one harvested; one that began before it and a `codex exec` in the same
/// directory are not. The usage is the session's last running total.
#[test]
fn a_codex_session_file_harvests_into_a_valid_record_with_its_last_total() {
    let (layout, home, cwd) = scratch("codex");
    let run_id = "01a103a8-0000-7000-8000-0000000002a1";
    open_codex_interactive_run(&layout, run_id, &cwd);
    let now = chrono::Utc::now();

    let turn = vec![
        json!({"type": "event_msg", "payload": {"type": "task_started", "turn_id": "t1"}}),
        json!({"type": "response_item", "payload": {
            "type": "message", "role": "developer",
            "content": [{"type": "input_text", "text": "the TUI's own instructions"}],
        }}),
        json!({"type": "response_item", "payload": {
            "type": "message", "role": "user",
            "content": [{"type": "input_text", "text": "list the files"}],
        }}),
        json!({"type": "turn_context", "payload": {"turn_id": "t1", "cwd": "elsewhere"}}),
        json!({"type": "response_item", "payload": {
            "type": "custom_tool_call", "call_id": "call_1", "name": "exec",
            "input": "ls", "status": "completed",
        }}),
        json!({"type": "response_item", "payload": {
            "type": "custom_tool_call_output", "call_id": "call_1",
            "output": [{"type": "input_text", "text": "a.txt"}],
        }}),
        json!({"type": "event_msg", "payload": {"type": "token_count", "info": {
            "total_token_usage": usage_of(1200, 1000, 80, 30),
            "last_token_usage": usage_of(1200, 1000, 80, 30),
        }}}),
        json!({"type": "response_item", "payload": {
            "type": "message", "role": "assistant",
            "content": [{"type": "output_text", "text": "There is one file, a.txt."}],
        }}),
        json!({"type": "token_usage_record", "payload": {
            "usage": usage_of(1300, 1000, 70, 30),
            "thread_token_usage": usage_of(2500, 2000, 150, 60),
        }}),
        json!({"type": "event_msg", "payload": {"type": "token_count", "info": null}}),
        json!({"type": "event_msg", "payload": {"type": "task_complete", "turn_id": "t1"}}),
    ];
    let mine = write_codex_session(
        &home,
        &cwd,
        "01a103a8-0000-7000-8000-0000000002a2",
        // What 0.159.2's terminal TUI writes, measured on the first real run.
        "vscode",
        now,
        &turn,
    );
    let before = turn[..3].to_vec();
    write_codex_session(
        &home,
        &cwd,
        "01a103a8-0000-7000-8000-0000000002a3",
        "cli",
        now - chrono::Duration::days(400),
        &before,
    );
    write_codex_session(
        &home,
        &cwd,
        "01a103a8-0000-7000-8000-0000000002a4",
        "exec",
        now - chrono::Duration::milliseconds(1),
        &before,
    );
    // A second TUI a human opened in the same directory while this one ran.
    write_codex_session(
        &home,
        &cwd,
        "01a103a8-0000-7000-8000-0000000002a5",
        "cli",
        now + chrono::Duration::seconds(5),
        &before,
    );

    let outcome = harvest_in(&layout, run_id, Some(0), &home).unwrap();
    assert_eq!(
        outcome,
        HarvestOutcome::Harvested {
            records: 1 + turn.len()
        },
        "{} should have been the one harvested",
        mine.display()
    );

    let text = std::fs::read_to_string(layout.run_file(run_id)).unwrap();
    hydra::record::validate::check(&text, Some(run_id)).assert_ok();

    let policies = hydra::policy::PolicySet::discover().unwrap();
    let run = hydra::normalize::read_file(&layout.run_file(run_id), &policies).unwrap();
    let error = run.run_end().unwrap().body["error"].as_str().unwrap();
    assert!(
        error.contains("0000000002a2, the first of 2"),
        "the later session is counted, not taken: {error}"
    );
    let usage = run
        .usage()
        .expect("a harvested codex run has a usage event");
    assert_eq!(usage.source, hydra::normalize::UsageSource::Provider);
    let counts = usage.counts.expect("the session file named its totals");
    // 2500 reported, 2000 of it cached: net of the cache, build ticket 259.
    assert_eq!(counts.input_tokens, 500);
    assert_eq!(counts.cache_read_tokens, 2000);
    assert_eq!(counts.output_tokens, 150);
    assert_eq!(counts.thinking_tokens, Some(60));
    assert_eq!(counts.cache_write_tokens, Some(0));

    let said: Vec<(Role, &str)> = run.messages().map(|m| (m.role, m.text.as_str())).collect();
    assert_eq!(
        said,
        [
            (Role::User, "list the files"),
            (Role::Assistant, "There is one file, a.txt."),
        ]
    );
    assert_eq!(run.tool_calls().count(), 1);
    assert_eq!(run.tool_results().count(), 1);

    let derived = hydra::outcome::derive(&run).unwrap();
    assert_eq!(derived.outcome, hydra::outcome::Outcome::Completed);
}

/// Build ticket 255. A `codex` session that never reached its first prompt
/// leaves no file, and the harvest closes the record with usage unknown.
#[test]
fn a_codex_run_with_no_session_file_ends_with_usage_unknown() {
    let (layout, home, cwd) = scratch("codex-none");
    let run_id = "01a103a8-0000-7000-8000-0000000002b1";
    open_codex_interactive_run(&layout, run_id, &cwd);

    let outcome = harvest_in(&layout, run_id, Some(0), &home).unwrap();
    assert_eq!(outcome, HarvestOutcome::Harvested { records: 0 });
    let text = std::fs::read_to_string(layout.run_file(run_id)).unwrap();
    hydra::record::validate::check(&text, Some(run_id)).assert_ok();

    let policies = hydra::policy::PolicySet::discover().unwrap();
    let run = hydra::normalize::read_file(&layout.run_file(run_id), &policies).unwrap();
    let error = run.run_end().unwrap().body["error"].as_str().unwrap();
    assert!(error.contains("no codex session file"), "{error}");
    let usage = run.usage().expect("an ended run still says what it knows");
    assert_eq!(usage.source, hydra::normalize::UsageSource::Unavailable);
    assert!(usage.counts.is_none());
    assert_eq!(
        hydra::outcome::derive(&run).unwrap().outcome,
        hydra::outcome::Outcome::Failed
    );
    assert_eq!(
        harvest_in(&layout, run_id, None, &home).unwrap(),
        HarvestOutcome::AlreadyDone
    );
}

/// Line 0 of an ungated interactive `cursor-agent` run: no chat id, because
/// the TUI takes none, and the liveness pair of this test's own process.
fn open_cursor_interactive_run(layout: &Layout, run_id: &str, cwd: &Path) {
    let mut start = unfilled_start(cwd);
    let pid = std::process::id();
    start.pid = Some(pid);
    start.process_start_time = hydra::procinfo::start_time(pid)
        .unwrap()
        .map(|t| t.to_rfc3339_opts(chrono::SecondsFormat::Micros, true));
    start.provider_version = Some("2026.10.01-e373342".into());
    start.provider_session_id = None;
    start.subscription = Some("cursor".into());
    let mut writer = Writer::create(
        layout.run_file(run_id),
        run_id,
        None,
        Some("cursor-agent"),
        None,
    )
    .unwrap();
    writer.hydra_line(&start).unwrap();
}

/// A blob's id the way the store names it: its SHA-256, in hex.
fn blob_id(data: &[u8]) -> String {
    use sha2::Digest;
    sha2::Sha256::digest(data)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// A state blob in the measured shape: first byte `0x0a`, the raw ids of the
/// messages so far in order.
fn state_blob(ids: &[&str]) -> Vec<u8> {
    let mut bytes = Vec::new();
    for id in ids {
        bytes.extend([0x0a, 0x20]);
        for at in (0..id.len()).step_by(2) {
            bytes.push(u8::from_str_radix(&id[at..at + 2], 16).unwrap());
        }
    }
    bytes
}

/// A chat directory in the shape `provider::cursor` measured: `meta.json`
/// and, when `blobs` is some, a `store.db` with its two tables, made by the
/// system's `sqlite3`. Every row is synthetic.
fn write_cursor_chat(
    home: &Path,
    cwd: &Path,
    chat: &str,
    created: chrono::DateTime<chrono::Utc>,
    blobs: Option<&[Vec<u8>]>,
) -> PathBuf {
    let dir = hydra::provider::cursor::chats_dir(home)
        .join("0123456789abcdef0123456789abcdef")
        .join(chat);
    std::fs::create_dir_all(&dir).unwrap();
    let meta = json!({
        "schemaVersion": 1,
        "createdAtMs": created.timestamp_millis(),
        "hasConversation": blobs.is_some(),
        "updatedAtMs": created.timestamp_millis() + 1000,
        "cwd": cwd.display().to_string(),
    });
    std::fs::write(dir.join("meta.json"), meta.to_string()).unwrap();
    let Some(blobs) = blobs else {
        return dir;
    };
    let mut sql = String::from(
        "CREATE TABLE blobs (id TEXT PRIMARY KEY, data BLOB);\n\
         CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT);\n\
         INSERT INTO meta VALUES ('k', 'v');\n",
    );
    for data in blobs {
        let hex: String = data.iter().map(|byte| format!("{byte:02x}")).collect();
        sql.push_str(&format!(
            "INSERT INTO blobs VALUES ('{}', X'{hex}');\n",
            blob_id(data)
        ));
    }
    let mut child = std::process::Command::new("sqlite3")
        .arg(dir.join("store.db"))
        .stdin(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    {
        use std::io::Write;
        child
            .stdin
            .take()
            .unwrap()
            .write_all(sql.as_bytes())
            .unwrap();
    }
    assert!(child.wait().unwrap().success());
    dir
}

/// Build ticket 266. The chat that began after the run, in its cwd, is the one
/// harvested; one from before the run and one in another directory are not.
/// The blobs go in out of order and come out in the order the latest state
/// blob names, and the protobuf blob that names nothing is counted unread.
#[test]
fn a_cursor_agent_chat_harvests_in_its_own_order_with_usage_unavailable() {
    let (layout, home, cwd) = scratch("cursor");
    let run_id = "01a1073a-0000-7000-8000-000000000266";
    open_cursor_interactive_run(&layout, run_id, &cwd);
    let now = chrono::Utc::now();

    let message = |body: Value| body.to_string().into_bytes();
    let system = message(json!({"role": "system", "content": "the TUI's own instructions"}));
    let user = message(json!({"role": "user", "content": "list the files"}));
    let call = message(json!({"role": "assistant", "id": "m1", "content": [
        {"type": "redacted-reasoning", "data": "opaque", "providerOptions": {}},
        {"type": "text", "text": "Looking."},
        {"type": "tool-call", "toolCallId": "tc1", "toolName": "run_terminal_cmd",
         "args": {"command": "ls"}},
    ]}));
    let result = message(json!({"role": "tool", "content": [
        {"type": "tool-result", "toolCallId": "tc1", "toolName": "run_terminal_cmd",
         "result": "a.txt", "experimental_content": []},
    ]}));
    let answer = message(json!({"role": "assistant", "content": [
        {"type": "text", "text": "There is one file, a.txt."},
    ]}));
    let ids: Vec<String> = [&system, &user, &call, &result, &answer]
        .iter()
        .map(|data| blob_id(data))
        .collect();
    let ids: Vec<&str> = ids.iter().map(String::as_str).collect();
    let blobs = vec![
        answer.clone(),
        state_blob(&ids[..2]),
        call.clone(),
        vec![0x12, 0x02, 0x08, 0x01],
        user.clone(),
        state_blob(&ids),
        result.clone(),
        system.clone(),
    ];
    let mine = write_cursor_chat(
        &home,
        &cwd,
        "0a1b2c3d-0000-4000-8000-000000000001",
        now,
        Some(&blobs),
    );
    let earlier = vec![user.clone()];
    write_cursor_chat(
        &home,
        &cwd,
        "0a1b2c3d-0000-4000-8000-000000000002",
        now - chrono::Duration::days(3),
        Some(&earlier),
    );
    let elsewhere = home.join("elsewhere");
    std::fs::create_dir_all(&elsewhere).unwrap();
    write_cursor_chat(
        &home,
        &elsewhere,
        "0a1b2c3d-0000-4000-8000-000000000003",
        now - chrono::Duration::milliseconds(1),
        Some(&earlier),
    );
    // A second TUI a human opened in the same directory while this one ran.
    write_cursor_chat(
        &home,
        &cwd,
        "0a1b2c3d-0000-4000-8000-000000000004",
        now + chrono::Duration::seconds(5),
        None,
    );

    let outcome = harvest_in(&layout, run_id, Some(0), &home).unwrap();
    assert_eq!(
        outcome,
        HarvestOutcome::Harvested { records: 5 },
        "{} should have been the one harvested",
        mine.display()
    );

    let text = std::fs::read_to_string(layout.run_file(run_id)).unwrap();
    hydra::record::validate::check(&text, Some(run_id)).assert_ok();

    let policies = hydra::policy::PolicySet::discover().unwrap();
    let run = hydra::normalize::read_file(&layout.run_file(run_id), &policies).unwrap();
    let error = run.run_end().unwrap().body["error"].as_str().unwrap();
    assert!(
        error.contains("000000000001, the first of 2"),
        "the later chat is counted, not taken: {error}"
    );
    assert!(
        error.contains("left unread, and any usage the chat recorded is among them: 1"),
        "{error}"
    );
    assert!(!error.contains("no state blob placed"), "{error}");

    let said: Vec<(Role, &str)> = run.messages().map(|m| (m.role, m.text.as_str())).collect();
    assert_eq!(
        said,
        [
            (Role::User, "list the files"),
            (Role::Assistant, "Looking."),
            (Role::Assistant, "There is one file, a.txt."),
        ]
    );
    let calls: Vec<_> = run.tool_calls().collect();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].call_id, "tc1");
    assert_eq!(calls[0].arguments, json!({"command": "ls"}));
    let results: Vec<_> = run.tool_results().collect();
    assert_eq!(results.len(), 1);
    assert_eq!(
        (results[0].call_id.as_str(), results[0].output.as_str()),
        ("tc1", "a.txt")
    );

    let usage = run.usage().expect("an ended run still says what it knows");
    assert_eq!(usage.source, hydra::normalize::UsageSource::Unavailable);
    assert!(usage.counts.is_none());
    assert_eq!(
        hydra::outcome::derive(&run).unwrap().outcome,
        hydra::outcome::Outcome::Completed
    );
    assert_eq!(
        harvest_in(&layout, run_id, None, &home).unwrap(),
        HarvestOutcome::AlreadyDone
    );
}

/// Build ticket 266. A `cursor-agent` session that left no chat closes the
/// record failed, saying where the harvest looked.
#[test]
fn a_cursor_agent_run_with_no_chat_derives_failed() {
    let (layout, home, cwd) = scratch("cursor-none");
    let run_id = "01a1073a-0000-7000-8000-000000000267";
    open_cursor_interactive_run(&layout, run_id, &cwd);
    // A chat from before the run started is not this run's.
    write_cursor_chat(
        &home,
        &cwd,
        "0a1b2c3d-0000-4000-8000-000000000005",
        chrono::Utc::now() - chrono::Duration::days(1),
        None,
    );

    let outcome = harvest_in(&layout, run_id, Some(0), &home).unwrap();
    assert_eq!(outcome, HarvestOutcome::Harvested { records: 0 });
    let text = std::fs::read_to_string(layout.run_file(run_id)).unwrap();
    hydra::record::validate::check(&text, Some(run_id)).assert_ok();

    let policies = hydra::policy::PolicySet::discover().unwrap();
    let run = hydra::normalize::read_file(&layout.run_file(run_id), &policies).unwrap();
    let error = run.run_end().unwrap().body["error"].as_str().unwrap();
    assert!(error.contains("no cursor-agent chat"), "{error}");
    assert_eq!(
        hydra::outcome::derive(&run).unwrap().outcome,
        hydra::outcome::Outcome::Failed
    );
}

/// Line 0 of an ungated interactive `agy` run: no conversation id, because
/// the TUI takes none, and the liveness pair of this test's own process.
fn open_agy_interactive_run(layout: &Layout, run_id: &str, cwd: &Path) {
    let mut start = unfilled_start(cwd);
    let pid = std::process::id();
    start.pid = Some(pid);
    start.process_start_time = hydra::procinfo::start_time(pid)
        .unwrap()
        .map(|t| t.to_rfc3339_opts(chrono::SecondsFormat::Micros, true));
    start.provider_version = Some("1.2.16".into());
    start.provider_session_id = None;
    start.subscription = Some("google".into());
    let mut writer =
        Writer::create(layout.run_file(run_id), run_id, None, Some("agy"), None).unwrap();
    writer.hydra_line(&start).unwrap();
}

/// A protobuf-shaped metadata blob naming `workspace` as field 1, with a
/// second field after it, the way the session measured the path: plain bytes
/// inside the blob.
fn workspace_blob(workspace: &str) -> Vec<u8> {
    let mut bytes = vec![0x0a];
    let mut len = workspace.len();
    loop {
        let low = (len & 0x7f) as u8;
        len >>= 7;
        if len == 0 {
            bytes.push(low);
            break;
        }
        bytes.push(low | 0x80);
    }
    bytes.extend(workspace.as_bytes());
    bytes.extend([0x12, 0x02, 0x08, 0x01]);
    bytes
}

/// One conversation in the shape `provider::agy` measured: a
/// `conversations/<uuid>.db` made by the system's `sqlite3` holding
/// `trajectory_metadata_blob`, and, when `transcript` is some, a
/// `brain/<uuid>/` with the transcript and one `output.txt` per step in
/// `outputs`. Every row and line is synthetic.
fn write_agy_conversation(
    home: &Path,
    id: &str,
    workspace: &str,
    transcript: Option<&[Value]>,
    outputs: &[(u64, &str)],
) {
    let root = hydra::provider::agy::data_dir(home);
    let conversations = root.join("conversations");
    std::fs::create_dir_all(&conversations).unwrap();
    let hex: String = workspace_blob(workspace)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    let sql = format!(
        "CREATE TABLE trajectory_metadata_blob (id text default \"main\", data blob);\n\
         INSERT INTO trajectory_metadata_blob (data) VALUES (X'{hex}');\n"
    );
    let mut child = std::process::Command::new("sqlite3")
        .arg(conversations.join(format!("{id}.db")))
        .stdin(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    {
        use std::io::Write;
        child
            .stdin
            .take()
            .unwrap()
            .write_all(sql.as_bytes())
            .unwrap();
    }
    assert!(child.wait().unwrap().success());

    let Some(transcript) = transcript else {
        return;
    };
    let brain = root.join("brain").join(id);
    let path = hydra::provider::agy::transcript_path(&brain);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let text: String = transcript.iter().map(|line| format!("{line}\n")).collect();
    std::fs::write(&path, text).unwrap();
    for (step, output) in outputs {
        let path = hydra::provider::agy::step_output_path(&brain, *step);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, output).unwrap();
    }
}

/// One transcript line in the measured shape.
fn agy_line(step: u64, kind: &str, at: chrono::DateTime<chrono::Utc>, extra: Value) -> Value {
    let source = match kind {
        "USER_INPUT" => "USER_EXPLICIT",
        "PLANNER_RESPONSE" => "MODEL",
        _ => "SYSTEM",
    };
    let mut line = json!({
        "type": kind,
        "source": source,
        "status": "DONE",
        "step_index": step,
        "created_at": at.to_rfc3339_opts(chrono::SecondsFormat::Micros, true),
        "content": null,
        "thinking": null,
        "tool_calls": null,
        "error": null,
    });
    for (key, value) in extra.as_object().unwrap() {
        line[key] = value.clone();
    }
    line
}

/// Build ticket 267. The conversation that began after the run, whose
/// metadata names the run's cwd exactly, is the one harvested; one from
/// before the run, one in a sibling directory whose path starts with the
/// cwd's, and a later one in the same cwd are not. Each step's output lands
/// after its step and answers the calls in order, the step that made two
/// calls is named, and usage is unavailable.
#[test]
fn an_agy_conversation_harvests_with_its_step_outputs_joined_by_order() {
    let (layout, home, cwd) = scratch("agy");
    let run_id = "01a1073a-0000-7000-8000-000000000268";
    open_agy_interactive_run(&layout, run_id, &cwd);
    let now = chrono::Utc::now();
    let cwd_text = cwd.display().to_string();

    let transcript = vec![
        agy_line(
            0,
            "SYSTEM_MESSAGE",
            now,
            json!({"content": "the TUI's own"}),
        ),
        agy_line(1, "USER_INPUT", now, json!({"content": "list the files"})),
        agy_line(
            2,
            "PLANNER_RESPONSE",
            now,
            json!({
                "content": "Looking.",
                "thinking": "two calls will do",
                "tool_calls": [
                    {"name": "run_command", "args": {"CommandLine": "ls"}},
                    {"name": "view_file", "args": {"AbsolutePath": "/x/a.txt"}},
                ],
            }),
        ),
        agy_line(3, "GENERIC", now, json!({})),
        agy_line(4, "GENERIC", now, json!({})),
        agy_line(
            5,
            "PLANNER_RESPONSE",
            now,
            json!({"content": "There is one file, a.txt."}),
        ),
        agy_line(6, "CHECKPOINT", now, json!({})),
    ];
    let mine = "0a1b2c3d-0000-4000-8000-000000000011";
    write_agy_conversation(
        &home,
        mine,
        &format!("file://{cwd_text}"),
        Some(&transcript),
        &[(3, "a.txt"), (4, "hello")],
    );
    let earlier = now - chrono::Duration::days(3);
    write_agy_conversation(
        &home,
        "0a1b2c3d-0000-4000-8000-000000000012",
        &cwd_text,
        Some(&[agy_line(
            0,
            "USER_INPUT",
            earlier,
            json!({"content": "old"}),
        )]),
        &[],
    );
    let sibling = format!("{cwd_text}-two");
    write_agy_conversation(
        &home,
        "0a1b2c3d-0000-4000-8000-000000000013",
        &sibling,
        Some(&[agy_line(
            0,
            "USER_INPUT",
            now - chrono::Duration::milliseconds(1),
            json!({"content": "elsewhere"}),
        )]),
        &[],
    );
    // A second TUI a human opened in the same directory while this one ran.
    let later = now + chrono::Duration::seconds(5);
    write_agy_conversation(
        &home,
        "0a1b2c3d-0000-4000-8000-000000000014",
        &cwd_text,
        Some(&[agy_line(
            0,
            "USER_INPUT",
            later,
            json!({"content": "later"}),
        )]),
        &[],
    );

    let outcome = harvest_in(&layout, run_id, Some(0), &home).unwrap();
    assert_eq!(
        outcome,
        HarvestOutcome::Harvested { records: 9 },
        "seven transcript lines and two step outputs"
    );

    let text = std::fs::read_to_string(layout.run_file(run_id)).unwrap();
    hydra::record::validate::check(&text, Some(run_id)).assert_ok();

    let policies = hydra::policy::PolicySet::discover().unwrap();
    let run = hydra::normalize::read_file(&layout.run_file(run_id), &policies).unwrap();
    let error = run.run_end().unwrap().body["error"].as_str().unwrap();
    assert!(
        error.contains("000000000011, the first of 2"),
        "the later conversation is counted, not taken: {error}"
    );
    assert!(
        error
            .contains("joined to calls by step order because the transcript carries no call id: 1"),
        "{error}"
    );
    assert!(
        !error.contains("no transcript line for their step"),
        "{error}"
    );

    let said: Vec<(Role, &str)> = run.messages().map(|m| (m.role, m.text.as_str())).collect();
    assert_eq!(
        said,
        [
            (Role::User, "list the files"),
            (Role::Assistant, "Looking."),
            (Role::Assistant, "There is one file, a.txt."),
        ]
    );
    let thought: Vec<&str> = run
        .events
        .iter()
        .filter_map(|event| match event {
            Event::Thinking(thinking) => Some(thinking.text.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(thought, ["two calls will do"]);
    let calls: Vec<(&str, &str)> = run
        .tool_calls()
        .map(|call| (call.call_id.as_str(), call.name.as_str()))
        .collect();
    assert_eq!(calls, [("2.0", "run_command"), ("2.1", "view_file")]);
    let results: Vec<(&str, &str)> = run
        .tool_results()
        .map(|result| (result.call_id.as_str(), result.output.as_str()))
        .collect();
    assert_eq!(results, [("2.0", "a.txt"), ("2.1", "hello")]);
    assert!(
        run.findings.iter().any(|finding| matches!(
            finding,
            Finding::Limit(text) if text.contains("a guess by order")
        )),
        "{:?}",
        run.findings
    );

    let usage = run.usage().expect("an ended run still says what it knows");
    assert_eq!(usage.source, hydra::normalize::UsageSource::Unavailable);
    assert!(usage.counts.is_none());
    assert_eq!(
        hydra::outcome::derive(&run).unwrap().outcome,
        hydra::outcome::Outcome::Completed
    );
    assert_eq!(
        harvest_in(&layout, run_id, None, &home).unwrap(),
        HarvestOutcome::AlreadyDone
    );
}

/// Build ticket 267. A session that left no conversation in its cwd closes
/// the record failed, saying where the harvest looked.
#[test]
fn an_agy_run_with_no_conversation_derives_failed() {
    let (layout, home, cwd) = scratch("agy-none");
    let run_id = "01a1073a-0000-7000-8000-000000000269";
    open_agy_interactive_run(&layout, run_id, &cwd);
    // A conversation from before the run started is not this run's.
    let earlier = chrono::Utc::now() - chrono::Duration::days(1);
    write_agy_conversation(
        &home,
        "0a1b2c3d-0000-4000-8000-000000000015",
        &cwd.display().to_string(),
        Some(&[agy_line(
            0,
            "USER_INPUT",
            earlier,
            json!({"content": "old"}),
        )]),
        &[],
    );

    let outcome = harvest_in(&layout, run_id, Some(0), &home).unwrap();
    assert_eq!(outcome, HarvestOutcome::Harvested { records: 0 });
    let text = std::fs::read_to_string(layout.run_file(run_id)).unwrap();
    hydra::record::validate::check(&text, Some(run_id)).assert_ok();

    let policies = hydra::policy::PolicySet::discover().unwrap();
    let run = hydra::normalize::read_file(&layout.run_file(run_id), &policies).unwrap();
    let error = run.run_end().unwrap().body["error"].as_str().unwrap();
    assert!(error.contains("no agy conversation"), "{error}");
    assert_eq!(
        hydra::outcome::derive(&run).unwrap().outcome,
        hydra::outcome::Outcome::Failed
    );
}

/// Build ticket 267. A conversation with no transcript yet is found by its
/// store's file time and its cwd, and closes the record saying it holds no
/// transcript rather than that there is no conversation.
#[test]
fn an_agy_conversation_with_no_transcript_yet_derives_failed() {
    let (layout, home, cwd) = scratch("agy-empty");
    let run_id = "01a1073a-0000-7000-8000-000000000270";
    open_agy_interactive_run(&layout, run_id, &cwd);
    write_agy_conversation(
        &home,
        "0a1b2c3d-0000-4000-8000-000000000016",
        &cwd.display().to_string(),
        None,
        &[],
    );

    let outcome = harvest_in(&layout, run_id, Some(0), &home).unwrap();
    assert_eq!(outcome, HarvestOutcome::Harvested { records: 0 });
    let policies = hydra::policy::PolicySet::discover().unwrap();
    let run = hydra::normalize::read_file(&layout.run_file(run_id), &policies).unwrap();
    let error = run.run_end().unwrap().body["error"].as_str().unwrap();
    assert!(error.contains("000000000016 has no transcript"), "{error}");
    assert_eq!(
        hydra::outcome::derive(&run).unwrap().outcome,
        hydra::outcome::Outcome::Failed
    );
}
