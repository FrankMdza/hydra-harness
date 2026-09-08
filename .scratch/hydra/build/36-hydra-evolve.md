# 36 - `hydra evolve`, Ouroboros rung two

Type: build
Status: resolved
Blocked by: 12, 34

## Goal

The last of harness spec 17's eleven commands. One rung-two pass: a proposer under `standard`
writes a patch to Hydra's own source in a worktree, the protected-path refusal and the
admission gate look at the diff before anyone reviews it, a panel of three `judge` runs
reviews it against the checklist and answers in one closed JSON shape, and the verdict lands
on a branch and on a harness run record. Map ticket 52 closed the three details spec 9 had
left open; build the rest as written.

## Where the design says it

- `docs/ouroboros-spec.md` section 4 in full, 4.1 to 4.12; section 2.1 for the three rules and
  the `ApprovedPatch` type that only review can construct; section 5 for the checklist and
  5.1's critical-threshold rule; section 7 for what Ouroboros writes; section 9.2 to 9.4 as
  closed by map ticket 52.
- `docs/run-record-contract.md` section 5.5, the harness run: a two-line record whose
  `command` is `evolve`, parent of the proposer and the three judges.
- `docs/harness-spec.md` 15.1 for the five protected configs a patch may not touch, and the
  `hydra evolve` row of section 17.
- `src/ouroboros/` for rung one, which already spawns a recorded run under a policy and
  commits a proposal to a branch; `src/run.rs` for spawning children with `--parent`;
  `src/policy/` for `judge`.

## The slices

**The pass record.** `hydra evolve [--model <id>] [--timeout <dur>] [--dry-run]` mints a run
id with `command: evolve`, writes the two-line harness record of contract 5.5, and every
child it spawns carries that id as `parent_run_id`.

**The proposer.** One recorded run under `standard`, task composed from spec 4.1's brief, in
a worktree of this repo. Its diff is read the way `hydra reap` reads one, against
`base_tree`. **A diff over the line cap is rejected for size before review**, recorded on the
pass with the count, and the pass ends there: the cap is one constant in `src/ouroboros/`,
named in its doc comment, and starts at 400 lines changed.

**Refused before review.** A diff touching any path of harness spec 15.1, or `policies/`, or
`config/`, is refused at admission and never reviewed, spec 4.4. The admission gate of 4.5
runs after: it compiles and tests the patched tree in the worktree, and a build or test
failure is a rejection with the output on the pass record.

**The panel.** Three `judge` runs, spec 4.6, each given the whole diff and the checklist of
spec 5 in the prompt, no skills, no MCP, as `judge` already runs. **Each answers one closed
JSON shape**: `{"verdict": "pass" | "block", "findings": [{"item": <checklist id>,
"severity": "critical" | "major" | "minor", "note": <string>}]}` and nothing else. The parser
takes the whole response as that object; any deviation, extra prose included, is recorded as
a block carrying the parse error, never retried, never read as a pass. Spec 5.1's rule turns
one critical finding into a block whatever the verdict field says. Spec 4.7's early abort on a
rate-limited judge applies.

**The verdict and the branch.** Spec 4.9: the proposer's branch keeps the patch, the pass
record carries the three answers and the verdict, and a pass constructs the `ApprovedPatch`
type of 2.1, which nothing outside the review module can build. No hot reload, spec 4.11:
nothing is merged into master by the pass, and the human merges or does not.

**Tests.** `tests/evolve.rs`, launching nothing: a synthetic proposer record over the cap is
rejected for size; a diff naming a protected path is refused at admission; three synthetic
judge answers in the closed shape produce the expected verdict, one critical finding blocks
whatever the verdict field says, and a response with one word of prose outside the object is
a block with a parse error; the harness record is two lines with the fields 5.5 fills and no
others; `ApprovedPatch` cannot be built from outside the review module (a compile-fail doc
test or a visibility assertion).

## Done when

- `hydra evolve --dry-run` on this machine composes the proposer's brief and prints it without
  spawning anything.
- The tests above pass and the rest of the suite is untouched.
- If any slice needs a decision the documents and map ticket 52 do not make, stop at that
  slice, finish the others, and say so in the report.

## Outcome

Resolved 2026-09-08. `cursor-agent` on Composer 2.5, run 01a08053-3d85 under `standard`: 5.77M
tokens, seven files, 941 lines of Rust across `evolve.rs` and `review.rs`, seven tests, fifty
minutes, in a Herdr pane. Every slice landed and the model reported no decision needed. Its
stated assumptions: the proposer runs on `claude`, the protected-path check reads `policies/`
and `config/` relative to the repo root, the cap counts insertions plus deletions from the diff
stat, and the verdict lives in the branch commit message rather than on the harness record,
which is what spec 4.9 says and what the ticket had loosely misstated. `hydra evolve --dry-run`
on this machine prints a brief naming 55 learnable runs and spawns nothing.

**The first real pass, the same evening, with the human's go-ahead and a haiku proposer.** Pass
01a08127-6f58 died before the proposer existed: the containment check refused a parent with no
policy. Pass 01a08129-de0c spawned proposer 01a0812a-ad31, 5.77M tokens, which undid the
orchestrating session's uncommitted edits seeded into its tree and called that the patch; the
tree came out equal to HEAD, the verdict commit was never made, `hydra status` called the live
pass abandoned over a millisecond-versus-microsecond start time, the admission gate blocked on
master's own `cargo fmt --check` and `clippy`, and the closed pass derived `crashed`. Five fixes
by the orchestrating session, all in `IMPLEMENTATION.md`'s rung-two section: a harness parent
bounds nothing, microseconds on the pass record, an empty verdict commit on a clean tree,
master formatted and clippy-clean, and a harness run's outcome derived from its own end. No
judge has answered yet.

**The second real pass, 01a0814a-c730.** Committed tree, haiku proposer, 5.18M tokens, a real
fifty-line patch to the three normalizers, and the admission gate blocked it on its own
misspelled clippy command, `-D warnings` without the `--`. The verdict commit landed where spec
4.9 puts it and the blocked branch pointed at the wrong HEAD; both fixed by the orchestrating
session, the branch re-pointed by hand. Still no judge has answered.

**The third pass, 01a08169-eb3b, found the deadlock.** Its proposer hit the five-hour window
with an empty diff, the pass ran the admission gate on it, and `cargo test` failed on the
contract walk, which was reading the pass's own open harness record and refusing it for having
one line where 5.5 says two. Every pass would have failed its own gate. An open harness record
is a pass still running now, contract 6's rule, and a proposer that could not run ends the pass
before any `cargo` command. Still no judge has answered.

**The fourth pass, 01a0822e-bc48.** Its haiku proposer, 8.07M tokens, invented a lowercase
`bash` and `claude` refused it with "No such tool available"; the replay called that a leak,
the leak walk failed inside the worktree, and the admission gate blocked the patch. A
provider-refused call is a limit on both the ungated and the leak path now. Four passes, four
real reading bugs, all found because the admission gate runs a suite that walks the corpus and
the proposer's own live record is in it. Still no judge has answered.
