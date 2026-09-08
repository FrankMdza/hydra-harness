//! MCP injection on `claude`, harness spec 6.5 and 6.1 item 6, without launching
//! a provider.
//!
//! Three things are under test here. The file one launch generates from a
//! document with servers admitted to some policies and not others. The launch
//! check that refuses a run whose policy admits a server the launch cannot
//! deliver. And the record that refusal writes, which is contract 8.3's shape
//! read back through the normalizer rather than asserted from memory.
//!
//! `src/mcp.rs` holds the parser's own tests: what the document format accepts
//! and what it refuses. This file is about what reaches the disk.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use hydra::mcp::{self, Checkout, Document};
use hydra::paths::Layout;
use hydra::policy::PolicySet;
use hydra::provider::{Provider, claude};
use hydra::record::Refusal;
use hydra::run::Call;

/// A document naming one stdio server per policy that should see it, plus one
/// admitted to `judge`, which is the shape no `claude` launch can deliver.
const DOCUMENT: &str = r#"
# A fixture, in the shape `config/mcp.md` ships in.

```toml
[[server]]
name = "filesystem"
transport = "stdio"
command = "/usr/bin/true"
args = ["--root", "/tmp"]
admit = ["standard", "permissive"]

[[server]]
name = "docs"
transport = "http"
url = "https://example.com/mcp"
admit = ["permissive"]

[[server]]
name = "verdicts"
transport = "stdio"
command = "/usr/bin/true"
admit = ["judge"]
```
"#;

