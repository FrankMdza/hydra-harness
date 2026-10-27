//! `hydra reap` and `hydra prune`, harness spec 10.
//!
//! The two cleanup commands share a fixture and a rule, so they share a file. The
//! rule is that neither destroys anything a human has not asked to lose: reap
//! commits a checkout to its branch before removing it, prune moves a record
//! rather than deleting it, and each has one flag that goes further.
//!
//! **The commit is what most of this checks, because it is the promise.** Harness
//! spec 10 calls reap the Hermes lesson turned on our own tooling, and a guarantee
//! written in a doc comment is worth what Hermes' was. So the assertions read the
//! branch back and look for the bytes.

#[path = "common/git.rs"]
mod fixture_git;

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use hydra::paths::Layout;
use hydra::provider::Provider;
use hydra::prune::Pruned;
use hydra::reap::Reaped;
use hydra::skills::{self, Store};
use serde_json::{Value, json};

// --- the fixture ---------------------------------------------------------

fn git(dir: &Path, args: &[&str]) -> String {
    let out = fixture_git::command()
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git {} failed: {}",
        args.join(" "),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).to_string()
}

/// A repository with one commit, and a `~/hydra` beside it that the test owns.
///
/// The two are siblings rather than nested, which is the shape a real run has:
/// `~/hydra/worktrees/<run-id>` is a checkout of a repo that lives somewhere else
/// entirely, and reap has to find its way back from the checkout to the repo.
struct Fixture {
    root: PathBuf,
    repo: PathBuf,
}

impl Fixture {
    fn new(name: &str) -> Self {
        let root =
            std::env::temp_dir().join(format!("hydra-cleanup-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let repo = root.join("repo");
        std::fs::create_dir_all(&repo).unwrap();

        git(&repo, &["init", "-q", "-b", "main"]);
        git(&repo, &["config", "user.email", "test@example.invalid"]);
        git(&repo, &["config", "user.name", "hydra tests"]);
        std::fs::write(repo.join("committed.txt"), "one\n").unwrap();
        std::fs::write(repo.join(".gitignore"), "ignored/\n").unwrap();
        git(&repo, &["add", "-A"]);
        git(&repo, &["commit", "-qm", "first"]);

        let fixture = Self {
            root: root.join("hydra"),
            repo,
        };
        fixture.layout().ensure().unwrap();
        fixture
    }

    fn layout(&self) -> Layout {
        Layout::at(&self.root)
    }

    /// A checkout the way `hydra run` makes one, spawn directory and all.
    ///
    /// The seed index lands in `~/hydra/spawn/<run-id>/`, so this creates the pair
    /// reap has to remove together without a second call.
    fn checkout(&self, run_id: &str) -> PathBuf {
        let layout = self.layout();
        let dest = layout.worktree(run_id);
        hydra::worktree::create(
            &self.repo,
            &dest,
            &format!("hydra/{run_id}"),
            &layout.seed_index(run_id),
        )
        .unwrap();
        dest
    }

    /// The skill store as ticket 07 will leave it: a git repository with one
    /// commit in it. The shape `tests/curate.rs` builds, plus one difference: a
    /// second call hands back the store it already made, because a test that
    /// wants a checkout wants the store it came off too.
    fn store(&self) -> PathBuf {
        let store = self.layout().skills();
        if store.join(".git").is_dir() {
            return store;
        }
        std::fs::create_dir_all(&store).unwrap();
        git(&store, &["init", "-q", "-b", "main"]);
        git(&store, &["config", "user.email", "test@example.invalid"]);
        git(&store, &["config", "user.name", "hydra tests"]);
        std::fs::write(store.join("README.md"), "the skill store\n").unwrap();
        git(&store, &["add", "-A"]);
        git(&store, &["commit", "-qm", "the store"]);
        store
    }

    /// A curate pass's checkout of the store, made by the code that makes the
    /// real one. Rungs spec 3.6: a git worktree of the store on
    /// `rungs/curate/<run-id>`, never the store's own working tree.
    fn store_checkout(&self, run_id: &str) -> PathBuf {
        let store = self.store();
        let checkout = self.layout().skill_checkout(run_id);
        hydra::rungs::curate::check_out(
            &store,
            &checkout,
            &format!("rungs/curate/{run_id}"),
            "HEAD",
        )
        .unwrap();
        checkout
    }

    fn branches(&self) -> String {
        git(
            &self.repo,
            &["branch", "--list", "--format=%(refname:short)"],
        )
    }

    fn store_branches(&self) -> String {
        git(
            &self.store(),
            &["branch", "--list", "--format=%(refname:short)"],
        )
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(self.root.parent().unwrap());
    }
}

/// A run file with the liveness pair set to whatever the test wants to prove.
fn write_run(layout: &Layout, run_id: &str, pid: Option<u32>, closed: bool) -> PathBuf {
    let start = pid.and_then(|pid| hydra::procinfo::start_time(pid).ok().flatten());
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
            "task": "probe",
            "policy": "standard",
            "policy_sha": null,
            "cwd": "/tmp",
            "worktree": {
                "path": layout.worktree(run_id).display().to_string(),
                "branch": format!("hydra/{run_id}"),
            },
            "depth": 0,
            "pid": pid,
            "process_start_time": start
                .map(|t| Value::from(t.to_rfc3339_opts(chrono::SecondsFormat::Micros, true)))
                .unwrap_or(Value::Null),
            "subscription": "none",
        }),
    );
    if closed {
        text.push_str(&envelope(
            run_id,
            1,
            json!({"type": "run_end", "exit_code": 0, "body_source": "stream"}),
        ));
    }
    let path = layout.run_file(run_id);
    std::fs::write(&path, text).unwrap();
    path
}

fn envelope(run_id: &str, seq: u64, hydra: Value) -> String {
    let mut line = json!({
        "run_id": run_id,
        "parent_run_id": null,
        "seq": seq,
        "ts": "2026-09-05T20:51:39.370Z",
        "provider": "claude",
        "model": null,
        "secret_flag": null,
        "hydra": hydra,
    })
    .to_string();
    line.push('\n');
    line
}

/// The `Done` inside, or a panic naming why there is not one.
fn as_done(reaped: &Reaped) -> &hydra::reap::Done {
    match reaped {
        Reaped::Done(done) => done,
        Reaped::Left { run_id, why } => panic!("{run_id} was left: {why}"),
        Reaped::Failed { run_id, why } => panic!("{run_id} failed: {why}"),
    }
}

/// The run's own checkout of the parent repo, or a panic saying there was none.
fn worktree_of(reaped: &Reaped) -> &hydra::reap::Checkout {
    as_done(reaped)
        .worktree
        .as_ref()
        .expect("the worktree was not swept")
}

/// A curate pass's checkout of the skill store, same shape and same panic.
fn store_checkout_of(reaped: &Reaped) -> &hydra::reap::Checkout {
    as_done(reaped)
        .store_checkout
        .as_ref()
        .expect("the store checkout was not swept")
}

/// Reaps one run. The grace period is a launch window and a named run skips it,
/// so this passes zero and the sweeps below say which of the two they mean.
fn reap_one(layout: &Layout, run_id: &str, delete_branch: bool) -> Vec<Reaped> {
    hydra::reap::reap(layout, Some(run_id), delete_branch, NO_GRACE).unwrap()
}

/// No launch is in progress in a test, so the sweeps judge a checkout on its
/// record rather than on its age. Two tests below want the age rule and say so.
const NO_GRACE: std::time::Duration = std::time::Duration::ZERO;

/// Prunes the named runs, or all of them with an empty slice.
fn prune(layout: &Layout, run_ids: &[String], delete: bool, dry_run: bool) -> Vec<Pruned> {
    hydra::prune::prune(layout, run_ids, delete, dry_run, NO_GRACE).unwrap()
}

// --- reap ----------------------------------------------------------------

#[test]
fn reap_drops_only_untracked_empty_settings() {
    for tracked in [false, true] {
        for content in ["", "{}", "{}\n", "{\"hooks\":{}}"] {
            let fixture = Fixture::new("empty-settings");
            if tracked {
                std::fs::create_dir_all(fixture.repo.join(".claude")).unwrap();
                for name in ["settings.json", "settings.local.json"] {
                    std::fs::write(fixture.repo.join(".claude").join(name), content).unwrap();
                }
                git(&fixture.repo, &["add", ".claude"]);
                git(&fixture.repo, &["commit", "-qm", "tracked settings"]);
            }
            let run = "01a07217-6007-7733-9ce2-7bb31830eab8";
            let checkout = fixture.checkout(run);
            std::fs::create_dir_all(checkout.join(".claude")).unwrap();
            for name in ["settings.json", "settings.local.json"] {
                std::fs::write(checkout.join(".claude").join(name), content).unwrap();
            }
            std::fs::write(checkout.join("child.txt"), "work\n").unwrap();
            let reaped = reap_one(&fixture.layout(), run, false);
            assert_eq!(hydra::reap::failures(&reaped), 0);
            let tree = git(
                &fixture.repo,
                &["ls-tree", "-r", "--name-only", &format!("hydra/{run}")],
            );
            for name in ["settings.json", "settings.local.json"] {
                assert_eq!(
                    tree.lines().any(|line| line == format!(".claude/{name}")),
                    tracked || !matches!(content, "" | "{}"),
                    "tracked={tracked}, content={content:?}"
                );
            }
        }
    }
}

#[test]
fn the_childs_work_is_on_the_branch_before_the_checkout_goes() {
    let fixture = Fixture::new("commits");
    let layout = fixture.layout();
    let run = "01a07217-6007-7733-9ce2-7bb31830eab8";
    let checkout = fixture.checkout(run);

    // What the child did: one new file and one edit to a tracked one.
    std::fs::write(checkout.join("child.txt"), "what the child wrote\n").unwrap();
    std::fs::write(checkout.join("committed.txt"), "one\ntwo\n").unwrap();

    let reaped = reap_one(&layout, run, false);
    let swept = worktree_of(&reaped[0]);

    assert!(swept.committed.is_some(), "reap removed without committing");
    assert!(!checkout.exists(), "the checkout is still there");
    assert!(
        !layout.spawn(run).exists(),
        "the spawn directory is still there"
    );

    // The bytes, read back off the branch. This is the assertion the whole
    // command exists for.
    let branch = format!("hydra/{run}");
    assert_eq!(
        git(&fixture.repo, &["show", &format!("{branch}:child.txt")]),
        "what the child wrote\n"
    );
    assert_eq!(
        git(&fixture.repo, &["show", &format!("{branch}:committed.txt")]),
        "one\ntwo\n"
    );
    assert!(
        fixture.branches().contains(&branch),
        "the branch holding the commit was deleted: {}",
        fixture.branches()
    );
    assert!(!swept.branch_removed);
}

#[test]
fn reap_does_not_commit_hydras_own_files_when_the_checkout_already_has_an_ignore() {
    // A repo that tracks its own `.cursor/.gitignore` is the shape `claim`
    // warns about: Hydra leaves that file alone, so `hooks.json` and
    // `mcp.json` sit untracked. Reap must still keep them off the branch.
    let fixture = Fixture::new("hydra-own-files");
    std::fs::create_dir_all(fixture.repo.join(".cursor")).unwrap();
    std::fs::write(fixture.repo.join(".cursor/.gitignore"), "rules/\n").unwrap();
    git(&fixture.repo, &["add", "-A"]);
    git(
        &fixture.repo,
        &["commit", "-qm", "the checkout's own ignore file"],
    );

    let layout = fixture.layout();
    let run = "01a07730-a1b2-7c3d-8e4f-500000000001";
    let checkout = fixture.checkout(run);
    std::fs::write(checkout.join("child.txt"), "the edit\n").unwrap();
    std::fs::write(checkout.join(".cursor/hooks.json"), "{}\n").unwrap();
    std::fs::write(checkout.join(".cursor/mcp.json"), "{}\n").unwrap();

    let reaped = reap_one(&layout, run, false);
    let swept = worktree_of(&reaped[0]);
    assert!(swept.committed.is_some(), "reap removed without committing");
    assert!(!checkout.exists());

    let branch = format!("hydra/{run}");
    assert_eq!(
        git(&fixture.repo, &["show", &format!("{branch}:child.txt")]),
        "the edit\n"
    );
    let stat = git(&fixture.repo, &["show", "--stat", &branch]);
    assert!(
        !stat.contains("hooks.json") && !stat.contains("mcp.json"),
        "Hydra's files landed on the branch: {stat}"
    );
    let tree = git(&fixture.repo, &["ls-tree", "-r", "--name-only", &branch]);
    assert!(
        !tree
            .lines()
            .any(|line| line == ".cursor/hooks.json" || line == ".cursor/mcp.json"),
        "Hydra's files are in the tree: {tree}"
    );
    assert!(
        tree.lines().any(|line| line == ".cursor/.gitignore"),
        "the checkout's own ignore file was dropped: {tree}"
    );
}

