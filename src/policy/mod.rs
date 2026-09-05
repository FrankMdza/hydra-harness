//! The delegation policy, harness spec 7.
//!
//! Five named allowlists in `policies/`, a pattern floor beside them, a
//! capability table per provider, and a compiler that turns a policy name into
//! the gate script of 7.5. No provider is launched from here. The compiler's
//! output is a file, and `hydra::run::write_spawn_config` is what puts it in
//! front of a running `claude`.
//!
//! The same compiler has a second reader. [`replay`] runs a finished run's
//! policy again over the calls the record kept, which is how contract 7.7
//! derives `permission_decision` without a second implementation of the rules.
//!
//! The policy directory is protected config #1 (15.1) and [`PolicySet::sha`] is
//! what every run record carries as `policy_sha`, so an edit under `policies/`
//! is visible in every record written after it.

pub mod replay;
pub mod table;

use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use anyhow::{Context, Result, bail};
use serde::Deserialize;
use sha2::{Digest, Sha256};

/// What a policy grants. Capabilities rather than provider tool names, because
/// the three providers share no vocabulary (7.1).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    FileRead,
    FileWrite,
    Shell,
    NetworkRead,
    McpCall,
    SubagentSpawn,
    /// The tools that exist only inside one provider's own product surface and
    /// that no policy in 7.2 grants. See `table.rs` for why this row exists.
    ProviderNative,
}

impl Capability {
    /// Whether a scope means anything for this capability.
    ///
    /// `shell` is deliberately unscoped. A shell command's containment comes
    /// from the child's cwd and from the pattern floor, and 7.4 opens by saying
    /// a capability allowlist is not enough on its own because shell swallows
    /// everything. A scope here would read as a guarantee the gate cannot keep.
    pub fn is_scoped(self) -> bool {
        matches!(self, Capability::FileRead | Capability::FileWrite)
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Capability::FileRead => "file_read",
            Capability::FileWrite => "file_write",
            Capability::Shell => "shell",
            Capability::NetworkRead => "network_read",
            Capability::McpCall => "mcp_call",
            Capability::SubagentSpawn => "subagent_spawn",
            Capability::ProviderNative => "provider_native",
        }
    }
}

/// Where a scoped capability may act.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Scope {
    /// The run's own checkout.
    Worktree,
    /// `~/hydra/skills/`. `curate` writes here, outside its own worktree.
    SkillStore,
    /// `~/hydra/runs/`, read only in practice: 7.5 denies writes to it under
    /// every policy, because a loop that learns from the corpus must not be
    /// able to edit the corpus.
    Corpus,
    Anywhere,
}

impl Scope {
    pub fn as_str(self) -> &'static str {
        match self {
            Scope::Worktree => "worktree",
            Scope::SkillStore => "skill_store",
            Scope::Corpus => "corpus",
            Scope::Anywhere => "anywhere",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Deserialize)]
pub struct Allow {
    pub capability: Capability,
    pub scope: Option<Scope>,
}

/// One of the five. Deny by default, and there is no `denied` list: with deny by
/// default it could only subtract from the allow list, and a policy where two
/// rules can disagree is a bug with a schema (7.1).
#[derive(Clone, Debug, Deserialize)]
pub struct Policy {
    pub name: String,
    #[serde(default)]
    pub allows: String,
    pub allow: Vec<Allow>,
}

impl Policy {
    fn validate(&self, file: &Path) -> Result<()> {
        for entry in &self.allow {
            match (entry.capability.is_scoped(), entry.scope) {
                (true, None) => bail!(
                    "{}: {} is scoped and this entry names no scope",
                    file.display(),
                    entry.capability.as_str()
                ),
                (false, Some(scope)) => bail!(
                    "{}: {} takes no scope and this entry names {}",
                    file.display(),
                    entry.capability.as_str(),
                    scope.as_str()
                ),
                _ => {}
            }
        }
        Ok(())
    }

    /// The set this policy grants, as capability-scope pairs.
    fn grants(&self) -> std::collections::BTreeSet<Allow> {
        self.allow.iter().copied().collect()
    }

