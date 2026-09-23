//! The routable providers.
//!
//! Four in the design and four in this build: `claude`, `cursor-agent`, `agy`,
//! `codex`.
//! Every provider-specific fact is a property of the binary rather than of the
//! model, so a model is a flag inside a provider and never a provider of its own.

pub mod agy;
pub mod claude;
pub mod codex;
pub mod cursor;

use std::fmt;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use anyhow::{Context, Result, bail};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Provider {
    Claude,
    CursorAgent,
    Agy,
    Codex,
}

/// Versions this code was measured against. `README.md` and `tests/drift.rs` read
/// this table rather than carrying their own copy.
pub const MEASURED_VERSIONS: [(Provider, &str); 4] = [
    (Provider::Claude, "2.1.268"),
    (Provider::CursorAgent, "2026.09.18-9a7762b"),
    (Provider::Agy, "1.2.8"),
    (Provider::Codex, "0.154.0"),
];

impl Provider {
    pub fn binary(self) -> &'static str {
        match self {
            Provider::Claude => "claude",
            Provider::CursorAgent => "cursor-agent",
            Provider::Agy => "agy",
            Provider::Codex => "codex",
        }
    }

    /// The version string in [`MEASURED_VERSIONS`] for this provider.
    pub fn measured_version(self) -> &'static str {
        MEASURED_VERSIONS
            .iter()
            .find(|(provider, _)| *provider == self)
            .map(|(_, version)| *version)
            .unwrap_or("unknown")
    }

    /// The response shape the gate answers this provider in (6.6).
    pub fn dialect(self) -> crate::policy::Dialect {
        match self {
            Provider::Claude => crate::policy::Dialect::Claude,
            Provider::CursorAgent => crate::policy::Dialect::Cursor,
            Provider::Agy => crate::policy::Dialect::Agy,
            Provider::Codex => crate::policy::Dialect::Claude,
        }
    }

    /// The string the binary printed, read at launch.
    ///
    /// Contract 5.1 keeps this on every `run_start` because two of the three
    /// providers self-update without asking, and a claim measured on one build must
    /// not silently become a claim about another.
    pub fn version(self) -> Result<String> {
        // The one vendor subprocess that used to inherit the launcher's
        // environment whole, `GEMINI_API_KEY` and the parent's messaging
        // socket included. It gets the same allowlist the run gets.
        let env = crate::env::ChildEnv::build(self)?;
        let out = std::process::Command::new(self.binary())
            .arg("--version")
            .env_clear()
            .envs(env.vars().map(|(k, v)| (k.to_string(), v.to_string())))
            .output();
        match out {
            Ok(out) if out.status.success() => {
                Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
            }
            Ok(out) => bail!(
                "{} --version exited {}: {}",
                self.binary(),
                out.status,
                String::from_utf8_lossy(&out.stderr).trim()
            ),
            Err(err) => bail!("{} is not on PATH: {err}", self.binary()),
        }
    }

    /// Where Hydra links this run's skills, relative to the worktree (6.4).
    ///
    /// `cursor-agent` reads its row from cwd or `--workspace`, and Hydra runs it
    /// with cwd set to the worktree. `agy` reads its row from neither: the
    /// worktree has to arrive as `--add-dir` or the directory is not its
    /// workspace and nothing in it loads.
    pub fn skill_root(self) -> &'static str {
        match self {
            Provider::Claude => ".claude/skills",
            Provider::CursorAgent => ".cursor/skills",
            Provider::Agy => ".agents/skills",
            Provider::Codex => ".codex/skills",
        }
    }

    /// Whether this provider reads injected skills only through shell commands.
    ///
    /// `codex` has no read tool, so skills have to be plain files in the
    /// worktree rather than links into the snapshot under `~/hydra`.
    pub fn reads_skills_by_shell(self) -> bool {
        matches!(self, Provider::Codex)
    }

    /// The user-scope roots this provider reads, which the collision check
    /// compares the store against.
    ///
    /// **`claude` reads one, `agy` reads one and `cursor-agent` reads six**, so
    /// this is a per-provider list rather than the index's own. Ticket 24 fired
    /// probes into all of them at both scopes; `crate::skills::index` scans the
    /// union and this is the filter over it.
    pub fn user_skill_roots(self, home: &std::path::Path) -> Vec<std::path::PathBuf> {
        match self {
            Provider::Claude => vec![home.join(".claude/skills")],
            // Its own two, `claude`'s, and the three vendor roots it reads as
            // well. `~/.cursor/skills-cursor` held twenty-six of Cursor's own
            // skills on this machine on 2026-09-06, so the collision check has
            // something to compare against on this provider from the first
            // launch.
            Provider::CursorAgent => vec![
                home.join(".claude/skills"),
                home.join(".cursor/skills"),
                home.join(".cursor/skills-cursor"),
                home.join(".agents/skills"),
                home.join(".codex/skills"),
                home.join(".grok/skills"),
            ],
            // One root, and its own. The builtin skills under
            // `~/.gemini/antigravity-cli/builtin/skills` and the plugin ones
            // under `~/.gemini/config/plugins/*/skills` are in the free listing
            // too, and neither is a directory a human drops a skill into, so
            // neither is compared against the store here. The provider's own
            // answer is the test that covers them: a name Hydra linked that the
            // listing does not carry is a collision whichever side won.
            Provider::Agy => vec![home.join(".gemini/config/skills")],
            Provider::Codex => vec![home.join(".codex/skills")],
        }
    }

    /// What this provider says it resolved: contract 5.3's `skills.resolved` and
    /// `mcp.resolved`, from one probe.
    ///
    /// **One question and not two, because it is one process.** The skill listing
    /// and the MCP tool list arrive in the same transcript, so asking twice would
    /// pay twice for an answer that is already there.
    ///
    /// Free on `claude` and on `agy`, paid on `cursor-agent`. See
    /// [`claude::resolve`] for what free costs in wall-clock time.
    pub fn resolve(
        self,
        worktree: &std::path::Path,
        mcp: crate::mcp::Probe<'_>,
    ) -> Result<Resolved> {
        match self {
            Provider::Claude => claude::resolve(worktree, mcp),
            Provider::CursorAgent => cursor::resolve(worktree, mcp),
            Provider::Agy => agy::resolve(worktree, mcp),
            Provider::Codex => codex::resolve(worktree, mcp),
        }
    }

    /// Which subscription pays for this binary's runs.
    ///
    /// Hydra authors this at spawn. `agy` serves Claude models on the Google
    /// subscription, so `provider` stopped answering which vendor's model replied
    /// and the two questions came apart. Hydra knows which binary it spawned and
    /// which subscription that binary is logged into, which is the only version
    /// that answers identically on all three.
    pub fn subscription(self) -> &'static str {
        match self {
            Provider::Claude => "anthropic",
            Provider::CursorAgent => "cursor",
            Provider::Agy => "google",
            Provider::Codex => "openai",
        }
    }

    /// Whether harness spec 17's resume command can continue this provider's
    /// sessions. Every routed provider in v1 can; there is no third branch.
    pub fn supports_resume(self) -> bool {
        true
    }

    /// What the provider reported about this frame that the envelope needs.
    pub fn observe(self, raw: &str) -> Observed {
        match self {
            Provider::Claude => claude::observe(raw),
            Provider::CursorAgent => cursor::observe(raw),
            Provider::Agy => agy::observe(raw),
            Provider::Codex => codex::observe(raw),
        }
    }
}

