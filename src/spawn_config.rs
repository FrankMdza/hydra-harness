//! The hook-config writer: compiles the gate and writes what the provider reads.
//!
//! Split out of [`crate::run`] so harness spec 15.2's row 4 is a file admission
//! can refuse. [`crate::run`] still re-exports this module's public items so
//! call sites keep their path.

use std::path::PathBuf;

use anyhow::{Context, Result};

use crate::paths::Layout;
use crate::provider::{Provider, agy, claude, codex, cursor};

/// What one spawn writes for the gate: the script, and the config that delivers it.
#[derive(Debug)]
pub struct SpawnFiles {
    /// The compiled gate, `0o700`, in `~/hydra/spawn/<run-id>/`.
    pub gate: PathBuf,
    /// The sha256 of that script, the same one the hook config checks against
    /// at every call. Contract 5.1 puts it on `run_start`.
    pub gate_sha: String,
    /// The hook config the provider reads. `~/hydra/spawn/<run-id>/settings.json`
    /// on `claude`, passed on `--settings`; `<worktree>/.cursor/hooks.json` on
    /// `cursor-agent`, read from cwd (6.6's table).
    pub hook_config: PathBuf,
    /// The absolute paths the gate was compiled against, keyed by scope name,
    /// which contract 5.1 puts on `run_start` as `scopes`. It is returned rather
    /// than recomputed by the caller because the gate and the record have to
    /// name the same paths, and one function is how that is made true.
    pub scopes: std::collections::BTreeMap<String, String>,
    /// The seatbelt profile beside the gate on macOS, and absent elsewhere.
    pub sandbox_profile: Option<PathBuf>,
    /// The sha256 of that profile for `run_start.sandbox`, absent on non-macOS.
    pub sandbox_sha: Option<String>,
}

/// Compiles the run's gate and writes the config that delivers it.
///
/// Split out from [`crate::run::run`] because it is the half of the launch that
/// can be checked without spending a subscription: `tests/spawn.rs` calls it
/// against a temporary layout and reads what it wrote. The other half needs a
/// provider.
///
/// **Real files, never links, rewritten at every spawn** (6.6). The spawn directory
/// is per run and the script is created inside it, so a second run in the same
/// directory is not a case that can arise. Cursor's config goes in the worktree,
/// which is per run too, and a checkout that already holds one is refused rather
/// than overwritten: `run::worktree_preconditions` checks the repo it was seeded
/// from and this is the second line.
#[allow(clippy::too_many_arguments)]
pub fn write_spawn_config(
    layout: &Layout,
    run_id: &str,
    policies: &crate::policy::PolicySet,
    provider: Provider,
    policy: &str,
    worktree: Option<&std::path::Path>,
    repo: Option<&std::path::Path>,
    store_sha: Option<&str>,
    floor_lifted: bool,
) -> Result<SpawnFiles> {
    write_spawn_config_for_launch(
        layout,
        run_id,
        policies,
        provider,
        policy,
        worktree,
        repo,
        store_sha,
        floor_lifted,
        false,
    )
}

/// Writes a gated interactive launch's gate and profile.
#[allow(clippy::too_many_arguments)]
pub fn write_interactive_spawn_config(
    layout: &Layout,
    run_id: &str,
    policies: &crate::policy::PolicySet,
    provider: Provider,
    policy: &str,
    worktree: Option<&std::path::Path>,
    repo: Option<&std::path::Path>,
    store_sha: Option<&str>,
    floor_lifted: bool,
) -> Result<SpawnFiles> {
    write_spawn_config_for_launch(
        layout,
        run_id,
        policies,
        provider,
        policy,
        worktree,
        repo,
        store_sha,
        floor_lifted,
        true,
    )
}

