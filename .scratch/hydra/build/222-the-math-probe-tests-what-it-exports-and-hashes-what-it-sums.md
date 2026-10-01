# 222 - The math probe tests what it exports and hashes what it sums

Type: build
Status: resolved
Blocked by: 213

## What

A read-only review by `codex` (gpt-6-astra) on 2026-10-01 of builds 221, 210 and 211 found
nothing left that runs a helper by a bare name or a relative path, and no change in what
`crates/puppets` exports. The session fixed two of its findings in Python the same day (a goal log
whose name starts with a dot was skipped; a named helper that cannot run made the board print a
traceback). Three are left, in the probe of build 211 and in one read command. It waits on 213,
which is extending the same probe.

## Do

Tests first, each shown failing before the change.

1. **The probe's test does not call the exported functions**
   (`crates/pyjson/tests/math_matches_python.rs` near 70, `crates/pyjson/src/math_probe.rs` near
   76). `unary_log` and `unary_log1p` call `ln()` and `ln_1p()` themselves, so a wrong
   `math::log` or `math::log1p` passes. Keep the domain rule in the probe and call the exported
   functions for the value. Break `math::log` once and say the test fails.
2. **The order of a sum's inputs is not checked** (`crates/pyjson/tests/record_math.py` near 118).
   Each list is hashed by its result alone: reverse every list and all 10,000 hashes stay the
   same. Hash each list's length and its inputs' bits in order, then the result, and record
   again. One hash per block of lists is enough: `sum_hashes.txt` is 650 KB and can be a few
   lines. The Rust generator also takes `% 199` of a `usize`, which differs from Python on a
   32-bit target: use `u64`.
3. **`pending` needs no vendors file when nothing asks for a seat**
   (`crates/puppets/src/read.rs` near 176). Since build 221 it resolves `vendors_file()` before
   `pending_approvals` knows whether a panel is reachable, so a helper with no data directory
   exits 1 on a goal with no plans and no runs, where `[]` is the answer. Resolve it only when
   the degraded check runs. No fixture changes.
4. The generator of the probe moves out of the library: `math_probe` is test code. Put it under
   `crates/pyjson/tests/` and keep `math` the only new public module.

## Done when

- fmt and clippy clean over the workspace; `cargo test -p hydra-pyjson`, `-p hydra-puppets` and
  `-p hydra-grader` pass with no fixture of `crates/puppets` changed.

## Outcome

Resolved 2026-10-01. Run `01a0f702` (cursor-agent composer-2.5, `standard`, 3.0M tokens), diff
read whole by the session and applied to master as written.

Built: the probe's generator moved to `crates/pyjson/tests/support/math_probe.rs`, and `math` is
the only public module the probe work added. `unary_log` and `unary_log1p` keep the domain rule
and call the exported `math::log` and `math::log1p`. Each block of 10,000 lists is hashed by every
list's length, its inputs' bits in order, and its result; `sum_hashes.txt` is one line where it
was 10,000. The list length is taken with `u64`. `pending` resolves the vendors file only when
`panel_gate_reachable` is true.

Measured on master: fmt and clippy clean over the workspace; `hydra-pyjson` 22 and 3 probes;
`hydra-puppets` 45, 5 and 18 with no fixture changed; `hydra-grader` 11 and 11. The session broke
`math::log` and the probe failed on block 0, which it did not before this ticket; and ran a
helper copied outside the checkout, with no data directory, on a goal with no plans: `pending`
prints `[]` and exits 0.

Left open: item 3 has no test of its own; the run checked it by reading and the session by hand.

