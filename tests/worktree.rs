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
    let worktree = hydra::worktree::create(&repo, &dest, "hydra/test-run").unwrap();

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
}
