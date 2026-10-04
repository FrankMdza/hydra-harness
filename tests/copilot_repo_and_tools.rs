//! Copilot repository argument and ungated tool block, build ticket 253.

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use hydra::provider::claude::{InteractiveLaunch, interactive_argv};

fn manifest() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn copilot_bin() -> PathBuf {
    manifest().join("integrations/copilot/bin/hydra-copilot")
}

fn run_copilot(
    home: &Path,
    hydra_stub: &Path,
    model: &str,
    repo: Option<&Path>,
    provider: Option<&str>,
) -> std::process::Output {
    let mut cmd = Command::new(copilot_bin());
    cmd.env("HYDRA_COPILOT_HOME", home)
        .env("HYDRA_BIN", hydra_stub)
        .arg(model);
    if let Some(repo) = repo {
        cmd.arg(repo);
    }
    if let Some(provider) = provider {
        cmd.arg(provider);
    }
    cmd.output().expect("running hydra-copilot")
}

#[test]
fn hydra_copilot_renders_the_repository_and_hands_off_to_cockpit() {
    let scratch = std::env::temp_dir().join(format!("hydra-copilot-test-{}", std::process::id()));
    std::fs::create_dir_all(&scratch).expect("scratch dir");
    let home = scratch.join("copilot-home");
    let other_repo = scratch.join("other-repo");
    std::fs::create_dir(&other_repo).expect("other repo dir");
    Command::new("git")
        .args(["init", "-q"])
        .current_dir(&other_repo)
        .output()
        .expect("git init other repo");

    let stub = scratch.join("hydra-stub");
    std::fs::write(
        &stub,
        "#!/usr/bin/env bash\nprintf 'cwd=%s\\n' \"$PWD\"\nprintf 'argv=%s\\n' \"$*\"\n",
    )
    .expect("write stub");
    let mut perms = std::fs::metadata(&stub)
        .expect("stub metadata")
        .permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&stub, perms).expect("chmod stub");

    let default_repo = manifest().canonicalize().expect("manifest path");
    let out = run_copilot(&home, &stub, "sonnet", None, None);
    assert!(
        out.status.success(),
        "hydra-copilot failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let rendered = std::fs::read_to_string(home.join("CLAUDE.md")).expect("rendered CLAUDE.md");
    assert!(
        rendered.contains(&default_repo.to_string_lossy().to_string()),
        "default launch must render Hydra's checkout into {{REPO}}"
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("cockpit --session claude --model sonnet"),
        "stub argv: {stdout}"
    );

    let out = run_copilot(&home, &stub, "opus", Some(&other_repo), None);
    assert!(
        out.status.success(),
        "named repo failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let rendered = std::fs::read_to_string(home.join("CLAUDE.md")).expect("rendered CLAUDE.md");
    let top = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .current_dir(&other_repo)
        .output()
        .expect("git toplevel");
    let top_line = String::from_utf8_lossy(&top.stdout);
    let top = top_line.trim();
    assert!(
        rendered.contains(top),
        "named repository must render git top level, got {rendered}"
    );

    let not_repo = scratch.join("not-git");
    std::fs::create_dir(&not_repo).expect("not a repo");
    let out = run_copilot(&home, &stub, "sonnet", Some(&not_repo), None);
    assert!(!out.status.success());
    let stderr_line = String::from_utf8_lossy(&out.stderr);
    let stderr = stderr_line.trim();
    assert_eq!(
        stderr,
        format!(
            "hydra-copilot: {} is not a git repository",
            not_repo.display()
        )
    );
}

#[test]
fn ungated_interactive_argv_disallows_file_edit_tools_gated_does_not() {
    let ungated = interactive_argv(&InteractiveLaunch {
        session: "s1",
        task: None,
        model: None,
        mcp: None,
        settings: None,
        policy: None,
        shapes: &[],
    });
    let at = ungated
        .iter()
        .position(|a| a == "--disallowed-tools")
        .expect("ungated copilot must pass --disallowed-tools");
    assert_eq!(ungated[at + 1], "Edit,Write,NotebookEdit");

    let gated = interactive_argv(&InteractiveLaunch {
        session: "s1",
        task: None,
        model: None,
        mcp: None,
        settings: Some(Path::new("/tmp/spawn/r1/settings.json")),
        policy: Some("standard"),
        shapes: &[],
    });
    assert!(
        !gated.iter().any(|a| a == "--disallowed-tools"),
        "gated argv must not carry copilot disallows: {gated:?}"
    );
}

#[test]
fn hydra_copilot_codex_renders_agents_md_and_passes_session_codex() {
    let scratch =
        std::env::temp_dir().join(format!("hydra-copilot-codex-test-{}", std::process::id()));
    std::fs::create_dir_all(&scratch).expect("scratch dir");
    let home = scratch.join("copilot-home");
    let stub = scratch.join("hydra-stub");
    std::fs::write(&stub, "#!/usr/bin/env bash\nprintf 'argv=%s\\n' \"$*\"\n").expect("write stub");
    let mut perms = std::fs::metadata(&stub)
        .expect("stub metadata")
        .permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&stub, perms).expect("chmod stub");

    let default_repo = manifest().canonicalize().expect("manifest path");
    let out = run_copilot(
        &home,
        &stub,
        "gpt-5.4",
        Some(default_repo.as_path()),
        Some("codex"),
    );
    assert!(
        out.status.success(),
        "hydra-copilot codex failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let rendered = std::fs::read_to_string(home.join("AGENTS.md")).expect("rendered AGENTS.md");
    assert!(
        rendered.contains(&default_repo.to_string_lossy().to_string()),
        "codex copilot must render {{REPO}} into AGENTS.md"
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("cockpit --session codex --model gpt-5.4"),
        "stub argv: {stdout}"
    );

    // No model named: `codex` keeps its own default rather than getting Sonnet.
    let out = run_copilot(
        &home,
        &stub,
        "",
        Some(default_repo.as_path()),
        Some("codex"),
    );
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.trim_end().ends_with("cockpit --session codex"),
        "stub argv: {stdout}"
    );
}

