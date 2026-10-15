# 321 - A refused shell command is recorded whole, not as the provider's truncated log line

Type: build
Status: open
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
