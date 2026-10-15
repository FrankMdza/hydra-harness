//! The fixture corpus: synthetic records CI's record walks read, build ticket 101.
//!
//! The walks in `record_contract.rs`, `normalize.rs` and `outcome.rs` read every
//! record under `runs/`. On the laptop that is the real corpus; in CI it is an
//! empty scratch root, so a green CI said the readers compile and nothing more.
//! That was the third hiring panel's H14. The fixture walks beside each live
//! walk read `tests/fixtures/records/` instead, with the same assertions.
//!
//! **The fixtures are built, not copied.** [`write_the_fixture_corpus`] writes
//! them from the record builders in `tests/common`, whose frame shapes were
//! measured on the binary each directory names. No byte comes from a real record,
//! so no prompt, path or credential from `~/hydra/runs` can reach the repository.
//! [`no_fixture_carries_a_real_path_a_secret_or_a_live_run_id`] holds that.
//!
//! **One directory per binary version**, `<provider>@<version>`, and `harness`
//! for a record with no provider. A reader change that breaks one version's
//! shape names the version in the failing path. When a provider's shape moves,
//! the new shape goes in a new directory beside the old one, because records in
//! the old shape stay on disk.
//!
//! **Apart from the live corpus.** No walk reads both directories, and every
//! fixture run id starts with [`common::FIXTURE_RUN_PREFIX`], a timestamp no
//! launch mints.
//!
//! To rewrite the corpus after a builder changes:
//! `cargo test --test fixture_corpus -- --ignored write_the_fixture_corpus`,
//! then read the diff.

mod common;

use common::{
    FIXTURE_ROOT, FIXTURE_RUN_PREFIX, Record, Shape, agy_usage, cursor_read_ok, cursor_usage,
    fixture_corpus, fixture_dir, fixture_layout, scopes_of, usage_frame,
};
use serde_json::{Value, json};

/// One fixture: where it goes, what binary wrote its shape, and the record.
struct Fixture {
    label: &'static str,
    /// The provider's `--version` output as a launch records it, contract 5.1.
    provider_version: Option<&'static str>,
    subscription: Option<&'static str>,
    text: String,
}

fn fixture_id(n: u32) -> String {
    format!("{FIXTURE_RUN_PREFIX}{n:012}")
}

/// The `policy_sha` every gated fixture carries. Not the shipped set's sha,
/// which moves whenever a policy does: a fixture is a record from some earlier
/// day, and the readers have to take one whose set has since moved.
const FIXTURE_POLICY_SHA: &str = "f1f1f1f1f1f1f1f1f1f1f1f1f1f1f1f1f1f1f1f1f1f1f1f1f1f1f1f1f1f1f1f1";

const CLAUDE: &str = "claude@2.1.261";
const CLAUDE_VERSION: &str = "2.1.261 (Claude Code)";
const CURSOR: &str = "cursor-agent@2026.09.02-c22c1a3";
const AGY: &str = "agy@1.1.26";
const CODEX: &str = "codex@0.154.0";
const HARNESS: &str = "harness";

