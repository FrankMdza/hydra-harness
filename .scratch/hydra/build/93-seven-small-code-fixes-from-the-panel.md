# 93 - Seven small code fixes from the second hiring panel

Type: build
Status: open
Blocked by:

## What

Each is under an hour and needs no decision. Findings from
`~/.claude/handoffs/2026-09-23-panel/consolidated.md`; line numbers are at `fad1b98`, so search by
the name given.

1. **G2, a test through the call site.** Build 87 made every `codex` launch refuse with
   `skill_collision` against its own copy while CI stayed green, because `tests/skills.rs` (~483-524)
   calls `Index::project_scope_without` and `collision::check` directly and nothing calls
   `run::inject_skills` (`src/run.rs` ~1195, `pub(crate)`). Add a test that goes through
   `inject_skills` for `codex` with a scratch store and worktree, the smallest way it can be reached
   (a unit test in `run.rs`, or making it `pub` behind `#[doc(hidden)]`). Revert `d264867`'s fix
   locally and say in the Outcome that the test then fails.
2. **G6, `pruned/` read-denied.** `sandbox::compose` (`src/sandbox.rs` ~86-105) emits
   `deny file-read*` for the corpus `runs/` and `spawns()`, and nothing for `Layout::pruned()`
   (`src/paths.rs` ~254), a sibling of `runs/` holding moved records. Deny it the same way, and add
   it to the existing profile test in `tests/spawn.rs` that checks the corpus read denial.
3. **G17, an OpenAI key class.** `src/export.rs` redacts Google, Anthropic, GitHub, Google OAuth and
   PEM (~495-501) and the verifier checks the same prefixes (~638-642). Add OpenAI's `sk-proj-` and
   legacy `sk-` keys to both, with a length floor so `sk-` in prose does not fire, and a test in
   `tests/export.rs` beside the existing class tests.
4. **G19, the gate's catch-all driven.** `src/policy/gate.py` ~961-975 is the fail-closed handler
   whose comment says no test drives it. Find a payload that makes `evaluate` raise, or a policy
   compiled from a fixture that makes it raise, and add a test in `tests/gate.rs` that the gate
   denies. If the only way to reach it is a test hook inside `gate.py`, stop and say so in the
   Outcome instead of adding one.
5. **G25, the seatbelt probe's skip seen.** `tests/spawn.rs` ~1332-1334 returns silently when
   `sandbox-exec` is absent. Print that it skipped, and fail instead when `CI` is set, since CI runs
   macOS.
6. **G15, the outcome comment.** `src/outcome.rs` ~13 says nothing reads the exit status; the
   harvested interactive arm (~185-205) reads `exit_code`. Correct the comment.
7. **G11, "three providers".** In `src/outcome.rs` (~13, ~456) and `src/policy/gate.py` (~16, ~364,
   ~816) the phrase describes today and is wrong since `codex`. Leave `src/skills/mod.rs` ~414, which
   is history. `docs/` is the session's.

## Done when

Each item has its test or edit, the suite passes, and the Outcome lists the seven with a line each.

## Notes for the run

Tests take a scratch `Layout`, never `Layout::discover()`. Ignore
`no_record_on_this_machine_shows_a_gate_that_leaked_or_stayed_silent`. Run `cargo fmt --check` and
`cargo clippy --all-targets -- -D warnings` before you stop.
