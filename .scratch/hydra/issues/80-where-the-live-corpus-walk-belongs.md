# 80 - Where the walk over this machine's records belongs

Type: grilling
Status: resolved
Blocked by:
Opened: 2026-09-24, by the session, after the walk went red twice in one day on vendor builds (build tickets 113, 116, 117) and the fourth panel's `claude` reader asked whether that is a test (weakness 7, "Before publishing" item 9, interview question 8).

## Question

`tests/normalize.rs` and its siblings walk every record under `~/hydra/runs/` inside
`cargo test`, beside the fixture walks that run on CI's empty corpus. On 2026-09-24 the
on-machine walk went red three times before noon: Cursor shipped `webSearchToolCall` (113),
Claude Code 2.1.280 changed the text of a hook refusal (116), and 2.1.281 renamed `Task` to
`Agent` (117). Each was a real finding and each was found by the walk. Each also meant that
`cargo test` on this laptop was red for reasons no contributor can reproduce, and that a
ticket's "suite green" claim depended on which vendor build was installed that morning.

The reader's proposal: the fixture twins stay in the suite as the regression tests, and the
live walk moves to `hydra doctor`, or to a scheduled job with an exit code, an allowlist of
known drift and an owner. The walk keeps its job as a detector and stops being a test.

What it costs: `hydra doctor` grows a `--walk` or the walk becomes its own command; the walks
in `tests/` that read the live corpus (`grep -rn "Layout::discover\|HYDRA_ROOT" tests/`) split
into a fixture half that stays and a live half that moves; `README.md`'s paragraph on what CI
proves (near lines 226 to 231) changes; the session's pre-push check gains a line.

Options:

1. Leave it. The suite is red when the world drifted, and that is the alarm; `CI` is green on
   fixtures and that is the regression half. Document that the on-machine suite is a monitor.
2. Move the live walks to `hydra doctor --walk` (or `hydra walk`), exit non-zero on a finding,
   keep the fixture walks in `cargo test`. The pre-push script runs both.

The session's recommendation is 2, and it should land after 116 and 117 so the first run of the
new command is green.

## Answer

Resolved 2026-09-24 by the human, option 2.

The walks over `~/hydra/runs/` leave `cargo test` and move to `hydra doctor --walk`. It runs
the same assertions over the live corpus and exits non-zero on the first finding it cannot
explain, naming the record and the assertion. The fixture walks over `tests/fixtures/records/`
stay in the suite as the regression half, and CI keeps running them. `cargo test` now means the
same thing on every machine. The walk keeps its job as the detector for vendor drift, and the
session's pre-push script runs both.

The question was answered with the walk red again. Run 01a0d484-2d8c, the fourth panel's
`codex` seat, stopped 27 seconds in with exit 0, no `turn.completed` and no quota text on the
stream. It derives `failed`, and `tests/outcome.rs` near line 1425 asserts a `failed` run never
exits 0. The contract, section 12, makes a streamed run that stopped without a terminator
`failed`, so the test's claim is the thing to fix. Build ticket 125 does that first, so that
build ticket 126's first run of `hydra doctor --walk` comes back green.

Build tickets 125 and 126.