/// Every fixture the corpus holds, built from the shared record builders.
fn fixtures() -> Vec<Fixture> {
    let layout = fixture_layout();
    let sha = FIXTURE_POLICY_SHA.to_string();
    let claude = |text: String| Fixture {
        label: CLAUDE,
        provider_version: Some(CLAUDE_VERSION),
        subscription: Some("anthropic"),
        text,
    };
    let mut out = Vec::new();

    // Hook evidence is independent of a provider's possibly truncated command.
    let id = fixture_id(321);
    let mut evidence = Record::shaped(Shape {
        run_id: &id,
        policy: Some("judge"),
        policy_sha: Some(&sha),
        ..Shape::default()
    });
    evidence.line(
        "hydra",
        json!({
            "type": "gate_refusal", "tool": "Bash", "call_id": "toolu_321",
            "command": "exit 0; cat > probe.txt <<'EOF'\nwhole heredoc body\nEOF",
            "command_truncated": false,
        "reason": "hydra/capability: policy judge does not allow shell (Bash).",
        }),
    );
    out.push(claude(evidence.result(usage_frame()).closed()));

    // A read-only run asks for one `Write` and the gate refuses it: the shape
    // ticket 02 measured, and the one gated run the corpus holds.
    let id = fixture_id(1);
    let worktree = layout.worktree(&id);
    out.push(claude(
        Record::shaped(Shape {
            run_id: &id,
            policy: Some("read-only"),
            policy_sha: Some(&sha),
            worktree: Some(&worktree),
            store_sha: Some("1f0dcafe1f0dcafe1f0dcafe1f0dcafe1f0dcafe"),
            ..Shape::default()
        })
        .init(&["Bash", "Read", "Write"])
        .tool_use(
            "toolu_01",
            "Write",
            json!({"file_path": worktree.join("probe.txt"), "content": "HELLO\n"}),
        )
        .hook_response(
            "Write",
            "deny",
            "hydra/capability: policy read-only does not allow file_write (Write).",
        )
        .tool_result(
            "toolu_01",
            "hydra/capability: policy read-only does not allow file_write (Write).",
            true,
        )
        .assistant(json!([{"type": "text", "text": "BLOCKED"}]))
        .result(usage_frame())
        .closed(),
    ));

    // A standard run with two `Bash` calls: one names the corpus and is
    // refused, one writes inside the worktree and is allowed. Ticket 06's
    // measurement, and the one fixture that carries `scopes`.
    let id = fixture_id(2);
    let worktree = layout.worktree(&id);
    let corpus_write = format!("echo x >> {FIXTURE_ROOT}/runs/r.jsonl");
    out.push(claude(
        Record::shaped(Shape {
            run_id: &id,
            policy: Some("standard"),
            policy_sha: Some(&sha),
            worktree: Some(&worktree),
            scopes: Some(scopes_of(&layout, &id, "standard", &worktree)),
            floor_lifted: Some(false),
            ..Shape::default()
        })
        .init(&["Bash", "Read", "Write"])
        .tool_use("toolu_01", "Bash", json!({"command": corpus_write}))
        .hook_response(
            "Bash",
            "deny",
            &format!(
                "hydra/protected-path: the command names {FIXTURE_ROOT}/runs, denied under \
                 every policy by harness spec 7.5."
            ),
        )
        .tool_result("toolu_01", "hydra/protected-path: refused", true)
        .tool_use(
            "toolu_02",
            "Bash",
            json!({"command": "echo ok > probe.txt"}),
        )
        .hook_response(
            "Bash",
            "allow",
            "hydra/allow: policy standard allows shell.",
        )
        .tool_result("toolu_02", "", false)
        .result(usage_frame())
        .closed(),
    ));

    // A run from before the gate reached a provider: no policy, no decision.
    let id = fixture_id(3);
    out.push(claude(
        Record::shaped(Shape {
            run_id: &id,
            worktree: Some(&layout.worktree(&id)),
            ..Shape::default()
        })
        .init(&["Bash", "Read"])
        .assistant(json!([{"type": "text", "text": "pong"}]))
        .result(usage_frame())
        .closed(),
    ));

    // The provider refused its own service: `api_error_status` on the
    // terminator, exit 1, and an outcome rung one never learns from.
    let id = fixture_id(4);
    out.push(claude(
        Record::shaped(Shape {
            run_id: &id,
            policy: Some("standard"),
            policy_sha: Some(&sha),
            worktree: Some(&layout.worktree(&id)),
            ..Shape::default()
        })
        .init(&["Bash", "Read"])
        .result(json!({
            "is_error": true,
            "api_error_status": 429,
            "result": "API Error: 429 rate limited",
        }))
        .closed_with(json!({"exit_code": 1})),
    ));

    // A launch Hydra refused: two lines, no stream, a refusal code.
    let id = fixture_id(5);
    out.push(claude(
        Record::shaped(Shape {
            run_id: &id,
            policy: Some("read-only"),
            policy_sha: Some(&sha),
            pid: None,
            process_start_time: None,
            ..Shape::default()
        })
        .closed_with(json!({
            "exit_code": null,
            "refusal": "policy_narrowing",
            "remediation": "name a policy the parent's contains",
            "body_source": null,
        })),
    ));

    // A run still being written: no `run_end`, which contract 6 says is not
    // corrupt, and which has no outcome yet.
    let id = fixture_id(6);
    out.push(claude(
        Record::shaped(Shape {
            run_id: &id,
            policy: Some("standard"),
            policy_sha: Some(&sha),
            worktree: Some(&layout.worktree(&id)),
            ..Shape::default()
        })
        .init(&["Bash", "Read"])
        .assistant(json!([{"type": "text", "text": "still going"}]))
        .unclosed(),
    ));

    // `cursor-agent` reads one file under `standard` and answers.
    let id = fixture_id(7);
    let worktree = layout.worktree(&id);
    let readme = worktree.join("README").display().to_string();
    out.push(Fixture {
        label: CURSOR,
        provider_version: Some("2026.09.02-c22c1a3"),
        subscription: Some("cursor"),
        text: Record::shaped(Shape {
            run_id: &id,
            provider: Some("cursor-agent"),
            policy: Some("standard"),
            policy_sha: Some(&sha),
            worktree: Some(&worktree),
            ..Shape::default()
        })
        .cursor_init()
        .cursor_user("read the README")
        .cursor_call(
            "c1",
            "readToolCall",
            json!({"path": readme}),
            cursor_read_ok(&readme, "hi\n"),
        )
        .cursor_assistant("It says hi.")
        .cursor_result("It says hi.", Some(cursor_usage()))
        .closed(),
    });

    // `agy` answers without a tool.
    let id = fixture_id(8);
    out.push(Fixture {
        label: AGY,
        provider_version: Some("1.1.26"),
        subscription: Some("google"),
        text: Record::shaped(Shape {
            run_id: &id,
            provider: Some("agy"),
            policy: Some("standard"),
            policy_sha: Some(&sha),
            worktree: Some(&layout.worktree(&id)),
            ..Shape::default()
        })
        .agy_init(&["view_file", "run_command"])
        .agy_user()
        .agy_response(1, &["po", "ng"], Some(agy_usage(12, 4, 0, 0)))
        .agy_result("SUCCESS", "pong", Some(agy_usage(12, 4, 0, 0)))
        .closed(),
    });

    // `codex` runs one command and answers.
    let id = fixture_id(9);
    out.push(Fixture {
        label: CODEX,
        provider_version: Some("codex-cli 0.154.0"),
        subscription: Some("openai"),
        text: Record::shaped(Shape {
            run_id: &id,
            provider: Some("codex"),
            policy: Some("standard"),
            policy_sha: Some(&sha),
            worktree: Some(&layout.worktree(&id)),
            ..Shape::default()
        })
        .raw(json!({"type": "thread.started", "thread_id": "fixture-thread"}))
        .raw(json!({"type": "turn.started"}))
        .raw(json!({"type": "item.completed", "item": {
            "type": "command_execution", "id": "c1", "command": "echo hi",
            "output": "hi\n", "exit_code": 0,
        }}))
        .raw(json!({"type": "item.completed", "item": {
            "type": "agent_message", "id": "m1", "text": "hi",
        }}))
        .raw(json!({"type": "turn.completed", "usage": {
            "input_tokens": 10, "cached_input_tokens": 0, "output_tokens": 4,
        }}))
        .closed(),
    });

    // `codex` stopped on the stream with no turn terminator: command items and
    // one `agent_message`, `run_end` exit 0. Run 01a0d484's shape, 2026-09-24.
    let id = fixture_id(11);
    out.push(Fixture {
        label: CODEX,
        provider_version: Some("codex-cli 0.154.0"),
        subscription: Some("openai"),
        text: Record::shaped(Shape {
            run_id: &id,
            provider: Some("codex"),
            policy: Some("standard"),
            policy_sha: Some(&sha),
            worktree: Some(&layout.worktree(&id)),
            ..Shape::default()
        })
        .raw(json!({"type": "thread.started", "thread_id": "fixture-thread-b"}))
        .raw(json!({"type": "turn.started"}))
        .raw(json!({"type": "item.completed", "item": {
            "type": "command_execution", "id": "c1", "command": "ls",
            "aggregated_output": "README\n", "exit_code": 0,
        }}))
        .raw(json!({"type": "item.completed", "item": {
            "type": "command_execution", "id": "c2", "command": "head README",
            "aggregated_output": "probe\n", "exit_code": 0,
        }}))
        .raw(json!({"type": "item.completed", "item": {
            "type": "agent_message", "id": "m1", "text": "still reading",
        }}))
        .closed(),
    });

    // `cursor-agent` searches the web under `judge` without asking the hook.
    let id = fixture_id(12);
    let worktree = layout.worktree(&id);
    out.push(Fixture {
        label: CURSOR,
        provider_version: Some("2026.09.02-c22c1a3"),
        subscription: Some("cursor"),
        text: Record::shaped(Shape {
            run_id: &id,
            provider: Some("cursor-agent"),
            policy: Some("judge"),
            policy_sha: Some(&sha),
            worktree: Some(&worktree),
            ..Shape::default()
        })
        .cursor_init()
        .cursor_user("search the web for tokio")
        .cursor_call(
            "tool_judge",
            "webSearchToolCall",
            json!({
                "searchTerm": "rust async runtime comparison",
                "toolCallId": "tool_judge"
            }),
            json!({"success": {
                "references": [{
                    "title": "Web search results",
                    "url": "",
                    "chunk": "Links:\n1. [Tokio](https://tokio.rs)\n"
                }]
            }}),
        )
        .cursor_assistant("Found Tokio.")
        .cursor_result("Found Tokio.", Some(cursor_usage()))
        .closed(),
    });

    // A `hydra evolve` pass: contract 5.5's two lines, no provider.
    let id = fixture_id(10);
    out.push(Fixture {
        label: HARNESS,
        provider_version: None,
        subscription: None,
        text: Record::shaped(Shape {
            run_id: &id,
            provider: None,
            command: "evolve",
            pid: None,
            process_start_time: None,
            ..Shape::default()
        })
        .closed_with(json!({"body_source": null})),
    });

    out
}

