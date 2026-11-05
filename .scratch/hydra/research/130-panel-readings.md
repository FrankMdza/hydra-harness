# Map 130, readings for the panel: what the human still has to decide on 2026-10-15

The human asked for every open item in one place, each written three ways before any
recommendation: the problem or the objective, how it is being solved, and the test that proves it
solved. The panel reads this file, checks the claims against the repo, and recommends per item.
The decisions stay the human's.

Numbers in this file were measured on 2026-10-15 unless a date says otherwise. The repo is at
`e10c63a` (build 330). The suite on macOS: 1419 tests, 4 skipped, all green.

## Item 1. The Linux CI job is red: 51 tests in 10 test binaries

**Problem.** Build 315 made Hydra compile on Linux and added a `linux` job to
`.github/workflows/ci.yml` that runs `cargo test --release --workspace --no-fail-fast -- --skip
sandbox` on `ubuntu-latest`. Its first run, on the push of `e10c63a`, failed in 4 minutes with 51
red tests. The `check` job on macOS stayed green. The failures are not one bug; they are eight
assumptions the suite makes about the machine it grew up on. Until the job says something true, a
visitor's first click shows red.

**The eight groups, from the log of run 38695261071.**

| Group | Tests | Where | What the log says | Reading |
|---|---|---|---|---|
| a | 30 of 32 in `tests/seats.rs` | `tests/seats.rs:47` | `creating /private/tmp/hs12343-0/runs: Permission denied` | The scratch root is spelled `/private/tmp` for `sun_path`'s 104-byte limit. On Linux `/private` does not exist and cannot be created. |
| b | 7 in `tests/spawn.rs`, several in `seats.rs` | `tests/corpus/mod.rs:675` | `sandbox-exec is required on CI (macOS)` | The guard panics whenever `CI` is set and `sandbox-exec` is missing. It was written so a macOS runner cannot silently skip the seatbelt tests. GitHub sets `CI=true` on Linux too, and the test names do not contain `sandbox`, so `--skip sandbox` does not reach them. |
| c | 1 in `tests/git_config.rs` | `tests/git_config.rs:353` | `git commit -q --no-verify -m reap failed: Author identity unknown` | `commit_everything` takes `user.name`/`user.email` from `identity()` (`src/worktree.rs:266`), which reads them as data from the repo's configuration at every level. The runner has no `~/.gitconfig`, the throwaway repo has none, and git's own guess from the runner's hostname is refused. On the Mac the human's global config supplies the name. A fresh machine without a git identity cannot reap. |
| d | 1 in `tests/gate.rs` | `tests/gate.rs:3464` | `hydra/gate-error: the gate raised ValueError while evaluating this call` | A single-call payload whose `cwd` holds a NUL. The test asserts the answer measured on 2026-09-22 on macOS (`/usr/bin/python3` is 3.9.6): allowed. Ubuntu's python3 is 3.12, and newer `os.path` functions raise `ValueError` on an embedded NUL. The gate refuses when it cannot decide, which is the designed fallback; the test pins the older behaviour. |
| e | 1 in `tests/pty_tee.rs` | `tests/pty_tee.rs:174` | `a frame was cut at a read boundary` | A 200,000-byte JSON line through the pty arrives in pieces and one piece is parsed alone. The tee's reassembly works for the chunk sizes a macOS pty delivers. Linux ptys deliver 4 KiB chunks. Unmeasured whether the tee or the test is wrong. |
| f | 1 in `tests/detached.rs` | `src/record/validate.rs:31` | `line 0: a Hydra line of type ""` | `a_grandchild_holding_the_slave_closes_the_record_within_the_deadline`. Its own comment says the held-slave harness exists because macOS revokes inherited pty fds and Linux does not. On Linux the record's first line came out with an empty type. Unmeasured. |
| g | 4 in `crates/grader/tests/differential.rs`, 2 in `crates/puppets` (`sheet::tests::lgamma_is_cpythons_and_not_the_c_librarys`, `the_ranking_arithmetic_matches_python`), 3 in `crates/pyjson/tests/math_matches_python.rs` | `crates/puppets/src/sheet.rs:644` and others | `lgamma(4) left: 1.7917594692280554 right: 1.791759469228055`; `lgamma differs on 4566 of 18000 arguments`; `input block 0 hash differs` | The Rust ports of CPython's `lgamma`, `log1p` and friends were verified against CPython on this Mac, and the pyjson hashes were recorded there. On glibc the last bit differs, and the differential against the runner's live CPython differs on a quarter of the arguments, so the port reproduces CPython as built on macOS (Apple libm, fused multiply-add) and not CPython. |
| h | 1 in `crates/puppets/tests/differential.rs` (`the_launch_commands_match_puppets_py...`, 22 of 87 files differ) and 1 grader differential (`collect::the_labels_are_what_the_python_gave`) | `crates/puppets/tests/differential.rs:3863`, `crates/grader/tests/differential.rs:147` | `stub/calls/hydra-show-1.env differs` with `__CF_USER_TEXT_ENCODING=...` on the expected side; `/bin/cat: ... No such file` against `cat: ...` | Recorded expectations carry a macOS-only environment variable, and a stub's error message prints the program as the shell names it (`/bin/cat` under dash, `cat` under zsh). |

