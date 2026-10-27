# 135 - What bubblewrap does with a deny path that does not exist, and with the run's private git directory

Type: grilling
Status: resolved
Blocked by:
Opened: 2026-10-27, by the session, from the four-seat review of `2e99a25..05a9e21` (`research/panel-review-05a9e21.md`, seats `01a17e81-3770`, `01a17e81-cf35`, `01a17e82-1d83`, `01a17e82-b1a5`). Build 364 stopped at both questions because each is a decision. The human, leaving on 2026-10-27: "si necesitas decisiones usa el panel". A four-seat panel answers this one, and the synthesis is in `research/135-synthesis.md`.

## Question 1, the private git directory

SBPL on macOS lets a run write only `index` and `index.lock` in `<git>/worktrees/<run-id>/`. Under bwrap a file bind cannot be renamed over, and git updates the index by renaming `index.lock` onto `index`, so build 364 binds the directory whole and puts `HEAD`, `commondir`, `gitdir` and `logs` read-only over it. The review found that the rest of the directory is now the run's, and `hydra reap` runs git there from outside the sandbox:

- A `COMMIT_EDITMSG` symlink makes reap's `git commit` write its message through the link, onto any file the user can write. Measured by the session on 2026-10-27 with git 2.55: the target held `reap-message` afterwards.
- `MERGE_HEAD` gives reap's commit a second parent the run chose. Measured by the Opus seat.
- `locked` makes `git worktree remove` exit 128. Measured by Opus and Grok.
- `config.worktree` runs a command named by `core.fsmonitor` or a filter driver, when the shared config has `extensions.worktreeConfig=true`. Measured by Opus with the extension on.
- A fifo in place of `index` hangs `git status`. Measured by Opus.

Options:

- **A. Reap sweeps.** Before reap or harvest runs any git command in the checkout, it removes every entry of the private directory except `HEAD`, `commondir`, `gitdir`, `index` and `logs`, using `symlink_metadata`, and refuses if `index` is anything but a regular file. Same pattern as `clear_private_logs` in `src/worktree.rs` line 50. Linux only in effect; on macOS the sweep finds nothing.
- **B. Move the index.** Point `GIT_INDEX_FILE` at a file in a directory the run owns, for example `<spawn>/index/`, on both platforms, so the private git directory stays read-only whole. Changes macOS too, and every host-side git call has to set the same variable.
- **C. Pre-create and pin.** Create the names git reads, `COMMIT_EDITMSG`, `MERGE_HEAD` and the rest, before launch and bind them read-only. Creating `MERGE_HEAD` or `locked` changes git's behavior, so this needs a list of names that are safe to create empty.
- **D. A and B together**, or another answer.

## Question 2, deny paths missing at launch

`--ro-bind SRC DEST` aborts when SRC does not exist. Build 364's test listed thirteen deny paths that do not exist on a clean launch:

- the protected configs `config/env-allowlist.json` and `config/export-signoff.json` in the Hydra checkout;
- the layout directories `pruned`, `puppets`, `approvals`, `skills`, `skills-snapshots` and `skill-worktrees` under the Hydra root;
- in each worktree, `.claude/skills`, `.claude/settings.json`, `.claude/settings.local.json` and `.agents/plugins`, plus `.codex/skills` on codex.

For each kind, the options are: skip the mount; create it empty and `--ro-bind` it; bind `/dev/null`, an empty tmpfs or a read-only `{}` kept under the spawn directory over it; `--ro-bind` the nearest existing parent; or refuse the launch. Skipping lets the run create the path, which for the settings files is the hook-planting attack the denial exists for. An empty file in the worktree is committed by reap. An empty JSON file parses differently from a missing one.

The review seats answered this already, in the four memos named above:

- GPT: refuse the launch while any is absent.
- Opus: create the layout directories empty; bind `config/` read-only whole; for paths in the worktree, create the nearest missing ancestor as an empty directory and bind it read-only.
- Grok: skip the two configs, because the checkout is already read-only under `hydra_home()`; create layout directories when the root is outside that mount; empty directories for the skill and plugin roots; `/dev/null` or a read-only `{}` under the spawn directory over the settings files.
- Flash: bind `config/` whole; create the layout directories in `Layout::ensure()`; empty directories for the skill roots; `{}` for the settings files.

## Answer

Resolved 2026-10-27 by a four-seat panel, which the human delegated the decision to. The reasoning, the votes and the dissent are in `research/135-synthesis.md`.

**Question 1: A, three to one.** Before any host-side git command on a run's checkout, not only in reap, a sweep removes every entry of `<git>/worktrees/<run-id>/` except `HEAD`, `commondir`, `gitdir`, `index` and a `logs` directory. It uses `symlink_metadata` and refuses when `index` is not a regular file. Both platforms. Opus voted B, and his point that reap is not the first host git is why the sweep goes before every entry.

**Question 2.** Bind `config/` read-only whole. `Layout::ensure()` creates the six layout directories, and the profile binds them read-only. Skill and plugin roots get an empty directory bound read-only. Each settings file gets a read-only `{}` held under the spawn directory, and reap removes the empty file bwrap leaves at the destination. A deny source still missing at compose time refuses the launch by name instead of panicking. Grok dissented on the settings files; his rule, `.claude/` bound read-only whole, is the fallback if Linux shows `{}` is not read as absence.

Build tickets 366, 367 and 370.
