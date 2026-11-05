# Four readers as a hiring panel, fourth round, 2026-09-24

Reviewed commit `85df6f6`. The third panel (`docs/reviews/2026-09-24-four-readers-third-panel.md`, findings H1 to H48) closed into build tickets 97 to 106 and 112; map tickets 75 to 77 and build tickets 108 to 111 arrived after it and were unread by prior panels. No tests or builds were run in this review; claims were checked by reading cited lines only.

## Verdict

**Advance to technical interview**, mid-level harness or infrastructure engineering.

The deciding reason is that the repository's central claim is implemented in code, not marketed: `src/outcome.rs:122-278` derives run outcomes from the normalized tool stream in a fixed order and explicitly refuses exit status on streamed runs (`src/outcome.rs:13-19`, `IMPLEMENTATION.md:2109-2116`), while `tests/gate.rs:1536-1591` pins shell bypasses the gate still admits and `SECURITY.md:18-28` states them honestly. That is rare in agent-harness portfolios and it survived three adversarial panels that turned findings into measured fixes (build tickets 97 to 106, 103, 105, 106). The Rust is competent agent-assisted systems code, not staff-level authorship; residual gaps (Homebrew-writable toolchain under admission, `webSearchToolCall` drift red on this machine, build ticket 112's tests never revert-verified) are real but do not erase the core.

Interview level: **mid-level**, with room to argue senior only if the candidate owns the security tradeoffs and the agent-authorship process out loud.

## Scores

| Axis | Score | Evidence |
|---|---|---|
| Rust engineering quality | 7 | Launch is phased with rollback (`src/run.rs:528-571`, `src/worktree.rs:381-400`), git calls are centralized behind `CheckoutGit` with explicit env pinning (`src/worktree.rs:117-183`), and errors use `anyhow` with context at boundaries; `phase_spawn` is now 124 lines per build ticket 99's Outcome, not the 163 the third panel caught. |
| Test discipline | 8 | `tests/normalize.rs:1594-1644` walks every live record for gate leaks; `tests/gate.rs:1536-1591` documents open bypasses as passing tests; `tests/cleanup.rs` (build 103/106 Outcomes) plants forged packs and alternates; build ticket 112's four new tests pass on master but their Outcome admits nobody revert-verified them (``.scratch/hydra/build/112-two-residuals-of-105-and-106.md:32-35``). |
| Security and threat-model honesty | 8 | `SECURITY.md:18-56` states gate limits, seatbelt as defence-in-depth, lethal trifecta under `standard` (`SECURITY.md:128-134`), and historical env leaks; `tests/gate.rs:1526-1532` and `tests/spawn.rs` (cited in build 103) pin seatbelt vs gate responsibilities. |
| Documentation and communication | 7 | `IMPLEMENTATION.md` at ~3,800 lines tracks gaps with reasons (`grep` headings through line 4062); `README.md:226-231` admits CI's empty corpus. Drift remains: `src/export.rs:4` still says "four fixed classes" while `redact_secrets` at `src/export.rs:576-617` implements five prefixes plus OpenAI matcher; build 102 was meant to align docs. |
| Depth and originality of the harness idea | 9 | Outcome-from-stream (`src/outcome.rs`), per-vendor hook dialects in one compiled `gate.py` (`src/policy/gate.py:48-67`), lossless JSONL contract, rung-two admission on a copied tree (`src/rungs/evolve.rs:917+`, build 98 Outcome), and Master of Puppets' fold-over-log pattern (`integrations/puppets/puppets.py:129-280`) are not shelf copies of existing tools (`README.md:170-176`). |
| Engineering process | 8 | Third panel findings map to build tickets with `## Outcome` measurements (e.g. build 97 lines 54-75, build 103 lines 96-125); authorship is disclosed (`README.md:239-268`). Weaker spots: build 112 closed without revert checks, tickets 110 to 115 still open on puppets hardening and drift. |

## Strengths

1. **Outcome derivation matches the README claim.** `src/outcome.rs:122-278` walks refusal, provider service denial, gated-everything, harness runs, harvest limits, then terminator/`exit_code` arms; module comment at `src/outcome.rs:13-19` states streamed runs do not use exit status, and harvested runs are the documented exception at `src/outcome.rs:188-207`.

2. **Threat model is written down and tested, not implied.** `SECURITY.md:34-56` separates gate, seatbelt, and what a `standard` shell can still read; `tests/gate.rs:1536-1591` keeps quote-split and `..` bypasses as intentional passing tests so nobody "fixes" them silently.