**How it is being solved.** Nothing yet. Three ways are on the table and the human picks:

1. Narrow the job today to what is true: `cargo build --release --workspace` plus `cargo test
   --release --lib --workspace`, and the README's sentence "compiles on Linux, untested at runtime
   there" stays. Then one build ticket per group, each widening the job by one test binary when
   it is green. Honest and green within the hour.
2. Leave the job red and open the eight tickets. Honest about the gap, red on every visitor's
   first click until the last one lands.
3. Fix the eight groups before the repository goes public, with the job as the gate.

Groups a, b and h are test-only fixes. Group c is a product fix: reap needs a fallback identity
when none is configured, and whose name goes on that commit is a decision. Group d is a decision:
which answer to a NUL `cwd` is right, and the test then asserts it on both Pythons. Groups e and f
need a Linux machine to measure; the human has a VM (map 119). Group g is a decision too: the
ports can match CPython per platform, with the hashes recorded on each, or the ranking can accept
a one-ulp tolerance and the test can say so.

**The test that proves it.** The `linux` job itself, green, with the README's Linux sentence
matching what the job runs. Per group: a test that is red on `ubuntu-latest` today and green
after, named in the ticket. For group c the test is `tests/git_config.rs` run in a child with no
`HOME` git config, which is what the runner already does.

## Item 2. Build 316: the record says which machine wrote it, as schema version 2 or as an additive field

**Problem.** Two machines will write records into one corpus (map 119). Today nothing on a record
says which machine wrote it, and `sandbox: null` does not say whether no wrapper ran by choice or
because the platform has none. Map 119's panel wanted `run_start.host` and a sandbox kind.

**How it is being solved.** Build 316 ran on Opus and sits unapplied in worktree
`01a13dff-4745-7361-a3b5-d3dc98cf6d74`, 27 files. It writes `host: {os, arch, machine}` through
`libc::gethostname` and turns `sandbox` into an object `{kind, sha}`, bumping the schema version
to 2 for new writers; the validator and the four normalizers accept both shapes, four fixtures are
regenerated, six tests added. The alternative is additive: a sibling field `sandbox_kind` beside
the sha, no version bump, and `host` as the ticket says. The contract's section 15 says a new field
is additive and older records stay valid; reshaping an existing field is what the version is for.
The session's recommendation is additive, because the only reader that gains from the object
shape is a future Linux writer, and a version bump makes every reader of 952 records carry two
shapes forever.

**The test that proves it.** `tests/record_contract.rs` accepts a record with and without `host`,
and with `sandbox` in both shapes; `tests/normalize.rs` reads both shapes to the same value;
`fixture_corpus.rs` regenerates and guards the fixtures; and `hydra doctor --walk` over the 952
records on this machine reports no new finding. The last one is the test that no old record broke.

## Item 3. Build 326: what reap does with a repository nested in a checkout

**Problem.** Build 325 measured it: when a run initializes a git repository inside its checkout
and stages it, `hydra reap` exits 128 (`'sub/.git' not recognized as a git repository`) and leaves
the checkout behind; when the run leaves it unstaged, reap commits a gitlink of mode 160000, `git
worktree remove` exits 128, and three unreferenced objects land in the parent. Both cases are red
tests in `tests/cleanup.rs` today, asserting what happens rather than what should.