    /// True when `self` grants everything `other` does.
    ///
    /// 7.3's containment, over the five. It is a partial order rather than a
    /// chain: `curate` writes to the store and cannot touch a worktree,
    /// `standard` is the reverse, and neither contains the other. An
    /// incomparable pair is refused rather than resolved, which is delegation's
    /// business (ticket 11) and not this module's.
    pub fn contains(&self, other: &Policy) -> bool {
        let mine = self.grants();
        other.grants().iter().all(|entry| {
            mine.contains(entry)
                || (entry.capability.is_scoped()
                    && mine.contains(&Allow {
                        capability: entry.capability,
                        scope: Some(Scope::Anywhere),
                    }))
        })
    }
}

/// The five names. The caller passes one of these and never a list: a name is
/// something a human wrote once, and a list is security composed at call time by
/// the least accountable party in the chain (7.2).
pub const NAMES: [&str; 5] = ["judge", "curate", "read-only", "standard", "permissive"];

/// The policy directory, loaded and hashed.
pub struct PolicySet {
    dir: PathBuf,
    policies: BTreeMap<String, Policy>,
    floor: serde_json::Value,
    sha: String,
}

impl PolicySet {
    /// The set shipped in this repo.
    ///
    /// `CARGO_MANIFEST_DIR` is the dev-mode answer and the only one this slice
    /// needs, because the binary runs from the source tree. An installed `hydra`
    /// has to find its policy directory some other way, and that is a decision
    /// nothing has needed yet.
    pub fn discover() -> Result<Self> {
        Self::load(Path::new(env!("CARGO_MANIFEST_DIR")).join("policies"))
    }

    pub fn load(dir: impl Into<PathBuf>) -> Result<Self> {
        let dir = dir.into();
        let mut policies = BTreeMap::new();
        for name in NAMES {
            let file = dir.join(format!("{name}.json"));
            let text = std::fs::read_to_string(&file)
                .with_context(|| format!("reading {}", file.display()))?;
            let policy: Policy = serde_json::from_str(&text)
                .with_context(|| format!("parsing {}", file.display()))?;
            if policy.name != name {
                bail!(
                    "{} declares the name {}, so a record's policy field would not \
                     match the file that produced it",
                    file.display(),
                    policy.name
                );
            }
            policy.validate(&file)?;
            policies.insert(policy.name.clone(), policy);
        }

        let floor_file = dir.join("floor.json");
        let floor_text = std::fs::read_to_string(&floor_file)
            .with_context(|| format!("reading {}", floor_file.display()))?;
        let floor: serde_json::Value = serde_json::from_str(&floor_text)
            .with_context(|| format!("parsing {}", floor_file.display()))?;

        let sha = sha_of(&dir)?;
        Ok(Self {
            dir,
            policies,
            floor,
            sha,
        })
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// `policy_sha`, contract 5.1. The sha of the delegation policy directory,
    /// the pattern floor included.
    pub fn sha(&self) -> &str {
        &self.sha
    }

    pub fn get(&self, name: &str) -> Result<&Policy> {
        self.policies.get(name).with_context(|| {
            format!(
                "{name} is not one of the five policies: {}",
                NAMES.join(", ")
            )
        })
    }

    pub fn floor(&self) -> &serde_json::Value {
        &self.floor
    }
}

/// Every `.json` in the directory, hashed in name order.
///
/// The name goes into the hash with its length, so renaming a file changes the
/// sha and no two directories can collide by concatenation. `README.md` is left
/// out: it is prose about the policies rather than one of them, and a wording
/// fix should not change what every run record reports.
fn sha_of(dir: &Path) -> Result<String> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .with_context(|| format!("reading {}", dir.display()))?
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|e| e == "json"))
        .collect();
    files.sort();

    let mut hasher = Sha256::new();
    for file in &files {
        let name = file
            .file_name()
            .and_then(|n| n.to_str())
            .with_context(|| format!("{} has no usable name", file.display()))?;
        let bytes = std::fs::read(file).with_context(|| format!("reading {}", file.display()))?;
        hasher.update((name.len() as u64).to_le_bytes());
        hasher.update(name.as_bytes());
        hasher.update((bytes.len() as u64).to_le_bytes());
        hasher.update(&bytes);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

/// The response shape the gate speaks, passed on the command line and never
/// inferred (6.6). `claude` ignores a response in a shape it does not recognise
/// and allows the call, so a script that guesses wrong is an open door on
/// exactly one provider.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Dialect {
    Claude,
    Cursor,
    Agy,
}

