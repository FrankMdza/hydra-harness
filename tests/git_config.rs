//! Build ticket 99, the third hiring panel's H5: git that Hydra runs outside the
//! seatbelt reads no global or system configuration, and still commits under
//! the identity that configuration names.
//!
//! **One test in its own file, because it sets process-wide variables.**
//! `GIT_CONFIG_GLOBAL` and `GIT_CONFIG_SYSTEM` stand in for a `~/.gitconfig` a
//! run rewrote and a system file the same user owns, and no other test shares
//! this binary to read them by accident.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Git for the test's own setup, reading neither of the planted files.
fn clean_git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .args(args)
        .current_dir(dir)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
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
    let base: PathBuf =
        std::env::temp_dir().join(format!("hydra-git-config-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
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
    // SAFETY: the only test in this binary, so no other thread reads the
    // environment while it changes.
    unsafe {
        std::env::set_var("GIT_CONFIG_GLOBAL", &global);
        std::env::set_var("GIT_CONFIG_SYSTEM", &system);
        std::env::remove_var("GIT_CONFIG_NOSYSTEM");
    }

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

    // SAFETY: as above.
    unsafe {
        std::env::remove_var("GIT_CONFIG_GLOBAL");
        std::env::remove_var("GIT_CONFIG_SYSTEM");
    }
    let _ = std::fs::remove_dir_all(&base);
}
