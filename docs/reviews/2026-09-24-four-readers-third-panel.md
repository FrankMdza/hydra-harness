# Four readers as a hiring panel, third round, 2026-09-23

Reviewed commit `2e03429`, and `04740db` for the `codex` reader, which has the same source; published on 2026-09-24 as written then.
Build tickets 97 to 106 under `.scratch/hydra/build/` record what was fixed, and build ticket 112 what is still open.

Three readers, `claude`, `cursor-agent` and `agy`, reviewed `2e03429`, and their 37 findings, H1 to
H37, were checked there first in `verified-three.md`. The fourth, `codex`, launched at 18:35 after
its usage limit reset. By then master was `04740db`, and its worktree's `base_tree` is `04740db`'s
tree plus the untracked `pendintes_hydra.md`. Its review says `2e03429` because the brief did. The
two commits differ only in five lines of build ticket 92's Outcome, which since `04740db` admits
that `phase_spawn` is 163 lines. That is probably why `codex` is the one reader who did not raise
H6. No source file differs, so its rows H38 to H48 were checked at `04740db` against the same code.

`codex` brought eleven new findings and repeated eight. None of its nineteen was wrong. Two of the
eleven I reproduced outside Hydra: the case-insensitive alias behind H41, and the decoy hook
behind H47, which `codex` only named as a scope question.

## Verdicts

