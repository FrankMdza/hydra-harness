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

/// What [`run`] printed and whether anything a launch needs was missing.
pub struct Report {
    pub text: String,
    pub missing: bool,
}

/// Prints the report to stdout through the caller. Returns exit 1 material as
/// [`Report::missing`].
pub fn run(layout: &Layout) -> Result<Report> {
    let mut missing = false;
    let mut out = String::new();
    let home = crate::paths::hydra_home();
    let policies = home.join("policies");

    path_line(&mut out, "root", layout.root(), &mut missing);
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

/// One protected config as the doctor reports it.
enum Config {
    /// A launch refuses without it.
    Launch(&'static str, PathBuf),
    /// A later command refuses without it, and a launch does not.
    Later(&'static str, PathBuf),
    /// Rust by design, never a file: `config/README.md` says which.
    InCode(&'static str, &'static str),
}

fn protected_configs(home: &Path) -> [Config; 6] {
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
    ]
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