3. **Third-panel security fixes are mostly real code, not comment-only.** Private object store per run (`src/policy/mod.rs` via build 103, `src/worktree.rs:400-521`), watcher env scrub (`src/watch.rs:129-130`), global git config off for Hydra's git (`src/worktree.rs:140-147`), and admission on a byte copy (`src/rungs/evolve.rs:917+`, `src/worktree.rs:523-526`) each have failing-before tests named in ticket Outcomes.

4. **Test corpus design is honest about what CI proves.** `tests/normalize.rs:1600-1604` mirrors the live walk on fixtures; `README.md:227-231` says green CI on an empty `~/hydra/runs` proves fixture shapes, not this laptop's records.

5. **Self-improvement loop is wired, not slideware.** `src/rungs/evolve.rs:33-105` enforces `--may-edit` over a floor; `IMPLEMENTATION.md:3243-3259` records ten rung-two passes and one human-merged two-word patch; judges get baseline tests from `git show` per `IMPLEMENTATION.md:3218-3227`.

## Weaknesses and red flags

1. **Design smell: admission toolchain still user-writable.** Build 98 closes `~/.cargo/config.toml` forgery but admits Homebrew's `/opt/homebrew/Cellar/rust/...` remains writable by the same uid (``.scratch/hydra/build/98-admission-on-a-fresh-copy-of-the-read-tree.md:128-133``); this is the largest hostile-code gap left in rung two.

2. **Documentation gap: export redaction class count.** `src/export.rs:4` says four classes; `redact_secrets` at `src/export.rs:576-617` implements five prefix classes plus `openai_key_spans`; third panel H45 named this and build 100 did not touch the module doc.

3. **Scope limit (admitted): gate does not interpret shell.** `tests/gate.rs:1536-1591` and `SECURITY.md:18-28` document respellings the gate misses; seatbelt is the backstop on macOS (`src/sandbox.rs:76-90`).

4. **Scope limit (admitted): codex hook fail-open.** `src/provider/codex.rs:37-40` and `tests/drift.rs:406` measure it; `SECURITY.md:201-220` states the record cannot show a crash the hook never answered.

5. **Design smell: replay inherits environment.** `src/policy/replay.rs:206-234` sets only `PWD`; symlink or env drift can change gate answers under the same compiled hash (third panel H42, still unstated in `IMPLEMENTATION.md` beyond replay module doc).

6. **Bug-class residual: build 112 verification incomplete.** Ticket Outcome (``.scratch/hydra/build/112-two-residuals-of-105-and-106.md:32-35``) says revert-before-fail was not done; build 114 is still `claimed` to do that. Code for store copy on reap exists (`src/worktree.rs:452-472`) and harvest tests exist (`tests/harvest.rs:669-722`), but unverified tests are not the same as pinned fixes.

7. **Design smell: Master of Puppets not yet bounded by Hydra's deny list.** Map ticket 76 decision B and build ticket 110 (``.scratch/hydra/build/110-puppets-joins-the-deny-list.md:9-14``) require `~/hydra/puppets/` on `denied_write_paths`; until 110 lands, a `standard` run could rewrite goal logs and walk around approvals (`integrations/puppets/README.md:60-65`).

## The third panel, checked

| Finding | Ticket | Verdict | Checked at |
|---|---|---|---|
| H1 forged object store reaches parent | 103 | Fixed | `src/worktree.rs:444-521` copies store before `pack-objects`; `tests/cleanup.rs` forgery test in build 103 Outcome lines 98-111 |
| H2 `~/.cargo` runner forgery in admission | 98, 105 | Fixed with caveat | `src/rungs/evolve.rs:917+`, build 105 Outcome 98.1; Homebrew toolchain still writable per build 98 Outcome lines 128-133 |
| H3 watcher kept launcher env | 97 | Fixed | `src/watch.rs:129-130` `env_clear`; build 97 Outcome lines 56-63 |
| H5 Hydra git read `~/.gitconfig` | 99, 105 | Fixed with caveat | `src/worktree.rs:140-147`; `pinned_config` reads excludes/safe.directory back at 105 (`IMPLEMENTATION.md:3141-3156`) |
| H38 admission on writable checkout | 98, 105 | Fixed with caveat | Fresh tree copy in build 98; proposer processes never killed (``.scratch/hydra/build/98-admission-on-a-fresh-copy-of-the-read-tree.md:56-64``); Linux bare |
| H39 worktree failure leaves orphan | 99 | Fixed with caveat | Rollback not failure record (`src/worktree.rs` roll_back per build 99 Outcome lines 55-61); interactive post-spawn gap moved to 112 |
| H41 export `--out` aliases preference | 100 | Fixed | `src/export.rs:382-399` device/inode compare before truncate |
| H47 launch from inside checkout | 99, 105 | Fixed | `src/run.rs:561` `refuse_launch_inside_checkout`; build 105 adds skill-worktrees and evolve |
| Build 112 reap race + interactive record | 112 | Fixed with caveat | `src/worktree.rs:452-472` store copy; `tests/harvest.rs:669-722`; **not fixed** on process: revert verification absent per 112 Outcome lines 32-35 |

