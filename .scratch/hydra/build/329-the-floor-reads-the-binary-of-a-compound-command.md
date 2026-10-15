# 329 - The floor reads `then`, `do` and `else` as the binary, so `if true; then git push --force; fi` passes

Type: build
Status: resolved
Blocked by:
Opened: 2026-10-15, by the session, from build 319's Outcome ("a hole I found and didn't fix").
The pattern floor of harness spec 7.4 reads each segment's first word as the program it runs
and checks it against the floor targets. A segment that starts with a shell keyword, `then`,
`do`, `else`, `elif`, `{`, `!`, `time`, hands the real program as its second word, so `if true;
then git push --force; fi` passed both the old gate and build 319's on a compiled `standard` gate.
The same shape works with `while :; do`, `else`, and `{ git push --force; }`.

## What

In `src/policy/gate.py`, where `floor_violation` reads a segment's binary (and
`protected_path_named_by` where it reads tokens), skip the shell's reserved words and the
constructs that precede a simple command before taking the binary: `if`, `then`, `elif`, `else`,
`fi`, `for`, `while`, `until`, `do`, `done`, `case`, `esac`, `{`, `}`, `!`, `time`, `function`,
and a `name=value` assignment prefix (which `first_word` from build 319 already skips). Also
`command`, `builtin`, `exec`, `nohup`, `env` with its assignments, `nice`, `timeout <n>`,
`xargs`: wrappers whose argument is the program. Read what `shell_hazard` already covers before
adding a list, and share one.

The corpus under `tests/corpus/` must keep refusing what it refuses; `tests/gate.rs` gets one
case per construct above, each red before the fix, the `if true; then git push --force; fi` one
first. Measure the result on a compiled gate as 319 did and paste the table in the Outcome.

## Do

1. Read build 319's Outcome and `shell_layout`'s docstring in `gate.py`.
2. The keyword and wrapper skip, as one function the two passes call.
3. The cases; the four checks.

## Done means

`cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo
nextest run --release --workspace`, `cargo test --release --workspace --doc` green. Amend
`IMPLEMENTATION.md` where the pattern floor is described, beside 319's sentences.

Rules inside the run: relative paths only, never look outside your own worktree, write files with
your file tool and not with shell heredocs, and the test suite is green inside a run since build
302 without any `env -u`.

## Outcome

Built `shell_command_words` in `src/policy/gate.py`, shared by `floor_violation` and
`protected_path_named_by`. It separates a segment's prefix from the program and arguments:
leading reserved words, assignments, function declarations, and the eight named wrappers,
including nested wrappers, option values, `--`, and `timeout`'s duration. The protected-path
pass still checks both pieces because an assignment or wrapper option can name a denied path.
`command -v` and `command -V` inspect names rather than execute them. Ordinary arguments are
not read as prefixes. `shell_hazard` has no keyword/wrapper list; it checks expansions and
operators for the bounded shell, so the new lists live only in the shared reader.

Added three tests in `tests/gate.rs`: 32 compound/wrapper floor probes (the requested `then`
probe first), eight allowed-command controls, and five relative protected-path probes that
exercise the token pass. No existing test or corpus assertion changed. The gate file now
has 130 tests, all passing (8.39 s for the focused release run).

Measured using the compiled `standard` gate from the test rig, with synthetic hook payloads
on `/usr/bin/python3`, never executing the commands below. The baseline release test binary
embedded the seeded template; the rebuilt binary embedded this change. `A` means allow and
`F` means `hydra/floor`.

| Probe | Before | After |
|---|---|---|
| `if true; then git push --force; fi` | A | F |
| `if git push --force; then :; fi` | A | F |
| `if false; then :; elif git push --force; then :; fi` | A | F |
| `if false; then :; else git push --force; fi` | A | F |
| `if true; then :; fi; git push --force` | F | F |
| `for x in one; do git push --force; done` | A | F |
| `while :; do git push --force; done` | A | F |
| `until git push --force; do :; done` | A | F |
| `while git push --force; do :; done` | A | F |
| `for x in one; do :; done; git push --force` | F | F |
| `case x in x) git push --force;; esac` | F | F |
| `case x in x) :;; esac; git push --force` | F | F |
| `{ git push --force; }` | A | F |
| `{ :; }; git push --force` | F | F |
| `! git push --force` | A | F |
| `time git push --force` | A | F |
| `function f { git push --force; }; f` | A | F |
| `f() { git push --force; }; f` | A | F |
| `MODE=one git push --force` | A | F |
| `command git push --force` | A | F |
| `builtin exec git push --force` | A | F |
| `exec git push --force` | A | F |
| `nohup git push --force` | A | F |
| `env MODE=one git push --force` | A | F |
| `nice git push --force` | A | F |
| `timeout 10 git push --force` | A | F |
| `xargs git push --force` | A | F |
| `if true; then command env MODE=one nice -n 5 timeout -k 2 10 git push --force; fi` | A | F |
| `exec -a worker git push --force` | A | F |
| `env -u MODE -- git push --force` | A | F |
| `xargs -I ITEM -n 1 git push --force ITEM` | A | F |
| `time -p command -- aws configure` | A | F |

Totals: 27 allows became floor refusals; five floor refusals stayed refused. All eight ordinary
controls were allowed before and after; all five protected-path controls were
`hydra/protected-path` before and after. The baseline floor test failed on exactly those
27 newly covered probes, and the rebuilt gate passes all 32.

What the ticket assumed that was false:

- Every construct cannot have a valid shell example red before the fix: commands after `fi`,
  `done`, `esac` or `}` already start a separate segment, and a `case` arm's `)` already
  separates its body. Those five probes were already refused. They remain regression guards.
- `first_word` skipped assignments for heredoc consumer classification, but the floor did
  not call it. Assignment prefixes therefore hid a command from the floor too.
- The protected-path token pass already checked every token, so keywords and wrappers did
  not hide paths from it. Sharing the reader must preserve prefix tokens, not discard them.

`IMPLEMENTATION.md` now describes the shared reader beside 319's paragraph and closes the
keyword hole. Script files written in one call and executed in another remain outside this
spelling check, as before. This is not a complete wrapper interpreter: for example, it does
not unpack `env -S` command strings. No gate tool call was refused during this work.

Required checks, all exit 0:

- `cargo fmt --all --check`: no formatter output.
- `cargo clippy --workspace --all-targets -- -D warnings`:
  `Finished dev profile [unoptimized + debuginfo] target(s) in 31.69s`.
- `cargo nextest run --release --workspace`:
  `Summary [  53.861s] 1412 tests run: 1412 passed, 4 skipped`.
- `cargo test --release --workspace --doc`:
  `test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s`.

`cargo build --release --workspace` also passed (32.54 s). Changes are left uncommitted.