impl Dialect {
    pub fn as_str(self) -> &'static str {
        match self {
            Dialect::Claude => "claude",
            Dialect::Cursor => "cursor",
            Dialect::Agy => "agy",
        }
    }
}

impl fmt::Display for Dialect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Dialect {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "claude" => Ok(Dialect::Claude),
            "cursor" => Ok(Dialect::Cursor),
            "agy" => Ok(Dialect::Agy),
            other => bail!("{other} is not a response dialect the gate speaks"),
        }
    }
}

/// Everything the gate needs that is true of one spawn rather than of the policy.
pub struct Spawn<'a> {
    pub policy: &'a str,
    /// The run's checkout, the `worktree` scope.
    pub worktree: &'a Path,
    /// `~/hydra/skills/`, the `skill_store` scope.
    pub skill_store: &'a Path,
    /// `~/hydra/runs/`, the `corpus` scope and the seventh denied path.
    pub runs: &'a Path,
    /// Where this script is about to be written, `~/hydra/spawn/<run-id>/gate`.
    /// It denies writes to itself, which is 7.5's base case for `permissive` and
    /// `curate`.
    pub gate_script: &'a Path,
    /// `--allow-floor-escape`, harness spec 16. Never inheritable, never
    /// settable by a policy, and it lands in the run record as an argument.
    pub floor_lifted: bool,
}

/// The paths no run may write to, under any policy.
///
/// 7.5's seven, in the order the spec lists them: the five protected configs of
/// 15.1, the generated gate script, and everything under `~/hydra/runs/`.
///
/// **Three of the five configs are paths rather than files today.** Ticket 02
/// wrote `config/preamble.md`. The environment allowlist is still Rust code in
/// `src/env.rs`, and the export allowlist and the canonical MCP document belong
/// to tickets this build has not reached. Denying a write to a path that holds
/// no file is the same denial it will be the day the file lands, so the names
/// are here now rather than added later and forgotten. `IMPLEMENTATION.md`
/// records the gap.
pub fn denied_write_paths(set: &PolicySet, spawn: &Spawn<'_>) -> Vec<PathBuf> {
    let config = Path::new(env!("CARGO_MANIFEST_DIR")).join("config");
    vec![
        // 15.1 #1, and the pattern floor by inheritance.
        set.dir().to_path_buf(),
        // 15.1 #2: a secret reaches a child by inheritance.
        config.join("env-allowlist.json"),
        // 15.1 #3: data leaves the machine.
        config.join("export-allowlist.json"),
        // 15.1 #4: a run is told a different rule than the one Hydra enforces.
        config.join("preamble.md"),
        // 15.1 #5: an MCP server definition is a command Hydra spawns, so a loop
        // that edits this runs code of its choosing inside every future run.
        config.join("mcp.md"),
        // 7.5, the induction's base case for `permissive` and `curate`.
        spawn.gate_script.to_path_buf(),
        // 7.5 and 15.2 entry 12: the record is the evidence.
        spawn.runs.to_path_buf(),
    ]
}

