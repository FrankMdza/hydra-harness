# 232 - apprentice in Rust: attempt asks the local model, and score

Type: build
Status: resolved
Blocked by: 231

## What

Map ticket 95. This is "A2" of `.scratch/hydra/research/95-the-plan-for-the-pieces-left.md`: read that section whole, and the plan's
answers under it. It names the files and functions to port, what the differential test compares
on, what must be identical, what is normalized, and the one thing most likely to differ.

Map 95, question C: the client is `hydra-records`' loopback client. A URL that names any other
host is refused with one sentence and exit 1, where the Python would connect; record that case
as a stated difference.

## Do

1. Port what the plan's section names into `crates/apprentice` (package and binary `hydra-apprentice`), with the
   Python's arguments, output and exit codes. The crate depends on `hydra-pyjson` and
   `hydra-records` as it needs, never on `hydra`, `hydra-puppets` or `hydra-grader`.
2. Record the Python once, as `crates/grader/tests/record_collect.py` does: an empty environment,
   frozen data, shell stubs for every program the Python calls, the checkout written as `<repo>`.
   The Rust tests compare with the fixtures and start no `python3`. Fixtures under 1 MB.
3. Break the Rust on purpose once and say how many comparisons fail.
4. The Python stays until the piece's cutover. `IMPLEMENTATION.md`: a passage on what was ported.

## Done when

- Each test failed before the change, or the report says why it could not; fmt and clippy clean over
  the workspace; the tests of every crate the ticket touches pass; no dependency added without a
  reason in the report.

## Outcome

Built 2026-10-04 by an Opus run (`01a10644-6583`), reviewed and applied by the session.

`crates/apprentice/src/attempt.rs` (lines 325 to 444: the URL and model, `prompt_sha`,
`generate` through `hydra_records::loopback_http`, `parse_reply`, `_norm`, `grade`, `attempt`)
and `score.rs` (lines 447 to 499); `cli.rs` parses `attempt [--limit] [--minutes]` and
`score [--model]` as argparse does; only `pairs` still exits 2. `tests/record_attempt.py`
recorded 114 cases against a stub server, with `PYTHONHASHSEED=0` because one `TypeError`'s
wording changed between recordings without it; `tests/attempt.rs` replays them, 600
comparisons. With the old dispatch 357 of 600 fail; with ASCII-only `\d` and `\s` 1 fails.
Stated difference: a URL naming a host other than loopback is refused, exit 1, no request.

**Measured by the session against a live Ollama and `gemma3:1b-it-qat`:** one bank of 20 tasks,
`attempt --limit 6` from each into its own home. The six attempts match on every field but
`seconds`, and `score` prints the same lines (2 of 6). Python 5.6 s, Rust 4.4 s, nearly all of
it the model.

**Found, a gate gap measured in the wild.** The run's first probe ran `env -i python3` with no
`HYDRA_GRADER_HOME`, so the Python resolved the real grader home under `~/hydra` and read it.
The gate did not stop it, because the shell check reads only the paths a command names, not
the ones a program resolves. It read and wrote nothing else. This is the gap `CLAUDE.md`'s
"Nothing here is enforced" section describes, now seen from an agent doing honest work.