| Reader | Model | Verdict | Level | The reason, in one sentence | Depth |
|---|---|---|---|---|---|
| `claude` | Opus | advance to technical interview | mid-level, room for senior if the ownership questions land | `src/outcome.rs:122-280` derives the outcome from the tool calls in a stated order, and `tests/gate.rs:1536-1600` pins open bypasses as passing tests, but cheap exposures (the watcher's environment, `~/.gitconfig`) are filed as stated rather than closed. | 4,072 words, about 185 file:line citations, read `src/sandbox.rs` and `src/outcome.rs` in full and most of `run.rs`, `evolve.rs` and `gate.py` by range; 8.8M tokens |
| `cursor-agent` | Composer 2.5 | advance to technical interview | mid-level harness or infrastructure, "staff-loop potential" if the candidate owns the security tradeoffs out loud | The central claim is implemented rather than marketed, and the response to the second panel was fast and backed by tests; the Rust is competent agent-assisted code, not staff-level authorship. | 2,492 words, about 92 citations, read by grep and range; did not open `src/watch.rs` or the normalizers; 1.4M tokens |
| `agy` | Gemini 3.1 Pro | advance to technical interview | mid-level systems or security | The sandbox and the policy compiler show systems and security skill above what the candidate's background predicts. | 1,817 words, about 49 citations, skimmed the Rust by string search; its method says `src/live.rs` was never opened; 0.6M tokens |
| `codex` | GPT, via the Codex CLI 0.154.0 | advance to technical interview | mid-level, not senior ownership of hostile-code isolation | Outcomes keep evidence apart from interpretation, but admission checks a writable checkout and a launch that fails inside `worktree::create` leaves state with no record, which is below what it would ask of a senior owner. | 4,162 words, about 178 citations, read `worktree.rs`, `gate.py`, `sandbox.rs`, `outcome.rs` and `replay.rs` in full; 7.9M tokens |

All four readers advanced the candidate in both rounds. What moved is the level. In the second
panel `agy` said senior and `cursor-agent` said senior "only if the candidate can explain
`src/policy/replay.rs:9-17` unprompted". This time all four say mid-level. The two readers who
leave room above that tie it to ownership in the interview, not to the code: `claude` says room
for senior, `cursor-agent` says staff-loop potential. `codex` held its level and its reason
changed. In round two it was G41 and G42. Now it is the half of G42 the fix left open, H38, and
the half of G4 that ticket 92 did not reach, H39.

The panel is carried by `claude` and `codex` again, and this time they barely overlap. `claude`
found the three most serious findings of the first three reviews, H2, H3 and H4, and two of them
held up when measured. `codex` found eleven more, all real, most of them in code the other three
did not open: `worktree::create`, `skills::inject`, the export's file names, the replay's inputs,
the `codex` MCP writer and its login probe. Between them they cover the repository. The
`cursor-agent` review is careful where it looked, but three of its fifteen findings are wrong and a
fourth is off by one. The `agy` review repeated G30, the concurrency cap "without a lock", for the
second panel in a row. It rests on a line that is the depth cap's doc comment and on a history
sentence that says the gap was closed by build 81. Its other seven weaknesses are admitted limits
read back from `SECURITY.md`. Weight it the way both earlier panels did.

## Scores

| Axis | `claude` | `cursor-agent` | `agy` | `codex` | Mean | Second panel mean | Spread |
|---|---|---|---|---|---|---|---|
| Rust engineering quality | 7 | 7 | 8 | 6 | 7.00 | 7.0 | 2 |
| Test discipline | 8 | 8 | 8 | 7 | 7.75 | 7.3 | 1 |
| Security and threat-model honesty | 7 | 9 | 9 | 7 | 8.00 | 7.0 | 2 |
| Documentation and communication | 7 | 8 | 8 | 5 | 7.00 | 7.0 | 3 |
| Depth and originality of the harness idea | 9 | 9 | 9 | 8 | 8.75 | 8.5 | 1 |
| Engineering process | 8 | 7 | 9 | 7 | 7.75 | 7.8 | 2 |
| Mean over the six axes | 7.67 | 8.00 | 8.50 | 6.67 | 7.71 | 7.42 | |

The panel mean rose from 7.42 to 7.71, and nearly all of it is `claude`. Its six scores went from
6, 6, 5, 5, 8, 7 to 7, 8, 7, 7, 9, 8, a mean of 6.17 to 7.67. Security moved most, 5 to 7, because
G1 is fixed and a test watches the child. `codex` moved from 6.50 to 6.67. It raised security 5 to
7 for the same repairs and dropped documentation 6 to 5, for a contract enum without `codex`, a
stale exit-status paragraph and a redaction count that is wrong in two places. `cursor-agent` fell
from 8.17 to 8.00, one point on Rust for `phase_spawn`. `agy` fell from 8.83 to 8.50 and is still
the highest scorer on the least reading.

The security split closed from four points to two. In round two it was 5, 9, 9, 5, divided exactly
by who had read the launch and git paths. Now `claude` and `codex` score 7, and the
gap to the two 9s is H1 to H5 and H38, which the high scorers either did not reach or filed as
admitted limits. Documentation is the one axis where the spread grew, and `codex` is the outlier.
Its 5 is the fairest number in the column: H7 to H13 and H44, H45 and H48 are thirteen live drifts
in a repository whose closing rule is "amend `IMPLEMENTATION.md` in the same change". Plan against
the `claude` and `codex` columns again.

## Findings

Every weakness, red flag, "does not hold" claim, caveated or unfixed second-panel verdict and
before-publishing item across the four reviews, deduplicated. H1 to H37 are the rows of
`verified-three.md`, checked at `2e03429`, with `codex` added to the raisers where it said the same
thing. H38 to H48 are the rows only `codex` raised, checked at `04740db`, which has the same source.
Status is real, partly, wrong or unconfirmed. "Measured" means someone ran something, and the
Method section says what. Everything else was checked by reading the cited lines. Cost is small
(under an hour, no decision), large, or needs a decision (a map ticket). New means no G row of the
second panel covers it. Repeat names the G row.

| Id | What | Raised by | File:line | Status and check | New or repeat | Cost |
|---|---|---|---|---|---|---|
| H1 | The shared object store is fully writable, so a forged pack reaches rung two's offered branch and every later worktree seed, not only reap; `SECURITY.md` names reap alone | `claude`, `cursor-agent`, `codex` | `src/policy/mod.rs:640-646`; `src/rungs/evolve.rs:238-241`, `1254-1281`; `SECURITY.md:75-82`; `.scratch/hydra/issues/74-*.md:14-24` | Real, by reading. `own_write_paths` grants `objects` whole. Admission runs under the proposer's own profile after the tree is read, so code it runs can plant a pack that maps the read sha to other bytes. Map 74 measured that git serves the forged bytes when the forged pack is the only copy. `commit-tree` binds a sha and nothing more | Repeat, the object half of G41, now map 74 | Needs a decision (map 74, open). The `SECURITY.md` sentence is small |
| H2 | Rung two's floor refuses `.cargo/` in the repository while the proposer can write `~/.cargo/config.toml`, whose `runner` forges `cargo test` and persists into the human's own builds | `claude`, `cursor-agent` | `src/rungs/evolve.rs:497-512`, `767-773`, `809-855`; `src/env.rs:43-52`; `src/policy/mod.rs:554-605` | Real, measured. A failing test exits 101, and with `runner = "/usr/bin/true"` in `$CARGO_HOME/config.toml` it exits 0 with "test result: ok". Admission passes `HOME`, and `CARGO_HOME` is unset on this machine. `cargo` also resolves to `/opt/homebrew/bin`, which `frank` can write | New | Large: scratch `CARGO_HOME` and an absolute toolchain for admission; `PATH` directories need a decision |
| H3 | The watcher that owns the pty keeps the launching shell's whole environment, readable with `ps -E`; the first sentence of `SECURITY.md` promises no shell credential reaches the agent | `claude`, `cursor-agent` | `src/watch.rs:103-134`, `17-20`; `src/sandbox.rs:79`; `SECURITY.md:5-7` | Real, measured on the host side: `ps -Eww` read a same-uid sleeper's planted variable on macOS 26.2. `detach` never calls `env_clear`. The human's own terminals expose the same keys, so clearing the watcher alone would not make the sentence true | New | Small for the sentence; large for a watcher started from `env_clear` with a test that reads its environment |
| H4 | A `codex` run that hits its quota derives `failed`, and rung one learns from `failed` | `claude` | `src/normalize/codex.rs:177-186`; `src/outcome.rs:74-79`, `233-241` | Real, measured on record `01a0cfdb-e5c3`, which `hydra show` prints as `failed`, `derived`. `turn.failed` appears nowhere in `src/` or `tests/`, and unknown frame types are dropped with no `Finding::Limit` | New | Small |
| H5 | Git that Hydra runs outside the seatbelt still reads a run-writable `~/.gitconfig` and a user-owned system config; `SECURITY.md` calls it "the same class as `~/.zshrc`", but reap runs it | `claude`, `cursor-agent`, `codex` | `src/worktree.rs:127-142`, `227-247`; `SECURITY.md:79-82` | Real. `CheckoutGit::command` sets neither `GIT_CONFIG_GLOBAL` nor `GIT_CONFIG_NOSYSTEM`, and the system config here is `/opt/homebrew/etc/gitconfig`, owned by `frank`. `codex` adds that hooks off is not filters off | New as a finding; build 95's outcome names it | Small; the `git` binary on `PATH` is H2's decision |
| H6 | Ticket 92's done-when says no function in `src/run.rs` is over 150 lines; `phase_spawn` is 163 | `claude`, `cursor-agent` | `src/run.rs:1182-1344`; `.scratch/hydra/build/92-*.md:36` | Real, measured. `phase_handoff` is 149, not 150 as `cursor-agent` said. `04740db` corrected the ticket's Outcome after the panel; the function is unchanged | New | Small; the ticket half is done |
| H7 | The authorship table stops at 94 "except 80 and 92"; 92, 95 and 96 are resolved and agent-written | `claude` | `README.md:222-245` | Real. `frontier.py build` prints 96 of 96 | Repeat of the G13 class | Small |
| H8 | "73 decision tickets, all resolved", while map ticket 74 is open | `claude`, `codex` | `README.md:255` | Real. `74-*.md:4` is `Status: open` | Repeat of the G7 class | Small |
| H9 | "23 test files"; there are 24 | `claude`, `codex` | `README.md:210`; `CLAUDE.md:102` | Real. `tests/launch_phases.rs` is the new one | New | Small |
| H10 | `IMPLEMENTATION.md` says nothing reads the exit status to pick an outcome | `claude`, `codex` | `IMPLEMENTATION.md:2054-2056`; `src/outcome.rs:13-19`, `197-205` | Real. The paragraph reads "Nothing reads the exit status to pick an outcome". At 197-205 the harvested arm maps `exit_code` 0 to `Completed` and anything else to `Failed`. The module comment was fixed; this paragraph was not | Repeat, G15 | Small |
| H11 | The harness spec says the floor "applies on all three providers" | `claude` | `docs/harness-spec.md:819` | Real. Four are routed | Repeat, G11 | Small |
| H12 | The provider version table has no `codex` row | `claude` | `IMPLEMENTATION.md:86-92` | Real | New | Small |
| H13 | The second panel's findings, which tickets 88 to 96 cite by G id, are not in the repository | `claude` | `.scratch/hydra/build/93-*.md:9-11`; `docs/reviews/` | Real | New | Small |
| H14 | CI walks an empty corpus | `claude`, `codex` | `README.md:209-213`; `tests/record_contract.rs:577-605` | Real, and the README says so. The red local walk `claude` mentions is history, fixed in `963bdd4` | Repeat, G21 | Large, a fixture corpus |
| H15 | A `codex` hook that dies fails open, and no automated test drives it | `claude`, `cursor-agent`, `agy`, `codex` | `src/provider/codex.rs:37-40`; `SECURITY.md:155-166` | Real and admitted. The G19 half is closed, `tests/gate.rs:2401` drives the catch-all | Repeat, G20 | Large |
| H16 | The admission canary test calls `set_var` in a unit-test binary, and its SAFETY comment names the wrong invariant | `claude` | `src/rungs/evolve.rs:1540-1571`; `CONTRIBUTING.md:27` | Real. Rung two's own admission runs a threaded `cargo test` (`evolve.rs:773`), so the race is live there | New | Small |
| H17 | `CheckoutGit::for_run` takes a `_layout` it never reads | `claude` | `src/worktree.rs:72-77` | Real | New | Small |
| H18 | Record fields are read as untyped `serde_json::Value` paths | `claude` | `src/reap.rs:329-351` | Real, a design smell | Same family as G16 | Large |
| H19 | Admission's `cargo` compiles ignored files that are not in the read tree | `claude` | `src/rungs/evolve.rs:437-443` | Partly. A tracked file still has to name the ignored one, and the judges see that line, so the damage is an offered branch that does not build, not hidden code. H38 is the general form | New | Small |
| H20 | A unit-test probe inside a file named by `--may-edit` can be weakened with the code it guards | `claude`, `cursor-agent` | `.scratch/hydra/issues/73-*.md:62-64`; `src/rungs/evolve.rs:494-512` | Real and admitted in map 73 | Repeat, G44 | Decided |
| H21 | Under `standard` a run holds all three legs of the lethal trifecta | `cursor-agent`, `claude` | `SECURITY.md:92-98` | Real and admitted, map 69 | New to the list, admitted since map 69 | Decided |
| H22 | Agent authorship is heavy, and from 89 on every agent diff needed an Opus fix | `cursor-agent`, `claude` | `README.md:239-245` | Partly. A disclosure, not a defect; an interview risk | Repeat, G14 | Small, interview preparation |
| H23 | The concurrency cap is a check without a lock | `agy` | `src/run.rs:316`; `IMPLEMENTATION.md:3740` | Wrong, the second time. Line 316 is the depth cap's doc comment; `live::reserve` holds `flock` across the scan (`src/live.rs:329-334`) and `tests/status.rs:501` races it. H48 is where the misreading comes from | Repeat, G30, wrong both times | None |
| H24 | The reservation does not cover the whole launch window, and "a child is never wider" does not survive an unset marker | `cursor-agent`, `codex` (as caveats in its claims table) | `IMPLEMENTATION.md:3476-3479`; `SECURITY.md:132-148` | Partly. Interactive launches count from line 0, as stated. A child that unsets the marker is admitted, and the seatbelt's corpus deny is what stops it | Repeat, G26 and G29 | None beyond what is stated |
| H25 | Under `standard` a shell reads `~/.ssh` and the rest of home | `agy` | `SECURITY.md:44-59` | Real and admitted | Repeat | Decided |
| H26 | The gate folds ASCII case only; non-ASCII roots are refused before launch | `agy` | `SECURITY.md:23-27`; `src/run.rs:550` | Real and admitted | Repeat, first panel | None |
| H27 | The seatbelt disables delegation from inside a run | `agy` | `SECURITY.md:149-154` | Real and admitted, map 64 | Repeat | Decided |
| H28 | The gate does not interpret shell | `agy` | `SECURITY.md:12-22`; `tests/gate.rs:1536-1600` | Real, admitted and pinned | Repeat | None |
| H29 | The documents drifted until the panels repaired them | `agy` | `docs/reviews/2026-09-22-*.md` | Partly. Too vague to act on; the live drift is H7 to H13, H44, H45 and H48 | Repeat | None of its own |
| H30 | The README does not mention the `codex` fail-open | `agy` | `README.md` | Partly. `SECURITY.md:155` states it; the first screen is a judgment call | New | Small |
| H31 | The comparison table's claims about other projects are unsourced | `cursor-agent` | `README.md:163-171` | Partly. Every row links the project and line 164 dates the reading; no cell carries a citation | Repeat, first panel | Small |
| H32 | Row A7 of the corpus README says "four names" | `cursor-agent` | `tests/corpus/README.md:16`; `src/env.rs:21` | Wrong in substance. `BASE` is four names plus per-provider additions, as the row says | New | None |
| H33 | `may_edit` is missing from contract 5.5 | `cursor-agent` | `docs/run-record-contract.md:378`, `398` | Wrong. Line 398 documents it | New | None |
| H34 | `take_model_flag` sits in a 185-line tail block | `cursor-agent` | `src/run.rs:1977-1985` | Wrong. It is nine lines, and after `phase_spawn` the longest function is 80 | New | None |
| H35 | Seatbelt probes skip outside macOS CI | `cursor-agent` | `tests/corpus/mod.rs:584-600` | Partly. The skip prints why and CI panics without `sandbox-exec`. The missed half: probes also skip inside a seatbelt, so none runs during rung two's admission | New | Small to state; making admission run them needs a decision |
| H36 | G42's tree half is "reasoned, not run", and `review_blocked`'s wiring is "covered by reading" | `claude` | `.scratch/hydra/build/89-*.md:73-74`; `96-*.md:61-63` | Real, the tickets say so | New | Large |
| H37 | Hard links into the corpus are not probed | `claude` | `src/sandbox.rs:76-103` | Unconfirmed. No probe exists and none was run | New | Small |
| H38 | Admission runs `cargo fmt`, `clippy`, `build` and `test` in the proposer's still-writable checkout, and nothing checks that the bytes they read are the tree the branch offers | `codex` | `src/rungs/evolve.rs:238-241`, `411-455`, `766-797`, `1254-1277`; `IMPLEMENTATION.md:3013-3024` | Real, by reading. `admission_gate(&patch.worktree, ...)` runs in the checkout after `read_proposer_patch` took the tree. The comment at `evolve.rs:1261` says it outright: "Admission and surviving proposer processes may have changed the checkout". The commit ignores those changes, so a surviving proposer process can put different bytes under the four checks than the branch carries. The documents do not overclaim: `IMPLEMENTATION.md:3015-3016` lists the size cap, the protected-path check and the judges, not admission. `codex` sourced the stronger claim to `README.md:55-58`, but it is the brief's wording. Re-reading the tree after admission would not close it, because a writer can swap back. H2 is the same gap from outside the checkout | Repeat, the half of G42 the fix left open | Large: admission on a fresh materialization of the read tree that the proposer's profile cannot write, with its processes stopped. Stating the residual is small |
| H39 | A failure inside `worktree::create` after `git worktree add` leaves a checkout and a `hydra/<run-id>` branch with no record, because the failure path starts only after `phase_worktree` returns | `codex` | `src/run.rs:480-486`, `747-756`; `src/worktree.rs:381-400`; `.scratch/hydra/build/92-*.md:21-23` | Real, by reading. `create` runs `git worktree add -b`, then `git_common_dir`, `CheckoutGit::new`, `fs::write` of the context file and `seed`, each with `?`. `run` calls `phase_worktree(&machine)?`, and `PostWorktree` is built on the next line. Ticket 92 asked for a record "from the moment the worktree exists". `tests/launch_phases.rs` fails a later step, the gate path. Only `reap::orphan_checkouts` finds what this leaves | Repeat, the half of G4 that ticket 92 did not reach | Large to record it through `PostWorktree::fail` with a test that fails the seed; rolling back inside `create` is small |
| H40 | A failure partway through skill injection leaves copies or links on disk and a failure record with no `skills` block, and reap then holds nothing back | `codex` | `src/skills/mod.rs:429-465`; `src/run.rs:1075-1085`; `src/reap.rs:329-338`, `429-438` | Real, by reading, narrow. `inject` writes each copy or link before it returns `linked`, and `recording.injected` is assigned only on success. `injected_skills` reads "no `skills` block" as "nothing was injected". Hydra's own `*` ignore hides the copies in the common case, and reap keeps it when it holds no names. So the leak needs a skill-root `.gitignore` that the seed copied from the human's repository and that does not ignore the injected names. "Reap never commits a skill Hydra injected" fails only then | New | Small: record the partial list on error, or have reap hold every root back for a `never_started` record |
| H41 | `--out Preference.jsonl` passes the guard on a case-insensitive volume, and the preference write then truncates the events file | `codex` | `src/export.rs:313-320`, `364-370`; `src/export_preference.rs:259-283` | Real. The guard compares the file name to `preference.jsonl` exactly. Measured: this Mac's data volume is case-insensitive APFS, and in the scratchpad writing `preference.jsonl` replaced `Preference.jsonl`. A symlink or hard link named anything is the same collision. The cost is a lost export file, not a leak, and I did not run `hydra export` | New as a row; round two noted the second write in its F4 recheck | Small: refuse when the two paths name one file (compare device and inode after creating the first), or create the second with `create_new` |
| H42 | Replay matches the gate's bytes, but the gate resolves paths with `realpath` against today's filesystem and inherits the reader's environment, so a changed symlink changes the answer under the same hash | `codex` | `src/policy/replay.rs:206-234`; `src/policy/gate.py:128-141`; `src/normalize/mod.rs:613-620`, `838-859` | Real, by reading, not measured. `run_gate` sets only `PWD`. `GateMark` and `cannot_back` downgrade a foreign gate or rebuilt scopes and nothing else. No document states this residual | New as a row; `codex` asked it as a question in round two | Small for the caveat in `IMPLEMENTATION.md`; large for evidence that survives it |
| H43 | The `codex` MCP writer builds TOML by `format!`, escaping quotes in `args` only and nothing in `command`, `env`, `url`, header values or the table name | `codex` | `src/provider/codex.rs:219-268` | Real, by reading. The values come from `config/mcp.md`, a protected config the human writes, so this is fragility, not an exploit. The `toml` crate is already a dependency, `Cargo.toml:22` | New | Small |
| H44 | The contract's `provider` enum lists `claude`, `cursor-agent`, `agy` or null, and omits `codex` | `codex` | `docs/run-record-contract.md:143`; `11-14` | Real. The preface at 11-14 says the document makes no claim about `codex`, and `src/record/validate.rs` checks no enum values, so no code breaks. A reader who builds a strict parser from the table rejects every `codex` record | Repeat, G11 named line 143 | Small |
| H45 | `SECURITY.md` says the export redacts five classes; there are six since the OpenAI class, and the module doc of `src/export.rs` says four | `codex` (the `SECURITY.md` half) | `SECURITY.md:176-177`; `src/export.rs:4`, `537-577` | Real. `redact_secrets` has five prefixes plus `openai_key_spans`. Line 4 of `export.rs` is my find | New, left over from the G17 fix | Small |
| H46 | The `codex` login-status probe inherits the launcher's whole environment, while every other probe clears it | `codex` | `src/provider/codex.rs:272-276`; `src/run.rs:967-975` | Real. `Command::new("codex")` with no `env_clear`, against `src/provider/mod.rs:77`, `claude.rs:463`, `cursor.rs:387`, `agy.rs:282` and `interactive.rs:258`, `376`. The probe runs the vendor binary outside the seatbelt, not the agent, so the exposure is small. A `CODEX_HOME` in the shell can make it answer about a different login than the run gets | Repeat of the G1 class, its fourth spawn path | Small |
| H47 | Launch discovery runs plain git in the directory the human launched from. From inside a run's checkout that git follows the checkout's `.git` file, which the run can rewrite, and `git worktree add` then runs a hook from a decoy outside the seatbelt | `codex` (as a scope on "every git call") | `src/run.rs:529`; `src/worktree.rs:159-169`, `381-395`, `574-593`; `SECURITY.md:67-74` | Real. Measured with plain git in the scratchpad, not through `hydra run`: a checkout whose `.git` points at a decoy with a `post-checkout` hook, then `git worktree add` from inside it, wrote the hook's marker. `repo_root`, `head_exists`, `git worktree add` and `seed`'s `git diff HEAD` all use the plain `Path` runner, which sets no `core.hooksPath` and no `--no-ext-diff`. Nothing refuses a launch from under `worktrees/`. It needs the human to launch from inside a run's checkout, or a run to launch Hydra itself where there is no seatbelt | New | Small: refuse a launch whose directory is under `worktrees/`, and put the `CheckoutGit` belt on the plain runner |
| H48 | The published first-panel report still marks F30 and F31 "not fixed", with no note that builds 81 and 83 fixed them | `codex` (strength 4 and before-publishing 7) | `docs/reviews/2026-09-22-seven-readers-as-a-hiring-panel.md:75-76` | Real. The report is dated, but its header says every row is fixed, held or stated, and nothing marks later fixes. `agy` read the same "check without a reservation" wording as current in both rounds, H23 and G30 | New | Small, a dated addendum |

Forty-eight findings. Real: H1 to H18, H20, H21, H25 to H28, H36 and H38 to H48, which is 36.
Partly: H19, H22, H24, H29, H30, H31 and H35, which is 7. Wrong: H23, H32, H33 and H34, which
is 4. Unconfirmed: H37.

By cost, over the 43 that are real or partly: 24 small (H4 to H13, H16, H17, H19, H22, H30, H31,
H35, H40, H41, H43 to H48), 10 large (H2, H3, H14, H15, H18, H36, H38, H39, H42, and H1's code
half once map 74 decides), 5 decided or needing a decision (H1, H20, H21, H25, H27), and 4 that need
nothing (H24, H26, H28, H29). Where a row has a small half and a large half it is counted by what
closes it, and the small half is listed first under Before publishing. H1 counts once, under
decisions. Several large rows have a small half, a sentence in `SECURITY.md` or
`IMPLEMENTATION.md` that stops the documents saying more than the code: H1, H2, H3, H38, H39 and
H42.

By reader:

- `claude` raised or touched 24 findings. None is wrong, H19 is partly and H37 is unconfirmed. Its
  H2 and H3 held up when measured, and H4 was confirmed on a real record.
- `codex` raised 19: eleven new (H38 to H48) and eight shared (H1, H5, H8, H9, H10, H14, H15,
  H24). None is wrong. Its one slip is a citation: it sourced the "checks and judges read"
  sentence to `README.md:55-58`, and that sentence is the brief's.
- `cursor-agent` raised 15. Three are wrong (H32, H33, H34), and it put `phase_handoff` at 150 when
  it is 149.
- `agy` raised 8. One is wrong (H23), for the second panel in a row. The other seven are admitted
  limits, two of them partly (H29, H30).

Across the two rounds the pattern holds. The readers who opened the Rust were never wrong on a
finding. The two who read by grep, or not at all, were wrong on six of 21 in round two and four
of 23 here.

## The second panel, rechecked

Ten G findings were marked fixed by build tickets 88 to 96 and map tickets 72 to 74. Here is what
each reader said about each one, and what I found.

G1, the pty inherited the shell. All four say fixed, and `claude` adds a caveat. Checked:
`src/pty.rs:120` clears, and `tests/pty_tee.rs` fails without it. The caveats are real, and they
are not G1. The watcher that owns the pty still holds the whole environment (H3, measured). `codex`
found a fourth spawn path with the same mistake, the `codex` login probe (H46). Round two recorded
the pattern: each fix went to the spawn path that was named. It happened again. No inventory of
`Command::new` sites exists, and one would have caught both.

G41, reap on a human branch and the private `HEAD`. `claude` and `agy` say fixed; `cursor-agent`
and `codex` say fixed with a caveat. Checked: `src/reap.rs:403-416` refuses a foreign branch
before it stages, and `src/sandbox.rs:118-128` denies `HEAD`, `commondir` and `gitdir`. The
caveats are the object store (H1, map 74, open and stated) and the global git config (H5). Both
are real, and neither is papered over: `SECURITY.md:75-82` states both. What it understates is
where substitution reaches.

The Opus reader's finding, git outside the seatbelt in a run-writable checkout. `cursor-agent`
and `agy` say fixed; `claude` and `codex` say fixed with a caveat. Checked: every call through
`CheckoutGit` pins three directories and turns hooks and fsmonitor off. The caveat both named is
global config and filters, H5. The fix also missed one path. Launch discovery and `git worktree
add` use the plain runner, and I made a decoy hook fire through them in a scratch repository
(H47). So `SECURITY.md:68`'s "every git call Hydra makes in a run's checkout" is true of reap, the
diff stat and rung two. It is not true of a launch the human starts from inside a checkout.

G42, the offered branch carried what admission wrote. `cursor-agent` and `agy` say fixed;
`claude` and `codex` say fixed with a caveat, and their caveats differ. Checked: the branch is
`commit-tree` of the read tree on the read parent, `src/rungs/evolve.rs:1254-1281`, and that
holds. `claude`'s caveat is the object store under the sha (H1). `codex`'s is that the four
admission steps run on the writable checkout and no check ties what they read to the tree offered
(H38). Both are real. The closure language is accurate: build 89 is titled "the panel judges the
tree it offers", and `IMPLEMENTATION.md:3015` lists what reads the tree without claiming
admission. The residual is undocumented, and the repository admits it in one code comment.

G43, admission inherited the shell. `agy` and `codex` say fixed; `claude` and `cursor-agent` say
fixed with a caveat. Checked: `apply_admission_env` clears and fills both branches,
`src/rungs/evolve.rs:809-816`. The caveat is H2, and it is worse than either reader said. `HOME`
reaches a `~/.cargo` the proposer can write, and a `runner` there turned a failing `cargo test`
into exit 0 when measured.

G44, editable probes. `agy` says fixed and the other three say fixed with a caveat. Checked:
`existing_test_hit_in` refuses baseline files under `tests/`. The caveat `claude` and
`cursor-agent` give, probes inside a named `src/` file (H20), is stated in map 73. `codex`'s
caveat is that the floor guards the patch's paths and not what admission code writes. That is
H38 and H2 again.

G4, early returns after the worktree. `claude` and `cursor-agent` say fixed; `codex` says fixed
with a caveat. `codex` is right. Handoff and spawn errors go through `PostWorktree::fail`, and
`tests/launch_phases.rs` fails the gate step. Errors inside `worktree::create` after `git worktree
add` still return before that path exists (H39). Ticket 92 asked for "from the moment the worktree
exists", and the refactor drew the line one function late.

G3, the 737-line `run`. `claude` says fixed with a caveat and `cursor-agent` docks a point for it.
Checked: `run()` is 26 lines and `refuse` takes a struct, but `phase_spawn` is 163 lines against
the ticket's 150 (H6). `04740db` corrected the Outcome after the panel, so the ticket is honest now
and the code is unchanged.

G2, G6, G19 and G23. The readers who checked them say fixed, and they are. G2 is the unit test
through `inject_skills` at `src/run.rs:2090`; `agy` put it in `tests/skills.rs`, the wrong file.
G6 is the `pruned/` read denial at `src/sandbox.rs:100-103`. G19's catch-all is driven at
`tests/gate.rs:2401`. G23's `skills::admit` is called from `curate::propose`, reverted to fail per
build 91.

G17, the OpenAI key class. `agy` says fixed and `codex` says fixed with a caveat. The class is
there, and redactor and verifier share its matcher. That is G18, admitted and unchanged. The fix
left two counts stale (H45).

In sum, none of the ten was papered over. Every "fixed" in the tickets holds for what the finding
named. Every caveat that survived checking is either a limit `SECURITY.md` already states (H1, H20)
or a neighbour of the fix that the ticket did not name (H2, H3, H38, H39, H46, H47). Round two
found the same shape: a fix goes to the named call site, and the next reader finds the same class
one call over.

## Interview question bank

Forty questions came in, ten from each reader. Deduplicated, twenty remain. One was dropped:
`agy`'s eighth asks why the cap has no lock, and it has had one since build 81 (H23). Each strong
answer below was checked against the code at `2e03429`. Where a reader's premise or suggested
answer was wrong, the correction is in the answer.

The top five are 1, 2, 3, 4 and 5. The rule is the same as in round two: the question a reviewer
who reads the code will hit first, and the answer that costs the most if the candidate has not
thought about it. Four of the five are about findings open at `04740db`. The fifth is about how
the candidate checks what agents claim, the thing every reader raised.

### Top five

1. Top five. "Rung two commits the tree sha you read before admission. What does a human get
   when they check out `rungs/<pass>`, and what did admission actually test?" Merged from
   `claude`'s first and sixth, `codex`'s first and `cursor-agent`'s fifth. Motivated by
   `src/rungs/evolve.rs:238-241`, `766-797` and `1254-1281`. A strong answer takes it in three
   layers. First, `commit-tree` binds the branch to the read tree and read parent, which stops
   admission edits landing on the branch. That is what build 89 fixed, and `tests/evolve.rs`
   moves `HEAD` to prove it. Second, the checks ran in the writable checkout, and the comment at
   `evolve.rs:1261` concedes a surviving proposer process can change it, so nothing proves the
   offered bytes passed (H38). Third, the proposer can write `~/.cargo/config.toml`, whose
   `runner` makes a failing `cargo test` exit 0 and then runs on the human's own builds (H2,
   measured). The fix is admission on a fresh materialization the proposer's profile cannot write,
   with its processes stopped, a scratch `CARGO_HOME` and an absolute toolchain. It should also
   say why re-hashing the checkout afterwards is not enough. Worrying: "git hashes are
   immutable", "the judges would catch it" (they never see a file outside the diff), or
   `commit-tree` offered as the whole answer.

