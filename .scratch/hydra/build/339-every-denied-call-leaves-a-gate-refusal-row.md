# 339 - Every denied call leaves a `gate_refusal` row, not only a denied shell command, so a `codex` run whose web lookup was refused does not read as eight allows and nothing else

Type: build
Status: resolved
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
sha of `tool_input` serialised by Python's `json.dumps(sort_keys=True, separators=(",", ":"),
ensure_ascii=False)` encoded as UTF-8, named so a Rust reader can reproduce it, and no argument text, because a refused `Read` path or a
URL may name what the policy hides. `tool`, `call_id` and `reason` are present on every row.
`watch.rs::copy_gate_refusals` already copies rows into the record and `walk.rs::check_gate_refusals`
corroborates them against the calls the stream names; a row whose call has no stream frame counts
under the existing `N with no call_id` or a new `no frame` count, the ticket decides which after
reading build 332's Outcome. `docs/run-record-contract.md`'s `gate_refusal` section gains the two
optional fields and says which of `command` or `arguments_sha256` a row carries and that never both are absent.

## Contract

Input: a hook payload and the gate's `deny`. Output: one JSON line appended to `gate-refusals.jsonl`
with `type`, `tool`, `call_id`, `reason`, and either `command` and `command_truncated` or
`arguments_sha256`. Latency: under 5 ms per row on this Mac, measured once in the Outcome.

## Cases

In `tests/gate.rs`, next to `refused_shell_commands_survive_the_hook_record_and_show_with_a_byte_cap`
and `watcher_records_refusals_from_each_live_hook_dialect`, written first:

1. A refused `Read` of `~/hydra/runs/x.jsonl` under `standard` leaves a row with `tool: Read`,
   the call id, the reason, an `arguments_sha256` of 64 hex characters, and no `command` key.
2. A refused `webrun2` on the `codex` dialect, a name the table does not hold, leaves a row with
   `reason` starting `hydra/unknown-tool`. Not `webrun`, which build 338 makes known.
3. A refused shell command still leaves the build 332 shape, byte for byte against the existing
   fixture.
4. An allowed call leaves no row, as today.
5. `check_gate_refusals` over a record carrying case 2's row and no matching frame prints it in
   its count line, and the validator accepts the record.

## Amended by the panel, 2026-10-15

The codex seat (`133-panel-sol.md`) read two things the ticket missed, both confirmed by the
session. `src/record/validate.rs::valid_gate_refusal` requires `command` as a string and
`command_truncated` as a boolean, and `watch.rs::copy_gate_refusals` copies only rows that
validator accepts, so a non-shell row written by `record_refusal` would be counted as damaged and
dropped: the validator changes in the same commit, one shape with `command`, one with
`arguments_sha256`, nothing else accepted. And `walk.rs::check_gate_refusals` joins a row to a
call by id and failure status alone, so a forged row carrying a real failed call's id and an
invented command reads as corroborated; hashing arguments adds no authenticity. The ticket does
not fix that, build 332 owns the sidecar's trust model, but the walk's count line separates "no
frame for this call id" from "no call id on the row", and the Outcome says how many of each the
real walk finds. Case 5 asserts the first count, not the second.

## Do

1. The cases, red.
2. `record_refusal`, the contract section, the walk count.
3. The fixture corpus regenerated if a fixture carries a refusal row, `fixture_corpus.rs` guards.
4. IMPLEMENTATION.md where build 332 is described.

## Done means

The four checks green, the Outcome quotes one row of each shape, and `hydra doctor --walk` on this
machine shows the new count with its number.

## Outcome

Built on 2026-10-15 in the run's worktree. Nothing committed.

**What was built.** `gate.py::record_refusal` writes a row for every denial with a string `tool`.
A shell row keeps the build 321 shape. Any other row carries `arguments_sha256`: the SHA-256 of
`tool_input` as Python's `json.dumps(sort_keys=True, separators=(",", ":"), ensure_ascii=False)`
encodes it to UTF-8, with `surrogatepass` so a lone surrogate hashes instead of losing the row.
`valid_gate_refusal` accepts exactly the two shapes and rejects a row with both. `walk.rs` adds
`RefusalCounts::no_frame`, a subset of `uncorroborated`, and the count line now reads
`gate_refusal rows: C corroborated, U uncorroborated, N with no frame, M with no call_id`. An
uncorroborated note for a non-shell row prints `arguments_sha256` where a shell row prints
`command`. The contract's `gate_refusal` section and IMPLEMENTATION.md are amended.

**Tests added, in `tests/gate.rs`.** Cases 1 to 5 as the ticket lists them:
`a_refused_read_leaves_a_row_that_hashes_its_arguments_and_names_no_command`,
`a_refused_tool_the_table_does_not_hold_leaves_a_row_that_names_the_unknown_tool`,
`a_refused_shell_command_keeps_the_build_332_shape_and_carries_no_hash`,
`an_allowed_call_leaves_no_refusal_row`, and
`a_row_whose_call_has_no_frame_is_counted_as_no_frame_and_the_record_validates`. Also
`the_validator_takes_one_of_the_two_refusal_shapes_and_no_mix`. The Read case checks the hash
against `serde_json` and `sha2` computed in Rust, so the reproduction the contract promises is
tested. `tests/doctor.rs` moves its count-line expectation to the new line. Nothing else changed.

**Red, honestly.** The cases went in first. The first run did not compile, since `no_frame` did
not exist yet. The Python change was then written before any behavioural case ran, so the
"no row was written" failure was never observed on the old hook. Two of my own assertions were
wrong and failed for that reason: the Read case asserted the path was absent from the whole
sidecar, and case 5 used a hand-built record without the `run_start` fields the validator needs.
Both were fixed, not weakened.

**Measured.** The four checks, then the cases:
- `cargo fmt --all --check`: clean after one `cargo fmt --all` run, which reflowed my own lines.
- `cargo clippy --workspace --all-targets -- -D warnings`: finished with no warnings.
- `cargo nextest run --release --workspace`: 1453 run, 1453 passed, 4 skipped.
- `cargo test --release --workspace --doc`: eight doctest suites, none with a failure. The
  workspace holds no doctests of its own, so the suites ran zero.
- The eight `gate` tests in the new block and the older refusal tests pass, and so do
  `walk_corroborates_each_gate_refusal_row_against_the_call_it_names` and `fixture_corpus`.
- Latency, `target/build339/measure.py` run with `/usr/bin/python3 -I`, 2000 rows each, timing
  `record_refusal` alone with the gate imported from `src/policy/gate.py`: a Read row takes
  0.22 ms on average, p99 2.6 ms; a Bash row takes 0.16 ms on average, p99 2.3 ms. Each shape
  had one outlier of 20 to 52 ms in a single run on this 8 GB Mac. The rule holds on the
  average and the p99, and not on the worst sample.

Sample rows, one of each shape, from `record_refusal` as the measurement script ran it:
`{"type": "gate_refusal", "tool": "Read", "call_id": "toolu_read", "arguments_sha256": "1357edd9b5126604fe41b99722ee0409a3be487617494411d3f0fc4b1dfb3559", "reason": "hydra/out-of-scope: a refused call, for timing."}`
`{"type": "gate_refusal", "tool": "Bash", "call_id": "toolu_bash", "command": "cat > probe.txt <<'EOF'\nbody\nEOF", "command_truncated": false, "reason": "hydra/out-of-scope: a refused call, for timing."}`

The real walk was not run. `hydra doctor --walk` reads `~/hydra/runs/`, this run's rules forbid
reading outside its worktree, and the gate refuses a command that names that directory. So the
count for this machine is not in this Outcome. The session takes it with
`./target/release/hydra doctor --walk | grep 'gate_refusal rows'`. Whether `codex` and
`cursor-agent` hook ids equal their stream ids is still unmeasured, as build 332 left it.

**What the ticket assumed that was false.**
1. The hash keeps the arguments out of the row, but not the path out of the record. The gate's
   `reason` for a refused path names the path, for example
   `hydra/out-of-scope: policy standard allows file_read inside worktree ... and <path> is outside it`.
   The ticket names `reason` as present on every row, so this build left it alone. Trimming it is
   a decision, and it belongs on the map.
2. The ticket let a no-frame row fall under the existing `no call_id` count or a new one. The panel
   asked for a separate count, so `no frame` is new and `no call_id` is unchanged.
3. No fixture needed regenerating. Fixture 321 is a shell row and its shape did not change.
4. Byte for byte against the fixture is true for the line the hook writes, not the fixture's body.
   Serde writes the fixture's keys sorted, and the hook writes its own order and Python's default
   separators. The test asserts the exact line.

**A limit left alone.** A deny with no string `tool_name` still writes no row, because the
validator requires `tool` to be a string. I did not test that case.

**Measured by the session on master, 2026-10-15, after the three builds of the night landed together:** `hydra doctor --walk` over 976 records prints `gate_refusal rows: 0 corroborated, 2 uncorroborated, 2 with no frame, 1 with no call_id`; the suite is 1460 tests, all green.
The two rows with no frame are both `cursor-agent` seats of the same night (`01a14291-8701`,
`01a142a1-e646`): the floor refused a `wc -w ... && python3 -c "..."` the shell layout could not
parse, the hook wrote the row, and Cursor's stream carries no tool call for that id. That is the
shape this ticket exists for, seen on a second provider the day it landed.
