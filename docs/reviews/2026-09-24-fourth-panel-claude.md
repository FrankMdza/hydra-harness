# Fourth panel, `claude` reader, 2026-09-24

Reviewed commit `ca2020f` in a worktree Hydra launched under `standard`. No build, no test run, no
provider launched. Everything below was checked by reading the cited lines, and the Method section
says how much of the repository that covers.

## Verdict

Advance to technical interview.

The core ideas are real and the code carries them. The outcome comes from the tool stream, the gate
is compiled per vendor, and the object-store and admission work since the third panel is careful
systems code with tests that were checked by reverting the fix. What kept this from a fast-track
is the newest work. The one ticket merged without a revert check, 112, carries a bug a revert check
would likely not have caught either: the failure record it writes derives `completed`. The
orchestration mode, written after three panels about untrusted checkouts, runs git and follows
links in a run's checkout from outside the seatbelt, the class tickets 95, 99, 103 and 106 spent a
week closing in Rust. The candidate builds well and fixes fast. What I need to find out in the
room is whether they carry a lesson into the next thing they build without a panel pointing at it.

Level: mid-level harness or agent-infrastructure engineer. A senior loop is possible if the
candidate finds the ticket 112 bug and the puppets checkout problem on their own when shown the
functions.

## Scores

| Axis | Score | Evidence |
|---|---|---|
| Rust engineering quality | 7 | `src/run.rs:496-521` is a launch split into typed phases with one failure path per phase, and `src/worktree.rs:537-660` copies a hostile directory with `openat(O_NOFOLLOW)` and `fstat` on the same descriptor. Against that, `run.rs` is 2,433 lines, and `refuse_escalation` and `refuse_narrowing` (`run.rs:606-711`) build the same line 0 twice. |
| Test discipline | 7 | `tests/gate.rs:1506-1600` pins open bypasses as tests that pass, so closing one breaks the build on purpose, and most tickets since 97 report the failure each new test gave with its fix reverted. But ticket 112's four tests have never been seen to fail, `tests/harvest.rs:669-722` checks the failure record's lines and never derives its outcome, and `integrations/puppets/tests/test_puppets.py:162-182` writes two events in order and asserts they are in that order. |
| Security and threat-model honesty | 7 | `SECURITY.md:18-56` and `128-134` state the shell residual, the reachable `~/.ssh` and the lethal trifecta before anyone asks, which is rare. But `SECURITY.md:117-118` describes a window and a scrub that `src/worktree.rs:452-475` replaced, and `integrations/puppets/README.md:3` says "human approvals enforced" for approvals any process can append (`puppets.py:706-733`). |
| Documentation and communication | 6 | The prose is plain and the numbers are mostly dated and sourced (`README.md:14-17`, `87-91`). The drift is back, though: the authorship table at `README.md:247-258` leaves out resolved tickets 108, 109 and 112, and `IMPLEMENTATION.md:3150-3157` and `3181-3184` describe code that ticket 112 changed. At 4,123 lines, `IMPLEMENTATION.md` is a changelog more than a document a stranger can use. |
| Depth and originality of the harness idea | 9 | `src/outcome.rs:122-298` derives nine outcomes in a stated order with a separate confidence field, and `src/worktree.rs:442-527` gives each run a private git object store that reap imports through `index-pack --strict`, so a forged pack cannot rename an object. I have not seen either in the tools `README.md:170-176` compares against. |
| Engineering process | 7 | Build tickets 98, 103 and 105 each have a section on what the ticket assumed that turned out false, and 105 found a hole in 98's merged profile (`src/rungs/evolve.rs:1002-1008`). That is a real loop. Against it, 112 is `Status: resolved` with its verification owed to ticket 114, and 42 commits landed on 2026-09-24, several of them pixel-art mascots (`git show --stat be8b03e`). |

## Strengths

1. The outcome is a derivation with a stated order and a confidence field, not a vendor flag.
   `src/outcome.rs:122-298` reads refusal, provider refusal, `refused_everything`, harness runs,
   the harvested limit and the terminator in that order, and keeps `unknown` as the one soft
   answer. The order is documented at `IMPLEMENTATION.md:2097-2107` and pinned in
   `tests/outcome.rs`. A lab that trains on trajectories needs labels that do not trust a vendor's
   `SUCCESS`, and this is that problem taken seriously.