/// Line 0 the way a launch of that binary writes it: the version string, the
/// subscription, and for a harness run the fields contract 5.5 leaves null.
fn finish(fixture: &Fixture) -> String {
    let mut lines: Vec<Value> = fixture
        .text
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let start = &mut lines[0]["hydra"];
    start["provider_version"] = json!(fixture.provider_version);
    start["subscription"] = json!(fixture.subscription);
    if fixture.label == HARNESS {
        start["launch_mode"] = Value::Null;
        start["task"] = Value::Null;
        start["cwd"] = json!(format!("{FIXTURE_ROOT}/repo"));
        start["may_edit"] = json!(["src/normalize", "src/show.rs"]);
    }
    lines.iter().map(|line| format!("{line}\n")).collect()
}

#[test]
#[ignore = "rewrites tests/fixtures/records; run by hand and read the diff"]
fn write_the_fixture_corpus() {
    let dir = fixture_dir();
    let _ = std::fs::remove_dir_all(&dir);
    for fixture in fixtures() {
        let text = finish(&fixture);
        let first: Value = serde_json::from_str(text.lines().next().unwrap()).unwrap();
        let run_id = first["run_id"].as_str().unwrap();
        let label = dir.join(fixture.label);
        std::fs::create_dir_all(&label).unwrap();
        std::fs::write(label.join(format!("{run_id}.jsonl")), text).unwrap();
    }
}

