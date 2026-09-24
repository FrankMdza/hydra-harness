//! The worktree carries the parent's work in progress, harness spec 6.3.
//!
//! A clean checkout would hide exactly the work most delegations are about, so the
//! seeding is the part worth testing rather than the `git worktree add`.

use std::path::{Path, PathBuf};
use std::process::Command;

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

fn a_repo(name: &str) -> PathBuf {
    let dir = std::env::temp_dir()
        .join(format!("hydra-worktree-{}", std::process::id()))
        .join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    git(&dir, &["init", "-q", "-b", "main"]);
    git(&dir, &["config", "user.email", "test@example.invalid"]);
    git(&dir, &["config", "user.name", "hydra tests"]);
    std::fs::write(dir.join("committed.txt"), "one\n").unwrap();
    std::fs::write(dir.join(".gitignore"), "ignored/\n").unwrap();
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-qm", "first"]);
    dir
}

#[test]
fn the_seed_carries_tracked_edits_and_untracked_files() {
    let repo = a_repo("seeded");

    // Work in progress of all three shapes.
    std::fs::write(repo.join("committed.txt"), "one\ntwo\n").unwrap();
    std::fs::write(repo.join("brand-new.txt"), "fresh\n").unwrap();
    std::fs::create_dir_all(repo.join("ignored")).unwrap();
    std::fs::write(repo.join("ignored/build.o"), "junk\n").unwrap();

    let dest = repo.parent().unwrap().join("wt");
    let _ = std::fs::remove_dir_all(&dest);
    let worktree =
        hydra::worktree::create(&repo, &dest, "hydra/test-run", &index(&repo, "seed")).unwrap();

    assert_eq!(worktree.branch, "hydra/test-run");
    assert_eq!(
        std::fs::read_to_string(dest.join("committed.txt")).unwrap(),
        "one\ntwo\n",
        "the tracked edit did not reach the child"
    );
    assert_eq!(
        std::fs::read_to_string(dest.join("brand-new.txt")).unwrap(),
        "fresh\n",
        "the untracked file did not reach the child"
    );
    assert!(
        !dest.join("ignored/build.o").exists(),
        "an ignored file was copied; that is how a parent's run records reach a child"
    );

    // The parent is untouched: this branch exists only in the child's checkout.
    assert_eq!(
        git(&repo, &["rev-parse", "--abbrev-ref", "HEAD"]).trim(),
        "main"
    );
}

#[test]
fn a_directory_outside_a_repo_has_no_root() {
    let plain = std::env::temp_dir().join(format!("hydra-plain-{}", std::process::id()));
    std::fs::create_dir_all(&plain).unwrap();
    // std::env::temp_dir() on macOS is under /var/folders, which is not a repo.
    assert!(hydra::worktree::repo_root(&plain).is_none());
    assert!(hydra::worktree::git_common_dir(&plain).is_none());
}

#[test]
fn a_linked_worktree_names_the_parents_git_directory_and_not_its_own() {
    // The path harness spec 7.5 denies is the one every checkout of a repo
    // shares, because it holds every run's branch. `git rev-parse
    // --show-toplevel` inside a linked worktree answers with the worktree, so
    // `<toplevel>/.git` there names a file in the child's own checkout and the
    // directory that matters goes undenied. `--git-common-dir` answers with the
    // shared one from either side.
    let repo = a_repo("common-dir");
    let dest = repo.parent().unwrap().join("common-dir-wt");
    let _ = std::fs::remove_dir_all(&dest);
    hydra::worktree::create(
        &repo,
        &dest,
        "hydra/common-dir",
        &index(&repo, "common-dir"),
    )
    .unwrap();

    let shared = std::fs::canonicalize(repo.join(".git")).unwrap();
    let common = |dir: &Path| {
        hydra::worktree::git_common_dir(dir).map(|path| std::fs::canonicalize(path).unwrap())
    };
    assert_eq!(common(&repo), Some(shared.clone()));
    assert_eq!(
        common(&dest),
        Some(shared),
        "a launch from inside a checkout left the parent's history undenied"
    );
    assert!(
        dest.join(".git").is_file(),
        "the checkout's own .git is a pointer file, which is what makes the join wrong"
    );
}

/// A scratch index path outside the checkout it stages.
fn index(repo: &Path, name: &str) -> PathBuf {
    repo.parent().unwrap().join(format!("{name}-index"))
}

// --- the diff stat, contract 17 ------------------------------------------

#[test]
fn the_diff_stat_counts_the_child_and_not_the_seed() {
    let repo = a_repo("stat");

    // The parent's work in progress. This is the trap: a naive `git diff HEAD`
    // inside the child's checkout reports these two lines as the child's.
    std::fs::write(repo.join("committed.txt"), "one\ntwo\n").unwrap();
    std::fs::write(repo.join("brand-new.txt"), "fresh\n").unwrap();

    let dest = repo.parent().unwrap().join("stat-wt");
    let _ = std::fs::remove_dir_all(&dest);
    let worktree =
        hydra::worktree::create(&repo, &dest, "hydra/stat-run", &index(&repo, "stat-seed"))
            .unwrap();
    let base = worktree
        .seed_tree
        .expect("the seed left no tree to measure from");

    // Before the child touches anything, the seed is the whole difference and
    // the stat against it is empty.
    let quiet = hydra::worktree::diff_stat(&dest, &index(&repo, "stat-a"), &base).unwrap();
    assert_eq!(
        quiet,
        hydra::worktree::Stat::default(),
        "the parent's uncommitted work was counted as the child's"
    );

    // What the child did: one line into a seeded file, one new file, one delete.
    std::fs::write(dest.join("committed.txt"), "one\ntwo\nthree\n").unwrap();
    std::fs::write(dest.join("child.txt"), "a\nb\n").unwrap();
    std::fs::remove_file(dest.join("brand-new.txt")).unwrap();

    let stat = hydra::worktree::diff_stat(&dest, &index(&repo, "stat-b"), &base).unwrap();
    assert_eq!(stat.files_changed, 3);
    assert_eq!(stat.insertions, 3, "one line edited plus two written");
    assert_eq!(stat.deletions, 1, "the seeded file the child removed");
}