/// A closed record for a run that injected `linked` into `provider`'s skill
/// root, which is where reap learns what Hydra put there.
fn write_skill_run(layout: &Layout, run_id: &str, provider: &str, linked: &[&str], sha: &str) {
    write_run(layout, run_id, None, true);
    let path = layout.run_file(run_id);
    let text = std::fs::read_to_string(&path).unwrap();
    let mut lines: Vec<Value> = text
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    for line in &mut lines {
        line["provider"] = json!(provider);
    }
    lines[0]["hydra"]["skills"] = json!({"store_sha": sha, "linked": linked, "resolved": null});
    let text: String = lines.iter().map(|line| format!("{line}\n")).collect();
    std::fs::write(&path, text).unwrap();
}

/// A store holding one skill, pinned, and its snapshot.
fn one_skill_snapshot(layout: &Layout, name: &str, pin: &str) -> (String, PathBuf) {
    let store = Store::open(layout).unwrap();
    let skill_dir = layout.skills().join(name);
    std::fs::create_dir_all(skill_dir.join("nested")).unwrap();
    std::fs::write(
        skill_dir.join("SKILL.md"),
        format!("---\nname: {name}\ndescription: An injected skill.\n---\n"),
    )
    .unwrap();
    std::fs::write(skill_dir.join("nested/notes.md"), "deeper\n").unwrap();
    let sha = store.pin(pin).unwrap();
    let snapshot = store.snapshot(layout, &sha).unwrap();
    (sha, snapshot)
}

#[test]
fn an_injected_skill_stays_off_the_branch_and_a_skill_the_run_wrote_beside_it_does_not() {
    // Map ticket 71: what Hydra injected never reaches a branch, and nothing
    // else under the root is Hydra's. The run deleted Hydra's ignore file, wrote
    // into the injected copy, and authored a skill of its own next to it.
    let fixture = Fixture::new("skill-root");
    let layout = fixture.layout();
    let (sha, snapshot) =
        one_skill_snapshot(&layout, "injected", "01a07730-a1b2-7c3d-8e4f-500000000010");

    let run = "01a07730-a1b2-7c3d-8e4f-500000000011";
    let checkout = fixture.checkout(run);
    skills::inject(&checkout, Provider::Codex, &snapshot, &sha).unwrap();
    write_skill_run(&layout, run, "codex", &["injected"], &sha);

    let root = checkout.join(".codex/skills");
    std::fs::remove_file(root.join(".gitignore")).unwrap();
    Command::new("chmod")
        .args(["-R", "u+w"])
        .arg(&root)
        .status()
        .unwrap();
    std::fs::write(root.join("injected/planted.md"), "not on the branch\n").unwrap();
    std::fs::create_dir_all(root.join("authored")).unwrap();
    std::fs::write(root.join("authored/SKILL.md"), "the run's own skill\n").unwrap();

    let reaped = reap_one(&layout, run, false);
    let swept = worktree_of(&reaped[0]);
    assert!(swept.committed.is_some(), "reap removed without committing");
    assert!(!checkout.exists());

    let branch = format!("hydra/{run}");
    assert_eq!(
        git(
            &fixture.repo,
            &["show", &format!("{branch}:.codex/skills/authored/SKILL.md")]
        ),
        "the run's own skill\n"
    );
    let tree = git(&fixture.repo, &["ls-tree", "-r", "--name-only", &branch]);
    assert!(
        !tree.contains(".codex/skills/injected"),
        "an injected skill reached the branch: {tree}"
    );
}

#[test]
fn reap_removes_a_codex_checkout_whose_skill_copy_is_read_only() {
    // The copy is 0444 files in 0555 directories. Neither `git clean` nor `git
    // worktree remove` can unlink an entry of a directory with no write bit, so
    // reap has to give the bit back before it removes anything. Hydra's ignore
    // file is still in place, and a skill the run authored under it is still
    // committed, as is untracked work elsewhere.
    let fixture = Fixture::new("codex-copy");
    let layout = fixture.layout();
    let (sha, snapshot) =
        one_skill_snapshot(&layout, "copied", "01a07730-a1b2-7c3d-8e4f-500000000012");

    let run = "01a07730-a1b2-7c3d-8e4f-500000000013";
    let checkout = fixture.checkout(run);
    skills::inject(&checkout, Provider::Codex, &snapshot, &sha).unwrap();
    write_skill_run(&layout, run, "codex", &["copied"], &sha);
    assert!(!checkout.join(".codex/skills/copied").is_symlink());
    std::fs::create_dir_all(checkout.join(".codex/skills/authored")).unwrap();
    std::fs::write(checkout.join(".codex/skills/authored/SKILL.md"), "mine\n").unwrap();
    std::fs::create_dir_all(checkout.join("new-dir")).unwrap();
    std::fs::write(checkout.join("new-dir/work.txt"), "kept\n").unwrap();

    let reaped = reap_one(&layout, run, false);
    let swept = worktree_of(&reaped[0]);
    assert!(swept.committed.is_some(), "reap removed without committing");
    assert!(swept.removed, "reap left the checkout");
    assert!(!checkout.exists(), "the checkout is still on disk");

    let branch = format!("hydra/{run}");
    assert_eq!(
        git(
            &fixture.repo,
            &["show", &format!("{branch}:new-dir/work.txt")]
        ),
        "kept\n"
    );
    let tree = git(&fixture.repo, &["ls-tree", "-r", "--name-only", &branch]);
    assert!(tree.contains(".codex/skills/authored/SKILL.md"), "{tree}");
    assert!(
        !tree.contains(".codex/skills/copied") && !tree.contains(".codex/skills/.gitignore"),
        "Hydra's copy or ignore file reached the branch: {tree}"
    );
}

#[test]
fn a_file_planted_inside_an_injected_skill_and_nothing_else_is_not_a_commit() {
    // `git status` names the file, not the injected directory, so the dirt check
    // has to count a path below an excluded one as excluded, or reap stages
    // nothing and `git commit` refuses an empty commit.
    let fixture = Fixture::new("skill-root-only");
    let layout = fixture.layout();
    let (sha, snapshot) =
        one_skill_snapshot(&layout, "planted", "01a07730-a1b2-7c3d-8e4f-500000000014");

    let run = "01a07730-a1b2-7c3d-8e4f-500000000015";
    let checkout = fixture.checkout(run);
    skills::inject(&checkout, Provider::Codex, &snapshot, &sha).unwrap();
    write_skill_run(&layout, run, "codex", &["planted"], &sha);
    let root = checkout.join(".codex/skills");
    std::fs::remove_file(root.join(".gitignore")).unwrap();
    Command::new("chmod")
        .args(["-R", "u+w"])
        .arg(&root)
        .status()
        .unwrap();
    std::fs::write(root.join("planted/hostile.md"), "not on the branch\n").unwrap();

    let reaped = reap_one(&layout, run, false);
    let swept = worktree_of(&reaped[0]);
    assert!(
        swept.committed.is_none(),
        "an injected skill alone was committed"
    );
    assert!(swept.removed);
    assert!(!checkout.exists());
}

#[test]
fn with_no_record_to_name_the_injected_skills_the_whole_root_is_held_back() {
    // The names come off line 0. Without it reap cannot tell Hydra's entries
    // from the run's, and holds back every skill root rather than risk one.
    let fixture = Fixture::new("skill-root-no-record");
    let layout = fixture.layout();
    let run = "01a07730-a1b2-7c3d-8e4f-500000000016";
    let checkout = fixture.checkout(run);
    std::fs::create_dir_all(checkout.join(".claude/skills/unknown")).unwrap();
    std::fs::write(checkout.join(".claude/skills/unknown/SKILL.md"), "?\n").unwrap();
    std::fs::write(checkout.join("child.txt"), "the edit\n").unwrap();

    let reaped = reap_one(&layout, run, false);
    let swept = worktree_of(&reaped[0]);
    assert!(swept.committed.is_some());
    let tree = git(
        &fixture.repo,
        &["ls-tree", "-r", "--name-only", &format!("hydra/{run}")],
    );
    assert!(tree.contains("child.txt"), "{tree}");
    assert!(!tree.contains(".claude/skills"), "{tree}");
}

#[test]
fn a_line_zero_reap_cannot_read_holds_every_root_back() {
    // Build ticket 101, the third panel's H18. Reap reads line 0 through a
    // typed view, so a `skills` block in a shape it does not know, or a
    // provider it cannot name, is a line it could not read. That is the
    // no-record case, and every root is held back. Read as loose `Value`
    // paths, both shapes came out as "nothing was injected", and the
    // injected copy was committed.
    for (name, provider, linked) in [
        ("linked-not-a-list", "codex", json!("injected")),
        ("provider-unknown", "hermes", json!(["injected"])),
    ] {
        let fixture = Fixture::new(&format!("skill-root-{name}"));
        let layout = fixture.layout();
        let (sha, snapshot) =
            one_skill_snapshot(&layout, "injected", "01a07730-a1b2-7c3d-8e4f-500000000017");
        let run = "01a07730-a1b2-7c3d-8e4f-500000000018";
        let checkout = fixture.checkout(run);
        skills::inject(&checkout, Provider::Codex, &snapshot, &sha).unwrap();
        write_skill_run(&layout, run, provider, &["injected"], &sha);
        let path = layout.run_file(run);
        let text = std::fs::read_to_string(&path).unwrap();
        let mut lines: Vec<Value> = text
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        lines[0]["hydra"]["skills"]["linked"] = linked;
        let text: String = lines.iter().map(|line| format!("{line}\n")).collect();
        std::fs::write(&path, text).unwrap();

        // Hydra's ignore file would hide the copy on its own, so it goes, the
        // way a run can remove it.
        let root = checkout.join(".codex/skills");
        std::fs::remove_file(root.join(".gitignore")).unwrap();
        Command::new("chmod")
            .args(["-R", "u+w"])
            .arg(&root)
            .status()
            .unwrap();
        std::fs::write(checkout.join("child.txt"), "the edit\n").unwrap();

        let reaped = reap_one(&layout, run, false);
        let swept = worktree_of(&reaped[0]);
        assert!(swept.committed.is_some(), "{name}: nothing was committed");
        let tree = git(
            &fixture.repo,
            &["ls-tree", "-r", "--name-only", &format!("hydra/{run}")],
        );
        assert!(tree.contains("child.txt"), "{name}: {tree}");
        assert!(
            !tree.contains(".codex/skills"),
            "{name}: an injected skill reached the branch: {tree}"
        );
    }
}

#[test]
fn a_checkout_holding_only_hydras_own_files_commits_nothing_and_drops_the_branch() {
    let fixture = Fixture::new("only-hydra-files");
    std::fs::create_dir_all(fixture.repo.join(".cursor")).unwrap();
    std::fs::write(fixture.repo.join(".cursor/.gitignore"), "rules/\n").unwrap();
    git(&fixture.repo, &["add", "-A"]);
    git(
        &fixture.repo,
        &["commit", "-qm", "the checkout's own ignore file"],
    );

    let layout = fixture.layout();
    let run = "01a07730-a1b2-7c3d-8e4f-500000000002";
    let checkout = fixture.checkout(run);
    std::fs::write(checkout.join(".cursor/hooks.json"), "{}\n").unwrap();
    std::fs::write(checkout.join(".cursor/mcp.json"), "{}\n").unwrap();

    let reaped = reap_one(&layout, run, false);
    let swept = worktree_of(&reaped[0]);
    assert!(
        swept.committed.is_none(),
        "a checkout holding only Hydra's files was committed"
    );
    assert!(swept.removed);
    assert!(swept.branch_removed);
    assert!(!checkout.exists());
    assert!(
        !fixture.branches().contains(run),
        "an empty branch stayed: {}",
        fixture.branches()
    );
}

#[test]
fn a_branch_with_nothing_on_it_goes_and_one_with_a_commit_stays() {
    // The difference is `git branch -d` and not a rule of ours, which is what
    // makes it hold: the branch survives exactly when deleting it would lose the
    // commit reap just made.
    let fixture = Fixture::new("branches");
    let layout = fixture.layout();

    let quiet = "01a07218-8817-7581-9ddb-47565e5d5ba7";
    let busy = "01a0721b-2883-7c30-ab56-f06ba0027cb8";
    fixture.checkout(quiet);
    let busy_tree = fixture.checkout(busy);
    std::fs::write(busy_tree.join("child.txt"), "work\n").unwrap();

    let quiet_done = reap_one(&layout, quiet, false);
    let busy_done = reap_one(&layout, busy, false);

    assert!(worktree_of(&quiet_done[0]).committed.is_none());
    assert!(worktree_of(&quiet_done[0]).branch_removed);
    assert!(worktree_of(&busy_done[0]).committed.is_some());
    assert!(!worktree_of(&busy_done[0]).branch_removed);

    let branches = fixture.branches();
    assert!(!branches.contains(quiet), "{branches}");
    assert!(branches.contains(busy), "{branches}");

    // In git's own words. A boolean here would say "it holds commits" for a branch
    // checked out in another worktree too, which `--delete-branch` cannot fix
    // either, so the human would be told the wrong reason and handed a useless flag.
    let said = hydra::reap::report(&busy_done);
    assert!(said.contains("not fully merged"), "{said}");
}