2. Top five. "`.git/objects` is writable from inside a run. Walk me from a forged pack to what the
   next run and the next rung-two offer see." Merged from `claude`'s first, `cursor-agent`'s third
   and `codex`'s third. Motivated by `src/policy/mod.rs:640-646`, `SECURITY.md:75-82` and
   `.scratch/hydra/issues/74-*.md`. A strong answer separates content addressing, which names
   objects, from the filesystem, which lets a writer plant a pack mapping an existing sha to
   other bytes. Map 74 measured that git serves the forged bytes when that pack is the only copy.
   On this machine the parent repository is Hydra's own checkout, so a substituted blob reaches
   every later `git worktree add` seed and rung two's offer, not only reap (H1). It compares map
   74's options: a per-run `GIT_OBJECT_DIRECTORY` with the parent as an alternate and an import
   from outside, creation-only writes, which leave the forge open, or verifying every reachable
   blob before `commit-tree`. It says what each costs `git add`. Worrying: "creation-only fixes
   it", or "content addressing prevents that".

3. Top five. "G1 cleared the child's environment. Where else in the process tree is
   `GEMINI_API_KEY`, and how would you find every launch path with the same mistake?" Merged from
   `claude`'s fifth, `cursor-agent`'s sixth and `codex`'s tenth. Motivated by
   `src/watch.rs:103-134`, `src/provider/codex.rs:272-276` and `SECURITY.md:5-7`. A strong answer
   names the watcher, a same-uid process outside the seatbelt, and knows `ps -E` reads a same-uid
   process's environment on macOS (measured). It names the `codex` login probe, which still
   inherits while every other probe clears (H46). Then the method: list every `Command::new` and
   `CommandBuilder`, give each an explicit environment contract, and add a test that reads the
   watcher's own environment. It should say that `SECURITY.md`'s first sentence is not true under
   `standard` even with all of that done, because a run can `cat ~/.zshrc` and read the human's
   terminals. Worrying: "the allowlist handles that", or one more unit test of `ChildEnv::build`.