2. The per-run object store holds up under adversarial reading. `src/worktree.rs:537-597` copies
   only hex-named files and pack files, each opened without following a link and checked as a
   regular file with one link through the descriptor that is then read (`worktree.rs:630-645`).
   `pack_into_parent` (`worktree.rs:477-527`) runs `pack-objects` over the copy and
   `index-pack --strict --max-input-size`, so git names each object from its bytes. The forged-pack
   test the ticket describes (`tests/cleanup.rs`, build 103's Outcome) plants a pack that maps an
   existing name to other bytes and follows it into a later worktree. Trajectory integrity is a
   harness team's problem, and this candidate found and closed a real integrity hole in it.

3. The known holes are tests. `tests/gate.rs:1506-1600` asserts that a quote-split path, a
   concatenated interpreter string, a `worktrees/../runs` launder, doubled slashes and a
   percent-encoded fetch URL are allowed, each with "if this now denies, the residual is closed".
   `SECURITY.md:18-28` says the same thing in prose. A reviewer can check the security claim
   against the suite, which I rarely see in a portfolio.

4. Rung two admits the bytes it offers. `prepare_admission` (`src/rungs/evolve.rs:917-955`)
   writes the read tree from the object store with `ls-tree` and `cat-file --batch`, avoiding
   `checkout-index` filters and `archive` export rules, under a profile the proposer cannot
   write. `commit_pass_branch` (`evolve.rs:1691-1716`) commits `patch.tree` on `patch.head`, both
   read before admission. The comment at `evolve.rs:1002-1008` records a measured SBPL fact,
   that a `file-read-metadata` rule outranks `file-read*` whatever the order. The candidate found
   it by testing, not by reading documentation.

5. The tickets correct each other in writing. Build 98's Outcome says the ticket's premise was
   false (cargo reads `.cargo/config.toml` in every ancestor, so a scratch `CARGO_HOME` alone
   left the `runner` forge working) and shows the measurement. Build 105 found that 98's own
   profile still let a linked `.cargo` through. Build 100 found a quota failure that had been
   deriving `completed`, not `failed`, a detail the panel had missed. For a team that wants
   agent-written code reviewed rather than trusted, this is the habit to hire for.

## Weaknesses and red flags

1. A bug, in the one ticket merged unverified. `record_then_wait` (`src/interactive.rs:548-620`)
   writes the pid-carrying failure record as `RunEnd::harvested(None, Some(failure))`
   (`interactive.rs:615`), which sets `body_source: "harvest"` (`src/record/lines.rs:186-198`).
   In `derive`, the harvested arm with no result reads only `exit_code`, and `None` maps to
   `Completed` (`src/outcome.rs:188-206`). `run_end.error` is never read on that arm. So the
   record of a launch that failed derives `completed` with source `unknown`, and
   `rung_one_learns(Completed)` is true (`outcome.rs:73-78`). Its test asserts line counts and the
   error text, then asserts that `hydra status` will never harvest it (`tests/harvest.rs:717-721`),
   which also means the session's real transcript never reaches the record. The same arm turns the
   "session wrote no transcript" record (`interactive.rs:698-718`) into `completed`, `derived`,
   when the exit code is 0. All three panels missed that older half.

2. A bug class reintroduced outside `src/`. `cmd_review` runs `hunk diff` with the run's worktree as
   cwd (`integrations/puppets/puppets.py:784-822`), outside any seatbelt, as the human. Hunk runs
   git there, and git follows the checkout's `.git` file, which the run can rewrite. That is H47's
   decoy, which the third panel reproduced with a `post-checkout` hook and `run.rs:389-406` now
   refuses for Hydra's own launch. A decoy config with `core.fsmonitor` would run on a plain diff.
   I did not run this. `cmd_plans` copies `sources[0]` from the run-writable
   `worktrees/<run-id>/docs/proposals/` with `shutil.copy2` (`puppets.py:579-587`), which follows
   links, so a run that plants a link to a sibling's record gets a corpus file copied, from
   outside the seatbelt, into `~/hydra/puppets/`, which the seatbelt does not deny to reads.
   Ticket 106 removed exactly this link-following from reap.

3. A design smell all three panels missed. `Capability::Unhooked` (`src/policy/mod.rs:54-57`)
   replaces a tool's real capability, and the Cursor reader files every unhooked call as `Allow`,
   decided by the provider, with no finding whatever the policy (`src/normalize/cursor.rs:535-561`).
   Build ticket 113 proposes exactly that for `webSearchToolCall`. Once it lands, a Cursor judge
   under `judge`, which grants no network, can search the web and the record shows an ordinary
   call. The red corpus walk is the only place that surfaces this today, and 113 would turn it
   green by renaming the leak.

4. A documentation gap, and a recurring one. `SECURITY.md:117-118` and `IMPLEMENTATION.md:3181-3184`
   say reap scrubs the store in place and a window stays open for ticket 112. The code copies the
   store and packs the copy (`src/worktree.rs:452-475`), and `scrub_run_store` no longer exists.
   `IMPLEMENTATION.md:3155-3157` says the interactive failure still leaves no record.
   `src/worktree.rs:434` and `src/interactive.rs:536` credit "build ticket 107", the number the
   ticket had before two renumberings, and 107 is now the `hy` launcher. Comments naming "three
   providers" survive in `src/outcome.rs:14-15`, `src/policy/gate.py:17-19` and
   `src/mcp.rs:4`. The closing rule in `CLAUDE.md` is "amend `IMPLEMENTATION.md` in the same
   change", and ticket 112 was closed without doing that.

5. A design smell in the new mode. Approval is `puppets.py approve`, an append to a file
   (`puppets.py:706-733`), with no check that a human typed it, and the orchestrator is a Claude
   Code session that runs the same CLI. The `plan` approval kind is accepted and never checked
   anywhere: `launch_refusal` reads the table and launch kinds only (`puppets.py:312-339`), and no
   command writes `human_plan`. A `diff` approval binds to a task and a revision, not to a run id
   or a diff (`puppets.py:247-263`, `901-924`), and a relaunch does not change the revision, so an
   approval given after reviewing attempt 1 is spent by a verdict on attempt 2. Map ticket 76's
   decision J names approval outside every file an agent can write as a precondition for
   automation, so the author knows this. The README's third line does not say it.

6. A scope limit the author admits, with a title that oversells it. Build ticket 98 is "a
   toolchain the proposer cannot change", and `admission_toolchain` (`src/rungs/evolve.rs:885-915`)
   resolves `/opt/homebrew/Cellar/rust/1.94.0/bin`, which a `standard` proposer can rewrite.
   `SECURITY.md:72-75` says so. Taken together, admission shows that a proposer who did not tamper
   with the toolchain wrote a patch that builds. It says nothing about a proposer that did.

7. A test-design smell. The corpus walks run inside `cargo test` against this laptop's private
   `~/hydra/runs` (`tests/normalize.rs:1593-1597`), so whether the suite passes depends on the
   day's vendor build. CI does not run the puppets tests at all (`.github/workflows/ci.yml:19-36`),
   and it runs `cargo clippy -- -D warnings` without the `--all-targets` every ticket's Done-when
   asks for.

On the question the brief asks directly: the red walk is the design working as a detector and
failing as a test. The detection is right. Cursor ran a network tool the hook never saw, and a
`judge` or `read-only` run doing the same would be a real breach. The drift on `claude` fails the
other way. The gate refused this reviewer's own `Agent` tool call with `hydra/unknown-tool`, since
`CLAUDE` in `src/policy/table.rs:90-126` still lists `Task`. That costs capability, not safety, and
the refusal named the cause. The alarm belongs in `hydra doctor` or a scheduled job with an exit
code, next to the fixture walks that already hold the regression half. It should not sit in the
suite a contributor runs before a pull request.

## The third panel, checked

| Finding | Status | Checked at |
|---|---|---|
| H1, shared object store writable | Fixed. The parent's `.git` is denied whole and reap imports through a nofollow copy and `index-pack --strict`. The documents trail the code, weakness 4 | `src/policy/mod.rs:594-596`, `src/worktree.rs:452-597` |
| H2, `~/.cargo` runner forge | Fixed with a caveat. Scratch `CARGO_HOME`, ancestor `.cargo` denied by name and by metadata since 105, which found 98's merged profile still open. The toolchain directory stays user-writable | `src/rungs/evolve.rs:957-1016`, `src/env.rs:54-86` |
| H3, watcher holds the shell's environment | Fixed. `env_clear` then the child's `ChildEnv`, and the ticket reports a test that reads the running watcher's environment from the kernel | `src/watch.rs:116-134` |
| H4, `codex` quota reads `failed` | Fixed. `codex_limit_refusal` runs beside `provider_refused` | `src/outcome.rs:136-143`, `389-400` |
| H5, git reads run-writable config | Fixed with a caveat. `GIT_CONFIG_GLOBAL=/dev/null` and `NOSYSTEM`, but `core.excludesFile` and `safe.directory` are still read from a `~/.gitconfig` a run can write and passed back with `-c`. A run can make reap ignore, and so lose, its own files. Low harm, and stated | `SECURITY.md:92-98`, IMPLEMENTATION 105 paragraph at `IMPLEMENTATION.md:3140-3154` |
| H38, admission reads a writable checkout | Fixed. Steps run on the materialized copy, and the branch is committed from the tree and `HEAD` read before admission | `src/rungs/evolve.rs:330-352`, `1691-1716` |
| H39, failure after `git worktree add` leaves no record | Fixed with a caveat. Rollback, not a record. `roll_back` returns on the first failed step, so a failed `worktree remove` leaves the branch. Rollback also means the failed launch is in no record, which sits oddly beside D10 | `src/worktree.rs:1079-1135` |
| H41, `--out` aliases its sibling | Fixed. Opened without truncating, compared by device and inode, truncated only after | `src/export.rs:382-414` |
| H47, launch from inside a checkout | Fixed for Hydra's own git, for launch and `evolve`. The class comes back in `puppets.py review`, weakness 2 | `src/run.rs:389-406`, `561` |
| Build 112 | Not fixed as closed. The store half reads right. The interactive half derives `completed` for a failure, no test reads the outcome, the documents were not amended, and 114 is still `claimed` | `src/interactive.rs:548-620`, `src/outcome.rs:188-206`, `.scratch/hydra/build/112-*.md`, `114-*.md` |

I count the third panel's fixes as real except for two things. The toolchain half of H2 was
relabelled, not closed. Ticket 112 was counted resolved on green tests that nobody has watched fail.

## Master of Puppets, read cold

It is a Python CLI over one append-only JSONL per goal, whose state is a fold (`puppets.py:129-280`), that launches `hydra run --timeout 0` per task and records `launch_intent` before and `run` after, so a crash leaves an intent the human must `reconcile` (`puppets.py:735-775`, `826-841`).
The table approval does cover one revision and a later assignment voids it (`test_approval_voided_by_later_assignment`, `test_puppets.py:125-140`), and the `codex` launch approval is required (`test_codex_task_refused_without_launch_approval`). No test shows a launch approval being spent, the plan approval is never checked, and a diff approval is not bound to the run it approved (weakness 5).
`append_event` takes no lock and numbers events by `len(read_log)+1`, and `read_log` raises on a torn last line (`puppets.py:76-97`), so two panes launching at once or one crash mid-write break the fold.
Before build ticket 110 lands, a run can read every goal log. It cannot write them on this machine, but only because `hydra_home()` is `~/hydra` here and is denied whole (`src/policy/mod.rs:574`). On an install where the checkout lives elsewhere, a run can append its own `approval` line.
`sheet.py`'s ranking is a beta bound over hand-set priors (`priors.json`, `sheet.py:192-239`) that `assign` ignores until a verdict exists, which is honest and so far measures nothing.
Engineering in its log-and-fold shape, theatre in "approvals enforced". I would not put it on a resume line yet, and I would put Hydra's outcome derivation there instead.

## Claims checked

| Claim | Source | Checked at | Verdict |
|---|---|---|---|
| The outcome is derived from the stream, not the exit code | `README.md:21-28` | `src/outcome.rs:122-298` | Holds with a caveat. Streamed runs never read the code as a signal. The harvested arm reads it, and ignores `run_end.error`, so ticket 112's failure record and a missing transcript derive `completed` (`outcome.rs:188-206`) |
| `src/env.rs` is the whole list | `README.md:40-46` | `src/env.rs:19-33`, `src/run.rs:773-811`, `src/worktree.rs` `prepare_run_objects` | Holds with a caveat. The inherited names are in `env.rs`. The names Hydra sets, the marker, `CURSOR_DATA_DIR` and the two git object variables, are composed in `run.rs` and `worktree.rs` |
| The gate refuses a protected path only when whole and canonical, and tests pin the misses | `SECURITY.md:18-28` | `src/policy/gate.py:260-329`, `tests/gate.rs:1506-1600` | Holds |
| A `codex` hook that dies fails open | `SECURITY.md:201-220` | `src/policy/gate.py:956-977`, `SECURITY.md:217-220` | Holds, admitted. The catch-all covers raises in `evaluate`, not an interpreter that never starts. I did not run the paid probe |
| The seatbelt denies the corpus whatever the spelling | `SECURITY.md:36-43` | `src/sandbox.rs:76-105` | Holds with a caveat. Denials are `subpath` on canonical paths. The profile's ordering is argued in comments, and 105 showed SBPL precedence is not plain last-match. macOS only |
| A child is never wider than its parent | `README.md:138-139` | `src/run.rs:541-555`, `606-711`, `SECURITY.md:172-194` | Holds with a caveat. The marker can be unset. On macOS the seatbelt and the `HYDRA_ROOT` refusal backstop it, and on Linux nothing does |
| The concurrency cap is a reservation under a lock | `IMPLEMENTATION.md:3703-3714` | `src/live.rs:322-402`, `src/run.rs:596` | Holds with a caveat. `flock` across scan and rename. Interactive launches take no reservation and are never refused by the cap |
| `hydra reap` commits only on the branch Hydra made | `SECURITY.md:78-84` | `src/reap.rs:473-485` | Holds. `HEAD` is read through the pinned private git directory the run cannot write |
| Every git call in a run's checkout pins its git directory from the record | `SECURITY.md:85-89` | `IMPLEMENTATION.md:3115-3124`, `src/worktree.rs` `CheckoutGit` | Holds for Hydra's Rust. Does not hold for `puppets.py review`, which runs Hunk in the checkout (`puppets.py:784-822`) |
| Each run writes objects into its own store and reap imports with `index-pack --strict` | `SECURITY.md:103-118` | `src/worktree.rs:442-597` | Holds. The sentence about a window still open is stale, since the copy closed it |
| Admission runs on a fresh copy of the read tree | `SECURITY.md:57-77` | `src/rungs/evolve.rs:917-955`, `330-352` | Holds |
| The watcher and every spawn start from a cleared environment | `SECURITY.md:9-14` | `src/watch.rs:122-133`, `src/provider/codex.rs:288-290` | Holds with a caveat. Git and the replay's `python3` still inherit (build 97's table), and the puppets helper's `subprocess.run` calls pass the whole shell to `hydra` (`puppets.py:366-375`) |
| Git outside the seatbelt reads no config a run can write | `SECURITY.md:85-98` | `SECURITY.md:92-98` | Holds with a caveat. Two values are still read from `~/.gitconfig` as data, and the document says so |
| Rung two offers exactly the tree its checks and judges read | `IMPLEMENTATION.md:3194-3207` | `src/rungs/evolve.rs:330-393`, `1691-1716` | Holds |
| A pass edits only what `--may-edit` names, over a floor | `IMPLEMENTATION.md:3278-3310` | `src/rungs/evolve.rs:573-815` | Holds with a caveat. A named file is fully editable, admitted in map 73 |
| Export aborts on a prefix or PEM header, warns on entropy, before either file exists | `SECURITY.md:226-237`, `README.md:51-54` | `src/export.rs:336-366`, `657-712` | Holds. Both files are rendered and verified before `open_events_out` |
| `codex` reads a seatbelt-denied copy of its skills | `SECURITY.md:159-163` | `src/sandbox.rs:150-158` | Holds on macOS, admitted `chmod`-able elsewhere |
| A non-ASCII root is refused before launch | `SECURITY.md:29-33` | `src/paths.rs:60-90`, `src/run.rs:585`, `src/interactive.rs:272` | Holds with a caveat. The launching repository's `.git` is not checked, as stated |
| The authorship table covers the 109 resolved build tickets | `README.md:240-266` | `README.md:247-258`, build tickets 108, 109, 112 | Does not hold. The rows sum to 106. 108 (composer-2.5), 109 (`codex`) and 112 (an Opus subagent) are missing, and the paragraph mixes denominators of 109 and 106 |
| Master of Puppets enforces human approvals | `integrations/puppets/README.md:3` | `puppets.py:283-339`, `706-733` | Does not hold. It enforces that an approval line exists at the revision, and anything that can run the CLI writes one |

