# 316 - `run_start` carries the host it ran on, and `sandbox` the kind of profile its sha hashed

Type: build
Status: resolved
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

## Outcome

Built additive, schema version 1, as decided. `record::Host { os, arch, machine }` and
`record::hostname()` (`libc::gethostname`, `None` on an error or an empty answer) sit in
`src/record/lines.rs`. `RunStart` gains `host` and `sandbox_kind`, both skipped when `None`. Four
writers fill `host`: `run::run_start` (recorded and refused launches), both interactive paths and
the `evolve` harness pass. `sandbox_kind` is `"seatbelt"` wherever `sandbox` holds a sha, which
today means `run::run_start` and the gated interactive path. The validator puts `sandbox_kind` in
`LATER_RUN_START_FIELDS` as non-null text, checks `host` against `HOST_LEAVES` the way it checks
`auth`, and refuses a kind beside a `sandbox` that holds no sha. `walk::hosts_of` and
`walk::Report::hosts` count records per host, and `hydra doctor --walk` prints `host <machine>
<os>/<arch>: N record(s)` per host before its count. `hydra show --events` needed no code: the
`run_start` body is printed verbatim, so the host rides along.

Tests: four in `record_contract.rs` (with and without each field, null host, each missing leaf,
a non-string leaf, a kind beside a null or absent sandbox, a null kind, and the writer's own host
read back); `delegation.rs` checks `run_start()` fills `host`, leaves the kind absent with no
sandbox and writes `seatbelt` with one; `show.rs` prints the host and prints none without it;
`normalize.rs` reads `sandbox_kind` and `sandbox` back on all four providers and counts the
fixtures' hosts. Fixtures 1, 7, 8, 9 carry `host` (`stub-host`, macos/aarch64), a synthetic
`sandbox` sha and `sandbox_kind`; harness fixture 10 carries `host` only. Regenerating changed
line 0 of those five files and nothing else. `fixture_corpus.rs` gains a byte-for-byte guard over
every fixture against its builder, plus the check that one fixture per provider names
`stub-host` and none names `gethostname()`.

Measured: `cargo nextest run --release --workspace` 1427 run, 1427 passed, 4 skipped, 134.8 s.

**The `hydra doctor --walk` of Do step 4 could not be measured inside the run.** It printed
`checked 0 record(s)` and exited 0: the run's seatbelt keeps `~/hydra/runs/` unreadable and
`walk::records_in` turns the `read_dir` error into an empty list. The session has to run it
outside a run. It should find nothing new, because no record written before today carries
either field and both are type-checked only when present.

Measured by the session on the Mac after applying, 2026-10-15, with the binary of this build:
`hydra doctor --walk` reports `checked 958 record(s)` and the same findings as before (six "owes
one decision per call", six `Ungated`, one "failed with exit 0"), nothing new. It prints no
`host` line yet because no record on this machine carries the field; the first run after this
build does.

What was false or moved:
- The ticket's "Reader: `src/normalize/*.rs`, `src/show.rs`" needed no code. Every normalizer
  and `show` pass the `run_start` body through verbatim.
- Contract 5.5's count paragraph was stale before this ticket. Build 68's `sandbox`, which is null
  on a harness run, was never counted. The paragraph now reads eight of twenty-seven filled.
- Build 336 item 2 asks for a per-host count. This ticket's walk line is half of that, and 336 now
  says so.
- `sandbox.kind` also survives in map 119's line in `map.md`, in issue 119 and in the panel
  research files. Only issue 126 was amended, as the ticket asked; the rest is history.
