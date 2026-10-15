//! The gated copilot's narrower seatbelt, build ticket 250.
//!
//! Map ticket 98: a gated interactive session under `standard` wrote
//! `~/prueba-fuera.txt` with a shell `printf`. The profile of that launch, and
//! of no other, denies writes under the home directory and then hands back the
//! worktree, the temp directories, the spawn directory and `claude`'s state.
//!
//! The live probe needs `sandbox-exec`, which cannot nest, so inside a Hydra
//! run it prints that it skipped. The profile text is checked either way.

#![cfg(target_os = "macos")]

use std::path::{Path, PathBuf};

use hydra::paths::Layout;
use hydra::policy::PolicySet;
use hydra::provider::Provider;
use hydra::run::write_spawn_config;

mod corpus;
use corpus::{canonical_subpath, refused, seatbelt_exec_or_skip, under_seatbelt};

struct Scratch {
    root: PathBuf,
    layout: Layout,
    worktree: PathBuf,
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

fn scratch(tag: &str) -> Scratch {
    let root = std::env::temp_dir().join(format!(
        "hydra-sandbox-copilot-{}-{tag}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    let layout = Layout::at(&root);
    layout.ensure().unwrap();
    let worktree = root.join("worktree");
    std::fs::create_dir_all(&worktree).unwrap();
    Scratch {
        root,
        layout,
        worktree,
    }
}

/// The headless profile a `standard` `claude` spawn writes.
fn headless_profile(scratch: &Scratch) -> PathBuf {
    write_spawn_config(
        &scratch.layout,
        "r1",
        &PolicySet::discover().unwrap(),
        Provider::Claude,
        "standard",
        Some(&scratch.worktree),
        None,
        None,
        false,
    )
    .unwrap()
    .sandbox_profile
    .expect("macOS spawn writes a profile")
}

fn home() -> PathBuf {
    hydra::env::real_home().unwrap()
}

fn home_deny() -> String {
    format!(
        r#"(deny file-write* (subpath "{}"))"#,
        canonical_subpath(&home())
    )
}

const TIOCSTI: &str = "(deny file-ioctl (ioctl-command TIOCSTI))";

#[test]
fn the_copilot_profile_is_the_headless_one_with_the_home_deny_and_its_allows_after_tiocsti() {
    let scratch = scratch("text");
    let profile = headless_profile(&scratch);
    let headless = std::fs::read_to_string(&profile).unwrap();
    assert!(
        !headless.lines().any(|line| line == home_deny()),
        "a headless profile denies writes under home:\n{headless}"
    );

    let sha = hydra::sandbox::narrow_for_copilot(&profile, &scratch.worktree).unwrap();
    let copilot = std::fs::read_to_string(&profile).unwrap();
    {
        use hydra::hex::hex_lower;
        use sha2::{Digest, Sha256};
        assert_eq!(sha, hex_lower(&Sha256::digest(copilot.as_bytes())));
    }

    let block =
        hydra::sandbox::copilot_block(&home(), &scratch.worktree, profile.parent().unwrap())
            .unwrap();
    let lines: Vec<&str> = copilot.lines().collect();
    let at = lines.iter().position(|line| *line == TIOCSTI).unwrap();
    assert_eq!(&lines[at + 1..at + 1 + block.len()], block.as_slice());
    assert_eq!(block[0], home_deny(), "the deny comes before its allows");
    let wt = canonical_subpath(&scratch.worktree);
    assert!(
        block[1..].contains(&format!(r#"(allow file-write* (subpath "{wt}"))"#)),
        "{block:#?}"
    );
    assert!(block[1..].iter().all(|line| line.starts_with("(allow ")));

    // Without the block, every other byte is the headless profile's.
    let mut rest = lines.clone();
    rest.drain(at + 1..at + 1 + block.len());
    assert_eq!(format!("{}\n", rest.join("\n")), headless);
}

#[test]
fn the_copilot_block_hands_back_no_file_claude_reads_as_instructions() {
    let scratch = scratch("instructions");
    let block = hydra::sandbox::copilot_block(&home(), &scratch.worktree, &scratch.root).unwrap();
    let claude = format!("{}/.claude", canonical_subpath(&home()));
    for line in &block[1..] {
        assert!(
            !line.contains(&format!(r#"(subpath "{claude}")"#)),
            "the whole of ~/.claude is writable: {line}"
        );
        for name in [
            "settings",
            "hooks",
            "skills",
            "plugins",
            "commands",
            "agents",
            "CLAUDE.md",
        ] {
            assert!(
                !line.contains(&format!("{claude}/{name}")),
                "{name} is writable: {line}"
            );
        }
    }
}

/// `git` that ignores the object store a Hydra run's shell points at.
fn git(repo: &Path, args: &[&str]) {
    let out = std::process::Command::new("git")
        .args(args)
        .current_dir(repo)
        .env_remove("GIT_OBJECT_DIRECTORY")
        .env_remove("GIT_ALTERNATE_OBJECT_DIRECTORIES")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn a_gated_interactive_launch_records_the_narrowed_profile() {
    let scratch = scratch("launch");
    let repo = scratch.root.join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    git(&repo, &["init", "-q", "-b", "main"]);
    git(&repo, &["config", "user.email", "test@example.invalid"]);
    git(&repo, &["config", "user.name", "hydra tests"]);
    std::fs::write(repo.join("README"), "seed\n").unwrap();
    git(&repo, &["add", "README"]);
    git(&repo, &["commit", "-qm", "seed", "README"]);

    let prep = hydra::interactive::prepare_policy_launch(
        &scratch.layout,
        "r1",
        "standard",
        &PolicySet::discover().unwrap(),
        &repo,
        &repo,
    )
    .unwrap();
    let profile = prep.spawn.sandbox_profile.as_ref().unwrap();
    let text = std::fs::read_to_string(profile).unwrap();
    let lines: Vec<&str> = text.lines().collect();
    let at = lines.iter().position(|line| *line == TIOCSTI).unwrap();
    assert_eq!(lines[at + 1], home_deny(), "{text}");
    {
        use hydra::hex::hex_lower;
        use sha2::{Digest, Sha256};
        assert_eq!(
            prep.spawn.sandbox_sha.as_deref(),
            Some(hex_lower(&Sha256::digest(text.as_bytes())).as_str()),
            "run_start.sandbox is not the sha of the profile the launch runs under"
        );
    }
}

#[test]
fn under_the_copilot_profile_a_shell_write_to_home_is_refused() {
    if !seatbelt_exec_or_skip("under_the_copilot_profile_a_shell_write_to_home_is_refused") {
        return;
    }
    let scratch = scratch("live");
    let profile = headless_profile(&scratch);
    hydra::sandbox::narrow_for_copilot(&profile, &scratch.worktree).unwrap();

    let planted = home().join(format!(".hydra-build-250-probe-{}", std::process::id()));
    let _ = std::fs::remove_file(&planted);
    let wt = canonical_subpath(&scratch.worktree);

    // The control: the probe can write where the session works, so the refusal
    // below is the kernel's and not a probe that never ran.
    let out = under_seatbelt(&profile, &format!("printf x > '{wt}/own.txt'"));
    assert!(!refused(&out), "a worktree write was refused: {out}");
    assert_eq!(
        std::fs::read_to_string(scratch.worktree.join("own.txt")).unwrap(),
        "x"
    );

    let out = under_seatbelt(&profile, &format!("printf x > '{}'", planted.display()));
    let landed = planted.exists();
    let _ = std::fs::remove_file(&planted);
    assert!(refused(&out), "a write to home was not refused: {out}");
    assert!(!landed, "{} was written", planted.display());
}
