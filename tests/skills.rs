//! The skill store, the snapshot and per-run injection, harness spec 6.4.
//!
//! **Everything here is offline.** The store is git, the snapshot is git, the
//! links are symlinks, and the collision check's first two tests read
//! directories. The only part that needs a provider is the third test, which asks
//! `claude` what it resolved, and that one is measured by hand and written up in
//! `IMPLEMENTATION.md` rather than driven from here: it launches a binary that
//! self-updates, and the two halves of it that can be pinned, the project slug and
//! the transcript shape, are unit tests in `src/provider/claude.rs`.
//!
//! **The self-ignoring root is checked against real git and not against a doc
//! comment.** 6.4's claim is that `git status` reports nothing and `git add -A`
//! stages nothing, verified by ticket 24 on all three builds. That is the kind of
//! claim that quietly stops being true, so it is asserted here.

use std::path::{Path, PathBuf};
use std::process::Command;

use hydra::paths::Layout;
use hydra::provider::Provider;
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

/// A `~/hydra` the test owns, with a repo beside it to make checkouts from.
struct Fixture {
    root: PathBuf,
    repo: PathBuf,
    /// Stands in for the home directory the user-scope index scans.
    home: PathBuf,
}

impl Fixture {
    fn new(name: &str) -> Self {
        let base = std::env::temp_dir().join(format!("hydra-skills-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let repo = base.join("repo");
        std::fs::create_dir_all(&repo).unwrap();
        git(&repo, &["init", "-q", "-b", "main"]);
        git(&repo, &["config", "user.email", "test@example.invalid"]);
        git(&repo, &["config", "user.name", "hydra tests"]);
        std::fs::write(repo.join("committed.txt"), "one\n").unwrap();
        git(&repo, &["add", "-A"]);
        git(&repo, &["commit", "-qm", "first"]);

        let fixture = Self {
            root: base.join("hydra"),
            repo,
            home: base.join("home"),
        };
        fixture.layout().ensure().unwrap();
        std::fs::create_dir_all(&fixture.home).unwrap();
        fixture
    }

    fn layout(&self) -> Layout {
        Layout::at(&self.root)
    }

    /// A checkout the way `hydra run` makes one.
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

    /// One skill in the store's working tree, uncommitted.
    fn store_skill(&self, name: &str, description: &str, sidecar: Option<&str>) {
        let dir = self.layout().skills().join(name);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("SKILL.md"),
            format!("---\nname: {name}\ndescription: {description}\n---\n\nbody\n"),
        )
        .unwrap();
        if let Some(sidecar) = sidecar {
            std::fs::write(dir.join("hydra.toml"), sidecar).unwrap();
        }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        // A `codex` copy is 0555 directories, which `remove_dir_all` cannot
        // empty, so the owner's write bit comes back first.
        let _ = Command::new("chmod")
            .args(["-R", "u+w"])
            .arg(self.root.parent().unwrap())
            .output();
        let _ = std::fs::remove_dir_all(self.root.parent().unwrap());
    }
}

/// A run file carrying the liveness pair and the sha it pinned.
fn write_run(layout: &Layout, run_id: &str, pid: Option<u32>, store_sha: Option<&str>) {
    let start = pid.and_then(|pid| hydra::procinfo::start_time(pid).ok().flatten());
    let mut line = json!({
        "run_id": run_id,
        "parent_run_id": null,
        "seq": 0,
        "ts": "2026-09-05T20:51:39.370Z",
        "provider": "claude",
        "model": null,
        "secret_flag": null,
        "hydra": {
            "type": "run_start",
            "schema_version": 1,
            "hydra_version": "0.2.0",
            "command": "run",
            "launch_mode": "recorded",
            "provider_version": "stub",
            "task": "probe",
            "policy": "standard",
            "policy_sha": null,
            "cwd": "/tmp",
            "worktree": null,
            "depth": 0,
            "pid": pid,
            "process_start_time": start
                .map(|t| Value::from(t.to_rfc3339_opts(chrono::SecondsFormat::Micros, true)))
                .unwrap_or(Value::Null),
            "skills": store_sha.map(|sha| json!({
                "store_sha": sha, "linked": [], "resolved": null
            })).unwrap_or(Value::Null),
            "subscription": "none",
        },
    })
    .to_string();
    line.push('\n');
    std::fs::write(layout.run_file(run_id), line).unwrap();
}

const NO_GRACE: std::time::Duration = std::time::Duration::ZERO;
const RUN_A: &str = "01a07357-58a5-79e2-a25a-3df578a750b4";
const RUN_B: &str = "01a07357-58a5-79e2-a25a-3df578a750b5";

// --- the store -----------------------------------------------------------

#[test]
fn the_store_opens_as_a_git_repo_with_a_sha_to_pin() {
    // A repo with no commits has no sha, so the first run would pin nothing and
    // `skills.store_sha` would be null on a store that exists.
    let fixture = Fixture::new("opens");
    let layout = fixture.layout();
    let store = Store::open(&layout).unwrap();
    assert!(store.dir().join(".git").exists());

    let sha = store.pin(RUN_A).unwrap();
    assert_eq!(sha.len(), 40, "{sha}");
}

#[test]
fn a_dirty_store_is_committed_before_pinning_and_the_trailer_names_the_run() {
    // 6.4: launching from HEAD and ignoring the working tree would mean editing a
    // skill, running, and watching nothing happen.
    let fixture = Fixture::new("dirty");
    let layout = fixture.layout();
    let store = Store::open(&layout).unwrap();
    let before = store.pin(RUN_A).unwrap();

    fixture.store_skill("alpha", "A probe. Marker ALPHA.", None);
    let after = store.pin(RUN_B).unwrap();

    assert_ne!(before, after, "the edit has to move the sha");
    assert!(
        git(store.dir(), &["status", "--porcelain"])
            .trim()
            .is_empty(),
        "the store is clean once it is pinned"
    );
    let message = git(store.dir(), &["log", "-1", "--format=%B"]);
    assert!(
        message.contains(&format!("Hydra-Pinned-By: {RUN_B}")),
        "{message}"
    );
    // The sha the run reports is the commit that holds the edit.
    let listed = git(store.dir(), &["show", "--name-only", "--format=", &after]);
    assert!(listed.contains("alpha/SKILL.md"), "{listed}");
}

#[test]
fn pinning_an_unchanged_store_twice_answers_the_same_sha() {
    let fixture = Fixture::new("stable");
    let store = Store::open(&fixture.layout()).unwrap();
    fixture.store_skill("alpha", "A probe.", None);
    assert_eq!(store.pin(RUN_A).unwrap(), store.pin(RUN_B).unwrap());
}

// --- the snapshot --------------------------------------------------------

#[test]
fn an_edit_after_the_pin_does_not_reach_the_run_that_pinned_it() {
    // The whole reason the links point at a snapshot. A symlink into the live
    // store resolves to the working tree, so an edit mid-session would reach a
    // session already running.
    let fixture = Fixture::new("frozen");
    let layout = fixture.layout();
    let store = Store::open(&layout).unwrap();
    fixture.store_skill("alpha", "A probe. Marker FIRST.", None);
    let sha = store.pin(RUN_A).unwrap();
    let snapshot = store.snapshot(&layout, &sha).unwrap();

    let worktree = fixture.checkout(RUN_A);
    let injected = skills::inject(&worktree, Provider::Claude, &snapshot, &sha).unwrap();
    assert_eq!(injected.linked, vec!["alpha".to_string()]);

    fixture.store_skill("alpha", "A probe. Marker SECOND.", None);
    let seen = std::fs::read_to_string(injected.root.join("alpha/SKILL.md")).unwrap();
    assert!(seen.contains("Marker FIRST"), "{seen}");
    assert!(!seen.contains("Marker SECOND"), "{seen}");
}

#[test]
fn two_runs_on_one_sha_share_one_snapshot() {
    // 6.4: snapshots are shared per sha and refcounted, because delegation caps
    // depth and not fan-out.
    let fixture = Fixture::new("shared");
    let layout = fixture.layout();
    let store = Store::open(&layout).unwrap();
    fixture.store_skill("alpha", "A probe.", None);
    let sha = store.pin(RUN_A).unwrap();

    let first = store.snapshot(&layout, &sha).unwrap();
    let second = store.snapshot(&layout, &sha).unwrap();
    assert_eq!(first, second);
    assert_eq!(
        git(store.dir(), &["worktree", "list"]).lines().count(),
        2,
        "the store itself plus one snapshot"
    );
}

#[test]
fn launches_racing_on_a_fresh_sha_all_end_with_the_one_snapshot() {
    // Build ticket 47: three launches in one second on a sha with no snapshot
    // yet, one made it and two died on the store's bookkeeping. Four threads,
    // one sha, no snapshot to start with. Every one has to come back with a
    // usable directory, and it has to be the same directory.
    let fixture = Fixture::new("race");
    let layout = fixture.layout();
    let store = Store::open(&layout).unwrap();
    fixture.store_skill("alpha", "A probe.", None);
    let sha = store.pin(RUN_A).unwrap();
    assert!(!layout.snapshot(&sha).exists());

    let barrier = std::sync::Barrier::new(4);
    let results: Vec<Result<PathBuf, String>> = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..4)
            .map(|_| {
                scope.spawn(|| {
                    barrier.wait();
                    store
                        .snapshot(&layout, &sha)
                        .map_err(|err| format!("{err:#}"))
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .collect()
    });

    let expected = layout.snapshot(&sha);
    for result in &results {
        assert_eq!(result.as_ref().unwrap(), &expected, "{results:?}");
    }

    // One directory, at the sha, and the store lists exactly it beside itself.
    // No staging checkout survives a loser. The one file beside it is the lock,
    // which carries nothing.
    let entries: Vec<String> = std::fs::read_dir(layout.snapshots())
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.is_dir())
        .map(|path| path.file_name().unwrap().to_string_lossy().to_string())
        .collect();
    assert_eq!(entries, vec![sha.clone()]);
    assert_eq!(
        git(&expected, &["rev-parse", "--verify", "HEAD"]).trim(),
        sha
    );
    assert!(expected.join("alpha/SKILL.md").is_file());

    let listed: Vec<String> = git(store.dir(), &["worktree", "list", "--porcelain"])
        .lines()
        .filter_map(|line| line.strip_prefix("worktree "))
        .map(|path| {
            std::fs::canonicalize(path)
                .unwrap()
                .to_string_lossy()
                .to_string()
        })
        .collect();
    assert_eq!(listed.len(), 2, "{listed:?}");
    assert!(
        listed.contains(
            &std::fs::canonicalize(&expected)
                .unwrap()
                .to_string_lossy()
                .to_string()
        ),
        "{listed:?}"
    );

    // And the sequential case on top of the raced one still reuses it.
    assert_eq!(store.snapshot(&layout, &sha).unwrap(), expected);
}

#[test]
fn a_snapshot_the_store_lost_track_of_is_repaired_and_one_at_the_wrong_sha_is_refused() {
    // The two states an existing directory can be in besides sound. Renamed
    // behind git's back, the store's listing points at a path that is gone and
    // `worktree repair` puts it right. Moved to another sha, it is not the
    // immutable thing a run was promised, and nothing here deletes it.
    let fixture = Fixture::new("verify");
    let layout = fixture.layout();
    let store = Store::open(&layout).unwrap();
    fixture.store_skill("alpha", "A probe.", None);
    let first = store.pin(RUN_A).unwrap();
    let snapshot = store.snapshot(&layout, &first).unwrap();

    let aside = layout.snapshots().join("aside");
    std::fs::rename(&snapshot, &aside).unwrap();
    std::fs::rename(&aside, &snapshot).unwrap();
    // Same path again, but git remembers the rename only through the file it
    // keeps, which a plain rename never updates. Point it somewhere wrong.
    let gitdir = std::fs::read_to_string(snapshot.join(".git")).unwrap();
    let admin = PathBuf::from(gitdir.trim().trim_start_matches("gitdir: "));
    std::fs::write(admin.join("gitdir"), format!("{}/.git\n", aside.display())).unwrap();
    assert!(!git(store.dir(), &["worktree", "list"]).contains(&first));

    assert_eq!(store.snapshot(&layout, &first).unwrap(), snapshot);
    assert!(
        git(store.dir(), &["worktree", "list"]).contains(&first),
        "repair puts the listing right"
    );

    fixture.store_skill("beta", "Another probe.", None);
    let second = store.pin(RUN_B).unwrap();
    git(&snapshot, &["checkout", "-q", "--detach", &second]);
    let err = store.snapshot(&layout, &first).unwrap_err();
    assert!(format!("{err:#}").contains("removed by hand"), "{err:#}");
    assert!(
        snapshot.join("beta/SKILL.md").is_file(),
        "nothing deleted it"
    );
}

// --- injection -----------------------------------------------------------

#[test]
fn the_injected_root_ignores_itself() {
    // 6.4's claim, against real git rather than against a doc comment.
    let fixture = Fixture::new("ignores");
    let layout = fixture.layout();
    let store = Store::open(&layout).unwrap();
    fixture.store_skill("alpha", "A probe.", None);
    fixture.store_skill("beta", "Another probe.", None);
    let sha = store.pin(RUN_A).unwrap();
    let snapshot = store.snapshot(&layout, &sha).unwrap();

    let worktree = fixture.checkout(RUN_A);
    let injected = skills::inject(&worktree, Provider::Claude, &snapshot, &sha).unwrap();
    assert_eq!(
        injected.linked,
        vec!["alpha".to_string(), "beta".to_string()]
    );
    assert_eq!(
        std::fs::read_to_string(injected.root.join(".gitignore")).unwrap(),
        "*\n"
    );

    assert_eq!(git(&worktree, &["status", "--porcelain"]).trim(), "");
    git(&worktree, &["add", "-A"]);
    assert_eq!(
        git(&worktree, &["diff", "--cached", "--name-only"]).trim(),
        "",
        "git add -A must stage nothing"
    );
}

#[test]
fn agys_own_root_ignores_itself_the_same_way() {
    // 6.4's third row. This provider's root is `.agents/skills/`, and the
    // self-ignoring trick has to hold there too: `git status` clean and
    // `git add -A` staging nothing, which is what keeps `hydra reap` from
    // committing Hydra's own links into the human's branch.
    let fixture = Fixture::new("agy-ignores");
    let layout = fixture.layout();
    let store = Store::open(&layout).unwrap();
    fixture.store_skill("alpha", "A probe.", None);
    let sha = store.pin(RUN_A).unwrap();
    let snapshot = store.snapshot(&layout, &sha).unwrap();

    let worktree = fixture.checkout(RUN_A);
    let injected = skills::inject(&worktree, Provider::Agy, &snapshot, &sha).unwrap();
    assert_eq!(injected.root, worktree.join(".agents/skills"));
    assert_eq!(injected.linked, vec!["alpha".to_string()]);
    assert_eq!(
        std::fs::read_to_string(injected.root.join(".gitignore")).unwrap(),
        "*\n"
    );

    assert_eq!(git(&worktree, &["status", "--porcelain"]).trim(), "");
    git(&worktree, &["add", "-A"]);
    assert_eq!(
        git(&worktree, &["diff", "--cached", "--name-only"]).trim(),
        "",
        "git add -A must stage nothing"
    );
}

#[test]
fn a_sidecar_that_admits_agy_alone_is_linked_for_agy_and_nobody_else() {
    let fixture = Fixture::new("agy-sidecar");
    let layout = fixture.layout();
    let store = Store::open(&layout).unwrap();
    fixture.store_skill("shared", "For everyone.", None);
    fixture.store_skill(
        "google-only",
        "For one binary.",
        Some("providers = [\"agy\"]\n"),
    );
    let sha = store.pin(RUN_A).unwrap();
    let snapshot = store.snapshot(&layout, &sha).unwrap();

    let for_agy = skills::inject(&fixture.checkout(RUN_A), Provider::Agy, &snapshot, &sha).unwrap();
    assert_eq!(
        for_agy.linked,
        vec!["google-only".to_string(), "shared".to_string()]
    );

    let for_claude =
        skills::inject(&fixture.checkout(RUN_B), Provider::Claude, &snapshot, &sha).unwrap();
    assert_eq!(for_claude.linked, vec!["shared".to_string()]);
}

#[test]
fn an_empty_store_links_nothing_and_still_pins_a_sha() {
    // The state of a fresh machine, and of this one once ticket 07's probes were
    // cleared out. Contract 5.3's three fields still have answers: an empty
    // `linked` is a measurement, not a missing one.
    let fixture = Fixture::new("empty-store");
    let layout = fixture.layout();
    let store = Store::open(&layout).unwrap();
    let sha = store.pin(RUN_A).unwrap();
    let snapshot = store.snapshot(&layout, &sha).unwrap();
    let worktree = fixture.checkout(RUN_A);

    let injected = skills::inject(&worktree, Provider::Claude, &snapshot, &sha).unwrap();
    assert!(injected.linked.is_empty());
    assert!(injected.blocked.is_empty());
    assert_eq!(skills::record(&injected, None)["store_sha"], sha);
    // The root and its `.gitignore` are made anyway, so the checkout stays clean
    // whether or not there was anything to put in it.
    assert_eq!(git(&worktree, &["status", "--porcelain"]).trim(), "");
}

#[test]
fn on_codex_the_skill_is_a_read_only_copy_with_the_snapshots_bytes() {
    let fixture = Fixture::new("codex-copy");
    let layout = fixture.layout();
    let store = Store::open(&layout).unwrap();
    fixture.store_skill("alpha", "A probe. Marker COPY.", None);
    let sha = store.pin(RUN_A).unwrap();
    let snapshot = store.snapshot(&layout, &sha).unwrap();
    let worktree = fixture.checkout(RUN_A);

    let injected = skills::inject(&worktree, Provider::Codex, &snapshot, &sha).unwrap();
    assert_eq!(injected.linked, vec!["alpha".to_string()]);

    let copy = injected.root.join("alpha");
    assert!(copy.is_dir());
    assert!(!copy.is_symlink());
    let seen = std::fs::read_to_string(copy.join("SKILL.md")).unwrap();
    let snap = std::fs::read_to_string(snapshot.join("alpha/SKILL.md")).unwrap();
    assert_eq!(seen, snap);
    assert!(seen.contains("Marker COPY"), "{seen}");

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(copy.join("SKILL.md"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o444, "copied files are read-only");
    }
}

#[test]
fn the_copy_check_reports_a_changed_a_missing_and_an_added_file_and_nothing_on_a_fresh_copy() {
    let fixture = Fixture::new("copy-check");
    let layout = fixture.layout();
    let store = Store::open(&layout).unwrap();
    fixture.store_skill("alpha", "A probe.", Some("origin = \"test\"\n"));
    let extra = layout.skills().join("alpha/refs");
    std::fs::create_dir_all(&extra).unwrap();
    std::fs::write(extra.join("one.md"), "one\n").unwrap();
    std::fs::write(extra.join("two.md"), "two\n").unwrap();
    std::os::unix::fs::symlink("one.md", extra.join("link.md")).unwrap();
    let sha = store.pin(RUN_A).unwrap();
    let snapshot = store.snapshot(&layout, &sha).unwrap();
    let worktree = fixture.checkout(RUN_A);
    let injected = skills::inject(&worktree, Provider::Codex, &snapshot, &sha).unwrap();
    let copy = injected.root.join("alpha");

    // The sidecar stays behind and a link stays a link, and neither is a
    // mismatch on a copy nobody touched.
    assert!(!copy.join("hydra.toml").exists());
    assert_eq!(
        std::fs::read_link(copy.join("refs/link.md")).unwrap(),
        Path::new("one.md")
    );
    assert_eq!(
        skills::copy_mismatch(&injected.root, &snapshot, &injected.linked),
        None
    );

    Command::new("chmod")
        .args(["-R", "u+w"])
        .arg(&injected.root)
        .status()
        .unwrap();
    std::fs::write(copy.join("SKILL.md"), "rewritten\n").unwrap();
    std::fs::remove_file(copy.join("refs/two.md")).unwrap();
    std::fs::write(copy.join("refs/three.md"), "three\n").unwrap();
    std::fs::write(copy.join("hydra.toml"), "pinned = true\n").unwrap();
    std::fs::create_dir_all(injected.root.join("planted")).unwrap();

    let note = skills::copy_mismatch(&injected.root, &snapshot, &injected.linked).unwrap();
    assert!(note.starts_with(skills::COPY_MISMATCH_NOTE), "{note}");
    for expected in [
        "alpha/SKILL.md differs",
        "alpha/refs/two.md is missing from the copy",
        "alpha/refs/three.md was added to the copy",
        "alpha/hydra.toml was added to the copy",
        "planted was added to the copy",
    ] {
        assert!(note.contains(expected), "{expected:?} not in {note}");
    }

    // A skill directory swapped for a link back into the snapshot would read
    // as identical through the link, so it is named on its own.
    std::fs::remove_dir_all(&copy).unwrap();
    std::os::unix::fs::symlink(snapshot.join("alpha"), &copy).unwrap();
    let note = skills::copy_mismatch(&injected.root, &snapshot, &injected.linked).unwrap();
    assert!(
        note.contains("alpha is no longer a directory in the copy"),
        "{note}"
    );
}

#[test]
fn on_the_other_three_providers_the_skills_are_still_links() {
    let fixture = Fixture::new("still-links");
    let layout = fixture.layout();
    let store = Store::open(&layout).unwrap();
    fixture.store_skill("alpha", "A probe.", None);
    let sha = store.pin(RUN_A).unwrap();
    let snapshot = store.snapshot(&layout, &sha).unwrap();

    for (run_id, provider) in [
        (RUN_A, Provider::Claude),
        (RUN_B, Provider::CursorAgent),
        ("01a07357-58a5-79e2-a25a-3df578a750b6", Provider::Agy),
    ] {
        let worktree = fixture.checkout(run_id);
        let injected = skills::inject(&worktree, provider, &snapshot, &sha).unwrap();
        let link = injected.root.join("alpha");
        assert!(link.is_symlink(), "{provider} should still link skills");
        assert_eq!(std::fs::read_link(&link).unwrap(), snapshot.join("alpha"));
    }
}

#[test]
fn the_links_are_links_and_they_point_into_the_snapshot() {
    let fixture = Fixture::new("links");
    let layout = fixture.layout();
    let store = Store::open(&layout).unwrap();
    fixture.store_skill("alpha", "A probe.", None);
    let sha = store.pin(RUN_A).unwrap();
    let snapshot = store.snapshot(&layout, &sha).unwrap();
    let worktree = fixture.checkout(RUN_A);

    let injected = skills::inject(&worktree, Provider::Claude, &snapshot, &sha).unwrap();
    let link = injected.root.join("alpha");
    assert!(link.is_symlink());
    assert_eq!(std::fs::read_link(&link).unwrap(), snapshot.join("alpha"));
}

#[test]
fn a_sidecar_that_names_another_provider_is_not_linked() {
    let fixture = Fixture::new("providers");
    let layout = fixture.layout();
    let store = Store::open(&layout).unwrap();
    fixture.store_skill(
        "only-cursor",
        "A probe.",
        Some("providers = [\"cursor-agent\"]\n"),
    );
    fixture.store_skill(
        "both",
        "A probe.",
        Some("providers = [\"claude\", \"agy\"]\n"),
    );
    fixture.store_skill("no-sidecar", "A probe.", None);
    let sha = store.pin(RUN_A).unwrap();
    let snapshot = store.snapshot(&layout, &sha).unwrap();
    let worktree = fixture.checkout(RUN_A);

    let injected = skills::inject(&worktree, Provider::Claude, &snapshot, &sha).unwrap();
    // No sidecar admits every provider: the field is how a skill opts out, not
    // how it opts in.
    assert_eq!(
        injected.linked,
        vec!["both".to_string(), "no-sidecar".to_string()]
    );
}

#[test]
fn a_skill_the_validator_refuses_is_left_out_rather_than_failing_the_launch() {
    // The store is a directory a human and a curator both write to. One bad file
    // in it must not be able to stop every launch on the machine.
    let fixture = Fixture::new("bad-skill");
    let layout = fixture.layout();
    let store = Store::open(&layout).unwrap();
    fixture.store_skill("good", "A probe.", None);
    fixture.store_skill("bad", "A probe. label: value here.", None);
    let sha = store.pin(RUN_A).unwrap();
    let snapshot = store.snapshot(&layout, &sha).unwrap();
    let worktree = fixture.checkout(RUN_A);

    let injected = skills::inject(&worktree, Provider::Claude, &snapshot, &sha).unwrap();
    assert_eq!(injected.linked, vec!["good".to_string()]);
}

#[test]
fn a_name_the_repo_already_carries_is_blocked_rather_than_overwritten() {
    // The repo's own `.claude/skills/` belongs to the human. Linking over it
    // would edit their checkout, so the store's copy is held back and the
    // collision check is what reports it.
    let fixture = Fixture::new("repo-own");
    let layout = fixture.layout();
    let own = fixture.repo.join(".claude/skills/repo-own");
    std::fs::create_dir_all(&own).unwrap();
    std::fs::write(
        own.join("SKILL.md"),
        "---\nname: repo-own\ndescription: The repo's own.\n---\n",
    )
    .unwrap();
    git(&fixture.repo, &["add", "-A"]);
    git(&fixture.repo, &["commit", "-qm", "a project skill"]);

    let store = Store::open(&layout).unwrap();
    fixture.store_skill("repo-own", "The store's copy.", None);
    let sha = store.pin(RUN_A).unwrap();
    let snapshot = store.snapshot(&layout, &sha).unwrap();
    let worktree = fixture.checkout(RUN_A);

    let injected = skills::inject(&worktree, Provider::Claude, &snapshot, &sha).unwrap();
    assert!(injected.linked.is_empty());
    assert_eq!(injected.blocked, vec!["repo-own".to_string()]);
    let kept = std::fs::read_to_string(injected.root.join("repo-own/SKILL.md")).unwrap();
    assert!(kept.contains("The repo's own"), "{kept}");

    let user = skills::index::Index::user_scope(&fixture.home).unwrap();
    let project = skills::index::Index::project_scope(&injected.root).unwrap();
    let hits = skills::collision::check(
        &injected,
        Provider::Claude,
        &fixture.home,
        &user,
        &project,
        None,
    );
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].name, "repo-own");
}

