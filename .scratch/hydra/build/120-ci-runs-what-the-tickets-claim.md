# 120 - CI runs what the tickets claim

Type: build
Status: open
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