## Interview questions

1. "Ticket 112's agent ran out of quota before reporting, and you merged its diff and marked the
   ticket resolved. Walk me through the record `record_then_wait` writes when the start time will
   not read, and tell me what outcome `hydra show` derives for it." Prompted by
   `src/interactive.rs:548-620` and `src/outcome.rs:188-206`. A strong answer traces
   `RunEnd::harvested` to `body_source: harvest`, finds the harvested arm mapping a null exit code
   to `completed`, notices rung one learns from it and that the real transcript is never harvested,
   and proposes reading `error` on that arm or giving the record a refusal. On process, it says a
   ticket is not resolved before its revert check, whoever wrote the code. Worrying: "the tests
   pass", or blaming the agent.

2. "Build ticket 113 makes `webSearchToolCall` an unhooked row. Afterwards, what does the record
   show when a Cursor judge under `judge` searches the web?" Prompted by
   `src/normalize/cursor.rs:535-561` and `src/policy/mod.rs:54-57`. Strong: `Unhooked` erases the
   capability, the call becomes a provider `Allow` with no finding, and the right shape is an
   unhooked row that keeps `network_read` so the replay can flag it against the policy. Then it
   separates the detector from the regression suite. Worrying: "it's recorded, so it's fine".

3. "You spent four tickets making Hydra's git distrust a checkout's `.git` file. Read
   `cmd_review` and `cmd_plans` with me." Prompted by `integrations/puppets/puppets.py:571-605`,
   `784-822`. Strong: spots Hunk running git in a run-writable checkout outside the seatbelt, a
   decoy `core.fsmonitor`, and `copy2` following a planted link into the corpus. Fixes: review only
   the reaped branch or hand Hunk a pinned `GIT_DIR`, and copy with `follow_symlinks=False` and a
   regular-file check. Then says why the lesson did not transfer. Worrying: "that's Python, not the
   harness".

