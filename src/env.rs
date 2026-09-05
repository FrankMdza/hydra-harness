//! The environment allowlist, harness spec 6.2.
//!
//! Hydra builds each child's environment from a list rather than passing its own
//! through. `GEMINI_API_KEY` is exported on line 1 of `~/.zshrc`, so without this
//! every child inherits it and D6 is false while nobody notices. This shell also
//! exports `CLAUDE_CODE_MESSAGING_SOCKET` and its token, and an unscrubbed child
//! `claude` joins the parent's messaging plane.
//!
//! Measured on 2026-09-05: a `claude` 2.1.261 child launched under this allowlist
//! reported `messaging_socket_path` pointing at its own pid, not the parent's.

use std::collections::BTreeMap;
use std::ffi::CStr;
use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow, bail};

use crate::provider::Provider;

/// Passed to every provider.
const BASE: &[&str] = &["HOME", "USER", "PATH", "TERM"];

/// `claude` only. `SHELL` is read for shell classification, `TMPDIR` has its own
/// fallback if absent.
const CLAUDE_ONLY: &[&str] = &["SHELL", "TMPDIR"];

/// Passed through when the network needs them.
const PROXY: &[&str] = &[
    "HTTPS_PROXY",
    "HTTP_PROXY",
    "NO_PROXY",
    "https_proxy",
    "http_proxy",
    "no_proxy",
];

/// For the two Node providers.
const NODE_CA: &str = "NODE_EXTRA_CA_CERTS";

/// Three names Hydra must never set, checked rather than assumed.
///
/// `CLAUDE_CONFIG_DIR` loses the Anthropic login even when set to its own default
/// value, because the keychain service name gains a hash of the config dir the
/// moment the variable is defined. `AGY_ADC_AUTH` is the real auth switch for
/// Google. `EDITOR` stays off for `agy`, which spawns it from the artifact view,
/// so a detached child would hang on an editor nobody is watching.
pub const NEVER_SET: &[&str] = &["CLAUDE_CONFIG_DIR", "AGY_ADC_AUTH", "EDITOR"];

/// The environment a child gets, and the names the record reports.
pub struct ChildEnv {
    vars: BTreeMap<String, String>,
}

impl ChildEnv {
    /// Builds the child's environment from the parent's, by allowlist.
    ///
    /// `HOME` is the rule to get right. `security list-keychains` returns only the
    /// system keychain when `HOME` points anywhere but the real home, so every
    /// keychain-backed provider comes up unauthenticated with exit code 0. An
    /// absent `HOME` is safer than a wrong one, because the search list falls back
    /// to `getpwuid`, so a `HOME` that disagrees with the passwd entry is dropped
    /// rather than forwarded. The record shows the drop, because `env_allowlist`
    /// carries the names actually passed.
    pub fn build(provider: Provider) -> Result<Self> {
        let mut names: Vec<&str> = BASE.to_vec();
        if provider == Provider::Claude {
            names.extend_from_slice(CLAUDE_ONLY);
            names.push(NODE_CA);
        }
        names.extend_from_slice(PROXY);

        let mut vars = BTreeMap::new();
        for name in names {
            let Ok(value) = std::env::var(name) else {
                continue;
            };
            if value.is_empty() {
                continue;
            }
            if name == "HOME" && Path::new(&value) != real_home()? {
                continue;
            }
            vars.insert(name.to_string(), value);
        }

        match vars.get("PATH") {
            None => bail!("PATH is not set, and a provider cannot run without it"),
            Some(path) if !path.split(':').any(|p| p == "/usr/bin") => bail!(
                "PATH does not contain /usr/bin, which the keychain read needs. \
                 A provider launched under this PATH reports success while logged out."
            ),
            Some(_) => {}
        }

        Ok(Self { vars })
    }

    pub fn vars(&self) -> impl Iterator<Item = (&str, &str)> {
        self.vars.iter().map(|(k, v)| (k.as_str(), v.as_str()))
    }

    /// The names for `run_start.env_allowlist`. Contract 5.3: names, never values.
    pub fn names(&self) -> Vec<String> {
        self.vars.keys().cloned().collect()
    }
}

/// The home directory from the passwd entry, ignoring `HOME`.
pub fn real_home() -> Result<PathBuf> {
    // getpwuid is not reentrant. Hydra calls it from one thread during pre-flight.
    let dir = unsafe {
        let pw = libc::getpwuid(libc::getuid());
        if pw.is_null() {
            return Err(anyhow!(
                "getpwuid returned no entry for uid {}",
                libc::getuid()
            ));
        }
        CStr::from_ptr((*pw).pw_dir).to_str()?.to_string()
    };
    Ok(PathBuf::from(dir))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passes_nothing_outside_the_allowlist() {
        let built = ChildEnv::build(Provider::Claude).unwrap();
        let allowed: Vec<&str> = BASE
            .iter()
            .chain(CLAUDE_ONLY)
            .chain(PROXY)
            .chain(std::iter::once(&NODE_CA))
            .copied()
            .collect();
        for (name, _) in built.vars() {
            assert!(allowed.contains(&name), "{name} is not on the allowlist");
        }
    }

    #[test]
    fn never_sets_the_three_forbidden_names() {
        let built = ChildEnv::build(Provider::Claude).unwrap();
        for forbidden in NEVER_SET {
            assert!(!built.vars.contains_key(*forbidden), "{forbidden} was set");
        }
    }

    #[test]
    fn drops_the_secrets_this_shell_exports() {
        // The two the spec names by measurement: the Google key on line 1 of
        // ~/.zshrc, and the parent's messaging plane.
        let built = ChildEnv::build(Provider::Claude).unwrap();
        for leaked in [
            "GEMINI_API_KEY",
            "CLAUDE_CODE_MESSAGING_SOCKET",
            "CLAUDE_CODE_MESSAGING_TOKEN",
            "CLAUDE_CODE_SESSION_ID",
            "CURSOR_AUTH_TOKEN",
            "CURSOR_API_KEY",
            "NODE_OPTIONS",
            "ANTHROPIC_API_KEY",
        ] {
            assert!(
                !built.vars.contains_key(leaked),
                "{leaked} reached the child"
            );
        }
    }

    #[test]
    fn home_is_the_real_home_or_absent() {
        let built = ChildEnv::build(Provider::Claude).unwrap();
        if let Some(home) = built.vars.get("HOME") {
            assert_eq!(PathBuf::from(home), real_home().unwrap());
        }
    }
}
