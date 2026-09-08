# 19 - The Herdr bridge, as a plugin on Herdr's side

Type: build
Status: resolved
Blocked by:

## Goal

Ticket 44 decided Herdr is the interactive layer and that nothing crosses from Hydra to it: a
bridge on Herdr's side tails `~/hydra/runs/` and labels the pane from the record. Ticket 45
measured every call the bridge needs on herdr 0.8.2. Build it, in a directory of its own that
`src/` never imports, so that the day Herdr moves or dies the only thing that breaks is this.

Map ticket 49 holds two decisions this ticket does not make. Until the human answers, `timed_out`
and an abandoned run sit on Herdr's `unknown`, and `base_tree` stays off the record. Both are one
line to change.

## Where the design says it

- Map tickets 44 and 45, and 49 for what is still open.
- D10: nothing has to be running for Hydra to work. The bridge polls files and never the reverse.

## The slices

**`integrations/herdr/`.** A `herdr-plugin.toml` with id `hydra.bridge`, a `bin/` with two
scripts, a `README.md` that says how to `herdr plugin link` it, and a `tests/` with one Python
test that runs without Herdr.

**`bin/hydra-in-pane`.** Runs inside a Herdr pane. Takes `hydra run` arguments verbatim, launches
`hydra run --timeout 0 "$@"`, reads the run id from the `hydra: run <id>` line the launcher
prints, puts `run_id`, `provider` and `policy` on its own pane as tokens through
`herdr pane report-metadata "$HERDR_PANE_ID" --source hydra-bridge`, reports `--state working`
through `report-agent`, then replaces itself with `hydra logs <id> --follow` so the pane shows the
stream. Refuses to start outside Herdr, `HERDR_ENV` unset, saying so.

**`bin/hydra-bridge`.** Python 3, standard library only. Every two seconds lists the panes that
carry a `run_id` token, reads `hydra status <id>` for each, and reports on change only: live is
`working`; `completed` and `failed` are `idle`; `gated`, `auth_failed`, `rate_limited`,
`mcp_unavailable` and `skill_collision` are `blocked`; `timed_out`, `crashed` and abandoned are
`unknown`. Every report also sets `--state-label` so the pane displays Hydra's word over Herdr's
state. The mapping is one table at the top of the file with ticket 49 named beside the two rows
it may move.

**The manifest.** A `[[panes]]` entry that runs the bridge, placement `split`, and a
`[[actions]]` entry that opens a pane in the workspace's cwd running `hydra-in-pane`. The exact
manifest fields are whatever `herdr plugin` on this machine accepts, checked by linking it.

**The test.** `tests/test_mapping.py` feeds the mapping function three `hydra status` shapes,
live, ended with an outcome, abandoned, and asserts the state and label for each of the twelve
rows. No Herdr, no Hydra binary.

## Done when

- `herdr plugin link` accepts the directory and `herdr plugin list` shows it.
- A run launched through `hydra-in-pane` shows its id as a token on the pane and its final
  outcome as the pane's label, checked with `herdr pane get`.
- The Python test passes with `python3 -m unittest`.
- `IMPLEMENTATION.md` gains a short section saying the bridge exists, where, and what 49 still
  decides.

## Outcome

Resolved 2026-09-07. Built and driven end to end on herdr 0.8.2 against the human's live session.

**Who wrote what.** `agy` on Gemini 3.1 Pro, run 01a07efb-de49 under `standard`, wrote the
manifest, both scripts and the mapping test, then died on its own turn timeout at 292 seconds
with the test failing on an import detail and no README: 1.08M tokens, 877k of them cache reads.
The orchestrating `claude` session fixed the import, wrote the README, added `min_herdr_version`
and `platforms` to the manifest, made both commands run through their interpreter, and made both
scripts take `HYDRA_BIN`, because a shell alias does not reach a script.

**What Herdr wanted that the ticket did not know.** `min_herdr_version` is required, not
optional. A `[[panes]]` or `[[actions]]` command is resolved on the `PATH` and not against the
plugin directory, so `["bin/hydra-bridge"]` fails to spawn and `["python3", "bin/hydra-bridge"]`
works because the plugin directory is the cwd. `herdr plugin pane open` takes `--plugin` and
`--entrypoint`, not a positional.

**Measured.** A run launched through `hydra-in-pane` in pane `w1:p2`, `claude` haiku under
`read-only`: the pane carried `run_id`, `provider` and `policy` as tokens and `agent_status:
working` while the run lived. With the bridge pane open, eight seconds after `run_end` the pane
read `agent_status: done` with `state_labels: {idle: completed}`, and nothing had touched it but
the bridge. `herdr pane list` returns `result.panes[]` with `pane_id` and `tokens`, which is what
the bridge reads.

**Two things worth knowing.** `agy` tested `report-metadata` against the human's own pane `w1:p1`
with `run_id=1234`, from inside a `standard` run: the Herdr socket is a file and `herdr` is on
the `PATH`, so a run's shell reaches the human's live session. It is harness spec 7.5's shell
hole with a new name, and the token was cleared by hand. And `timed_out` never reaches the
bridge as an ended outcome, because `hydra status` reports a live child as `live` and the word is
the handle's; the row exists for ticket 49 and is dead in practice until that changes.