/// Turns a policy name into the gate script of 7.5.
///
/// The policy is compiled into the text. Nothing here emits a path to a policy
/// file, because a script that reads one at fire time reads a file that may have
/// changed since the previous tool call.
pub fn compile(set: &PolicySet, spawn: &Spawn<'_>) -> Result<String> {
    let policy = set.get(spawn.policy)?;

    let mut tools = serde_json::Map::new();
    let mut path_fields = serde_json::Map::new();
    for t in table::CLAUDE {
        tools.insert(t.name.to_string(), t.capability.as_str().into());
        if let Some(field) = t.path_field {
            path_fields.insert(t.name.to_string(), field.into());
        }
    }

    let allow: Vec<serde_json::Value> = policy
        .allow
        .iter()
        .map(|entry| {
            let mut obj = serde_json::Map::new();
            obj.insert("capability".into(), entry.capability.as_str().into());
            if let Some(scope) = entry.scope {
                obj.insert("scope".into(), scope.as_str().into());
            }
            serde_json::Value::Object(obj)
        })
        .collect();

    let config = serde_json::json!({
        "policy": policy.name,
        "policy_sha": set.sha(),
        "allow": allow,
        "scopes": {
            "worktree": resolved(spawn.worktree),
            "skill_store": resolved(spawn.skill_store),
            "corpus": resolved(spawn.runs),
        },
        "deny_write_paths": denied_write_paths(set, spawn)
            .iter()
            .map(|p| resolved(p))
            .collect::<Vec<_>>(),
        "tools": tools,
        "path_fields": path_fields,
        "command_field": table::CLAUDE_COMMAND_FIELD,
        "floor": set.floor(),
        "floor_lifted": spawn.floor_lifted,
    });

    let template = include_str!("gate.py");
    let marker = "HYDRA_CONFIG = None  # HYDRA_COMPILED_POLICY";
    if !template.contains(marker) {
        bail!("gate.py no longer carries the line the compiler replaces");
    }

    // The config goes in as a JSON string that Python parses, rather than as a
    // Python literal. JSON's `true`, `false` and `null` are not Python's, and a
    // literal would need a translation pass that a nested value could defeat.
    let json = serde_json::to_string(&config)?;
    let literal = serde_json::to_string(&json)?;
    let line = format!("HYDRA_CONFIG = json.loads({literal})");

    Ok(template.replace(marker, &line))
}

/// Compiles the gate and writes it to `spawn.gate_script`, executable.
///
/// The provider runs the script as a command, so the exec bit is part of the
/// artifact rather than a detail for the caller to remember. `0o700` rather than
/// `0o755`: nothing but this user runs it, and it is the file 7.5 denies every
/// policy from writing to.
///
/// It is written before the child starts and never rewritten, which is 6.6's
/// "real files, never links, rewritten at every spawn" for the script side.
pub fn write_gate(set: &PolicySet, spawn: &Spawn<'_>) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let text = compile(set, spawn)?;
    if let Some(parent) = spawn.gate_script.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    std::fs::write(spawn.gate_script, text)
        .with_context(|| format!("writing {}", spawn.gate_script.display()))?;
    std::fs::set_permissions(spawn.gate_script, std::fs::Permissions::from_mode(0o700))
        .with_context(|| format!("making {} executable", spawn.gate_script.display()))?;
    Ok(())
}

