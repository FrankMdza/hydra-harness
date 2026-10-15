# 316 - `run_start` carries the host it ran on, and `sandbox` the kind of profile its sha hashed

Type: build
Status: open
Blocked by:
Opened: 2026-10-15, by the session, from map 119's answer. Two machines will write records into
one corpus one day, and today nothing on a record says which machine wrote it: `policy_sha`
hashes the same bytes on both at one commit, `base_tree` names content and not a host, and
`sandbox: null` means "no wrapper ran" without saying whether that was a choice or a platform.
The panel wanted both facts on `run_start`, under the contract's rule for a new field.

## What

**Decision, 2026-10-15, the human on panel 130's recommendation: additive, schema version stays 1.**
The first run of this ticket (Opus, worktree `01a13dff-4745`, 27 files) followed the What as it
was first written, re-typed `sandbox` into an object and bumped the schema to 2, which is what
contract section 15 says a re-typed field costs. The instruction was wrong, not the run; the
worktree was reaped without applying. Nothing reads `run_start.sandbox` today outside `src/record/`.

Read `docs/run-record-contract.md` section 15 (Versioning) before touching the writer: a new
field is additive, older records without it stay valid, the validator in `src/record/` accepts
both, and a group's leaves are checked too. Then:

- `run_start.host`: an object `{os, arch, machine}`, all three strings, all three required when
  `host` is present. `os` and `arch` are `std::env::consts::OS` and `ARCH`; `machine` is the
  hostname through `libc::gethostname`. Never the user name. The validator rejects `host: null` and
  a `host` missing a leaf.
- `run_start.sandbox_kind`: a sibling of `sandbox`, a string, `"seatbelt"` when a seatbelt profile
  wrapped the launch, absent otherwise. `sandbox` stays the sha-or-null it is. Absent means
  unknown; a Linux writer adds its own kind with build 318.

`hydra doctor --walk` and `hydra show` print the host when the record has one. The fixture corpus
under `tests/fixtures/records/` gets the new fields on one record per provider, with a synthetic
`machine` (`"stub-host"`), never this machine's hostname, since those files go public;
`fixture_corpus.rs` regenerates and guards them and asserts no fixture `machine` equals
`gethostname()`.

## Do

1. Writer: `src/run/` where `run_start` is composed. Reader: `src/normalize/*.rs`, `src/show.rs`,
   `src/walk.rs`. Validator: `src/record/`.
2. Tests: `tests/record_contract.rs` accepts a record with and without `host` and with and without
   `sandbox_kind`, rejects `host: null` and a `host` missing a leaf; `tests/show.rs` prints the
   host; `tests/normalize.rs` reads `sandbox_kind` back and leaves `sandbox` as it was.
3. Contract: amend `docs/run-record-contract.md` where `run_start` is listed, two additive fields
   under section 15's rule, with the date. Amend map 126's wording (`sandbox.kind`) to
   `sandbox_kind` where the ticket quotes it.
4. After the four checks, `hydra doctor --walk` over every record on this machine must report
   nothing new; paste its last lines in the Outcome.

## Done means

Fixtures regenerated and guarded, the four test files above green, `cargo fmt --all --check`,
`cargo clippy --workspace --all-targets -- -D warnings`, `cargo nextest run --release
--workspace`, `cargo test --release --workspace --doc`. Amend `IMPLEMENTATION.md` where the
record's fields are described.

Rules inside the run: relative paths only, never look outside your own worktree, write files with
your file tool and not with shell heredocs, and the test suite is green inside a run since build
302 without any `env -u`.
