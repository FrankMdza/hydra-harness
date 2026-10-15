# 321 - A refused shell command is recorded whole, not as the provider's truncated log line

Type: build
Status: resolved
Blocked by:
Opened: 2026-10-15, by the session, from map 120's answer. The session read run `01a1332c`'s
record to learn what it was refused for: the refused command at seq 19 is `mkdir -p src/sources;
cat > src/sources/mod.rs <<'EOF'` and then nothing, because codex's log truncates the command at
63 characters and the record copies the log. Nobody can prove what the body said. A refusal
record that does not carry the command it refused cannot be audited, replayed or used as a
training row.

## What

The gate hook sees the whole command in its payload before it refuses. Record it from there:
when the hook refuses a shell command, the refusal event on the record carries the full command
text as the hook received it, whatever the provider's own stream later prints. Read
`docs/run-record-contract.md` for the refusal event's shape and section 15 for adding a field;
the new field is additive and older records stay valid.

The hook runs inside the run and cannot write to `~/hydra/runs/`; find how refusals reach the
record today (the hook writes somewhere under the spawn directory or signals the watcher, read
`src/spawn_config.rs` and `src/watch.rs`) and carry the command text the same way. Cap it at a
size you name (64 KiB is a reasonable first cap) and record that it was capped when it was.

## Do

1. The hook's refusal path writes the command; the watcher or the normalizer puts it on the
   event; `hydra show --events` prints it.
2. `tests/gate.rs` or `tests/spawn.rs`: a refused command of 200 characters with a heredoc body
   arrives whole on the record; one over the cap arrives capped and marked.
3. The fixture corpus gets one refusal with the new field, regenerated and guarded by
   `fixture_corpus.rs`.

## Done means

`cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo
nextest run --release --workspace`, `cargo test --release --workspace --doc` green. Amend
`docs/run-record-contract.md` (additive, dated) and `IMPLEMENTATION.md` where refusals are
described.

Rules inside the run: relative paths only, never look outside your own worktree, write files with
your file tool and not with shell heredocs, and the test suite is green inside a run since build
302 without any `env -u`.

## Outcome

Built a live-hook sidecar at `spawn/<run-id>/gate-refusals.jsonl`. The watcher and
interactive harvest copy its entries into stored `gate_refusal` events before `run_end`;
all four normalizers preserve them and `hydra show --events` prints the command inside
the untrusted-content boundary. Each event carries the tool, optional call id, reason,
command and `command_truncated`. The cap is 65,536 UTF-8 bytes before JSON escaping,
cut at a character boundary. Replay does not append evidence. Sidecar write failures
leave the denial intact; damaged evidence is noted without preventing record closure.
The seatbelt grants the hook one literal write exception, while tool-level spawn-path
denials remain in force. Evidence is collected at close and is not tamper-proof against
a process using that kernel write allowance.

Added two gate tests and one fixture guard, plus one generated refusal fixture. Measured:
the 200-character ASCII heredoc arrived as 200/200 bytes; an 80,036-byte Unicode command
arrived as 65,536 bytes, omitting 14,500 bytes and marking truncation. Evidence survived
deleting the sidecar and remained visible through show. Watcher stubs exercised all 4/4
providers without launching a vendor or spending quota. The fixture generator rewrote
the corpus and its guard passed.

The ticket's suggested existing hook-to-watcher path was absent: refusals previously
reached the record through provider output alone. A new sidecar and additive event were
needed. Existing derived permission decisions keep their counts and attribution; old
records remain valid at schema version 1. Contract section 7.7/15 and IMPLEMENTATION.md
now describe the addition. No tool call was refused by this run's gate. No commits made.

Validation, without `env -u`:

- `cargo fmt --all --check`: exit 0, no output.
- `cargo clippy --workspace --all-targets -- -D warnings`:
  `Finished dev profile [unoptimized + debuginfo] target(s) in 9.16s`
- `cargo nextest run --release --workspace`:
  `Summary [  59.909s] 1406 tests run: 1406 passed, 4 skipped`
- `cargo test --release --workspace --doc`:
  `test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s`
- `cargo build --release`: green, 0.27s. `git diff --check`: green.
