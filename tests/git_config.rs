//! Build ticket 99, the third hiring panel's H5: git that Hydra runs outside the
//! seatbelt reads no global or system configuration, and still commits under
//! the identity that configuration names.
//!
//! Each test runs in a child copy of this binary with its own environment.
//! `GIT_CONFIG_GLOBAL` and `GIT_CONFIG_SYSTEM` stand in for a `~/.gitconfig` a
//! run rewrote and a system file the same user owns, and `XDG_CONFIG_HOME` for
//! the directory git reads its default attributes and ignore files from. No
//! test mutates the environment of the parallel test runner.
//!
//! Build ticket 105 added four, one per item of the review of build ticket 99
//! that changed what Hydra's git reads.

#[path = "common/git.rs"]
mod fixture_git;

use std::path::{Path, PathBuf};
use std::process::Command;

/// Re-execute exactly one test with the planted environment. Check that the
/// named test actually ran, so a misspelled filter cannot silently pass.
fn isolated_test(name: &str, configure: impl FnOnce(&mut Command, &Path)) -> Option<PathBuf> {
    const CHILD: &str = "HYDRA_GIT_CONFIG_TEST_CHILD";
    const ROOT: &str = "HYDRA_GIT_CONFIG_TEST_ROOT";
    if std::env::var(CHILD).as_deref() == Ok(name) {
        return Some(PathBuf::from(std::env::var_os(ROOT).unwrap()));
    }
    let base = scratch(name);
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args(["--exact", name, "--nocapture"])
        .env(CHILD, name)
        .env(ROOT, &base);
    configure(&mut command, &base);
    let out = command.output().unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "child failed:\n{stdout}\n{stderr}");
    assert!(
        stdout.contains(&format!("test {name} ... ok")),
        "the child did not run the named test:\n{stdout}\n{stderr}"
    );
    let _ = std::fs::remove_dir_all(&base);
    None
}

