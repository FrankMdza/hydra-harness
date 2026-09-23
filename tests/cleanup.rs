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

use std::path::{Path, PathBuf};
use std::process::Command;

use hydra::paths::Layout;
use hydra::provider::Provider;
use hydra::prune::Pruned;
use hydra::reap::Reaped;
use hydra::skills::{self, Store};
use serde_json::{Value, json};

// --- the fixture ---------------------------------------------------------

fn git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
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
