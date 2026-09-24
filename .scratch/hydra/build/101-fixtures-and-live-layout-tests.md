# 101 - A fixture corpus for CI, a codex dead-hook probe, and tests with a live layout

Type: build
Status: resolved
Blocked by: 

## What

The third hiring panel, 2026-09-23. Implement `~/.claude/handoffs/2026-09-23-panel3/consolidated.md` "Before publishing" item 17 and 18, findings H14, H15, H36 and H18.
Read those findings in its Findings table first; each names the lines and what was measured.
Files: tests/, a new fixtures directory, src/reap.rs for the typed line-0 view.

Documentation changes go in the Outcome as proposed sentences, not into `SECURITY.md`,
`IMPLEMENTATION.md` or `README.md`: several tickets touch those files at once and the session
applies them together, build ticket 102.

## Done when

CI's record walks read a sanitized, version-labelled fixture corpus kept apart from live measurements; an automated probe shows what codex does when its hook dies; `review_blocked`'s refusal wiring and G42's tree half are driven with a scratch live layout; reap reads line 0 through a typed view. Every new test fails before the change, checked by reverting it. `cargo fmt --check` and
`cargo clippy --all-targets -- -D warnings` are clean and the suite passes outside a seatbelt.

## Outcome

Built by a Claude Code session working on Hydra's source, not through `hydra run`. No provider
was launched.

**H14, the fixture corpus.** `tests/fixtures/records/` holds ten synthetic records in five
directories named by the binary that wrote the shape: `claude@2.1.261` (six: a read-only `Write`
denial, ticket 06's refused-and-allowed `Bash` pair with `scopes`, a pre-gate run, a 429 on the
terminator, a launch refused with `policy_narrowing`, and an open run with no `run_end`),
`cursor-agent@2026.09.02-c22c1a3`, `agy@1.1.26`, `codex@0.154.0` and `harness` (an `evolve`
pass). The ignored test `write_the_fixture_corpus` in the new `tests/fixture_corpus.rs` writes them
from the builders in `tests/common`, so no byte comes from `~/hydra/runs`. Every path is under
`/fixture/hydra`, which `common::fixture_layout()` reads them against, every run id starts
`00000000-0000-7000-8000-`, a v7 timestamp of zero that `Uuid::now_v7` never mints, and every
gated record carries a fixed `policy_sha` that is not the shipped set's, so the readers meet a set
that has moved, as they do on real records. Three tests guard the corpus: the directory label
matches line 0's provider and version, every routed provider and the harness are present, and no
line carries a secret the scanner flags, a home or temp path, a live run id or an absolute path
outside the fixture root.

Each live walk now has a fixture twin with the same assertions, the body moved into a function
that takes the records and a reader: `every_fixture_meets_the_contract`,
`every_fixture_normalizes`, `the_fixture_write_denial_derives_to_a_deny_under_capability`,
`no_fixture_shows_a_gate_that_leaked_or_stayed_silent`, `normalizing_the_fixtures_writes_nothing`,
`the_fixture_shell_denial_and_the_call_beside_it_derive_apart`, `every_fixture_derives_an_outcome`,
`the_fixture_write_denial_is_the_only_gated_fixture_on_the_stream` and
`a_fixture_is_unknown_only_where_a_result_is_unclassified`. The live walks read what they read
before and passed against this machine's corpus. `.github/workflows/ci.yml` gains a `fixture count`
step that puts `checked 10 fixture record(s)` on the job summary beside the live count of zero.

**H15, the `codex` dead-hook probe.** `tests/drift.rs::a_codex_hook_that_dies_without_answering_lets_the_call_run`
launches `codex exec` with `codex::argv`, hook override included, in a scratch repository, with a
hook that writes a marker and then raises. It asks for `touch probe-ran` and asserts the marker
exists, so the hook fired, and that `probe-ran` exists, so the call ran. A `codex` that starts
failing closed fails the test with its version and a pointer to `SECURITY.md` and `codex.rs`. It
costs one short turn, so it runs only under `HYDRA_DRIFT_PAID=1` with `codex` on the `PATH`.
**It has not been run.** The session was told not to spend `codex` quota, so the assertion is the
2026-09-21 hand measurement written down, and the first paid run is what measures it.

**H36, the live layout.** Two seams in `src/rungs/evolve.rs`, both taking the judge panel as a
closure, `JudgePanel`:

- `land_proposer_patch(&Landing, &RunResult, panel)` is everything `evolve()` did after the
  proposer ended, moved out unchanged: the read, the size cap, the refusals, the rate-limit check,
  admission on the copy, the panel and the commit. `evolve()` calls it with `run_judge_panel`.
- `review_blocked_in(layout, policies, repo, pass_id, evidence, panel)` is `--review` after
  discovery. `read_blocked_patch` and `reviewed_may_edit` now read records against the layout
  they are given through `normalize::read_with`, not `read_file`, which discovered the real
  `~/hydra`.

`tests/evolve.rs::a_fresh_review_refuses_before_the_panel_what_its_pass_could_not_edit` drives five
blocked passes on a scratch layout: `Cargo.toml` refused under the default set and under a recorded
set naming it, `src/run.rs` refused unnamed and admitted when the pass record names it, and
`src/show.rs` admitted by default. A refused one never reaches the panel closure and writes no
record; an admitted one reaches it once, the prompt carries the path and the evidence, and the
offered branch has the blocked branch's tree. Reverted two ways, both failed: with the refusal
disabled, "floor: a review of a patch on Cargo.toml was not refused"; with the recorded set
ignored, "named: ... src/run.rs is outside --may-edit".