Papered-over items: H39's ticket asked for a record from the moment the worktree exists; build 99 chose rollback for early failures and 112 only partially closes the interactive window. H38's "stop proposer processes" was never built; location plus seatbelt inheritance is the substitute.

## Master of Puppets, read cold

Master of Puppets is a Python orchestrator (`integrations/puppets/puppets.py`) that keeps one append-only JSONL per goal, derives state by folding events (`puppets.py:129-280`), and launches `hydra run` only after approvals (`puppets.py:312-329`, tested in `integrations/puppets/tests/test_puppets.py:117-150`).

Table and launch approvals work as README claims: `approval_available` at `puppets.py:283-298` binds to `revision`, and `test_launch_refused_without_table_approval` at `test_puppets.py:117-123` fails without `--table`.

Before build ticket 110, a gated run under `standard` can write anywhere in its worktree and, by shell, under `~/hydra/puppets/` because that path is not yet on `denied_write_paths` (build 110 lines 9-14); a run could rewrite assignments or approvals in the log.

`choose` still records an empty judge list until build 111 (`IMPLEMENTATION.md:2905-2906`); plan selection is human or path-based, not panel-judged yet.

Engineering, not theatre: the fold, approval spend rules, and stub-tested launcher are real. Theatre: unbounded log directory, unimplemented judged plan/diff steps, and rankings that need hand-built sheets (`integrations/puppets/README.md:93-98`).

Resume line: yes, as "designed orchestration mode with append-only goal log and approval-gated `hydra run` launcher", not as "production multi-agent supervisor".

## Claims checked

