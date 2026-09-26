# Four readers as a hiring panel, fourth round, 2026-09-24

Reviewed commit `ca2020f` on 2026-09-24. Build tickets 113 to 120 under `.scratch/hydra/build/` record what the session closed the same afternoon; map tickets 78 to 80 are open grillings for the human.

Three readers, `claude`, `cursor-agent` and `agy`, reviewed `ca2020f`. The brief was the third panel's with three changes: each reader is the hiring manager and names an interview level; a section "Master of Puppets, read cold"; and the question whether a corpus walk that goes red on vendor drift is the design working or a suite that goes red whenever a vendor ships. I checked their findings against the code and against those tickets' Outcomes before writing this file.

The fourth seat, `codex`, was launched three times and never wrote a report. Run 01a0d3e1-56f2 at 14:45 UTC died at 118 record lines on its usage limit; 01a0d483-b756 at 17:43 UTC stopped at 35 lines; the limit now reads "try again at Sep 27th, 2026 11:04 PM". Its partial text on the first attempt is not a review: it said the third panel's security repairs were substantive, that the documents understated the object-import repair while overstating ticket 112's interactive failure handling, and that marking the new Cursor tool "unhooked" would classify the gap rather than enforce a policy on it. I treat that as a fragment, not a fourth verdict. The `cursor-agent` seat was tried first on `claude-opus-5-thinking-high`, which came back `rate_limited` in six record lines (Cursor's premium allowance resets 2026-09-29), then on Composer 2.5.

## Verdicts

| Reader | Model | Verdict | Level | The reason, in one sentence | Depth |
|---|---|---|---|---|---|
| `claude` | Opus 5.5 | advance to technical interview | mid-level harness or agent-infrastructure; room for senior if ticket 112 and the puppets checkout land unprompted | `src/outcome.rs:188-206` ignored `run_end.error` on the harvested arm, so ticket 112's failure record derived `completed` until build 114, and `integrations/puppets/puppets.py:784-822` runs Hunk in a run-writable checkout outside the seatbelt, the class build 106 closed in Rust. | 4,811 words, 166 file:line citations, read `src/outcome.rs`, `src/interactive.rs`, `src/worktree.rs` and `puppets.py` by range and most of `IMPLEMENTATION.md`'s git and interactive sections; 15.7M tokens (15.4M cache reads) |
| `cursor-agent` | Composer 2.5 | advance to technical interview | mid-level harness or infrastructure; staff-loop potential only if the candidate owns the security tradeoffs and the agent-authorship process out loud | The central claim holds in code (`src/outcome.rs:122-278`, `tests/gate.rs:1536-1591`) and the third panel's fixes mostly survived measurement, but it filed stale gaps (Homebrew-writable toolchain, build 112 unverified) and misread `src/export.rs:4` as a live drift count. | 2,690 words, 137 citations, read by grep and range; did not open `src/interactive.rs` except through harvest tests; 1.68M tokens |
| `agy` | Gemini 3.1 Pro | advance to technical interview | mid-level systems or security | The prose reads mature on isolation and threat modeling, but its Method says it skipped `src/` and `tests/`, and nine of its "Before publishing" items restate third-panel findings that build tickets 97 to 105 had already closed. | 1,811 words, 60 citations, skimmed by string search; Method names no Rust file it opened; 0.85M tokens |
| `codex` | GPT, via the Codex CLI | no verdict | n/a | Three launches, 118 and 35 record lines, no report; the first fragment agrees the security repairs were real and questions whether an unhooked row is the right fix for Cursor drift. | 118 and 35 record lines, no report |

All three readers who finished advanced the candidate. The level stayed mid-level across the board, as in the third panel, and again the readers who leave room above it tie that room to ownership in the interview, not to the code alone. The panel is carried by `claude`, as before. It found the two bugs the session verified (ticket 112's derived outcome and the puppets checkout class), the design smell build 113 surfaced and map 79 now owns, and the corpus-walk question map 80 carries. The `cursor-agent` review is careful where it looked and added nothing the session checked beyond what overlaps the Opus report, except the authorship-table denominator, which was real at `ca2020f` and is fixed in build 119. The `agy` review repeated H38, H3, H39, H47, H41 and H1 as open; each was closed by build tickets 97 to 106. Weight it the way both earlier panels did for the reader who did not open the Rust.

## Findings

Every weakness, red flag, "does not hold" claim and before-publishing item across the three finished reviews, deduplicated. Status is fixed in build N, open as map N, wrong, or not yet checked. "Fixed" means the session read the ticket Outcome or the cited lines after the ticket landed. The `agy` reader's nine before-publishing items that restate H38, H3, H39, H1, H5, H41 and the contract enum are not rows here; they are named in the verdicts paragraph above.

| Id | What | Raised by | File:line | Kind | Status |
|---|---|---|---|---|---|
| I1 | Ticket 112's pid-carrying failure record derived `completed` because the harvested arm read only `exit_code` and never `run_end.error` | `claude`, `cursor-agent` | `src/interactive.rs:548-620`; `src/outcome.rs:188-206`; `src/record/lines.rs:186-198` | Bug | Fixed in build 114. Both interactive tests failed before with `Completed`; revert check on 112's four tests done in the same ticket |
| I2 | The same harvested arm turned a no-transcript record with exit 0 into `completed`, `derived` | `claude` | `src/interactive.rs:698-718`; `src/outcome.rs:188-206` | Bug | Fixed in build 114. Rule narrowed after the full suite: an error counts as failure only when the body holds no message, so a dead session harvested with its transcript stays `completed` (`tests/status.rs`) |
| I3 | Build 112's four tests had never been revert-verified | `claude`, `cursor-agent` | `.scratch/hydra/build/112-*.md:32-35`; `tests/harvest.rs:669-722`; `tests/cleanup.rs` | Design smell | Fixed in build 114. Literal revert failures in 114's Outcome |
| I4 | `cmd_review` runs Hunk with the run's worktree as cwd, outside the seatbelt, so git follows a writable `.git` file | `claude` | `integrations/puppets/puppets.py:784-822`; `src/run.rs:389-406` | Bug | Fixed in build 118. Review runs over `base_sha...hydra/<run-id>` after reap |
| I5 | `cmd_plans` copied with `shutil.copy2`, which follows links out of a run-writable tree | `claude` | `integrations/puppets/puppets.py:571-605`; `src/worktree.rs:537-597` | Bug | Fixed in build 118. Copy through `O_NOFOLLOW` with regular-file checks |
| I6 | `Capability::Unhooked` replaces the tool's real capability, so a Cursor judge under `judge` can search the web with no finding after build 113 | `claude`; `codex` fragment | `src/policy/mod.rs:54-57`; `src/normalize/cursor.rs:535-561`; `src/policy/table.rs:166-177` | Design smell | Open as map 79. Build 113 turned the walk green by adding the row; recommendation is keep the capability and flag the policy violation |
| I7 | `SECURITY.md:115-118` and `IMPLEMENTATION.md:3150-3184` described a scrub and an open window that the copy in `src/worktree.rs:452-475` closed | `claude` | `SECURITY.md:115-118`; `IMPLEMENTATION.md:3155-3184`; `src/worktree.rs:452-475` | Documentation gap | Fixed in build 114 |
| I8 | Comments and tests still said "build ticket 107" for work that is ticket 112; 107 is the `hy` launcher | `claude` | `src/worktree.rs:434`; `src/interactive.rs:536`; `tests/cleanup.rs:1786`; `tests/harvest.rs:665`, `724` | Documentation gap | Fixed in build 114 |
| I9 | Stale "three providers" and "`codex` is unmeasured" comments after build 87 routed `codex` | `claude` | `src/outcome.rs:14-15`; `src/policy/gate.py:17-19`; `src/mcp.rs:4`; `src/policy/mod.rs:965`; `src/normalize/mod.rs:106`, `265` | Documentation gap | Fixed in build 115 (`src/outcome.rs`) and build 119 (the rest) |
| I10 | `src/export.rs:4` said "four fixed classes" while readers counted six secret classes | `cursor-agent`; third panel H45 | `src/export.rs:4`; `src/export.rs:576-617` | Documentation gap | Wrong as a finding. Build 115 reworded to four kinds of redaction over six secret classes, which is what the code does; both this reader and the third panel misread the sentence |
| I11 | Authorship table stopped at 106 rows while the paragraph said 109 or 112 resolved; 108, 109 and 112 missing | `claude`, `cursor-agent` | `README.md:240-266` | Documentation gap | Fixed in build 119, and kept up as the afternoon's tickets closed. One denominator, 72 of 121 at the end of the day |
| I12 | Master of Puppets README said approvals enforced; `plan` kind accepted but unchecked; `diff` approval not bound to a run id; append with no lock | `claude` | `integrations/puppets/README.md:3`; `puppets.py:312-339`, `706-733`, `901-924`; `puppets.py:76-97` | Design smell | Partly fixed in build 118. README honest, `tasks` needs plan approval, diff carries run id, `flock` on append; human-presence still not proven |
| I13 | Admission toolchain resolves Homebrew's Cellar path, which a `standard` proposer can rewrite | `claude`, `cursor-agent` | `src/rungs/evolve.rs:885-915`; `SECURITY.md:72-75` | Admitted scope limit | Not yet checked beyond the readers' citations. Build 98 relabelled, not closed |
| I14 | Live corpus walks run inside `cargo test` against this machine's records and went red three times on vendor builds in one day | `claude`; `cursor-agent` Q8 | `tests/normalize.rs:1593-1597`; build tickets 113, 116, 117 | Design smell | Open as map 80. Detection was right; recommendation is move the live half to `hydra doctor` or a scheduled job |
| I15 | CI ran `cargo clippy` without `--all-targets` and never ran the puppets tests | `claude` | `.github/workflows/ci.yml:19-36` | Documentation gap | Fixed in build 120 |
| I16 | `webSearchToolCall` was absent from the Cursor table and normalizer, so the walk filed a leak | corpus walk; build 113 | `src/policy/table.rs:166-177`; `src/normalize/cursor.rs:119+`; `tests/normalize.rs` | Bug | Fixed in build 113. The policy question survives as I6 and map 79 |
| I17 | Claude Code 2.1.280+ prefixes hook refusals with `PreToolUse:<Tool> hook error: `, so `reason_code` missed live records | corpus walk on run 01a0d3e4-4245; build 116 | `src/normalize/claude.rs:855+`; `src/policy/replay.rs:76` | Bug | Fixed in build 116. Found on the Opus reader's own live record during the panel |
| I18 | Claude Code 2.1.281 renamed `Task` to `Agent`; the table still had only `Task` | corpus walk; build 117 | `src/policy/table.rs:90-126`; run 01a0d3e4-4245 | Bug | Fixed in build 117. Same live record and gate refusal both readers hit as `hydra/unknown-tool` |
| I19 | Claude Code writes large tool output under `~/.claude/projects/.../tool-results/` and the model `Read`s it back; `standard` refuses the read | corpus walk; map 78 | run 01a0d3e4-4245; `src/policy/mod.rs:785` (precedent for Cursor `cursor-data`) | Design smell | Open as map 78. Functional loss on 2.1.281, not a gate leak |
| I20 | Replay inherits the reader's environment and resolves paths with today's `realpath` | `cursor-agent` | `src/policy/replay.rs:206-234`; third panel H42 | Design smell | Not yet checked. Third panel row still open |
| I21 | A `codex` hook that dies fails open | `cursor-agent`, `agy` | `src/provider/codex.rs:37-40`; `SECURITY.md:201-220` | Admitted scope limit | Decided. Stated in `SECURITY.md` |
| I22 | Under `standard` the gate does not interpret shell | `cursor-agent`, `agy` | `tests/gate.rs:1536-1591`; `SECURITY.md:18-28` | Admitted scope limit | Decided. Pinned as passing tests |
| I23 | Until build 110 lands, a `standard` run can shell-write under `~/hydra/puppets/` | `cursor-agent`, `agy` | `.scratch/hydra/build/110-*.md:9-14`; `integrations/puppets/README.md:60-65` | Admitted scope limit | Fixed in build 110, the same afternoon, by the session that owns the mode: `~/hydra/puppets/` is in `denied_write_paths` and the seatbelt profile, with a test through the real hook |
| I24 | Vendored pixel-art mascot under `integrations/herdr/mascot/` | `claude` | `integrations/herdr/mascot/hydrapet.py` | Documentation gap | Not yet checked |
| I25 | Untracked `pendintes_hydra.md` reached the reviewer's worktree through the seed | `claude` | worktree seeding | Admitted scope limit | Working as designed |

Twenty-five findings. Fixed: I1 to I5, I7 to I9, I11, I15 to I18, I23. Partly fixed: I12. Open as map tickets: I6, I14, I19. Admitted and stated: I13, I21, I22. Wrong: I10. Not yet checked: I20, I24. Working as designed, and for the human to change: I25.

By reader:

- `claude` raised or touched 20 distinct findings. None is wrong. I12 is partly closed; I13 and I14 remain the largest open items after the afternoon's builds.
- `cursor-agent` raised 12 that overlap heavily with `claude`. I10 is wrong. It added nothing the session verified beyond I11 and the shared third-panel recheck.
- `agy` raised eight weaknesses that are almost all third-panel repeats, wrong at `ca2020f`. Its Method says it skipped `src/` and `tests/`.

## The third panel, rechecked

The third panel's H1 to H48 and build tickets 97 to 106 were the baseline. Here is what each finished reader said about the fixes, and what I found at `ca2020f` and after tickets 113 to 120.

| Finding | `claude` | `cursor-agent` | `agy` | Checked |
|---|---|---|---|---|
| H1, shared object store writable | Fixed. Copy and `index-pack --strict`; documents trail the code | Fixed | Not fixed | Fixed. Build 103 and 106; `agy` wrong |
| H2, `~/.cargo` runner forge | Fixed with caveat. Toolchain directory still user-writable | Fixed with caveat | Not fixed | Fixed with caveat. Builds 98 and 105; Homebrew path open; `agy` wrong on the forge |
| H3, watcher kept launcher env | Fixed | Fixed | Not fixed | Fixed. Build 97; `agy` wrong |
| H4, `codex` quota derived `failed` | Fixed | (not in table) | (not in table) | Fixed. Build 100 |
| H5, git read run-writable config | Fixed with caveat. Two values still read as data | Fixed with caveat | Fixed with caveat | Fixed with caveat. Builds 99 and 105 |
| H38, admission on writable checkout | Fixed. Materialized copy | Fixed with caveat | Not fixed | Fixed. Builds 98 and 105; `agy` wrong |
| H39, failure after `worktree add` leaves orphan | Fixed with caveat. Rollback, not record | Fixed with caveat | Not fixed | Fixed with caveat. Build 99 rollback; `agy` wrong |
| H41, `--out` aliases preference | Fixed | Fixed | Not fixed | Fixed. Build 100; `agy` wrong |
| H47, launch from inside checkout | Fixed for Hydra git; back in `puppets.py` | Fixed | Not fixed | Fixed in Rust. Build 99; puppets gap closed in build 118; `agy` wrong |
| Build 112, store copy and interactive failure record | Not fixed as closed. Derives `completed`; 114 still claimed | Fixed with caveat. Revert check absent | Not fixed | Fixed after build 114. Store half was always real; outcome bug was real |
| H42, replay env and symlinks | (not raised) | Still unstated | (not raised) | Not yet checked |
| H10 to H13, H44, H45 doc drift | Named in weakness 4 and before publishing | Named export line 4 only | Named contract enum | Mostly fixed in builds 115 and 119; H45's export line was a misread (I10) |

Disagreements worth naming: on build 112, `cursor-agent` counted the store copy and harvest tests as fixed while `claude` read the outcome bug; `claude` was right and build 114 closed it. On H2, both Rust readers call the toolchain caveat admitted; neither treats it as papering over. On the corpus walk, `claude` and `cursor-agent` agree the red runs were real drift; they split on whether that belongs in `cargo test` (map 80). `agy` disagreed with all three Rust readers on whether H38, H3, H39, H47 and H41 were fixed, and was wrong on each.

## Master of Puppets, read cold

Both `claude` and `cursor-agent` read `integrations/puppets/puppets.py` cold. `agy` named the fold from the README and tests it did not open.

`claude` called the log-and-fold shape engineering and the README's "approvals enforced" theatre. It found `cmd_review` running Hunk inside a run checkout and `cmd_plans` following links (I4, I5), unchecked `plan` approvals, diff approvals not bound to a run, and unlocked append (I12). Build 118 closed the checkout class the way build 106 closed it in Rust: review over the reaped branch, plan copy without following links, plan approval enforced on `tasks`, diff approval carrying the run id, append under `flock`, README saying an approval is a log line the CLI checks. `sheet.py`'s beta ranking over hand-set priors is honest and measures little; `claude` would not put the mode on a resume line yet.

`cursor-agent` read the same file as orchestration outside the gate: table and launch approvals work (`test_puppets.py:117-150`), but until build 110 lands a `standard` run can shell-write goal logs (I23), and build 111's judged plan step is still open. It would put a narrower resume line: approval-gated launcher, not production supervisor.

They agree the fold is real and the security boundary was incomplete at `ca2020f`. They disagree on resume weight and on how much build 118 plus an open 110 closes the story.

## Interview question bank

Twenty-eight questions came in across the three reviews. Deduplicated, eighteen remain. The top five are 1 to 5.

The rule matches round three: the question a reviewer who reads the code will hit first, and the answer that costs the most if the candidate has not thought about it. Four of the five are about findings still open or only partly closed when the panel ran.

### Top five

1. Top five. "Ticket 112's agent ran out of quota before reporting, and you merged its diff. Walk me through the record `record_then_wait` writes when the start time will not read, and tell me what `hydra show` derived before build 114." Merged from `claude`'s first and tenth and `cursor-agent`'s build-112 weakness. Motivated by `src/interactive.rs:548-620` and `src/outcome.rs:188-206`. A strong answer traces `RunEnd::harvested` to `body_source: harvest`, finds the arm mapping a null exit code to `completed` while ignoring `run_end.error`, notices rung one learns from it, and proposes reading `error` before `exit_code` with a narrow rule for harvested transcripts. On process it says revert-before-green, which build 114 did. Worrying: "the tests pass", or blaming the agent.

2. Top five. "Build 113 made `webSearchToolCall` an unhooked row. Afterwards, what does the record show when a Cursor judge under `judge` searches the web?" From `claude`'s second and the `codex` fragment. Motivated by `src/normalize/cursor.rs:535-561` and map 79. A strong answer: `Unhooked` erases the capability, the call becomes a provider `Allow` with no finding, and the walk went green without enforcing `judge`. The alternative keeps the real capability and flags a policy violation. Worrying: "it's recorded, so it's fine".

3. Top five. "You spent a week making Hydra's git distrust a checkout's `.git` file. Read `cmd_review` and `cmd_plans` with me." From `claude`'s third. Motivated by `integrations/puppets/puppets.py:571-605`, `784-822`, and build 118's Outcome. A strong answer spots Hunk and `copy2` outside the seatbelt and names the same fixes build 106 used in Rust. Then it says why the lesson did not transfer until a panel read Python. Worrying: "that's Python, not the harness".

4. Top five. "Your suite went red because Cursor and Claude shipped builds. Is that a test?" From `claude`'s eighth and `cursor-agent`'s eighth. Motivated by `tests/normalize.rs:1593-1597` and map 80. A strong answer: fixture twins are regression tests; the live walk is a monitor for `hydra doctor` or a scheduled job; the claude side fails closed at the gate, which is a different risk from an unhooked web search under `judge`. Worrying: "red on drift is the point" with no answer for contributors.

5. Top five. "Claude Code now writes big tool output under `~/.claude/projects/` and reads it back. Your gate refused that read on the Opus reader's own run. What should `standard` do?" From map 78, found during the walk on run 01a0d3e4-4245. Motivated by build 38's Cursor carve-out at `src/policy/mod.rs:785`. A strong answer names the functional loss, compares a slug-scoped read carve-out to moving the directory Hydra owns, and cites the skills-under-`~/.claude` tension. Worrying: treating the refusal as pure safety with no product cost.

### Design decisions

6. "Why derive outcome from the tool stream instead of exit code, and where does exit code matter?" From `cursor-agent`'s first. Motivated by `src/outcome.rs:13-19`, `122-278`, `188-207`. Strong: ordered checks, harvested exception, ticket 112 as the case where harvest lied. Weak: "vendors lie" without naming `refused_everything`.

7. "Why a UUIDv7 run id, and why does `src/env.rs` drop a `HOME` that disagrees with the passwd entry?" From `agy`'s second and sixth. Motivated by `src/run.rs:6` and `src/env.rs:99-104`. Strong: time-ordered ids for corpus walks; wrong `HOME` logs Claude out of keychains. Weak: "arbitrary" for either.

8. "In `worktree::create` you roll back on failure. In `record_then_wait` you write a failure record instead. D10 says what is not in a record did not happen. Defend both." From `claude`'s seventh. Motivated by `src/worktree.rs:1079-1135` and `src/interactive.rs:527-547`. Strong: nothing ran versus a live session, and rollback should still write `never_started`. Weak: no tension seen.

9. "Your ranking puts a beta posterior on human verdicts with priors you set by hand. With three verdicts in a class, what is it measuring?" From `claude`'s sixth. Motivated by `integrations/puppets/sheet.py:176-239`. Strong: priors dominate; proposes held-out tasks and cost per acceptance. Weak: the bound treated as the result.

10. "Master of Puppets: what did you build in Python instead of Rust, and what would ticket 110 change?" From `cursor-agent`'s tenth and `agy`'s ninth. Motivated by `integrations/puppets/puppets.py:312-329` and build 110. Strong: orchestrator outside the gate, fold semantics, deny-list gap. Weak: README only.

### Security model and its limits

11. "Walk me from a forged pack in the private object store to what `hydra reap` and the next seed see." From `cursor-agent`'s second and `agy`'s second. Motivated by `src/worktree.rs:444-521` and build 103. Strong: per-run store, copy-then-pack, `index-pack --strict`. Weak: "content addressing prevents that".

12. "Build 98 moved admission off the proposer checkout. What threat remained?" From `cursor-agent`'s fifth and `claude`'s ninth. Motivated by `src/rungs/evolve.rs:885-915`. Strong: Homebrew toolchain writable; judges do not replace a pinned toolchain. Weak: "the copy fixes everything."

13. "The gate refuses `cat ~/hydra/runs/x` but allows a respelled path. Is that a bug?" From `cursor-agent`'s third and `agy`'s sixth. Motivated by `tests/gate.rs:1552-1558`. Strong: intentional limit, seatbelt backstop. Weak: promises regex shell parsing.

14. "Why does codex fail open when the hook dies?" From `cursor-agent`'s fourth and `agy`'s seventh. Motivated by `src/provider/codex.rs:37-40` and `tests/drift.rs`. Strong: measured vendor behavior, record gap, compensating seatbelt. Weak: blames codex only.

15. "What can a `standard` run still do that SECURITY.md admits?" From `cursor-agent`'s seventh and `agy`'s implied trifecta rows. Motivated by `SECURITY.md:50-56`, `128-134`. Strong: `~/.ssh`, lethal trifecta, unscoped network. Weak: "Hydra is a sandbox."

16. "Replay recompiles today's gate. Which inputs can change its answer after a run ends?" From `cursor-agent`'s weakness 5 and third panel H42. Motivated by `src/policy/replay.rs:206-234`. Strong: `realpath` and inherited env; proposes recording resolved paths. Weak: "same bytes, so replay is pure."

17. "Ticket 105 found that a `file-read-metadata` rule beats `file-read*` whatever their order. How do you know the launch profile has no inversion?" From `claude`'s fifth. Motivated by `src/rungs/evolve.rs:1002-1008` and `src/sandbox.rs:76-160`. Strong: proposes generated sandbox-exec probes. Weak: "SBPL is last-match."

### Evaluation and trajectories

18. "A `--may-edit src/normalize/` pass could weaken a gate probe in that directory. Is that acceptable?" From `cursor-agent`'s ninth and map 73. Motivated by `.scratch/hydra/issues/73-*.md:62-64`. Strong: human names the risk; baseline tests refused. Weak: "the corpus protects every probe."

### Working with agents

19. "Build 108's Outcome says your session changed five mechanisms in Composer's puppets diff. Pick one. What was wrong, and would you have caught it without writing the ticket?" From `claude`'s tenth; `agy`'s tenth asked the same thing more loosely. Motivated by `.scratch/hydra/build/108-puppets-log-and-launcher.md`. Strong: names the table approval spent by the first launch or the plan read from a worktree file the brief never asked for, says what they check in an agent diff now, and admits the same review did not catch the link-following or the checkout git that build 118 closed. Weak: a narrative with no mechanism, or "the tests passed".

## Before publishing

The union of the three readers' lists plus the fixes the findings imply, ranked by how much each would hurt if a reviewer found it first. Status is done in build N, open as map N, or for the human.

1. Done in build 114. Make ticket 112's failure record and the no-transcript path derive `failed`, with outcome assertions and a revert check before calling 112 resolved (I1 to I3).
2. Done in build 118. Stop running Hunk inside a run checkout and stop following links in `plans`; say what an approval is in the README (I4, I5, I12).
3. Open as map 79, for the human. Keep the real capability on unhooked Cursor rows and flag policy violations under `judge`, rather than silencing the walk (I6). The `codex` fragment pointed at the same fork.
4. Done in build 114. Describe the store copy, not the scrub; fix "build ticket 107" (I7, I8).
5. Done in builds 115 and 119. Stale provider counts and authorship rows (I9 to I11).
6. Open as map 80, for the human. Move the live corpus walk to `hydra doctor` or a scheduled job; keep fixture twins in the suite (I14).
7. Done in build 120. Run clippy with `--all-targets` and the puppets unit tests in CI (I15).
8. Done in builds 113, 116, 117. Provider drift rows for `webSearchToolCall`, the hook-error prefix, and `Agent` (I16 to I18).
9. Open as map 78, for the human. Claude Code's out-of-worktree tool-result reads under `standard` (I19).
10. Not yet checked. State the replay residual or store evidence that survives symlink drift (I20).
11. Done in build 110, the same afternoon. `~/hydra/puppets/` is denied to writes under every policy and named in the seatbelt profile (I23).
12. Not yet checked. Optional: move the vendored mascot out of the security repo (I24).
13. Working as designed; keep private notes out of the checkout reviewers launch from (I25).

Dropped: `agy`'s nine items that restate build tickets 97 to 105 (admission copy, watcher `env_clear`, worktree rollback, object store, `GIT_CONFIG_GLOBAL`, `codex` in the enum, `--out` inode check); the `cursor-agent` item to fix `src/export.rs:4` as six classes (I10, wrong); `cursor-agent`'s "resolve build 113" as a before-publish fix without map 79's decision (113 is done; the policy question is map 79).

## Method

This file was drafted by Composer 2.5 on a `hydra run` under `standard` (run 01a0d486-07ac, 0.74M tokens) from a task that carried every fact the session had verified, and the session then read it whole and corrected five things: build 110 had landed while the draft was written, the authorship denominator had moved, question 19 was not a question, and this section said "I" for work the drafter had not done. The verification behind the status column is the session's, not the drafter's.

What the session verified, and how: I1 and I2 by reading `src/outcome.rs`'s harvested arm and `src/record/lines.rs` before build 114, and by build 114's tests failing with `left: Completed, right: Failed` before its change; I3 by build 114's revert check, four literal failure messages in its Outcome; I16 to I18 by measurement on the corpus, the walk's own panic naming the record and the tool, and by reading the record's hook frames; I6 by reading `src/normalize/cursor.rs`'s unhooked arm; I7 to I11 and I15 by reading the diffs of builds 114, 115, 119 and 120 before applying them; I4, I5, I12 and I23 from build 110's and 118's Outcomes, written by the session that owns the mode, not by reading `puppets.py`. Nothing in the `cursor-agent` or `agy` reports was verified beyond what overlaps the Opus report.

The drafter read in full the third panel's consolidated report for shape and voice, the three reader reports, build tickets 113 to 120 and map tickets 78 to 80, and counted words with `wc` and `file:line` citations with a regex (166, 137 and 60). It did not run the suite, build or launch a provider, and could not read `~/hydra/runs`.

Not done by anyone: verify I13, I20 and I24 against fresh lines; reproduce the outcome bug or the puppets findings on a live run; read the `codex` partial records beyond their last frames.

The two readers' outreach blurbs, the sentences the brief asked each hiring manager to write for a
recruiter, were removed from their raw reports on 2026-09-26 before the repository went public. They
praised the repository in the voice of a company that does not exist, and in a public repository
that reads as an endorsement. Nothing else in the reports changed.