4. Top five. "A `codex` run runs out of quota. What does its record derive, and which of your
   outcomes would you let a training pipeline use as a label?" Merged from `claude`'s second,
   `cursor-agent`'s first and `agy`'s third. Motivated by `src/normalize/codex.rs:177-186` and
   `src/outcome.rs:74-79`, `122-285`. A strong answer traces `turn.failed` being dropped with no
   `Finding::Limit`, the outcome coming out `failed`, and `rung_one_learns(Failed)` being true.
   Record `01a0cfdb-e5c3` is a live case. It proposes reading the frame, mapping the usage
   sentence to `rate_limited`, and a limit for any frame type the adapter drops, as unknown item
   types already get. On labels it walks `derive` in order and rejects `completed` as success,
   since it means no failure signal arrived. It treats `gated`, `auth_failed`, `rate_limited` and
   the rest as harness facts, not task facts, and weighs `outcome_source`. Worrying: "`failed` is
   close enough", or `completed` taken as the task succeeding.

5. Top five. "Ticket 92 said no function in `run.rs` was over 150 lines, and `phase_spawn` is 163.
   Who checked that, and pick one agent diff you rejected: what was wrong, and could you rebuild
   why without the ticket?" Merged from `claude`'s third, `codex`'s seventh, `cursor-agent`'s
   eighth and `agy`'s tenth. Motivated by `.scratch/hydra/build/92-*.md:36` and the Outcomes of
   89, 90 and 92. A strong answer admits the miss, and that the panel caught it and `04740db`
   corrected the Outcome. It proposes a mechanical check for a mechanical claim: clippy's
   `too_many_lines` with a threshold in `clippy.toml`, run in CI. Then something specific: on 89,
   the parent read from the checkout's `HEAD` after admission; on 90, an agent test that drove a
   hand-built command rather than the function; on 92, an agent test that called the failure
   writer directly and passed with the wiring removed. It separates what the candidate judged from
   what the Opus reviewer judged. Worrying: "the Opus review covers it", or a ticket narrative
   with no way into the code.

