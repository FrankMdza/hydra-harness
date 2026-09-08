# 24 - The pane says what the record knows

Type: build
Status: resolved
Blocked by:

## Goal

Map ticket 49 put `timed out` on Herdr's `unknown` and added a requirement: since Herdr cannot
classify a Hydra run, the pane carries what a human needs to understand the run without
Herdr's help. The bridge already puts `run_id`, `provider` and `policy` on the pane. Add the
facts that change while the run works, and a title that reads at a glance.

## Where the design says it

- Map ticket 49's `## Answer`, first paragraph.
- `integrations/herdr/bin/hydra-bridge`, the poller, and `integrations/herdr/README.md`.
- `hydra status <id>` for the liveness fields, `hydra show <id> --events` for the last call.

## The slices

**Tokens on every report.** Beside the three that exist: `state` as `hydra status` reports it,
`live`, `ended` or `abandoned`; `outcome` once there is one; `alive` as `yes` or `no` from the
pid check `hydra status` already did; `last_tool`, the `name` of the latest `tool_call` event in
`hydra show --events`, with its `seq`; `elapsed`, seconds since `started_at`, rounded; and
`tokens`, the projection's total once the run ended. Report through the one
`report-metadata` call the bridge already makes, so a change to any of them is one call.

**A title.** `report-metadata --title` with the provider, the short run id, and the outcome or
the last tool, for instance `claude 01a07f08 · completed` while it is over and `claude 01a07f08 ·
Read` while it works. Plain ASCII if Herdr rejects the dot.

**`timed out` and abandoned.** Both sit on `unknown`, as decided. The `unknown` label says
which: `timed out, child alive`, `timed out, child gone`, or `abandoned`, from `alive` and
`state`. The mapping table at the top of the file loses its ticket 49 comment and gains the
decision date.

**Report on change only, still.** The comparison key grows to the whole token set, and the
poll stays at two seconds. `hydra show --events` on a long record is the one call that could
grow expensive; read it at most once per poll and only for live runs.

**The test.** `tests/test_mapping.py` gains cases for the three `unknown` labels and for the
token set built from a `hydra status` shape plus a short events list. `tests/smoke.sh` asserts
`tokens.outcome` equals `completed` beside the label it already checks.

## Done when

- A run launched through `hydra-in-pane` shows `last_tool` changing while it works and
  `outcome`, `tokens` and the title once it ends, checked with `herdr pane get`.
- The Python test and the smoke test pass.
- `README.md` lists the tokens.

## Outcome

Resolved 2026-09-07. `cursor-agent` on Composer 2.5, run 01a07f18-9a28 under `standard`, wrote
the whole ticket in one pass: 582k tokens, 526k cache reads, six tests green. The bridge now
puts `state`, `alive`, `outcome`, `tokens`, `elapsed` and `last_tool` on the pane beside the
three it had, sets a title such as `claude 01a07f1a · completed`, and labels `unknown` with
`timed out, child alive`, `timed out, child gone` or `abandoned`. One change by the
orchestrating session: `elapsed` is reported in thirty-second steps, because a value that
moves every poll would have sent Herdr a report every two seconds for the life of every run.

Measured with `tests/smoke.sh` after the bridge pane was restarted on the new code: the pane
went from no tokens to `run_id` on the second poll and to `idle=completed` with
`outcome=completed` on the fourth. The smoke test asserts both now.

Two housekeeping facts. The bridge pane runs the script it was opened with, so a change to
`hydra-bridge` needs `herdr plugin pane close` and a fresh open, and the smoke test closes the
bridge pane it opened, leaving none behind; the README says both. And `__pycache__` had reached
git through the earlier commit; it is ignored inside `integrations/herdr/` now.