4. "Why does a run find its private object store through environment variables, what happens when
   a provider's shell strips them, and why does reap copy the store before packing it?" Prompted by
   `src/run.rs:800-808` and `src/worktree.rs:452-597`. Strong: git has no config key that moves
   the object directory, a stripped variable makes `git add` fail at the denied parent rather than
   write anywhere shared, and the copy removes the window between the scrub and `pack-objects`
   that a leftover process could use. It also knows `index-pack` names objects by content, so a
   forged pack arrives as an object nobody asked for. Worrying: "content addressing makes forgery
   impossible".

5. "Ticket 105 found that a `file-read-metadata` rule beats `file-read*` whatever their order.
   `sandbox.rs` argues its profile from last-match. How do you know the recorded-launch profile
   has no inversion like that?" Prompted by `src/rungs/evolve.rs:1002-1008` and
   `src/sandbox.rs:76-160`. Strong: admits the ordering is held by comments and a few probes, and
   proposes a generated test that runs `sandbox-exec` over every denied and owned path for each
   operation and asserts the final decision. Worrying: "SBPL is last-match".

6. "Your ranking puts a beta posterior on human verdicts with priors you set by hand. With three
   verdicts in a class, what is it measuring, and how would you evaluate a router?" Prompted by
   `integrations/puppets/sheet.py:176-239` and `priors.json`. Strong: the priors dominate, and
   `completed` is not acceptance. It proposes a held-out task set with fixed grading, acceptance
   per unit cost with intervals, an A/B against the default table, and verdicts recorded blind to
   the provider. Worrying: the posterior bound treated as the result.