#[test]
fn the_record_carries_the_sha_the_names_and_the_listing() {
    // Contract 5.3: a sha without names does not say what was selected, and names
    // without a sha do not say what bytes they resolved to.
    let fixture = Fixture::new("record");
    let layout = fixture.layout();
    let store = Store::open(&layout).unwrap();
    fixture.store_skill("alpha", "A probe.", None);
    let sha = store.pin(RUN_A).unwrap();
    let snapshot = store.snapshot(&layout, &sha).unwrap();
    let worktree = fixture.checkout(RUN_A);
    let injected = skills::inject(&worktree, Provider::Claude, &snapshot, &sha).unwrap();

    let recorded = skills::record(
        &injected,
        Some(&["alpha".to_string(), "unslop".to_string()]),
    );
    assert_eq!(recorded["store_sha"], sha);
    assert_eq!(recorded["linked"], json!(["alpha"]));
    assert_eq!(recorded["resolved"], json!(["alpha", "unslop"]));

    // A listing nobody could be asked for is null and never an empty array, which
    // would claim the model saw no skills at all.
    assert!(skills::record(&injected, None)["resolved"].is_null());
}

#[test]
fn the_write_check_refuses_a_name_the_index_already_holds_and_a_broken_description() {
    // 6.4's first of the two collision checks, plus the write-time validator.
    // Nothing calls it yet, for the reason its doc comment gives, so this is what
    // says the rule is right before the caller arrives.
    let fixture = Fixture::new("admit");
    let layout = fixture.layout();
    let store = layout.skills();

    let user_skill = fixture.home.join(".claude/skills/taken");
    std::fs::create_dir_all(&user_skill).unwrap();
    std::fs::write(
        user_skill.join("SKILL.md"),
        "---\nname: taken\ndescription: The human's own.\n---\n",
    )
    .unwrap();
    let user = skills::index::Index::user_scope(&fixture.home).unwrap();

    fixture.store_skill("taken", "The store's copy.", None);
    fixture.store_skill("fine", "A description with no colon in it.", None);
    fixture.store_skill("broken", "A description with a label: value in it.", None);

    let err = skills::admit(&store, "taken", &user, &fixture.home).unwrap_err();
    assert!(
        format!("{err:#}").contains("already holds that name"),
        "{err:#}"
    );

    let err = skills::admit(&store, "broken", &user, &fixture.home).unwrap_err();
    assert!(
        format!("{err:#}").contains("colon followed by a space"),
        "{err:#}"
    );

    skills::admit(&store, "fine", &user, &fixture.home).unwrap();
}

