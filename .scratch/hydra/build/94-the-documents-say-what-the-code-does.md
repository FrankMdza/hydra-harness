# 94 - README, SECURITY, IMPLEMENTATION and CONTRIBUTING say what the code does

Type: build
Status: resolved
Blocked by:

## What

Documentation findings of the second hiring panel, 2026-09-23. Every one was checked against the
code; the fix is a sentence or a paragraph each. Line numbers are at `7f6a948` and may have moved,
so find each by its words. **Before you change a sentence, open the code it describes and confirm
the new sentence is true.** A corrected sentence that is wrong in a new way is worse than the old one.

Edit only `README.md`, `SECURITY.md`, `IMPLEMENTATION.md`, `CONTRIBUTING.md` and
`tests/corpus/README.md`. Do not edit `docs/`, `config/`, `policies/`, `.scratch/` or `CLAUDE.md`;
the session does those. Do not run `cargo` at all: nothing here needs a build.

`README.md`
1. **G7.** ~246 says 71 decision tickets, 68 resolved. Replace it with what
   `python3 .scratch/hydra/frontier.py | tail -1` and `python3 .scratch/hydra/frontier.py build | tail -1`
   print when you run them, and say in your report what they printed.
2. **G13.** The authorship table (~216-236) stops at build ticket 76. Extend it to 88 from each
   ticket's `## Outcome` in `.scratch/hydra/build/77-*.md` to `88-*.md` (who wrote it: which
   provider and model, or "the session by hand"). Where an outcome records a subagent fixing the
   agent's diff before it was applied (81 and 87 do), say so in one sentence under the table rather
   than claiming the session applied it by hand unchanged.
3. **G45.** ~21-24 says a run whose tools failed is `failed`. `src/outcome.rs` ~184-267 decides from
   the terminator's `is_error` or `agy`'s `status`, a harvested run with no terminator from the exit
   code, and one with neither is `completed`; individual tool results do not decide it. Say that.
4. **G46.** The roadmap (~185-190) says `codex` resume is refused. It is supported since build ticket
   71: `src/provider/codex.rs` `resume_args`, `tests/resume.rs` ~158. Remove or correct the line.

`SECURITY.md`
5. **G48.** ~136-139 lists a payload whose `cwd` is not a string as denied. `src/policy/gate.py`
   ~759-761 falls back to the worktree and `tests/gate.rs` ~2332-2348 asserts the call is allowed.
   Correct that one shape.
6. **G28.** Add one sentence: a shell command `shlex` cannot split skips the token pass
   (`gate.py` `split_segments`, ~316-321) and is checked by the pattern floor alone, which
   `--allow-floor-escape` turns off.
7. **G32.** Add one sentence: a plain `--interactive` run compiles no gate; only the gated
   interactive `claude` launch has one.

`IMPLEMENTATION.md`
8. **G8.** ~3370-3391 says the cap is a reservation and then, eleven lines on, that it is "checked
   and not held". The second paragraph predates build ticket 81; rewrite it to match `src/live.rs`
   `reserve`.
9. **G26.** ~3379-3380 says interactive launches are not counted against the cap. They reserve
   nothing, but they are counted: `src/interactive.rs` ~270-309 writes line 0 with the child's pid
   right after the spawn, and `live::reserve` counts every live view. The uncounted window is between
   the spawn and line 0. Say that.
10. **G9.** ~2165-2171 prices replay at a Python process per tool call. `src/policy/replay.rs`
    ~19-21 says one process per record, one per call only when the batch fails. Match it.
11. **G10.** ~3735-3748: since build ticket 88, `HYDRA_ROOT` never reaches a recorded child, which
    is true again; the half saying a run can move its children's records is stale, because
    `src/run.rs` ~411-424 refuses the variable inside a seatbelt. Fix that half.
12. **G47.** ~3037-3061, the rung-two paragraph, names three judges (four since build ticket 61,
    `codex`), and puts the hook-config writer in `src/run.rs`; it is `src/spawn_config.rs`, on
    `PROTECTED_CODE_PATHS` in `src/rungs/evolve.rs`, so the sentence calling it an open gap
    describes one that closed.

`CONTRIBUTING.md`
13. **G39.** One line pointing at the section "Nothing here is enforced" in `CLAUDE.md`: an
    outside contributor working on Hydra's source is not gated by it.

`tests/corpus/README.md`
14. **G12.** ~13-20 cite `gate.py` and `tests/gate.rs` by line numbers that drifted (`gate.py:124` is
    `fold_ascii`, not `realpath`). Replace line numbers with function and test names throughout, and
    point rows A7 and A8 also at `tests/pty_tee.rs::the_child_gets_the_environment_it_is_handed_and_nothing_else`.

## Done when

All fourteen are done, and your report lists each with the file and the code you checked it
against. `git diff --stat` touches only the five files above.

## Outcome

Built by `agy` on gemini-3.1-pro-high under `standard`, run 01a0cf8d-1e06, 3.1M tokens, no cargo,
then fixed by the session after an Opus review, which found eight of the fourteen right and six
wrong or partly done. What the session corrected: G7, where the counts were wrong the day they were
written (71 of 73 for a map at 73 of 73) and credited `map.md` with the build tickets, now dated and
pointing at `frontier.py`; G13, where the heading still said 01 to 76, the `codex` row named no
model, and "several, including 81 and 87" undercounted seven tickets; G45, which missed the streamed
run that stops without a terminator; G28, which said the floor alone checks an unparseable command
while the raw prefix pass reads it first; G8, whose race example had the arithmetic wrong; G9,
where the agent replaced build 86's measured numbers with filler; G10, which lost the Linux caveat;
G12, where three rows pointed at `decide_payload`, a wrapper, instead of `evaluate` and
`Capability::OpaqueShell`. The authorship table now runs to 94. No em dash was introduced.
