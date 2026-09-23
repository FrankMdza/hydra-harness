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
    /// Invoking a skill the run was linked (7.1). Its own capability rather than
    /// one of the nineteen that answer to [`Capability::ProviderNative`], because
    /// a run that is linked a skill and cannot invoke it was given a
    /// listing entry rather than a skill. Ticket 39 measured the delivery on
    /// `claude`: an allowed `Skill` call brings the body into context with no
    /// file read the gate sees, so this carries that tier on its own.
    ///
    /// `ToolSearch` answers here too, since ticket 43. A deferred tool needs it
    /// before anything can call the tool, so denying it to a run that was handed
    /// a server hands that run a listing entry the same way.
    SkillInvoke,
    /// The tools that exist only inside one provider's own product surface and
    /// that no policy in 7.2 grants. See `table.rs` for why this row exists.
    ProviderNative,
    /// A tool the provider runs without asking the hook. No policy grants it,
    /// and the compiler skips the row so `gate.py` never sees a name it cannot
    /// answer for. Map ticket 51.
    Unhooked,
    /// A shell facility that carries no command for the gate to read.
    ///
    /// `WriteShellStdin` on `cursor-agent` and `send_command_input` on `agy`
    /// write to the stdin of a command that is already running. The command was
    /// judged when it started; what is typed into it afterwards is not, and
    /// `raw_command` is empty, so the pattern floor of 7.4 and the path denials
    /// of 7.5 both have nothing to read. They were `shell` rows until build
    /// ticket 57's review, which is how "denied under every policy" went with
    /// them: a run under `permissive` could feed `cat > ~/hydra/runs/x` to a
    /// shell it had already opened.
    ///
    /// No policy grants this, and the row stays compiled rather than skipped, so
    /// the gate answers `hydra/capability` by name instead of
    /// `hydra/unknown-tool`. The cost is real and named in `IMPLEMENTATION.md`:
    /// a run cannot answer an interactive prompt. It gets the facility back the
    /// day the gate can see what is being typed. Build ticket 59.
    OpaqueShell,
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
            Capability::SkillInvoke => "skill_invoke",
            Capability::ProviderNative => "provider_native",
            Capability::Unhooked => "unhooked",
            Capability::OpaqueShell => "opaque_shell",
        }
    }
}

/// Where a scoped capability may act.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Scope {
    /// The run's own checkout.
    Worktree,
    /// `~/hydra/skills/`, the store's own working tree.
    ///
    /// **No policy writes it.** 7.5 denies it under all five since ticket 40,
    /// and its only writers are [`crate::skills::Store::pin`] and a human. A
    /// curator writing here would hand its half-finished proposal to whichever
    /// run launches next, which is what [`Scope::SkillCheckout`] exists for.
    SkillStore,
    /// `~/hydra/skill-worktrees/<run-id>/`, the `curate` pass's own git
    /// worktree of the store.
    ///
    /// The one `file_write` scope any policy puts outside the run's worktree,
    /// and the reason 7.5's deny list has a second kind of entry: the
    /// checkouts directory is denied and this run's own checkout is carved out
    /// of that denial. Rungs spec 3.6 is the design and ticket 40 settled
    /// the name.
    SkillCheckout,
    /// `~/hydra/skills-snapshots/<sha>/`, the checkout this run's links point
    /// into. Read only: 6.4 gives a run its own skills to read and nothing to
    /// write, and a run that could edit the snapshot could edit its own
    /// instructions mid-session. Resolved per run from the sha the run pinned,
    /// which contract 7.7 recomputes from `hydra.skills.store_sha`.
    SkillSnapshot,
    /// `~/hydra/runs/`, read only in practice: 7.5 denies writes to it under
    /// every policy, because a loop that learns from the corpus must not be
    /// able to edit the corpus.
    Corpus,
    Anywhere,
}

impl Scope {
    /// `const` so [`keys`] can name these without a second spelling.
    pub const fn as_str(self) -> &'static str {
        match self {
            Scope::Worktree => "worktree",
            Scope::SkillStore => "skill_store",
            Scope::SkillCheckout => "skill_checkout",
            Scope::SkillSnapshot => "skill_snapshot",
            Scope::Corpus => "corpus",
            Scope::Anywhere => "anywhere",
        }
    }
}

/// The names a resolved path is keyed by, in one place.
///
/// **Three readers key on these and nothing but equality ties them together**:
/// the compiled gate's `scopes` block, `run_start.scopes` on the record, and the
/// reader that prefers a stored path over a rebuilt one. A typo in any of the
/// three is a scope that silently stops matching, which is the failure map
/// ticket 42 is about, so the strings live here and the enum above owns five of
/// them. The other two are per-run denied paths rather than scopes a policy can
/// name, and they are on the record because nothing else recovers them.
pub mod keys {
    use super::Scope;