/// The relative paths Hydra writes into a checkout that `hydra reap` must not
/// stage.
///
/// Built from each provider's `gitignore` so a file added there is excluded
/// here without a second list. The dummy worktree is empty, which makes the
/// paths `gitignore` returns relative.
pub fn own_worktree_paths() -> Vec<&'static str> {
    static PATHS: std::sync::OnceLock<Vec<&'static str>> = std::sync::OnceLock::new();
    PATHS
        .get_or_init(|| {
            [cursor::gitignore, agy::gitignore, codex::gitignore]
                .into_iter()
                .flat_map(paths_listed_by)
                .collect()
        })
        .clone()
}

fn paths_listed_by(gitignore: fn(&Path) -> (PathBuf, &'static str)) -> Vec<&'static str> {
    let (ignore_path, body) = gitignore(Path::new(""));
    let dir = match ignore_path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent.to_path_buf(),
        _ => PathBuf::new(),
    };
    body.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| {
            let name = line.trim_start_matches('/');
            let relative = dir
                .join(name)
                .components()
                .map(|c| c.as_os_str().to_string_lossy())
                .collect::<Vec<_>>()
                .join("/");
            Box::leak(relative.into_boxed_str()) as &'static str
        })
        .collect()
}

/// The path one of Hydra's per-run files will be written to inside the checkout,
/// once the checkout has been checked for the human's own copy and the ignore
/// file is in place.
///
/// **A file the checkout already holds is refused rather than overwritten.** The
/// worktree is seeded from the human's repo (6.3), so a hook or MCP config in it
/// is theirs: writing over it edits their work and loading theirs instead of
/// Hydra's launches an ungated run. [`crate::run`] checks the parent repo for
/// the same files before a run id exists; this is the second line, on the
/// checkout itself.
///
/// **An existing `.gitignore` is left alone**, the same rule the skill root
/// follows: a repo that tracks its own would see the overwrite as an edit to the
/// human's checkout.
///
/// **And when that happens the human is told, because they will see the file.**
/// `hydra reap` excludes Hydra's own paths from what it stages, so the file
/// does not land on `hydra/<run-id>`. It still sits untracked in the checkout
/// while the run is live. Refusing the launch was the alternative and it is too
/// harsh for a repo that merely has a `.gitignore` of its own, so this warns
/// and names the file.
///
/// Two providers reach this and the parts that differ are arguments: the
/// directory the files live in, and which names the ignore file lists.
pub(crate) fn claim(
    worktree: Option<&Path>,
    binary: &str,
    what: &str,
    file: impl Fn(&Path) -> PathBuf,
    ignore: impl Fn(&Path) -> (PathBuf, &'static str),
) -> Result<PathBuf> {
    let worktree = worktree.with_context(|| {
        format!("{binary} reads its {what} out of the worktree and this run has none")
    })?;
    let file = file(worktree);
    if file.exists() {
        bail!(
            "{} already exists in the checkout, and Hydra will not write over a file the \
             human's repo carries",
            file.display()
        );
    }
    if let Some(parent) = file.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    let (ignore, body) = ignore(worktree);
    match std::fs::read_to_string(&ignore) {
        Ok(existing) if !covers(&existing, &file) => eprintln!(
            "hydra: {} is the checkout's own and does not ignore {}, so that file stays \
             untracked in the checkout while the run is live. `hydra reap` will not commit \
             Hydra's {what}. Add /{} to that file if you want git to ignore it.",
            ignore.display(),
            file.display(),
            basename(&file),
        ),
        Ok(_) => {}
        Err(_) => {
            std::fs::write(&ignore, body)
                .with_context(|| format!("writing {}", ignore.display()))?;
        }
    }
    Ok(file)
}

fn basename(file: &Path) -> String {
    file.file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_default()
}

/// Whether an ignore file already names the file Hydra is about to write.
///
/// A line match rather than a substring one, so `hooks.json` inside a comment or
/// inside a longer path does not read as coverage. `*` counts, because a
/// directory ignored whole is a directory reap will not stage from.
fn covers(existing: &str, file: &Path) -> bool {
    let name = basename(file);
    existing
        .lines()
        .map(str::trim)
        .any(|line| line == "*" || line.trim_start_matches('/') == name.trim_start_matches('/'))
}

/// What one free pre-flight probe found, for contract 5.3's two `resolved`
/// fields.
///
/// Each half is `None` when the probe could not answer it. Contract 5.3 already
/// requires a reader to treat that as unanswered rather than as an empty listing,
/// and the two halves fail independently: a transcript can carry a tool list and
/// no skill listing.
#[derive(Default)]
pub struct Resolved {
    pub skills: Option<Vec<String>>,
    pub mcp: Option<Vec<String>>,
    /// Servers the provider named as failed, each as `name: reason`. Not a
    /// refusal: 6.5 gives the fatal pre-flight to Cursor alone and says why, so
    /// this reaches stderr and the record shows the server as injected and not
    /// resolved.
    pub failed_mcp: Vec<String>,
}

/// The two fields a frame can teach the envelope. Contract 4 and 5.1 both backfill
/// in memory rather than rewriting line 0.
#[derive(Default)]
pub struct Observed {
    pub model: Option<String>,
    pub session_id: Option<String>,
}

impl fmt::Display for Provider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.binary())
    }
}

