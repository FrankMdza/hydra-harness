# 73 - What rung two may edit: a denylist that keeps missing, or an allowlist

Type: grilling
Status: claimed
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