    pub const WORKTREE: &str = Scope::Worktree.as_str();
    pub const SKILL_STORE: &str = Scope::SkillStore.as_str();
    pub const SKILL_SNAPSHOT: &str = Scope::SkillSnapshot.as_str();
    pub const SKILL_CHECKOUT: &str = Scope::SkillCheckout.as_str();
    pub const CORPUS: &str = Scope::Corpus.as_str();
    pub const GATE_SCRIPT: &str = "gate_script";
    pub const PARENT_GIT_DIR: &str = "parent_git_dir";
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
    /// chain: `curate` writes to its checkout of the store and cannot touch a
    /// worktree, `standard` is the reverse, and neither contains the other. An
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
    /// The set shipped beside [`crate::paths::hydra_home`].
    pub fn discover() -> Result<Self> {
        let home = crate::paths::hydra_home();
        ensure_shipped_config(&home)?;
        Self::load(home.join("policies"))
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

/// Refuses before any policy file is opened when the shipped tree is missing.
fn ensure_shipped_config(home: &Path) -> Result<()> {
    let policies = home.join("policies");
    if !policies.is_dir() {
        bail!(
            "cannot find policies at {}: set {} to the directory that contains policies/ and config/",
            policies.display(),
            crate::paths::HYDRA_HOME_VAR
        );
    }
    let config = home.join("config");
    if !config.is_dir() {
        bail!(
            "cannot find config at {}: set {} to the directory that contains policies/ and config/",
            config.display(),
            crate::paths::HYDRA_HOME_VAR
        );
    }
    Ok(())
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
    /// Which binary the gate is compiled for. It picks the capability table of
    /// 7.1, the field the shell command rides in, the MCP prefix, and the
    /// dialect the script will insist on being asked in.
    pub provider: crate::provider::Provider,
    pub policy: &'a str,
    /// The `~/hydra` layout this run was launched under.
    ///
    /// It carries seven of the directories 7.5 denies, one accessor each.
    /// Passing the layout rather than the paths is what keeps the deny list and
    /// harness spec 5's table readable against each other. The scopes below are
    /// named paths instead, because a replay takes those off the record and only
    /// the deny list has to come from the reader's own layout.
    pub layout: &'a crate::paths::Layout,
    /// The run's checkout, the `worktree` scope and the first carve-out.
    pub worktree: &'a Path,
    /// The skill store, `~/hydra/skills/`, which every policy that grants a
    /// read of a skill is scoped to.
    pub skill_store: &'a Path,
    /// `~/hydra/runs/`, the `corpus` scope `curate` reads.
    pub corpus: &'a Path,
    /// This run's snapshot of the store, the `skill_snapshot` scope. It is where
    /// the injected links resolve to, so a read through a link lands inside it.
    /// A run with no checkout has no links and no snapshot, and the caller
    /// passes a path nothing creates rather than the snapshots directory.
    pub skill_snapshot: &'a Path,
    /// This run's checkout of the skill store, the `skill_checkout` scope and
    /// the second carve-out. `Some` on a `curate` pass and `None` on every other
    /// policy, so a spawn that was given no checkout can write in none.
    pub skill_checkout: Option<&'a Path>,
    /// The parent repo's `.git`, which `hydra run` knows because it ran
    /// `git worktree add` from it. `None` for a run outside a repo, which has
    /// no parent to protect.
    pub parent_git_dir: Option<&'a Path>,
    /// Where this script is about to be written, `~/hydra/spawn/<run-id>/gate`.
    /// It denies writes to itself, which is 7.5's base case for `permissive` and
    /// `curate`.
    pub gate_script: &'a Path,
    /// `--allow-floor-escape`, harness spec 16. Never inheritable, never
    /// settable by a policy, and it lands in the run record as an argument.
    pub floor_lifted: bool,
}

/// The scope names contract 5.1 puts on `run_start.scopes`.
///
/// `skill_snapshot` is not among them: it recomputes from `skills.store_sha`,
/// which is already on the record, and a stored copy would be a second answer to
/// a question line 0 answers once. `gate_script` and `parent_git_dir` are, even
/// though the script resolves no scope against them, because both are denied
/// paths the record is the only copy of: a moved spawn directory flips a
/// `permissive` write from `protected-path` to `allow`, measured under ticket 42,
/// and the parent's `.git` is unrecoverable once `hydra reap` has taken the
/// worktree, measured under ticket 14.
pub const RECORDED_SCOPES: &[&str] = &[
    keys::WORKTREE,
    keys::SKILL_STORE,
    keys::SKILL_CHECKOUT,
    keys::CORPUS,
    keys::GATE_SCRIPT,
    keys::PARENT_GIT_DIR,
];

/// The scope names the compiled script resolves a call against.
const GATE_SCOPES: &[&str] = &[
    keys::WORKTREE,
    keys::SKILL_STORE,
    keys::SKILL_SNAPSHOT,
    keys::CORPUS,
    keys::SKILL_CHECKOUT,
];

impl Spawn<'_> {
    /// Every path this spawn names, canonicalized, keyed by scope name.
    ///
    /// **One function, so the gate and the record cannot disagree.** The script
    /// compiled below and the `scopes` object on `run_start` are two views over
    /// this map rather than two lists of accessors, which is what makes contract
    /// 7.7's "the re-evaluation resolves against the stored paths" a property of
    /// the code instead of a promise two files keep separately.
    ///
    /// A path the spawn has none of is absent, never a placeholder: no
    /// `skill_checkout` outside a `curate` pass, no `parent_git_dir` outside a
    /// repository.
    ///
    /// A launch calls this twice, once for the record and once inside
    /// [`compile`], and each call canonicalizes seven paths. That is deliberate:
    /// a handful of `canonicalize` calls next to a process spawn is not worth a
    /// second signature to thread the map through.
    pub fn paths(&self) -> BTreeMap<&'static str, PathBuf> {
        let mut paths = BTreeMap::new();
        paths.insert(keys::WORKTREE, resolved_path(self.worktree));
        paths.insert(keys::SKILL_STORE, resolved_path(self.skill_store));
        paths.insert(keys::SKILL_SNAPSHOT, resolved_path(self.skill_snapshot));
        paths.insert(keys::CORPUS, resolved_path(self.corpus));
        paths.insert(keys::GATE_SCRIPT, resolved_path(self.gate_script));
        if let Some(checkout) = self.skill_checkout {
            paths.insert(keys::SKILL_CHECKOUT, resolved_path(checkout));
        }
        if let Some(git_dir) = self.parent_git_dir {
            paths.insert(keys::PARENT_GIT_DIR, resolved_path(git_dir));
        }
        paths
    }

