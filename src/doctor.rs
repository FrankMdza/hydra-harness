//! `hydra doctor`, the twelfth command.
//!
//! Reports what a launch would find: the data root, the policy directory, the four
//! vendor binaries, the policy set sha, and which protected configs are present.
//! It launches no provider, runs no probe, and writes no run record.

use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::paths::Layout;
use crate::policy::PolicySet;
use crate::provider::Provider;
use crate::walk;

/// What [`run`] printed and whether anything a launch needs was missing.
pub struct Report {
    pub text: String,
    pub missing: bool,
}

/// Walks the layout's `runs/` directory and prints one line per finding.
pub fn run_walk(layout: &Layout) -> Result<walk::Report> {
    walk::walk_corpus(layout)
}

/// Prints the report to stdout through the caller. Returns exit 1 material as
/// [`Report::missing`].
pub fn run(layout: &Layout) -> Result<Report> {
    let mut missing = false;
    let mut out = String::new();
    let home = crate::paths::hydra_home();
    let policies = home.join("policies");

    path_line(&mut out, "root", layout.root(), &mut missing);
    ascii_root_line(&mut out, "data root", layout.root(), &mut missing);
    ascii_root_line(&mut out, "checkout", &home, &mut missing);
    path_line(&mut out, "policies", &policies, &mut missing);

    for provider in [
        Provider::Claude,
        Provider::CursorAgent,
        Provider::Agy,
        Provider::Codex,
    ] {
        let measured = provider.measured_version();
        match provider.version() {
            Ok(installed) => {
                out.push_str(&format!(
                    "{} {} measured {}\n",
                    provider.binary(),
                    installed,
                    measured
                ));
            }
            Err(_) => {
                missing = true;
                out.push_str(&format!(
                    "{} missing measured {}\n",
                    provider.binary(),
                    measured
                ));
            }
        }
    }

    out.push_str(&path_hydra_line());

    if policies.is_dir() {
        let set = PolicySet::load(&policies)?;
        out.push_str(&format!("policy_sha {}\n", set.sha()));
    } else {
        missing = true;
        out.push_str("policy_sha missing\n");
    }

    for config in protected_configs(&home) {
        match config {
            Config::Launch(label, path) => path_line(&mut out, label, &path, &mut missing),
            // Exit 1 is for what a launch needs. The export allowlist is what
            // `hydra export` needs, and it says so itself when it is absent.
            Config::Later(label, path) => {
                out.push_str(&format!("{label} {} {}\n", path.display(), presence(&path)));
            }
            Config::InCode(label, module) => {
                out.push_str(&format!("{label} in code, {module}\n"));
            }
        }
    }

    Ok(Report { text: out, missing })
}

/// Where `hydra` on the human's `PATH` points. A report, never a refusal: a
/// run writes its own path into the face's config and needs no `PATH` entry.
fn path_hydra_line() -> String {
    let own = std::env::current_exe()
        .and_then(|exe| exe.canonicalize())
        .ok();
    let found = std::env::var_os("PATH").and_then(|paths| {
        std::env::split_paths(&paths)
            .map(|dir| dir.join("hydra"))
            .find(|candidate| candidate.is_file())
    });
    match found {
        None => "hydra on PATH none, see README Install\n".to_string(),
        Some(candidate) => match candidate.canonicalize() {
            Ok(resolved) if Some(&resolved) == own.as_ref() => {
                format!("hydra on PATH {} this binary\n", candidate.display())
            }
            Ok(resolved) => format!(
                "hydra on PATH {} another binary, {}\n",
                candidate.display(),
                resolved.display()
            ),
            Err(_) => format!("hydra on PATH {} unreadable\n", candidate.display()),
        },
    }
}

/// One protected config as the doctor reports it.
enum Config {
    /// A launch refuses without it.
    Launch(&'static str, PathBuf),
    /// A later command refuses without it, and a launch does not.
    Later(&'static str, PathBuf),
    /// Rust by design, never a file: `config/README.md` says which.
    InCode(&'static str, &'static str),
}

fn protected_configs(home: &Path) -> [Config; 7] {
    let config = home.join("config");
    [
        Config::Launch("delegation policy set", home.join("policies")),
        Config::InCode("environment allowlist", "src/env.rs"),
        Config::Later("export allowlist", config.join("export-allowlist.json")),
        Config::Later(
            "export sign-off manifest",
            config.join("export-signoff.json"),
        ),
        Config::Launch("instruction preamble", config.join("preamble.md")),
        Config::Launch("canonical MCP document", config.join("mcp.md")),
        // A missing file is no approved shapes, and every request for seats is
        // then refused, so a launch does not need it.
        Config::Later("approved panel shapes", config.join("panels.json")),
    ]
}

/// Map ticket 68: judged on the resolved path, the form the gate compiles.
/// See [`crate::paths::refuse_non_ascii_roots`] for the path it leaves out.
fn ascii_root_line(out: &mut String, label: &str, path: &Path, missing: &mut bool) {
    let state = if crate::paths::path_is_ascii(path) {
        "ascii"
    } else {
        *missing = true;
        "non-ascii"
    };
    out.push_str(&format!("{label} {} {state}\n", path.display()));
}

fn path_line(out: &mut String, label: &str, path: &Path, missing: &mut bool) {
    let state = presence(path);
    if state == "missing" {
        *missing = true;
    }
    out.push_str(&format!("{label} {} {state}\n", path.display()));
}

fn presence(path: &Path) -> &'static str {
    if path.is_dir() || path.is_file() {
        "present"
    } else {
        "missing"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_ascii_root_line_passes_and_a_non_ascii_one_fails() {
        let mut out = String::new();
        let mut missing = false;
        ascii_root_line(&mut out, "data root", Path::new("/tmp/hydra"), &mut missing);
        assert_eq!(out, "data root /tmp/hydra ascii\n");
        assert!(!missing);

        let mut out = String::new();
        let mut missing = false;
        ascii_root_line(&mut out, "checkout", Path::new("/tmp/Café"), &mut missing);
        assert_eq!(out, "checkout /tmp/Café non-ascii\n");
        assert!(missing);
    }
}
