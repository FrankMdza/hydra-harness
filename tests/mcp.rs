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

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use hydra::mcp::{self, Document};
use hydra::paths::Layout;
use hydra::policy::PolicySet;
use hydra::provider::Provider;
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
    let plan = mcp::plan(&document, Provider::Claude, policy);
    let file = mcp::write_config(layout, run_id, Provider::Claude, plan.injected()).unwrap();
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
fn the_launch_check_refuses_a_judge_run_that_admits_a_server() {
    // 6.1 item 6. `verdicts` is admitted to `judge`, and 6.7's `judge` argv
    // carries no `--mcp-config`, so this launch cannot deliver it.
    let document = Document::parse(DOCUMENT).unwrap();
    let plan = mcp::plan(&document, Provider::Claude, "judge");

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
        let plan = mcp::plan(&document, Provider::Claude, policy);
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
    let plan = mcp::plan(&document, Provider::Claude, "judge");

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
        "mcp": { "doc_sha": document.sha, "injected": [], "resolved": null },
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

    let end: serde_json::Value = serde_json::from_str(text.lines().next_back().unwrap()).unwrap();
    assert_eq!(end["hydra"]["refusal"], "mcp_unavailable");
    assert!(
        end["hydra"]["remediation"]
            .as_str()
            .unwrap()
            .contains("verdicts")
    );
}

#[test]
fn the_shipped_document_admits_nothing_to_anybody() {
    // Protected config #5 as it stands on this machine: no server is configured
    // here, so the document ships with the format and an example in comments.
    // Every policy therefore launches with an empty `mcpServers` object.
    let document = Document::load().unwrap();
    for policy in ["judge", "curate", "read-only", "standard", "permissive"] {
        let plan = mcp::plan(&document, Provider::Claude, policy);
        assert!(plan.admitted.is_empty(), "{policy}");
        assert!(
            plan.shortfall().is_none(),
            "a document that admits nothing refuses nothing, {policy} included"
        );
    }
}
