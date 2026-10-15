# 332 - A forged `gate_refusal` row is read back as what it is, a damaged sidecar closes the record, and the walk corroborates each row against a call

Type: build
Status: open
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