#[test]
fn the_fixture_corpus_preserves_hook_refusal_evidence() {
    let expected = finish(
        &fixtures()
            .into_iter()
            .find(|fixture| fixture.text.contains("\"gate_refusal\""))
            .unwrap(),
    );
    let path = fixture_dir()
        .join(CLAUDE)
        .join(format!("{}.jsonl", fixture_id(321)));
    assert_eq!(std::fs::read_to_string(path).unwrap(), expected);
    hydra::record::validate::check(&expected, None).assert_ok();
}

/// The label a record's own line 0 says it belongs under.
fn label_of(first: &Value) -> String {
    match first["provider"].as_str() {
        None => HARNESS.to_string(),
        Some(provider) => {
            let version = first["hydra"]["provider_version"]
                .as_str()
                .unwrap_or_default()
                .split_whitespace()
                .find(|token| token.starts_with(|c: char| c.is_ascii_digit()))
                .unwrap_or_default()
                .to_string();
            format!("{provider}@{version}")
        }
    }
}

#[test]
fn every_fixture_sits_under_the_binary_version_that_wrote_its_shape() {
    let records = fixture_corpus();
    assert!(
        !records.is_empty(),
        "no fixtures under {}",
        fixture_dir().display()
    );
    for path in records {
        let text = std::fs::read_to_string(&path).unwrap();
        let first: Value = serde_json::from_str(text.lines().next().unwrap()).unwrap();
        let dir = path
            .parent()
            .unwrap()
            .file_name()
            .unwrap()
            .to_string_lossy();
        assert_eq!(
            dir,
            label_of(&first),
            "{} is filed under a label its line 0 does not carry",
            path.display()
        );
        assert_eq!(
            path.file_stem().unwrap().to_string_lossy(),
            first["run_id"].as_str().unwrap(),
            "{}",
            path.display()
        );
    }
}

