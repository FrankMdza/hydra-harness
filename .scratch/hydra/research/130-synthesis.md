# Map 130, the session's synthesis for the human

Four seats sat on 2026-10-15, all under `standard`, none ran cargo: Opus 5.5 on `claude`
(`130-seat-opus.md`, 5.8M tokens), Grok 4.7 on `cursor-agent` (`130-seat-grok.md`, 5.6M),
Gemini 3.8 Flash on `agy` (`130-seat-gemini.md`), GPT 6.1 Sol on `codex` (`130-seat-gpt.md`,
0.6M). One caveat on independence before the votes: the session copied each memo into master's
working tree as it arrived, and a run's worktree is seeded from that tree. Gemini's worktree held
Opus's memo and GPT's held Opus's and Grok's. Gemini's text follows Opus's closely, item by item.
Grok launched before any memo existed and is the one fully independent second reading. Count the
votes with that in mind. The session checked every code claim quoted below against master.

## Corrections to the readings, verified

The seats found eight things wrong in `130-panel-readings.md`. All eight checked out.

1. Option 1 for the Linux job, `cargo test --lib --workspace`, is not green:
   `sheet::tests::lgamma_is_cpythons_and_not_the_c_librarys` is a lib test of `crates/puppets`
   and is one of the group g failures. (Opus, Gemini, GPT.)
2. Group g is not "Apple libm plus fused multiply-add on the whole". `c_mul_add`
   (`crates/puppets/src/sheet.rs:76-82`) already branches on `target_arch`: fused on aarch64,
   unfused elsewhere. The runner is x86-64 and the expected bits were recorded on the arm64 Mac.
   And no differential test runs a live Python: `crates/grader/tests/differential.rs:7` says "No
   test starts `python3`", and `python_said` reads `tests/fixtures/python/`. So the x86-64 branch
   has never been compared with anything, and the pyjson failure begins at the input block, before
   any `lgamma`. (Opus, Grok.)
3. Group f is a test bug visible from here. `tests/detached.rs:401-409` writes its `run_start`
   with the key `kind` and no `type`; `src/record/validate.rs:284` reads `type` and gets `""`. On
   the Mac the test returns at line 418 before `check` runs; on Linux `pty_still_held` is true,
   `check` runs, and the test's own JSON fails. No VM needed. (Grok, Gemini.)
4. The 325 cases are green tests that pin a failed reap, not red tests, and the readings swapped
   the two messages: the staged case fails at `git status --porcelain` with `failed in submodule
   sub` (`tests/cleanup.rs:1934-1940`); the unstaged case commits a `160000` gitlink to the branch
   and then `git worktree remove` exits 128 (`tests/cleanup.rs:2010-2016`). (All four.)
5. The 321 standing is already in the contract. `docs/run-record-contract.md:761-762`: "This is
   hook evidence, not tamper-proof provenance against a process that can write the sidecar through
   the kernel's necessary write allowance." Item 4 asks less than the readings said. (Grok, GPT.)
6. There is no `v0.1.0` to close. `CHANGELOG.md:103` reads `## 0.2.0, 2026-09-08` and
   `Cargo.toml:6` says `version = "0.2.0"`; `git tag` is empty. The next release is 0.3.0, and
   `Cargo.toml` bumps in the same commit. (Opus, Gemini.)
7. `.github/` is not protected ground. Harness spec 15.1 lists six configs, none under
   `.github`, and `PROTECTED_CODE_PATHS` (`src/rungs/evolve.rs:591`) does not name it. The
   workflow file is the session's to edit; the repository's Actions settings stay the human's.
   (Opus, Grok, Gemini.)
8. Two more files sit one `git add -A` from publication beside `pendintes_hydra.md`:
   `.scratch/hydra/labeling/2026-09-29/shard-c.jsonl` (157,929 bytes) and
   `classes-c-agy-discarded.jsonl` (40,489). They belong to the other session and are untracked,
   not ignored. (Opus, Gemini.)

Two seat claims were false. Grok: "a long token can eat the link" in a 320 refusal. `quoted`
(`src/policy/gate.py:199-208`) cuts the token at 200 characters and `resolution_facts` appends the
link clause afterwards regardless. GPT: `IMPLEMENTATION.md:317` "now discusses case folding, not
the personal path"; the path example moved to line 327 and is still there.

The session also read what no seat could: the `Ungated { tool: "Edit" }` record of build 327
(`~/hydra/runs/01a13e24-243b-7ab2-858b-bd0f215ee858.jsonl`, seq 326 and 327). The Edit's
`old_string` was not in the file; Claude Code answered `String to replace not found` 7 ms after the
call, with no `hook_started` between the two lines. Claude Code validates an Edit's input before
it runs PreToolUse hooks, so a rejected Edit has no hook decision and nothing was written. It is a
normalizer finding, not a bypass: a call whose result is an error and which has no hook decision
should be classed "rejected by the provider before the hook", not `Ungated`.