#[test]
fn a_judge_run_against_a_nonempty_store_links_nothing() {
    let fixture = Fixture::new("judge-no-inject");
    let layout = fixture.layout();
    let store = Store::open(&layout).unwrap();
    fixture.store_skill("alpha", "A probe.", None);
    let _sha = store.pin(RUN_A).unwrap();

    let worktree = fixture.checkout(RUN_A);

    let line = json!({
        "run_id": RUN_A,
        "parent_run_id": null,
        "seq": 0,
        "ts": "2026-09-05T20:51:39.370Z",
        "provider": "claude",
        "model": "claude-3-5-sonnet-20241022",
        "secret_flag": null,
        "hydra": {
            "type": "run_start",
            "schema_version": 1,
            "hydra_version": "0.2.0",
            "command": "run",
            "launch_mode": "recorded",
            "provider_version": "2.1.263",
            "provider_session_id": null,
            "task": "probe",
            "policy": "judge",
            "policy_sha": "abc123",
            "cwd": worktree.display().to_string(),
            "worktree": {
                "path": worktree.display().to_string(),
                "branch": "hydra/01a07357-58a5-79e2-a25a-3df578a750b4",
                "base_tree": null
            },
            "depth": 0,
            "pid": null,
            "process_start_time": null,
            "floor_lifted": false,
            "mcp": {
                "doc_sha": "def456",
                "injected": [],
                "resolved": []
            },
            "env_allowlist": ["HOME", "USER", "PATH", "TERM", "SHELL", "TMPDIR"],
            "auth": {
                "mechanism": "subscription-oauth",
                "source": "keychain",
                "allow_billed": false,
                "gated": true
            },
            "subscription": "claude-pro",
            "scopes": {
                "worktree": worktree.display().to_string(),
                "skill_snapshot": "/tmp/fake",
                "skill_store": "/tmp/fake",
                "corpus": "/tmp/fake"
            }
        },
    });

    let mut line_str = line.to_string();
    line_str.push('\n');
    let run_file = layout.run_file(RUN_A);
    std::fs::write(&run_file, line_str).unwrap();

    let report = hydra::record::validate::check_file(&run_file).unwrap();
    assert!(
        report.ok(),
        "judge record should validate: {}",
        report.problems.join(", ")
    );

    let text = std::fs::read_to_string(&run_file).unwrap();
    let parsed: Value = serde_json::from_str(&text).unwrap();
    let run_obj = &parsed["hydra"];
    assert!(
        !run_obj.get("skills").is_some_and(|v| v.is_object()),
        "skills should be absent or null on judge, got {:?}",
        run_obj.get("skills")
    );
}