#[test]
fn the_flag_is_what_drops_a_branch_holding_work() {
    let fixture = Fixture::new("delete-branch");
    let layout = fixture.layout();
    let run = "01a07307-2693-7ef0-b572-a565ffcc3190";
    let checkout = fixture.checkout(run);
    std::fs::write(checkout.join("child.txt"), "work\n").unwrap();

    let reaped = reap_one(&layout, run, true);
    assert!(
        worktree_of(&reaped[0]).committed.is_some(),
        "the flag skipped the commit"
    );
    assert!(worktree_of(&reaped[0]).branch_removed);
    assert!(!fixture.branches().contains(run), "{}", fixture.branches());
}

#[test]
fn a_live_runs_checkout_is_never_reaped() {
    // Runs are detached, so a human can type `hydra reap` while one is writing.
    // Removing the checkout would pull it out from under a running child.
    let fixture = Fixture::new("live");
    let layout = fixture.layout();
    let run = "01a07308-39e3-79c2-8901-ac052723b79a";
    let checkout = fixture.checkout(run);
    write_run(&layout, run, Some(std::process::id()), false);

    let named = reap_one(&layout, run, false);
    assert!(
        matches!(named[0], Reaped::Left { .. }),
        "a live run was reaped"
    );
    assert!(checkout.is_dir());

    // And the sweep leaves it out entirely rather than reporting it as skipped.
    let swept = hydra::reap::reap(&layout, None, false, NO_GRACE).unwrap();
    assert!(swept.is_empty(), "the sweep picked up a live run");
    assert!(checkout.is_dir());
}

/// A `setsid` child that keeps a cwd or an open file under a checkout. Drop
/// kills it, so a panic cannot leave it behind.
struct HeldProcess {
    child: Option<std::process::Child>,
}

impl HeldProcess {
    fn with_cwd(dir: &Path) -> Self {
        Self::spawn(dir, None)
    }

    fn with_file(path: &Path) -> Self {
        let dir = path.parent().expect("held file has a parent");
        Self::spawn(dir, Some(path))
    }

    fn spawn(dir: &Path, file: Option<&Path>) -> Self {
        let mut command = Command::new("/usr/bin/python3");
        command.arg("-c");
        if let Some(path) = file {
            command
                .arg(
                    "import os, sys, time\n\
                     try:\n\
                     \tos.setsid()\n\
                     except OSError:\n\
                     \tpass\n\
                     f = open(sys.argv[1])\n\
                     time.sleep(120)",
                )
                .arg(path);
        } else {
            command.arg(
                "import os, time\n\
                 try:\n\
                 \tos.setsid()\n\
                 except OSError:\n\
                 \tpass\n\
                 time.sleep(120)",
            );
        }
        let child = command
            .current_dir(dir)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("python3 starts a holder");
        Self { child: Some(child) }
    }

    fn pid(&self) -> u32 {
        self.child.as_ref().expect("holder still owned").id()
    }