#[test]
fn the_fixture_corpus_holds_every_routed_provider_and_the_harness() {
    let labels: Vec<String> = fixture_corpus()
        .iter()
        .map(|path| {
            path.parent()
                .unwrap()
                .file_name()
                .unwrap()
                .to_string_lossy()
                .to_string()
        })
        .collect();
    for prefix in ["claude@", "cursor-agent@", "agy@", "codex@", HARNESS] {
        assert!(
            labels.iter().any(|label| label.starts_with(prefix)),
            "no fixture under {prefix}: {labels:?}"
        );
    }
}

/// Absolute paths a fixture may name: the fixture root, and the generic system
/// directories a frame names on any machine.
const ALLOWED_ROOTS: &[&str] = &["/fixture", "/tmp", "/bin", "/usr"];

/// Every string anywhere in a JSON value.
fn strings<'a>(value: &'a Value, out: &mut Vec<&'a str>) {
    match value {
        Value::String(text) => out.push(text),
        Value::Array(items) => items.iter().for_each(|item| strings(item, out)),
        Value::Object(map) => map.values().for_each(|item| strings(item, out)),
        _ => (),
    }
}

#[test]
fn no_fixture_carries_a_real_path_a_secret_or_a_live_run_id() {
    let home = hydra::env::real_home().ok();
    for path in fixture_corpus() {
        let text = std::fs::read_to_string(&path).unwrap();
        let shown = path.display();
        for (index, line) in text.lines().enumerate() {
            assert_eq!(
                hydra::record::secrets::scan(line),
                None,
                "{shown}:{}: the secret scan flags this line",
                index + 1
            );
            for marker in ["/Users/", "/home/", "/var/folders/", "/private/"] {
                assert!(
                    !line.contains(marker),
                    "{shown}:{}: names {marker}, a path off a real machine",
                    index + 1
                );
            }
            if let Some(home) = &home {
                assert!(
                    !line.contains(&home.display().to_string()),
                    "{shown}:{}: names this machine's home",
                    index + 1
                );
            }
            let envelope: Value = serde_json::from_str(line).unwrap();
            let run_id = envelope["run_id"].as_str().unwrap_or_default();
            assert!(
                run_id.starts_with(FIXTURE_RUN_PREFIX),
                "{shown}:{}: run id {run_id} is not a fixture id",
                index + 1
            );
            if let Some(parent) = envelope["parent_run_id"].as_str() {
                assert!(parent.starts_with(FIXTURE_RUN_PREFIX), "{shown}");
            }
            let mut found = Vec::new();
            strings(&envelope, &mut found);
            for text in found {
                if text.starts_with('/') {
                    assert!(
                        ALLOWED_ROOTS
                            .iter()
                            .any(|root| text == *root || text.starts_with(&format!("{root}/"))),
                        "{shown}:{}: {text} is outside the fixture root",
                        index + 1
                    );
                }
            }
        }
    }
}
