# Panel 71, opus

## 1. Recommendation

Option 1 with the holes closed: on `codex` only, copy the snapshot's admitted skills into `<worktree>/.codex/skills/` as plain files, deny writes to that directory in the seatbelt profile, and put the skill roots on reap's exclude list, leaving the gate untouched.

## 2. Mechanism

- `src/skills/mod.rs:445`, `inject` links with `symlink`. Branch on a new `Provider::reads_skills_by_shell()` (true for `Codex`, next to `skill_root` at `src/provider/mod.rs:100`). Copy the skill directory recursively instead, skip `hydra.toml`, and set 0444/0555. About 35 lines. `linked` in the record keeps its meaning.
- `src/sandbox.rs:77`, `compose`: append `(deny file-write* (subpath "<worktree>/.codex/skills"))` after the own-write allows at 104-110. SBPL takes the last matching rule (`src/sandbox.rs:139-142`), so the deny wins over the worktree allow. About 8 lines.
- `src/reap.rs:296` passes `own_worktree_paths()` (`src/provider/mod.rs:205`), which lists no skill root. Add the four roots so `commit_everything` (`src/worktree.rs:96-110`) unstages them even if the run deleted the `.gitignore`. About 5 lines.
- Spec 6.4's table gets a `codex` row that says "copied". Nothing changes in `gate.py`. `cat .codex/skills/x/SKILL.md` now resolves inside the own worktree, so `protected_root` returns None for it (`src/policy/gate.py:325`).

About 50 lines plus tests in `tests/skills.rs`.

## 3. Cost per provider

- `claude`, `cursor-agent`: none. They keep links.
- `agy`: none from this change. It still reads through `view_file`, which the gate scopes into `skill_snapshot`. One preamble line should say so, because its `run_command` hits the same refusal.
- `codex`: a few KB copied per run. Where `wraps_launches()` is false, Codex runs `workspace-write` (`src/provider/codex.rs:81-87`) and only the file mode protects the copy, and the owner can undo that.

## 4. What a hostile run gains

Close to nothing. Today the gate already lets a `codex` run `rm -rf .codex/skills` and write its own `SKILL.md` there, because the token resolves into the own worktree (`gate.py:325`, `protected_root` at 218). Skill roots are write-denied to file tools only (`gate.py:819`, `src/policy/mod.rs:743`), and the seatbelt denies nothing inside the worktree (`src/sandbox.rs:144-149`). The link protects the shared per-sha snapshot, and that stays denied (`src/policy/mod.rs:589-590`). The copy adds one thing. A run on an unwrapped launch can `chmod` and edit its own copy in place, and its record still names `store_sha`. That is self-deception, not reach. The reap exclude removes a channel that exists today: delete `.gitignore`, and the planted skill lands on the branch.

## 5. Strongest argument against

On `codex`, "the bytes the run read are the bytes at `store_sha`" stops being true by construction and becomes true only by assertion. With links, contract 7.7 can re-derive the answer from the sha. With a copy it cannot, unless `watch.rs` hashes the copy against the snapshot at exit. I would add that check once a run is seen to tamper, not before.

## 6. The four options

1. Copy. Right, but as written it leans on reap to drop files. The kernel deny and the exclude list are what make it hold.
2. Shell carve-out. No. Even an argv allowlist of read-only binaries makes the gate judge what a command does, which 7.4 and 7.5 refuse to do. And the raw pass (`gate.py:312`) would still refuse the absolute spelling.
3. Do not inject on `codex`. Honest, but it drops the store from a provider that can use it, over a problem that costs 50 lines.
4. Tell the run. Run `01a0cc2a-f20b` already shows the result: the agent stopped rather than go on without the skill.