**How it is being solved.** Two treatments, the ticket says "Decision: pending" and is not
takeable until the human's word is on it. Treatment one: before `commit_everything`, reap finds
every index entry of mode 160000 and every directory holding a `.git`, renames the nested `.git`
to `.git.nested`, drops the gitlink, adds the directory as plain files, and prints one line per
path. Treatment two: reap refuses before any commit, names each path, and leaves the checkout for
the human. The session recommends treatment one, because a refusal leaves the checkout behind
exactly as today's exit 128 does, and the work inside the nested repository is the run's work.

**The test that proves it.** The two 325 cases flipped to the chosen behaviour, a third case with
two nested repositories, and for treatment one an assertion that the parent's object store holds
no unreferenced objects after reap (`git fsck --unreachable` empty), which is what makes "the
files land as plain files" checkable.

## Item 4. Build 321, for review: the refused command recorded whole, through a sidecar the hook writes

**Problem.** Before 321, a refusal reached the record only through the provider's own output,
truncated or paraphrased, so an audit could not say what command was refused.

**How it was solved.** The hook appends each refusal to `spawn/<run-id>/gate-refusals.jsonl`;
the watcher and the interactive harvest copy the entries into stored `gate_refusal` events before
`run_end` (eleventh event type, additive, schema version 1). Fields: `tool`, `call_id`, `reason`,
`command`, `command_truncated`, capped at 65,536 UTF-8 bytes. The seatbelt grants the hook one
literal write exception for that file. The Outcome says it plainly: the evidence "is not
tamper-proof against a process using that kernel write allowance". A run's shell can append a
forged line, or truncate the file, through the same exception.

**The test that proves it.** Two gate tests and one fixture guard exist, and the Outcome measured
a 200-byte heredoc arriving whole and an 80,036-byte command cut at 65,536 with the flag set. What
does not exist: a test that plants a forged line in the sidecar during a run and reads back what
the record says about it. The decision is whether "written by the hook, not tamper-proof" is
accepted and written into the contract as the evidence's standing, or whether the hook has to sign
its lines (a per-run secret the hook holds and the run cannot read), which is a build.

## Item 5. Build 320, for review: three facts in the refusal text, none in JSON fields

**Problem.** A refusal said "protected path" without saying which deny entry matched, which token
of the command named it, or which symlink it crossed.

**How it was solved.** The reason text now names the longest matching deny entry, the token as
written, and the link crossed when there was one. The ticket also asked for the same three facts as
JSON fields where the reply is JSON. Not done: harness spec 6.6 records that `claude` allows a
call whose hook reply shape it does not recognise, and no provider's tolerance of an extra key is
measured, so an extra key could turn a deny into an allow.

**The test that proves it.** `tests/gate.rs` has three new cases, one per fact, and reverting
`longest` to first-match fails exactly one. For the fields, the test that has to come first is a
`tests/drift.rs` probe per provider: a hook reply with one extra key, and the call still denied.
Only after four greens would the fields be added. The decision is whether to spend four paid
probes on that or to leave the facts in text, where every dialect already carries them.

## Item 6. Map 126: a corpus written on two machines

The ticket is `.scratch/hydra/issues/126-a-corpus-written-on-two-machines.md`; read it for the
two questions.

**Problem.** A Linux record read on the Mac either fails the gate's path check (its paths resolve
differently under `os.path.realpath`) or is filed `Foreign` by `GateMark::of` and never
re-evaluated, depending on whether `gate_sha` matches. Contract 7.7 forbids the reader to take the
layout off the record, so a record cannot lie about what it was gated by. And until bubblewrap
exists (build 318, blocked by 315 and 316), a Linux record is hook-only evidence.

**How it would be solved.** Question 1: a carve-out in 7.7 for records whose `host` is not the
reader's, with one of three treatments: replay with the record's layout, mark `Foreign` and count,
or refuse to walk. Question 2: Rungs splits evidence by `sandbox.kind`, weighs hook-only records
less, or excludes them from gate proposals. The session's view, not an answer: `Foreign` and
counted; excluded from gate proposals until a Linux wrapper exists.

**The test that proves it.** A fixture record with a foreign `host` walked by `hydra doctor
--walk` and reported under its own count, never re-evaluated; and a `tests/curate.rs` case where a
hook-only record is present in the corpus and absent from the evidence of a proposal that touches
the gate.