| Claim | Source | Checked at | Verdict |
|---|---|---|---|
| Outcome derived from stream, not exit code (streamed runs) | `README.md:21-28` | `src/outcome.rs:13-19`, `122-278` | Holds |
| Env allowlist is whole list in `src/env.rs` | `README.md:40-46` | `src/env.rs:21-53`, `113-187`; Hydra-set names via `compose_child_env` at `src/run.rs:785` | Holds with caveat: record lists `HYDRA_RUN_ID`, `GIT_OBJECT_DIRECTORY`, etc. on `hydra_env`, not all in `BASE` |
| Gate refuses protected path only whole/canonical; tests pin respellings | `README.md:35-36`, `SECURITY.md:18-28` | `tests/gate.rs:1536-1591` | Holds |
| Codex hook that dies fails open | `README.md:38-39`, `SECURITY.md:201-207` | `src/provider/codex.rs:37-40`, `tests/drift.rs:406` | Holds |
| Seatbelt denies corpus under every spelling | `README.md:158-160`, `SECURITY.md:39-40` | `src/sandbox.rs:86-103`, build 103 spawn probe | Holds on macOS; gate substring floor still documented |
| Child never wider than parent | `README.md:138-139` | `src/run.rs:145-187` lineage; containment marker at `src/run.rs:616`, `785` | Holds with caveat: unset `HYDRA_RUN_ID` walks marker (`SECURITY.md:185-187`) |
| Concurrency cap is reservation under lock | `IMPLEMENTATION.md:3703-3726` | `src/live.rs:321-334` `flock`; `tests/status.rs` concurrent test cited in IMPLEMENTATION | Holds |
| `hydra reap` commits only on branch Hydra made | `README.md:31-32`, `SECURITY.md:82-83` | `src/reap.rs:480-484` refuses foreign branch before staging | Holds |
| Git in run checkout pins dirs from record / context | `SECURITY.md:86-88` | `src/worktree.rs:163-181` sets `GIT_DIR`, `GIT_COMMON_DIR`, `GIT_WORK_TREE` | Holds |
| Run writes objects to own store; reap uses `index-pack --strict` | `SECURITY.md:103-108` | `src/worktree.rs:444-521`; build 103 Outcome | Holds |
| Admission on fresh copy of read tree | `SECURITY.md:57-67` | `src/rungs/evolve.rs:917+`, `src/worktree.rs:523-526` materialize without `info/` | Holds with caveat: Linux, leftover proposer |
| Watcher and spawns start cleared env | `SECURITY.md:9-12` | `src/watch.rs:129-130`, `src/pty.rs:120`, build 97 Outcome table | Holds |
| Git outside seatbelt reads no config a run can write | `SECURITY.md:85-92` | `src/worktree.rs:140-147` | Holds with caveat: `pinned_config` re-reads excludes/safe.directory |
| Rung two offers tree its checks/judges read | build 89 / `IMPLEMENTATION.md:3194-3207` | `src/rungs/evolve.rs:1254-1281` `commit-tree` of read tree before admission side effects | Holds |
| Rung-two pass edits only `--may-edit` over floor | `README.md:59-60`, `IMPLEMENTATION.md:3278-3301` | `src/rungs/evolve.rs:33`, `684+`, `admission_refusal` | Holds with caveat: named file fully editable per map 73 |
| Export aborts on PEM/prefix; warns on entropy | `SECURITY.md:226-233` | `src/export.rs:576-617`, `635-647` | Holds |
| Codex reads seatbelt-denied copy of skills | `SECURITY.md:160-161` | `src/skills/mod.rs:404-436`, `src/skills/index.rs:69` | Holds |
| Non-ASCII root refused before launch | `SECURITY.md:29-31` | `src/paths.rs:75-90`, `src/run.rs:583` | Holds |
| Authorship table matches ticket Outcomes | `README.md:247-258` | Cross-check: 97-106 subagents noted; 110, 111, 113-115 open per `CLAUDE.md:28`, build 113 `claimed` | Holds with caveat: README says "109 build tickets resolved" while frontier says 110 of 115; table omits 108, 109 puppet builds |

## Interview questions

1. **Why derive outcome from the tool stream instead of exit code, and where does exit code matter?** Prompted by `src/outcome.rs:13-19` and `188-207`. Strong: ordered checks in `derive`, gated vs one denial, harvested exception. Weak: "vendors lie" without naming `refused_everything` or cursor's no-terminator failure path.

2. **Walk me through what happens when a run forges a pack in its private object store and someone runs `hydra reap`.** Prompted by `src/worktree.rs:444-521` and build 103. Strong: `GIT_OBJECT_DIRECTORY`, scrub, copy-then-pack, `index-pack --strict`, rehash from checkout. Weak: "git is secure" without the copy or alternates window build 106/112 discuss.

3. **The gate refuses `cat ~/hydra/runs/x` but allows `sh -c 'cat .../ru\"ns\"/x'`. Is that a bug?** Prompted by `tests/gate.rs:1552-1558`. Strong: intentional spec limit, seatbelt on macOS, why parsing shell is out of scope. Weak: promises to fix with regex without mentioning performance or false positives.

4. **Why does codex fail open when the hook dies, and what would you do about it?** Prompted by `src/provider/codex.rs:37-40`. Strong: vendor behavior measured in `tests/drift.rs`, record gap, `--dangerously-bypass-hook-trust`, seatbelt as compensating control. Weak: blames codex only, no mention of what the record can prove.

5. **Build 98 moved admission off the proposer checkout. What threat remained?** Prompted by `.scratch/hydra/build/98-admission-on-a-fresh-copy-of-the-read-tree.md:128-136`. Strong: Homebrew toolchain writable, PATH shadowing closed by absolute cargo, Linux no seatbelt. Weak: "copy fixes everything."

6. **Explain the concurrency reservation in `live::reserve`.** Prompted by `src/live.rs:321-376`. Strong: flock, pending json, handoff to line 0 pid, interactive counted but not reserving. Weak: "we cap at eight" with no race story.

7. **What can a `standard` run still do that SECURITY.md admits?** Prompted by `SECURITY.md:50-56`, `128-134`. Strong: read `~/.ssh`, lethal trifecta, network_read unscoped. Weak: "Hydra is a sandbox."

