# 73 - What rung two may edit: a denylist that keeps missing, or an allowlist

Type: grilling
Status: resolved
Blocked by:
Opened: 2026-09-23, by the session, from findings G5 (`claude`) and G44 (`codex`) of the second
hiring panel. Bears on build ticket 80.

## Question

Rung two refuses a patch that touches a protected path before any judge sees it
(`docs/rungs-spec.md` 4.4). The code half of that list is `PROTECTED_CODE_PATHS` at
`src/rungs/evolve.rs:448-465`, a denylist. Its own comment calls `src/sandbox.rs` "the third time a
mechanism landed outside this list", and it still misses:

- `src/pty.rs` and `src/watch.rs`, which carry the environment clear of build ticket 88 (G1);
- `src/paths.rs`, which carries the non-ASCII root refusal of build ticket 78;
- `src/run.rs`, the launch sequence that decides the policy, the seatbelt and the gate a child gets.

G44 is the same shape one directory over. `tests/corpus/` is protected, but it holds only `mod.rs`
and `README.md`. Eight of the fourteen rows in `tests/corpus/README.md` name probes that live in
`tests/gate.rs`, `tests/spawn.rs`, `tests/mcp.rs` and `tests/delegation.rs`, and A12's is a unit
test in `src/face.rs`. None of those files is protected. The comment at `evolve.rs:462-463`, "a
proposer must not delete or rewrite the probes", is not what the list does. A proposer can weaken
a probe and the gate it guards in one patch, and admission then runs the weakened probe.

Harness spec 15.2 owns the rows by role; `evolve.rs` maps them to files. The options:

1. **An allowlist.** Rung two may edit a short named set (for example `src/provider/`,
   `src/normalize/`, `src/preamble.rs`, `config/preamble.md`) and everything else, `tests/`
   included, is refused. A new mechanism file is protected on the day it lands. Proposers get
   narrower, and every widening is a human edit to a protected file.
2. **A wider denylist, and the probes moved.** Add the four files to `PROTECTED_CODE_PATHS` and move
   the eight probes into `tests/corpus/`. Cheaper, and the fourth mechanism outside the list repeats
   the failure.
3. **A wider denylist and all of `tests/`.** A proposer can no longer add or change a test, so a
   patch arrives with no test of its own.

The human leans to 1 and asked a panel of four readers to decide, and to say whether an option
not listed here is better (`.scratch/hydra/research/73-panel-*.md`).

## Answer

Resolved 2026-09-23 with the human, after a panel of four readers
(`.scratch/hydra/research/73-panel-*.md`). All four wanted a new file refused by default. `codex`,
composer and `agy` chose option 1, `codex` with an even narrower list. Opus showed that option 1's
example set is unsafe and would have refused five of the six real proposer patches, including pass
10, the only one ever merged: `src/provider/claude.rs` adds the `--settings` flag that loads the
gate. It proposed a fourth option, and the human chose it:

1. **The human names what a pass may edit.** `hydra evolve` takes a repeatable `--may-edit <path>`
   beside `--task`. Without it, a short default set applies. A patch touching any path not named is
   refused before any judge, like a protected path today. The pass record carries the list.
2. **A floor no name can lift.** Today's protected rows, plus `src/pty.rs`, `src/watch.rs`,
   `src/paths.rs`, `Cargo.toml`, `Cargo.lock`, any `build.rs` and `.cargo/`. Naming a floor path
   refuses the pass before it starts. `src/run.rs` stays off the floor, because harness spec 15.2 and
   map ticket 60 keep it editable on purpose; it is refused unless named.
3. **Build ticket 80 replaces moving the probes.** A patch that edits or deletes a file under
   `tests/` that exists at `base_tree` is refused; a new test file is allowed.

Opus's other findings go with it: `CLAUDE.md` and `IMPLEMENTATION.md` are refused unless named,
which the default-refusal already gives, and a new `build.rs`, `.cargo/` or dependency is on the
floor because the human's own `cargo build` runs it after the merge. What stays open, stated in
spec 4.4: a named file is fully editable, and a unit-test probe inside a named file, A12's in
`src/face.rs`, can be weakened with the code it guards. Build tickets 80 and 96.