    /// The paths contract 5.1 asks `run_start` to carry.
    ///
    /// **The case is kept here and folded in the script.** 15.2 matches
    /// case-insensitively, so the gate compares casefolded strings, and a
    /// lowercased path is not a path on a case-sensitive filesystem. The record
    /// stores what a reader can open; [`compile`] folds it again on the way
    /// back in, so a replay hands the script the bytes the launch's script had.
    pub fn recorded_scopes(&self) -> BTreeMap<String, String> {
        let paths = self.paths();
        RECORDED_SCOPES
            .iter()
            .filter_map(|name| {
                let path = paths.get(name)?;
                Some(((*name).to_string(), path.to_string_lossy().into_owned()))
            })
            .collect()
    }
}

/// The paths no run may write to, under any policy, before the policy is
/// consulted at all.
///
/// 7.5's "denied outright", in the order the spec lists it. The rule behind the
/// list is one sentence: a run writes only inside what Hydra created for it.
/// The per-run parents are denied rather than the siblings enumerated, because
/// siblings appear and disappear while a run is live, and what the run does own
/// comes back out of the denial in [`own_write_paths`].
///
/// Ticket 40 measured why the flat list of seven was not enough. Under
/// `standard`, whose `shell` carries no scope, `rm -rf` on a sibling's worktree,
/// on `~/hydra/pruned/`, on the store's `.git` and on the parent repo's `.git`
/// all came back allowed, and a `Write` into a sibling's gate script came back
/// allowed under `permissive`.
///
/// **Three of the five configs are paths rather than files today.** Ticket 02
/// wrote `config/preamble.md`. The environment allowlist is still Rust code in
/// `src/env.rs`, and the export allowlist and the canonical MCP document belong
/// to tickets this build has not reached. Denying a write to a path that holds
/// no file is the same denial it will be the day the file lands, so the names
/// are here now rather than added later and forgotten. `IMPLEMENTATION.md`
/// records the gap.
pub fn denied_write_paths(set: &PolicySet, spawn: &Spawn<'_>) -> Vec<PathBuf> {
    let config = crate::paths::hydra_home().join("config");
    let layout = spawn.layout;
    let mut paths = vec![
        // 15.1 #1, and the pattern floor by inheritance.
        set.dir().to_path_buf(),
        // 15.1 #2: a secret reaches a child by inheritance.
        config.join("env-allowlist.json"),
        // 15.1 #3: data leaves the machine.
        config.join("export-allowlist.json"),
        // The sign-off manifest beside it, build ticket 65: the human's word on
        // which judge findings a pair may leave under.
        config.join("export-signoff.json"),
        // 15.1 #4: a run is told a different rule than the one Hydra enforces.
        config.join("preamble.md"),
        // 15.1 #5: an MCP server definition is a command Hydra spawns, so a loop
        // that edits this runs code of its choosing inside every future run.
        config.join("mcp.md"),
        // Map ticket 65, build ticket 74: the checkout the binary was built from.
        // Own worktrees and a curate pass's checkout are carved back out below.
        crate::paths::hydra_home(),
        // 7.5, the induction's base case for `permissive` and `curate`.
        spawn.gate_script.to_path_buf(),
        // 7.5 and 15.2 entry 12: the record is the evidence, and a loop that
        // learns from the corpus must not be able to edit the corpus.
        layout.runs(),
        // `hydra prune` moves a record rather than deleting it, and that promise
        // is empty if a child can delete the moved copy.
        layout.pruned(),
        // Build ticket 81: a run that deletes a pending launch's reservation, or
        // its lock, lets the next launch past the cap.
        layout.reservations(),
        // The store and its snapshots are the other half of D5's seam. Rung one
        // proposes rather than lands, and a run that writes its own snapshot
        // edits its own instructions mid-session.
        layout.skills(),
        layout.snapshots(),
    ];
    // Shared by every worktree, so it holds every run's branch and, on a machine
    // where the parent is Hydra's own checkout, Hydra's own history.
    if let Some(git_dir) = spawn.parent_git_dir {
        paths.push(git_dir.to_path_buf());
    }
    // The three per-run parents. A sibling's worktree holds work `hydra reap`
    // has not committed yet, a sibling's checkout holds a proposal no human has
    // seen, and a sibling's spawn directory holds the gate its every call runs
    // through: a replaced one ends that run as `hydra/gate-tampered`.
    paths.push(layout.worktrees());
    paths.push(layout.spawns());
    paths.push(layout.skill_worktrees());
    paths
}

