//! Delegation's two pre-flight checks, harness spec 6.1 items 2 and 3.
//!
//! **Nothing here launches a provider.** Both checks run before a child process
//! exists, so the seam is the check itself: `Lineage::of` reads the parent's
//! record and computes the depth, `Lineage::narrowing` compares the two policies,
//! and `hydra::run::refuse` writes the record contract 8.3 asks for. The launch
//! that would follow a passing pair stops at `write_spawn_config`, which is the
//! last step before an argv.
//!
//! **The parent records are built by hand**, the way `tests/cleanup.rs` builds
//! them, because a parent written by a real launch would cost a subscription and
//! the only three fields read off one are `depth`, `policy` and the run id.

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use hydra::mcp::{self, Checkout, Document};
use hydra::paths::Layout;
use hydra::policy::PolicySet;
use hydra::provider::Provider;
use hydra::record::Refusal;
use hydra::run::{Call, Lineage, Request};
use serde_json::{Value, json};

/// A layout per test, because the suite runs in parallel.
fn layout() -> (Layout, PathBuf) {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let root = std::env::temp_dir().join(format!(
        "hydra-delegation-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&root);
    let layout = Layout::at(&root);
    layout.ensure().unwrap();
    (layout, root)
}

/// A parent's record: line 0 and a closing line, with the two fields a child
/// reads off it set to whatever the test wants to prove.
fn write_parent(layout: &Layout, run_id: &str, policy: Option<&str>, depth: u32) -> PathBuf {
    let mut text = envelope(
        run_id,
        0,
        json!({
            "type": "run_start",
            "schema_version": 1,
            "hydra_version": "0.2.0",
            "command": "run",
            "launch_mode": "recorded",
            "provider_version": "stub",
            "provider_session_id": null,
            "task": "the parent's own task",
            "policy": policy,
            "policy_sha": null,
            "cwd": "/tmp",
            "worktree": null,
            "depth": depth,
            "pid": null,
            "process_start_time": null,
            "subscription": "anthropic",
        }),
    );
    text.push_str(&envelope(
        run_id,
        1,
        json!({"type": "run_end", "exit_code": 0, "body_source": "stream"}),
    ));
    let path = layout.run_file(run_id);
    std::fs::write(&path, text).unwrap();
    path
}

fn envelope(run_id: &str, seq: u64, hydra: Value) -> String {
    let mut line = json!({
        "run_id": run_id,
        "parent_run_id": null,
        "seq": seq,
        "ts": "2026-09-06T09:12:04.100Z",
        "provider": "claude",
        "model": null,
        "secret_flag": null,
        "hydra": hydra,
    })
    .to_string();
    line.push('\n');
    line
}

fn request(policy: &str, parent: Option<&str>) -> Request {
    Request {
        provider: Provider::Claude,
        task: "narrow this down".to_string(),
        model: None,
        policy: policy.to_string(),
        floor_lifted: false,
        timeout: Duration::from_secs(600),
        run_id: None,
        parent: parent.map(str::to_string),
        command: "run".to_string(),
        resumed_from: None,
        resume_session_id: None,
        working_dir: None,
        quiet: false,
        handed_off: None,
    }
}

/// How many run files are under `runs/`, for the refusals that write none.
fn records(layout: &Layout) -> usize {
    std::fs::read_dir(layout.runs()).unwrap().count()
}

/// The refusal, driven the way `hydra::run::run` drives it: compose line 0 from
/// the request and the lineage, then write the two-line record.
fn refuse(
    layout: &Layout,
    policies: &PolicySet,
    request: &Request,
    lineage: &Lineage,
    run_id: &str,
    remediation: String,
) -> Call {
    // Contract 8.3's "no provider fields": the check that refuses runs before the
    // version is read and before the child's environment is built, so neither is
    // on the line and neither is guessed.
    let line_zero = serde_json::to_value(hydra::run::run_start(
        request,
        hydra::run::RunStartRequest {
            task: "the preamble, then the task",
            policy_sha: policies.sha(),
            provider_version: None,
            cwd: std::path::Path::new("/tmp"),
            worktree: None,
            env_allowlist: None,
            skills: None,
            mcp: None,
            lineage,
            // No gate was compiled, so the refusal resolved no scope (contract 5.1).
            scopes: None,
            gate_sha: None,
            sandbox: None,
        },
    ))
    .unwrap();

    hydra::run::refuse(hydra::run::RefuseRequest {
        run_file: &layout.run_file(run_id),
        run_id,
        provider: request.provider,
        line_zero: &line_zero,
        refusal: Refusal::PolicyNarrowing,
        remediation,
        policies,
        worktree: None,
        seed_tree: None,
        layout,
        parent_run_id: lineage.parent_run_id(),
        quiet: false,
    })
    .unwrap()
}

const PARENT: &str = "01a07600-0000-7000-8000-0000000000a1";
const CHILD: &str = "01a07600-0000-7000-8000-0000000000b1";

#[test]
fn a_child_naming_a_wider_policy_is_refused_with_a_record() {
    // Contract 8.3: no child process starts, so without the record the attempt is
    // invisible, and attempted escalation is exactly the signal the corpus should
    // carry.
    let (layout, _root) = layout();
    let policies = PolicySet::discover().unwrap();
    write_parent(&layout, PARENT, Some("read-only"), 0);

    let request = request("permissive", Some(PARENT));
    let lineage = Lineage::of(&layout, request.parent.as_deref()).unwrap();
    assert_eq!(lineage.depth, 1, "a child of a depth 0 run is depth 1");

    let remediation = lineage
        .narrowing(&policies, &request.policy)
        .unwrap()
        .expect("permissive is wider than read-only");
    assert!(remediation.contains("read-only"), "{remediation}");
    assert!(remediation.contains("permissive"), "{remediation}");
    assert!(remediation.contains("7.3"), "{remediation}");

    let call = refuse(&layout, &policies, &request, &lineage, CHILD, remediation);
    let Call::Ended(result) = call else {
        panic!("a refused launch has ended by definition");
    };
    assert_eq!(result.outcome, Some("gated"));
    assert_eq!(result.outcome_source, Some("refusal"));
    assert!(
        result.exit_code.is_none(),
        "no child ran, so there is no exit status"
    );

    let text = std::fs::read_to_string(layout.run_file(CHILD)).unwrap();
    hydra::record::validate::check(&text, Some(CHILD)).assert_ok();
    assert_eq!(
        text.lines().count(),
        2,
        "run_start, run_end, nothing between"
    );

    let start: Value = serde_json::from_str(text.lines().next().unwrap()).unwrap();
    // Contract 4: the parent writes the edge at spawn, on every line of the
    // child's file rather than only on line 0.
    for line in text.lines() {
        let line: Value = serde_json::from_str(line).unwrap();
        assert_eq!(line["parent_run_id"], PARENT);
    }
    // Contract 8.3: the requested policy and the parent's beside it.
    assert_eq!(start["hydra"]["policy"], "permissive");
    assert_eq!(start["hydra"]["parent_policy"], "read-only");
    assert_eq!(start["hydra"]["depth"], 1);
    assert!(start["hydra"]["pid"].is_null());
    assert!(start["hydra"]["process_start_time"].is_null());
    // Nothing was created for this run, so line 0 says so rather than guessing.
    assert!(start["hydra"]["worktree"].is_null());
    assert!(start["hydra"].get("skills").is_none());
    // Contract 8.3: no provider fields. The version is null because no binary was
    // asked, and the environment list is absent because none was built.
    assert!(start["hydra"]["provider_version"].is_null(), "{start}");
    assert!(start["hydra"].get("env_allowlist").is_none(), "{start}");

    let end: Value = serde_json::from_str(text.lines().next_back().unwrap()).unwrap();
    assert_eq!(end["hydra"]["refusal"], "policy_narrowing");
}

#[test]
fn a_standard_parent_cannot_delegate_curate_and_the_refusal_says_incomparable() {
    // 7.3's accepted cost: the five are a partial order rather than a chain, so
    // this is not an escalation and it is still refused. A curator pass starts at
    // depth 0 from a human.
    let (layout, _root) = layout();
    let policies = PolicySet::discover().unwrap();
    write_parent(&layout, PARENT, Some("standard"), 0);

    let request = request("curate", Some(PARENT));
    let lineage = Lineage::of(&layout, request.parent.as_deref()).unwrap();
    let remediation = lineage
        .narrowing(&policies, &request.policy)
        .unwrap()
        .expect("standard does not contain curate");

    assert!(
        remediation.contains("incomparable"),
        "a partial order is not a chain, and the word for this pair is not \
         `wider`: {remediation}"
    );
    assert!(!remediation.contains("wider than"), "{remediation}");
    assert!(remediation.contains("depth 0"), "{remediation}");
}

#[test]
fn a_child_of_the_deepest_run_is_refused_before_a_run_id_exists() {
    // 6.1 item 2. The cap is 2, so a child of a depth 2 run would be depth 3.
    // Contract 8.3 promises a record for the narrowing refusal and for nothing
    // else, so this one is a typed error and the parent's own record shows the
    // refused call.
    let (layout, _root) = layout();
    write_parent(&layout, PARENT, Some("standard"), 1);
    let deepest = "01a07600-0000-7000-8000-0000000000a2";
    write_parent(&layout, deepest, Some("standard"), 2);

    let allowed = Lineage::of(&layout, Some(PARENT)).unwrap();
    assert_eq!(
        allowed.depth, 2,
        "the cap is a limit, not a forbidden value"
    );

    let err = Lineage::of(&layout, Some(deepest)).unwrap_err();
    let said = err.to_string();
    assert!(said.contains(deepest), "{said}");
    assert!(said.contains('2'), "the cap is in the sentence: {said}");
    assert_eq!(
        records(&layout),
        2,
        "the refusal wrote no record of its own"
    );
}

#[test]
fn a_parent_with_no_record_is_refused_before_a_run_id_exists() {
    // The run id is the whole of what `--parent` takes, and a run id with no file
    // under `~/hydra/runs/` is a typo rather than a lineage. It is refused the way
    // a policy name that is not one of the five is.
    let (layout, _root) = layout();
    let err = Lineage::of(&layout, Some(PARENT)).unwrap_err();
    let said = err.to_string();
    assert!(said.contains(PARENT), "{said}");
    assert_eq!(records(&layout), 0);
}

#[test]
fn a_parent_whose_record_has_no_policy_cannot_delegate() {
    // A null policy is a record written before the gate reached a provider, or an
    // interactive run (contract 5.6). Neither states a bound, and 7.3 needs one to
    // compare against, so the child is refused rather than given the benefit.
    let (layout, _root) = layout();
    write_parent(&layout, PARENT, None, 0);

    let err = Lineage::of(&layout, Some(PARENT)).unwrap_err();
    let said = err.to_string();
    assert!(said.contains("policy"), "{said}");
    assert_eq!(records(&layout), 1);
}

#[test]
fn a_harness_parent_bounds_nothing_and_its_child_is_depth_one() {
    // Contract 5.5: a `hydra evolve` pass is a harness run, `command: evolve`,
    // no provider, no launch mode, no policy, and spec 4.10 makes it the parent
    // of the proposer and the judges. Its authority is the human's who typed the
    // pass, so 7.3 has nothing to compare against and the child runs under the
    // policy it asks for. Measured 2026-09-08 on the first real pass, which the
    // no-policy refusal above ended before the proposer existed.
    let (layout, _root) = layout();
    let mut text = envelope(
        PARENT,
        0,
        json!({
            "type": "run_start",
            "schema_version": 1,
            "hydra_version": "0.2.0",
            "command": "evolve",
            "launch_mode": null,
            "provider_version": null,
            "provider_session_id": null,
            "task": null,
            "policy": null,
            "policy_sha": null,
            "cwd": "/tmp",
            "worktree": null,
            "depth": 0,
            "pid": 1,
            "process_start_time": null,
            "subscription": null,
        }),
    );
    text.push_str(&envelope(
        PARENT,
        1,
        json!({"type": "run_end", "exit_code": null, "body_source": "stream"}),
    ));
    std::fs::write(layout.run_file(PARENT), text).unwrap();

    let policies = PolicySet::discover().unwrap();
    let lineage = Lineage::of(&layout, Some(PARENT)).unwrap();
    assert_eq!(lineage.depth, 1);
    assert_eq!(lineage.parent_policy(), None);
    for child in ["standard", "judge"] {
        assert!(
            lineage.narrowing(&policies, child).unwrap().is_none(),
            "{child}"
        );
    }
}

#[test]
fn a_permissive_parent_delegating_read_only_passes_both_checks() {
    // The pair that runs. It stops at `write_spawn_config`, the last step before
    // an argv, and the gate it wrote is compiled under the child's own policy
    // rather than the parent's.
    let (layout, _root) = layout();
    let policies = PolicySet::discover().unwrap();
    write_parent(&layout, PARENT, Some("permissive"), 0);

    let request = request("read-only", Some(PARENT));
    let lineage = Lineage::of(&layout, request.parent.as_deref()).unwrap();
    assert_eq!(lineage.depth, 1);
    assert_eq!(lineage.parent_policy(), Some("permissive"));
    assert!(
        lineage
            .narrowing(&policies, &request.policy)
            .unwrap()
            .is_none(),
        "permissive contains every policy, read-only included"
    );

    let spawn = hydra::run::write_spawn_config(
        &layout,
        CHILD,
        &policies,
        request.provider,
        &request.policy,
        None,
        None,
        None,
        false,
    )
    .unwrap();
    let gate = std::fs::read_to_string(&spawn.gate).unwrap();
    assert!(gate.contains("\\\"policy\\\":\\\"read-only\\\""), "{gate}");
    assert!(spawn.hook_config.is_file());
}

#[test]
fn the_written_config_carries_the_parent_id_the_face_needs_for_its_own_depth() {
    // Map ticket 48. The document has no field for this, so the flag exists
    // only in the file one run writes for itself: without it, a face spawned
    // from the document would hand every child depth 0, and the cap the tests
    // above drive by hand would bound nothing beneath it.
    let (layout, _root) = layout();
    let document = Document::parse(
        r#"
```toml
[[server]]
name = "delegate"
transport = "stdio"
command = "hydra"
args = ["delegate-face"]
admit = ["standard"]
```
"#,
    )
    .unwrap();
    let plan = mcp::plan(&document, Provider::Claude, "standard", Checkout::Hydra);
    let file = mcp::write_config(&layout, PARENT, Provider::Claude, plan.injected(), None).unwrap();
    let config: Value = serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
    let args = config["mcpServers"]["delegate"]["args"].as_array().unwrap();
    assert_eq!(
        args,
        &vec![json!("delegate-face"), json!("--parent"), json!(PARENT)],
        "the face reads this id back with --parent and hands it to `Lineage::of` \
         on every call it makes in turn"
    );
}

#[test]
fn a_run_with_no_parent_is_depth_zero_and_bound_by_nothing() {
    // A human at depth 0 is the only thing that can widen anything (7.3), and this
    // is the shape every run had before delegation was built.
    let (layout, _root) = layout();
    let policies = PolicySet::discover().unwrap();

    let lineage = Lineage::of(&layout, None).unwrap();
    assert_eq!(lineage.depth, 0);
    assert_eq!(lineage.parent_run_id(), None);
    assert!(
        lineage
            .narrowing(&policies, "permissive")
            .unwrap()
            .is_none(),
        "nothing bounds a run a human started"
    );

    let line_zero = serde_json::to_value(hydra::run::run_start(
        &request("permissive", None),
        hydra::run::RunStartRequest {
            task: "the preamble, then the task",
            policy_sha: policies.sha(),
            provider_version: Some("2.1.263 (Claude Code)"),
            cwd: std::path::Path::new("/tmp"),
            worktree: None,
            env_allowlist: Some(vec!["PATH".to_string()]),
            skills: None,
            mcp: None,
            lineage: &lineage,
            scopes: None,
            gate_sha: None,
            sandbox: None,
        },
    ))
    .unwrap();
    assert_eq!(line_zero["depth"], 0);
    assert!(
        line_zero.get("parent_policy").is_none(),
        "absent rather than null: with no parent there is no policy to name"
    );
}

// --- containment, build ticket 57 -----------------------------------------

/// A parent's record with no closing line and a liveness pair that matches this
/// test process, so `live::read_view` calls it live.
fn write_live_parent(layout: &Layout, run_id: &str, policy: &str) -> PathBuf {
    let pid = std::process::id();
    let start = hydra::procinfo::start_time(pid)
        .unwrap()
        .expect("this process has a start time");
    let text = envelope(
        run_id,
        0,
        json!({
            "type": "run_start",
            "schema_version": 1,
            "hydra_version": "0.2.0",
            "command": "run",
            "launch_mode": "recorded",
            "provider_version": "stub",
            "provider_session_id": null,
            "task": "the parent's own task",
            "policy": policy,
            "policy_sha": null,
            "cwd": "/tmp",
            "worktree": null,
            "depth": 0,
            "pid": pid,
            "process_start_time": start.to_rfc3339_opts(chrono::SecondsFormat::Micros, true),
            "subscription": "anthropic",
        }),
    );
    let path = layout.run_file(run_id);
    std::fs::write(&path, text).unwrap();
    path
}

#[test]
fn a_marker_naming_a_live_run_puts_the_caller_inside_it() {
    let (layout, root) = layout();
    write_live_parent(&layout, "01a0-live", "standard");
    assert_eq!(
        hydra::containment::classify(&layout, Some("01a0-live")),
        hydra::containment::Caller::Inside("01a0-live".into())
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_marker_naming_a_run_that_ended_is_stale_and_never_permission() {
    let (layout, root) = layout();
    // `write_parent` closes the record, which is what an ended run looks like.
    write_parent(&layout, "01a0-ended", Some("standard"), 0);
    assert_eq!(
        hydra::containment::classify(&layout, Some("01a0-ended")),
        hydra::containment::Caller::Stale("01a0-ended".into())
    );
    // The shape a relocated `HYDRA_ROOT` takes: the marker is real and the
    // corpus it is looked up in is not the one that holds it.
    assert_eq!(
        hydra::containment::classify(&layout, Some("01a0-nowhere")),
        hydra::containment::Caller::Stale("01a0-nowhere".into())
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn no_marker_at_all_is_a_human_and_launches_anything() {
    let (layout, root) = layout();
    for marker in [None, Some("")] {
        assert_eq!(
            hydra::containment::classify(&layout, marker),
            hydra::containment::Caller::Outside
        );
    }
    let _ = std::fs::remove_dir_all(&root);
}

/// Whether the binary will honour `HYDRA_ROOT` in this process, printing that
/// the test skipped when it will not.
///
/// Inside a seatbelt `hydra run` refuses the variable before it reads the
/// marker, build ticket 57's review fix, so every launch below would get that
/// sentence instead of the one under test. The positive assertion would fail
/// and the negative ones would pass without testing anything. Rung two's
/// admission runs the suite inside a seatbelt. Build ticket 104.
fn binary_takes_hydra_root_or_skip(test_name: &str) -> bool {
    if hydra::sandbox::inside_seatbelt() {
        eprintln!("skipped {test_name}: inside a seatbelt, where the binary refuses HYDRA_ROOT");
        return false;
    }
    true
}

/// The binary, launched the way a run's own shell would launch it.
///
/// Drives the built binary rather than `run::run` because the marker is read out
/// of the process environment, and a test that set one would set it for every
/// other test in this file. The refusal lands before the provider is probed, so
/// nothing here needs a provider installed and nothing costs quota.
fn launch_with_marker(root: &std::path::Path, marker: Option<&str>, args: &[&str]) -> String {
    assert!(
        !hydra::sandbox::inside_seatbelt(),
        "call binary_takes_hydra_root_or_skip first: inside a seatbelt every launch is refused \
         for HYDRA_ROOT before the containment check reads the marker"
    );
    let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_hydra"));
    command
        .args(args)
        .env("HYDRA_ROOT", root)
        .env_remove("HYDRA_RUN_ID");
    if let Some(marker) = marker {
        command.env("HYDRA_RUN_ID", marker);
    }
    let out = command.output().expect("the binary runs");
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

#[test]
fn a_run_relaunching_the_binary_without_a_parent_is_refused_and_recorded() {
    if !binary_takes_hydra_root_or_skip(
        "a_run_relaunching_the_binary_without_a_parent_is_refused_and_recorded",
    ) {
        return;
    }
    // Build ticket 57 and the one critical finding of the adversarial review.
    // `standard` grants shell, shell reaches the binary, and until this landed
    // `hydra run --policy permissive` with no `--parent` was a child bounded by
    // nothing. No forging was needed; the flag was simply absent.
    let (layout, root) = layout();
    write_live_parent(&layout, "01a0-live", "standard");
    let before = records(&layout);

    let said = launch_with_marker(
        &root,
        Some("01a0-live"),
        &[
            "run",
            "--provider",
            "claude",
            "--policy",
            "permissive",
            "--task",
            "x",
        ],
    );
    assert!(
        said.contains("named no parent") && said.contains("--parent 01a0-live"),
        "the escalation was not refused: {said}"
    );
    assert_eq!(
        records(&layout),
        before + 1,
        "contract 8.3: a refused spawn is a run record, so the attempt is not invisible"
    );

    // Naming somebody else's run is the same refusal with a different sentence,
    // and it is the attack that survives `Lineage::of`: a parent that does not
    // exist is an error before a run id is minted, so the forge that matters
    // names a run this machine really has and is wider than the caller's.
    write_live_parent(&layout, "01a0-wide", "permissive");
    let other = launch_with_marker(
        &root,
        Some("01a0-live"),
        &[
            "run",
            "--provider",
            "claude",
            "--policy",
            "permissive",
            "--parent",
            "01a0-wide",
            "--task",
            "x",
        ],
    );
    assert!(
        other.contains("may only parent a child to itself"),
        "a forged parent was not refused: {other}"
    );

    // And the honest launch gets past this check. It fails later, on the depth
    // or the narrowing or the provider binary, which is somebody else's test.
    let honest = launch_with_marker(
        &root,
        Some("01a0-live"),
        &[
            "run",
            "--provider",
            "claude",
            "--policy",
            "read-only",
            "--parent",
            "01a0-live",
            "--task",
            "x",
        ],
    );
    assert!(
        !honest.contains("named no parent") && !honest.contains("may only parent"),
        "the honest child was refused by the containment check: {honest}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_launch_with_no_marker_is_not_refused_by_the_containment_check() {
    if !binary_takes_hydra_root_or_skip(
        "a_launch_with_no_marker_is_not_refused_by_the_containment_check",
    ) {
        return;
    }
    let (layout, root) = layout();
    let _ = layout;
    let said = launch_with_marker(
        &root,
        None,
        &[
            "run",
            "--provider",
            "claude",
            "--policy",
            "permissive",
            "--task",
            "x",
        ],
    );
    assert!(
        !said.contains("named no parent"),
        "a human at depth 0 was refused: {said}"
    );
    let _ = std::fs::remove_dir_all(&root);
}