### Design decisions

6. "Walk me through a launch that fails after the first skill is copied, and one that fails after
   `git worktree add` but before the seed. Who owns the checkout and the record?" From `codex`'s
   second and ninth. Motivated by `src/worktree.rs:381-400`, `src/run.rs:480-486`, `1075-1085` and
   `src/skills/mod.rs:437-465`. A strong answer finds H39: the failure path is built after
   `phase_worktree` returns, so an error in `create` leaves a branch and checkout with no record.
   It finds H40: `inject` writes before it returns `linked`, so the failure record carries no
   names, and reap reads that as nothing injected. It explains why the `*` ignore usually saves it
   and when it does not. It proposes an incremental manifest or rollback, and a test through
   `run::run` for each. It also knows why `codex` gets copies rather than links: it reads skills
   only by shell, and the gate refuses any path under `~/hydra`. Worrying: "reap's orphan sweep
   cleans it up".

7. "Replay recompiles today's gate and checks the hash. Which inputs can change its answer after a
   run ends, and what would you store instead?" Merged from `claude`'s fourth, `cursor-agent`'s
   seventh and `codex`'s fifth. Motivated by `src/policy/replay.rs:1-21`, `206-234` and
   `src/policy/gate.py:128-141`. A strong answer says why the real Python gate runs rather than a
   Rust copy: a copy drifts and reports leaks that never happened. It gives the cost: a process
   per record, one per call only when the batch fails. Then the inputs the hash does not cover:
   `realpath` against today's disk, so a changed or removed symlink changes the answer (H42), and
   the reader's environment and interpreter. It proposes recording the resolved path with each
   live decision and downgrading a disagreement whose path is gone, the way `GateMark` already
   downgrades a foreign gate. Worrying: "same bytes, so replay is a pure function".