#[test]
fn the_seed_snapshot_leaves_the_childs_own_index_alone() {
    let repo = a_repo("index");
    std::fs::write(repo.join("committed.txt"), "one\ntwo\n").unwrap();

    let dest = repo.parent().unwrap().join("index-wt");
    let _ = std::fs::remove_dir_all(&dest);
    hydra::worktree::create(&repo, &dest, "hydra/index-run", &index(&repo, "kept")).unwrap();

    // A child asked to finish work in progress runs `git status` and has to see
    // the seed as work in progress. Staging it into the checkout's own index
    // would show a tree the parent does not have.
    let staged = git(&dest, &["diff", "--cached", "--name-only"]);
    assert!(
        staged.trim().is_empty(),
        "the snapshot staged into the child's index: {staged}"
    );
    assert!(
        git(&dest, &["status", "--porcelain"]).contains("committed.txt"),
        "the seed is not showing as uncommitted work"
    );
}

// --- build ticket 99 -----------------------------------------------------

#[test]
fn a_seed_that_fails_after_worktree_add_leaves_no_checkout_and_no_branch() {
    // The third hiring panel's H39. `create` ran `git worktree add -b` and then
    // four steps with `?`, and the launch writes no record until `create`
    // returns, so a failed seed left a checkout and a branch no record named.
    // An untracked file the seed cannot read fails the copy step.
    use std::os::unix::fs::PermissionsExt;

    let repo = a_repo("rollback");
    let unreadable = repo.join("secret.txt");
    std::fs::write(&unreadable, "no\n").unwrap();
    std::fs::set_permissions(&unreadable, std::fs::Permissions::from_mode(0o000)).unwrap();

    let dest = repo.parent().unwrap().join("rollback-wt");
    let _ = std::fs::remove_dir_all(&dest);
    let failed = hydra::worktree::create(&repo, &dest, "hydra/rollback", &index(&repo, "rollback"));
    std::fs::set_permissions(&unreadable, std::fs::Permissions::from_mode(0o644)).unwrap();

    let err = format!("{:#}", failed.err().expect("the seed should have failed"));
    assert!(err.contains("secret.txt"), "{err}");
    assert!(!dest.exists(), "the checkout outlived its failed seed");
    assert!(
        !dest.with_extension("git-context.json").exists(),
        "the saved git context outlived its checkout"
    );
    assert!(
        git(&repo, &["branch", "--list", "hydra/rollback"])
            .trim()
            .is_empty(),
        "the branch outlived its failed seed"
    );
    assert_eq!(
        git(&repo, &["worktree", "list", "--porcelain"])
            .lines()
            .filter(|line| line.starts_with("worktree "))
            .count(),
        1,
        "git still lists the rolled-back checkout"
    );
}

#[test]
fn a_launch_from_under_worktrees_is_refused() {
    // The third hiring panel's H47: launch discovery runs plain git where the
    // human stands, and inside a run's checkout that git follows a `.git` file
    // the run can rewrite.
    let base = std::env::temp_dir().join(format!("hydra-launch-dir-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let layout = hydra::paths::Layout::at(base.join("hydra"));
    let inside = layout
        .worktree("01a09900-0000-7000-8000-000000000001")
        .join("src");
    std::fs::create_dir_all(&inside).unwrap();
    let outside = base.join("repo");
    std::fs::create_dir_all(&outside).unwrap();

    let err = hydra::run::refuse_launch_inside_checkout(&layout, &inside)
        .expect_err("a launch from a run's checkout went ahead");
    assert!(format!("{err:#}").contains("worktrees"), "{err:#}");
    hydra::run::refuse_launch_inside_checkout(&layout, &outside).unwrap();

    // The same directory reached through a symlink is the same directory.
    let link = base.join("alias");
    std::os::unix::fs::symlink(&inside, &link).unwrap();
    assert!(hydra::run::refuse_launch_inside_checkout(&layout, &link).is_err());

    let _ = std::fs::remove_dir_all(&base);
}

/// The review of build ticket 99, item 4. A curate pass's checkout of the skill
/// store lives under `skill-worktrees/` with a `.git` file the pass can write,
/// the same shape as a run's checkout, and the refusal covered only
/// `worktrees/`.
#[test]
fn a_launch_from_under_skill_worktrees_is_refused() {
    let base = std::env::temp_dir().join(format!("hydra-launch-skill-dir-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let layout = hydra::paths::Layout::at(base.join("hydra"));
    let inside = layout
        .skill_worktrees()
        .join("01a09900-0000-7000-8000-000000000002")
        .join("alpha");
    std::fs::create_dir_all(&inside).unwrap();

    let err = hydra::run::refuse_launch_inside_checkout(&layout, &inside)
        .expect_err("a launch from a curate pass's checkout went ahead");
    assert!(format!("{err:#}").contains("skill-worktrees"), "{err:#}");

    let _ = std::fs::remove_dir_all(&base);
}