/// What this one spawn owns, carved out of the denials above.
///
/// 7.5's second kind of entry. The check both halves of the gate apply is
/// "inside a denied path and not inside an own path", which is what lets the
/// parents be denied while a run still writes in its own worktree, and a
/// `curate` pass in its own checkout of the store.
///
/// **The run's own spawn directory is deliberately not here.** Hydra writes it
/// at spawn, and a write into a sibling's copy is how a `permissive` run
/// replaces that sibling's gate. The vendor's own data root under it is a read
/// carve-out, not a write one: see [`own_read_paths`].
pub fn own_write_paths(spawn: &Spawn<'_>) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    // A run outside a repo has no checkout, and `hydra run` scopes it to
    // `<spawn dir>/no-worktree`, a path nothing creates. Carving that out would
    // hand back a piece of the run's own spawn directory, which the paragraph
    // above keeps whole, so the worktree is an own path only when it is one.
    // Such a run reads and shells out and writes nowhere, which is what
    // `src/run.rs` says it does.
    if !spawn.worktree.starts_with(spawn.layout.spawns()) {
        paths.push(spawn.worktree.to_path_buf());
        // The checkout's private git directory lives inside the parent's `.git`,
        // which is denied whole. `index`, `HEAD` and `logs/` for this run sit
        // there, at `worktrees/<run-id>/` when Hydra names the checkout from
        // the run id.
        if let Some(git_dir) = spawn.parent_git_dir
            && let Some(run_id) = spawn.gate_script.parent().and_then(|p| p.file_name())
        {
            paths.push(git_dir.join("worktrees").join(run_id));
            // `git add` writes the blob into the shared object store before it
            // touches the index, measured 2026-09-22 under a real profile:
            // "unable to create temporary file: Operation not permitted". The
            // store is content-addressed, so a run can add objects and cannot
            // alter one; `refs/` stays denied, so no branch moves from inside
            // and `git commit` still fails on its own ref.
            paths.push(git_dir.join("objects"));
        }
    }
    if let Some(checkout) = spawn.skill_checkout {
        paths.push(checkout.to_path_buf());
    }
    paths
}

/// The files inside the worktree that decide what gates the run, denied under
/// every policy even though the worktree is the run's own. 7.5's third kind of
/// entry, and 15.2 row 9 read to its end.
///
/// The worktree is carved out of the denials because a run writes inside what
/// Hydra created for it, and two of the three providers read their hook config
/// out of exactly that directory: `cursor-agent` from `.cursor/hooks.json`,
/// `agy` from `.agents/hooks.json`. `standard` grants `file_write` over the whole
/// worktree, so before build ticket 44 a run could rewrite the file that points
/// at its gate. Whether either provider reloads hooks mid-session is not
/// measured, and the denial does not depend on the answer: the rewritten file
/// lands on the branch `hydra reap` commits and is the human's repo's copy on
/// the next seed, which [`crate::provider::claim`] then refuses to write over.
///
/// **Every provider's files are denied on every provider's gate.** A `claude`
/// run has no `.cursor/hooks.json`, and one it writes is committed all the same.
/// The MCP configs are here because a server definition is a command the
/// provider spawns (15.1 #5, one directory down). `claude`'s three are the
/// project-scope files it reads from cwd, which add hooks rather than remove
/// Hydra's; [`crate::provider::claude::project_config_files`] says why that is
/// enough. The skill roots are not here: they are denied to `file_write` only,
/// by [`write_denied_worktree_paths`], because a shell command naming one is how
/// two providers read a skill.
///
/// **The `.gitignore` files Hydra drops beside these stay writable.** A repo may
/// track its own, `claim` reads that copy rather than replacing it, and the
/// ignore file decides what `hydra reap` stages, not what the gate allows.
///
/// The gate checks this list before the own carve-out, so it is the one place a
/// path can be inside the worktree and still `protected-path`. The names come
/// off the provider modules, so a provider that moves its file moves the denial.
pub fn protected_worktree_paths(worktree: &Path) -> Vec<PathBuf> {
    use crate::provider::{agy, claude, codex, cursor};
    let mut paths = vec![
        cursor::hooks_file(worktree),
        cursor::mcp_file(worktree),
        agy::hooks_file(worktree),
        agy::mcp_file(worktree),
        codex::hooks_file(worktree),
        codex::mcp_file(worktree),
    ];
    paths.extend(claude::project_config_files(worktree));
    paths
}

/// Every spelling of every denied path this spawn does not own.
///
/// The raw pass of `protected_path_named_by` searches a command for these as
/// substrings, which is how it catches a path spelled in full inside a larger
/// token: `sh -c 'cat /…/runs/x'` is one token to shlex and names the corpus in
/// full. A substring search cannot resolve anything, so both the resolved form
/// and the literal one go in; on macOS a temporary directory is reachable as
/// `/var/folders/...` and as `/private/var/folders/...` and a command may use
/// either.
fn denied_spellings(set: &PolicySet, spawn: &Spawn<'_>) -> Vec<String> {
    let own: Vec<String> = own_write_paths(spawn).iter().map(|p| resolved(p)).collect();
    // A denied path with one of this spawn's own paths under it cannot go on
    // this list: `~/hydra/worktrees` is denied and the run's own worktree lives
    // in it, so a substring search would refuse the run's own working
    // directory. `protected_root` expresses that carve-out and a substring
    // cannot, so those paths are left to the token pass and the residual is
    // written down in `gate.py` rather than papered over.
    let holds_own = |path: &str| {
        own.iter()
            .any(|o| o == path || o.starts_with(&format!("{}/", path.trim_end_matches('/'))))
    };
    let mut out: Vec<String> = Vec::new();
    let push = |path: &Path, out: &mut Vec<String>| {
        for spelling in [resolved(path), casefold(path)] {
            if !spelling.is_empty() && !out.contains(&spelling) {
                out.push(spelling);
            }
        }
    };
    for path in protected_worktree_paths(spawn.worktree) {
        push(&path, &mut out);
    }
    for path in denied_write_paths(set, spawn) {
        if !holds_own(&resolved(&path)) {
            push(&path, &mut out);
        }
    }
    out
}