7. "In `worktree::create` you roll back on failure. In `record_then_wait` you write a failure
   record instead. D10 says what is not in a record did not happen. Defend both." Prompted by
   `src/worktree.rs:1079-1135` and `src/interactive.rs:527-547`. Strong: nothing ran in the first
   case and a live session exists in the second, but a refused launch that leaves no record is a
   gap D10 names, so a rollback should still write a `never_started` record. Worrying: no tension
   seen.

8. "Your suite went red because Cursor shipped a build. Is that a test?" Prompted by
   `tests/normalize.rs:1593-1597` and build ticket 113. Strong: the fixture twins are the
   regression tests, and the live walk is a monitor that belongs in `hydra doctor` or a scheduled
   job with an allowlist of known drift and an owner. It adds that the claude side fails closed at
   the gate, which is a different risk. Worrying: "red on drift is the point" with no answer for
   contributors.

9. "A `standard` proposer can rewrite `/opt/homebrew/Cellar/rust`. What does your admission gate
   prove, and what would it take to make it prove more?" Prompted by
   `src/rungs/evolve.rs:885-915` and `SECURITY.md:72-75`. Strong: it proves the patch builds for a
   proposer that did not tamper with the toolchain. Closing that takes a toolchain outside
   user-writable space, a hash-pinned toolchain checked before admission, or admission in a VM.
   Judges reading the diff do not replace any of those. Worrying: "the seatbelt covers it".

