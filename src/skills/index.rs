//! The read-only index over the user-scope skill roots, harness spec 6.4.
//!
//! **The skills already on disk do not move.** Every provider reads a user-scope
//! root as well as a project one and `HOME` cannot be moved, so those skills are in
//! every run whatever Hydra decides. What Hydra can do is know they are there, and
//! this is that: rebuilt at every launch, never written to.
//!
//! **It reads leniently and the store's validator does not.** A user-scope skill is
//! the human's file, so a `SKILL.md` this index cannot parse is still an entry: the
//! directory name is what `claude` keys on and it is on disk whatever the
//! frontmatter says. [`crate::skills::frontmatter`] is strict because it guards what
//! Hydra writes, and this is strict about nothing because it guards nothing.
//!
//! **A skill with no `hydra.toml` beside it is one Hydra does not own** (6.4), which
//! makes it pinned and unproposable by definition. Nothing reads that yet; rung two
//! is the reader.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

/// One skill directory the index found.
#[derive(Debug, Clone)]
pub struct Entry {
    /// What `claude` keys on.
    pub dir_name: String,
    /// What `agy` keys on, when the frontmatter parses far enough to say.
    pub frontmatter_name: Option<String>,
    pub path: PathBuf,
    /// Which root it came from, so a per-provider check can filter.
    pub root: PathBuf,
}

impl Entry {
    /// Whether this entry answers to `name` under either provider's rule.
    pub fn answers_to(&self, name: &str) -> bool {
        self.dir_name == name || self.frontmatter_name.as_deref() == Some(name)
    }
}

/// Every skill under a set of roots.
#[derive(Debug, Clone, Default)]
pub struct Index {
    entries: Vec<Entry>,
}

impl Index {
    /// Scans the roots in order. A root that does not exist contributes nothing
    /// and is not an error: `~/.codex/skills` and `~/.grok/skills` appear the day
    /// the matching CLI is installed and are absent until then.
    pub fn scan(roots: &[PathBuf]) -> Result<Self> {
        Self::walk(roots, false)
    }

    /// The skills the checkout itself carries, which are the ones Hydra did not
    /// link.
    ///
    /// **Symlinks are skipped, and that is the whole of how the two are told
    /// apart.** A link in the injected root is Hydra's and a real directory is the
    /// repo's, which survives a worktree seeded from a parent that carries its own
    /// `.claude/skills/`. No list is kept, so nothing can fall out of step with
    /// the directory.
    pub fn project_scope(root: &Path) -> Result<Self> {
        Self::walk(std::slice::from_ref(&root.to_path_buf()), true)
    }

    /// The project scope without the entries Hydra itself put there. Skipping
    /// links was enough while every provider got a link; since build ticket 87
    /// `codex` gets a copy, which is a real directory, and without this the
    /// launch-time check read each copy as the checkout's own skill and refused
    /// every `codex` launch as a collision with itself.
    pub fn project_scope_without(root: &Path, injected: &[String]) -> Result<Self> {
        let mut index = Self::project_scope(root)?;
        index
            .entries
            .retain(|entry| !injected.iter().any(|name| entry.path == root.join(name)));
        Ok(index)
    }

    fn walk(roots: &[PathBuf], skip_links: bool) -> Result<Self> {
        let mut entries = Vec::new();
        for root in roots {
            let dir = match std::fs::read_dir(root) {
                Ok(dir) => dir,
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => continue,
                Err(err) => return Err(err).with_context(|| format!("reading {}", root.display())),
            };
            let mut found: Vec<PathBuf> = dir
                .flatten()
                .map(|entry| entry.path())
                .filter(|path| !(skip_links && path.is_symlink()))
                .filter(|path| path.join("SKILL.md").is_file())
                .collect();
            found.sort();
            for path in found {
                let Some(dir_name) = path.file_name().and_then(|n| n.to_str()) else {
                    continue;
                };
                entries.push(Entry {
                    dir_name: dir_name.to_string(),
                    frontmatter_name: declared_name(&path),
                    path: path.clone(),
                    root: root.clone(),
                });
            }
        }
        Ok(Self { entries })
    }

    /// The user-scope roots of every provider in the design, 6.4.
    ///
    /// All seven are scanned even though this build routes one provider. The spec
    /// asks the index to cover `~/.codex/skills` and `~/.grok/skills`, which
    /// `cursor-agent` reads at both scopes, and a `read_dir` on a directory that
    /// does not exist costs a syscall. The per-provider filter is
    /// [`Index::under`], and it is what keeps the check honest.
    pub fn user_scope(home: &Path) -> Result<Self> {
        Self::scan(&user_roots(home))
    }

