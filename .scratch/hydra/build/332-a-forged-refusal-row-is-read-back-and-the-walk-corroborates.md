# 332 - A forged `gate_refusal` row is read back as what it is, a damaged sidecar closes the record, and the walk corroborates each row against a call

Type: build
Status: resolved
Blocked by:
Opened: 2026-10-15, by the session, from map 130's answer, question 4. Build 321 records refused
shell commands whole through a sidecar the hook appends under `spawn/<run-id>/gate-refusals.jsonl`,
and the contract (7.7, 2026-10-15) says what it is: hook evidence, not tamper-proof provenance,
because the hook runs under the run's own seatbelt profile and the kernel allowance that lets the
hook write the file lets the run's shell write it too. The human accepted that standing and
declined a signature (a secret the hook can read, the shell can read). What the panel asked for
instead is here.

## What

Read `src/watch.rs` `copy_gate_refusals`, `src/policy/gate.py` where the hook appends the row, the
`gate_refusal` section of `docs/run-record-contract.md`, and `src/walk.rs`. Then:

1. **The forged row.** A test in the style of `tests/detached.rs`: a stub run whose script appends
   a well-formed row to its own sidecar mid-run (`tool: "Bash"`, a `call_id` no call has, a
   `command` of its choosing), then exits. The record is read back: the forged row is there as a
   `gate_refusal` event, `hydra show --events` prints its command inside the untrusted-content
   boundary, and the derived `permission_decision` counts are unchanged, since those come from the
   provider's own stream. The test is the proof that the contract sentence is true, nothing more.
2. **The damaged sidecar.** A test where the sidecar holds a truncated line, a non-JSON line and
   an oversized row: `copy_gate_refusals` keeps the well-formed rows, notes the damage once on the
   record where 321 said it does, and `run_end` is still written. The record passes `check`.
3. **Corroboration in the walk.** `hydra doctor --walk` reads each `gate_refusal` row's `call_id`
   against the provider stream of the same record. A row whose `call_id` matches no tool call, or
   matches a call whose result is a success, is reported as `Uncorroborated` with the call id and
   the row's command, as a limit of the record and not as a leak (the mark the walk already uses
   for `Foreign` is the model). A row whose call failed is `Corroborated` and silent. Rows with a
   null `call_id` are counted apart. Run the walk over every record on this machine and paste the
   counts in the Outcome: today's rows all come from the hook and should corroborate.
4. `SECURITY.md` gets the contract's sentence where it describes what a record proves about a
   refusal, so the trust language is the same in both documents.

## Do

1. The two tests on a stub, launching no provider.
2. The walk's check and its test in `tests/doctor.rs` on a synthetic record.
3. `SECURITY.md`; `IMPLEMENTATION.md` where 321's paragraph is.

## Done means

`cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo
nextest run --release --workspace`, `cargo test --release --workspace --doc` green; the walk's
counts over the real corpus in the Outcome.

Rules inside the run: relative paths only, never look outside your own worktree, write files with
your file tool and not with shell heredocs, and the test suite is green inside a run since build
302 without any `env -u`.

## Outcome

Built on 2026-10-15.

**The forged row.** `tests/detached.rs::a_row_the_run_appends_to_its_own_sidecar_is_read_back_as_hook_evidence`
runs a `/bin/sh` stub under the real watcher. It streams one refused `Bash` call,
`toolu_real`, and appends a well-formed row for `toolu_forged` to its own
`spawn/<run-id>/gate-refusals.jsonl`. The record passes `check`, and the normalizer returns the
forged row byte for byte as the only `gate_refusal` event. `show --events` prints its command
inside the `<<<hydra-result>>>` boundary. The same record with that line removed derives the
same decisions, one row for `toolu_real`.

**The damaged sidecar.** The ticket assumed `copy_gate_refusals` already kept the well-formed
rows. It did not. Build 321 used `ensure!` on every line, so the first bad line ended the copy
and every row after it was lost, though `run_end` was still written. It now reads bytes up to a
1 MiB cap (`GATE_REFUSAL_LINE_CAP`) and skips a truncated line, a line that is not JSON or not
a valid row, and an oversized line, whose remainder it reads and throws away so that tail
cannot pass for a row. It returns the count, and `gate_refusals_at_close` writes
`gate refusal evidence: N damaged line(s) skipped, the rest copied` once on `run_end.error`.
`a_damaged_sidecar_keeps_its_good_rows_and_still_closes_the_record` writes a good row, a non-JSON
line, a row of 1 MiB plus, a second good row and a truncated tail. Both good rows are copied,
the note says 3, it appears once, and `check` passes.

**Corroboration.** `walk::check_gate_refusals` joins each row's `call_id` to the same record's
tool calls and results. `error` and `unclassified` corroborate. A row naming no call, a call
with no result, or an `ok` result is noted as `Uncorroborated` with its seq, call id and
command, quoted with `{:?}` so control bytes from the run cannot reach the terminal raw. A null
`call_id` is counted apart. Notes are on `Report::uncorroborated` and do not set exit 1.
`hydra doctor --walk` prints `gate_refusal rows: C corroborated, U uncorroborated, N with no
call_id`. `tests/doctor.rs::walk_corroborates_each_gate_refusal_row_against_the_call_it_names`
reads 1, 2 and 1 off a four-row record and the walk exits 0.

**The real corpus was not walked.** This run's rules forbid reading outside the worktree, and
`hydra doctor --walk` reads `~/hydra/runs/`. The counts are the session's to take:
`./target/release/hydra doctor --walk | grep 'gate_refusal rows'`. One prediction to check
against: the dialect test in `tests/gate.rs` sends hook payloads with no id, and `agy`'s
stream ids are step indices the normalizer makes up, so an `agy` row can only land under no
`call_id` or uncorroborated. Whether `cursor-agent` and `codex` hook ids equal their stream ids
is not measured either.

**Not done, found.** `show --events` marks only `command`. A forged row's `reason`, and any
extra key a forger adds (`valid_gate_refusal` does not refuse extra keys), print unmarked.

Docs: `SECURITY.md` has the contract 7.7 sentence in a new bullet under the vendor-build one,
`IMPLEMENTATION.md` a paragraph after 321's, and contract 7.7 one sentence on the copy and the
walk. Full suite 1430 passed, 4 skipped. One run saw
`cockpit::v_asks_before_a_review_and_y_starts_the_helper_detached` time out at 10 s under load.
It passed alone in 0.5 s and in the next full run.

Measured by the session on the Mac after applying, 2026-10-15, with this build's binary:
`hydra doctor --walk` over 961 records prints `gate_refusal rows: 0 corroborated, 0
uncorroborated, 0 with no call_id`. Zero is the true count: the only record carrying the event
type is the fixture 321 generated, since no gated run after 321 landed has had a shell command
refused (the seats and builds of the day report "no tool call was refused by the gate"), and the
spawn directories of reaped runs are gone with their sidecars. The first refusal of a real run
after today will be the first row the walk corroborates.