#[test]
fn a_standard_run_against_a_nonempty_store_still_links_skills() {
    let fixture = Fixture::new("standard-links");
    let layout = fixture.layout();
    let store = Store::open(&layout).unwrap();
    fixture.store_skill("alpha", "A probe.", None);
    let sha = store.pin(RUN_A).unwrap();
    let snapshot = store.snapshot(&layout, &sha).unwrap();

    let worktree = fixture.checkout(RUN_A);
    let injected = skills::inject(&worktree, Provider::Claude, &snapshot, &sha).unwrap();

    assert_eq!(injected.linked, vec!["alpha".to_string()]);
    assert_eq!(injected.store_sha, sha);
    assert!(injected.root.join("alpha/SKILL.md").exists());
}

// --- reaping the snapshot ------------------------------------------------

#[test]
fn a_snapshot_no_live_run_references_is_swept_and_one_a_live_run_holds_is_not() {
    // Harness spec 5's second row. The refcount is the corpus: a snapshot is
    // referenced while a live run's record names its sha.
    let fixture = Fixture::new("sweep");
    let layout = fixture.layout();
    let store = Store::open(&layout).unwrap();

    fixture.store_skill("alpha", "A probe.", None);
    let held = store.pin(RUN_A).unwrap();
    store.snapshot(&layout, &held).unwrap();
    fixture.store_skill("beta", "Another probe.", None);
    let loose = store.pin(RUN_B).unwrap();
    store.snapshot(&layout, &loose).unwrap();

    // One live run, pinned to the first sha. The second is referenced by nobody.
    write_run(&layout, RUN_A, Some(std::process::id()), Some(&held));
    write_run(&layout, RUN_B, None, Some(&loose));

    let swept = hydra::reap::snapshots(&layout, NO_GRACE).unwrap();
    let removed: Vec<String> = swept
        .iter()
        .map(|item| match item {
            hydra::reap::SnapshotSweep::Removed { sha } => sha.clone(),
            other => panic!(
                "{}",
                hydra::reap::snapshot_report(std::slice::from_ref(other))
            ),
        })
        .collect();
    assert_eq!(removed, vec![loose.clone()]);
    assert!(layout.snapshot(&held).exists());
    assert!(!layout.snapshot(&loose).exists());

    // Removed through git, so the store can check the same sha out again rather
    // than refusing a path it still thinks it owns.
    assert!(!git(store.dir(), &["worktree", "list"]).contains(&loose));
    store.snapshot(&layout, &loose).unwrap();
}

