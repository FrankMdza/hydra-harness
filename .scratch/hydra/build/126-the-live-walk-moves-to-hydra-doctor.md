# 126 - The walk over this machine's records moves to `hydra doctor --walk`

Type: build
Status: open
Blocked by: 125

## What

Map ticket 80, resolved 2026-09-24. Read its `## Answer` first.

Several test files walk every record under `~/hydra/runs/` inside `cargo test`, beside a fixture
walk over `tests/fixtures/records/` that runs the same assertions. On 2026-09-24 the live walk
went red four times on vendor behaviour (build tickets 113, 116, 117, 125) and never on a Hydra
regression. The answer: the fixture walks stay in the suite, the live walks become a command.

## Do

1. Find every live walk: `grep -ln "Layout::discover\|HYDRA_ROOT" tests/*.rs` names seven files
   today (delegation, face, launch_phases, evolve, normalize, gate, spawn), and `outcome.rs` and
   `record_contract.rs` walk too. Read each hit. Some use the layout to launch a stub rather than
   to walk the corpus; those stay. List what moved and what stayed in `## Outcome`.
2. The assertions a live walk makes move into the library (a `walk` module, or beside
   `src/doctor.rs`), so the command and the fixture tests call the same function over different
   directories. A fixture test that passes today still passes, unchanged in what it asserts.
3. `hydra doctor --walk` runs that function over the live corpus. It prints one line per
   finding naming the record path and the assertion, a count of records read, and exits
   non-zero if there is any finding. Plain `hydra doctor` without the flag behaves as it does
   today.
4. Documents: README.md's paragraph on what CI proves (near lines 226 to 233) says the live walk
   is `hydra doctor --walk` and CI never runs it. `IMPLEMENTATION.md` and `CLAUDE.md`'s test
   list: say where the walk went. `~/.claude/handoffs/2026-09-23-builds/full-suite.sh` is
   outside the repo; say in `## Outcome` the line it needs, and the session adds it.

## Done when

- `cargo test` on this machine and on CI reads no file under `~/hydra/runs/`. Show how you
  checked (for example, the suite passing with `HYDRA_ROOT` pointed at an empty directory, if
  the layout honours it).
- `hydra doctor --walk` on this machine exits 0 once 125 is in. A test plants one bad record
  (the `failed with exit 0` shape with a terminator, or whatever 125 left as the refused case)
  in a scratch corpus and reads the non-zero exit and the record path back.
- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test` pass.
