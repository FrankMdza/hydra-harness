# 218 - grader in Rust: the local judge labels and agrees

Type: build
Status: resolved
Blocked by: 212, 214

## What

Map ticket 94, the grader's port. This is "plan 216" of `.scratch/hydra/research/94-the-grader-port-plan.md`: read that section whole,
and "What puppets taught" above it. It names the Python files and functions to port, the inputs
the differential test compares on, what must be identical, what is normalized, and the one thing
most likely to make Python and Rust differ.

Map 94: the HTTP client is hand-written over `TcpStream`, plain HTTP to localhost only; refuse
any other host by name.

## Do

1. Port what the plan's section names into `crates/grader` (package `hydra-grader`, binary
   `hydra-grader`), with the Python's arguments, output and exit codes. The crate depends on
   `hydra-pyjson` and not on `hydra-puppets` or `hydra`.
2. Record the Python once: `crates/grader/tests/record.py` runs the Python on every input of this
   ticket from an empty environment, with frozen data files, and writes
   `crates/grader/tests/fixtures/`. The Rust tests compare with those files and start no
   `python3`. Follow `crates/puppets/tests/record.py` and `differential.rs`. Record with the
   `python3` on `PATH`, which is 3.14.7, and write its version into the fixtures.
3. Break the Rust on purpose once (one constant, one word) and say how many comparisons fail.
4. The Python stays. `IMPLEMENTATION.md`: a passage on what this ticket ported and what it
   normalizes.

## Done when

- The report lists each command and input compared, what was normalized and why, and what the
  plan assumed that turned out false. fmt and clippy clean over the workspace; `cargo test --lib`
  and `cargo test -p hydra-grader` pass.

## Outcome

Resolved 2026-10-01. Run `01a0f72c` (claude opus 5.5, `standard`, 6.3M tokens). The session took
the port on its tests and its deliberate breaks and applied it to master as written.

Built: `crates/grader/src/judge.rs` (`judge.py` whole: `cut`, `show_projection`, `build_prompt`,
`generate`, `parse_answer`, `label_line`, `label`, `agree`), `judge_cli.rs` (`hydra-grader judge
label [--limit N] [--only-unlabelled]` and `judge agree`) and `judge_http.rs`, a client over
`std::net::TcpStream`. It takes `http://` to `127.0.0.1`, `localhost` or `[::1]`, compared as
text, and refuses anything else before it resolves a name or opens a socket (map 94). No
dependency added.

Measured on master: fmt and clippy clean over the workspace; `cargo test -p hydra-grader` 34 unit
and 16 differential tests; the judge's test makes 1,261 comparisons over 98 requests (exit code,
stdout, stderr, the files left, the bytes of `judge-labels.jsonl`, what the stub `hydra` was
called with, and each request's method, path, content type and body). With the dispatch off 807
of 1,163 failed; one word changed in the instructions failed 157; a task budget of 3,001 failed
4. A second test names a listening stub as `127.1`, `LOCALHOST.`, `0.0.0.0`, `user@127.0.0.1`
and `example.com` and checks none reaches it; with the refusal off, `127.1` did.

Normalized: `judged_at`, the stub's port, the scratch directory; a traceback by its class, an
`argparse` error by its message, help by its first words.

The one departure from the Python, on purpose: `urllib` goes wherever `HYDRA_LOCAL_LLM_URL`
points, follows redirects and uses proxies. The Rust refuses any host but this machine's
loopback, with one sentence and exit 1, keeping the labels already written. The default,
`http://127.0.0.1:11434/api/generate`, is unaffected.

Left open: the 300-second timeout was never waited out. No real Ollama was contacted. The scratch
`runs` link in `show_projection` is dead code under `label` and was not ported. The differential
suite of the crate now takes 75 seconds.

