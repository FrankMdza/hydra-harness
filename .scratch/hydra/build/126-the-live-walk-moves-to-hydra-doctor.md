# 126 - The walk over this machine's records moves to `hydra doctor --walk`

Type: build
Status: resolved
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

## Outcome

Resolved 2026-09-24. Built by `composer-2.5` on run 01a0d57a-ab27 under `standard`, about 4.42M
tokens, 4.26M of them cache reads. The session applied the diff as written.

Nine live walks left the suite: `every_record_on_this_machine_meets_the_contract` in
`record_contract.rs`; five in `normalize.rs` (`every_record_on_this_machine_normalizes`,
`the_recorded_write_denial_derives_to_a_deny_under_capability`,
`no_record_on_this_machine_shows_a_gate_that_leaked_or_stayed_silent`, `normalizing_writes_nothing`,
`the_recorded_shell_denial_and_the_call_beside_it_derive_apart`); three in `outcome.rs`
(`every_record_on_this_machine_derives_an_outcome`,
`the_recorded_write_denials_are_the_only_gated_runs_on_this_machine`,
`the_one_unclassified_result_on_this_machine_is_the_only_unknown`). Their assertions are
functions in `src/walk.rs` that return findings instead of panicking. The fixture walks call
the same functions over `tests/fixtures/records/`. The other files the ticket's grep named use
`HYDRA_ROOT` to launch stubs, not to walk, and stayed.

`hydra doctor --walk` prints one line per finding (record path, assertion), then `checked N
record(s)`, and exits 1 on any finding. Plain `hydra doctor` is unchanged. `tests/doctor.rs`
plants a clean record and a `failed`-with-terminator-at-exit-0 record in a scratch corpus and
reads exit 0 and exit 1 back. CI lost its live corpus-count step, which could only ever print
`checked 0`.

Measured by the session on master: `hydra doctor --walk` reads 429 records and exits 0. With
build 125's no-terminator exemption switched off in `src/walk.rs`, it exits 1 and names two
records, 01a0d484-2d8c (the codex panel seat) and 01a0d56e-ba1b (another session's run, the same
shape), both `failed with exit 0`. Restored, it exits 0. `cargo test --test normalize` went from
36 s to 0.6 s. lib 210, doctor 2, normalize 87, outcome 44, record_contract 20, all passing, and fmt
and clippy are clean.

One loss: the port dropped most of the comments that said why each assertion exists. The three
test files lose 105 comment lines and `src/walk.rs` has 20. The code matches, and the session
checked the leak walk line by line against the original. Build ticket 127 brings the comments back.
The session updated `CLAUDE.md`'s test list and added the walk to
`~/.claude/handoffs/2026-09-23-builds/full-suite.sh`, which now runs `hydra doctor --walk` after a
green `cargo test` and prints red if either fails.

Where the code is: `git apply --3way` staged the diff in the shared index, and a concurrent
session's commit `88f59b4` ("puppets: a plan approval is about the plan...") took the staged
files with its own: `src/cli.rs`, `src/doctor.rs`, `src/lib.rs`, `src/main.rs`, the three test
files, `README.md`, `IMPLEMENTATION.md` and `.github/workflows/ci.yml`. `06b3987` holds
`src/walk.rs`, `tests/doctor.rs` and the tickets. Read both together for build 126. The session
did not rewrite `88f59b4`, which belongs to the other session.
