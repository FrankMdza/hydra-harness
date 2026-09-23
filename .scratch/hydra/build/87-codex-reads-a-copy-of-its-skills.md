# 87 - `codex` reads a copy of its skills, and no skill root reaches a branch

Type: build
Status: resolved
Blocked by:

## What

Map ticket 71, resolved 2026-09-22. `codex` reads only by shell, and the shell check refuses any
command naming a path under `~/hydra`, where the snapshot the skill links point into lives. So today
`codex` cannot open a skill Hydra injects. The design, from `.scratch/hydra/research/71-panel-opus.md`:

1. **Copy on `codex`.** `skills::inject` (`src/skills/mod.rs` ~445) links with `symlink`. Add a
   provider fact next to `skill_root` in `src/provider/mod.rs` (~100), for example
   `reads_skills_by_shell()`, true for `Codex` only, and on it copy each admitted skill directory
   recursively from the snapshot instead of linking, skipping `hydra.toml` if the store has one, with
   files 0444 and directories 0555. The record's `skills` block keeps its meaning; say in a comment
   what `linked` means for a copy.
2. **The seatbelt denies writes to the copy.** `src/sandbox.rs` `compose` (~77): add a
   `(deny file-write* (subpath "<worktree>/.codex/skills"))` that comes after the worktree's own write
   allows, since the profile takes the last matching rule (~139). Only when the launch is `codex`.
   `tests/spawn.rs` has the pattern for a profile test that runs a command under the real profile.
3. **No skill root reaches a branch.** `own_worktree_paths` (`src/provider/mod.rs` ~205), which
   `hydra reap` passes to `commit_everything` (`src/reap.rs` ~296, `src/worktree.rs` ~96), gains the
   four skill roots of `skill_root`, so reap unstages them even if the run deleted the `.gitignore`
   inside one. This is a hole on every provider today. Test it in `tests/cleanup.rs`: a run that
   removes the skill root's `.gitignore` and plants a file there, then reap, and the branch does not
   carry it.
4. **A hash at exit.** When the run ends, the watcher (`src/watch.rs`) compares the copy under
   `.codex/skills/` with the snapshot it came from, and if any file differs, is missing or was added,
   records it. Where it is recorded is your call within the contract: read
   `docs/run-record-contract.md` for where the `skills` block lives and whether `run_end` or an
   existing event can carry a field without a contract change. If it needs a new field, stop and
   say so in your report rather than inventing one; the session decides. Test it with a stub run.

`gate.py` does not change: `cat .codex/skills/x/SKILL.md` resolves inside the run's own worktree.

## Done when

`tests/skills.rs`: on `codex` the skill is a regular read-only file with the snapshot's bytes; on the
other three it is still a link. The profile test for 2, the reap test for 3, the hash test for 4.
`cargo test --release --test skills --test spawn --test cleanup --test detached`, clippy and fmt
clean. Edits `src/` and `tests/` only. Harness spec 6.4 and `IMPLEMENTATION.md` are the session's.

## Outcome

All four items built. The ticket's item 3 said to put the four skill roots on `own_worktree_paths`,
and that turned out wrong twice over: it would have discarded a skill the run authored in a repo
that keeps its own under `.claude/skills`, and Hydra's `*` `.gitignore` already hid such a skill from
`git add -A`, so `git worktree remove` deleted it silently. Reap now reads the injected names from
the record and excludes only those. The Opus review also found that 0555 directories made
`git worktree remove` fail, that a plant inside a skill root alone made reap attempt an empty commit,
that exclude paths needed guarding against `.`, absolute, `..` and symlinked spellings, and that the
copy followed symlinks (a loop recursed forever). Item 4 needed no contract change: the note goes on
`run_end.error`, which a `codex` run with `turn.completed` reads as a fidelity note. Measured on
master with an empty `HYDRA_ROOT`: every suite green, clippy and fmt clean. The seatbelt test for the
copy was measured outside a run. Built by `cursor-agent` on Composer 2.5 under `standard`, run
01a0cc69-bbd2, 4.9M tokens; an Opus subagent reviewed and fixed it in two rounds.

A regression the suite did not catch, found by the first real `codex` launch after the push (run
01a0cc9b-5397, 2026-09-23): the launch-time collision check builds its project index skipping links,
and a copy is a real directory, so every `codex` launch was refused with `skill_collision` naming its
own copy. `Index::project_scope_without` drops the entries Hydra injected, and
`tests/skills.rs::a_codex_copy_is_not_a_collision_with_itself` shows the old index reporting the hit
and the new one not. Measured again with a real `codex` launch, which started.