    fn kill(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

impl Drop for HeldProcess {
    fn drop(&mut self) {
        self.kill();
    }
}

fn wait_until_held(roots: &[&Path], pid: u32) {
    let roots: Vec<PathBuf> = roots.iter().map(|p| p.to_path_buf()).collect();
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        let found = hydra::procinfo::holders(&roots).unwrap();
        if found.iter().any(|h| h.pid == pid) {
            return;
        }
        if Instant::now() >= deadline {
            panic!("pid {pid} never held {}", roots[0].display());
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn assert_holder_refused(reaped: &[Reaped], pid: u32) {
    match &reaped[0] {
        Reaped::Failed { why, .. } => {
            assert!(
                why.contains(&format!("pid {pid}")),
                "refusal did not name pid {pid}: {why}"
            );
            assert!(
                why.contains("hydra stop"),
                "refusal did not name hydra stop: {why}"
            );
            assert!(why.contains("kill"), "refusal did not name kill: {why}");
        }
        Reaped::Left { why, .. } => panic!("holder was left rather than refused: {why}"),
        Reaped::Done(_) => panic!("holder was reaped"),
    }
}

#[test]
fn reap_refuses_a_setsid_process_whose_cwd_is_the_worktree() {
    let fixture = Fixture::new("holder-cwd");
    let layout = fixture.layout();
    let held = "01a17edc-0373-7000-8000-000000000001";
    let free = "01a17edc-0373-7000-8000-000000000002";
    let checkout = fixture.checkout(held);
    fixture.checkout(free);
    write_run(&layout, held, None, true);
    write_run(&layout, free, None, true);
    std::fs::write(checkout.join("child.txt"), "work\n").unwrap();

    let mut holder = HeldProcess::with_cwd(&checkout);
    wait_until_held(&[&checkout], holder.pid());

    let named = reap_one(&layout, held, false);
    assert_holder_refused(&named, holder.pid());
    assert!(checkout.is_dir(), "a held checkout was removed");

    let swept = hydra::reap::reap(&layout, None, false, NO_GRACE).unwrap();
    assert!(
        swept
            .iter()
            .any(|item| matches!(item, Reaped::Failed { run_id, .. } if run_id == held)),
        "the sweep did not refuse the held run"
    );
    assert!(
        swept.iter().any(|item| match item {
            Reaped::Done(done) => done.run_id == free,
            _ => false,
        }),
        "the sweep skipped the free run"
    );
    assert!(layout.worktree(held).is_dir());
    assert!(!layout.worktree(free).exists());

    holder.kill();
    let after = reap_one(&layout, held, false);
    assert_work_on_branch(&fixture, held, &after);
}

#[test]
fn reap_refuses_a_process_that_holds_a_file_in_the_private_git_dir() {
    let fixture = Fixture::new("holder-private");
    let layout = fixture.layout();
    let run = "01a17edc-0373-7000-8000-000000000003";
    let private = planted(&fixture, run);
    write_run(&layout, run, None, true);
    let file = private.join("HEAD");

    let mut holder = HeldProcess::with_file(&file);
    wait_until_held(&[&private], holder.pid());

    let named = reap_one(&layout, run, false);
    assert_holder_refused(&named, holder.pid());
    assert!(
        layout.worktree(run).is_dir(),
        "a checkout held through the private git directory was removed"
    );

    holder.kill();
    let after = reap_one(&layout, run, false);
    assert_work_on_branch(&fixture, run, &after);
}

#[test]
fn the_sweep_takes_every_checkout_no_live_run_is_using() {
    let fixture = Fixture::new("sweep");
    let layout = fixture.layout();
    let ended = "01a07340-67a3-7c41-8c69-8d1e28bb58a7";
    let orphan = "01a07357-2c4e-7083-9ff9-2ebb23f6a3f2";

    fixture.checkout(ended);
    write_run(&layout, ended, Some(std::process::id()), true);
    // A checkout whose record was pruned. Nothing can explain it, and reap is
    // what a human types about it.
    fixture.checkout(orphan);

    let reaped = hydra::reap::reap(&layout, None, false, NO_GRACE).unwrap();
    assert_eq!(reaped.len(), 2);
    assert!(hydra::reap::failures(&reaped) == 0);
    assert!(!layout.worktree(ended).exists());
    assert!(!layout.worktree(orphan).exists());

    // Reap appends to no run file and removes none. Contract 6 gives a file one
    // writer, and the record is what survives the checkout.
    assert!(layout.run_file(ended).is_file());
}

#[test]
fn a_worktree_with_no_spawn_directory_still_reaps() {
    // The three oldest runs on this machine predate the spawn directory, so the
    // pair reap assumes is not always there.
    let fixture = Fixture::new("no-spawn");
    let layout = fixture.layout();
    let run = "01a07357-58a5-79e2-a25a-3df578a750b4";
    fixture.checkout(run);
    std::fs::remove_dir_all(layout.spawn(run)).unwrap();

    let reaped = reap_one(&layout, run, false);
    assert!(worktree_of(&reaped[0]).removed);
    assert!(!as_done(&reaped[0]).spawn_removed);
    assert!(!layout.worktree(run).exists());
}

#[test]
fn a_spawn_directory_with_no_checkout_is_reaped_on_its_own() {
    // A run outside a repo gets a spawn directory and no worktree, so a sweep
    // that only walked `worktrees/` would leave half of harness spec 5's pair.
    let fixture = Fixture::new("lone-spawn");
    let layout = fixture.layout();
    let run = "01a073ef-701c-7522-b5bb-56d5ecba0f57";
    std::fs::create_dir_all(layout.spawn(run)).unwrap();
    std::fs::write(layout.gate_script(run), "#!/usr/bin/env python3\n").unwrap();

    // Through the sweep rather than by name, because finding it at all is what
    // is under test: `worktrees/` holds nothing for this run.
    let reaped = hydra::reap::reap(&layout, None, false, NO_GRACE).unwrap();
    assert_eq!(reaped.len(), 1);
    assert!(as_done(&reaped[0]).spawn_removed);
    assert!(as_done(&reaped[0]).worktree.is_none());
    assert!(!layout.spawn(run).exists());
}

#[test]
fn a_sweep_leaves_pending_reservations_alone() {
    // Build ticket 81. A reservation is held by a launcher that has not made a
    // checkout yet, so a sweep that took it would let the next launch past the
    // cap. Reservations live beside `spawn/`, which is what keeps them out.
    let fixture = Fixture::new("reservations");
    let layout = fixture.layout();
    let held = hydra::live::reserve(&layout, "pending", 4).unwrap();
    let reaped = hydra::reap::reap(&layout, None, false, NO_GRACE).unwrap();
    assert!(reaped.is_empty(), "the sweep reaped {} runs", reaped.len());
    assert!(layout.reservations().join("pending.json").exists());
    assert!(layout.reservations().join(".lock").exists());
    drop(held);
}

#[test]
fn a_launch_in_progress_is_not_swept() {
    // `hydra run` creates the checkout, then the spawn directory, and only then
    // does the detached half write line 0. In that window neither has a record
    // and both look exactly like a run whose record was pruned. A sweep that
    // took them would remove the checkout of a run that is about to start.
    let fixture = Fixture::new("racing");
    let layout = fixture.layout();
    fixture.checkout("01a07471-0a3e-74d2-b576-153ac0e7427b");
    std::fs::create_dir_all(layout.spawn("01a07484-1667-7500-b299-2889144b84c1")).unwrap();

    let reaped = hydra::reap::reap(&layout, None, false, hydra::run::LAUNCH_BUDGET).unwrap();
    assert!(reaped.is_empty(), "a launch in progress was reaped");
}

#[test]
fn a_record_that_exists_and_says_nothing_is_a_launch_and_not_a_corpse() {
    // `hydra watch` creates the run file, then spawns the child, then writes line
    // 0, so for as long as the launch budget allows there is an empty record
    // beside a checkout being seeded. It reads as abandoned with a null pid, which
    // is indistinguishable from a run that died, and anything removing things on
    // that reading takes a run that is starting: the checkout out from under the
    // child, or the record out from under the writer that has it open.
    let fixture = Fixture::new("starting");
    let layout = fixture.layout();
    let run = "01a07500-1111-7000-8000-000000000001";
    fixture.checkout(run);
    std::fs::write(layout.run_file(run), "").unwrap();

    let swept = hydra::reap::reap(&layout, None, false, hydra::run::LAUNCH_BUDGET).unwrap();
    assert!(
        swept.is_empty(),
        "the checkout of a starting run was reaped"
    );
    assert!(layout.worktree(run).is_dir());

    let pruned =
        hydra::prune::prune(&layout, &[], false, false, hydra::run::LAUNCH_BUDGET).unwrap();
    assert!(pruned.is_empty(), "the record of a starting run was pruned");
    assert!(layout.run_file(run).is_file());

    // Named explicitly it is reported rather than acted on, `--delete` included.
    let named = hydra::prune::prune(
        &layout,
        &[run.to_string()],
        true,
        false,
        hydra::run::LAUNCH_BUDGET,
    )
    .unwrap();
    match &named[0] {
        Pruned::Left { why, .. } => assert!(why.contains("still starting"), "{why}"),
        _ => panic!("a starting run's record was deleted"),
    }
    assert!(layout.run_file(run).is_file());

    // Once the budget is past, both take it. Age is the only evidence there is.
    assert_eq!(
        hydra::reap::reap(&layout, None, false, NO_GRACE)
            .unwrap()
            .len(),
        1
    );
    assert_eq!(prune(&layout, &[], false, false).len(), 1);
}

#[test]
fn a_refused_spawn_is_not_mistaken_for_a_launch_in_progress() {
    // Contract 5.2's null pid is a run that never had a process, and the closing
    // line is what says so. Reading it as a record that has not spoken yet would
    // hold every refused spawn out of reach for the length of the grace.
    let fixture = Fixture::new("refused");
    let layout = fixture.layout();
    write_run(&layout, "01a07500-1111-7000-8000-000000000002", None, true);

    let pruned =
        hydra::prune::prune(&layout, &[], false, false, hydra::run::LAUNCH_BUDGET).unwrap();
    assert!(matches!(pruned[0], Pruned::Moved { .. }), "not moved");
}

#[test]
fn a_record_whose_envelope_disagrees_with_its_filename_is_handled_by_path() {
    // `read_view` takes the run id off line 0 and only falls back to the filename.
    // Rebuilding the path from that id moves a file that is not there and leaves
    // the one that is, and on the named side it finds no view at all and skips the
    // liveness check, which is the half that loses a live run's record.
    let fixture = Fixture::new("prune-mismatch");
    let layout = fixture.layout();
    let on_disk = "01a07500-1111-7000-8000-000000000003";
    let in_the_envelope = "01a07500-1111-7000-8000-000000000004";

    // Ended, and claiming to be a different run.
    write_run(&layout, on_disk, Some(std::process::id()), true);
    let text = std::fs::read_to_string(layout.run_file(on_disk))
        .unwrap()
        .replace(on_disk, in_the_envelope);
    std::fs::write(layout.run_file(on_disk), text).unwrap();

    // Live, and claiming to be a different run.
    let live_on_disk = "01a07500-1111-7000-8000-000000000005";
    write_run(&layout, live_on_disk, Some(std::process::id()), false);
    let text = std::fs::read_to_string(layout.run_file(live_on_disk))
        .unwrap()
        .replace(live_on_disk, "01a07500-1111-7000-8000-000000000006");
    std::fs::write(layout.run_file(live_on_disk), text).unwrap();

    let swept = prune(&layout, &[], false, false);
    assert_eq!(swept.len(), 1, "the sweep took the live run too");
    assert!(
        !layout.run_file(on_disk).exists(),
        "the file on disk stayed"
    );
    assert!(
        layout.pruned_file(on_disk).is_file(),
        "it was filed under the envelope's id rather than its own name"
    );

    let named = prune(&layout, &[live_on_disk.to_string()], false, false);
    match &named[0] {
        Pruned::Left { why, .. } => assert!(why.contains("still writing"), "{why}"),
        _ => panic!("a live record was pruned because its envelope named another run"),
    }
    assert!(layout.run_file(live_on_disk).is_file());
}

#[test]
fn a_detached_head_is_refused_rather_than_committed_into_nothing() {
    // A commit on a detached HEAD is reachable from nothing once the checkout is
    // gone, which is the one shape where committing first saves nothing.
    let fixture = Fixture::new("detached");
    let layout = fixture.layout();
    let run = "01a07470-e247-7283-8aae-62d72aa86eda";
    let checkout = fixture.checkout(run);
    git(&checkout, &["checkout", "-q", "--detach"]);
    std::fs::write(checkout.join("child.txt"), "work\n").unwrap();

    let reaped = reap_one(&layout, run, false);
    match &reaped[0] {
        Reaped::Failed { why, .. } => assert!(why.contains("detached HEAD"), "{why}"),
        _ => panic!("a detached HEAD was reaped"),
    }
    assert!(checkout.is_dir(), "the checkout went anyway");
    assert!(layout.spawn(run).is_dir(), "the pair came apart");
    assert_eq!(hydra::reap::failures(&reaped), 1);
}

#[test]
fn reaping_a_run_that_was_never_there_does_nothing_and_says_so() {
    let fixture = Fixture::new("absent");
    // Was `"nobody"` until build ticket 39. The behaviour this pins is that an
    // absent run is benign, and a well-formed id is now the only way to say
    // "absent run" rather than "typo": a short string that matches no record is
    // an error, and the test below pins that. The uuid is deliberately one no
    // record on any machine carries.
    let absent = "00000000-0000-7000-8000-000000000000";
    let reaped = reap_one(&fixture.layout(), absent, false);
    let done = as_done(&reaped[0]);
    assert!(done.worktree.is_none() && !done.spawn_removed);
    assert_eq!(hydra::reap::failures(&reaped), 0);
}

#[test]
fn reaping_a_prefix_that_names_no_run_is_an_error_rather_than_nothing_to_do() {
    let fixture = Fixture::new("typo");
    // `Reaped` is not `Debug`, so the Ok side is mapped away before asserting.
    let err = hydra::reap::reap(&fixture.layout(), Some("nobody"), false, NO_GRACE)
        .map(|_| ())
        .expect_err("a prefix matching no record is a typo, not an absent run");
    assert!(err.to_string().contains("no run matches"), "{err}");
}

// --- reap and a curate pass's checkout -----------------------------------

#[test]
fn a_curate_proposal_is_on_its_branch_before_the_checkout_goes() {
    // Map ticket 40 decision 5. The store checkout is the other directory a
    // model writes into that reap removes, so it gets the guarantee the
    // worktree has: the commit lands first, and `git worktree remove` without
    // `--force` is what stops a removal the commit did not precede. Measured
    // before this ticket: reap took the worktree, the spawn directory and the
    // snapshot, and left the checkout with its uncommitted `proposal.md`.
    let fixture = Fixture::new("curate-checkout");
    let layout = fixture.layout();
    let run = "01a07615-4c11-7a53-9d1e-2b7a51c0d3f1";
    let worktree = fixture.checkout(run);
    let checkout = fixture.store_checkout(run);
    std::fs::write(checkout.join("proposal.md"), "a skill the curator wrote\n").unwrap();

    let reaped = reap_one(&layout, run, false);
    let swept = store_checkout_of(&reaped[0]);

    assert!(
        swept.committed.is_some(),
        "reap removed the checkout without committing"
    );
    assert!(swept.removed);
    assert!(!checkout.exists(), "the checkout is still there");

    // The bytes, read back off the branch in the store.
    let branch = format!("rungs/curate/{run}");
    assert_eq!(
        git(
            &fixture.store(),
            &["show", &format!("{branch}:proposal.md")]
        ),
        "a skill the curator wrote\n"
    );
    assert!(
        fixture.store_branches().contains(&branch),
        "the branch holding the proposal was deleted: {}",
        fixture.store_branches()
    );
    assert!(!swept.branch_removed);

    // The run's own pair goes in the same call. A curate run has all three, and
    // a human who typed one run id should not have to type it again.
    assert!(!worktree.exists());
    assert!(!layout.spawn(run).exists());

    // And the human is told where the proposal went, because that branch is the
    // whole of what reap saved.
    let said = hydra::reap::report(&reaped);
    assert!(said.contains(&branch), "{said}");
}

#[test]
fn a_checkout_nothing_wrote_to_goes_and_takes_its_branch() {
    // A pass whose curator proposed nothing leaves a clean checkout on a branch
    // with nothing on it. `git branch -d` decides that rather than this module,
    // which is what `curate::propose` already does to an empty pass.
    let fixture = Fixture::new("curate-empty");
    let layout = fixture.layout();
    let run = "01a07616-9d02-7b84-8f2c-6e0d84a1b7c2";
    let checkout = fixture.store_checkout(run);

    let reaped = reap_one(&layout, run, false);
    let swept = store_checkout_of(&reaped[0]);

    assert!(swept.committed.is_none(), "an empty checkout was committed");
    assert!(swept.removed);
    assert!(swept.branch_removed);
    assert!(!checkout.exists());
    assert!(
        !fixture.store_branches().contains(run),
        "an empty branch stayed: {}",
        fixture.store_branches()
    );
}

#[test]
fn a_live_curate_runs_checkout_of_the_store_is_never_reaped() {
    // A pass whose run outlived its call is exactly this shape: the curator is
    // still writing and the checkout holds half a proposal. Rungs spec 3.5
    // commits nothing in that case, and neither may a reap typed beside it.
    let fixture = Fixture::new("curate-live");
    let layout = fixture.layout();
    let run = "01a07617-2ea3-7cd5-b013-3f9b62d5e8a3";
    let checkout = fixture.store_checkout(run);
    write_run(&layout, run, Some(std::process::id()), false);

    let named = reap_one(&layout, run, false);
    assert!(
        matches!(named[0], Reaped::Left { .. }),
        "a live pass's checkout was reaped"
    );
    assert!(checkout.is_dir());

    // And the sweep leaves it out rather than reporting it as skipped.
    let swept = hydra::reap::reap(&layout, None, false, NO_GRACE).unwrap();
    assert!(swept.is_empty(), "the sweep picked up a live pass");
    assert!(checkout.is_dir());
}

#[test]
fn the_sweep_finds_a_checkout_with_no_worktree_and_no_spawn_directory() {
    // `hydra curate` mints the run id and takes the checkout before it calls
    // the launch, so a launch that failed leaves this and nothing else. A sweep
    // that walked only `worktrees/` and `spawn/` would never see it.
    let fixture = Fixture::new("curate-sweep");
    let layout = fixture.layout();
    let run = "01a07618-7b44-7de6-a124-4c8d73e6f9b4";
    let checkout = fixture.store_checkout(run);

    let reaped = hydra::reap::reap(&layout, None, false, NO_GRACE).unwrap();
    assert_eq!(reaped.len(), 1);
    assert!(store_checkout_of(&reaped[0]).removed);
    let done = as_done(&reaped[0]);
    assert!(done.worktree.is_none() && !done.spawn_removed);
    assert!(!checkout.exists());
}

#[test]
fn the_stores_own_working_tree_is_never_written_by_a_reap() {
    // Rungs spec 3.6: the store's working tree is where a concurrent
    // `Store::pin` commits without asking, so reap runs `git worktree remove`
    // and `git branch -d` from it and puts nothing in it. The registration goes
    // with the directory, or `git worktree list` keeps naming a path that is not
    // there.
    let fixture = Fixture::new("curate-store");
    let layout = fixture.layout();
    let run = "01a07619-1c85-7ef7-8235-5daf84f70ac5";
    let store = fixture.store();
    let head = git(&store, &["rev-parse", "HEAD"]);
    let checkout = fixture.store_checkout(run);
    std::fs::write(checkout.join("proposal.md"), "a proposal\n").unwrap();

    reap_one(&layout, run, false);

    assert_eq!(
        git(&store, &["status", "--porcelain"]),
        "",
        "reap left changes in the store's working tree"
    );
    assert_eq!(
        git(&store, &["rev-parse", "HEAD"]),
        head,
        "the store's own branch moved"
    );
    let listed = git(&store, &["worktree", "list"]);
    assert!(
        !listed.contains(run),
        "the checkout is still registered: {listed}"
    );
}

// --- prune ---------------------------------------------------------------

#[test]
fn a_pruned_record_is_moved_and_not_gone() {
    let fixture = Fixture::new("prune-moves");
    let layout = fixture.layout();
    let run = "01a07471-0a3e-74d2-b576-153ac0e7427b";
    let path = write_run(&layout, run, Some(std::process::id()), true);
    let before = std::fs::read_to_string(&path).unwrap();

    let pruned = prune(&layout, &[], false, false);
    assert!(matches!(pruned[0], Pruned::Moved { .. }), "not moved");
    assert!(!path.exists());
    assert_eq!(
        std::fs::read_to_string(layout.pruned_file(run)).unwrap(),
        before,
        "the record changed on the way out of runs/"
    );
}

#[test]
fn only_the_flag_deletes() {
    let fixture = Fixture::new("prune-deletes");
    let layout = fixture.layout();
    let run = "01a07484-1667-7500-b299-2889144b84c1";
    write_run(&layout, run, Some(std::process::id()), true);

    let pruned = prune(&layout, &[run.to_string()], true, false);
    assert!(matches!(pruned[0], Pruned::Deleted { .. }));
    assert!(!layout.run_file(run).exists());
    assert!(
        !layout.pruned_file(run).exists(),
        "a delete left a copy behind, so the flag means nothing"
    );
}

#[test]
fn a_dry_run_changes_nothing() {
    let fixture = Fixture::new("prune-dry");
    let layout = fixture.layout();
    let run = "01a07217-6007-7733-9ce2-7bb31830eab8";
    write_run(&layout, run, Some(std::process::id()), true);

    let pruned = prune(&layout, &[], false, true);
    match &pruned[0] {
        Pruned::Would { what, .. } => assert!(what.starts_with("move "), "{what}"),
        _ => panic!("a dry run did something"),
    }
    assert!(layout.run_file(run).is_file());
    assert!(!layout.pruned_file(run).exists());
}

#[test]
fn a_live_runs_record_is_never_pruned() {
    // Contract 6 gives a run file one writer, and a rename does not take the file
    // away from an open descriptor: the writer would keep appending into a path
    // nothing looks in.
    let fixture = Fixture::new("prune-live");
    let layout = fixture.layout();
    let live = "01a07218-8817-7581-9ddb-47565e5d5ba7";
    let over = "01a0721b-2883-7c30-ab56-f06ba0027cb8";
    write_run(&layout, live, Some(std::process::id()), false);
    write_run(&layout, over, Some(std::process::id()), true);

    let swept = prune(&layout, &[], false, false);
    assert_eq!(swept.len(), 1, "the sweep took a live run");
    assert!(layout.run_file(live).is_file());
    assert!(!layout.run_file(over).is_file());

    // Named explicitly it is reported rather than skipped quietly, because the
    // human asked about that one.
    let named = prune(&layout, &[live.to_string()], false, false);
    match &named[0] {
        Pruned::Left { why, .. } => assert!(why.contains("still writing"), "{why}"),
        _ => panic!("a live record was pruned"),
    }
    assert!(layout.run_file(live).is_file());
}

#[test]
fn pruning_the_same_run_twice_refuses_rather_than_overwrites() {
    // The second copy is a different run's worth of evidence only by accident of
    // sharing a name, and overwriting would destroy what the first prune saved.
    let fixture = Fixture::new("prune-twice");
    let layout = fixture.layout();
    let run = "01a07307-2693-7ef0-b572-a565ffcc3190";

    write_run(&layout, run, Some(std::process::id()), true);
    prune(&layout, &[], false, false);
    std::fs::write(layout.run_file(run), "a second file with the same name\n").unwrap();

    let again = prune(&layout, &[run.to_string()], false, false);
    assert_eq!(hydra::prune::failures(&again), 1);
    assert!(
        layout.run_file(run).is_file(),
        "the second record went anyway"
    );
    assert!(
        std::fs::read_to_string(layout.pruned_file(run))
            .unwrap()
            .contains("run_start"),
        "the first record was overwritten"
    );
}

#[test]
fn pruning_a_run_that_is_not_there_says_so() {
    let fixture = Fixture::new("prune-absent");
    let pruned = prune(&fixture.layout(), &["nobody".to_string()], false, false);
    assert_eq!(hydra::prune::failures(&pruned), 1);
    assert!(hydra::prune::report(&pruned).contains("no run record"));
}

#[test]
fn prune_leaves_the_checkout_and_the_spawn_directory_to_reap() {
    let fixture = Fixture::new("prune-scope");
    let layout = fixture.layout();
    let run = "01a07308-a250-7792-9e8b-0d99697c99d9";
    fixture.checkout(run);
    write_run(&layout, run, Some(std::process::id()), true);

    prune(&layout, &[], false, false);
    assert!(layout.worktree(run).is_dir());
    assert!(layout.spawn(run).is_dir());
}

#[test]
fn reap_refuses_a_checkout_on_another_branch_before_committing() {
    let fixture = Fixture::new("foreign-branch");
    let layout = fixture.layout();
    let run = "01a07217-6007-7733-9ce2-7bb31830eab9";
    let checkout = fixture.checkout(run);
    git(&checkout, &["checkout", "-qb", "human"]);
    std::fs::write(checkout.join("human.txt"), "human commit\n").unwrap();
    git(&checkout, &["add", "-A"]);
    git(&checkout, &["commit", "-qm", "human work"]);
    let before = git(&fixture.repo, &["rev-parse", "human"]);
    std::fs::write(checkout.join("child.txt"), "child work\n").unwrap();
    let reaped = reap_one(&layout, run, false);
    assert!(matches!(&reaped[0], Reaped::Failed { why, .. } if why.contains("human")));
    assert_eq!(git(&fixture.repo, &["rev-parse", "human"]), before);
    assert!(checkout.is_dir());
    assert!(layout.spawn(run).is_dir());
}

#[cfg(unix)]
fn redirect_git_to_decoy(fixture: &Fixture, checkout: &Path) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let decoy = checkout.join("decoy.git");
    git(
        &fixture.repo,
        &["clone", "--bare", "-q", ".", decoy.to_str().unwrap()],
    );
    let branch = git(checkout, &["symbolic-ref", "HEAD"]);
    git(&decoy, &["symbolic-ref", "HEAD", branch.trim()]);
    let marker = fixture.root.join("decoy-was-used");
    let monitor = checkout.join("decoy-monitor");
    std::fs::write(
        &monitor,
        format!(
            "#!/bin/sh\ntouch '{}'\nprintf 'token\\0'\n",
            marker.display()
        ),
    )
    .unwrap();
    std::fs::set_permissions(&monitor, std::fs::Permissions::from_mode(0o755)).unwrap();
    git(&decoy, &["config", "core.bare", "false"]);
    git(
        &decoy,
        &["config", "core.worktree", checkout.to_str().unwrap()],
    );
    git(
        &decoy,
        &["config", "core.fsmonitor", monitor.to_str().unwrap()],
    );
    // The fsmonitor belt alone would stop the monitor, so the decoy also
    // defines a clean filter the checkout selects: only the pinned directories
    // keep git from reading the decoy's configuration at all.
    let filter = checkout.join("decoy-filter");
    std::fs::write(
        &filter,
        format!("#!/bin/sh\ntouch '{}'\ncat\n", marker.display()),
    )
    .unwrap();
    std::fs::set_permissions(&filter, std::fs::Permissions::from_mode(0o755)).unwrap();
    git(
        &decoy,
        &["config", "filter.decoy.clean", filter.to_str().unwrap()],
    );
    std::fs::write(checkout.join(".gitattributes"), "* filter=decoy\n").unwrap();
    std::fs::write(
        checkout.join(".git"),
        format!("gitdir: {}\n", decoy.display()),
    )
    .unwrap();
    marker
}

#[test]
#[cfg(unix)]
fn reap_ignores_a_rewritten_git_file_and_its_configuration() {
    let fixture = Fixture::new("decoy-reap");
    let layout = fixture.layout();
    let run = "01a07217-6007-7733-9ce2-7bb31830eaba";
    let checkout = fixture.checkout(run);
    let marker = redirect_git_to_decoy(&fixture, &checkout);
    std::fs::write(checkout.join("child.txt"), "real branch\n").unwrap();
    let reaped = reap_one(&layout, run, false);
    assert!(worktree_of(&reaped[0]).committed.is_some());
    assert_eq!(
        git(&fixture.repo, &["show", &format!("hydra/{run}:child.txt")]),
        "real branch\n"
    );
    assert!(!marker.exists(), "decoy configuration executed");
}

#[test]
#[cfg(unix)]
fn run_end_diff_stat_ignores_a_rewritten_git_file_and_its_configuration() {
    let fixture = Fixture::new("decoy-stat");
    let layout = fixture.layout();
    let run_id = "01a07217-6007-7733-9ce2-7bb31830eabb";
    let checkout = fixture.checkout(run_id);
    let base = git(&checkout, &["rev-parse", "HEAD^{tree}"])
        .trim()
        .to_string();
    let file = write_run(&layout, run_id, None, true);
    let text = std::fs::read_to_string(&file).unwrap();
    let mut lines: Vec<Value> = text
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    lines[0]["hydra"]["worktree"]["base_tree"] = json!(base);
    lines[0]["hydra"]["scopes"]["parent_git_dir"] = json!(fixture.repo.join(".git"));
    let text = lines
        .iter()
        .map(|line| format!("{line}\n"))
        .collect::<String>();
    let policies = hydra::policy::PolicySet::discover().unwrap();
    let run = hydra::normalize::read_with(&text, &policies, &layout).unwrap();
    let marker = redirect_git_to_decoy(&fixture, &checkout);
    std::fs::write(checkout.join("child.txt"), "new work\n").unwrap();
    let result = hydra::run::result(
        &run,
        &file,
        Some(&hydra::record::WorktreeRef {
            path: checkout.to_string_lossy().into_owned(),
            branch: format!("hydra/{run_id}"),
            base_tree: Some(base),
        }),
        None,
        &layout,
    );
    assert!(result.diff_stat.is_some());
    assert!(!marker.exists(), "decoy configuration executed");
}

#[test]
fn reap_refuses_a_curate_checkout_on_another_branch() {
    let fixture = Fixture::new("foreign-curate-branch");
    let layout = fixture.layout();
    let run = "01a07217-6007-7733-9ce2-7bb31830eabc";
    let checkout = fixture.store_checkout(run);
    std::fs::create_dir_all(layout.spawn(run)).unwrap();
    git(&checkout, &["checkout", "-qb", "human"]);
    std::fs::write(checkout.join("human.txt"), "human work\n").unwrap();
    git(&checkout, &["add", "-A"]);
    git(&checkout, &["commit", "-qm", "human work"]);
    let before = git(&fixture.store(), &["rev-parse", "human"]);
    std::fs::write(checkout.join("child.txt"), "child work\n").unwrap();
    let reaped = reap_one(&layout, run, false);
    assert!(matches!(&reaped[0], Reaped::Failed { why, .. } if why.contains("human")));
    assert_eq!(git(&fixture.store(), &["rev-parse", "human"]), before);
    assert!(checkout.is_dir());
    assert!(layout.spawn(run).is_dir());
}

#[test]
#[cfg(unix)]
fn a_link_planted_as_the_private_logs_takes_no_reflog_line_to_its_target() {
    let fixture = Fixture::new("logs-link");
    let layout = fixture.layout();
    let run = "01a07217-6007-7733-9ce2-7bb31830eab9";
    let checkout = fixture.checkout(run);
    std::fs::write(checkout.join("child.txt"), "work\n").unwrap();
    let victim = fixture.root.join("victim.txt");
    std::fs::write(&victim, "untouched\n").unwrap();
    let private = fixture.repo.join(".git/worktrees").join(run);
    let _ = std::fs::remove_dir_all(private.join("logs"));
    let fake = fixture.root.join("fake-logs");
    std::fs::create_dir_all(&fake).unwrap();
    std::os::unix::fs::symlink(&victim, fake.join("HEAD")).unwrap();
    std::os::unix::fs::symlink(&fake, private.join("logs")).unwrap();
    let reaped = reap_one(&layout, run, false);
    assert_eq!(hydra::reap::failures(&reaped), 0);
    assert_eq!(std::fs::read_to_string(&victim).unwrap(), "untouched\n");
}

// --- what a run plants in its private git directory, build ticket 367 -------

/// The run's private git directory, `<git>/worktrees/<run-id>/`, with work in
/// the checkout so reap has something to commit.
fn planted(fixture: &Fixture, run: &str) -> PathBuf {
    let checkout = fixture.checkout(run);
    std::fs::write(checkout.join("child.txt"), "work\n").unwrap();
    fixture.repo.join(".git/worktrees").join(run)
}

#[test]
#[cfg(unix)]
fn a_commit_message_link_writes_nothing_through_to_its_target() {
    let fixture = Fixture::new("editmsg-link");
    let run = "01a17ea1-0367-7000-8000-000000000001";
    let private = planted(&fixture, run);
    let victim = fixture.root.join("victim.txt");
    std::fs::write(&victim, "untouched\n").unwrap();
    std::os::unix::fs::symlink(&victim, private.join("COMMIT_EDITMSG")).unwrap();

    let reaped = reap_one(&fixture.layout(), run, false);
    assert_eq!(hydra::reap::failures(&reaped), 0);
    assert_eq!(std::fs::read_to_string(&victim).unwrap(), "untouched\n");
}

#[test]
fn a_planted_merge_head_gives_the_reaped_commit_no_second_parent() {
    let fixture = Fixture::new("merge-head");
    let run = "01a17ea1-0367-7000-8000-000000000002";
    // An orphan, because git reduces a `MERGE_HEAD` that descends from `HEAD`
    // to one parent, and that parent is then the planted commit.
    git(&fixture.repo, &["checkout", "-q", "--orphan", "side"]);
    std::fs::write(fixture.repo.join("side.txt"), "side\n").unwrap();
    git(&fixture.repo, &["add", "-A"]);
    git(&fixture.repo, &["commit", "-qm", "side"]);
    let side = git(&fixture.repo, &["rev-parse", "HEAD"]);
    git(&fixture.repo, &["checkout", "-q", "main"]);
    let private = planted(&fixture, run);
    std::fs::write(private.join("MERGE_HEAD"), &side).unwrap();

    let reaped = reap_one(&fixture.layout(), run, false);
    assert_eq!(hydra::reap::failures(&reaped), 0);
    let parents = git(
        &fixture.repo,
        &["rev-list", "--parents", "-n1", &format!("hydra/{run}")],
    );
    assert_eq!(
        parents.split_whitespace().count(),
        2,
        "the reaped commit is a merge: {parents}"
    );
    assert_eq!(
        parents.split_whitespace().nth(1),
        Some(git(&fixture.repo, &["rev-parse", "main"]).trim())
    );
}

#[test]
fn a_planted_lock_does_not_keep_the_checkout() {
    let fixture = Fixture::new("locked");
    let run = "01a17ea1-0367-7000-8000-000000000003";
    let private = planted(&fixture, run);
    std::fs::write(private.join("locked"), "the run says so\n").unwrap();

    let reaped = reap_one(&fixture.layout(), run, false);
    assert_eq!(
        hydra::reap::failures(&reaped),
        0,
        "{:?}",
        worktree_why(&reaped)
    );
    assert!(!fixture.layout().worktree(run).exists());
}

#[test]
fn a_planted_worktree_config_runs_no_filter_from_reaps_git() {
    let fixture = Fixture::new("config-worktree");
    let run = "01a17ea1-0367-7000-8000-000000000004";
    git(
        &fixture.repo,
        &["config", "core.repositoryformatversion", "1"],
    );
    git(
        &fixture.repo,
        &["config", "extensions.worktreeConfig", "true"],
    );
    let private = planted(&fixture, run);
    let canary = fixture.root.join("canary");
    std::fs::write(
        private.join("config.worktree"),
        format!(
            "[filter \"canary\"]\n\tclean = touch {} && cat\n",
            canary.display()
        ),
    )
    .unwrap();
    std::fs::write(
        fixture.layout().worktree(run).join(".gitattributes"),
        "* filter=canary\n",
    )
    .unwrap();

    let reaped = reap_one(&fixture.layout(), run, false);
    assert_eq!(
        hydra::reap::failures(&reaped),
        0,
        "{:?}",
        worktree_why(&reaped)
    );
    assert!(!canary.exists(), "a filter the run configured ran");
}

/// Reap on a thread, so a git that blocks on a fifo fails the test instead
/// of hanging it.
fn reap_within(fixture: &Fixture, run: &str, what: &str) -> Vec<Reaped> {
    let layout = fixture.layout();
    let (tx, rx) = std::sync::mpsc::channel();
    let reaping = run.to_string();
    std::thread::spawn(move || {
        let _ = tx.send(hydra::reap::reap(&layout, Some(&reaping), false, NO_GRACE));
    });
    rx.recv_timeout(std::time::Duration::from_secs(30))
        .unwrap_or_else(|_| panic!("reap hung on {what}"))
        .unwrap()
}

/// The child's work reached the run's branch.
fn assert_work_on_branch(fixture: &Fixture, run: &str, reaped: &[Reaped]) {
    assert_eq!(
        hydra::reap::failures(reaped),
        0,
        "{:?}",
        worktree_why(reaped)
    );
    assert_eq!(
        git(&fixture.repo, &["show", &format!("hydra/{run}:child.txt")]),
        "work\n"
    );
}

#[test]
#[cfg(unix)]
fn a_fifo_as_the_index_is_removed_without_hanging() {
    let fixture = Fixture::new("fifo-index");
    let run = "01a17ea1-0367-7000-8000-000000000005";
    let private = planted(&fixture, run);
    let index = private.join("index");
    std::fs::remove_file(&index).unwrap();
    let made = Command::new("mkfifo").arg(&index).status().unwrap();
    assert!(made.success());

    let reaped = reap_within(&fixture, run, "a fifo index");
    assert_work_on_branch(&fixture, run, &reaped);
}

// --- the index host git rebuilds, build ticket 371 ---------------------------

/// Grok's recipe from the review of `05a9e21..435f156`. The run stages a
/// blob the checkout does not hold and sets the skip-worktree bit on it.
/// `git add -A` leaves the entry alone, so reap committed the planted bytes
/// and the checkout a reviewer reads showed something else.
#[test]
fn a_skip_worktree_entry_cannot_smuggle_a_blob_onto_the_branch() {
    let fixture = Fixture::new("skip-worktree");
    let run = "01a17ea1-0371-7000-8000-000000000001";
    let _ = planted(&fixture, run);
    let checkout = fixture.layout().worktree(run);
    let scratch = fixture.root.parent().unwrap().join("scratch");
    let planted_blob = sha_of(&scratch, "planted\n");
    git(
        &fixture.repo,
        &[
            "hash-object",
            "-w",
            &scratch.join("content").to_string_lossy(),
        ],
    );
    std::fs::write(checkout.join("committed.txt"), "on disk\n").unwrap();
    git(
        &checkout,
        &[
            "update-index",
            "--cacheinfo",
            &format!("100644,{planted_blob},committed.txt"),
        ],
    );
    git(
        &checkout,
        &["update-index", "--skip-worktree", "committed.txt"],
    );

    let reaped = reap_one(&fixture.layout(), run, false);
    assert_work_on_branch(&fixture, run, &reaped);
    assert_eq!(
        git(
            &fixture.repo,
            &["show", &format!("hydra/{run}:committed.txt")]
        ),
        "on disk\n"
    );
}

/// Why `hydra run` refuses a sparse repository. A sparse checkout is index
/// entries with the skip-worktree bit and no file on disk. The rebuilt index
/// has no bits, so reap commits every such file as deleted.
#[test]
fn a_skip_worktree_entry_with_no_file_is_reaped_as_a_deletion() {
    let fixture = Fixture::new("sparse-deletion");
    let run = "01a17ea1-0371-7000-8000-000000000002";
    let _ = planted(&fixture, run);
    let checkout = fixture.layout().worktree(run);
    git(
        &checkout,
        &["update-index", "--skip-worktree", "committed.txt"],
    );
    std::fs::remove_file(checkout.join("committed.txt")).unwrap();

    let reaped = reap_one(&fixture.layout(), run, false);
    assert_work_on_branch(&fixture, run, &reaped);
    let tree = git(
        &fixture.repo,
        &["ls-tree", "--name-only", &format!("hydra/{run}")],
    );
    assert!(!tree.lines().any(|name| name == "committed.txt"), "{tree}");
}

#[test]
fn a_sparse_repository_is_refused_at_launch_by_name() {
    for how in ["file", "config"] {
        let fixture = Fixture::new(&format!("sparse-{how}"));
        let run = "01a17ea1-0371-7000-8000-000000000003";
        match how {
            "file" => {
                let info = fixture.repo.join(".git/info");
                std::fs::create_dir_all(&info).unwrap();
                std::fs::write(info.join("sparse-checkout"), "/*\n").unwrap();
            }
            _ => {
                git(&fixture.repo, &["config", "core.sparseCheckout", "true"]);
            }
        }
        let layout = fixture.layout();
        let Err(err) = hydra::worktree::create(
            &fixture.repo,
            &layout.worktree(run),
            &format!("hydra/{run}"),
            &layout.seed_index(run),
        ) else {
            panic!("{how}: a sparse repository launched");
        };
        let err = format!("{err:#}");
        assert!(err.contains("sparse"), "{how}: {err}");
        assert!(
            err.contains(&*fixture.repo.to_string_lossy()),
            "{how}: {err}"
        );
        assert!(!layout.worktree(run).exists(), "{how}");
        assert!(!fixture.branches().contains(run), "{how}");
    }
}

#[test]
fn a_zero_byte_index_is_rebuilt_and_the_work_still_reaps() {
    let fixture = Fixture::new("empty-index");
    let run = "01a17ea1-0371-7000-8000-000000000004";
    let private = planted(&fixture, run);
    std::fs::write(private.join("index"), "").unwrap();

    let reaped = reap_one(&fixture.layout(), run, false);
    assert_work_on_branch(&fixture, run, &reaped);
}

#[test]
#[cfg(unix)]
fn a_fifo_as_head_is_refused_by_name_without_hanging() {
    let fixture = Fixture::new("fifo-head");
    let run = "01a17ea1-0371-7000-8000-000000000005";
    let private = planted(&fixture, run);
    let head = private.join("HEAD");
    std::fs::remove_file(&head).unwrap();
    let made = Command::new("mkfifo").arg(&head).status().unwrap();
    assert!(made.success());

    let reaped = reap_within(&fixture, run, "a fifo HEAD");
    assert!(
        matches!(&reaped[0], Reaped::Failed { why, .. } if why.contains(&*head.to_string_lossy())),
        "{:?}",
        worktree_why(&reaped)
    );
    assert!(fixture.layout().worktree(run).join("child.txt").is_file());
}

#[test]
#[cfg(unix)]
fn an_unreadable_reflog_or_logs_directory_does_not_stop_reap() {
    use std::os::unix::fs::PermissionsExt;
    for what in ["logs/HEAD", "logs"] {
        let fixture = Fixture::new(&format!("unreadable-{}", what.replace('/', "-")));
        let run = "01a17ea1-0371-7000-8000-000000000006";
        let private = planted(&fixture, run);
        let path = private.join(what);
        if !path.exists() {
            std::fs::create_dir_all(private.join("logs")).unwrap();
            std::fs::write(private.join("logs/HEAD"), "").unwrap();
        }
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o000)).unwrap();

        let reaped = reap_one(&fixture.layout(), run, false);
        if path.exists() {
            let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700));
        }
        assert_work_on_branch(&fixture, run, &reaped);
    }
}

