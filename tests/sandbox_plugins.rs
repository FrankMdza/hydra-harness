//! The seatbelt refuses a planted plugin, build ticket 257.
//!
//! Map 103 C: a plugin is code a provider loads into its own process, and on
//! `claude` one can approve a call the gate refused. Every recorded launch's
//! profile denies writes to where a later launch would load one, after the
//! worktree allow, so a shell `mkdir` is refused at the kernel. `tests/gate.rs`
//! holds the file-tool half.
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

/// Where a plugin loads from inside the worktree, spelled out here rather than
/// read off the library so a dropped entry fails a test.
const WORKTREE_PLUGIN_PATHS: [&str; 4] = [
    ".claude/skills",
    ".claude/settings.json",
    ".claude/settings.local.json",
    ".agents/plugins",
];

/// The same in the home directory.
const HOME_PLUGIN_PATHS: [&str; 2] = [".claude/plugins", ".claude/skills"];

struct Seat {
    root: PathBuf,
    worktree: PathBuf,
    profile: PathBuf,
}

impl Drop for Seat {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

fn seat(tag: &str, provider: Provider) -> Seat {
    let root = std::env::temp_dir().join(format!(
        "hydra-sandbox-plugins-{}-{tag}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    let layout = Layout::at(&root);
    layout.ensure().unwrap();
    let set = PolicySet::discover().unwrap();
    let worktree = layout.worktree("r1");
    std::fs::create_dir_all(worktree.join("src")).unwrap();
    let spawn = write_spawn_config(
        &layout,
        "r1",
        &set,
        provider,
        "standard",
        Some(&worktree),
        None,
        None,
        false,
    )
    .unwrap();
    let profile = spawn.sandbox_profile.expect("macOS spawn writes a profile");
    Seat {
        root,
        worktree,
        profile,
    }
}

/// The index of the last line of `text` that is exactly `rule`.
fn last(text: &str, rule: &str) -> Option<usize> {
    text.lines()
        .enumerate()
        .filter(|(_, line)| *line == rule)
        .map(|(index, _)| index)
        .last()
}

/// Every plugin path is denied, and after the last rule that allows writes to
/// the worktree, the home directory or anything else.
fn assert_denied_after_every_allow(text: &str, worktree: &Path) {
    let home = canonical_subpath(&hydra::env::real_home().unwrap());
    let wt = canonical_subpath(worktree);
    let allow = format!(r#"(allow file-write* (subpath "{wt}"))"#);
    let allowed = last(text, &allow).expect("the worktree is not allowed to writes");
    let last_allow = text
        .lines()
        .enumerate()
        .filter(|(_, line)| line.starts_with("(allow file-write*"))
        .map(|(index, _)| index)
        .last()
        .unwrap();
    let mut rules: Vec<String> = WORKTREE_PLUGIN_PATHS
        .iter()
        .map(|rel| format!(r#"(deny file-write* (subpath "{wt}/{rel}"))"#))
        .collect();
    rules.extend(
        HOME_PLUGIN_PATHS
            .iter()
            .map(|rel| format!(r#"(deny file-write* (subpath "{home}/{rel}"))"#)),
    );
    rules.push(r##"(deny file-write* (regex #"/\.claude-plugin$"))"##.to_string());
    rules.push(r##"(deny file-write* (regex #"/\.claude-plugin/"))"##.to_string());
    for rule in rules {
        let denied = last(text, &rule).unwrap_or_else(|| panic!("no {rule} in:\n{text}"));
        assert!(
            allowed < denied && last_allow < denied,
            "{rule} comes before an allow that hands it back:\n{text}"
        );
    }
}

#[test]
fn every_provider_s_profile_denies_the_plugin_paths_after_the_worktree_allow() {
    for (tag, provider) in [
        ("claude", Provider::Claude),
        ("cursor", Provider::CursorAgent),
        ("agy", Provider::Agy),
        ("codex", Provider::Codex),
    ] {
        let seat = seat(tag, provider);
        let text = std::fs::read_to_string(&seat.profile).unwrap();
        assert_denied_after_every_allow(&text, &seat.worktree);
    }
}

#[test]
fn the_copilot_s_narrowed_profile_keeps_the_plugin_denials_last() {
    // The interactive launch inserts its home deny and its allows after the
    // TIOCSTI line, before everything `compose` wrote. The plugin denials have
    // to stay after those allows.
    let seat = seat("copilot", Provider::Claude);
    hydra::sandbox::narrow_for_copilot(&seat.profile, &seat.worktree).unwrap();
    let text = std::fs::read_to_string(&seat.profile).unwrap();
    assert_denied_after_every_allow(&text, &seat.worktree);
}

#[test]
fn under_the_seatbelt_a_shell_cannot_plant_a_plugin_and_can_still_write_src() {
    if !seatbelt_exec_or_skip(
        "under_the_seatbelt_a_shell_cannot_plant_a_plugin_and_can_still_write_src",
    ) {
        return;
    }
    let seat = seat("live", Provider::Claude);
    let wt = canonical_subpath(&seat.worktree);

    // The control: the probe can write in the worktree, so the refusals below
    // are the plugin denials and not a probe that never worked.
    let control = format!("touch '{wt}/src/f'");
    let out = under_seatbelt(&seat.profile, &control);
    assert!(!refused(&out), "{control} was refused: {out}");
    assert!(seat.worktree.join("src/f").exists());

    for command in [
        format!("mkdir -p '{wt}/.claude/skills/x' && touch '{wt}/.claude/skills/x/f'"),
        format!("mkdir -p '{wt}/.agents/plugins' && touch '{wt}/.agents/plugins/p.json'"),
        format!("echo '{{}}' > '{wt}/.claude/settings.local.json'"),
        format!("mkdir -p '{wt}/src/p/.claude-plugin'"),
    ] {
        let out = under_seatbelt(&seat.profile, &command);
        assert!(refused(&out), "{command} was not refused: {out}");
    }
    assert!(!seat.worktree.join(".claude/skills/x/f").exists());
    assert!(!seat.worktree.join(".agents/plugins/p.json").exists());
    assert!(!seat.worktree.join("src/p/.claude-plugin").exists());
}