#[test]
fn a_launch_in_progress_stands_the_whole_sweep_down() {
    // A launch reuses an existing snapshot without touching it, so age cannot see
    // that one and removing it would leave a running child with dangling links.
    let fixture = Fixture::new("launching");
    let layout = fixture.layout();
    let store = Store::open(&layout).unwrap();
    fixture.store_skill("alpha", "A probe.", None);
    let sha = store.pin(RUN_A).unwrap();
    store.snapshot(&layout, &sha).unwrap();

    // A checkout with no record at all: `hydra run` makes one before the detached
    // half writes line 0.
    fixture.checkout(RUN_B);

    let swept = hydra::reap::snapshots(&layout, std::time::Duration::from_secs(600)).unwrap();
    assert!(
        matches!(
            swept.as_slice(),
            [hydra::reap::SnapshotSweep::Deferred { .. }]
        ),
        "{}",
        hydra::reap::snapshot_report(&swept)
    );
    assert!(layout.snapshot(&sha).exists());
}

#[test]
fn a_snapshot_younger_than_the_grace_is_left_alone() {
    let fixture = Fixture::new("young");
    let layout = fixture.layout();
    let store = Store::open(&layout).unwrap();
    fixture.store_skill("alpha", "A probe.", None);
    let sha = store.pin(RUN_A).unwrap();
    store.snapshot(&layout, &sha).unwrap();

    let swept = hydra::reap::snapshots(&layout, std::time::Duration::from_secs(600)).unwrap();
    assert!(swept.is_empty(), "{}", hydra::reap::snapshot_report(&swept));
    assert!(layout.snapshot(&sha).exists());
}

#[test]
fn a_sweep_with_no_snapshots_directory_does_nothing() {
    let fixture = Fixture::new("empty");
    let swept = hydra::reap::snapshots(&fixture.layout(), NO_GRACE).unwrap();
    assert!(swept.is_empty());
}