8. "Show me the cap's linearization point, and does your threads-in-one-process test prove the
   cross-process property?" Merged from `claude`'s tenth and `codex`'s eighth; `agy`'s eighth
   rests on H23 and is replaced by this. Motivated by `src/live.rs:322-402` and
   `tests/status.rs:501-545`. A strong answer names the rename under the `flock` as the admission
   point. It knows `flock` binds an open file description, so threads that each open the file
   contend as processes would. It notes that staleness by pid plus start time is never exercised
   across real processes, and that interactive launches count only from line 0. Worrying: "the
   rename is atomic, so the whole launch is", or not knowing `flock` from `fcntl` locks.

9. "SBPL takes the last matching rule. How do you know the next edit to `compose` does not reopen
   a sibling's worktree?" From `claude`'s ninth. Motivated by `src/sandbox.rs:76-170`. A strong
   answer points at the profile-text test and the live probes in `tests/spawn.rs`, admits the
   ordering is held by comments, and proposes a test that checks every denied path against every
   allow for the final decision. Worrying: not knowing the ordering matters.

10. "What would it cost to make the types prove the four judges are independent, and what does
    `ApprovedPatch` prove today?" From `agy`'s fifth. Motivated by `src/rungs/review.rs:12-27`,
    `54-73`. A strong answer: the private constructor and one slot per provider in `PANEL_ORDER`
    prove four labels and a tree sha. Which binary wrote each answer is procedural, set in
    `run_judge_panel`, and no type can prove it. Four judges reading one prompt are correlated,
    whatever the labels say. Worrying: a type-state design offered as if it proved independence.

