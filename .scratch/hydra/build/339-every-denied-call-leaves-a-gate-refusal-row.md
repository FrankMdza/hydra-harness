# 339 - Every denied call leaves a `gate_refusal` row, not only a denied shell command, so a `codex` run whose web lookup was refused does not read as eight allows and nothing else

Type: build
Status: open
Size: haiku
Blocked by:
Opened: 2026-10-15, by the session, from run `01a14290-c0a1`. The run's own report says the gate
refused its web lookup twice; `hydra show --events` on the record shows eight `permission_decision`
allows, no refusal and no finding. Two facts meet: `gate.py::record_refusal` writes a row only when
`capability_of(tool) == "shell"` and the input carries a command, and a denied `codex` call leaves
no stream frame (IMPLEMENTATION.md, the `codex` paragraph since build 48). On `claude` the stream
carries the denial as a tool result, so the gap shows on `codex` first, and on any provider for a
tool the table does not know.

## What

`record_refusal` writes a row for every `deny`, whatever the capability. The row keeps its shape
for shell (`command`, `command_truncated`) and for everything else carries `arguments_sha256`, the
sha of the canonical JSON of `tool_input`, and no argument text, because a refused `Read` path or a
URL may name what the policy hides. `tool`, `call_id` and `reason` are present on every row.
`watch.rs::copy_gate_refusals` already copies rows into the record and `walk.rs::check_gate_refusals`
corroborates them against the calls the stream names; a row whose call has no stream frame counts
under the existing `N with no call_id` or a new `no frame` count, the ticket decides which after
reading build 332's Outcome. `docs/run-record-contract.md`'s `gate_refusal` section gains the two
optional fields.

## Contract

Input: a hook payload and the gate's `deny`. Output: one JSON line appended to `gate-refusals.jsonl`
with `type`, `tool`, `call_id`, `reason`, and either `command` and `command_truncated` or
`arguments_sha256`. Latency: under 5 ms per row on this Mac, measured once in the Outcome.

## Cases

In `tests/gate.rs`, next to `refused_shell_commands_survive_the_hook_record_and_show_with_a_byte_cap`
and `watcher_records_refusals_from_each_live_hook_dialect`, written first:

1. A refused `Read` of `~/hydra/runs/x.jsonl` under `standard` leaves a row with `tool: Read`,
   the call id, the reason, an `arguments_sha256` of 64 hex characters, and no `command` key.
2. A refused `webrun` on the `codex` dialect, unknown tool, leaves a row with `reason` starting
   `hydra/unknown-tool`.
3. A refused shell command still leaves the build 332 shape, byte for byte against the existing
   fixture.
4. An allowed call leaves no row, as today.
5. `check_gate_refusals` over a record carrying case 2's row and no matching frame prints it in
   its count line, and the validator accepts the record.

## Do

1. The cases, red.
2. `record_refusal`, the contract section, the walk count.
3. The fixture corpus regenerated if a fixture carries a refusal row, `fixture_corpus.rs` guards.
4. IMPLEMENTATION.md where build 332 is described.

## Done means

The four checks green, the Outcome quotes one row of each shape, and `hydra doctor --walk` on this
machine shows the new count with its number.