## Question 1: the Linux job

**All four: narrow the job now, not to `--lib`, and do not buy the green with an invented author,
a NUL allow or a one-ulp tolerance.** Opus, Gemini and GPT want the eight groups fixed before the
flip; Grok wants the unmeasured groups (e, g) skipped by name and left so until a Linux machine
measures them, with the README listing the skipped names.

The three decisions:

- **c, whose identity.** Opus, Gemini and GPT: a fixed literal, `Hydra <hydra@localhost.invalid>`,
  only for a key `identity()` found empty, never the OS user or the hostname. Grok: no fallback at
  all. Its reason is real: `identity()` also returns nothing when the `~/.gitconfig` read times out
  (`src/worktree.rs:185-191`, five seconds, a fifo defence from build 99), so a fallback would put
  an invented name on a commit of the human's work on a slow machine. Grok's fix is to pin
  `user.name` in the test fixture, as `committed_repo` already does for its own commits. The
  session verified that the commit runs with `GIT_CONFIG_GLOBAL=/dev/null` and
  `GIT_CONFIG_NOSYSTEM=1` (`src/worktree.rs:143-145`), so the name can only come from
  `identity()`'s earlier read.
- **d, the NUL cwd.** All four: refuse on purpose, before any `os.path` call, as `hydra/bad-payload`,
  and the test asserts that deny on both Pythons. Grok adds that `decide_payload` already turns
  the exception into `hydra/gate-error`, so the test could assert that as it stands; the explicit
  check is cleaner.
- **g, the CPython ports.** All four: no tolerance. `near_tie_ranks_as_python_lgamma` exists
  because one last bit swapped two agents in build 207. Opus, Gemini and GPT: gate the bit-exact
  tests to `cfg(all(target_os = "macos", target_arch = "aarch64"))` with a comment, and record a
  second fixture set only when a Linux machine ranks. Grok: skip by name. Opus suggests one cheap
  experiment first: a job on `ubuntu-24.04-arm` takes the fused branch and would leave only the
  libm differences.

**Session's recommendation.** Fix a, b, f and h this week in one build ticket, they are test-only
and f is a bug the suite hid. Take d now as the explicit refusal. On c, pin the fixture now (Grok)
and open a small ticket for the fallback question with one measurement first: what git does on a
fresh macOS with no config, since the runner's failure is a container's and may not be a laptop's.
Skip e and g by test name until the VM measures them, and the README lists the names. The job then
runs the whole suite minus a named list that only shrinks.

Tests: the job green; one named test per skipped group still red on `ubuntu-latest` without the
filter; for c, a child with `GIT_CONFIG_GLOBAL=/dev/null`, `GIT_CONFIG_NOSYSTEM=1` and a scratch
`HOME`; for d, `hydra/bad-payload` on a NUL cwd; the README sentence names the same command and
skips the job runs.

## Question 2: build 316

**All four: additive, no version bump.** Two shades. Opus: land `host` now and add `sandbox_kind`
only with build 318, when a second kind exists, since `host.os` already tells "no wrapper on this
platform" from "a choice". Gemini and GPT: `host` and `sandbox_kind` together. Grok: additive, and
the recommendation is void until ticket 316's What section stops ordering the object, because a
session that follows the ticket as written ships version 2 again; also amend map 126, which says
`sandbox.kind`.

Three facts the seats added. Nothing reads `run_start.sandbox` today outside `src/record/`.
Ticket 316 contradicts itself: it tells the run to read section 15 and then to re-type a field,
which section 15 says is exactly what the version is for; the run applied the contract correctly
to a wrong instruction. And the regenerated fixtures under `tests/fixtures/records/` must carry a
synthetic `machine`, not this Mac's hostname, because those files go public.