11. "Why a UUIDv7 run id, and why does `src/env.rs` drop a `HOME` that disagrees with the passwd
    entry?" From `agy`'s second and sixth, short questions for the opening minutes. Motivated by
    `src/run.rs:6` and `src/env.rs:99-104`. Strong: time-ordered ids sort a corpus walk and
    reservation files without a separate index; `security list-keychains` returns only the system
    keychain under a wrong `HOME`, which logs the vendor CLI out, and an absent `HOME` falls back
    to `getpwuid`. Worrying: "arbitrary" for either.

### Security model and its limits

12. "Show me a command that passes the gate and fails the seatbelt, one that fails both, and what
    happens in the team when someone closes a pinned hole." Merged from `cursor-agent`'s second
    and `agy`'s fourth. Motivated by `tests/gate.rs:1536-1600` and `SECURITY.md:12-43`. Strong:
    an assembled or quote-split path walks past the raw pass and the kernel refuses the write on
    macOS; a canonical spelling fails both. The pinned test is there so whoever closes the hole
    must rewrite it and `SECURITY.md` together. Worrying: "the gate closes shell interpretation".

13. "On `codex` a hook that dies is a call that runs. Why `--dangerously-bypass-hook-trust`, and
    how would you detect a crashed gate after the fact?" Merged from `cursor-agent`'s fourth,
    `claude`'s seventh and `agy`'s ninth. Motivated by `src/provider/codex.rs:20-40` and
    `src/policy/gate.py:956-977`. Strong: the per-run hook cannot carry a trusted hash, so trust is
    bypassed and the sha check inside the hook answers a tamper with a deny. The catch-all turns
    raises into denies, but an interpreter that dies before replying fails open, measured by hand
    in build 48. Detection: the gate appends each answer to a log outside the worktree, and a
    reader joins it to the stream so a call with no gate line becomes a finding. Worrying: "the
    catch-all handles it", or "the same as `claude`".

14. "What did pinning `GIT_DIR` buy, and what still executes outside the sandbox?" Merged from
    `codex`'s third and `agy`'s seventh. Motivated by `src/worktree.rs:127-142`, `159-169` and
    `SECURITY.md:67-82`. Strong: pointer substitution is closed for reap, the diff stat and rung
    two. `~/.gitconfig` filters and a user-owned system config still run (H5), and two
    environment variables plus `-c user.name` close them. Launch discovery uses plain git, so a
    human who launches from inside a checkout runs a decoy's hooks (H47). Worrying: "hooks are
    off, so nothing runs".

15. "Why an allow-default seatbelt, what can a process inside it still ask `launchd` to do, and
    would you deploy this for mutually untrusted customers?" Merged from `agy`'s first and
    `codex`'s fourth. Motivated by `src/sandbox.rs:78-160` and `SECURITY.md:28-59`, `92-98`.
    Strong: the profile is defence behind the gate for one person's machine. It binds the process
    tree and not what `launchd` starts for it, credentials sit in the keychain and `HOME`, and the
    network is open. For a service the boundary becomes a VM or container per run, with scoped
    injected credentials and egress through a logging proxy, and the answer names what breaks
    (keychain and OAuth logins). Worrying: "the seatbelt is a container".

16. "You chose to state the lethal trifecta rather than build a domain list. Defend it, then tell
    me what you ship when a customer says no exfiltration." From `claude`'s eighth. Motivated by
    `SECURITY.md:92-98`. Strong: the seatbelt filters by address, not host name, so a list binds
    the fetch tools and not `curl`. The real answer is a network namespace or an egress proxy with
    the vendor API allowlisted. Worrying: the stated exposure treated as the end of the talk.

17. "Why is `HYDRA_RUN_ID` not containment, and what did map 62 choose instead?" From
    `cursor-agent`'s ninth. Motivated by `src/containment.rs:12-21` and `SECURITY.md:139-148`.
    Strong: a child can unset the marker and is then a human to the check. What stops a wider
    child writing a record is the seatbelt's corpus deny, and on macOS nested delegation is
    refused in Hydra's words. Linux has neither. Worrying: "the marker is a wall".

### Evaluation and trajectories

18. "Ten passes produced one two-word patch. How would you show four judges improve safety enough
    to pay for them?" From `codex`'s sixth. Motivated by `README.md:55-62` and
    `src/rungs/review.rs:59-77`, `517-544`. Strong: a labelled patch set with seeded defects,
    false positive and false negative rates per judge, correlation between judges, human
    adjudication, a cost baseline, and held-out material the proposer cannot edit. Worrying:
    rejection rate or unanimity called an evaluation.

19. "A `--may-edit src/normalize/` pass could weaken a gate probe living in that directory. Is that
    acceptable?" From `cursor-agent`'s tenth. Motivated by `.scratch/hydra/issues/73-*.md:62-64`.
    Strong: the human names the risk when they pass the flag, baseline tests under `tests/` are
    refused, and A12's probe in `src/face.rs` is the live example. Closing it takes a new map
    ticket. Worrying: "the corpus directory protects every probe".

### Working with agents

20. Question 5 is this theme's strongest. The rest of what came in merged into it. `agy`'s tenth
    asked whether anything was merged unread. A strong answer to that is the audit trail in the
    build Outcomes, not an assurance.

## Before publishing

The union of the four lists plus the fixes the findings imply, ranked by how much each would hurt
if a reviewer found it first. Each is small, large or needs a decision. Items I dropped are at the
end.

1. Small, then large: `SECURITY.md:5-7`. The first sentence of the security document says no shell
   credential reaches the agent, and a same-uid `ps -E` reads the watcher's environment (measured).
   Say the allowlist bounds the child's inherited environment, and that under `standard` secrets
   on disk and in same-uid processes are reachable. Then start the watcher from `env_clear` in
   `src/watch.rs:103-134` with a test that reads its environment. H3.
2. Small, then large: rung two's admission integrity, `src/rungs/evolve.rs:238-241`, `766-797`.
   First say in `IMPLEMENTATION.md`'s rung-two section and `SECURITY.md` that admission runs on the
   writable checkout and reads a writable `~/.cargo`. Then run admission on a fresh materialization
   of the read tree, with a scratch `CARGO_HOME`, an absolute toolchain path and the proposer's
   processes stopped. The `runner` forge is measured, and a reviewer who knows cargo will look for
   it. H2, H38.
3. Needs a decision, with a small sentence first: map ticket 74, the object store,
   `src/policy/mod.rs:640-646`. Until it closes, `SECURITY.md:75-79` should say substitution
   reaches rung two's offered branch and every later worktree seed, not only reap. H1.
4. Small: git outside the seatbelt. Set `GIT_CONFIG_GLOBAL=/dev/null` and `GIT_CONFIG_NOSYSTEM=1`
   in `CheckoutGit::command` and pass the identity with `-c`, `src/worktree.rs:127-142`. Refuse a
   launch whose directory is under `worktrees/`, and give the plain runner the same belt,
   `src/worktree.rs:159-169`, `574-593`. Rewrite `SECURITY.md:67-82`. The decoy hook is
   reproduced. H5, H47.