## Item 7. Map 127: may a shell under `standard` read the run's own pinned skill snapshot

The ticket is `.scratch/hydra/issues/127-may-a-shell-read-the-runs-own-pinned-snapshot.md`.

**Problem.** Linked skills resolve to `~/hydra/skills-snapshots/<sha>/...`, and the pattern floor
refuses any shell command naming `~/hydra`'s runtime directories, so `cat` of a linked skill is
refused on `claude`, `cursor-agent` and `agy` (`codex` gets a copy since map 71). Build 320 made
the two skill tests link the skill and assert the refusal, so the tests are now honest about it.
`cursor-agent` and `agy` read files through their own file tools, which the floor does not see,
so the practical effect is on shell reads only.

**How it would be solved.** Three options, the ticket's three questions: the token pass exempts the
run's own snapshot directory when `run_start.sandbox` is set (the seatbelt denies writes there
already); or the preamble (protected config) tells runs to read skills with the file tool and the
floor stays as it is; or nothing changes and the refusal is documented as the floor's cost.

**The test that proves it.** The two skill tests in `tests/gate.rs` (lines near 2543 and 2601
before 320) flip to the chosen behaviour, and one live measurement on a `cursor-agent` and an
`agy` run that reads a `SKILL.md` through a link, with the record showing the read and no
refusal.

## Item 8. The publication list from research 129

`.scratch/hydra/research/129-ready-to-go-public.md`, section "What to do before the flip, in
order". Nine steps; the README (step 3) is done.

**Problem.** The repository is private with 0 stars, no tag, no release, no topics, a red CI on
master and on every Dependabot PR, a `security` workflow that fails at startup on every pull
request, 681 commits under the human's personal email, a personal absolute path in
`config/export-allowlist.json`, and a stray `pendintes_hydra.md` one `git add -A` away from being
published.

**How it is being solved.** In order: a real `gitleaks` scan of the whole history (not installed;
installing changes the machine, the human's call); the `sha2` 0.11 compile fix in `hydra-grader`
(`LowerHex` is gone from the digest array; format the bytes) so the Dependabot PR goes green; the
`security` workflow's `startup_failure`, which is in Actions settings or the workflow file,
protected ground; close `CHANGELOG.md`'s Unreleased as `v0.1.0`, tag, release; topics and
homepage; a sentence in `config/README.md` saying the allowlist path is per machine, or move it;
`pendintes_hydra.md` into `.gitignore`; `git gc` for the stray pack. The session can do the
compile fix, the changelog, the gitignore and the README sentence now; the scan, the Actions
settings, the tag push and `git gc` wait for the human.

**The test that proves it.** `gitleaks detect --source . --log-opts=--all` exits 0; both
workflows green on master and on one Dependabot PR; `git add -A --dry-run` lists no
`pendintes_hydra.md`; the `v0.1.0` tag exists and its release notes are the CHANGELOG section
byte for byte; the repository settings page shows the topics. The Linux job of item 1 is part of
"CI green".

## Item 9. Tickets not yet opened

Found this week and unticketed; the panel ranks them or says which not to open:

- `hydra doctor --walk` reports one `Ungated { tool: "Edit" }` on build 327's run
  (`01a13e24-243b`, call `toolu_01Xt5rNaN1rRLwgWUxeSZEK6`, 1 of 28 Edits with no hook decision).
  Either a gap in the hook or a provider that skipped the hook once.
- `hydra show --verdict`: print the verdict a record carries, so a reader does not open the
  events.
- OpenCode as a fifth provider (research 129 says it is the one agent CLI with a background
  service and permission rules, MIT, 213.6k stars).
- The JSON refusal fields of item 5, behind the four drift probes.
- `a_helper_that_hangs_on_approve_is_waited_on_five_seconds` still checks `5 s <= waited < 7 s`
  against the wall clock, the same shape build 330 removed from two cockpit tests.

## Items for the human alone, not for the panel

- Reaping 33 old worktrees under `~/hydra/worktrees` (55 GB with the cloned `target/`), `cargo
  sweep --installed`, and 775 empty directories under `~/hydra/spawn`. Each deletion needs a yes.
- Run `01a13df4-8f59` is a run on a private repository of 2026-10-15 05:07Z; its reap is the
  human's. Run
  `01a13432-0c19` is map 117's GPT seat that died on codex's quota; the session reaps it.