/// Why reap failed, for an assertion message.
fn worktree_why(reaped: &[Reaped]) -> Option<&str> {
    reaped.iter().find_map(|item| match item {
        Reaped::Failed { why, .. } => Some(why.as_str()),
        _ => None,
    })
}

#[test]
fn reap_without_trusted_git_metadata_leaves_checkout_untouched() {
    let fixture = Fixture::new("missing-trusted-git");
    let layout = fixture.layout();
    let run = "01a07217-6007-7733-9ce2-7bb31830eabd";
    let checkout = fixture.checkout(run);
    let saved = checkout.with_extension("git-context.json");
    if saved.exists() {
        std::fs::remove_file(saved).unwrap();
    }
    std::fs::write(checkout.join("child.txt"), "keep this work\n").unwrap();
    let before = git(&fixture.repo, &["rev-parse", &format!("hydra/{run}")]);
    let reaped = reap_one(&layout, run, false);
    assert!(
        matches!(&reaped[0], Reaped::Failed { why, .. } if why.contains("no trusted git directory"))
    );
    assert_eq!(
        git(&fixture.repo, &["rev-parse", &format!("hydra/{run}")]),
        before
    );
    assert!(checkout.join("child.txt").is_file());
    assert!(layout.spawn(run).is_dir());
}