/// The skill roots Hydra links into the worktree, one per provider, denied to
/// `file_write` under every policy and to nothing else.
///
/// A write there replaces a link into the snapshot. A shell command naming the
/// root is not refused, because `cat .claude/skills/x/SKILL.md` is how
/// `cursor-agent` and `agy` read a skill, and `ls -la .claude/skills/` on run
/// 01a09405-f451 is what a first version of build ticket 44 refused.
pub fn write_denied_worktree_paths(worktree: &Path) -> Vec<PathBuf> {
    use crate::provider::Provider;
    [
        Provider::Claude,
        Provider::CursorAgent,
        Provider::Agy,
        Provider::Codex,
    ]
    .into_iter()
    .map(|provider| worktree.join(provider.skill_root()))
    .collect()
}

/// What this one spawn may read outside the scopes the policy named.
///
/// A second kind of carve-out, beside [`own_write_paths`]. The spawn directory
/// is a denied parent, and no policy names it as a read scope, so a `file_read`
/// of a file the provider itself wrote there is refused as out-of-scope unless
/// this list names it. On `cursor-agent` it is the run's own `cursor-data/`,
/// which is where `CURSOR_DATA_DIR` points and where Cursor writes the terminal
/// files it then reads to answer `awaitToolCall`. No other provider adds an
/// entry: their binaries do not ask the hook to read a path under the spawn
/// directory.
pub fn own_read_paths(spawn: &Spawn<'_>) -> Vec<PathBuf> {
    if spawn.provider != crate::provider::Provider::CursorAgent {
        return Vec::new();
    }
    // The gate lives at `<spawn>/<run-id>/gate`; `cursor-data/` sits beside it,
    // which is what [`crate::paths::Layout::cursor_data_dir`] names.
    match spawn.gate_script.parent() {
        Some(dir) => vec![dir.join("cursor-data")],
        None => Vec::new(),
    }
}

