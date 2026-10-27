# 368 - One table of secret classes in a leaf crate, read by the write path, export and `hydra-records`

Type: build
Status: resolved
Blocked by:

## Objective

Carry out the first half of map ticket 108, `.scratch/hydra/issues/108-one-list-of-secret-classes.md`, with the panel's reasoning in `.scratch/hydra/research/108-synthesis.md`. Read both first.

The same secret classes are written out four times today:

- `src/record/secrets.rs` line 11, the write-path sweep, five classes;
- `src/export.rs` around line 632, export's redactor;
- `src/export.rs` `scan_text` around line 765, export's verifier;
- `crates/records/src/redact.rs` line 16, `PREFIXES`, which the grader, apprentice and otel use.

## Do

1. Add a crate `crates/secret-classes`, package `hydra-secret-classes`, with no dependencies and `publish = false`. It holds data only:
   - the twelve `(prefix, label)` pairs in the order `crates/records/src/redact.rs` has them, since that order is the Python's replacement order and the differential tests pin it;
   - a flag on the five the write path scans: `AIza`, `sk-ant-`, `ghp_`, `ya29.`, `-----BEGIN`, with the labels `src/record/secrets.rs` uses today;
   - the OpenAI span constants: body lengths 20 and 32, and the kinds `proj-`, `svcacct-`, `admin-`, as `src/export.rs` and `crates/records/src/redact.rs` define them now.
   Keep matching code out of it. `hydra-records` counts Python code points and `src/export.rs` counts bytes, and each keeps its own matcher.
2. Point each of the four lists at the table, and delete the local copies. `src/record/secrets.rs` scans only the flagged rows. Add the crate as a dependency of the root `hydra` package and of `crates/records`.
3. A test in the new crate pins the flagged set to exactly those five, so a widening shows up as a failing test and needs a map decision. A test in `src/record/secrets.rs` checks that `scan` flags the same five labels as before on the same inputs.
4. Records stay byte-identical. Run `cargo nextest run --release -p hydra --test fixture_corpus --test record_contract --test export` and the grader and records differential tests, `cargo nextest run --release -p hydra-records -p hydra-grader`, unchanged.
5. `record::secrets::prefixes()` has no caller. Leave it for build 369, which owns the cockpit.
6. Amend `IMPLEMENTATION.md` where it names the secret lists, and `CLAUDE.md`'s layout paragraph only if it lists crates. Do not touch the contract or `SECURITY.md`; build 369 does.
7. Run `cargo fmt --all --check`, `cargo clippy --release --workspace --all-targets -- -D warnings`, `cargo build --release --workspace` and the tests above.

Builds 366 and 367 run beside you in `src/sandbox.rs`, `src/paths.rs`, `src/reap.rs` and `src/worktree.rs`. Do not touch those files.

## Outcome

Built. `crates/secret-classes` is package `hydra-secret-classes`, with no dependencies.
The table holds the twelve prefixes in the Python replacement order, a write-path flag on
five of them, and the OpenAI body lengths and kinds. Matching stays in each reader.
`src/record/secrets.rs` scans only the flagged rows. Export's redactor and verifier and
`crates/records/src/redact.rs` iterate the same table. The four local lists are gone.
`record::secrets::prefixes()` still returns the flagged five and still has no caller.
A unit test in the new crate pins the flagged set. A unit test in `secrets.rs` pins the
same five labels on one input, and that three export-only prefixes flag nothing.
`CLAUDE.md`'s layout paragraph does not list crates, so it was left alone.

Measured. `cargo nextest run --release -p hydra-secret-classes` passed 1 of 1.
`cargo nextest run --release -p hydra record::secrets` passed 4 of 4.
`cargo nextest run --release -p hydra --test fixture_corpus --test record_contract --test export`
passed 57 of 57, one skipped.
`cargo nextest run --release -p hydra-records -p hydra-grader` passed 96 of 96, including
`the_redactor_gives_what_the_python_gave`.
`cargo fmt --all --check` is clean.
`cargo clippy --release --workspace --all-targets -- -D warnings` is clean.
`cargo build --release --workspace` is clean.