`tests/evolve.rs::a_pass_offers_the_tree_it_read_whatever_the_checkout_holds_at_the_verdict` runs
`land_proposer_patch` on a scratch layout with a real checkout, a real proposer profile and real
admission on a small crate (it fails admission when the patch is unformatted, checked). The panel
closure stands in for a leftover proposer process: it rewrites `src/show.rs` and plants
`src/policy/guard.rs` after admission, just before the commit. The offered branch carries the read
tree, the proposer's `src/show.rs` and no planted file. With `commit_pass_branch` reverted to
commit the checkout's current tree it failed on the tree assertion. That is G42's tree half run,
where build 89 had it reasoned.

**H18, a typed line 0 in reap.** `src/reap.rs` reads line 0 once per run into `LineZero`, a serde
struct with the three fields reap acts on: `provider`, `hydra.skills.linked` and
`hydra.scopes.parent_git_dir`. A line that does not parse into it counts as no line, and every
skill root is held back, as with no record. `CheckoutGit::for_run_from` takes the parent git
directory as a path, and `for_run` delegates to it, so the four other callers are untouched.
`tests/cleanup.rs::a_line_zero_reap_cannot_read_holds_every_root_back` injects a `codex` skill and
writes line 0 with `skills.linked` as a string, then with provider `hermes`. Before the change the
`Value` reading took both as "nothing was injected" and committed `.codex/skills/injected/SKILL.md`
to the branch; it failed on that, and passes now.

**Fixture walks checked by mutation.** Before this change there was no fixture corpus, so the walks
had nothing to fail on. Mutating a fixture made each family fail: a dropped line failed
`every_fixture_meets_the_contract` ("seq is 3, so a line was lost"), a `/Users/` cwd failed the
sanitization test, a record moved to the wrong directory failed the label test and the coverage
test, a corpus write pointed elsewhere failed the shell-pair walk (0 pairs), and the read-only
fixture relabelled `standard` failed `the_fixture_write_denial_is_the_only_gated_fixture_on_the_stream`.
The corpus was rewritten by the generator afterwards.

**What turned out false or stays open.**

- The fixture leak walk can catch a reader that invents a leak. It cannot catch a real leak.
  Build ticket 49 reports a leak only on a record whose `gate_sha` names the gate this build
  compiles, and a frozen fixture names none, so a disagreement reads as `StalePolicy`. The
  synthetic tests in `tests/normalize.rs` that build records with `Record::gated` hold the leak
  rule, as before.
- The fixtures reuse the builders' frame shapes, which were measured on the versions the
  directories name. No fixture yet has the 2026-09-22 versions (`claude` 2.1.268,
  `cursor-agent` 2026.09.18-9a7762b, `agy` 1.2.8). A shape change on one of those goes in a new
  directory beside the old one.

Suite, outside any seatbelt, with `~/.local/bin` off `PATH` so `tests/drift.rs`'s `claude` probes
skip: 759 passed, 0 failed, 1 ignored (the generator), across 29 targets with doc tests. `cargo
fmt --check` and `cargo clippy --all-targets -- -D warnings` are clean.

**Proposed sentences for build ticket 102.**

For `README.md`, "Status, in numbers", replacing the sentence that starts "23 test files":

> 26 test files, green in CI on macOS. CI's corpus is empty, so the walks over real records check
> nothing there; beside each one a fixture walk runs the same assertions over
> `tests/fixtures/records/`, ten synthetic records filed by the provider version whose shapes
> they reproduce, and the job summary prints both counts: zero live records and ten fixtures. A
> green CI says the readers handle every shape in the fixtures, not that every record on this
> laptop still meets the contract.

For `SECURITY.md`, at the end of the `codex` fail-open paragraph:

> `tests/drift.rs::a_codex_hook_that_dies_without_answering_lets_the_call_run` re-measures this
> with a hook that raises before it answers. It costs one turn, runs only under
> `HYDRA_DRIFT_PAID=1`, and fails if `codex` starts refusing the call.

For `IMPLEMENTATION.md`, rung-two section, after the build ticket 98 paragraph:

> **Since build ticket 101 the path after the proposer is a function a test can drive.**
> `land_proposer_patch` and `review_blocked_in` take the layout, the repository and the judge
> panel as arguments; `hydra evolve` passes the real ones. `tests/evolve.rs` runs both on a
> scratch layout with a panel closure: a fresh review refuses a floor path, and a path outside the
> set its pass recorded, before the panel, and a pass whose checkout changes after admission
> still offers the tree it read.

For `IMPLEMENTATION.md`, where reap's skill hold-back is described:

> Reap reads line 0 through a typed view, `LineZero` in `src/reap.rs`. A `skills` block whose
> `linked` is not a list of strings, or one under a provider this build cannot name, makes the
> line unreadable, and reap holds every skill root back as it does when there is no record.

For `CLAUDE.md`'s layout list, the `tests/` entry: "twenty-six files", and after
`record_contract.rs`: "`fixture_corpus.rs` writes and guards `tests/fixtures/records/`, the
synthetic records the fixture walks in `record_contract.rs`, `normalize.rs` and `outcome.rs`
read;". The `drift.rs` sentence gains: "and a paid `codex` probe of a hook that dies".

Run by the session on 2026-09-24 after the merge, with `HYDRA_DRIFT_PAID=1`:
`drift.rs::a_codex_hook_that_dies_without_answering_lets_the_call_run` passed in 12.9 s against the
installed `codex`, so a codex hook that dies still fails open, now measured by a test and not only
by hand.