10. "Build 108's Outcome says your session changed five mechanisms in Composer's puppets diff.
    Pick one. What was wrong, and would you have caught it without writing the ticket?"
    Prompted by `.scratch/hydra/build/108-puppets-log-and-launcher.md`. Strong: names the table
    approval spent by the first launch, or the plan read from a worktree file the brief never
    asked for, and says what they check in an agent diff now. Then admits that the same review did
    not catch the link-following or the checkout git. Worrying: a narrative with no mechanism.

## Before publishing

1. Small, `src/interactive.rs:548-620` and `src/outcome.rs:188-206`. Make the pid-carrying failure
   record derive a failure, and do the same for the missing-transcript record. Add the outcome
   assertion to `tests/harvest.rs:669-722`, then do ticket 114's revert check before anything else
   counts 112 as resolved.
2. Small, `SECURITY.md:115-118`, `IMPLEMENTATION.md:3150-3157` and `3181-3184`. Describe the copy,
   not the scrub, and say the window is closed. Fix "build ticket 107" at `src/worktree.rs:434` and
   `src/interactive.rs:536`, and in `tests/cleanup.rs:1786` and `tests/harvest.rs:665`, `724`.
3. Small, `integrations/puppets/puppets.py:571-605` and `784-822`. Stop following links in
   `plans`, and do not run Hunk inside a run's checkout. Review the reaped branch, or pin `GIT_DIR`.
