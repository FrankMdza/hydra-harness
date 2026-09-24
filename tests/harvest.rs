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

/// Build ticket 107, item 1. The start time is read after `claude` is
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

    // Closed, so `hydra status` never harvests it as abandoned mid-session.
    assert_eq!(
        harvest_in(&layout, run_id, None, &home).unwrap(),
        HarvestOutcome::AlreadyDone
    );
}

/// Build ticket 107, item 1. A record that cannot be written at all is said
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
