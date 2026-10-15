# 316 - `run_start` carries the host it ran on, and `sandbox` the kind of profile its sha hashed

Type: build
Status: claimed
Blocked by:
Opened: 2026-10-15, by the session, from map 119's answer. Two machines will write records into
one corpus one day, and today nothing on a record says which machine wrote it: `policy_sha`
hashes the same bytes on both at one commit, `base_tree` names content and not a host, and
`sandbox: null` means "no wrapper ran" without saying whether that was a choice or a platform.
The panel wanted both facts on `run_start`, under the contract's rule for a new field.

## What

Read `docs/run-record-contract.md` section 15 (Versioning) before touching the writer: a new
field is additive, older records without it stay valid, and the validator in `src/record/`
accepts both. Then:

- `run_start.host`: an object `{os, arch, machine}`. `os` and `arch` are
  `std::env::consts::OS` and `ARCH`; `machine` is the hostname (`libc::gethostname` or the
  `hostname` crate if it is already a dependency, otherwise read `/proc/sys/kernel/hostname` on
  Linux and `sysctl kern.hostname` on macOS through the existing `procinfo` style). Never the
  user name.
- `run_start.sandbox`: today a sha or null. Make it an object `{kind, sha}` where `kind` is
  `"seatbelt"` on macOS, with the old shape still read by the normalizers (a bare string is
  `kind: "seatbelt"` of that sha). Write the new shape only; read both.

`hydra doctor --walk` and `hydra show` print the host when the record has one. The fixture corpus
under `tests/fixtures/records/` gets the new fields on one record per provider, and
`fixture_corpus.rs` regenerates and guards them as it does today.

## Do

1. Writer: `src/run/` where `run_start` is composed. Reader: `src/normalize/*.rs`, `src/show.rs`,
   `src/walk.rs`. Validator: `src/record/`.
2. Tests: `tests/record_contract.rs` accepts a record with and without `host`, and with `sandbox`
   as a string and as the object; `tests/show.rs` prints the host; `tests/normalize.rs` reads
   both sandbox shapes back to the same value.
3. Contract: amend `docs/run-record-contract.md` where `run_start` is listed, as an additive field
   under section 15's rule, with the date.

## Done means

Fixtures regenerated and guarded, the four test files above green, `cargo fmt --all --check`,
`cargo clippy --workspace --all-targets -- -D warnings`, `cargo nextest run --release
--workspace`, `cargo test --release --workspace --doc`. Amend `IMPLEMENTATION.md` where the
record's fields are described.

Rules inside the run: relative paths only, never look outside your own worktree, write files with
your file tool and not with shell heredocs, and the test suite is green inside a run since build
302 without any `env -u`.