4. Small, `integrations/puppets/README.md:3`. Say approvals are log lines the CLI checks, not a
   human-presence check. Enforce or delete the `plan` kind (`puppets.py:312-339`), and bind a
   `diff` approval to a run id (`puppets.py:901-924`).
5. Needs a decision, build ticket 113. Keep the real capability on unhooked rows and flag a policy
   violation, rather than adding a row that silences the walk.
6. Small, `README.md:240-266`. Add 108, 109 and 112 to the authorship table and use one
   denominator.
7. Small, stale "three providers" and "`codex` is unmeasured" comments in `src/outcome.rs:14-15`,
   `src/policy/gate.py:17-19`, `src/mcp.rs:4`, `55`, `80`, `src/policy/mod.rs:965` and
   `src/normalize/mod.rs:106`, `265`. Ticket 115 covers two of them.
8. Small, `.github/workflows/ci.yml:19-36`. Run the puppets unit tests, and run clippy with
   `--all-targets` the way the tickets claim.
9. Large, `tests/normalize.rs:1593-1597` and the other live walks. Move them into
   `hydra doctor` or a scheduled job. Keep the fixture twins in the suite.
10. Small, `integrations/puppets/puppets.py:76-97`. Lock appends, and tolerate a torn last line
    the way `sheet.py` already does.
