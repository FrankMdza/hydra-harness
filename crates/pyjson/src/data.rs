//! Where a checkout keeps the data files both ports read: `vendors.json`, `priors.json`,
//! `aliases.json` and `prompts/`.
//!
//! One rule for `hydra-puppets` and `hydra-grader` (build tickets 196, 221 and 220), so the
//! two never read different alias tables: `HYDRA_PUPPETS_DATA` when set, else
//! `integrations/puppets` under `HYDRA_HOME`, the variable `hydra` finds its own checkout
//! by, else the first directory above the binary that holds `integrations/puppets/prompts`,
//! which is the checkout for a binary under its `target/`. With none of those the path is
//! the one beside the binary, so the error that follows names where the files were looked
//! for. Nothing about the checkout a binary was built in is compiled in.

use std::path::{Path, PathBuf};

/// The data directory inside a checkout.
pub const DATA_DIR: &str = "integrations/puppets";

/// The directory the rule above names, whether or not it exists. `data` and `hydra_home` are
/// the two variables when they are set and not empty, and `exe` is the running binary.
#[must_use]
pub fn dir_from(data: Option<PathBuf>, hydra_home: Option<PathBuf>, exe: Option<&Path>) -> PathBuf {
    if let Some(data) = data {
        return data;
    }
    if let Some(home) = hydra_home {
        return home.join(DATA_DIR);
    }
    let exe = exe.map(|exe| exe.canonicalize().unwrap_or_else(|_| exe.to_path_buf()));
    let above = exe
        .as_deref()
        .map(Path::ancestors)
        .into_iter()
        .flatten()
        .skip(1);
    for dir in above {
        let found = dir.join(DATA_DIR);
        if found.join("prompts").is_dir() {
            return found;
        }
    }
    match exe.as_deref().and_then(Path::parent) {
        Some(beside) => beside.join(DATA_DIR),
        None => PathBuf::from(DATA_DIR),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_variable_wins_over_the_checkout_above_the_binary() {
        let dir = std::env::temp_dir().join(format!("hydra-pyjson-data-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let checkout = dir.join("checkout");
        std::fs::create_dir_all(checkout.join(DATA_DIR).join("prompts")).unwrap();
        std::fs::create_dir_all(checkout.join("target/release")).unwrap();
        std::fs::create_dir_all(dir.join("elsewhere/bin")).unwrap();
        let checkout = checkout.canonicalize().unwrap();
        let built = checkout.join("target/release/hydra-grader");
        let moved = dir
            .canonicalize()
            .unwrap()
            .join("elsewhere/bin/hydra-grader");
        let (data, home) = (PathBuf::from("/d"), PathBuf::from("/h"));

        assert_eq!(
            dir_from(Some(data), Some(home.clone()), Some(&built)),
            PathBuf::from("/d")
        );
        assert_eq!(
            dir_from(None, Some(home), Some(&built)),
            PathBuf::from("/h").join(DATA_DIR)
        );
        assert_eq!(dir_from(None, None, Some(&built)), checkout.join(DATA_DIR));
        assert_eq!(
            dir_from(None, None, Some(&moved)),
            moved.parent().unwrap().join(DATA_DIR)
        );
        assert_eq!(dir_from(None, None, None), PathBuf::from(DATA_DIR));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