// --- the run's own object store, map ticket 74 ------------------------------

/// `git` in a run's checkout the way the run's own git runs there: with the
/// object directory `hydra run` hands the child.
fn run_git(checkout: &Path, env: &[(&'static str, String)], args: &[&str]) -> String {
    let out = fixture_git::command()
        .args(args)
        .current_dir(checkout)
        .envs(env.iter().map(|(name, value)| (*name, value)))
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "the run's git {} failed: {}",
        args.join(" "),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).to_string()
}

fn has_object(repo: &Path, sha: &str) -> bool {
    fixture_git::command()
        .args(["cat-file", "-e", sha])
        .current_dir(repo)
        .status()
        .unwrap()
        .success()
}

/// The sha git would give `content`, without writing it anywhere.
fn sha_of(scratch: &Path, content: &str) -> String {
    std::fs::create_dir_all(scratch).unwrap();
    std::fs::write(scratch.join("content"), content).unwrap();
    git(scratch, &["hash-object", "--no-filters", "content"])
        .trim()
        .to_string()
}

/// A subdirectory the run inited and committed into, with the two pointers
/// [`hydra::worktree::run_object_env`] sets. Its `.git` has no `objects/` of
/// its own. Answers the nested HEAD.
fn nested_repository(
    checkout: &Path,
    env: &[(&'static str, String)],
    dir: &str,
    file: &str,
    content: &str,
) -> String {
    let nested = checkout.join(dir);
    std::fs::create_dir_all(&nested).unwrap();
    run_git(&nested, env, &["init", "-q", "-b", "main"]);
    run_git(
        &nested,
        env,
        &["config", "user.email", "test@example.invalid"],
    );
    run_git(&nested, env, &["config", "user.name", "hydra tests"]);
    std::fs::write(nested.join(file), content).unwrap();
    run_git(&nested, env, &["add", file]);
    run_git(&nested, env, &["commit", "-qm", "nested"]);
    assert!(
        !nested.join(".git/objects").exists(),
        "the nested .git grew an objects/ directory"
    );
    run_git(&nested, env, &["rev-parse", "HEAD"])
        .trim()
        .to_string()
}

/// Every object the parent's own store holds. Not the unreachable ones:
/// since build ticket 371 reap drops the run's index, so an object only that
/// index named turns unreachable without anything arriving.
fn all_objects(repo: &Path) -> Vec<String> {
    git(
        repo,
        &[
            "cat-file",
            "--batch-all-objects",
            "--batch-check=%(objectname)",
        ],
    )
    .lines()
    .map(str::to_string)
    .collect()
}

/// Plants a pack in `store` whose index claims `claimed` and whose bytes are
/// `bytes`: map ticket 74's forged pack. Built as a real one-object pack, then
/// the index's one name and its fan-out table are rewritten. Git does not
/// check an index's trailing checksum when it reads an object.
fn forge_pack(scratch: &Path, bytes: &str, claimed: &str, store: &Path) {
    std::fs::create_dir_all(scratch).unwrap();
    git(scratch, &["init", "-q"]);
    std::fs::write(scratch.join("evil"), bytes).unwrap();
    let real = git(scratch, &["hash-object", "-w", "evil"])
        .trim()
        .to_string();
    let mut pack = fixture_git::command()
        .args(["pack-objects", "-q", "forged"])
        .current_dir(scratch)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    {
        use std::io::Write;
        writeln!(pack.stdin.take().unwrap(), "{real}").unwrap();
    }
    let name = String::from_utf8(pack.wait_with_output().unwrap().stdout).unwrap();
    let base = scratch.join(format!("forged-{}", name.trim()));
    let hex = |text: &str| -> Vec<u8> {
        (0..text.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&text[i..i + 2], 16).unwrap())
            .collect()
    };
    let (real, claimed_bytes) = (hex(&real), hex(claimed));
    let mut index = std::fs::read(base.with_extension("idx")).unwrap();
    let at = index
        .windows(real.len())
        .position(|window| window == real.as_slice())
        .unwrap();
    index[at..at + real.len()].copy_from_slice(&claimed_bytes);
    // A version 2 index: 8 bytes of header, then 256 cumulative counts.
    for slot in 0..256usize {
        let count: u32 = if slot < claimed_bytes[0] as usize {
            0
        } else {
            1
        };
        index[8 + slot * 4..12 + slot * 4].copy_from_slice(&count.to_be_bytes());
    }
    let packs = store.join("pack");
    std::fs::create_dir_all(&packs).unwrap();
    std::fs::copy(base.with_extension("pack"), packs.join("pack-forged.pack")).unwrap();
    std::fs::write(packs.join("pack-forged.idx"), index).unwrap();
}

#[test]
fn reap_moves_the_runs_objects_and_a_forged_pack_reaches_neither_the_parent_nor_a_later_worktree() {
    let fixture = Fixture::new("run-objects");
    let layout = fixture.layout();
    let run = "01a0d103-0000-7000-8000-000000000001";
    let checkout = fixture.checkout(run);
    let env = hydra::worktree::prepare_run_objects(&layout, run, &fixture.repo).unwrap();
    let store = layout.run_objects(run);
    let scratch = fixture.root.parent().unwrap().join("scratch");

    // The run's own work: a file it staged, and an object only its store holds.
    std::fs::write(checkout.join("staged.txt"), "the run staged this\n").unwrap();
    run_git(&checkout, &env, &["add", "staged.txt"]);
    std::fs::create_dir_all(checkout.join("ignored")).unwrap();
    std::fs::write(checkout.join("ignored/note"), "only the run's store\n").unwrap();
    let own = run_git(&checkout, &env, &["hash-object", "-w", "ignored/note"])
        .trim()
        .to_string();
    assert!(!has_object(&fixture.repo, &own), "the run wrote the parent");

    // Map ticket 74's attack: a pack naming the sha of content the human has
    // not written yet. The run's own git serves the forged bytes.
    let future = "the human writes this later\n";
    let claimed = sha_of(&scratch.join("sha"), future);
    forge_pack(&scratch.join("forge"), "EVIL\n", &claimed, &store);
    assert_eq!(
        run_git(&checkout, &env, &["cat-file", "-p", &claimed]),
        "EVIL\n"
    );

    let reaped = reap_one(&layout, run, false);
    assert_eq!(hydra::reap::failures(&reaped), 0);
    let branch = format!("hydra/{run}");
    assert_eq!(
        git(&fixture.repo, &["show", &format!("{branch}:staged.txt")]),
        "the run staged this\n"
    );
    assert_eq!(
        git(&fixture.repo, &["cat-file", "-p", &own]),
        "only the run's store\n",
        "reap did not move the run's object"
    );
    assert!(
        !has_object(&fixture.repo, &claimed),
        "the forged name reached the parent"
    );

    // The human then writes that content and commits it, and a later run is
    // seeded from the parent. Had the forged pack arrived, `git add` would
    // have found the name already stored and kept the forged bytes.
    std::fs::write(fixture.repo.join("future.txt"), future).unwrap();
    git(&fixture.repo, &["add", "future.txt"]);
    git(&fixture.repo, &["commit", "-qm", "later"]);
    assert_eq!(git(&fixture.repo, &["show", "HEAD:future.txt"]), future);
    let later = fixture.checkout("01a0d103-0000-7000-8000-000000000002");
    assert_eq!(
        std::fs::read_to_string(later.join("future.txt")).unwrap(),
        future
    );
}

#[test]
fn an_entry_the_run_staged_against_a_forged_object_is_hashed_again_from_the_checkout() {
    let fixture = Fixture::new("forged-entry");
    let layout = fixture.layout();
    let run = "01a0d103-0000-7000-8000-000000000003";
    let checkout = fixture.checkout(run);
    let env = hydra::worktree::prepare_run_objects(&layout, run, &fixture.repo).unwrap();
    let scratch = fixture.root.parent().unwrap().join("scratch");

    // Planted before the `git add`, so git finds the name stored and writes
    // nothing: the index names the forged bytes.
    let content = "what the file really says\n";
    let claimed = sha_of(&scratch.join("sha"), content);
    forge_pack(
        &scratch.join("forge"),
        "EVIL\n",
        &claimed,
        &layout.run_objects(run),
    );
    std::fs::write(checkout.join("real.txt"), content).unwrap();
    // An hour old, so the entry is not racily clean and reap's own `git add`
    // trusts its stat data instead of hashing the file again.
    std::fs::File::options()
        .write(true)
        .open(checkout.join("real.txt"))
        .unwrap()
        .set_modified(std::time::SystemTime::now() - std::time::Duration::from_secs(3600))
        .unwrap();
    run_git(&checkout, &env, &["add", "real.txt"]);
    assert_eq!(
        run_git(&checkout, &env, &["cat-file", "-p", ":real.txt"]),
        "EVIL\n"
    );

    let reaped = reap_one(&layout, run, false);
    assert_eq!(
        hydra::reap::failures(&reaped),
        0,
        "reap failed on the forged entry"
    );
    assert_eq!(
        git(&fixture.repo, &["show", &format!("hydra/{run}:real.txt")]),
        content
    );
}

// --- build ticket 106: reap imports only what the run's own store holds -----

/// A repository beside the fixture holding one loose blob, `content`, which
/// stands for any repository the user can read and the run cannot. Answers
/// the repository and the blob's sha.
fn other_repository(dir: &Path, content: &str) -> (PathBuf, String) {
    std::fs::create_dir_all(dir).unwrap();
    git(dir, &["init", "-q"]);
    std::fs::write(dir.join("secret"), content).unwrap();
    let sha = git(dir, &["hash-object", "-w", "secret"])
        .trim()
        .to_string();
    (dir.to_path_buf(), sha)
}

/// Puts in the run's store a pack index naming `sha` and no pack beside it:
/// a name for reap to list that the store itself cannot serve.
fn index_naming(repo: &Path, sha: &str, store: &Path) {
    let mut pack = fixture_git::command()
        .args(["pack-objects", "-q", "named"])
        .current_dir(repo)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    {
        use std::io::Write;
        writeln!(pack.stdin.take().unwrap(), "{sha}").unwrap();
    }
    let name = String::from_utf8(pack.wait_with_output().unwrap().stdout).unwrap();
    let packs = store.join("pack");
    std::fs::create_dir_all(&packs).unwrap();
    std::fs::copy(
        repo.join(format!("named-{}.idx", name.trim())),
        packs.join(format!("pack-named-{}.idx", &sha[..12])),
    )
    .unwrap();
}

/// The review of build ticket 103, item 1, confirmed there by experiment. The
/// run writes `info/alternates` in its own store, naming another repository's
/// objects, plus a pack index naming a blob there, and stages that blob in its
/// checkout's index under a new path. Reap's `pack-objects` read the store's
/// alternates, packed the other repository's blob into the parent, and the
/// commit carried it on the run's branch.
#[test]
fn an_alternates_file_in_the_runs_store_brings_nothing_into_the_parent() {
    let fixture = Fixture::new("store-alternates");
    let layout = fixture.layout();
    let run = "01a0d106-0000-7000-8000-000000000001";
    let checkout = fixture.checkout(run);
    let env = hydra::worktree::prepare_run_objects(&layout, run, &fixture.repo).unwrap();
    let store = layout.run_objects(run);
    let (other, secret) = other_repository(&fixture.root.join("other"), "not the run's\n");

    std::fs::write(
        store.join("info/alternates"),
        format!("{}\n", other.join(".git/objects").display()),
    )
    .unwrap();
    index_naming(&other, &secret, &store);
    // Work of its own, so reap keeps the branch.
    std::fs::write(checkout.join("work.txt"), "the run's work\n").unwrap();
    run_git(
        &checkout,
        &env,
        &[
            "update-index",
            "--add",
            "--cacheinfo",
            &format!("100644,{secret},leak.txt"),
        ],
    );

    let reaped = reap_one(&layout, run, false);
    assert_eq!(hydra::reap::failures(&reaped), 0);
    assert!(
        !has_object(&fixture.repo, &secret),
        "reap imported a blob from the repository the store's alternates named"
    );
    let files = git(
        &fixture.repo,
        &["ls-tree", "-r", "--name-only", &format!("hydra/{run}")],
    );
    assert!(files.contains("work.txt"), "{files}");
    assert!(
        !files.contains("leak.txt"),
        "the run's branch carries another repository's blob: {files}"
    );
}

/// The review of build ticket 103, item 2. The name listing skipped links, but
/// `pack-objects` looks names up by itself. A fan-out directory that is a link
/// into another repository, or a loose object that is a second hard link to
/// one of its files, led reap to that repository's blob just as an alternates
/// file did.
#[test]
fn a_linked_fan_out_directory_or_a_hard_link_in_the_runs_store_brings_nothing_in() {
    let fixture = Fixture::new("store-links");
    let layout = fixture.layout();
    // One run per case: a name git cannot serve fails the whole
    // `pack-objects`, and would hide the other case.
    let store_for = |run: &str| {
        fixture.checkout(run);
        hydra::worktree::prepare_run_objects(&layout, run, &fixture.repo).unwrap();
        layout.run_objects(run)
    };

    let linked_run = "01a0d106-0000-7000-8000-000000000002";
    let store = store_for(linked_run);
    let (linked_repo, linked) = other_repository(&fixture.root.join("linked"), "by a link\n");
    index_naming(&linked_repo, &linked, &store);
    std::os::unix::fs::symlink(
        linked_repo.join(".git/objects").join(&linked[..2]),
        store.join(&linked[..2]),
    )
    .unwrap();

    let hard_run = "01a0d106-0000-7000-8000-000000000004";
    let store = store_for(hard_run);
    let (hard_repo, hard) = other_repository(&fixture.root.join("hard"), "by a hard link\n");
    std::fs::create_dir_all(store.join(&hard[..2])).unwrap();
    std::fs::hard_link(
        hard_repo
            .join(".git/objects")
            .join(&hard[..2])
            .join(&hard[2..]),
        store.join(&hard[..2]).join(&hard[2..]),
    )
    .unwrap();

    for run in [linked_run, hard_run] {
        let reaped = reap_one(&layout, run, false);
        assert_eq!(hydra::reap::failures(&reaped), 0);
    }
    assert!(
        !has_object(&fixture.repo, &linked),
        "reap imported a blob through a linked fan-out directory"
    );
    assert!(
        !has_object(&fixture.repo, &hard),
        "reap imported a blob through a hard link"
    );
}

/// The review of build ticket 103, item 3. A store past the cap imports
/// nothing, and `index-pack` refuses a stream past it, which covers a store
/// that grows between the walk and the pack. The pack stream for one small
/// object is larger than its loose file, so a cap of exactly the store's size
/// passes the walk and stops at `index-pack`.
#[test]
fn a_store_over_the_cap_imports_nothing() {
    let fixture = Fixture::new("store-cap");
    let layout = fixture.layout();
    let run = "01a0d106-0000-7000-8000-000000000003";
    let checkout = fixture.checkout(run);
    let env = hydra::worktree::prepare_run_objects(&layout, run, &fixture.repo).unwrap();
    let store = layout.run_objects(run);
    std::fs::write(checkout.join("own.txt"), "the run's own object\n").unwrap();
    let own = run_git(&checkout, &env, &["hash-object", "-w", "own.txt"])
        .trim()
        .to_string();
    let size = std::fs::metadata(store.join(&own[..2]).join(&own[2..]))
        .unwrap()
        .len();

    for (cap, stage, said) in [
        (size - 1, "the walk", "reap imports"),
        (size, "index-pack", "git would not verify"),
    ] {
        let err = hydra::worktree::import_run_objects_within(&fixture.repo, &store, cap)
            .expect_err(&format!("a store over a cap of {cap} bytes was imported"));
        assert!(
            format!("{err:#}").contains(said),
            "{stage} did not refuse it: {err:#}"
        );
        assert!(
            !has_object(&fixture.repo, &own),
            "{stage} let a store over the cap into the parent: {err:#}"
        );
    }
    let moved = hydra::worktree::import_run_objects(&fixture.repo, &store).unwrap();
    assert_eq!(moved, 1);
    assert!(has_object(&fixture.repo, &own));
}

// --- build ticket 112: reap packs from a copy the run cannot reach ------------

/// The residual build ticket 106 named. Reap removed `info/` from the store and
/// checked again, and a process the run left running could write
/// `info/alternates` after that check and before `pack-objects` read the store.
/// Here the store names another repository's blob in a pack index, and the
/// alternates file and a linked fan-out directory arrive at the worst moment,
/// after reap has done everything but start git. The run's own object, alone
/// in a second store, still arrives by the same path.
#[test]
fn an_alternates_file_written_while_reap_imports_brings_nothing_in() {
    let fixture = Fixture::new("store-race");
    let layout = fixture.layout();
    let run = "01a0d107-0000-7000-8000-000000000001";
    let checkout = fixture.checkout(run);
    let env = hydra::worktree::prepare_run_objects(&layout, run, &fixture.repo).unwrap();
    let store = layout.run_objects(run);
    let (other, secret) = other_repository(&fixture.root.join("other"), "not the run's\n");
    let (linked_repo, linked) = other_repository(&fixture.root.join("linked"), "by a link\n");
    index_naming(&other, &secret, &store);
    index_naming(&linked_repo, &linked, &store);

    let err = hydra::worktree::import_run_objects_racing(
        &fixture.repo,
        &store,
        hydra::worktree::RUN_STORE_IMPORT_CAP,
        || {
            std::fs::create_dir_all(store.join("info")).unwrap();
            std::fs::write(
                store.join("info/alternates"),
                format!("{}\n", other.join(".git/objects").display()),
            )
            .unwrap();
            std::os::unix::fs::symlink(
                linked_repo.join(".git/objects").join(&linked[..2]),
                store.join(&linked[..2]),
            )
            .unwrap();
        },
    )
    .expect_err("a store naming blobs it does not hold imported cleanly");
    assert!(
        format!("{err:#}").contains("git would not verify"),
        "{err:#}"
    );
    assert!(
        !has_object(&fixture.repo, &secret),
        "reap imported a blob through an alternates file written during the import"
    );
    assert!(
        !has_object(&fixture.repo, &linked),
        "reap imported a blob through a fan-out link made during the import"
    );
    assert!(
        !hydra::worktree::run_store_copy(&store).exists(),
        "the copy outlived the import"
    );

    let own_run = "01a0d107-0000-7000-8000-000000000002";
    let own_checkout = fixture.checkout(own_run);
    let own_env = hydra::worktree::prepare_run_objects(&layout, own_run, &fixture.repo).unwrap();
    std::fs::write(own_checkout.join("own.txt"), "the run's own object\n").unwrap();
    let own = run_git(&own_checkout, &own_env, &["hash-object", "-w", "own.txt"])
        .trim()
        .to_string();
    let moved =
        hydra::worktree::import_run_objects(&fixture.repo, &layout.run_objects(own_run)).unwrap();
    assert_eq!(moved, 1);
    assert!(has_object(&fixture.repo, &own));
    let _ = (checkout, env);
}

// --- builds 325 and 326: refuse nested repositories before any mutation ----

/// Verify both index-only gitlinks and unstaged repositories fail before the
/// import, pointer repair, injected-file removal, or commit can change anything.
fn assert_nested_refusal(fixture: &Fixture, run: &str, paths: &[&str]) {
    let layout = fixture.layout();
    let checkout = layout.worktree(run);
    let branch = format!("hydra/{run}");
    let head = git(&fixture.repo, &["rev-parse", &branch]);
    let objects = git(&fixture.repo, &["count-objects", "-v"]);
    let objects_before = all_objects(&fixture.repo);
    let pointer = std::fs::read(checkout.join(".git")).unwrap();
    let markers: Vec<_> = paths
        .iter()
        .filter_map(|path| {
            let marker = checkout.join(path).join(".git");
            std::fs::symlink_metadata(&marker).ok().map(|meta| {
                let bytes = if meta.is_file() {
                    Some(std::fs::read(&marker).unwrap())
                } else {
                    None
                };
                (marker, meta.is_dir(), bytes)
            })
        })
        .collect();
    // This is normally removed before staging. Refusal must leave it intact.
    std::fs::create_dir_all(checkout.join(".claude/skills")).unwrap();
    std::fs::write(checkout.join(".claude/skills/.gitignore"), "*\n").unwrap();

    let reaped = reap_one(&layout, run, false);
    assert_eq!(hydra::reap::failures(&reaped), 1);
    let report = hydra::reap::report(&reaped);
    assert!(
        report.contains("refusing to reap nested repositories or gitlinks"),
        "{report}"
    );
    for path in paths {
        assert!(
            report.lines().any(|line| line == format!("  {path:?}")),
            "{report}"
        );
    }
    assert_eq!(report.lines().count(), paths.len() + 1, "{report}");
    assert_eq!(git(&fixture.repo, &["rev-parse", &branch]), head);
    assert_eq!(git(&fixture.repo, &["count-objects", "-v"]), objects);
    assert_eq!(all_objects(&fixture.repo), objects_before);
    assert_eq!(std::fs::read(checkout.join(".git")).unwrap(), pointer);
    assert_eq!(
        std::fs::read(checkout.join(".claude/skills/.gitignore")).unwrap(),
        b"*\n"
    );
    for (marker, directory, bytes) in markers {
        assert_eq!(marker.is_dir(), directory);
        if let Some(bytes) = bytes {
            assert_eq!(std::fs::read(marker).unwrap(), bytes);
        }
    }
    assert!(checkout.is_dir());
    assert!(layout.spawn(run).is_dir());
    assert!(
        hydra::status::abandoned_worktrees(
            &layout,
            &hydra::live::scan(&layout).unwrap(),
            std::time::Duration::ZERO,
        )
        .unwrap()
        .contains(&checkout)
    );
    println!(
        "{run}: refused {} path(s), branch/index/pointer unchanged; parent count-objects before/after:\n{objects}",
        paths.len()
    );
}

/// Build 325 previously pinned a git-status failure after object import.
/// Build 326's human-approved contract refuses by name before importing.
#[test]
fn reap_fails_when_the_run_staged_a_nested_repository() {
    let fixture = Fixture::new("nested-staged");
    let layout = fixture.layout();
    let run = "01a0d325-0000-7000-8000-000000000001";
    let checkout = fixture.checkout(run);
    let env = hydra::worktree::prepare_run_objects(&layout, run, &fixture.repo).unwrap();
    let nested_head = nested_repository(&checkout, &env, "sub", "note.txt", "nested\n");
    run_git(&checkout, &env, &["add", "-A"]);
    let staged = run_git(&checkout, &env, &["ls-files", "--stage", "sub"]);
    assert!(
        staged.contains("160000") && staged.contains(&nested_head),
        "{staged}"
    );
    assert_nested_refusal(&fixture, run, &["sub"]);
    assert_eq!(
        std::fs::read_to_string(checkout.join("sub/note.txt")).unwrap(),
        "nested\n"
    );
}

/// Build 325 previously committed a gitlink, imported three unreachable objects,
/// and failed at removal. Build 326 now refuses before any of those effects.
#[test]
fn reap_refuses_an_unstaged_nested_repository_before_import_or_commit() {
    let fixture = Fixture::new("nested-unstaged");
    let layout = fixture.layout();
    let run = "01a0d325-0000-7000-8000-000000000002";
    let checkout = fixture.checkout(run);
    let env = hydra::worktree::prepare_run_objects(&layout, run, &fixture.repo).unwrap();
    nested_repository(&checkout, &env, "sub", "note.txt", "nested\n");
    assert_nested_refusal(&fixture, run, &["sub"]);
    assert_eq!(
        std::fs::read_to_string(checkout.join("sub/note.txt")).unwrap(),
        "nested\n"
    );
}

#[test]
fn reap_names_both_nested_repositories_including_ignored_paths() {
    let fixture = Fixture::new("nested-two");
    let layout = fixture.layout();
    let run = "01a0d326-0000-7000-8000-000000000003";
    let checkout = fixture.checkout(run);
    let env = hydra::worktree::prepare_run_objects(&layout, run, &fixture.repo).unwrap();
    nested_repository(&checkout, &env, "sub", "note.txt", "one\n");
    nested_repository(&checkout, &env, "ignored/deep", "note.txt", "two\n");
    assert_nested_refusal(&fixture, run, &["ignored/deep", "sub"]);
}

#[test]
fn reap_refuses_a_nested_gitdir_file_without_following_it() {
    let fixture = Fixture::new("nested-pointer");
    let run = "01a0d326-0000-7000-8000-000000000004";
    let checkout = fixture.checkout(run);
    std::fs::create_dir_all(checkout.join("sub")).unwrap();
    // An invalid target still counts: the scan must not resolve a run's pointer.
    std::fs::write(checkout.join("sub/.git"), "gitdir: missing-metadata\n").unwrap();
    assert_nested_refusal(&fixture, run, &["sub"]);
}

#[test]
fn reap_refuses_a_legitimate_staged_submodule_instead_of_flattening() {
    let fixture = Fixture::new("nested-submodule");
    let run = "01a0d326-0000-7000-8000-000000000005";
    let checkout = fixture.checkout(run);
    let upstream = fixture.root.join("upstream");
    std::fs::create_dir_all(&upstream).unwrap();
    git(&upstream, &["init", "-q", "-b", "main"]);
    git(&upstream, &["config", "user.email", "test@example.invalid"]);
    git(&upstream, &["config", "user.name", "hydra tests"]);
    std::fs::write(upstream.join("note.txt"), "upstream\n").unwrap();
    git(&upstream, &["add", "-A"]);
    git(&upstream, &["commit", "-qm", "upstream"]);
    git(
        &checkout,
        &[
            "-c",
            "protocol.file.allow=always",
            "submodule",
            "add",
            upstream.to_str().unwrap(),
            "sub",
        ],
    );
    let env = hydra::worktree::prepare_run_objects(&fixture.layout(), run, &fixture.repo).unwrap();
    run_git(&checkout, &env, &["add", "-A"]);
    assert!(checkout.join("sub/.git").is_file());
    assert!(git(&checkout, &["ls-files", "--stage", "sub"]).starts_with("160000 "));
    assert_nested_refusal(&fixture, run, &["sub"]);
    assert_eq!(
        std::fs::read_to_string(checkout.join("sub/note.txt")).unwrap(),
        "upstream\n"
    );
}

/// Since build ticket 371 reap rebuilds the index from `HEAD` before reading
/// it, so a gitlink staged in the index alone is gone before the nested scan,
/// and the branch never carries it.
#[test]
fn an_index_only_gitlink_with_no_nested_git_marker_never_reaches_the_branch() {
    let fixture = Fixture::new("nested-index-only");
    let run = "01a0d326-0000-7000-8000-000000000006";
    let checkout = fixture.checkout(run);
    std::fs::write(checkout.join("child.txt"), "work\n").unwrap();
    let head = git(&fixture.repo, &["rev-parse", "HEAD"]);
    git(
        &checkout,
        &[
            "update-index",
            "--add",
            "--cacheinfo",
            &format!("160000,{},sub", head.trim()),
        ],
    );
    assert!(!checkout.join("sub").exists());
    let reaped = reap_one(&fixture.layout(), run, false);
    assert_work_on_branch(&fixture, run, &reaped);
    let tree = git(
        &fixture.repo,
        &["ls-tree", "--name-only", &format!("hydra/{run}")],
    );
    assert!(!tree.lines().any(|name| name == "sub"), "{tree}");
}