impl FromStr for Provider {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "claude" => Ok(Provider::Claude),
            "cursor-agent" => Ok(Provider::CursorAgent),
            "agy" => Ok(Provider::Agy),
            "codex" => Ok(Provider::Codex),
            // Ticket 22 unrouted it. Its guardrail rules are still copied into
            // the pattern floor and its binary is not launched.
            other => bail!("{other} is not a routable provider"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_ignore_file_the_checkout_owns_is_read_rather_than_replaced() {
        // `hydra reap` excludes Hydra's own paths, so a file that is neither
        // tracked nor ignored stays untracked in the live checkout rather than
        // landing on the run's branch. An existing ignore file that already
        // names it is fine; one that does not gets a warning, and either way
        // the human's file is untouched.
        let file = std::path::Path::new("/wt/.agents/hooks.json");
        assert!(covers("/.gitignore\n/hooks.json\n/mcp_config.json\n", file));
        assert!(covers("*\n", file));
        assert!(covers("hooks.json", file));
        // A name inside a longer path or a comment is not coverage.
        assert!(!covers("/.agents/hooks.json\n", file));
        assert!(!covers("# hooks.json\n", file));
        assert!(!covers("/rules\n/plugins\n", file));
        assert!(!covers("", file));
    }

    #[test]
    fn own_worktree_paths_are_the_names_each_gitignore_lists() {
        assert_eq!(
            own_worktree_paths(),
            vec![
                ".cursor/.gitignore",
                ".cursor/hooks.json",
                ".cursor/mcp.json",
                ".agents/.gitignore",
                ".agents/hooks.json",
                ".agents/mcp_config.json",
                ".codex/.gitignore",
                ".codex/hooks.json",
                ".codex/config.toml",
            ]
        );
    }

    #[test]
    fn agy_reads_one_user_scope_root_and_injects_into_the_agents_directory() {
        // 6.4's third row, and the flag that makes it visible is on the argv
        // rather than here. Ticket 24 measured the root; the builtin and plugin
        // roots in the free listing are not directories a human drops a skill
        // into, so the collision check does not compare the store against them.
        let home = std::path::Path::new("/Users/probe");
        assert_eq!(
            Provider::Agy.user_skill_roots(home),
            vec![home.join(".gemini/config/skills")]
        );
        assert_eq!(Provider::Agy.skill_root(), ".agents/skills");
        assert_eq!(Provider::Agy.subscription(), "google");
        assert_eq!(Provider::Agy.dialect(), crate::policy::Dialect::Agy);
    }

    #[test]
    fn cursor_reads_six_user_scope_roots_and_claude_reads_one() {
        // 6.4 and ticket 24. `~/.cursor/skills-cursor` held twenty-six of
        // Cursor's own skills on this machine on 2026-09-06, so the collision
        // check on that provider has names to compare a store against from the
        // first launch.
        let home = std::path::Path::new("/Users/probe");
        let cursor = Provider::CursorAgent.user_skill_roots(home);
        assert_eq!(cursor.len(), 6);
        for root in [
            ".claude/skills",
            ".cursor/skills",
            ".cursor/skills-cursor",
            ".agents/skills",
            ".codex/skills",
            ".grok/skills",
        ] {
            assert!(cursor.contains(&home.join(root)), "{root}");
        }
        assert_eq!(
            Provider::Claude.user_skill_roots(home),
            vec![home.join(".claude/skills")]
        );
        assert_eq!(Provider::CursorAgent.skill_root(), ".cursor/skills");
    }

    #[test]
    fn the_three_routed_names_parse_and_the_unrouted_one_does_not() {
        assert_eq!("claude".parse::<Provider>().unwrap(), Provider::Claude);
        assert_eq!(
            "cursor-agent".parse::<Provider>().unwrap(),
            Provider::CursorAgent
        );
        assert_eq!("agy".parse::<Provider>().unwrap(), Provider::Agy);
        assert_eq!("codex".parse::<Provider>().unwrap(), Provider::Codex);
        assert!("hermes".parse::<Provider>().is_err());
    }

    #[test]
    fn codex_reads_one_user_scope_root_and_answers_hooks_in_claudes_dialect() {
        let home = std::path::Path::new("/Users/probe");
        assert_eq!(
            Provider::Codex.user_skill_roots(home),
            vec![home.join(".codex/skills")]
        );
        assert_eq!(Provider::Codex.skill_root(), ".codex/skills");
        assert_eq!(Provider::Codex.subscription(), "openai");
        assert_eq!(Provider::Codex.dialect(), crate::policy::Dialect::Claude);
    }
}