/// Turns a policy name into the gate script of 7.5.
///
/// The policy is compiled into the text. Nothing here emits a path to a policy
/// file, because a script that reads one at fire time reads a file that may have
/// changed since the previous tool call.
pub fn compile(set: &PolicySet, spawn: &Spawn<'_>) -> Result<String> {
    let policy = set.get(spawn.policy)?;

    // 7.1's table is per provider, because the three share no vocabulary. The
    // gate is compiled for one binary and the config says which, so the script
    // can refuse to answer in another's shape.
    let table = table::Table::of(spawn.provider);
    let mut tools = serde_json::Map::new();
    let mut path_fields = serde_json::Map::new();
    let mut patch_path_tools = serde_json::Map::new();
    for t in table.tools {
        // Unhooked rows exist for the replay, not for the hook. Compiling one
        // would add a capability `gate.py` does not know, and the hook never
        // asks.
        if matches!(t.capability, Capability::Unhooked) {
            continue;
        }
        tools.insert(t.name.to_string(), t.capability.as_str().into());
        if let Some(field) = t.path_field {
            path_fields.insert(t.name.to_string(), field.into());
        }
        if t.patch_paths {
            patch_path_tools.insert(t.name.to_string(), table.command_field.into());
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

    // The same map the record's `scopes` object is built from, folded to the
    // case 15.2 matches on. `skill_checkout` is emitted only when the spawn was
    // given one: a policy naming that scope on a spawn without a checkout finds
    // no root to match, which denies rather than falling back to a directory
    // the run does not own.
    let named = spawn.paths();
    let mut scopes = serde_json::Map::new();
    for name in GATE_SCOPES {
        if let Some(path) = named.get(name) {
            scopes.insert((*name).into(), casefold(path).into());
        }
    }

    let config = serde_json::json!({
        "policy": policy.name,
        "policy_sha": set.sha(),
        // The shape this gate answers in. The script is also told on argv, and
        // a caller naming a different one is refused: the two name the same
        // binary or something is wired wrong, and a wrong shape is an open
        // door on `claude`.
        "dialect": spawn.provider.dialect().as_str(),
        "allow": allow,
        "scopes": scopes,
        "deny_write_paths": denied_write_paths(set, spawn)
            .iter()
            .map(|p| resolved(p))
            .collect::<Vec<_>>(),
        // 7.5's carve-out. The gate reads a write as protected when it is inside
        // a denied path and inside none of these.
        "own_write_paths": own_write_paths(spawn)
            .iter()
            .map(|p| resolved(p))
            .collect::<Vec<_>>(),
        // The hole in that carve-out: the run's own hook and provider config,
        // inside the worktree and denied anyway. Checked before the own list.
        "protected_worktree_paths": protected_worktree_paths(spawn.worktree)
            .iter()
            .map(|p| resolved(p))
            .collect::<Vec<_>>(),
        // The skill roots, denied to writes only. The floor never reads this key.
        "write_denied_worktree_paths": write_denied_worktree_paths(spawn.worktree)
            .iter()
            .map(|p| resolved(p))
            .collect::<Vec<_>>(),
        // Every spelling of a denied path the raw shell pass looks for, build
        // ticket 56. Two forms per path, because `/var/folders/...` and the
        // `/private/var/folders/...` it resolves to are both things a command
        // can write down, and a substring search cannot resolve either one.
        // The own carve-out is applied here, so a run naming its own worktree
        // is not refused.
        "denied_spellings": denied_spellings(set, spawn),
        // The matching read carve-out. Empty on every provider but
        // `cursor-agent`, whose own `cursor-data/` is the one path under the
        // spawn directory a gated tool call has reason to open.
        "own_read_paths": own_read_paths(spawn)
            .iter()
            .map(|p| resolved(p))
            .collect::<Vec<_>>(),
        "tools": tools,
        "path_fields": path_fields,
        "patch_path_tools": patch_path_tools,
        "command_field": table.command_field,
        "mcp_prefix": table.mcp_prefix,
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

/// The sha256 of the gate this build compiles for `spawn`.
///
/// The same bytes [`write_gate`] puts on disk, which is what `src/run.rs`
/// hashes back off the file and writes on `run_start` as `gate_sha`. Contract
/// 7.7's re-evaluation calls this to ask whether the gate it is about to
/// compile is the gate that answered, build ticket 49.
pub fn gate_sha(set: &PolicySet, spawn: &Spawn<'_>) -> Result<String> {
    use sha2::{Digest, Sha256};
    Ok(format!(
        "{:x}",
        Sha256::digest(compile(set, spawn)?.as_bytes())
    ))
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

/// The hook command a provider runs: check the gate against the sha it was
/// written with, then run it, answering in `dialect`.
///
/// **The script is a file the child can write and the config is not.** All three
/// providers snapshot their hook configuration at process start (6.6), while 7.5
/// says the script is "read fresh on every invocation". 7.5 closes that for
/// `judge`, `read-only` and `standard` by scoping their writes to the worktree,
/// and gives `permissive` and `curate` a base case by denying writes to the script
/// under every policy. Ticket 02's review found the base case reachable anyway:
/// the denial was applied to tools carrying a path and `Bash` carries none, so a
/// shell redirect emptied the script, and an empty script exits 0 printing nothing,
/// which 6.6 measured `claude` treating as permission to proceed.
///
/// The path check now refuses a command that names the script, and this is the
/// half that does not depend on reading argv. The sha is computed after the script
/// is written and travels in the config, which the provider has already snapshotted
/// by the time the child can run anything. A rewritten or deleted script therefore
/// fails **closed**, in Hydra's own voice, whatever spelling reached it.
///
/// **The refusal is spoken in the caller's dialect**, the same as every answer
/// the gate gives, because a refusal the provider cannot parse is an invalid
/// response rather than a deny: `claude` would allow the call, and Cursor would
/// block it under `failClosed` and lose the reason.
///
/// The script still lives where 7.5 puts it. Only the pointer got stricter.
pub fn hook_command(gate_script: &Path, gate_sha: &str, dialect: Dialect) -> String {
    // `python3 -c <source> <dialect>` puts the dialect at sys.argv[1], which is
    // where the gate reads it, and leaves `__name__` as `__main__` so its own
    // entry point still fires under `exec`.
    format!(
        "/usr/bin/python3 -c {} {}",
        shell_quote(&hook_source(gate_script, gate_sha, dialect)),
        dialect.as_str()
    )
}

/// The Python behind [`hook_command`], for a provider that wants it in a file.
///
/// `codex` cannot take the multi-line command through its config override and
/// reads no project hook file inside a git worktree, measured 2026-09-21, so
/// its writer puts this source at `spawn/<run-id>/hook.py` and the command line
/// names the file. Same check, same refusal, same dialect argument.
pub fn hook_source(gate_script: &Path, gate_sha: &str, dialect: Dialect) -> String {
    // JSON string literals are Python string literals, so this quotes a path
    // containing anything a filesystem allows without a second escaping rule.
    let path = serde_json::to_string(&gate_script.display().to_string())
        .unwrap_or_else(|_| "\"\"".to_string());
    let sha = serde_json::to_string(gate_sha).unwrap_or_else(|_| "\"\"".to_string());
    let reason = "hydra/gate-tampered: the compiled gate is gone or no longer matches the \
                  sha this run was launched with, so nothing can be decided and the call \
                  is refused.";
    let refusal = match dialect {
        Dialect::Claude => serde_json::json!({
            "hookSpecificOutput": {
                "hookEventName": "PreToolUse",
                "permissionDecision": "deny",
                "permissionDecisionReason": reason,
            }
        }),
        Dialect::Cursor => serde_json::json!({
            "permission": "deny",
            "user_message": reason,
            "agent_message": reason,
        }),
        Dialect::Agy => serde_json::json!({ "decision": "deny", "reason": reason }),
    };
    let refusal =
        serde_json::to_string(&refusal.to_string()).unwrap_or_else(|_| "\"\"".to_string());
    format!(
        r#"import hashlib, sys
GATE = {path}
SHA = {sha}
try:
    body = open(GATE, "rb").read()
except OSError:
    body = None
if body is None or hashlib.sha256(body).hexdigest() != SHA:
    sys.stdout.write({refusal})
    sys.exit(0)
exec(compile(body, GATE, "exec"))
"#
    )
}

/// One shell word.
///
/// The hook `command` is a string a shell runs, and it carries both a path under a
/// home directory Hydra does not name and a block of Python. A space or a quote in
/// either would split the command and run the gate with no dialect, which denies
/// and exits 2 rather than failing open, but would deny every call on every run.
pub(crate) fn shell_quote(text: &str) -> String {
    format!("'{}'", text.replace('\'', r"'\''"))
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
/// **The case is kept.** The gate compares casefolded, [`casefold`] is what
/// does it, and the run record stores what this returns: a lowercased path is
/// not a path on a case-sensitive filesystem, and a reader has to be able to
/// open what line 0 names.
fn resolved_path(path: &Path) -> PathBuf {
    let mut tail: Vec<std::ffi::OsString> = Vec::new();
    let mut cursor = path.to_path_buf();
    loop {
        if let Ok(base) = std::fs::canonicalize(&cursor) {
            let mut out = base;
            for part in tail.iter().rev() {
                out.push(part);
            }
            return out;
        }
        match (cursor.file_name(), cursor.parent()) {
            (Some(name), Some(parent)) if !parent.as_os_str().is_empty() => {
                tail.push(name.to_os_string());
                cursor = parent.to_path_buf();
            }
            // Nothing on the way up exists, so there is nothing to resolve.
            _ => return path.to_path_buf(),
        }
    }
}

/// Lowercase ASCII letters only. macOS filesystem case-insensitivity does not
/// fold Unicode characters like `ß` or `ﬁ`, and Rust's `to_lowercase` and
/// Python's `casefold` disagree on those anyway, so both sides use this map.
fn fold_ascii(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii() {
                c.to_ascii_lowercase()
            } else {
                c
            }
        })
        .collect()
}

/// One path as the compiled script compares it, 15.2's case-insensitive match.
fn casefold(path: &Path) -> String {
    fold_ascii(&path.to_string_lossy())
}

/// A resolved path, folded to the case the script matches on.
fn resolved(path: &Path) -> String {
    casefold(&resolved_path(path))
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
    fn standard_contains_read_only() {
        // 7.3, and the reason it is worth its own test: `skill_invoke` and the
        // `skill_snapshot` scope have to land on both policies or containment
        // stops holding and 7.3 loses a delegation. Map ticket 39 named it.
        let set = set();
        let standard = set.get("standard").unwrap();
        let read_only = set.get("read-only").unwrap();
        assert!(standard.contains(read_only));
        assert!(!read_only.contains(standard));
    }

    #[test]
    fn every_policy_that_reads_at_all_invokes_and_reads_its_snapshot() {
        // 7.2. `judge` reads nothing and gets neither; the other four get both,
        // and no policy may write to the snapshot.
        let set = set();
        for name in NAMES {
            let policy = set.get(name).unwrap();
            let reads = policy
                .allow
                .iter()
                .any(|a| a.capability == Capability::FileRead);
            let invokes = policy
                .allow
                .iter()
                .any(|a| a.capability == Capability::SkillInvoke);
            assert_eq!(reads, invokes, "{name} reads and invokes must agree");

            let snapshot = policy.allow.iter().any(|a| {
                a.capability == Capability::FileRead && a.scope == Some(Scope::SkillSnapshot)
            });
            let anywhere = policy
                .allow
                .iter()
                .any(|a| a.scope == Some(Scope::Anywhere));
            assert_eq!(
                snapshot || anywhere,
                reads,
                "{name} may read and cannot reach the snapshot it was linked"
            );
            // Named on no policy's `file_write`, and since ticket 40 the
            // snapshots directory is denied outright as well, so `permissive`
            // no longer reaches one through `anywhere`. This asserts the
            // narrower thing it always could: nobody wrote the scope down.
            let names_snapshot_on_write = policy.allow.iter().any(|a| {
                a.capability == Capability::FileWrite && a.scope == Some(Scope::SkillSnapshot)
            });
            assert!(
                !names_snapshot_on_write,
                "{name} names skill_snapshot on file_write, so a run may edit its own \
                 instructions by policy rather than by omission"
            );
        }
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
        let layout = crate::paths::Layout::at("/tmp/hydra-compile");
        let script = compile(
            &set,
            &Spawn {
                provider: crate::provider::Provider::Claude,
                policy: "standard",
                layout: &layout,
                worktree: &layout.worktree("r1"),
                skill_store: &layout.skills(),
                corpus: &layout.runs(),
                skill_snapshot: &layout.snapshot("abc"),
                skill_checkout: None,
                parent_git_dir: None,
                gate_script: &layout.gate_script("r1"),
                floor_lifted: false,
            },
        )
        .unwrap();
        assert!(!script.contains("standard.json"));
        assert!(!script.contains("floor.json"));
        assert!(script.contains("\\\"policy\\\":\\\"standard\\\""));
    }

    #[test]
    fn every_directory_the_layout_creates_is_inside_a_denied_path() {
        // 7.5 asks a reader to check its list against harness spec 5's table by
        // hand. This is that check: a directory added under `~/hydra` later is
        // covered the day it exists only if someone adds it to the deny list, so
        // the test fails the day one is not.
        let set = set();
        let layout = crate::paths::Layout::at("/tmp/hydra-denied");
        let spawn = Spawn {
            provider: crate::provider::Provider::Claude,
            policy: "permissive",
            layout: &layout,
            worktree: &layout.worktree("r1"),
            skill_store: &layout.skills(),
            corpus: &layout.runs(),
            skill_snapshot: &layout.snapshot("abc"),
            skill_checkout: None,
            parent_git_dir: None,
            gate_script: &layout.gate_script("r1"),
            floor_lifted: false,
        };
        let denied = denied_write_paths(&set, &spawn);
        for directory in [
            layout.runs(),
            layout.worktrees(),
            layout.spawns(),
            layout.skills(),
            layout.snapshots(),
            layout.skill_worktrees(),
            layout.pruned(),
            layout.reservations(),
        ] {
            assert!(
                denied.iter().any(|root| directory.starts_with(root)),
                "{} is a directory Hydra writes and no policy denies it",
                directory.display()
            );
        }
    }

    #[test]
    fn a_spawn_with_a_parent_git_dir_carves_out_its_private_git_directory() {
        let layout = crate::paths::Layout::at("/tmp/hydra-own-git");
        let worktree = layout.worktree("r1");
        let git_dir = PathBuf::from("/tmp/parent/.git");
        let spawn = Spawn {
            provider: crate::provider::Provider::Claude,
            policy: "standard",
            layout: &layout,
            worktree: &worktree,
            skill_store: &layout.skills(),
            corpus: &layout.runs(),
            skill_snapshot: &layout.snapshot("abc"),
            skill_checkout: None,
            parent_git_dir: Some(&git_dir),
            gate_script: &layout.gate_script("r1"),
            floor_lifted: false,
        };
        assert_eq!(
            own_write_paths(&spawn),
            vec![
                worktree,
                git_dir.join("worktrees").join("r1"),
                git_dir.join("objects")
            ]
        );
    }

    #[test]
    fn only_a_curate_spawn_carves_a_checkout_out_of_the_denials() {
        // The carve-out is per spawn and not per policy name: a spawn that was
        // given no checkout owns none, which is what makes the last row of
        // ticket 40's matrix read `protected-path` under four policies.
        let layout = crate::paths::Layout::at("/tmp/hydra-own");
        let worktree = layout.worktree("r1");
        let checkout = layout.skill_checkout("r1");
        let base = Spawn {
            provider: crate::provider::Provider::Claude,
            policy: "curate",
            layout: &layout,
            worktree: &worktree,
            skill_store: &layout.skills(),
            corpus: &layout.runs(),
            skill_snapshot: &layout.snapshot("abc"),
            skill_checkout: None,
            parent_git_dir: None,
            gate_script: &layout.gate_script("r1"),
            floor_lifted: false,
        };
        assert_eq!(own_write_paths(&base), vec![worktree.clone()]);
        let with_checkout = Spawn {
            skill_checkout: Some(&checkout),
            ..base
        };
        assert_eq!(own_write_paths(&with_checkout), vec![worktree, checkout]);
    }

    #[test]
    fn a_run_with_no_checkout_owns_nothing_at_all() {
        // 7.5 leaves the run's own spawn directory denied, and a run outside a
        // repo is scoped to a placeholder inside it. Carving the placeholder out
        // would open a hole in the one directory that stays whole, so this run
        // writes nowhere rather than under `spawn/<run-id>/no-worktree`.
        let layout = crate::paths::Layout::at("/tmp/hydra-no-checkout");
        let placeholder = layout.spawn("r1").join("no-worktree");
        let spawn = Spawn {
            provider: crate::provider::Provider::Claude,
            policy: "standard",
            layout: &layout,
            worktree: &placeholder,
            skill_store: &layout.skills(),
            corpus: &layout.runs(),
            skill_snapshot: &layout.snapshot("abc"),
            skill_checkout: None,
            parent_git_dir: None,
            gate_script: &layout.gate_script("r1"),
            floor_lifted: false,
        };
        assert!(own_write_paths(&spawn).is_empty());
    }

    #[test]
    fn only_a_cursor_spawn_carves_out_its_cursor_data_for_reads() {
        let layout = crate::paths::Layout::at("/tmp/hydra-own-read");
        let worktree = layout.worktree("r1");
        let gate = layout.gate_script("r1");
        let base = Spawn {
            provider: crate::provider::Provider::Claude,
            policy: "standard",
            layout: &layout,
            worktree: &worktree,
            skill_store: &layout.skills(),
            corpus: &layout.runs(),
            skill_snapshot: &layout.snapshot("abc"),
            skill_checkout: None,
            parent_git_dir: None,
            gate_script: &gate,
            floor_lifted: false,
        };
        assert!(own_read_paths(&base).is_empty());
        let cursor = Spawn {
            provider: crate::provider::Provider::CursorAgent,
            ..base
        };
        assert_eq!(own_read_paths(&cursor), vec![layout.cursor_data_dir("r1")]);
        let agy = Spawn {
            provider: crate::provider::Provider::Agy,
            ..cursor
        };
        assert!(own_read_paths(&agy).is_empty());
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