/// A layout per test, because the suite runs in parallel.
fn layout() -> (Layout, PathBuf) {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let root = std::env::temp_dir().join(format!(
        "hydra-mcp-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&root);
    let layout = Layout::at(&root);
    layout.ensure().unwrap();
    (layout, root)
}

fn written(layout: &Layout, run_id: &str, policy: &str) -> serde_json::Value {
    let document = Document::parse(DOCUMENT).unwrap();
    let plan = mcp::plan(&document, Provider::Claude, policy, Checkout::Hydra);
    let file = mcp::write_config(layout, run_id, Provider::Claude, plan.injected(), None).unwrap();
    assert_eq!(file, layout.mcp_file(run_id), "6.5's table names this path");
    serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap()
}

#[test]
fn a_server_the_policy_does_not_admit_is_not_written() {
    let (layout, _root) = layout();

    let standard = written(&layout, "r1", "standard");
    let servers = standard["mcpServers"].as_object().unwrap();
    assert_eq!(servers.len(), 1, "got {standard}");
    assert!(servers.contains_key("filesystem"));
    assert!(
        !servers.contains_key("docs"),
        "`docs` is admitted to permissive alone and this run is standard"
    );
    assert_eq!(servers["filesystem"]["command"], "/usr/bin/true");
    assert_eq!(servers["filesystem"]["args"][0], "--root");

    let permissive = written(&layout, "r2", "permissive");
    let servers = permissive["mcpServers"].as_object().unwrap();
    assert_eq!(servers.len(), 2);
    assert_eq!(servers["docs"]["type"], "http");
    assert_eq!(servers["docs"]["url"], "https://example.com/mcp");
}

#[test]
fn a_policy_that_admits_nothing_still_gets_a_file() {
    // 6.5: `--strict-mcp-config` is what makes an empty file mean something. It
    // is the difference between a run with no servers and a run with whatever
    // the human configured, and the record can only claim the set because of it.
    let (layout, _root) = layout();
    let config = written(&layout, "r1", "read-only");
    assert!(config["mcpServers"].as_object().unwrap().is_empty());
    assert!(layout.mcp_file("r1").is_file());
}

#[test]
fn the_probe_a_judge_launch_would_run_is_the_shape_that_answers_for_servers() {
    // Ticket 43, the argv half. This checks the flags a `judge` launch hands the
    // probe and Hydra's rule about when a probe's answer may be recorded. It
    // launches nothing, so it says nothing about what the binary does with those
    // flags: `tests/drift.rs` runs them and reads the empty set back.
    let document = Document::parse(DOCUMENT).unwrap();
    let plan = mcp::plan(&document, Provider::Claude, "judge", Checkout::Hydra);
    assert!(
        plan.undeliverable.is_some(),
        "the launch writes no file for this policy, so the probe passes none"
    );

    let shape = mcp::Probe {
        policy: "judge",
        config: None,
    };
    let probe = claude::probe_argv("01a07600-0000-7000-8000-00000000000b", shape);
    assert!(probe.iter().any(|a| a == "--restricted"), "{probe:?}");
    assert!(
        probe.iter().any(|a| a == "--strict-mcp-config"),
        "{probe:?}"
    );
    assert!(!probe.iter().any(|a| a == "--mcp-config"), "{probe:?}");
    assert!(
        claude::mcp_flags(shape).answers_for_servers(),
        "with the strict flag the set is what the argv says, so the record claims it"
    );
}

#[test]
fn the_launch_check_refuses_a_judge_run_that_admits_a_server() {
    // 6.1 item 6. `verdicts` is admitted to `judge`, and 6.7's `judge` argv
    // carries no `--mcp-config`, so this launch cannot deliver it.
    let document = Document::parse(DOCUMENT).unwrap();
    let plan = mcp::plan(&document, Provider::Claude, "judge", Checkout::Hydra);

    assert!(
        plan.shortfall().is_some(),
        "a shortfall is a refused launch"
    );
    assert!(plan.injected().is_empty());
    let remediation = plan.remediation();
    assert!(remediation.contains("verdicts"), "{remediation}");
    assert!(
        remediation.contains("mcp.md"),
        "the human is told where to change it: {remediation}"
    );
}

#[test]
fn every_other_policy_delivers_what_it_admits() {
    let document = Document::parse(DOCUMENT).unwrap();
    for policy in ["standard", "read-only", "curate", "permissive"] {
        let plan = mcp::plan(&document, Provider::Claude, policy, Checkout::Hydra);
        assert!(plan.shortfall().is_none(), "{policy}");
        assert_eq!(
            plan.injected().len(),
            plan.admitted.len(),
            "{policy} lost a server it admits"
        );
    }
}

#[test]
fn the_refusal_writes_a_record_that_derives_mcp_unavailable() {
    // Contract 8.3: no child starts, so without the record the attempt is
    // invisible. The record is read back through the normalizer rather than
    // asserted from memory, which is what makes contract 17's JSON a projection
    // of the file.
    let (layout, _root) = layout();
    let run_id = "01a07520-0000-7000-8000-00000000000a";
    let document = Document::parse(DOCUMENT).unwrap();
    let plan = mcp::plan(&document, Provider::Claude, "judge", Checkout::Hydra);

    let line_zero = serde_json::json!({
        "type": "run_start",
        "schema_version": 1,
        "hydra_version": "0.2.0",
        "command": "run",
        "launch_mode": "recorded",
        "provider_version": "2.1.263 (Claude Code)",
        "provider_session_id": null,
        "task": "weigh these two answers",
        "policy": "judge",
        "policy_sha": "0000000000000000000000000000000000000000000000000000000000000000",
        "cwd": "/tmp",
        "worktree": null,
        "depth": 0,
        "pid": null,
        "process_start_time": null,
        // `resolved` is `[]` and not null on this policy since ticket 43. The
        // probe runs before this refusal, under the run's own
        // `--restricted --strict-mcp-config`, and measures the empty set. This
        // line is written by hand, so what follows checks that the validator and
        // the reader take that shape, not that the probe produces it.
        "mcp": { "doc_sha": document.sha, "injected": [], "resolved": [] },
        "subscription": "anthropic",
    });

    let run_file = layout.run_file(run_id);
    let call = hydra::run::refuse(
        &run_file,
        run_id,
        Provider::Claude,
        &line_zero,
        Refusal::McpUnavailable,
        plan.remediation(),
        &PolicySet::discover().unwrap(),
        None,
        None,
        &layout,
        None,
    )
    .unwrap();

    let Call::Ended(result) = call else {
        panic!("a refused launch has ended by definition");
    };
    assert_eq!(result.outcome, Some("mcp_unavailable"));
    assert_eq!(result.outcome_source, Some("refusal"));
    assert!(
        result.exit_code.is_none(),
        "no child ran, so there is no exit status to report"
    );

    let text = std::fs::read_to_string(&run_file).unwrap();
    hydra::record::validate::check(&text, Some(run_id)).assert_ok();
    assert_eq!(
        text.lines().count(),
        2,
        "run_start, run_end, nothing between"
    );

    let start: serde_json::Value = serde_json::from_str(text.lines().next().unwrap()).unwrap();
    assert_eq!(
        start["hydra"]["mcp"]["resolved"],
        serde_json::json!([]),
        "an empty array survived the round trip as an empty array. Contract 5.3 \
         needs a reader to tell it from null, and `tests/drift.rs` is what checks \
         that a judge probe produces the empty one"
    );

    let end: serde_json::Value = serde_json::from_str(text.lines().next_back().unwrap()).unwrap();
    assert_eq!(end["hydra"]["refusal"], "mcp_unavailable");
    assert!(
        end["hydra"]["remediation"]
            .as_str()
            .unwrap()
            .contains("verdicts")
    );
}

// ---------------------------------------------------------------------------
// the delegate face: map ticket 48's rule, and only in the file one run writes
// ---------------------------------------------------------------------------

/// Four servers that could be the delegate face, admitted to the two policies
/// the human named for it. Only the first two are `hydra`'s own binary; the
/// other two are close enough in spelling to prove the match is exact.
const FACE_DOCUMENT: &str = r#"
```toml
[[server]]
name = "face-bare"
transport = "stdio"
command = "hydra"
args = ["delegate-face"]
admit = ["standard", "permissive"]

[[server]]
name = "face-absolute"
transport = "stdio"
command = "/opt/homebrew/bin/hydra"
args = ["delegate-face"]
admit = ["standard", "permissive"]

[[server]]
name = "not-hydra"
transport = "stdio"
command = "hydra-something"
args = ["delegate-face"]
admit = ["standard", "permissive"]

[[server]]
name = "python"
transport = "stdio"
command = "python3"
args = ["-m", "some_server"]
admit = ["standard", "permissive"]
```
"#;

#[test]
fn a_server_named_hydra_carries_the_parent_flag_and_a_lookalike_does_not() {
    let (layout, _root) = layout();
    let document = Document::parse(FACE_DOCUMENT).unwrap();
    let plan = mcp::plan(&document, Provider::Claude, "standard", Checkout::Hydra);
    let run_id = "01a07610-0000-7000-8000-0000000000f1";
    let file = mcp::write_config(&layout, run_id, Provider::Claude, plan.injected(), None).unwrap();
    let config: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
    let servers = config["mcpServers"].as_object().unwrap();

    for name in ["face-bare", "face-absolute"] {
        let args = servers[name]["args"].as_array().unwrap();
        assert_eq!(
            args,
            &vec![
                serde_json::json!("delegate-face"),
                serde_json::json!("--parent"),
                serde_json::json!(run_id),
            ],
            "{name} is hydra's own binary, matched on the basename"
        );
    }

    for name in ["not-hydra", "python"] {
        let args = servers[name]["args"].as_array().unwrap();
        assert!(
            !args.iter().any(|a| a == "--parent"),
            "{name} is not hydra's binary and should be untouched: {args:?}"
        );
    }

    // The document itself carries what was declared, never what one run
    // happened to write into its own file.
    let declared = document
        .servers
        .iter()
        .find(|s| s.name == "face-bare")
        .unwrap();
    assert_eq!(declared.args, vec!["delegate-face".to_string()]);
}

#[test]
fn read_only_judge_and_curate_write_none_of_the_face() {
    // The human admitted the face to `standard` and `permissive` alone (map
    // ticket 48). This is the document saying so, not a special case in the
    // writer: a policy that admits none of these four servers gets an empty
    // file the same way any other policy would.
    let (layout, _root) = layout();
    let document = Document::parse(FACE_DOCUMENT).unwrap();
    for policy in ["read-only", "judge", "curate"] {
        let plan = mcp::plan(&document, Provider::Claude, policy, Checkout::Hydra);
        assert!(plan.admitted.is_empty(), "{policy}");
        let run_id = format!("r-{policy}");
        let file =
            mcp::write_config(&layout, &run_id, Provider::Claude, plan.injected(), None).unwrap();
        let config: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
        assert!(
            config["mcpServers"].as_object().unwrap().is_empty(),
            "{policy}: {config}"
        );
    }
}

// ---------------------------------------------------------------------------
// cursor-agent: the config lives in the worktree, and the pre-flight is fatal
// ---------------------------------------------------------------------------

fn cursor_written(layout: &Layout, run_id: &str, policy: &str, worktree: &Path) -> serde_json::Value {
    let document = Document::parse(DOCUMENT).unwrap();
    let plan = mcp::plan(&document, Provider::CursorAgent, policy, Checkout::Hydra);
    let file = mcp::write_config(
        layout,
        run_id,
        Provider::CursorAgent,
        plan.injected(),
        Some(worktree),
    )
    .unwrap();
    assert_eq!(file, worktree.join(".cursor/mcp.json"), "6.5's table names this path");
    serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap()
}

#[test]
fn a_cursor_run_writes_its_config_into_the_worktree_with_url_alone() {
    let (layout, root) = layout();
    let worktree = root.join("worktree");
    std::fs::create_dir_all(&worktree).unwrap();

    let permissive = cursor_written(&layout, "r1", "permissive", &worktree);
    let servers = permissive["mcpServers"].as_object().unwrap();
    assert_eq!(servers.len(), 2, "got {permissive}");
    assert_eq!(servers["filesystem"]["command"], "/usr/bin/true");
    assert_eq!(servers["docs"]["url"], "https://example.com/mcp");
    assert!(
        servers["docs"].get("type").is_none(),
        "`type` is claude's key; Cursor takes `url` alone (6.5)"
    );

    // The ignore file went in beside it, so `git status` stays clean.
    let ignore = std::fs::read_to_string(worktree.join(".cursor/.gitignore")).unwrap();
    assert!(ignore.contains("/mcp.json"));
}

#[test]
fn a_cursor_run_that_admits_nothing_still_gets_a_file() {
    let (layout, root) = layout();
    let worktree = root.join("worktree");
    std::fs::create_dir_all(&worktree).unwrap();
    let config = cursor_written(&layout, "r1", "read-only", &worktree);
    assert!(config["mcpServers"].as_object().unwrap().is_empty());
    assert!(worktree.join(".cursor/mcp.json").is_file());
}

#[test]
fn a_checkout_that_already_holds_an_mcp_config_is_refused_rather_than_overwritten() {
    let (layout, root) = layout();
    let worktree = root.join("worktree");
    std::fs::create_dir_all(worktree.join(".cursor")).unwrap();
    std::fs::write(worktree.join(".cursor/mcp.json"), "{\"mcpServers\":{}}").unwrap();

    let document = Document::parse(DOCUMENT).unwrap();
    let plan = mcp::plan(&document, Provider::CursorAgent, "standard", Checkout::Hydra);
    let err = mcp::write_config(
        &layout,
        "r1",
        Provider::CursorAgent,
        plan.injected(),
        Some(&worktree),
    )
    .unwrap_err();
    assert!(err.to_string().contains("already exists"), "{err:#}");
}

#[test]
fn the_pre_flight_refusal_writes_a_record_that_derives_mcp_unavailable() {
    // 6.1 item 7, on the listing shape `cursor-agent mcp list` prints. The
    // listing says `docs` is not loaded, which on this provider means the
    // human's same-named server may be starting instead, and that is the one
    // failure mode that produces a wrong answer rather than no answer.
    use hydra::provider::cursor::{Preflight, parse_listing};

    let (layout, _root) = layout();
    let run_id = "01a07520-0000-7000-8000-00000000000b";
    let document = Document::parse(DOCUMENT).unwrap();
    let preflight = Preflight {
        listing: parse_listing("filesystem: ready\ndocs: not loaded (needs approval)\n"),
    };
    let injected = vec!["docs".to_string(), "filesystem".to_string()];
    let short = preflight.shortfall(&injected);
    assert_eq!(short.len(), 1);
    assert_eq!(short[0].name, "docs");
    assert_eq!(preflight.resolved(), vec!["filesystem".to_string()]);

    let line_zero = serde_json::json!({
        "type": "run_start",
        "schema_version": 1,
        "hydra_version": "0.2.0",
        "command": "run",
        "launch_mode": "recorded",
        "provider_version": "2026.09.02-c22c1a3",
        "provider_session_id": null,
        "task": "say pong",
        "policy": "permissive",
        "policy_sha": "0000000000000000000000000000000000000000000000000000000000000000",
        "cwd": "/tmp",
        "worktree": null,
        "depth": 0,
        "pid": null,
        "process_start_time": null,
        "mcp": { "doc_sha": document.sha, "injected": injected, "resolved": preflight.resolved() },
        "subscription": "cursor",
    });

    let run_file = layout.run_file(run_id);
    let call = hydra::run::refuse(
        &run_file,
        run_id,
        Provider::CursorAgent,
        &line_zero,
        Refusal::McpUnavailable,
        preflight.remediation(&short),
        &PolicySet::discover().unwrap(),
        None,
        None,
        &layout,
        None,
    )
    .unwrap();

    let Call::Ended(result) = call else {
        panic!("a refused launch has ended by definition");
    };
    assert_eq!(result.outcome, Some("mcp_unavailable"));
    assert_eq!(result.outcome_source, Some("refusal"));
    assert!(result.exit_code.is_none());

    let text = std::fs::read_to_string(&run_file).unwrap();
    hydra::record::validate::check(&text, Some(run_id)).assert_ok();
    let end: serde_json::Value = serde_json::from_str(text.lines().next_back().unwrap()).unwrap();
    assert_eq!(end["hydra"]["refusal"], "mcp_unavailable");
    let remediation = end["hydra"]["remediation"].as_str().unwrap();
    assert!(remediation.contains("pre-flight"), "{remediation}");
    assert!(remediation.contains("docs: not loaded"), "{remediation}");
}

#[test]
fn the_shipped_document_admits_the_face_to_standard_and_permissive_only() {
    // Protected config #5 as it stands on this machine since map ticket 48:
    // one server, the delegate face, admitted to the two policies a human
    // chose and to nothing narrower. Every other policy still launches with an
    // empty `mcpServers` object and refuses nothing.
    let document = Document::load().unwrap();
    for policy in ["judge", "curate", "read-only"] {
        let plan = mcp::plan(&document, Provider::Claude, policy, Checkout::Hydra);
        assert!(plan.admitted.is_empty(), "{policy}");
        assert!(
            plan.shortfall().is_none(),
            "a policy the face is not admitted to refuses nothing, {policy} included"
        );
    }
    for policy in ["standard", "permissive"] {
        let plan = mcp::plan(&document, Provider::Claude, policy, Checkout::Hydra);
        let names: Vec<&str> = plan.admitted.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, ["hydra"], "{policy}");
    }
}

// ---------------------------------------------------------------------------
// agy: the config lives in the worktree and the remote key is its own
// ---------------------------------------------------------------------------

fn agy_written(layout: &Layout, run_id: &str, policy: &str, worktree: &Path) -> serde_json::Value {
    let document = Document::parse(DOCUMENT).unwrap();
    let plan = mcp::plan(&document, Provider::Agy, policy, Checkout::Hydra);
    let file = mcp::write_config(layout, run_id, Provider::Agy, plan.injected(), Some(worktree))
        .unwrap();
    assert_eq!(
        file,
        worktree.join(".agents/mcp_config.json"),
        "6.5's table names this path, and research 23 measured it against three others"
    );
    serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap()
}

#[test]
fn an_agy_run_writes_its_config_into_the_worktree_with_serverurl() {
    let (layout, root) = layout();
    let worktree = root.join("worktree");
    std::fs::create_dir_all(&worktree).unwrap();

    let permissive = agy_written(&layout, "r1", "permissive", &worktree);
    let servers = permissive["mcpServers"].as_object().unwrap();
    assert_eq!(servers.len(), 2, "got {permissive}");
    // The stdio body is byte-identical to the other two providers'.
    assert_eq!(servers["filesystem"]["command"], "/usr/bin/true");
    assert_eq!(servers["filesystem"]["args"][0], "--root");
    // And the remote key is where the three diverge.
    assert_eq!(servers["docs"]["serverUrl"], "https://example.com/mcp");
    assert!(
        servers["docs"].get("url").is_none(),
        "`url` is Cursor's key; this provider takes `serverUrl` (6.5)"
    );
    assert!(servers["docs"].get("type").is_none(), "`type` is claude's");

    let ignore = std::fs::read_to_string(worktree.join(".agents/.gitignore")).unwrap();
    assert!(ignore.contains("/mcp_config.json"));
}

#[test]
fn an_agy_run_outside_a_checkout_can_be_delivered_nothing() {
    // 6.5: this provider reads the file out of the worktree, so a run with no
    // checkout of Hydra's own gets no config at all, and a policy that admits a
    // server then fails the launch rather than launching short.
    let document = Document::parse(DOCUMENT).unwrap();
    let plan = mcp::plan(&document, Provider::Agy, "permissive", Checkout::None);
    assert!(plan.undeliverable.is_some());
    assert_eq!(plan.admitted.len(), 2);
    let why = plan.shortfall().expect("permissive admits two servers");
    assert!(why.contains("no checkout"), "{why}");
    let remediation = plan.remediation();
    assert!(remediation.contains("filesystem"), "{remediation}");
    assert!(remediation.contains("no checkout"), "{remediation}");

    // And a policy that admits none is not refused for having none.
    let quiet = mcp::plan(&document, Provider::Agy, "read-only", Checkout::None);
    assert!(quiet.shortfall().is_none());
}

#[test]
fn an_agy_checkout_that_already_holds_an_mcp_config_is_refused() {
    let (layout, root) = layout();
    let worktree = root.join("worktree");
    std::fs::create_dir_all(worktree.join(".agents")).unwrap();
    std::fs::write(worktree.join(".agents/mcp_config.json"), "{\"mcpServers\":{}}").unwrap();

    let document = Document::parse(DOCUMENT).unwrap();
    let plan = mcp::plan(&document, Provider::Agy, "standard", Checkout::Hydra);
    let err = mcp::write_config(
        &layout,
        "r1",
        Provider::Agy,
        plan.injected(),
        Some(&worktree),
    )
    .unwrap_err();
    assert!(err.to_string().contains("already exists"), "{err:#}");
}