/// A scratch directory for one test, empty.
fn scratch(name: &str) -> PathBuf {
    let base = std::env::temp_dir().join(format!("hydra-git-config-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(&base).unwrap();
    // Canonical, so a path written into a config matches the one git computes.
    std::fs::canonicalize(&base).unwrap()
}

/// A repository at `base/repo` with one commit, made with no global config.
fn committed_repo(base: &Path) -> PathBuf {
    let repo = base.join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    clean_git(&repo, &["init", "-q", "-b", "main"]);
    std::fs::write(repo.join("committed.txt"), "one\n").unwrap();
    clean_git(&repo, &["add", "-A"]);
    clean_git(
        &repo,
        &[
            "-c",
            "user.name=setup",
            "-c",
            "user.email=setup@example.invalid",
            "commit",
            "-qm",
            "first",
        ],
    );
    repo
}

/// Git for the test's own setup, reading neither of the planted files and
/// using the real ownership while the safe-directory probe assumes another.
fn clean_git(dir: &Path, args: &[&str]) -> String {
    let out = fixture_git::command()
        .args(args)
        .current_dir(dir)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env_remove("GIT_TEST_ASSUME_DIFFERENT_OWNER")
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

fn executable(path: &Path, body: &str) {
    use std::os::unix::fs::PermissionsExt;
    std::fs::write(path, body).unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
}

fn fired(markers: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(markers)
        .unwrap()
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().to_string())
        .collect();
    names.sort();
    names
}

#[test]
fn hydras_git_runs_nothing_global_or_system_config_names_and_keeps_the_identity() {
    let Some(base) = isolated_test(
        "hydras_git_runs_nothing_global_or_system_config_names_and_keeps_the_identity",
        |command, base| {
            command
                .env("GIT_CONFIG_GLOBAL", base.join("gitconfig"))
                .env("GIT_CONFIG_SYSTEM", base.join("system-gitconfig"))
                .env_remove("GIT_CONFIG_NOSYSTEM");
        },
    ) else {
        return;
    };
    let markers = base.join("markers");
    let hooks = base.join("hooks");
    std::fs::create_dir_all(&markers).unwrap();
    std::fs::create_dir_all(&hooks).unwrap();

    // What a run can write by shell under `standard`: a global config with a
    // hook directory, a clean filter and an external diff, and a system config
    // with a filter of its own.
    executable(
        &hooks.join("post-checkout"),
        &format!("#!/bin/sh\ntouch {}/hook\n", markers.display()),
    );
    let ext_diff = base.join("ext-diff");
    executable(
        &ext_diff,
        &format!("#!/bin/sh\ntouch {}/ext-diff\n", markers.display()),
    );
    let global = base.join("gitconfig");
    std::fs::write(
        &global,
        format!(
            "[user]\n\tname = Global Tester\n\temail = global@example.invalid\n\
             [core]\n\thooksPath = {hooks}\n\
             [diff]\n\texternal = {ext}\n\
             [filter \"evil\"]\n\tclean = \"touch {m}/global-filter; cat\"\n",
            hooks = hooks.display(),
            ext = ext_diff.display(),
            m = markers.display(),
        ),
    )
    .unwrap();
    let system = base.join("system-gitconfig");
    std::fs::write(
        &system,
        format!(
            "[filter \"sysevil\"]\n\tclean = \"touch {}/system-filter; cat\"\n",
            markers.display()
        ),
    )
    .unwrap();

    // A parent with no identity of its own and work in progress, so the seed's
    // `git diff HEAD` has something to say.
    let repo = base.join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    clean_git(&repo, &["init", "-q", "-b", "main"]);
    std::fs::write(repo.join("committed.txt"), "one\n").unwrap();
    clean_git(&repo, &["add", "-A"]);
    clean_git(
        &repo,
        &[
            "-c",
            "user.name=setup",
            "-c",
            "user.email=setup@example.invalid",
            "commit",
            "-qm",
            "first",
        ],
    );
    std::fs::write(repo.join("committed.txt"), "one\ntwo\n").unwrap();

    let dest = base.join("checkout");
    let worktree =
        hydra::worktree::create(&repo, &dest, "hydra/git-config", &base.join("seed-index"))
            .unwrap();
    assert_eq!(
        std::fs::read_to_string(dest.join("committed.txt")).unwrap(),
        "one\ntwo\n",
        "the seed's patch did not apply"
    );
    assert!(worktree.seed_tree.is_some());

    // What the run then writes into its own checkout, and what reap commits.
    std::fs::write(
        dest.join(".gitattributes"),
        "*.txt filter=evil\n*.md filter=sysevil\n",
    )
    .unwrap();
    std::fs::write(dest.join("work.txt"), "child\n").unwrap();
    std::fs::write(dest.join("notes.md"), "child\n").unwrap();
    let common = hydra::worktree::git_common_dir(&repo).unwrap();
    let context = hydra::worktree::CheckoutGit::new(&dest, &common).unwrap();
    let commit = hydra::worktree::commit_everything(&context, "reap", &[])
        .unwrap()
        .expect("the checkout held work");

    assert_eq!(
        fired(&markers),
        Vec::<String>::new(),
        "git that Hydra ran read a program out of global or system configuration"
    );
    assert_eq!(
        clean_git(&repo, &["log", "-1", "--format=%an <%ae>", &commit]).trim(),
        "Global Tester <global@example.invalid>",
        "the commit lost the identity the ignored configuration named"
    );

    let _ = std::fs::remove_dir_all(&base);
}

/// The review of build ticket 99, item 1, measured there. A human who ignores
/// a file only through a custom `core.excludesFile` in `~/.gitconfig` still has
/// it ignored: the seed does not copy it into the run's checkout, and reap's
/// `add -A` does not commit it when the run writes one of its own.
#[test]
fn a_file_ignored_only_by_a_global_excludes_file_is_neither_seeded_nor_committed() {
    let Some(base) = isolated_test(
        "a_file_ignored_only_by_a_global_excludes_file_is_neither_seeded_nor_committed",
        |command, base| {
            command.env("GIT_CONFIG_GLOBAL", base.join("gitconfig"));
        },
    ) else {
        return;
    };
    let repo = committed_repo(&base);
    std::fs::write(repo.join("secret.env"), "TOKEN=parent\n").unwrap();
    std::fs::write(repo.join("kept.txt"), "work in progress\n").unwrap();

    let excludes = base.join("my-ignores");
    std::fs::write(&excludes, "secret.env\n").unwrap();
    let global = base.join("gitconfig");
    std::fs::write(
        &global,
        format!(
            "[user]\n\tname = Global Tester\n\temail = global@example.invalid\n\
             [core]\n\texcludesFile = {}\n",
            excludes.display()
        ),
    )
    .unwrap();

    let dest = base.join("checkout");
    let created = hydra::worktree::create(&repo, &dest, "hydra/excludes", &base.join("seed-index"));
    let seeded_secret = dest.join("secret.env").exists();
    let seeded_kept = dest.join("kept.txt").exists();

    // What the run writes, and what reap commits.
    std::fs::write(dest.join("secret.env"), "TOKEN=child\n").unwrap();
    std::fs::write(dest.join("work.txt"), "child\n").unwrap();
    let common = hydra::worktree::git_common_dir(&repo).unwrap();
    let context = hydra::worktree::CheckoutGit::new(&dest, &common).unwrap();
    let committed = hydra::worktree::commit_everything(&context, "reap", &[]);

    created.unwrap();
    assert!(
        seeded_kept,
        "the seed skipped an untracked file nobody ignores"
    );
    assert!(
        !seeded_secret,
        "the seed copied a file the human's global excludes file ignores"
    );
    let commit = committed.unwrap().expect("the checkout held work");
    let files = clean_git(&repo, &["ls-tree", "-r", "--name-only", &commit]);
    assert!(files.contains("work.txt"), "{files}");
    assert!(
        !files.contains("secret.env"),
        "reap committed a file the human's global excludes file ignores: {files}"
    );
    let _ = std::fs::remove_dir_all(&base);
}

/// The review of build ticket 99, item 2. Git honours `safe.directory` only
/// from system, global or command-line configuration, so once Hydra's git read
/// neither of the first two, a repository owned by someone else stopped being
/// found at all. `GIT_TEST_ASSUME_DIFFERENT_OWNER` is git's own switch for
/// making every repository look that way, and release builds read it.
#[test]
fn a_repository_that_needs_safe_directory_is_still_found() {
    let Some(base) = isolated_test(
        "a_repository_that_needs_safe_directory_is_still_found",
        |command, base| {
            command
                .env("GIT_CONFIG_GLOBAL", base.join("gitconfig"))
                .env("GIT_TEST_ASSUME_DIFFERENT_OWNER", "1");
        },
    ) else {
        return;
    };
    let repo = committed_repo(&base);
    let global = base.join("gitconfig");
    std::fs::write(
        &global,
        format!("[safe]\n\tdirectory = {}\n", repo.display()),
    )
    .unwrap();

    let root = hydra::worktree::repo_root(&repo);
    let common = hydra::worktree::git_common_dir(&repo);

    assert_eq!(
        root.as_deref(),
        Some(repo.as_path()),
        "a repository the human marked safe was not found"
    );
    assert_eq!(common, Some(repo.join(".git")));
    let _ = std::fs::remove_dir_all(&base);
}

/// The review of build ticket 99, item 5. Git reads
/// `$XDG_CONFIG_HOME/git/attributes` with global configuration off, and a run
/// can write that file. `*.txt text` there made reap's `git add` turn the run's
/// CRLF file into LF, so the commit held bytes the checkout never did.
#[test]
fn an_attributes_file_under_xdg_config_home_changes_nothing_reap_commits() {
    let Some(base) = isolated_test(
        "an_attributes_file_under_xdg_config_home_changes_nothing_reap_commits",
        |command, base| {
            command.env("XDG_CONFIG_HOME", base.join("xdg"));
        },
    ) else {
        return;
    };
    let repo = committed_repo(&base);
    let dest = base.join("checkout");
    hydra::worktree::create(&repo, &dest, "hydra/attributes", &base.join("seed-index")).unwrap();

    let xdg = base.join("xdg");
    std::fs::create_dir_all(xdg.join("git")).unwrap();
    std::fs::write(xdg.join("git/attributes"), "*.txt text\n").unwrap();
    std::fs::write(dest.join("crlf.txt"), b"one\r\ntwo\r\n").unwrap();
    let common = hydra::worktree::git_common_dir(&repo).unwrap();
    let context = hydra::worktree::CheckoutGit::new(&dest, &common).unwrap();
    let committed = hydra::worktree::commit_everything(&context, "reap", &[]);

    let commit = committed.unwrap().expect("the checkout held work");
    let out = fixture_git::command()
        .args(["cat-file", "-p", &format!("{commit}:crlf.txt")])
        .current_dir(&repo)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .unwrap();
    assert_eq!(
        out.stdout, b"one\r\ntwo\r\n",
        "reap committed bytes an attributes file the run wrote had converted"
    );
    let _ = std::fs::remove_dir_all(&base);
}

/// The review of build ticket 99, item 5. `identity` and every `git` Hydra
/// starts read `~/.gitconfig` as data first, and a run can write an
/// `[include]` there naming a fifo. Git blocks opening it until something
/// writes the other end, so reap would have hung for good. Now each read gives
/// up after `CONFIG_READ_TIMEOUT` and the git that follows reads no global
/// value.
#[test]
fn a_global_config_that_includes_a_fifo_does_not_hang_hydras_git() {
    let Some(base) = isolated_test(
        "a_global_config_that_includes_a_fifo_does_not_hang_hydras_git",
        |command, base| {
            command.env("GIT_CONFIG_GLOBAL", base.join("gitconfig"));
        },
    ) else {
        return;
    };
    let repo = committed_repo(&base);
    let fifo = base.join("fifo");
    let made = Command::new("mkfifo").arg(&fifo).status().unwrap();
    assert!(made.success(), "mkfifo failed");
    let global = base.join("gitconfig");
    std::fs::write(
        &global,
        format!(
            "[user]\n\tname = Global Tester\n\temail = global@example.invalid\n\
             [include]\n\tpath = {}\n",
            fifo.display()
        ),
    )
    .unwrap();

    let (sent, answer) = std::sync::mpsc::channel();
    let probe = repo.clone();
    std::thread::spawn(move || {
        let identity = hydra::worktree::identity(probe.as_path());
        let root = hydra::worktree::repo_root(&probe);
        let _ = sent.send((identity, root));
    });
    let budget = hydra::worktree::CONFIG_READ_TIMEOUT * 4;
    let answered = answer.recv_timeout(budget);
    if answered.is_err() {
        // Open the other end so the git still blocked on it can finish.
        let _ = std::fs::OpenOptions::new().write(true).open(&fifo);
    }

    let (identity, root) = answered.unwrap_or_else(|_| {
        panic!("identity and repo_root did not answer within {budget:?} with a fifo included")
    });
    assert!(
        identity.is_empty(),
        "a read that timed out still answered {identity:?}"
    );
    assert_eq!(root.as_deref(), Some(repo.as_path()));
    let _ = std::fs::remove_dir_all(&base);
}