5. Small: `src/normalize/codex.rs:177-186`. Read `turn.failed`, map the usage sentence to
   `rate_limited`, and emit a limit for any dropped frame type. Rung one is learning from a quota
   failure today. H4.
6. Large, with a small alternative: `src/worktree.rs:381-400` and `src/run.rs:480-486`. Put
   `create`'s post-`worktree add` steps under the failure path, or roll back inside `create`, and
   record partial skill injection, `src/skills/mod.rs:429-465`. Ticket 92's own "from the moment
   the worktree exists" is the sentence a reviewer will quote. H39, H40.
7. Small: counts and dates. The authorship table to 96, `README.md:222-245`; 73 of 74 decision
   tickets, `README.md:255`; 24 test files, `README.md:210` and `CLAUDE.md:102`; the second
   panel's report into `docs/reviews/` so the G ids resolve; a dated addendum on the first panel's
   F30 and F31 rows. H7, H8, H9, H13, H48.
8. Small: stale paragraphs. `IMPLEMENTATION.md:2054-2056` on the exit status; `codex` in the
   provider enum at `docs/run-record-contract.md:143`; "three providers" at
   `docs/harness-spec.md:819`; a `codex` row in the version table at `IMPLEMENTATION.md:86-92`;
   six redaction classes at `SECURITY.md:176-177` and `src/export.rs:4`. H10, H11, H12, H44, H45.
9. Small: `src/provider/codex.rs:272-276`, clear and fill the login probe's environment, the fourth
   spawn path of the G1 class. Do an inventory of every `Command::new` while there. H46.
10. Small: `src/export.rs:313-320`, refuse an `--out` that names the same file as its preference
    sibling, by identity rather than spelling. H41.
11. Small: `src/provider/codex.rs:219-268`, serialize the MCP fragment with the `toml` crate
    already in `Cargo.toml`, with round-trip tests for quotes, backslashes and newlines. H43.
12. Small: `src/rungs/evolve.rs:1540-1571`, move the canary to an integration test that sets the
    variable on the child, or fix the SAFETY comment and pass `--test-threads=1` in admission. H16.
13. Small: `src/run.rs:1182-1344`, split `phase_spawn` under 150, or leave it and let the
    corrected Outcome stand. H6.
14. Small: a caveat in `IMPLEMENTATION.md`'s replay section that `realpath` reads today's disk and
    the replay inherits the reader's environment. H42.
15. Small: `CheckoutGit::for_run`'s unused `_layout`, `src/worktree.rs:72-77`; one sentence in the
    rung-two section on ignored files, H19; a sentence on seatbelt probes skipping inside a
    seatbelt, and so during admission, H35; one hard-link probe into the corpus, H37. H17.
16. Small, optional: a line in the README pointing at the `codex` fail-open, H30, and a citation per
    cell in the comparison table, `README.md:163-171`, H31.
17. Large: a sanitized fixture corpus so CI's record walks read something, and an automated
    `codex` dead-hook probe. H14, H15.
18. Large: a test with a live layout for G42's tree half and `review_blocked`'s wiring, H36; a typed
    view of line 0 in place of `Value` paths, `src/reap.rs:329-351`, H18.
19. Large: evidence for replay that survives changed symlinks before disagreements become labels.
    H42.
20. Decided, keep stated: the trifecta, `~/.ssh` under `standard`, delegation under the seatbelt,
    editable probes in named files. H21, H25, H27, H20.

Dropped: `agy`'s "add a reservation lock to the cap" and "align the README with the lock", done in
build 81 (H23); `cursor-agent`'s "add `may_edit` to contract 5.5", already at line 398 (H33), and
"A7 still says four names", which is right (H32).

## Method

Read in full: the four third-panel reviews, `verified-three.md`, the brief `task-base.txt`,
`RUNS.md`, `launch-codex-later.sh`, `run-*.sh`, and the second panel's consolidated report for its
format and its G rows. H1 to H37 and their measurements are carried over from `verified-three.md`,
checked at `2e03429` by a session that edited nothing. Its measurements were: the cargo `runner`
forge in a scratch crate under `env -i`; `ps -Eww` and `sysctl(KERN_PROCARGS2)` reading a same-uid
environment on macOS 26.2; `hydra show 01a0cfdb-e5c3` deriving `failed`; function lengths in
`src/run.rs`; and the lock in `live::reserve`.

For H38 to H48 I read, at `04740db`: `src/run.rs:390-495`, `747-810`, `960-980`, `1060-1100`,
`515-535`; `src/worktree.rs:111-142`, `150-175`, `340-420`, `520-640`; `src/skills/mod.rs:420-500`;
`src/reap.rs:300-355`, `425-442`; `src/rungs/evolve.rs:170-290`, `405-460`, `755-860`,
`1250-1285`; `src/export.rs:300-375`, `537-580`, `680-700` and line 4;
`src/export_preference.rs:250-290`; `src/policy/replay.rs:200-240`; `src/policy/gate.py:125-145`;
`src/normalize/mod.rs:605-625`, `835-862`; `src/provider/codex.rs:210-290`; `src/env.rs` by grep;
`docs/run-record-contract.md:8-16`, `132-148`; `IMPLEMENTATION.md:2048-2060`, `3005-3035`;
`SECURITY.md:64-76`, `172-186`; `README.md:50-64`;
`docs/reviews/2026-09-22-seven-readers-as-a-hiring-panel.md:1-12`, `70-82`; build tickets 89, 91
and 92 in the parts cited. I grepped for every `env_clear`, every `login_status` caller, every
`admit(` caller, the validator's provider handling, and `IMPLEMENTATION.md` for any stated replay
residual. I ran `hydra show` on the four reader runs for outcome and tokens, read line 0 of the
`codex` run for its `base_tree`, and compared that tree's ticket 92 blob with `2e03429` and
`04740db`.

Two things were measured by me, both in the scratchpad and neither through Hydra. For H41: this
Mac's data volume is case-insensitive APFS, and writing `preference.jsonl` replaced the contents of
`Preference.jsonl`. For H47: in a scratch repository with a linked worktree, I pointed the
worktree's `.git` file at a decoy repository whose `post-checkout` hook touches a marker, then ran
`git worktree add -b child ... HEAD` from inside it under `env -i`. The marker appeared. That is
the command `worktree::create` runs, with the same plain runner, but I did not run `hydra run` from
inside a checkout.

Not done: I did not run the suite, build, or launch any provider or `hydra run`. H38, H39, H40, H42
and H43 are confirmed by reading both sides of the code, not by making them happen. The tests their
fixes need are those reproductions: a surviving proposer process that rewrites the checkout during
admission, a seed that fails after `git worktree add`, an injection that fails after one copy under
a non-covering ignore, a replay after a symlink changes, and an MCP value with a backslash. I did
not reread the rows H1 to H37 beyond the lines H38 to H48 share with them. Citation and word counts
in the verdict table come from `wc` and a grep over each review and are approximate. A note for the
next panel: the seed copied the untracked `pendintes_hydra.md` into the `codex` reader's worktree.
That is working as designed, and a private notes file left in the checkout reaches the reviewer.
