# 211 - `exp`, `log` and `log1p` match CPython to the bit, or they do not

Type: build
Status: resolved
Blocked by: 210

## What

Map ticket 94 chose byte-for-byte equality with CPython 3.14.7 for the grader's numbers, on one
condition: over a million sampled inputs, Rust's `f64::exp`, `ln` and `ln_1p` and a port of
Python's compensated `sum()` give the same bits as `math.exp`, `math.log`, `math.log1p` and
`sum` on this machine. This ticket measures it before anything is built on it.

## Do

1. A script `crates/pyjson/tests/record_math.py` that writes, from a fixed seed, one million
   inputs across the ranges the logistic regression reaches (`baseline.py` `fit`, `sigmoid`, the
   loss: read them for the ranges, and add subnormals, huge values and both signs), with the
   bits of `math.exp`, `math.log`, `math.log1p` for each, and 10,000 float lists with the bits of
   `sum()` of each. Keep the fixture small: store the seed and a SHA-256 of the result bits per
   block of inputs, not the inputs, and regenerate the inputs in Rust from the same generator
   (write the generator so both sides produce the same sequence, and prove that with the hash of
   the inputs too).
2. `pyjson::math`: `fsum_like_python` (CPython 3.12's compensated `sum` for floats, Neumaier),
   and thin wrappers for the three functions.
3. A test that regenerates the inputs and compares the hashes. If every block matches, say so
   with the counts. If one differs, find the first differing input, print both bit patterns, and
   stop: the report then starts with that, since the decision goes back to the map.

## Done when

- The report states plainly: equal on all inputs, or the first input that differs for each
  function. fmt and clippy clean; `cargo test -p hydra-pyjson` passes or fails only on a real
  difference.

## Outcome

Resolved 2026-10-01. Run `01a0f6c7-9574` (cursor-agent composer-2.5, `standard`, 2.8M tokens),
diff read whole by the session and applied to master as written.

**Equal on every input.** `f64::exp`, `ln` and `ln_1p` and a Neumaier sum give the bits CPython
3.14.7 gives for `math.exp`, `math.log`, `math.log1p` and `sum()` on this machine, over 1,000,000
inputs in 100 blocks and 10,000 float lists. Map ticket 94's choice of byte-for-byte equality
stands.

Built: `hydra_pyjson::math` (`exp`, `log`, `log1p`, `fsum_like_python`), a SplitMix64 input
generator written twice, in `math_probe.rs` and in `crates/pyjson/tests/record_math.py`, and
`tests/math_matches_python.rs`, which regenerates the inputs, checks their hash against the
recorded one, and compares a SHA-256 per block of results. `sha2` is a dev-dependency of the
crate, already in the lock for the root package.

Measured on master by the session: the test passes; with one bit of `exp` flipped at `700.0` it
fails on block 0; with the sum's correction term dropped it fails on list 0; a plain fold differs
from Python's `sum` on about 7,900 of 10,000 lists of this kind, so the lists do exercise the
compensation.

Left open, for build 213 to keep in mind:

- Two of every eleven inputs are constants, so about 640,000 of the million are distinct random
  values. They cover `exp` on -40 to 40 and the integers 0 to 707, and `log` and `log1p` on 0 to
  1, on `e^-40` to 1 and on those integers. `log1p` of a large count (a task's length in
  characters) is not in the probe: add that range when 213 knows the real one.
- The probe's generator lives in the library (`math_probe`, public) and belongs under `tests/`.
- The recorded hashes are this machine's: arm64 macOS, CPython 3.14.7, the system libm on both
  sides. Another platform's libm is a different question.

