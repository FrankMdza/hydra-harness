//! Where `aliases.json` and the other data files are read from, for every command of this
//! binary (build ticket 220 made it one rule; three commands each had a copy of an older
//! one).
//!
//! The directory is the one `hydra-puppets` reads, by the rule in `hydra_pyjson::data`:
//! `HYDRA_PUPPETS_DATA` when set, else `integrations/puppets` under `HYDRA_HOME`, else the
//! first directory above this binary that holds `integrations/puppets/prompts`. The Python
//! read the file beside its own checkout and ignored the environment. A binary has no
//! checkout of its own, so the launchd job names the directory.
//!
//! A directory that is not there, or one with no `aliases.json` in it, is an error that
//! names the directory and the two variables. It is never an empty table: a pass with no
//! aliases would write another `model` into the dataset and say nothing.

use std::path::{Path, PathBuf};

use anyhow::{Result, bail};
use hydra_pyjson::models::{self, Pair};

use crate::labels::Env;

const HOW: &str = "set HYDRA_PUPPETS_DATA to the directory that holds aliases.json, or HYDRA_HOME to the checkout";

fn dir_from(env: &Env, exe: Option<&Path>) -> Result<PathBuf> {
    let named = |name: &str| env.named(name).map(PathBuf::from);
    let dir = hydra_pyjson::data::dir_from(named("HYDRA_PUPPETS_DATA"), named("HYDRA_HOME"), exe);
    if !dir.is_dir() {
        bail!(
            "FileNotFoundError: no data directory at {}; {HOW}",
            dir.display()
        );
    }
    Ok(dir)
}

/// The data directory, which exists.
pub fn dir(env: &Env) -> Result<PathBuf> {
    dir_from(env, std::env::current_exe().ok().as_deref())
}

pub(crate) fn aliases_in(dir: &Path) -> Result<Vec<(Pair, Pair)>> {
    let file = dir.join("aliases.json");
    if !file.is_file() {
        bail!(
            "FileNotFoundError: no aliases.json in {}; {HOW}",
            dir.display()
        );
    }
    models::load_aliases(&file)
}

/// `records.load_aliases()`: the alias table of the data directory.
pub fn aliases(env: &Env) -> Result<Vec<(Pair, Pair)>> {
    aliases_in(&dir(env)?)
}

/// [`aliases`] in the environment of this process.
pub(crate) fn process_aliases() -> Result<Vec<(Pair, Pair)>> {
    aliases(&Env::process())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("hydra-grader-data-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir.canonicalize().unwrap()
    }

    #[test]
    fn a_directory_with_no_alias_table_is_an_error_that_names_it() {
        let dir = scratch("empty");
        let env = Env::from_pairs([("HYDRA_PUPPETS_DATA", dir.as_os_str())]);
        let found = dir_from(&env, None).unwrap();
        assert_eq!(found, dir);
        let said = aliases_in(&found).unwrap_err().to_string();
        for part in [
            "FileNotFoundError: ",
            dir.to_str().unwrap(),
            "HYDRA_PUPPETS_DATA",
            "HYDRA_HOME",
        ] {
            assert!(said.contains(part), "{said}");
        }
        std::fs::write(dir.join("aliases.json"), "[]").unwrap();
        assert!(aliases_in(&found).unwrap().is_empty());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_binary_outside_a_checkout_with_neither_variable_has_no_data_directory() {
        let dir = scratch("moved");
        let exe = dir.join("bin/hydra-grader");
        let said = dir_from(&Env::default(), Some(&exe))
            .unwrap_err()
            .to_string();
        let looked = dir.join("bin/integrations/puppets");
        for part in [looked.to_str().unwrap(), "HYDRA_PUPPETS_DATA", "HYDRA_HOME"] {
            assert!(said.contains(part), "{said}");
        }
        // The variable wins, and an empty one is not set.
        let env = Env::from_pairs([
            ("HYDRA_PUPPETS_DATA", dir.as_os_str()),
            ("HYDRA_HOME", std::ffi::OsStr::new("/nowhere")),
        ]);
        assert_eq!(dir_from(&env, Some(&exe)).unwrap(), dir);
        let env = Env::from_pairs([("HYDRA_PUPPETS_DATA", ""), ("HYDRA_HOME", "/nowhere")]);
        let said = dir_from(&env, Some(&exe)).unwrap_err().to_string();
        assert!(said.contains("/nowhere/integrations/puppets"), "{said}");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
