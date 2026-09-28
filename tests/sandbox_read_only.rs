//! The seatbelt under a bounded shell, build ticket 139.
//!
//! Map 82 answer 3: a seat that reads files gets a shell limited to a list, and
//! the list is the first filter. The guarantee is the profile: under a policy
//! whose shell is bounded, the kernel refuses a write inside the run's own
//! worktree and a read of the rest of Hydra's tree. `tests/gate.rs` holds the
//! list half. The policy is `tests/fixtures/policies/read-only.json`, because
//! `policies/` is protected config #1 and the human's to write.
//!
//! The live probes need `sandbox-exec`, which cannot nest, so inside a Hydra
//! run they print that they skipped. The profile text is checked either way.

#![cfg(target_os = "macos")]

use std::path::{Path, PathBuf};

use hydra::paths::Layout;
use hydra::policy::PolicySet;
use hydra::provider::Provider;
use hydra::run::write_spawn_config;

mod corpus;
use corpus::{
    bounded_shell_set, canonical_subpath, refused, seatbelt_exec_or_skip, under_seatbelt,
};

const SHA: &str = "0123456789abcdef0123456789abcdef01234567";

struct Seat {
    root: PathBuf,
    layout: Layout,
    worktree: PathBuf,
    profile: PathBuf,
}

impl Drop for Seat {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

/// A spawn under `policy` out of `set`, with a file in the worktree, one in a
/// sibling's worktree, one in the store and one in this run's snapshot.
fn seat(tag: &str, policy: &str, set: impl FnOnce(&Path) -> PolicySet) -> Seat {
    let root = std::env::temp_dir().join(format!(
        "hydra-sandbox-read-only-{}-{tag}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    let layout = Layout::at(&root);
    layout.ensure().unwrap();
    let set = set(&root.join("policies"));
    let worktree = layout.worktree("r1");
    let sibling = layout.worktree("r2");
    let snapshot = layout.snapshot(SHA);
    for (dir, file) in [
        (&worktree, "own.txt"),
        (&sibling, "theirs.txt"),
        (&layout.skills(), "store.txt"),
        (&snapshot, "SKILL.md"),
    ] {
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(dir.join(file), "x\n").unwrap();
    }
    let spawn = write_spawn_config(
        &layout,
        "r1",
        &set,
        Provider::Claude,
        policy,
        Some(&worktree),
        None,
        Some(SHA),
        false,
    )
    .unwrap();
    let profile = spawn.sandbox_profile.expect("macOS spawn writes a profile");
    Seat {
        root,
        layout,
        worktree,
        profile,
    }
}

fn bounded(tag: &str) -> Seat {
    seat(tag, "read-only", bounded_shell_set)
}

/// The index of the last line of `text` that is exactly `rule`.
fn last(text: &str, rule: &str) -> Option<usize> {
    text.lines()
        .enumerate()
        .filter(|(_, line)| *line == rule)
        .map(|(index, _)| index)
        .last()
}

#[test]
fn the_profile_for_a_bounded_shell_denies_worktree_writes_and_hydra_reads() {
    let seat = bounded("text");
    let text = std::fs::read_to_string(&seat.profile).unwrap();
    let wt = canonical_subpath(&seat.worktree);
    let deny_write = format!(r#"(deny file-write* (subpath "{wt}"))"#);
    let allow_write = format!(r#"(allow file-write* (subpath "{wt}"))"#);
    // SBPL takes the last matching rule, so the deny has to come last.
    let denied = last(&text, &deny_write).expect("the worktree is not denied to writes");
    assert!(
        last(&text, &allow_write).is_none_or(|allowed| allowed < denied),
        "a later rule hands the worktree back to writes:\n{text}"
    );

    // Hydra's whole tree, which holds the sibling worktrees, the store and the
    // snapshots, and then this run's own worktree and snapshot handed back.
    let root = canonical_subpath(seat.layout.root());
    let deny_read = format!(r#"(deny file-read-data (subpath "{root}"))"#);
    let d = last(&text, &deny_read).expect("Hydra's tree is not denied to reads");
    for own in [&seat.worktree, &seat.layout.snapshot(SHA)] {
        let allow_read = format!(
            r#"(allow file-read* (subpath "{}"))"#,
            canonical_subpath(own)
        );
        let a = last(&text, &allow_read).expect("an own path is not readable again");
        assert!(
            d < a,
            "{} is denied after it is allowed:\n{text}",
            own.display()
        );
    }
}

#[test]
fn an_unbounded_shell_keeps_its_worktree_writable() {
    // `standard` is the control: nothing that runs today changes.
    let seat = seat("standard", "standard", |_| PolicySet::discover().unwrap());
    let text = std::fs::read_to_string(&seat.profile).unwrap();
    let wt = canonical_subpath(&seat.worktree);
    assert!(text.contains(&format!(r#"(allow file-write* (subpath "{wt}"))"#)));
    assert!(!text.contains(&format!(r#"(deny file-write* (subpath "{wt}"))"#)));
    assert!(!text.contains("file-read-data"), "{text}");
}

#[test]
fn under_a_bounded_shell_the_seatbelt_refuses_a_worktree_write_and_a_read_outside_it() {
    if !seatbelt_exec_or_skip(
        "under_a_bounded_shell_the_seatbelt_refuses_a_worktree_write_and_a_read_outside_it",
    ) {
        return;
    }
    let seat = bounded("live");
    let wt = canonical_subpath(&seat.worktree);
    let sibling = canonical_subpath(&seat.layout.worktree("r2"));
    let store = canonical_subpath(&seat.layout.skills());
    let snapshot = canonical_subpath(&seat.layout.snapshot(SHA));

    // The controls: the probe can read what the policy reads, so the refusals
    // below are the kernel's and not a probe that never worked.
    for command in [
        format!("cat '{wt}/own.txt'"),
        format!("cat '{snapshot}/SKILL.md'"),
    ] {
        let out = under_seatbelt(&seat.profile, &command);
        assert!(!refused(&out), "{command} was refused: {out}");
    }

    for command in [
        format!("echo x > '{wt}/planted'"),
        format!("echo x >> '{wt}/own.txt'"),
        format!("cat '{sibling}/theirs.txt'"),
        format!("ls '{sibling}'"),
        format!("cat '{store}/store.txt'"),
    ] {
        let out = under_seatbelt(&seat.profile, &command);
        assert!(refused(&out), "{command} was not refused: {out}");
    }
    assert_eq!(
        std::fs::read_to_string(seat.worktree.join("own.txt")).unwrap(),
        "x\n"
    );
    assert!(!seat.worktree.join("planted").exists());
}