    /// Every entry, whatever root it came from. For an index built over one root
    /// by [`Index::project_scope`], where filtering by root says nothing.
    pub fn all(&self) -> &[Entry] {
        &self.entries
    }

    /// The entries that came from one of `roots`.
    pub fn under(&self, roots: &[PathBuf]) -> Vec<&Entry> {
        self.entries
            .iter()
            .filter(|entry| roots.contains(&entry.root))
            .collect()
    }

    /// Every entry answering to `name`, under either provider's identity rule.
    pub fn holding<'a>(&'a self, name: &str, roots: &[PathBuf]) -> Vec<&'a Entry> {
        self.under(roots)
            .into_iter()
            .filter(|entry| entry.answers_to(name))
            .collect()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// The seven user-scope roots, in the order 6.4 and ticket 24 name them.
pub fn user_roots(home: &Path) -> Vec<PathBuf> {
    vec![
        // `claude`, and `cursor-agent` reads it too.
        home.join(".claude/skills"),
        // `cursor-agent`'s own two, plus the three vendor roots it also reads.
        home.join(".cursor/skills"),
        home.join(".cursor/skills-cursor"),
        home.join(".agents/skills"),
        home.join(".codex/skills"),
        home.join(".grok/skills"),
        // `agy`'s user root, and it reads no other.
        home.join(".gemini/config/skills"),
    ]
}

/// The frontmatter `name`, when the file has one this index can read.
///
/// Lenient on purpose: see the module note. A file that will not parse still has
/// a directory name, and that is the key `claude` uses.
fn declared_name(dir: &Path) -> Option<String> {
    let text = std::fs::read_to_string(dir.join("SKILL.md")).ok()?;
    let block = super::frontmatter::frontmatter(&text).ok()?;
    let docs = yaml_rust2::YamlLoader::load_from_str(block).ok()?;
    docs.first()?["name"].as_str().map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(root: &Path, name: &str, body: &str) {
        let dir = root.join(name);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("SKILL.md"), body).unwrap();
    }

    fn tempdir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("hydra-index-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn it_keys_on_the_directory_and_on_the_frontmatter() {
        let home = tempdir("both-keys");
        let root = home.join(".claude/skills");
        write(
            &root,
            "dirname-only",
            "---\nname: frontmatter-name\ndescription: A probe.\n---\n",
        );
        let index = Index::user_scope(&home).unwrap();
        let roots = vec![root.clone()];

        // Ticket 24's identity probe. One directory, two names, and the check has
        // to hit on either because the four providers disagree about which one
        // is the skill's name.
        assert_eq!(index.holding("dirname-only", &roots).len(), 1);
        assert_eq!(index.holding("frontmatter-name", &roots).len(), 1);
        assert!(index.holding("neither", &roots).is_empty());
        std::fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn a_skill_it_cannot_parse_is_still_an_entry() {
        let home = tempdir("unparseable");
        let root = home.join(".claude/skills");
        // The shape `agy` drops and Cursor empties. The human owns this file and
        // the index reports it rather than refusing it.
        write(
            &root,
            "colon",
            "---\nname: colon\ndescription: A probe. label: value.\n---\n",
        );
        let index = Index::user_scope(&home).unwrap();
        let entry = index.under(std::slice::from_ref(&root))[0];
        assert_eq!(entry.dir_name, "colon");
        assert!(entry.frontmatter_name.is_none(), "the parse failed");
        assert_eq!(index.holding("colon", &[root]).len(), 1);
        std::fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn a_root_that_does_not_exist_contributes_nothing() {
        let home = tempdir("absent-roots");
        let index = Index::user_scope(&home).unwrap();
        assert!(index.is_empty());
        std::fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn the_filter_keeps_one_provider_from_reading_another_s_root() {
        let home = tempdir("per-provider");
        write(
            &home.join(".grok/skills"),
            "only-grok",
            "---\nname: only-grok\ndescription: A probe.\n---\n",
        );
        let index = Index::user_scope(&home).unwrap();
        // Scanned, because 6.4 asks the index to cover it. Invisible to a
        // `claude` check, because `claude` does not read that root.
        assert_eq!(index.len(), 1);
        assert!(
            index
                .holding("only-grok", &[home.join(".claude/skills")])
                .is_empty()
        );
        std::fs::remove_dir_all(&home).ok();
    }
}
