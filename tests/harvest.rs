//! `hydra run --interactive`'s harvest, harness spec 3 and 10, contract 7.9.
//!
//! Every test here builds a vendor transcript by hand, in the shape map ticket
//! 38 measured on real `~/.claude/projects/` files, and points
//! [`hydra::interactive::harvest_in`] at a scratch home directory rather than
//! the developer's own. No provider is launched.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use hydra::interactive::{HarvestOutcome, harvest_in};
use hydra::normalize::{Event, Role};
use hydra::paths::Layout;
use hydra::provider::claude::transcript_path;
use hydra::record::{Auth, RunStart, SCHEMA_VERSION, Writer};
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
    };
    let mut writer =
        Writer::create(layout.run_file(run_id), run_id, None, Some("claude"), None).unwrap();
    writer.hydra_line(&start).unwrap();
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

fn assistant_tool_use(uuid: &str, parent: &str, tool_use_id: &str, name: &str, input: Value) -> String {
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
        record(
            "a4",
            Some("a3"),
            "file-history-snapshot",
            json!({}),
        ),
        human_words("a5", "a4", "use printf to write the word hello into it instead"),
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
    assert!(matches!(outcome, HarvestOutcome::Harvested { records: 6 }), "{outcome:?}");

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
    assert_eq!(correction.rejected, json!({"command": "touch t35probe.txt"}));
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
        &[assistant_tool_use("f1", "root", "toolu_1", "Bash", json!({"command": "echo hi"}))],
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
        assistant_tool_use("g1", "root", "toolu_1", "Bash", json!({"command": "rm -rf /tmp/x"})),
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
        !run.events.iter().any(|event| matches!(event, Event::Correction(_))),
        "an argued reply is a rejection without a correction"
    );
}
