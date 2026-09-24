# 120 - CI runs what the tickets claim

Type: build
Status: resolved
Blocked by:

## What

Every build ticket's Done-when asks for `cargo clippy --all-targets -- -D warnings`, and
`.github/workflows/ci.yml` (lines near 19 to 36) runs `cargo clippy -- -D warnings`, without
`--all-targets`, so a warning in a test file passes CI and fails the ticket's own check. The
same workflow never runs `integrations/puppets/tests/`, which is the only test suite the
orchestration mode has. Fourth panel, `claude` reader, "Before publishing" item 8.

## Do

Add `--all-targets` to the clippy step. Add a step that runs `python3 -m pytest
integrations/puppets/tests -q` on the same macOS runner, after the Rust steps, and say in a
comment why it is there. Do not change the test steps that exist, and do not add a Linux job.

## Done when

The workflow file has both changes. Run `cargo clippy --all-targets -- -D warnings` and the
pytest line locally once and report both results, so the first CI run after this has no
surprise waiting.

## Outcome

Built by `cursor-agent` on `composer-2.5` under `standard`, run 01a0d3f5-50e5-73b1, 0.69M
tokens, applied to master by hand (`~/.claude/handoffs/2026-09-24-panel4/diff-120.patch`).
`--all-targets` on the clippy step, clean locally in 18 s. The agent wrote the pytest step the
ticket asked for and then reported that the macOS image ships no pytest, that the tests are
plain `unittest` cases, and that `python3 -m unittest discover -s integrations/puppets/tests`
runs the same 45 green. The session changed the step to that, so the first CI run needs no
install. Not yet seen on CI: the push is Frank's.