11. Small, `integrations/herdr/mascot/hydrapet.py`. 890 vendored lines of pixel-art dragon in a
    security-focused repository cost reviewer attention. Move it to its own repository.
12. Small. The untracked `pendintes_hydra.md` reached this reviewer's worktree through the seed,
    as it reached the third panel's `codex`. Keep private notes out of the checkout reviewers are
    launched from.

## Method

Read in full: `README.md`, `SECURITY.md`, `CONTRIBUTING.md`, `CLAUDE.md`; build tickets 97 to 106
and 112 to 115 (Outcomes in full, some Done-when sections); map tickets 75, 76 and 77, and the
Answer sections of 72 to 74; `integrations/puppets/README.md`, `puppets.py`; `src/watch.rs`
`detach`; `src/sandbox.rs`; the diff of `3c454ec` for `src/worktree.rs`.

Read by range: the third panel report, lines 1 to 140 and 280 to 554; `IMPLEMENTATION.md` 1 to
200, 2088 to 2180, 2888 to 2923, 3113 to 3330 and 3695 to 3730; `docs/harness-spec.md` 7.3, 7.4
and 15.1; `src/outcome.rs` 1 to 330 and 389 to 435; `src/policy/gate.py` 1 to 47, 118 to 420, 731
to 800 and 937 to 1094; `src/policy/table.rs` 1 to 140, 160 to 180 and 330 to 470;
`src/policy/mod.rs` 45 to 60 and 554 to 625; `src/run.rs` 389 to 720 and 773 to 815;
`src/worktree.rs` 400 to 700 and 1058 to 1165; `src/interactive.rs` 500 to 640 and 696 to 722;
`src/rungs/evolve.rs` 219 to 396, 917 to 1020 and 1691 to 1716; `src/rungs/review.rs` 1 to 120;
`src/export.rs` 1 to 40 and 284 to 420; `src/env.rs` 1 to 120; `src/live.rs` 317 to 403;
`src/reap.rs` 458 to 518; `src/normalize/cursor.rs` 505 to 570; `src/record/lines.rs` 186 to 198;
`tests/gate.rs` 1500 to 1610; `tests/harvest.rs` 665 to 745; `tests/normalize.rs` 1590 to 1640;
`integrations/puppets/tests/test_puppets.py` 109 to 392; `sheet.py` 176 to 250.

Skimmed by grep only: `src/skills/mod.rs` (`inject`), `src/paths.rs` (the ASCII refusal),
`docs/harness-spec.md` headings, `.scratch/hydra/map.md` decisions list, the CI workflow, the
authorship rows against tickets 107 to 109, and the `git log` from `2e03429` to `ca2020f`.

Not read: `src/provider/codex.rs` beyond the login probe lines, `src/normalize/mod.rs`,
`src/policy/replay.rs`, `src/skills/mod.rs` in full, `docs/run-record-contract.md` sections 1 to 5,
`docs/rungs-spec.md`, `tests/corpus/`, `tests/cleanup.rs`, `tests/record_contract.rs`,
`tests/git_config.rs` and `tests/status.rs` bodies, `integrations/puppets/tests/test_sheet.py`,
research documents 75 and 77, and the other fourth-panel review in `docs/reviews/`, which I
left unopened to keep this one independent. I tried to split the authorship count and some claim
checks out to subagents, and the gate refused the `Agent` tool as `hydra/unknown-tool`, so I did
both by hand and the count covers tickets 107 to 112 only. The ticket 112 outcome bug and both
puppets checkout findings come from reading both sides of the code. I reproduced none of them.