#[test]
fn hydra_copilot_unknown_provider_exits_one() {
    let scratch =
        std::env::temp_dir().join(format!("hydra-copilot-bad-provider-{}", std::process::id()));
    std::fs::create_dir_all(&scratch).expect("scratch dir");
    let home = scratch.join("copilot-home");
    let stub = scratch.join("hydra-stub");
    std::fs::write(&stub, "#!/usr/bin/env bash\nexit 0\n").expect("write stub");
    let mut perms = std::fs::metadata(&stub)
        .expect("stub metadata")
        .permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&stub, perms).expect("chmod stub");

    let default_repo = manifest().canonicalize().expect("manifest path");
    let out = run_copilot(
        &home,
        &stub,
        "sonnet",
        Some(default_repo.as_path()),
        Some("agy"),
    );
    assert!(!out.status.success());
    let stderr_line = String::from_utf8_lossy(&out.stderr);
    let stderr = stderr_line.trim();
    assert_eq!(
        stderr,
        "hydra-copilot: provider must be claude, codex or cursor-agent"
    );
}

/// Build ticket 266: `cursor-agent` is accepted, gets the copilot's document
/// as `AGENTS.md`, and keeps its own model when none is named.
#[test]
fn hydra_copilot_cursor_agent_renders_agents_md_and_passes_its_session() {
    let scratch =
        std::env::temp_dir().join(format!("hydra-copilot-cursor-test-{}", std::process::id()));
    std::fs::create_dir_all(&scratch).expect("scratch dir");
    let home = scratch.join("copilot-home");
    let stub = scratch.join("hydra-stub");
    std::fs::write(&stub, "#!/usr/bin/env bash\nprintf 'argv=%s\\n' \"$*\"\n").expect("write stub");
    let mut perms = std::fs::metadata(&stub)
        .expect("stub metadata")
        .permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&stub, perms).expect("chmod stub");

    let default_repo = manifest().canonicalize().expect("manifest path");
    let out = run_copilot(
        &home,
        &stub,
        "",
        Some(default_repo.as_path()),
        Some("cursor-agent"),
    );
    assert!(
        out.status.success(),
        "hydra-copilot cursor-agent failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let rendered = std::fs::read_to_string(home.join("AGENTS.md")).expect("rendered AGENTS.md");
    assert!(
        rendered.contains(&default_repo.to_string_lossy().to_string()),
        "the cursor-agent copilot must render {{REPO}} into AGENTS.md"
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout
            .trim_end()
            .ends_with("cockpit --session cursor-agent"),
        "stub argv: {stdout}"
    );
    let _ = std::fs::remove_dir_all(&scratch);
}

/// Master of Puppets mode: the start directory gets the mode's document after the copilot's in
/// both instruction files, and the `puppets` skill as a project skill, all naming the repository.
#[test]
fn hydra_copilot_in_puppets_mode_installs_the_skill_and_the_loop() {
    let scratch =
        std::env::temp_dir().join(format!("hydra-copilot-puppets-{}", std::process::id()));
    std::fs::create_dir_all(&scratch).expect("scratch dir");
    let home = scratch.join("copilot-home");
    let stub = scratch.join("hydra-stub");
    std::fs::write(&stub, "#!/usr/bin/env bash\nprintf 'argv=%s\\n' \"$*\"\n").expect("write stub");
    let mut perms = std::fs::metadata(&stub)
        .expect("stub metadata")
        .permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&stub, perms).expect("chmod stub");
    let repo = manifest().canonicalize().expect("manifest path");

    let out = Command::new(copilot_bin())
        .env("HYDRA_COPILOT_HOME", &home)
        .env("HYDRA_BIN", &stub)
        .env("HYDRA_COPILOT_MODE", "puppets")
        .args(["sonnet", repo.to_str().unwrap()])
        .output()
        .expect("running hydra-copilot");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    for name in ["CLAUDE.md", "AGENTS.md"] {
        let text = std::fs::read_to_string(home.join(name)).expect("rendered document");
        assert!(text.contains("## Master of Puppets mode"), "{name}");
        assert!(
            text.contains("## The loop"),
            "{name} carries the skill's text"
        );
        assert!(!text.contains("{{"), "{name} has an unrendered placeholder");
        assert!(
            !text.contains("name: puppets"),
            "{name} carries no frontmatter"
        );
    }
    let skill = std::fs::read_to_string(home.join(".claude/skills/puppets/SKILL.md"))
        .expect("the skill is installed as a project skill");
    assert!(skill.starts_with("---\nname: puppets"));

    let out = Command::new(copilot_bin())
        .env("HYDRA_COPILOT_HOME", scratch.join("other"))
        .env("HYDRA_BIN", &stub)
        .env("HYDRA_COPILOT_MODE", "nope")
        .output()
        .expect("running hydra-copilot");
    assert_eq!(out.status.code(), Some(1));
}