8. **How do you detect provider drift, and is a red corpus walk a CI failure or a feature?** Prompted by build 113 and `tests/normalize.rs:1594-1644`. Strong: capability table as alarm (map 43), fixture twin for CI, drift tests in `tests/drift.rs`; webSearch as unhooked tool like map 51 rows. Weak: "update the table" without unhooked vs gated distinction.

9. **Rung two: what may a pass edit, and why can a unit test inside a named file still weaken a guard?** Prompted by `src/rungs/evolve.rs:33`, map 73. Strong: floor vs `--may-edit`, `DEFAULT_MAY_EDIT`, admitted probe weakness. Weak: "AI wrote the patch."

10. **Master of Puppets: what did you build in Python instead of Rust, and what would ticket 110 change?** Prompted by `integrations/puppets/puppets.py:312-329`, build 110. Strong: orchestrator outside gate, fold semantics, deny-list gap for puppet logs. Weak: describes README only, no approval revision model.

## Before publishing

1. **Small:** Resolve build 113 (`webSearchToolCall` unhooked row) so live corpus walk is green again; `src/policy/table.rs:166-177`, `src/normalize/cursor.rs:119+`.

2. **Small:** Revert-verify build 112's four tests per build 114; update `IMPLEMENTATION.md` sentences for interactive failure record and reap store copy (`IMPLEMENTATION.md:3155-3156`, `3183-3184`).

3. **Small:** Fix `src/export.rs:4` redaction class count to match `576-617`; align `SECURITY.md:226-233` if needed.

4. **Small:** Close build 110 (puppets on deny list) before advertising Master of Puppets; `.scratch/hydra/build/110-puppets-joins-the-deny-list.md`.

5. **Large:** Map decision on binding `PATH`/toolchain directories for admission and for `standard` runs (third panel H2 tail); blocks claiming rung two is hostile-code isolated.

6. **Large:** Publish second and third panel review files cited by build tickets (`docs/reviews/`; third panel H13).

7. **Small:** Reconcile README build counts ("109 resolved" vs `CLAUDE.md:28` "110 of 115"); `README.md:244-245`, `281-282`.

## Method

**Read in full:** `README.md`, `SECURITY.md`, `CONTRIBUTING.md`, third panel review (first 200 lines plus findings table), build tickets 97 to 100, 103, 105, 106, 112 to 114, map issues 75 and 110, `src/outcome.rs`, `src/env.rs`, `src/watch.rs` (detach and env sections), `src/export.rs` (open path and redaction), `integrations/puppets/README.md`, `integrations/puppets/puppets.py` (fold, approvals, launch refusal), `integrations/puppets/tests/test_puppets.py` (first 150 lines), `tests/normalize.rs` (corpus walk), `tests/gate.rs` (bypass pins), `tests/harvest.rs` (112 tests).

**Read by range / grep:** `IMPLEMENTATION.md` sections via heading grep (Deriving an outcome, Git since 95, Master of Puppets, Rung two, concurrency cap, Choices); `src/run.rs` (preflight, containment, compose_child_env); `src/worktree.rs` (git env, import objects); `src/policy/table.rs`, `gate.py` (header and path logic); `src/sandbox.rs`; `src/live.rs` (reserve); `src/reap.rs` (branch refusal, inject holdback); `src/provider/codex.rs`; `src/normalize/cursor.rs`; `src/rungs/evolve.rs` (may_edit, admission); `src/policy/replay.rs`; `src/skills/mod.rs`; `src/paths.rs`; `tests/corpus/mod.rs`; `tests/drift.rs`; `.scratch/hydra/map.md` (Decisions list); `git log --oneline 59f57bf..85df6f6`.

**Skimmed:** `CLAUDE.md` (layout index), harness spec and run-record-contract (not full sections 6, 7, 9, 15 as instructed; relied on IMPLEMENTATION and SECURITY cross-refs where time-bound), third panel review remainder (second panel recheck, before publishing), build tickets 101, 102, 104, 107 to 111, 115, map issues 72 to 77, `integrations/puppets/sheet.py`, `src/rungs/review.rs`, `tests/cleanup.rs`, `tests/record_contract.rs`, `tests/git_config.rs`, `tests/status.rs` (by citation only).

**Not read:** Full `docs/harness-spec.md` sections 6, 7, 9, 15; full `docs/run-record-contract.md` sections 1 to 5; `src/interactive.rs` except via harvest test imports; `src/run.rs` spawn/handoff bodies line-by-line; Herdr integrations; full third panel prose after findings table; agent transcripts; live run records under `~/hydra/runs/` (refused by policy, as expected).
