# 211 - `exp`, `log` and `log1p` match CPython to the bit, or they do not

Type: build
Status: open
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
