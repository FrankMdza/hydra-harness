# 21 - A smoke test for the bridge, against a running Herdr

Type: build
Status: resolved
Blocked by:

## Goal

Herdr is 105 days old, on one developer, and its socket protocol moved from 15 to 20 while its
public docs stood still. Build ticket 19's bridge was driven by hand once. One script should
redo that drive on demand, so that a Herdr update that breaks a flag or a JSON field fails a
test rather than a demo.

## The slices

**`integrations/herdr/tests/smoke.sh`.** Bash. Refuses to start unless `herdr status server`
says a server is running and `HYDRA_BIN` names an executable, and says which. Then, in order:
splits a new pane from the first pane `herdr pane list` returns, with `--cwd` set to the repo
root and `--env HYDRA_BIN`, and remembers its id; runs `bin/hydra-in-pane --provider claude
--model haiku --policy read-only --task "Answer in one line: what is 2+2? Do not read any
file."` in it through `herdr pane run`; polls `herdr pane get` every three seconds for up to
120 seconds until `tokens.run_id` is present and `state_labels.idle` equals `completed`; prints
each poll on one line; closes the pane it opened whether or not it passed; exits 0 on the label
and 1 otherwise, naming what it saw. It opens the bridge pane itself if no pane's `cwd` ends in
`integrations/herdr`, through `herdr plugin pane open --plugin hydra.bridge --entrypoint bridge`,
and closes it too if it opened it. It never touches a pane it did not open.

**`README.md`.** One paragraph under a `## Smoke test` heading saying what it does, that it
spends one haiku call, and that it needs Herdr running.

## Done when

- `HYDRA_BIN=$PWD/target/release/hydra integrations/herdr/tests/smoke.sh` passes on this machine.
- Run with the server stopped, it refuses in one line and exits 1.

## Outcome

Resolved 2026-09-07. `cursor-agent` on Grok 4.6 Fast, run 01a07f05-7a50 under `standard`, wrote
`integrations/herdr/tests/smoke.sh` and the README paragraph in one pass and ran the test to a
pass against the human's live Herdr: 973k tokens, 800k of them cache reads. The pass opened pane
`w1:p4`, launched run 01a07f08 through `hydra-in-pane`, saw the label go `none` to `completed`
on the fifth poll, and closed the pane. One thing the ticket did not know: `herdr pane split`
requires `--direction`, and the script splits down.

The run it launched is a real recorded run, `read-only` on haiku, and it was reaped by hand
afterwards with its branch deleted. Every smoke pass leaves one such run and one worktree behind,
which `hydra status` reports and the human reaps. The script does not reap it, because reap is a
command a human types and the ticket did not ask the test to grow that.