#[allow(clippy::too_many_arguments)]
fn write_spawn_config_for_launch(
    layout: &Layout,
    run_id: &str,
    policies: &crate::policy::PolicySet,
    provider: Provider,
    policy: &str,
    worktree: Option<&std::path::Path>,
    repo: Option<&std::path::Path>,
    store_sha: Option<&str>,
    floor_lifted: bool,
    gated_interactive: bool,
) -> Result<SpawnFiles> {
    // The gate is compiled against the path it will really live at, because it
    // denies writes to itself and a placeholder would deny the wrong file.
    let gate = layout.gate_script(run_id);
    let hook_config = match provider {
        Provider::Claude => layout.settings_file(run_id),
        Provider::CursorAgent => cursor::claim(worktree, "hook config", cursor::hooks_file)?,
        Provider::Agy => agy::claim(worktree, "hook config", agy::hooks_file)?,
        Provider::Codex => codex::claim(worktree, "hook config", codex::hooks_file)?,
    };

    // With no repo there is no worktree, and the child runs where Hydra was
    // invoked. Scoping the `worktree` capability to that directory would hand the
    // run write access to whatever the human happened to be standing in, so the
    // scope points at a path inside the spawn directory that nothing creates. A
    // `standard` run outside a repo has none.
    let no_worktree = layout.spawn(run_id).join("no-worktree");

    // A curate pass writes into its own checkout of the store and every other
    // policy is given none: rungs spec 3.6 keeps the curator out of
    // `~/hydra/skills/` because the next launch commits that tree before pinning
    // it, and two writers to one working tree is the collision. The path is a
    // function of the run id alone, because contract 7.7's read-time
    // re-evaluation has to reach the same answer from the record.
    let skill_checkout = (policy == "curate").then(|| layout.skill_checkout(run_id));

    // 7.5 denies the parent repo's `.git` to every policy, and this is the one
    // denied path that is not under `~/hydra`. The common directory rather than
    // `<repo>/.git`, because when a human launches from inside a linked worktree
    // that name is a file and the directory holding every branch is elsewhere. A
    // run outside a repo has no parent to protect, which is the same `None` that
    // leaves it without a worktree.
    let parent_git_dir = repo.and_then(crate::worktree::git_common_dir);

    // 6.4 and 7.2: the run's links resolve into its own snapshot, so a read
    // through one lands inside this and a write to the same path lands nowhere,
    // because no policy puts this scope on `file_write`. A run with no store sha
    // was linked nothing and gets a path inside its spawn directory that nothing
    // creates, the same shape `no_worktree` has above.
    let skill_snapshot = layout.skill_snapshot_scope(run_id, store_sha);

    let spawn = crate::policy::Spawn {
        provider,
        policy,
        layout,
        worktree: worktree.unwrap_or(&no_worktree),
        skill_store: &layout.skills(),
        corpus: &layout.runs(),
        skill_snapshot: &skill_snapshot,
        skill_checkout: skill_checkout.as_deref(),
        parent_git_dir: parent_git_dir.as_deref(),
        gate_script: &gate,
        floor_lifted,
    };
    // Contract 5.1's `scopes`, taken off the spawn the gate is compiled from
    // rather than composed again here. The two cannot disagree, which is what
    // map ticket 42 asked for after two correct builds resolved one scope two
    // ways and no field on the record told them apart.
    let scopes = spawn.recorded_scopes();
    crate::policy::write_gate(policies, &spawn)?;

    let (sandbox_profile, sandbox_sha) = if crate::sandbox::wraps_launches() {
        let profile = crate::sandbox::profile_path(&layout.spawn(run_id));
        let sha = crate::sandbox::write_profile_for_launch(
            policies,
            &spawn,
            &profile,
            gated_interactive,
        )?;
        (Some(profile), Some(sha))
    } else {
        (None, None)
    };

    // Read back rather than hashed in memory, so the sha is of the bytes that
    // reached the disk. The hook config carries it and the provider snapshots the
    // config at process start, which is what makes a mid-run rewrite of the script
    // fail closed.
    let written =
        std::fs::read(&gate).with_context(|| format!("reading back {}", gate.display()))?;
    let gate_sha = {
        use crate::hex::hex_lower;
        use sha2::{Digest, Sha256};
        hex_lower(&Sha256::digest(&written))
    };

    let config = match provider {
        Provider::Claude => claude::settings(&gate, &gate_sha),
        Provider::CursorAgent => cursor::hooks_config(&gate, &gate_sha),
        Provider::Agy => agy::hooks_config(&gate, &gate_sha),
        Provider::Codex => codex::hooks_config(&gate, &gate_sha),
    };
    // `codex` takes the hook on the command line as a file, because it reads no
    // project hook file inside a git worktree; `provider/codex.rs` has the
    // measurement. The file is the same Python the other dialects carry inline.
    if provider == Provider::Codex {
        let hook = codex::hook_file(layout, run_id);
        std::fs::write(
            &hook,
            crate::policy::hook_source(&gate, &gate_sha, provider.dialect()),
        )
        .with_context(|| format!("writing {}", hook.display()))?;
    }
    std::fs::write(&hook_config, serde_json::to_string_pretty(&config)?)
        .with_context(|| format!("writing {}", hook_config.display()))?;

    Ok(SpawnFiles {
        gate,
        gate_sha,
        hook_config,
        scopes,
        sandbox_profile,
        sandbox_sha,
    })
}
