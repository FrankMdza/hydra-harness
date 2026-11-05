# 93 - Seven small code fixes from the second hiring panel

Type: build
Status: resolved
Blocked by:

## What

Each is under an hour and needs no decision. Findings from
`~/.claude/handoffs/2026-09-23-panel/consolidated.md`; line numbers are at `7f6a948`, so search by
the name given.

1. **G2, a test through the call site.** Build 87 made every `codex` launch refuse with
   `skill_collision` against its own copy while CI stayed green, because `tests/skills.rs` (~483-524)
   calls `Index::project_scope_without` and `collision::check` directly and nothing calls
   `run::inject_skills` (`src/run.rs` ~1195, `pub(crate)`). Add a test that goes through
   `inject_skills` for `codex` with a scratch store and worktree, the smallest way it can be reached
   (a unit test in `run.rs`, or making it `pub` behind `#[doc(hidden)]`). Revert `15deace`'s fix
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

## Outcome

Built by `cursor-agent` on composer-2.5 under `standard`, run 01a0cfa1-507d, 3.1M tokens, then
fixed by the session after an Opus review.

1. **G2.** `run::tests::inject_skills_does_not_treat_a_codex_copy_as_a_collision_with_itself`
   goes through `inject_skills` for `codex`; the agent reverted `15deace`'s `project_scope_without`
   and saw it fail with `skill_collision`. `inject_skills` reads the real home's user roots, so the
   session renamed the probe skill to `hydra-g2-inject-probe`, a name no user root will hold.
2. **G6.** `sandbox::compose` denies reads of `pruned/`. The session moved the profile-text check
   into `a_spawn_writes_a_sandbox_profile_that_denies_hydra_paths`, which runs without
   `sandbox-exec`, and kept the live read in the corpus test. Reverting the deny on master fails
   both. `SECURITY.md` names pruned records.
3. **G17.** The agent's rule matched `sk-` anywhere, so `task-management-system`,
   `risk-assessment-notes` and `flask-sqlalchemy` were redacted as keys. The session replaced it
   with `export::openai_key_spans`: a left boundary, `sk-proj-`, `sk-svcacct-` or `sk-admin-`
   followed by 20 or more of `[A-Za-z0-9_-]`, or `sk-` followed by 32 or more alphanumerics, ending
   at a non-token character. Redactor and verifier call the same function, so the verifier catches
   a skipped span and nothing the rule misses; G18 stands. Tests cover the three words, prose
   naming the prefix, and three key shapes with trailing punctuation.
4. **G19.** `tests/gate.rs::evaluate_raise_is_answered_as_a_deny` writes a non-string
   `scopes.worktree` into a compiled gate's config, `inside` raises, and the catch-all answers
   `hydra/gate-error`. No hook in `gate.py`. The session made the rewrite helper assert it replaced
   exactly one line and updated the handler's comment, which said no test drove it.
5. **G25.** `tests/corpus/mod.rs::seatbelt_exec_or_skip` prints a skip, panics when `CI` is set,
   and, added by the session, skips inside a seatbelt, where nested `sandbox-exec` fails and every
   `refused(..)` assertion would pass on a probe that never ran.
6. **G15.** `src/outcome.rs`'s module comment says the harvested arm reads the exit code.
7. **G11.** The agent wrote "four providers" in four places. Two were wrong in a new way: `codex`
   also reads skills by shell, so it is three of four in `gate.py` twice, and the exit-0 and
   hook-snapshot claims were measured on three providers and say so now, `codex` unmeasured.

Measured on master: lib 200, spawn 32, gate 97, export 21, skills 27 passed; fmt and clippy clean.
