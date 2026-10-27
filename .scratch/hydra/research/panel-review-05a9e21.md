# Review panel on 2e99a25..05a9e21

You are one seat on a four-seat review panel. Read only. Do not edit, create or delete any file, and do not commit. Your seat name is in your task line.

## What changed

Run `git log --oneline 2e99a25..05a9e21` and `git diff 2e99a25..05a9e21 -- src tests .github SECURITY.md IMPLEMENTATION.md`. Three build tickets:

- 362, `src/policy/gate.py` and `tests/gate.rs`: the `git push` floor reads `push.default` and `remote.<name>.push` from the repository config when a push names no refspec.
- 363, `src/rungs/evolve.rs` and `.github/workflows/ci.yml`: admission never runs proposer code bare on Linux, and Linux CI runs `sandbox::tests::`.
- 364, `src/sandbox.rs`, `src/spawn_config.rs` and `src/interactive.rs`: under bubblewrap, allow sources are created before the profile is written, the seat socket is bound only on a gated interactive launch, and the private git directory `<git>/worktrees/<run-id>` is bound whole with `HEAD`, `commondir`, `gitdir` and `logs` read-only over it.

The tickets are `.scratch/hydra/build/362-*.md`, `363-*.md` and `364-*.md`. Their `## Outcome` says what the builder measured. No Linux host has run any of this.

## Questions

1. Does each change do what its ticket says? Find a concrete input that gets past it. For 362, think of config the gate does not parse: `include.path`, `includeIf`, `config.worktree`, a value continued with a trailing backslash, a section written as `[remote.origin]`, a refspec with a wildcard.
2. 364 binds the whole private git directory writable. Before, only `index` and `index.lock` were writable. What else lives in `<git>/worktrees/<run-id>/` that git or `hydra reap` reads from outside the sandbox afterwards, such as `ORIG_HEAD`, `FETCH_HEAD`, `config.worktree`, `refs/`, `locked`, or a symlink planted in place of `index`? Is any of them a way out? Name the git behavior you rely on.
3. A write-deny path that does not exist at launch cannot be `--ro-bind`'ed, so bwrap aborts. The 364 Outcome lists thirteen such paths. Give your recommendation for what bwrap should do with each kind: skip it, create it empty and bind it read-only, bind `/dev/null` or an empty tmpfs over it, deny its parent, or refuse the launch. Say what each option breaks. This is a decision for the human; give a recommendation, not a fix.
4. Anything in this range that makes `SECURITY.md`, `README.md`, `CHANGELOG.md` or `IMPLEMENTATION.md` false.

## Answer

At most 800 words. Each finding gets a severity, a file and line, and the input that shows it. Say which findings you checked by running something and which by reading. End with `## Pending from me`, `## Changed` and `## Found`.