**Session's recommendation.** Additive, `host` and `sandbox_kind` together (the field costs
nothing and spares 318 a dependency), `absent` meaning unknown. Rewrite 316's What, reap the held
worktree without applying it, relaunch. Tests beyond the readings': the validator rejects
`host: null` and a `host` missing a leaf (section 15's leaf rule), a fixture guard that no fixture
`machine` equals `gethostname()`, and `hydra doctor --walk` over the 952 records quiet.

## Question 3: build 326

**Split two to two.** Opus and Gemini: commit the files (treatment one), with two amendments: move
the nested `.git` out of the tree rather than renaming it in place, since the next `git add -A`
would commit `.git.nested/` as plain files, and find `.git` files (a `gitdir:` pointer from a
worktree or submodule) as well as directories. Grok and GPT: refuse by name (treatment two).

Grok's two facts against treatment one checked out. `unstage_missing_objects` skips mode 160000
on purpose, "a submodule entry names a commit in another repository and is left alone"
(`src/worktree.rs:758`); flattening every gitlink also flattens a real submodule pin a run set
to an upstream commit. And `import_run_objects` runs before `commit_everything` (`src/reap.rs:
500-507`), so the nested commit, tree and blob already arrive in the parent unreferenced; the
readings' proof, `git fsck --unreachable` empty, fails under the treatment the readings
recommended. GPT adds that a rename before a reap that then fails halfway leaves the human a
dismantled nested repository.

**Session's recommendation, reversed from the ticket's.** Refuse by name, before any commit. It is
the smaller change, it stops the gitlink reaching the branch (today's unstaged path already
publishes one), it leaves the nested repository intact, and it keeps flattening available later as
an explicit flag with its own spec. Tests: the 325 cases assert no commit, the checkout still
present and the message naming `sub`; a two-repository case; a `.git` file case; a legitimate
submodule case, which must also refuse rather than flatten.

## Question 4: build 321

**All four: do not sign. The standing is in the contract already.** The reason is structural, not
a cost: the hook is `python3` running under the provider's own profile (`src/sandbox.rs:183-185`
reopens the run's spawn directory to the provider), so any secret the hook can read, the run's
shell can read. Signing would be the string floor in a different hat.

What all four demand instead: the missing test, a stub run that appends a forged row to the
sidecar mid-run and reads back what the record says; and a reader-side corroboration check (Opus,
Gemini): `hydra doctor --walk` flags a `gate_refusal` whose `call_id` matches no `tool_call` in
the provider stream, or whose call has a successful `tool_result`. A forger would then need a real
failed call behind each line. GPT adds the deletion case: signing would not catch removed lines
either, so completeness is a different problem from authenticity.

**Session's recommendation.** Accept the standing as written, open one build ticket for the forged
row test, the damaged sidecar test and the corroboration check in the walk.

## Question 5: build 320

**All four: leave the three facts in text, spend no probes.** Three of four say that if structured
fields are ever wanted, they belong on the 321 sidecar row, which Hydra writes and reads and no
provider parses, so it carries no drift risk. Its limit: the sidecar holds shell refusals only
(`gate.py:1797`), so a file tool refusal's link stays in the text. Grok adds a nuance the session
agrees with: spec 6.6 records fail-open on an unrecognised dialect, not on an extra key in a known
one, so the risk is untested rather than shown; still not worth four paid probes for a field
nobody consumes.

**Session's recommendation.** Close 320 as it stands. If a consumer appears, add the fields to the
sidecar row under 321's ticket, not to the hook reply. One more test, cheap: a token past 200
characters whose link clause is still present.

## Question 6: map 126

**Question 1, all four: never replay a record with its own layout, and never refuse to walk it.**
The disagreement is one counter or two. Opus and Gemini: `Foreign` and counted, which is what
`GateMark::of` (`src/normalize/mod.rs:755-763`) does today when the sha differs. Grok and GPT: a
separate count, because `Foreign` already fires for a stale gate on the same machine and a Linux
record would share the counter. Grok adds why cross-machine records will land `Foreign` every
time: `skill_snapshot` is always rebuilt from the reader's layout, so the recompiled script and its
sha differ; Gemini's `realpath` failure needs a matching sha and cannot happen. And 7.7 does not
say what the readings said: the reader takes stored scopes off the record and rebuilds only root,
home and the deny list; that is the sentence that stops a record from supplying a `/` read scope.

**Question 2, three of four: the readings' "exclude from gate proposals" is an empty set.**
`hydra evolve` refuses any patch to `src/policy`, `policies`, `src/sandbox.rs` or
`src/containment.rs` before review, and `curate` writes skills only. No Rungs proposal touches the
gate. What is left is whether a hook-only record counts as evidence for a skill proposal. Opus and
Gemini: Rungs refuses a mixed-host corpus at load until build 318. Grok: a weight nobody can
justify yet, and no test can be written against `sandbox_kind` until the writer emits it.

**Session's recommendation.** Q1: no carve-out in 7.7; once `host` exists, the walk tells
"another host" from "another build" and counts per host, never re-evaluating. Q2: Rungs refuses a
corpus that mixes hosts at load, with a message naming both, until a Linux wrapper exists; a
weight is not a decision anyone can make yet. Both wait on 316.

## Question 7: map 127

**Split two to two, with GPT's no closer to a later yes.** Opus and Gemini: yes, exempt the run's
own `skill_snapshot` directory from the shell token pass, compiled only when the launch is wrapped,
because the file tools already read it (`tests/gate.rs:1328-1345` and `2680-2692` assert the
allow under three policies) and the exemption tells the run nothing it cannot already read. Grok:
no, the floor is a deny list of strings, the token pass cannot tell `cat` from `cp` onto the same
path, and a prefix exemption repeats the bug `the_snapshot_scope_is_this_runs_snapshot_and_not_the
_directory_above_it` exists for. GPT: only when a field says an applicable kernel wrapper is in
force, which a non-null sha does not say today; until then document the file tool reads.

All four agree on question 2 (the own snapshot only, never `spawn/`, which holds the gate's
source and `launch.json`) and on question 3 (the preamble stays as it is; if anything is written
it is the human's).

**Session's recommendation.** Not now. The practical loss is a shell `cat` on three providers
whose file tools already read the skill; the gain is small and the floor change is not. Revisit
once 316's `sandbox_kind` exists so the exemption can be conditioned on `seatbelt` rather than on a
non-null sha, and then only with Gemini's five tests: allowed when wrapped, refused when not, a
sibling sha refused, `spawn/` refused, and a write through the link failing at the kernel.

## Question 8: the publication list

**The order, amended by the panel.** The scan first; then `.gitignore` for three files, since it
costs a minute and prevents the one accident that cannot be undone; then the `security` diagnostic
and CI; then the changelog closed as 0.3.0 with the `Cargo.toml` bump, tag and release; then
topics and the social preview; then the allowlist sentence; `git gc` last, from the main checkout.

What the seats changed in the list:

- **0.3.0, not 0.1.0.** See correction 6.
- **The `security` fix is unknown until the diagnostic is read.** The `osv-pr` job grants
  `contents: read` and `actions: read` and sets `upload-sarif: false`, so "add `security-events:
  write`" is a guess (Grok, GPT). The run also fails on pushes to master, where that job is
  skipped by its `if`, which points at workflow validation. The human's Details button on run
  38695261402 shows the text; the file is the session's to fix once read.
- **`sha2` 0.11 is a workspace change, not a grader one.** Eleven `format!("{:x}", ...)` sites,
  among them the sandbox profile sha (`src/sandbox.rs:100`, `306`) and the hook config sha
  (`src/spawn_config.rs:138`). A changed hex string there turns every old record `Foreign`. Either
  the bump comes with a fixed-vector test (one known input, its hex byte for byte, before and
  after) or the PR is closed and 0.10 stays. (Opus, Gemini.)
- **`config/README.md`.** Grok says a session does not write under `config/`; the session's
  reading is that 15.1 protects six files and the README is not one. The human decides who writes
  the one sentence.
- **The email.** All four agree with research 129: no history rewrite.
- **Missing, per GPT:** explicit consent for what personal text goes public (hostnames once
  `host.machine` exists, research notes under `.scratch/`), and the sidecar's trust language in
  `SECURITY.md`, not only in the contract.

Tests: `gitleaks detect --source . --log-opts=--all` exits 0; both workflows green on master and
on one Dependabot PR; `git add -A --dry-run` lists none of the three files; the tag's notes equal
the 0.3.0 section; a fresh clone builds and passes the suite.

## Question 9: the unopened tickets

Agreement on the first two and the last two; the middle one splits.

1. **The `Ungated` Edit.** All four: read the record before anything else. Done, see above. The
   ticket to open is a normalizer one: a call with an `is_error` result and no hook decision is
   "rejected by the provider before the hook", with the 327 record as fixture, and the walk's
   `Ungated` count excludes it.
2. **The five-second wall-clock test** (`tests/cockpit.rs:3865-3875`). All four: open, in build
   330's shape, with `APPROVE_BUDGET` injected and the production five seconds pinned.
3. **`hydra show --verdict`.** Opus, Gemini and GPT: open, low. Grok: the record has no verdict
   field; verdicts live in the puppets goal logs, so the flag reads a different file and the ticket
   has to say which before it is a small command. The session sides with Grok on the wording.
4. **JSON refusal fields.** All four: do not open; it is question 5.
5. **OpenCode as a fifth provider.** All four: not before the flip. If ever, a map ticket, since
   OpenCode decides allow/ask/deny inside its own process and Hydra's gate is a hook in someone
   else's CLI; routability is the first question.

## What the human has to type

Nine words would close the docket. Per item, the choice the session recommends is first.

1. Linux job: "fix a b f h, skip e g by name, c pin the fixture, d refuse" or another mix.
2. 316: "aditivo" (host and sandbox_kind, version 1) or "v2".
3. 326: "refuse" or "commit".
4. 321: "accept" (and the test ticket) or "sign".
5. 320: "text" or "fields".
6. 126: "foreign by host, refuse mixed corpus" or another pair.
7. 127: "not now" or "exempt when wrapped".
8. Publication: a yes to install `gitleaks`, the Details text of the `security` run, whether the
   session writes the `config/README.md` sentence, and "0.3.0".
9. Tickets: "open 1 2 3" or a subset.

Nothing under 130's Answer is written. The human answers.