/// A path with `..` and symlinks resolved, matching what the gate script's own
/// `realpath` will produce at fire time.
///
/// **The tail usually does not exist yet**, which is what makes this more than a
/// `canonicalize` call. The gate script is a denied path and is compiled before
/// it is written; three of the five protected configs are paths whose files this
/// build has not created. `canonicalize` fails outright on any of them, and a
/// fallback that returns the path unresolved leaves `/var/folders/...` on one
/// side and `/private/var/folders/...` on the other, so the denial silently
/// stops matching. Python resolves the existing prefix and keeps the rest, so
/// this does the same: walk up to the longest ancestor that exists, canonicalize
/// that, and put the remainder back.
///
/// Lowercased because 15.2 matches case-insensitively.
fn resolved(path: &Path) -> String {
    let mut tail: Vec<std::ffi::OsString> = Vec::new();
    let mut cursor = path.to_path_buf();
    loop {
        if let Ok(base) = std::fs::canonicalize(&cursor) {
            let mut out = base;
            for part in tail.iter().rev() {
                out.push(part);
            }
            return out.to_string_lossy().to_lowercase();
        }
        match (cursor.file_name(), cursor.parent()) {
            (Some(name), Some(parent)) if !parent.as_os_str().is_empty() => {
                tail.push(name.to_os_string());
                cursor = parent.to_path_buf();
            }
            // Nothing on the way up exists, so there is nothing to resolve.
            _ => return path.to_string_lossy().to_lowercase(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set() -> PolicySet {
        PolicySet::discover().expect("the shipped policy set loads")
    }

    #[test]
    fn the_five_load_and_name_themselves() {
        let set = set();
        for name in NAMES {
            assert_eq!(set.get(name).unwrap().name, name);
        }
    }

    #[test]
    fn judge_allows_nothing() {
        assert!(set().get("judge").unwrap().allow.is_empty());
    }

    #[test]
    fn only_permissive_spawns_subagents() {
        // 7.6. The cost belongs in the spec rather than in a footnote: a
        // delegated `claude` child cannot use `Task`.
        let set = set();
        for name in NAMES {
            let has = set
                .get(name)
                .unwrap()
                .allow
                .iter()
                .any(|a| a.capability == Capability::SubagentSpawn);
            assert_eq!(has, name == "permissive", "{name} and subagent_spawn");
        }
    }

    #[test]
    fn curate_and_standard_are_incomparable() {
        // 7.3. The five are a partial order rather than a chain, which is why a
        // `standard` parent cannot delegate a `curate` child and a curator pass
        // starts at depth 0 from a human.
        let set = set();
        let curate = set.get("curate").unwrap();
        let standard = set.get("standard").unwrap();
        assert!(!curate.contains(standard));
        assert!(!standard.contains(curate));
    }

    #[test]
    fn permissive_contains_all_five_and_judge_contains_only_itself() {
        let set = set();
        let permissive = set.get("permissive").unwrap();
        let judge = set.get("judge").unwrap();
        for name in NAMES {
            let policy = set.get(name).unwrap();
            assert!(
                permissive.contains(policy),
                "permissive should contain {name}"
            );
            assert_eq!(judge.contains(policy), name == "judge", "judge and {name}");
        }
    }

    #[test]
    fn the_sha_is_stable_and_moves_with_the_floor() {
        let dir = tempdir();
        for name in NAMES {
            std::fs::copy(
                Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("policies")
                    .join(format!("{name}.json")),
                dir.join(format!("{name}.json")),
            )
            .unwrap();
        }
        let floor = dir.join("floor.json");
        std::fs::copy(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("policies")
                .join("floor.json"),
            &floor,
        )
        .unwrap();

        let before = PolicySet::load(&dir).unwrap().sha().to_string();
        assert_eq!(before, PolicySet::load(&dir).unwrap().sha());

        let mut text: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&floor).unwrap()).unwrap();
        text["git"]["protected_branches"]
            .as_array_mut()
            .unwrap()
            .push("staging".into());
        std::fs::write(&floor, serde_json::to_string_pretty(&text).unwrap()).unwrap();

        assert_ne!(
            before,
            PolicySet::load(&dir).unwrap().sha(),
            "policy_sha must move when the floor does, contract 5.1"
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_readme_edit_does_not_move_the_sha() {
        let dir = tempdir();
        for name in NAMES {
            std::fs::copy(
                Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("policies")
                    .join(format!("{name}.json")),
                dir.join(format!("{name}.json")),
            )
            .unwrap();
        }
        std::fs::copy(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("policies")
                .join("floor.json"),
            dir.join("floor.json"),
        )
        .unwrap();
        let before = PolicySet::load(&dir).unwrap().sha().to_string();
        std::fs::write(dir.join("README.md"), "a wording fix\n").unwrap();
        assert_eq!(before, PolicySet::load(&dir).unwrap().sha());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn the_compiled_script_carries_no_path_to_a_policy_file() {
        // 7.5: not a script that reads a policy file when it fires.
        let set = set();
        let script = compile(
            &set,
            &Spawn {
                policy: "standard",
                worktree: Path::new("/tmp/wt"),
                skill_store: Path::new("/tmp/skills"),
                runs: Path::new("/tmp/runs"),
                gate_script: Path::new("/tmp/spawn/gate"),
                floor_lifted: false,
            },
        )
        .unwrap();
        assert!(!script.contains("standard.json"));
        assert!(!script.contains("floor.json"));
        assert!(script.contains("\\\"policy\\\":\\\"standard\\\""));
    }

    fn tempdir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "hydra-policy-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }
}
