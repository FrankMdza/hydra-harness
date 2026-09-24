# Four readers as a hiring panel, 2026-09-23

Reviewed commits `15d0d7a` and `d264867`, and `fad1b98` for the `codex` reader, published on 2026-09-24 as written then.
Build tickets 88 to 96 and 103 under `.scratch/hydra/build/` record what was fixed; the third panel's report rechecks each G row.

The first three reviews were checked against `d264867`, one commit after `15d0d7a`. The `claude`
reader reviewed `d264867`; the `cursor-agent` and `agy` readers reviewed `15d0d7a`. The one commit
between them is the fix for the build 87 regression, G2 below, and changes nothing else a finding
rests on.

The `codex` reader reported last, and it read `fad1b98`, not `d264867`. Its second attempt started
after build ticket 88 landed, the worktree was seeded from that commit, and its method section says
so: it found `env_clear` in `src/pty.rs` and the build 88 paragraph in `SECURITY.md`, and it does not
claim G1. Its findings G41 to G49 are checked against `fad1b98`.

G1 is fixed in `fad1b98` (build ticket 88). `src/pty.rs` now clears the environment before the
allowlist, `tests/pty_tee.rs::the_child_gets_the_environment_it_is_handed_and_nothing_else` fails
without it, three providers answered a real launch after it, and `SECURITY.md` states the exposure
with its dates. G10 and G38 moved with it. The same class is still open on one path, rung two's
admission steps, G43.

## Verdicts

| Reader | Model | Verdict | Level | The reason, in one sentence | Depth |
|---|---|---|---|---|---|
| `claude` | Opus | advance to technical interview | mid-level | The core idea is in the code, but the first security claim in the README is false: the environment allowlist is built and then handed to a pty builder that starts from the watcher's whole environment. | 3,959 words, about 215 file:line citations, read the Rust in large part and the locked `portable-pty` source |
| `cursor-agent` | Composer 2.5 | advance to technical interview | mid-level, senior only if the candidate can explain `src/policy/replay.rs:9-17` unprompted | Outcome-from-stream, per-vendor hook dialects and pinned residuals are implemented rather than asserted; the agent authorship table is the main interview risk. | 2,542 words, about 135 file:line citations, read code for the modules it named, did not open `src/pty.rs` or `src/watch.rs` |
| `agy` | Gemini 3.1 Pro | advance to technical interview | senior | A contract-first harness that derives outcomes from the tool stream shows systems thinking and boundary design. | 1,530 words, 39 citations, all to documents, a single line each; its own method says it did not read the Rust |
| `codex` | GPT, via the Codex CLI | advance to technical interview | mid-level, not senior ownership of agent isolation | Replay that downgrades a finding when gate identity or scopes cannot back it treats trajectories as evidence with limits, but the git write carve-outs and the gap between the reviewed tree and the committed one break two safety claims. | 3,502 words, about 200 file:line citations, read `src/run.rs`, `gate.py`, `sandbox.rs`, `env.rs` and `resume.rs` in full and traced the rest; reviewed `fad1b98`, 7.4M tokens on its second attempt |

The `claude` and `codex` reviews carry the panel between them, and they score within one point of
each other on every axis. `claude` found G1, the defect behind the README's first security claim,
by reading the dependency's source rather than trusting the comment above the call. `codex` found
the two defects still open at `fad1b98`, G41 and G42, each of which breaks a sentence the README or
`SECURITY.md` states without a caveat, plus G43, the G1 class surviving on the admission path. None
of the eighteen findings `codex` raised was wrong when checked, and on G26 it corrected `claude`.
If one review has to be read first now, it is `codex`'s, because its findings are the open ones.
Every finding `claude` raised checked out against the code except one caveat about the reservation
race test, G27, which is partly right, and the interactive-cap claim, G26, where `codex` is right.
The `cursor-agent` review is careful
where it looked, but it gave security 9 and listed "environment scrubbing is tested" as strength 5
on the strength of unit tests that check the list and not the child, which is exactly the gap G1
lives in. Three of its smaller findings were wrong when checked, and so was half of a fourth. The `agy` review cites documents
only, never opens a source file, and three of its seven weaknesses are wrong: it read the history
line that says the cap was a check "closed by build ticket 81" as saying the cap is still a check,
and it repeated a first-panel README finding that was fixed a day earlier. It gave the highest
scores on the least evidence, the same pattern the first panel recorded for it, and it should be
weighted the same way.

## Scores

| Axis | `claude` | `cursor-agent` | `agy` | `codex` | Mean | Spread |
|---|---|---|---|---|---|---|
| Rust engineering quality | 6 | 8 | 8 | 6 | 7.0 | 2 |
| Test discipline | 6 | 8 | 8 | 7 | 7.3 | 2 |
| Security and threat-model honesty | 5 | 9 | 9 | 5 | 7.0 | 4 |
| Documentation and communication | 5 | 8 | 9 | 6 | 7.0 | 4 |
| Depth and originality of the harness idea | 8 | 9 | 9 | 8 | 8.5 | 1 |
| Engineering process | 7 | 7 | 10 | 7 | 7.8 | 3 |

The spreads did not change, because `codex` landed inside the range on every axis. What changed is
the shape: on security the panel is now two against two, and the split is exactly who read the
launch and git paths. `claude` scored 5 for G1. `codex` scored 5 at `fad1b98`, with G1 already
fixed, for G41 and the cwd sentence, G48. The two readers who did not open `src/pty.rs` or
`src/policy/mod.rs`'s write carve-outs scored 9. On documentation the gap is still volume and
drift: `claude` counted it in eight places, `codex` in five more, and `agy` did not look. The
`claude` and `codex` columns agree within a point on every axis, and they are the ones to plan
against.

## Findings

Every weakness, red flag and "does not hold" claim across the four reviews, deduplicated. The
check column is mine, made by opening the cited lines at `d264867` for G1 to G40 and at `fad1b98`
for G41 to G49 and for the rows the G1 fix moved. Status is real, partly, wrong
or unconfirmed. Cost is small, under an hour and no decision needed; large; or needs a decision.
A wrong finding costs nothing and says so.

| Id | What | Raised by | File:line | Kind | Status and my check | Cost |
|---|---|---|---|---|---|---|
| G1 | The environment allowlist is not a boundary on recorded launches: the pty child inherits the watcher's whole environment | `claude` | `src/pty.rs:79-82`, `109-117`; `src/watch.rs:103-133`; `portable-pty-0.9.0/src/cmdbuilder.rs:74-86`, `215-224`, `528-536` | bug | Real. `CommandBuilder::new` sets `envs: get_base_env()`, which copies `std::env::vars_os()`; `as_command` clears the std `Command` and then sets that inherited map. `open` never calls `cmd.env_clear()`, and `detach` starts the watcher with no scrub. Every probe path does call `env_clear`, `src/provider/mod.rs:77`, `claude.rs:463`, `cursor.rs:387`, `agy.rs:282`, `interactive.rs:258`, `375`. The comment at `src/env.rs:114-116`, "contract 5.3 records what reached the child", is false for the same reason. Mechanism confirmed by reading; not observed at a live child. Fixed in `fad1b98`: `cmd.env_clear()` at `src/pty.rs:120`, and `tests/pty_tee.rs` plants names the child must not see | large, done in `fad1b98` |
| G2 | Build 87 refused every `codex` launch with CI green, and the regression test does not go through the call site | `claude` | `src/run.rs:1195`, `1225-1227`; `tests/skills.rs:483-524`; `.scratch/hydra/build/87-codex-reads-a-copy-of-its-skills.md:60-65` | bug, fixed; test gap remains | Real. The test calls `Index::project_scope_without` and `collision::check` directly. `grep` finds `inject_skills` called only from `src/run.rs:637` and `src/interactive.rs:108`, never from a test | small |
| G3 | `run::run` is one 737-line function with three copied `never_started` blocks, and `refuse` takes eleven positional arguments | `claude` | `src/run.rs:382-1118`, `856-872`, `915-931`, `1026-1037`, `1262-1276` | design smell | Real. The function closes at line 1118; `#[allow(clippy::too_many_arguments)]` sits at 1262 over eleven parameters; `refuse(` appears seven times | large |
| G4 | Early `?` returns after the worktree exists write no record, against the file's own D10 comment | `claude` | `src/run.rs:586-594`, `619-625`, `637-646`, `829-839`, `987-993`; `src/reap.rs:675` | bug | Real. `worktree::create` runs at 586; `mcp::write_config(...)?`, `inject_skills(...)?` and `write_spawn_config(...)?` then return with no `never_started`. Only `reap::orphan_checkouts` finds the leftovers | large |
| G5 | Rung two's protected set is a denylist that still misses the files that carry G1 and the root refusal | `claude` | `src/rungs/evolve.rs:448-465` | design smell | Real. The list holds `review.rs`, `evolve.rs`, `spawn_config.rs`, `policy`, `record`, `env.rs`, `sandbox.rs`, `containment.rs`, `tests/corpus`, and not `src/pty.rs`, `src/watch.rs`, `src/paths.rs` or `src/run.rs`. Its own comment calls `sandbox.rs` "the third time a mechanism landed outside this list" | needs a decision: spec 15.2 owns the rows, and allowlist versus denylist is a map question |
| G6 | The seatbelt read-denies `runs/` and `spawn/` only; `pruned/` and sibling worktrees are readable | `claude` | `src/sandbox.rs:86-105`; `src/policy/mod.rs:580-582`; `src/paths.rs:254-256`; `SECURITY.md:27-39` | scope limit, partly admitted | Real. `compose` emits `deny file-read*` for the corpus and `spawns()`, nothing for `root/pruned`, which is a sibling of `runs/`, not inside it. `pruned/` is on the write list only. `SECURITY.md` says "reads of the corpus", and a pruned record is the corpus moved | small, either the deny line or the sentence |
| G7 | Ticket counts are stale in two places | `claude`, `codex` | `README.md:246`; `CLAUDE.md:142` | documentation gap | Real. The README says 71 decision tickets, 68 resolved; `.scratch/hydra/issues/` holds 71 files, all `Status: resolved`. `CLAUDE.md:142` still comments the frontier command "67 of 67" while line 28 says 71 of 71 | small |
| G8 | `IMPLEMENTATION.md` says the cap is a reservation, then eleven lines later that it is "checked and not held" | `cursor-agent`, `codex` | `IMPLEMENTATION.md:3370-3380`, `3386-3391` | documentation gap | Real. Both paragraphs are present in that order | small |
| G9 | `IMPLEMENTATION.md` still prices replay at a Python process per tool call | `claude` | `IMPLEMENTATION.md:2165-2171`; `src/policy/replay.rs:19-21` | documentation gap | Real. The module doc now says a process per record, one per call only when the batch fails | small |
| G10 | `IMPLEMENTATION.md` says `HYDRA_ROOT` never reaches a child and that a run can move its children's records | `claude` | `IMPLEMENTATION.md:3735-3744`; `src/run.rs:411-424` | documentation gap | Partly since `fad1b98`. At `d264867` both halves were stale. Since the pty clears the environment, "never reaches a child" is true again for recorded runs; the half about moving children's records is still stale, because `run.rs` refuses the variable inside a seatbelt | small |
| G11 | "three providers" survives in code comments and the contract | `claude` | `src/outcome.rs:13`, `456`; `src/policy/gate.py:16`, `364`, `816`; `IMPLEMENTATION.md:51`; `docs/run-record-contract.md:8`, `143`, `686`, `705`, `1112` | documentation gap | Real. `src/skills/mod.rs:414` is a historical statement about ticket 24 and is fine as written; the rest describe today | small |
| G12 | `tests/corpus/README.md` cites `gate.py` lines that have drifted, and its environment rows cite the wrong test | `claude`, `codex` | `tests/corpus/README.md:13-20` | documentation gap | Real. `gate.py:124` is `fold_ascii`, not `realpath`, which is at 140; `753` is inside `tool_input` handling; `tests/gate.rs:1766` is an `agy` MCP capability test, not the shell residual. `codex` adds that rows A7 and A8 still cite only the `src/env.rs` builder tests, and since `fad1b98` the test that watches the child is in `tests/pty_tee.rs` | small |
| G13 | The authorship table stops at ticket 76, and "applied it to master by hand" sits beside tickets whose outcome says an Opus subagent fixed the diff | `claude` | `README.md:216-236`; `.scratch/hydra/build/81-the-cap-is-a-reservation.md` outcome; `87-...md:57-58` | documentation gap | Partly. The table says outright it covers 01 to 76, so nothing is hidden, but builds 77 to 87 include `codex` on 81 and `cursor-agent` on 87, and both outcomes record a subagent fixing the agent's diff in the worktree before the session applied it | small |
| G14 | Agent authorship dominates the Rust core, so the interview has to separate design ownership from review | `cursor-agent` | `README.md:218-236` | admitted scope limit | Partly. The arithmetic holds, 41 of 76, and the table is a disclosure rather than a defect. It is an interview risk, not a repository flaw | small, interview preparation |
| G15 | `outcome.rs` says nothing reads the exit status, and the harvested interactive arm does | `claude` | `src/outcome.rs:13`, `185-205` | documentation gap | Real. `end.body["exit_code"]` picks `Completed` on 0 and `Failed` otherwise when a harvested run has no result frame | small |
| G16 | Provider identity is a string where outcomes are derived, and an unknown refusal code becomes `Gated` | `claude` | `src/outcome.rs:223`, `231`; `src/record/lines.rs:290-302` | design smell | Partly. The string comparisons are real although `Provider` parses the same field in `src/reap.rs:308-310`. The `_ => Outcome::Gated` arm is a documented choice, its comment argues that falling through to `completed` would be worse | small |
| G17 | Export has no OpenAI key class although `codex` is routed | `claude` | `src/export.rs:495-501`, `638-642` | bug | Real. The five classes are Google, Anthropic, GitHub, Google OAuth and PEM; an OpenAI-style key reaches the entropy warning at best | small |
| G18 | The export verifier checks the redactor's own prefixes, so an abort means only that the redactor missed; commit `6daad9c` calls these "rules of its own" | `claude`, `cursor-agent`, `codex` | `src/export.rs:629-642`; `SECURITY.md:140-147` | admitted scope limit | Real. The comment at 636-637 says "The prefixes are the redactor's own". `SECURITY.md` states it plainly; the commit subject oversells it | large |
| G19 | The gate's catch-all, the F2 fix, has no test that drives it | `claude`, `cursor-agent` | `src/policy/gate.py:961-975` | admitted scope limit | Real. The comment says no reachable raise was found, "so this handler has no test that drives it" | small, a test that compiles a gate whose `evaluate` raises |
| G20 | A `codex` hook that dies fails open, measured by hand, with no automated test; a missing interpreter is still open | `cursor-agent`, `claude`, `codex` | `SECURITY.md:119-134`; `src/provider/codex.rs:37-40`; `IMPLEMENTATION.md:3765-3771` | admitted scope limit | Real. Probe p3 of build 48 is the only evidence | large |
| G21 | CI walks an empty corpus, so the three record walks prove nothing there | `cursor-agent`, `claude`, `codex` | `tests/record_contract.rs:576-584`; `README.md:206-212` | admitted scope limit | Real. Two early `return`s before any assertion; the README says what green CI means | large, a synthetic fixture corpus |
| G22 | The gate is 1,092 lines of Python outside the type system, with an `except Exception` catch-all | `cursor-agent` | `src/policy/gate.py`; `src/policy/replay.rs:9-17` | design smell | Partly. The size is real. The catch-all is the fail-closed path the first panel asked for, and `replay.rs` argues a single implementation on purpose | needs a decision |
| G23 | `skills::admit`, the write-time collision check, has no caller | `cursor-agent` | `src/skills/mod.rs:475-503` | admitted scope limit | Real. The doc comment says "a gap rather than a decision"; `tests/skills.rs:797-809` unit-tests it directly | needs a decision, rung one's write path |
| G24 | The seatbelt is macOS only and Linux launches, including rung two's admission steps, run bare | `cursor-agent`, `agy` | `src/sandbox.rs:18-27`; `SECURITY.md:38-39`; `IMPLEMENTATION.md:3040-3045`; `README.md:154-156` | admitted scope limit | Partly. The limit is real and stated in three places. `cursor-agent`'s claim that the comparison table at `README.md:163-169` leans on the kernel is wrong: its columns are vendor count, worktree, record and compiled gate, none of them a seatbelt property | large |
| G25 | The seatbelt spelling probe passes silently when `sandbox-exec` is absent | `claude`, `cursor-agent`, `codex` | `tests/spawn.rs:1332-1334` | test gap | Real. A bare `return` on `!exec_available()`. CI runs macOS, so it binds there | small |
| G26 | Interactive launches reserve nothing and are not counted against the cap; `codex` says they are counted | `claude`; contradicted by `codex` | `IMPLEMENTATION.md:3379-3380`; ticket 81 outcome, lines 39-40; `src/live.rs:371-375`; `src/interactive.rs:270-309` | documentation gap | Partly. They reserve nothing, which is true. They are counted: an interactive launch writes line 0 with the child's pid right after it spawns, and `live::reserve` counts every live view that `scan` returns. The only uncounted window is between the spawn and line 0. So the sentence in `IMPLEMENTATION.md` and ticket 81 is wrong, and `codex` is right | small, the sentence |
| G27 | The cap race test runs threads of one process | `claude` | `tests/status.rs:500-522` | test gap | Partly. Each thread opens the lock file itself, and `flock` binds an open file description, so the threads contend as processes would. A second process would add only the launcher-liveness path | small |
| G28 | A command `shlex` cannot split skips the token pass and leans on the floor, which `--allow-floor-escape` turns off; `SECURITY.md` does not say so | `claude` | `src/policy/gate.py:316-321`, `862`, `928` | documentation gap | Real. `split_segments` raising `ValueError` returns `None` from the token pass | small |
| G29 | "A child is never wider than its parent" binds children Hydra launches; a `standard` run can exec a vendor CLI itself with no gate | `claude`, `codex` | `SECURITY.md:96-112`; `src/run.rs:430-532` | admitted scope limit | Real, and spec 7.5 says it | needs a decision |
| G30 | The concurrency cap is a check without a reservation lock | `agy` | `IMPLEMENTATION.md:3640`; `src/live.rs:322-403` | claimed design smell | Wrong. Line 3640 reads "the concurrency cap was a check without a reservation (closed by build ticket 81)". `live::reserve` holds an `flock` across count and rename, `src/run.rs:447` calls it | none |
| G31 | The export verifier disables entropy checks, leaving keys exportable | `agy` | `SECURITY.md:140-147`; `src/export.rs:644-660` | claimed open exposure | Partly. Nothing is disabled: the entropy pass runs and warns rather than aborts, because aborting stopped a third of the records on digests. The exposure it leaves is stated | none beyond G17 and G18 |
| G32 | Interactive runs compile no gate and the documents underplay it | `agy` | `docs/run-record-contract.md:428-436`; `IMPLEMENTATION.md:242-243`, `2699-2715`; `SECURITY.md:38` | documentation gap | Partly. An ungated `--interactive` run compiles no gate by design, and the contract says so at the line `agy` cited. A gated interactive `claude` run exists since build 28 and runs under the seatbelt since 73. `SECURITY.md` names only the gated arm, so one sentence is missing there | small |
| G33 | The 16 MiB tee line cap still risks memory spikes | `agy` | `IMPLEMENTATION.md:2104-2108`; `pty::LINE_CAP` | claimed design smell | Wrong. The cap is the bound: at 16 MiB the tee writes what it holds as a frame and keeps reading | none |
| G34 | Nothing pins what an MCP server advertises after admission | `agy` | `SECURITY.md:73-75`; `tests/corpus/README.md` row A5 | admitted scope limit | Real, stated as open | large |
| G35 | The README says `gated` needs one refused call | `agy` | `README.md:21-23` | documentation gap | Wrong. The README says every call; this was first-panel F19, fixed before either commit reviewed here | none |
| G36 | `evolve` calls `Layout::discover()`, coupling rung two to the developer's machine | `cursor-agent` | `src/rungs/evolve.rs:82` | design smell | Wrong. That is the command's entry point, where every command discovers its layout; the admission steps get a scratch `HYDRA_ROOT`, `IMPLEMENTATION.md:3040-3044` | none |
| G37 | The README's "86 of 87" contradicts open ticket 80 | `cursor-agent` | `README.md:206-214`; `CLAUDE.md:28` | documentation gap | Wrong. 86 of 87 with 80 open is consistent, and `frontier.py build` prints exactly that | none |
| G38 | The env claim holds only with a caveat, because `CURSOR_DATA_DIR` is set at launch and not in the base constants | `cursor-agent` | `src/env.rs:110-118`; `README.md:34-37` | documentation gap | Wrong as a caveat. The README says "the two names Hydra sets itself", which are the containment marker and `CURSOR_DATA_DIR`, `src/run.rs:543`, `582`. At `d264867` the claim failed for G1's reason instead; since `fad1b98` it holds, and `codex` checked it there, with the fair note that allowlisting proxy variables does not inspect their contents | none |
| G39 | External contributors are not told they are the third, ungated population | `cursor-agent` | `CLAUDE.md:178`; `CONTRIBUTING.md:46` | documentation gap | Partly. `CLAUDE.md:178` onward says it at length; `CONTRIBUTING.md` points at `CLAUDE.md` for prose conventions only | small |
| G40 | The first panel's closing count, "27 of the 33 rows were code with a test", is generous | `claude` | `docs/reviews/2026-09-22-seven-readers-as-a-hiring-panel.md:81-82` | documentation gap | Real. F19 to F21 and F23 to F25 are marked fixed and are documentation edits | small |
| G41 | The git carve-outs let a run rewrite its checkout's `HEAD` and delete or replace files in the shared object store, and reap commits on whatever branch that `HEAD` names before it checks the name, so "your branch never moves" does not hold | `codex` | `src/policy/mod.rs:625-642`; `src/sandbox.rs:106-112`; `src/reap.rs:374-423`, `478-483`; `SECURITY.md:52-56`; `README.md:25-27`; `tests/spawn.rs:1445-1544` | bug | Real, by reading, not reproduced. `own_write_paths` carves out `.git/worktrees/<run-id>/` whole, which holds that checkout's own `HEAD`, and `.git/objects/`, and `compose` turns both into `allow file-write*`, which covers unlink and overwrite, not only creation. The gate applies the same carve-out. `SECURITY.md` says `HEAD` stays denied; the test, `the_seatbelt_allows_git_add_in_the_worktrees_private_git_directory`, checks only the main `.git/HEAD`. What a hostile `standard` run could do: point its checkout at one of the human's branches, which needs no path spelled and no ref written; then `hydra reap`, running outside any seatbelt, reads that branch name, commits the run's whole tree on top of it, and only afterwards says it is "not the branch Hydra made". The human's branch moves, and a commit whose tree is the run's can silently undo work that landed on that branch after the run started. Separately, the run can delete pack files or loose objects in the human's repository, which content addressing does not prevent. The comment at `reap.rs:414-418` treats committing on a human branch as intended, so the code and the README disagree on purpose | needs a decision, then large: which promise wins is a decision; denying the private `HEAD` and refusing to commit off `hydra/<run-id>` are small; narrowing the object store to creation needs a profile measured under a real `git add` |
| G42 | Rung two checks and judges one diff and commits another: the patch is read before the admission steps run, and the offered branch is the worktree as it stands after them | `codex` | `src/rungs/evolve.rs:139-187`, `226-274`, `968-1007`; `src/rungs/review.rs:12-21` | bug | Real, by reading. `read_proposer_patch` diffs the worktree once; the size cap and `protected_path_hit` read that diff; `admission_gate` then runs `cargo build` and `cargo test` in the same worktree under the proposer's profile, which lets them write there; the judges get the diff read before; `commit_pass_branch` runs `commit_everything` on the worktree as it is now. `ApprovedPatch` holds a pass id, not a tree id. What a hostile proposer could do: ship a build step or test that edits files during admission, a protected path such as `src/policy/` included, after both checks ran. The offerable branch then carries changes no check and no judge saw, under a commit message carrying the panel's passing verdict. A process the proposer left running could do the same. The human merge is the backstop, and it holds only if the human reads the branch's diff rather than the judged one | large: the check is a few lines, take a tree id when the diff is read and refuse a different one before the commit; the test needs an admission run with a build step that edits the tree |
| G43 | Rung two's admission steps inherit the orchestrator's whole environment, the G1 class on the one path build 88 did not touch | `codex` | `src/rungs/evolve.rs:563-599`; `SECURITY.md:40-50` | bug | Real. Both branches of `run_admission_step`, seatbelt and bare, build a `Command` with only `.env("HYDRA_ROOT", ...)` and no `env_clear`. So a proposer's test runs with the human's shell environment, API keys and agent sockets included, under a profile that leaves the network open and home readable. `SECURITY.md` admits the `~/.ssh` read and says nothing about the environment | large: `env_clear` plus what cargo needs is short; finding what cargo needs, and a test that plants a variable, are not |
| G44 | The protected attack corpus is the rig and the index; the probes it indexes live in `tests/gate.rs` and `tests/spawn.rs`, which a proposer may edit | `codex` | `src/rungs/evolve.rs:448-465`; `tests/corpus/mod.rs:1-8`; `tests/corpus/README.md:8-23`; `IMPLEMENTATION.md:3671-3681` | design smell | Real. `tests/corpus/` holds `mod.rs` and `README.md` only. Eight of the fourteen rows name probes in `tests/gate.rs`, `tests/spawn.rs`, `tests/mcp.rs` or `tests/delegation.rs`, and A12's is a unit test in `src/face.rs`; none of those files is on `PROTECTED_CODE_PATHS`. The probes inside `src/env.rs` and `src/containment.rs` are protected only because their files are. The comment at 462-463, "a proposer must not delete or rewrite the probes", is not what the list does | needs a decision, with G5 and build ticket 80 |
| G45 | "A run whose tools failed is `failed`" is not the rule the code implements | `codex` | `README.md:21-24`; `src/outcome.rs:184-205`, `247-267` | documentation gap | Real. With a terminator the outcome reads `is_error` or `agy`'s `status` and nothing about individual tool results. A harvested run with no terminator reads the exit code, and one with no exit code reads `completed`. The README sentence says more than the code; G15 is the comment-level version of the same drift | small, the sentence |
| G46 | The README's roadmap still says `codex` resume is refused | `codex` | `README.md:185-190`; `src/provider/mod.rs:192-194`; `src/provider/codex.rs:103-122`; `tests/resume.rs:158` | documentation gap | Real. `supports_resume` returns true for every provider, `codex::resume_args` builds `exec resume`, a test pins it, and build ticket 71's outcome says the refusal came off | small |
| G47 | `IMPLEMENTATION.md`'s rung-two paragraph names three judges and puts the hook-config writer in `src/run.rs` | `codex` | `IMPLEMENTATION.md:3037-3042`, `3057-3058`; `src/spawn_config.rs:51`; `src/rungs/evolve.rs:452-458` | documentation gap | Real. The panel is four judges since `codex` joined it in build 61. `write_spawn_config` lives in `src/spawn_config.rs`, which is on the protected list, so the sentence saying that gap "is on the record here rather than closed" describes a gap that closed | small |
| G48 | `SECURITY.md` says a payload whose `cwd` is not a string is denied; the gate falls back to the worktree and the test asserts the call is allowed | `codex` | `SECURITY.md:136-139`; `src/policy/gate.py:759-761`; `tests/gate.rs:2332-2348` | documentation gap | Real. The crash fix is real. The sentence lists the wrong answer for one of its four shapes, in the file that is supposed to be the exact one | small |
| G49 | Ten rung-two passes and one two-word patch show the pipeline ran, not that it improves the harness | `codex` | `README.md:210-213` | admitted scope limit | Real, and the README says it in those words. No held-out task set or regression metric exists, and the judges and tests a pass is measured against are partly editable, G44 | large |

Forty-nine findings. Real: G1 to G9, G11, G12, G15, G17 to G21, G23, G25, G28, G29, G34, G40 and
G41 to G49, which is 32. Partly: G10, G13, G14, G16, G22, G24, G26, G27, G31, G32 and G39, which is
11. Wrong: G30, G33, G35, G36, G37 and G38, which is 6. Unconfirmed: none, with the note that G1,
G41 and G42 are confirmed by reading and not by observing them happen. By cost, over the 43 that are
real or partly: 25 small, 11 large, 6 need a decision, and G31 needs nothing beyond G17 and G18.
G1 is done in `fad1b98`, and G2's code is fixed with its test gap still open.

By reader: all 25 findings `claude` raised are real or partly real; the two that moved to partly,
G10 and G26, moved because of `fad1b98` and because of `codex`. Of the 14 `cursor-agent` raised, 11
are real or partly and 3 are wrong, plus the comparison-table half of G24. Of the 7 `agy` raised, 4
are real or partly and 3 are wrong. Of the 18 `codex` raised, nine new and nine shared, all 18 are
real or partly real, and on G26 its half is the right one.

## The first panel, rechecked

The 2026-09-22 report marks F30 and F31 "stated, not fixed". Both were fixed afterwards, by build
tickets 81 and 83, and `claude`, `cursor-agent` and `codex` agree they hold, each with a caveat:
interactive launches reserve nothing, and `refuse` still takes eleven arguments. `codex` adds that
the obsolete "checked and not held" paragraph survives, G8, and that interactive runs are counted
after all once line 0 is written, G26. `agy` says F30 is still open, and that is its misreading,
G30.

Where the readers disagree with a "fixed" status:

F11 was marked fixed because `Provider::version` got the child's allowlist. `claude` points out
that the same class is open on the main launch path, one file over, G1. The fix went to the probe
and the pty path next to it kept inheriting. That made F11 fixed as written and the finding behind
it not fixed, until `fad1b98`. `codex` then found the class a third time: F14 is marked fixed
because admission runs under the seatbelt, and the admission steps still inherit the whole
environment, G43. The pattern is the finding. Each fix went to the spawn path that was named, and
nothing audited the other spawn paths for the same mistake.

F16 was marked fixed by carving the private git directory and the object store out of the denial
so `git add` works. `codex` says it was papered over, and it is right: the carve-out that fixed
staging is what lets a run rewrite its checkout's `HEAD` and delete objects, and reap then commits
on whatever branch that `HEAD` names, G41. With G1 fixed, this is the most important open
disagreement across both panels, because it breaks a sentence in the README's first screen.

F12 was marked fixed by adding `sandbox.rs` and `containment.rs` to the protected set. `claude`
calls it fixed for those two files and shows the pattern recurring with `src/pty.rs`,
`src/watch.rs`, `src/paths.rs` and `src/run.rs`, G5.

F2 was marked fixed. `claude`, `cursor-agent` and `codex` say fixed with a caveat. For the first
two it is that the catch-all that turns any raise into a deny has no test, which its own comment
admits, G19. For `codex` it is that the crash fix is real and `SECURITY.md` then says the malformed
`cwd` is denied, while the gate allows it and the regression test asserts that, G48.

F5 was marked fixed. `codex` agrees the argv is right and points out that the README's roadmap
still says `codex` resume is refused, G46.

F4 was marked fixed. `codex` agrees and notes that the two export files are written one after the
other with no transaction, so an I/O failure on the second leaves the first on disk. True by
reading, `src/export.rs:364-370`, small, and not a row, because verification now finishes before
either file exists, which was the finding.

F1 was marked fixed. `codex` says fixed with a caveat that `SECURITY.md` already states: the root
override is refused only inside the macOS seatbelt, and ancestry is a marker a run can remove. That
is G29's territory.

F29 was marked fixed by read-denying sibling spawn directories. Nobody disputed that, but `claude`
found the same door open under `pruned/`, which holds whole records, G6.

F33 was marked "stated, an independent verifier is not built". `claude` agrees it is not fixed and
adds that commit `6daad9c` describes the verifier as having "rules of its own", which overstates
it, G18. `codex` calls it fixed with a caveat: the scan decodes and walks keys and escaped strings
on its own, and still aborts only on the redactor's families. The two readers agree on the facts.
The scan is independent in what it reads and not in what it looks for, and "independent verifier"
means the second.

F32 stays documented and not fixed; the three readers who checked it agree, G21.

The report's own closing line, 27 of 33 rows "code with a test", counts six documentation edits as
code, G40.

`cursor-agent` checked F1, F6, F16, F19 and F29 and found all fixed at the lines it cited. For F16
that is true of what the finding asked, that `git add` works, and `codex` shows what the fix cost,
G41. `agy` marked eight findings fixed, F1 to F5, F13, F14 and F16, citing test
files with no lines. Its table adds no evidence beyond the first report's own statuses.

`codex` also names what the first panel missed: content addressing is not write protection, G41;
nothing binds the offered branch to the reviewed tree, G42; the seam between building the
environment and applying it, which is G1 and which `claude` found first; and the harvested
exit-status branch under the first panel's endorsement of stream-derived outcomes, G45.

## Interview question bank

Forty questions came in, ten from each reader. Deduplicated, twenty-three remain, grouped by
theme. Nine of `codex`'s ten merged into existing questions, and those questions say so; the tenth,
on git integrity, is new as question 23 and sits under the security theme, so the numbers there
skip. Each strong answer below is checked against the code, at `d264867` for the first three
readers and at `fad1b98` for what `codex` added and for question 9; where a reader's suggested answer was
wrong about the code, the correction is in the answer.

The top five, re-picked across all four readers, are 23, 6, 9, 18 and 22. The rule is still the
question a reviewer who reads the code will hit first, and the fourth reader moved it. Questions
23 and 6 are about defects that are open at `fad1b98`, G41 and G42. A reviewer who finds either
before the candidate has said it will not ask a friendly version. Question 9 stays although G1 is
fixed, because `SECURITY.md` now discloses it with dates, the disclosure invites the question, and
G43 is the same class still open. Question 18 was asked independently by the two readers who read
the most code, and it is the question closest to what a lab's harness team does all day. Question
22 is the one all four asked. Questions 1 and 10 leave the five: both have a strong answer written
down in the repository already, and neither is about something broken.

### Design decisions

1. "Why does `policy/replay.rs` run the Python gate again at read time instead of
   re-implementing the decision in Rust?" Raised by `cursor-agent`, prompted by
   `src/policy/replay.rs:9-21`. A strong answer: the gate's decision depends on the denied paths
   and carve-outs, scope resolution, the `mcp__` prefix rule and a `shlex`-based floor. A Rust copy
   would drift from the Python the first time either changed, and a drifted copy that answers
   `allow` where the gate said `deny` reports a gate leak that never happened. Running the real
   script means a disagreement can only mean the policy moved or the environment differs. The cost
   is a Python process per record, one per call when a batch fails, and `/usr/bin/python3` as a
   read-time dependency. The answer that would worry them: "for speed, I'd port it", with no
   mention of the false leak.

2. "Walk me through `derive` in order. What would you change if rung one needed runs that were
   mostly refused?" Raised by `cursor-agent`, prompted by `src/outcome.rs:120-286` and `423-441`.
   A strong answer walks the order: a Hydra refusal, then a provider refusal, then
   `refused_everything`, then the terminator. It says `refused_everything` needs at least one call,
   every call denied, zero gate leaks and at least one denial by Hydra's policy. It names the known
   weak spot, one allowed read plus four denied writes reads `completed`, `IMPLEMENTATION.md:2055-2060`,
   and says a threshold is a map ticket because `tests/outcome.rs` pins both directions on purpose.
   Worrying: "check the exit code" or "one deny is enough".

3. "Walk me through what happens when `write_spawn_config` fails at `src/run.rs:829-839`." Raised
   by `claude`. A strong answer notices that the worktree and branch already exist and no record
   will be written, which breaks the D10 rule the comment at `987-993` states, and that
   `mcp::write_config` at 619 and `inject_skills` at 637 have the same shape. It proposes a launch
   guard that writes `never_started` on drop, which also removes the three copied blocks at 856,
   915 and 1026. Worrying: "reap's orphan sweep cleans it up", treating `src/reap.rs:675` as the
   design.

4. "Show me the race the concurrency cap used to have, and what `live::reserve` does about it."
   Raised by `cursor-agent` and, framed as still open, by `agy`. Prompted by `src/live.rs:322-403`
   and `tests/status.rs:500-522`. A strong answer: two launchers could each count one free slot and
   both fork. Now an `flock` on `reservations/.lock` is held across reading pending reservations,
   scanning live runs and renaming `<run_id>.json` into place; the guard drops the file once line 0
   carries the child's pid, and a file whose launcher is dead, reused or unparseable is removed by
   the next reservation. The directory sits beside `spawn/`, not inside it, because reap treats
   every directory under `spawn/` as a run's and would have deleted the lock, which is what the
   Opus review caught on ticket 81. Interactive launches reserve nothing, and they are counted once
   line 0 is written, G26. No daemon and no database were added. Worrying: not knowing why the
   reservations moved out of `spawn/`, or proposing a database.

   `codex` asks the same question sharper: "Show me the linearization point of the cap, then walk
   through a launcher dying before and after line zero is published." Prompted by the same lines
   and `tests/status.rs:501-544`. A strong answer names the rename under the lock as the admission
   point. A launcher that dies before the rename published nothing. One that dies after the rename
   and before line 0 leaves a file the next reservation removes, because `procinfo::is_alive`
   compares pid and start time and so is not fooled by pid reuse; if the watcher was already forked,
   that run holds no counted slot until its line 0 lands. After line 0 the live scan counts the run
   and `view.launched()` marks the reservation stale. The candidate should say which of those
   interleavings a test drives: the threads in `tests/status.rs` contend on the lock, and the dying
   launcher is reasoned, not tested. Worrying: "the rename is atomic, so the whole launch is".

5. "How does `hydra reap` decide what not to commit from `.codex/skills`?" Raised by
   `cursor-agent`, prompted by `src/reap.rs:286-334`. A strong answer: reap reads line 0 of the
   record, from `runs/` or `pruned/`, takes the provider and `hydra.skills.linked`, and excludes
   only those names under that provider's skill root, plus Hydra's own `.gitignore`. Names that
   are not a single path component are dropped. With no record at all, all four roots are held
   back whole. A skill the run wrote beside the injected ones can still be committed. Ticket 87's
   first plan, putting whole roots on `own_worktree_paths`, would have discarded such a skill.
   Worrying: "reap ignores all skills".

   `codex` adds the first half: "Why copy skills for `codex` at all?" Prompted by
   `src/skills/mod.rs:445-464`, `517-559`. A strong answer: `codex` reads only by shell, and the
   shell check refuses any command naming a path under `~/hydra`, where the snapshot a link points
   into lives, so a link is a skill it cannot open. The copy has 0444 files and 0555 directories,
   the seatbelt denies writes to `.codex/skills` with a rule placed after the worktree allow because
   the profile takes the last match, links inside a skill are recreated rather than followed so a
   link out of the store does not become a copy of its target, and the watcher compares the copy
   with the snapshot at exit. File modes are advice to a process that owns the files; the kernel
   rule is the denial, and only on macOS. Worrying: following every link, or dropping the whole
   skill root at reap.

6. Top five. "Why is `ApprovedPatch` a private struct with one constructor, and what does the type
   actually prove?" Raised by `cursor-agent`, and by the first panel. `codex` asks the version that
   finds a defect: "Which exact tree did the judges approve, and how do you prove it is the tree
   offered to the human?" Prompted by `src/rungs/review.rs:54-73`
   and `505-530`. A strong answer: `ApprovedPatch` can only come from `panel_verdict` returning
   `Offerable`, and `PanelAnswers::new` fills one slot per provider in `PANEL_ORDER`, writing each
   provider's name into its outcome, so a panel always carries four distinct labels. The type
   cannot check which binary produced the text; `run_judge_panel` in `evolve.rs` fills each slot
   from a run of that provider. So independence is procedural, not proven by the type.

   Then the part `codex` is after, G42, prompted by `src/rungs/evolve.rs:139-274`, `968-1007`: the
   type carries a pass id, not a tree. The size cap, the protected-path check and the judges all
   read the diff taken before admission, and admission then runs `cargo build` and `cargo test` in
   the same worktree, which they can write. The offered branch is `commit_everything` on the
   worktree after all of that. A strong answer admits the gap, and binds every stage to one object
   id: take the tree id when the diff is read, run admission on a copy or re-check the tree after
   it, and refuse to create the branch when the id changed, with the id carried in
   `ApprovedPatch`. It also stops any process the proposer left running before the diff is taken.
   Worrying: "the constructor is private", "the tests guarantee it", or "the human reads it
   anyway" offered as the whole answer.

7. "Rung two protects files by listing them, and you have missed a mechanism three times. Why not
   protect everything and list what a proposer may touch?" Raised by `claude`, prompted by
   `src/rungs/evolve.rs:448-465`. A strong answer weighs an allowlist of editable paths against how
   useful the proposer stays, names the files still missing, `src/pty.rs`, `src/watch.rs`,
   `src/paths.rs` and `src/run.rs`, and ties it to build ticket 80 on baseline-test provenance.
   Worrying: "add `pty.rs` to the list and move on".

   `codex` asks where that leads: "What evaluation would convince you rung two improves the harness
   rather than learning to satisfy its own editable tests and judges?" Prompted by
   `src/rungs/evolve.rs:448-465`, `520-527`, `IMPLEMENTATION.md:3671-3681` and `README.md:210-213`.
   A strong answer starts from what is editable today: the attack probes live in `tests/gate.rs` and
   `tests/spawn.rs`, not in the protected `tests/corpus/`, G44. It proposes a baseline test runner
   the proposer cannot touch, held-out tasks, regression and cost measured per pass, failures a
   human adjudicated, and a note that four judges reading one prompt are correlated, not four
   independent votes. It says ten passes and one two-word patch show the pipeline ran, G49.
   Worrying: counting offerable patches, or treating four provider labels as four pieces of
   evidence.

8. "What happens when a provider ships a powerful new tool your capability table does not list?"
   Raised by `agy`, prompted by `docs/harness-spec.md:722-726`. A strong answer: the gate denies
   it with `hydra/unknown-tool`, `src/policy/gate.py:763-770`, and records it, so a self-update
   breaks the next run loudly instead of quietly widening a child. On `agy` the `init` frame lists
   tool names for free and Hydra compares them against the table. Worrying: not knowing the
   fallback.

### Security model and its limits

9. Top five. "Show me how you know a recorded `claude` child cannot see `GEMINI_API_KEY`." Raised
   by `claude`, prompted by `src/pty.rs:79-82`. `codex`, reading `fad1b98`, asks the follow-up:
   "The environment builder's tests were green while recorded children inherited the whole shell.
   Where would you put the regression test, and which launch paths remain different?" Prompted by
   `SECURITY.md:79-88`, `src/pty.rs:115-123`, `src/watch.rs:146-167` and
   `src/rungs/evolve.rs:584-599`. At `d264867` the true answer was that it could, G1. At `fad1b98` a
   strong answer tells it as a story with a date. `portable-pty`'s `CommandBuilder::new` copies the
   caller's environment, the watcher had inherited the launcher's, and `open` only added and
   overrode, so every recorded run before build 88 got the shell's whole environment while the
   README said the opposite. The fix is `cmd.env_clear()` before the loop, and the proof is observed
   at the child: `tests/pty_tee.rs::the_child_gets_the_environment_it_is_handed_and_nothing_else`
   runs `env` on the pty and fails if `HOME` or `CARGO_MANIFEST_DIR` arrive unasked. The candidate
   should say that `env_allowlist` on old records names what Hydra meant to pass, not what the child
   got, and that `SECURITY.md` says so. Then the audit `codex` asks for: the probes and the
   interactive launch already cleared; the watcher is harmless now because the pty clears; rung
   two's admission steps do not clear and pass the human's environment to a proposer's tests, G43;
   and the replay gate inherits the reader's environment, which is lower stakes because the script
   is Hydra's own. The honest part is that it was broken until a reviewer read the dependency's
   source, and that the same mistake survived on a second path. Worrying: pointing at the
   `src/env.rs` unit tests or at `run_start.env_allowlist`, both statements of intent, or adding
   one more test of `ChildEnv::build`.

10. "You pinned an open bypass as a passing test. Why not write a shell parser, and would
    you ever delete that test?" Raised by all three: `claude` on the parser, `agy` on pinning,
    `cursor-agent` on `a_protected_path_built_by_substitution_still_walks_past`. Prompted by
    `src/policy/gate.py:296-328` and `tests/gate.rs:1483-1510`. A strong answer: the gate reads a
    raw substring pass plus a `shlex` token pass, and refuses a protected path only when the
    command spells it. A parser still cannot evaluate expansions, so it narrows the residual without
    closing it; the kernel layer is what closes it, and on macOS the seatbelt refuses the write
    whatever the spelling. The test is there so that whoever closes the hole has to rewrite it and
    `SECURITY.md`. It should also name the path where `shlex` fails, which skips the token pass and
    leans on the floor, and `--allow-floor-escape`, G28. Worrying: "regular expressions would fix
    it", or "I'd delete it, it's embarrassing".

11. "Your profile is `(allow default)`. What would deny-by-default cost on each provider, and why is
    `pruned/` readable?" Raised by `claude`, prompted by `src/sandbox.rs:78-137`. A strong answer
    knows what each vendor reads, keychain, `~/.gemini`, Node and its caches, treats the missing
    `pruned/` read deny as a miss rather than a choice, and says the profile is defence behind the
    gate, which map ticket 67 settled. Worrying: defending `(allow default)` with nothing tighter
    ever tried.

    `codex` takes it to the job: "Why an allow-default seatbelt while allowing arbitrary shell, and
    what would have to change before this ran on a lab worker with credentials?" Prompted by
    `src/sandbox.rs:78-112` and `SECURITY.md:25-50`, `66-72`. A strong answer keeps the layers
    apart: the gate is lexical, the seatbelt binds one process tree and not what launchd starts for
    it, credentials sit in the keychain, in `HOME` and, on the admission path, in the environment,
    G43, and the network is open. On a lab worker the boundary is the machine: a VM or container per
    run, credentials scoped to that run and injected rather than inherited, and egress through a
    proxy that can log. It names the cost, that vendor CLIs log in through a keychain or an OAuth
    file and a fresh worker has neither. Worrying: "the hook contains the shell", or a domain list
    offered as egress control.

12. "On `codex`, what happens if the hook exits non-zero with nothing on stdout, and what would you
    build so it fails closed anyway?" Raised by `cursor-agent` and `claude`, prompted by
    `src/provider/codex.rs:37-40` and `SECURITY.md:119-134`. A strong answer: the call runs, measured
    by hand as probe p3 in build 48. That is why the sha check in `hook.py` answers a tamper with a
    deny in `claude`'s dialect instead of an exit code, and why `evaluate` raises are now caught and
    denied, `gate.py:961-975`, though untested. What would close it: a wrapper that prints a deny on
    every non-zero path, running Python isolated from site packages, a launch-time smoke call of
    the compiled gate, and the missing-interpreter case in `IMPLEMENTATION.md:3765-3771`. Worrying:
    "the gate always blocks", "same as `claude`", or "wait for the vendor".

13. "What can a `standard` run still do to `~/.ssh` that the gate does not stop?" Raised by
    `cursor-agent`, prompted by `SECURITY.md:40-50`. A strong answer: under `standard` the path
    scopes bind the file tools, but the shell check reads only the floor and the denied write paths,
    so a shell command reads and writes the rest of the machine, `~/.ssh` and the user's own
    `~/.claude` included. The seatbelt denies only Hydra's paths. The honest sentence is the one
    `SECURITY.md` ends on: the gate bounds what a run may do to Hydra, not to you. Worrying:
    "`standard` is safe" or "the seatbelt is a container".

14. "Why is the network left open, and what does that do to the threat model?" Raised by `agy`,
    prompted by `SECURITY.md:26-30` and `60-72`. A strong answer: the vendor CLI needs its model, and
    the seatbelt filters by address rather than host name, so a domain list would bind the fetch
    tools and not `curl`; map ticket 69 chose to state the exposure. Only `read-only`, `judge` and
    `curate` grant no network. The floor refuses Slack's posting methods and nothing else.
    Worrying: not knowing the network is open.

15. "The hook config for `cursor-agent` and `agy` sits inside the worktree the run can write. How
    is the gate not rewritten away?" Raised by `agy`, prompted by `IMPLEMENTATION.md:138-145`. A
    strong answer: since build 44 `protected_worktree_paths`, `src/policy/mod.rs:682`, denies writes
    to those files under every policy; the hook config carries a sha of the gate script and a
    rewritten or deleted script fails closed with `hydra/gate-tampered`; the gate itself sits in
    the spawn directory, which every policy denies writes to; and on `codex` the hook rides a `-c`
    override naming `spawn/<run-id>/hook.py`, because `codex` reads no project hook file inside a
    worktree. It should add that whether either provider reloads hooks mid-session is unmeasured.
    Worrying: forgetting the protection exists.

16. "Rung two runs a proposer's tests under a seatbelt on macOS and bare on Linux. How would you
    secure it on Linux?" Raised by `agy`, prompted by `IMPLEMENTATION.md:3040-3048`. A strong answer
    names namespaces, a read-only bind of `~/hydra`, seccomp or landlock for the write denials, and
    says that even the macOS profile bounds what the steps do to Hydra and not to the user: a
    hostile test can still read `~/.ssh`. Worrying: calling Linux out of scope with no plan.

17. "Export: aborting on entropy stopped a third of your records. How would you build redaction for
    unstructured trajectories?" Raised by `agy`, prompted by `SECURITY.md:140-147`. The strong answer
    corrects the premise first: the entropy pass is not disabled, it warns by field, class and
    length and the export proceeds. Then: the abort list is the redactor's own prefixes, so it only
    proves the redactor ran, G18; an independent verifier with its own rules, an OpenAI class since
    `codex` is routed, G17, and an allowlist of exported fields, which `hydra export` already
    enforces behind a sign-off manifest. Worrying: tuning the threshold and calling it done.

23. Top five. "Walk me from a writable git object directory and a writable private `HEAD` to your
    promise that the user's branch never moves. What invariant would you enforce instead?" Raised by
    `codex`, prompted by `src/policy/mod.rs:625-642` and `src/reap.rs:374-423`. New, G41. A strong
    answer admits the promise does not hold today and says why in three steps. The carve-out for
    `git add` covers the checkout's whole private git directory, which holds its `HEAD`, and the
    whole object store, as `file-write*`, which includes deleting and replacing files. A run can
    therefore point its checkout at a human branch without spelling any protected path. And
    `hydra reap`, outside the seatbelt, commits on whatever branch `HEAD` names and only then
    notices it is not `hydra/<run-id>`. It separates content addressing, which names objects,
    from the filesystem, which lets any writer unlink them. The invariant: reap commits only on the
    branch Hydra created and refuses otherwise, leaving the checkout for a human; the private
    `HEAD` is denied by a rule after the carve-out, since the profile takes the last match and
    `git add` never writes `HEAD`; the object store is narrowed toward creation as far as a
    measured `git add` allows; and a test in a throwaway repository repoints `HEAD` and checks
    that reap leaves the human's branch where it was. It should say the Linux path has only the gate,
    which carves out the same directories. Worrying: "git is content-addressed, so it is safe", or
    "refs are denied, so no branch moves", which forgets that reap writes the ref from outside.

### Evaluation and trajectories

18. Top five. "Which of your nine outcomes would you let an RL pipeline use as reward, and which
    would you drop?" Raised by `claude`, prompted by `src/outcome.rs:27-41`, `423-441` and
    `IMPLEMENTATION.md:2055-2060`. A strong answer rejects `completed` as success: it means the run
    ended without a failure signal, and the one-read-four-denied-writes case and a run blocked
    entirely by the user's own hooks both read `completed`. `gated`, `auth_failed`,
    `skill_collision`, `mcp_unavailable` and `rate_limited` are about the harness, not the task,
    and rung one never learns from refusals. It weighs `outcome_source` and says a task-level
    verifier is what the record lacks. Worrying: treating `completed` as the task succeeding.

    `codex` asks it as "What does `completed` mean, and which labels would you permit as success
    supervision?", prompted by `src/outcome.rs:184-267` and `src/normalize/mod.rs:825-860`. The
    answer should add two facts from the code. With a terminator, the outcome reads `is_error`, or
    `status` on `agy`, and never aggregates failed tool results, so the README's "a run whose tools
    failed is `failed`" says more than the code does, G45. A harvested run with no exit code reads
    `completed` because nobody watched it end. Four things need separate labels: the process
    finished, the task succeeded, the provider refused, and how far the evidence can be trusted.
    Then independent task checks and a held-out set. Worrying: a clean terminator or a panel pass
    taken as ground truth.

19. "`permission_decision` is re-derived at read time by today's gate. If I train on your corpus in
    March, how do I know the labels match September's?" Raised by `claude`, prompted by
    `src/policy/replay.rs:1-21` and `src/normalize/mod.rs:985-999`. A strong answer: the record
    carries `policy_sha` and the gate sha, and `policy_moved` already treats a record written under a
    different policy directory as a different question rather than a contradiction, so a
    disagreement is only a finding when both sources answered the same policy. The live answer
    survives only where the provider echoes it, `claude`'s hook frames; contract 7.7 calls that the
    one place two independent sources answer the same question. A training set should pin the
    labels at export with the sha they were derived under. Worrying: "the record is lossless, so
    it's fine".

    `codex` pushes past the sha: "Your replay matches the compiled gate hash. Which other inputs can
    change its answer after a run ends?" Prompted by `src/normalize/mod.rs:606-640` and
    `src/policy/replay.rs:205-234`. A strong answer names the filesystem first: the gate resolves
    paths with `realpath` against today's disk, so a symlink the run made, or a worktree reap
    already removed, resolves differently at replay. Then the interpreter, `/usr/bin/python3` on the
    reading machine, and the environment, since the replay process inherits the reader's and sets
    only `PWD`. It proposes recording the resolved path with each live decision where the provider
    allows, and downgrading a disagreement when the path no longer exists, the way `GateMark`
    already downgrades a foreign gate. Worrying: "same gate bytes, so replay is a pure function".

20. "Providers change their event formats. How does outcome derivation stay right?" Raised by
    `agy`, prompted by `IMPLEMENTATION.md:2014`. A strong answer corrects the premise that drift
    tests cover it: `tests/drift.rs` probes only `claude`'s MCP flags under `judge`. What catches
    format drift is an unknown tool name being denied and recorded, results the adapter cannot
    classify staying `unclassified` and never upgraded to `ok`, `outcome_source`, and `hydra doctor`
    printing the measured versions. Raw frames stay in the record so a normalizer fix can re-read
    old runs. Worrying: assuming vendor schemas are stable.

21. "What trajectory signal would you export for preference learning, and why keep raw frames at
    all?" Merged from `cursor-agent` and `agy`, prompted by `docs/run-record-contract.md:28-37`,
    `87-126` and `src/export.rs:283-330`. A strong answer: per-call `permission_decision`, the
    outcome with its source, corrections and judge verdicts in `preference.jsonl`, which has a
    writer since build ticket 65, behind the human's sign-off manifest; raw frames because
    normalization logic changes and derived fields are recomputed at read time, which is also why
    the record can be re-audited. Worrying: only the final assistant text or the exit code, or
    dropping raw frames to save space.

### Deliberate tradeoffs

The replay choice, question 1, the pinned residual, question 10, and `(allow default)`, question
11, are the three tradeoffs the first three readers asked about, and each is listed once above
under its closer theme. `codex` found a fourth that the code states and the README contradicts:
reap commits on a human branch that a run checked out, by design, `src/reap.rs:414-418`. It is
under question 23.

### Working with agents

22. Top five. "Pick ticket 81 or 87. Tell me one diff an agent produced that you rejected, why, and
    how you knew it was a decision for the map rather than a fix. And for 87, why did no test see
    the regression?" Merged from `claude`'s two questions, `cursor-agent`'s, `agy`'s and `codex`'s,
    which adds "what reasoning did you personally contribute?" Prompted by
    `README.md:216-236` and the outcomes of tickets 81 and 87. A strong answer is specific. On 81,
    `codex` put reservations under `spawn/`, the session's Opus reviewer saw that reap would delete
    the lock, and a subagent moved them to `reservations/`, pinned by
    `tests/cleanup.rs::a_sweep_leaves_pending_reservations_alone`; later the session found
    `seed_tree` copying reservations into worktrees, fixed in `e734038`. On 87, the ticket's own
    item 3 was wrong twice, and after two review rounds the first real `codex` launch still found
    every launch refused, because no test goes through `run::inject_skills`, G2. The candidate
    should say what they now ask a reviewing agent to check, and separate what they judged from
    what the Opus reviewer judged. The `CLAUDE.md` rule is that a decision reopens the map. `codex`
    warns about the other failure: a candidate who can retell the ticket but cannot open the file
    and walk the code path live. Worrying: "the agent ran the tests and I merged", "run a real
    launch after every push" with nothing automated, reading every diff given as the whole answer
    for 41 tickets, or a ticket narrative with no way into the code.

### Other

None of the forty questions fell outside the themes above.

## Before publishing

The union of the four lists plus the fixes the findings imply, ranked by how much each would hurt
if a reviewer found it first. Items the readers proposed that I dropped are at the end.

1. Done in `fad1b98`: G1. `src/pty.rs` clears the environment, `tests/pty_tee.rs` watches the
   child, and `SECURITY.md` states the exposure with its dates.
2. Needs a decision, then large, and first among what is open because it breaks a sentence on the
   README's first screen: G41. Decide whether reap may ever commit on a branch Hydra did not make.
   Then make reap refuse before committing when the branch is not `hydra/<run-id>`, deny the
   checkout's private `HEAD` after the carve-out in `src/sandbox.rs` and at the gate, narrow the
   object-store grant as far as a measured `git add` allows, and add a throwaway-repository test
   that repoints `HEAD` and checks the human's branch did not move. Until then, `README.md:25-27`
   and `SECURITY.md:52-56` say what is true. From `codex`.
3. Large: G42. Bind the size cap, the protected-path check, admission and the judges to one tree id
   and refuse to create the offered branch when the worktree no longer matches it, in
   `src/rungs/evolve.rs` and `ApprovedPatch`. From `codex`.
4. Large: G43. Clear the environment for rung two's admission steps and pass what cargo needs, with
   a test that plants a variable. `SECURITY.md:40-50` should say it either way. This is the same
   class as G1, disclosed a day ago, and the first place a reader of build 88 will look. From
   `codex`.
5. Small: read-deny `pruned/` in `src/sandbox.rs`, or say in `SECURITY.md:27-39` that it and sibling
   worktrees are readable. G6.
6. Small: a test through `run::inject_skills` for `codex`, in `tests/skills.rs` or by making the
   call reachable. G2.
7. Small: counts and tables. `README.md:246` to 71 of 71, from an audited count rather than another
   guess, `CLAUDE.md:142`, and the authorship table at `README.md:216-236` extended to 88 or its
   range restated, with a sentence on the subagent fixes in 81 and 87. G7, G13.
8. Small: the stale `IMPLEMENTATION.md` paragraphs: the replay price, the "checked and not held"
   cap paragraph, the `HYDRA_ROOT` half at 3737-3748, the rung-two paragraph's three judges and
   hook-writer location at 3037-3061, and the interactive-count sentence at 3380-3381 with ticket
   81's line 39-40. G8, G9, G10, G26, G47.
9. Small: README and `SECURITY.md` sentences. The outcome bullet at `README.md:21-24`, G45; the
   `codex` resume line in the roadmap, G46; the malformed-`cwd` sentence at `SECURITY.md:136-139`,
   G48; one sentence for the unparseable-command path and `--allow-floor-escape`, G28; one saying a
   plain `--interactive` run compiles no gate, G32.
10. Small: `src/outcome.rs:13`, which says nothing reads the exit status. G15.
11. Small: "three providers" in the files listed at G11, and the corpus README's line numbers,
    better replaced by function names, with rows A7 and A8 pointing at the pty test. G11, G12.
12. Small: an OpenAI key class in `src/export.rs:495-501` and `638-642`. G17.
13. Small: make the seatbelt probe's skip visible in `tests/spawn.rs:1332-1334`, and a test that
    drives the gate's catch-all. G25, G19.
14. Small: a line in `CONTRIBUTING.md` pointing at `CLAUDE.md`'s "Nothing here is enforced". G39.
15. Small: a note under the first panel's closing count in
    `docs/reviews/2026-09-22-seven-readers-as-a-hiring-panel.md:81-82`. G40.
16. Small: a warning in `src/rungs/evolve.rs` when admission runs bare on Linux, from `agy`. G24.
17. Needs a decision, one map ticket for both: what rung two may not edit. `src/pty.rs`,
    `src/watch.rs`, `src/paths.rs` and `src/run.rs` on `PROTECTED_CODE_PATHS`, or an allowlist
    instead, G5; and the probes the corpus indexes, which live in unprotected test files, G44, with
    build ticket 80.
18. Needs a decision: wire `skills::admit` into rung one's write path or remove it. G23.
19. Large: split `run::run` into phases with a guard that writes the record on any early return.
    G3, G4.
20. Large: a one-page "what is true today" at the top of `IMPLEMENTATION.md`, 3,773 lines now. Five
    of the stale paragraphs above are what it would replace.
21. Large: an independent export verifier. G18.
22. Large: a sanitized, version-labelled fixture corpus so CI's record walks read something, and a
    repeatable vendor conformance job that includes the `codex` dead-hook case, keeping synthetic
    fixtures and live measurements apart. G20, G21, merged from `cursor-agent`, `claude` and
    `codex`.
23. Large: a Linux job in CI, or a Linux sandbox. G24.
24. Large: an evaluation of rung two against held-out tasks, run from material a proposer cannot
    edit, before the README says anything about improvement. G49.

Dropped: `agy`'s "fix the concurrency cap to use a lock file", done in build 81, G30;
`cursor-agent`'s "reconcile 86 of 87 with open ticket 80", already consistent, G37.

## Method

Read in full: the four reviews, `task-base.txt`, `RUNS.md`, and the first panel's consolidated
report. Then, for each finding, the cited lines at `d264867` in this repository:
`src/pty.rs:60-130`; `src/watch.rs:95-165`; the locked `portable-pty-0.9.0` `cmdbuilder.rs` around
`get_base_env`, `new` and `as_command`; `src/run.rs:405-450`, `580-650`, `820-875`, `980-1000`,
`1215-1232`, `1260-1280`; `src/sandbox.rs:40-137`; `src/policy/mod.rs:570-590` and `1320-1350`;
`src/paths.rs` layout functions; `src/rungs/evolve.rs:75-90` and `440-470`; `src/skills/index.rs:60-82`;
`src/skills/mod.rs:470-505`; `src/outcome.rs:27-41`, `180-235`, `415-445`; `src/record/lines.rs:290-302`;
`src/export.rs:488-530`, `626-662`; `src/policy/gate.py:14-18`, `124-145`, `310-325`, `950-975`,
and the unknown-tool lines; `src/policy/replay.rs:1-30`; `src/normalize/mod.rs:980-1000`;
`src/reap.rs:286-334`; `src/rungs/review.rs:50-75`, `505-530`; `src/env.rs:100-125`;
`src/provider/codex.rs:1-45`; `tests/skills.rs:478-526`; `tests/gate.rs:1470-1510`, `1760-1770`;
`tests/spawn.rs:1330-1345`; `tests/status.rs:498-524`; `tests/record_contract.rs:574-604`;
`README.md:14-42`, `150-172`, `200-250`; `SECURITY.md:1-50`, `68-78`, `115-150`; `IMPLEMENTATION.md`
at `45-56`, `134-142`, `238-246`, `2050-2062`, `2100-2108`, `2160-2175`, `2690-2715`, `3040-3048`,
`3365-3395`, `3636-3644`, `3730-3746`; `docs/run-record-contract.md:428-437` and `800-819`;
`docs/harness-spec.md:718-730`; `tests/corpus/README.md:10-22`; `CONTRIBUTING.md` by grep; and
the outcomes of build tickets 81 and 87. I ran `frontier.py build`, counted the map tickets by
`Status:` line, grepped for every `env_clear` and every `inject_skills` caller, and read the
subjects of commits `6daad9c` and `40f9983`.

The fourth reader, `codex`, arrived after the rest of this file was written. Its first attempt,
run `01a0cc9d-8f15`, ran out of credits partway with no report; the second, run `01a0cd3e-50ea`,
finished at 7.4M tokens, and `RUNS.md` has both ids. It read `fad1b98`, as its own method says, so
its rows were checked there. For G41 to G49 and the rows the G1 fix moved I read, at `fad1b98`:
`src/policy/mod.rs:585-660`; `src/sandbox.rs:70-140`; `src/reap.rs:340-483`; `src/worktree.rs:96-116`;
`tests/spawn.rs:1445-1544`; `src/rungs/evolve.rs:120-290`, `397-430`, `440-470`, `511-545`,
`555-600`, `940-1010`; `src/rungs/review.rs:1-40`; `src/live.rs:320-405`; `src/interactive.rs:265-320`;
`src/outcome.rs:180-270`; `src/resume.rs:60-130`; `src/provider/codex.rs:95-125` and
`supports_resume` in `src/provider/mod.rs`; `src/policy/gate.py:750-765`; `tests/gate.rs:2325-2350`;
`src/normalize/mod.rs:600-642`; `src/policy/replay.rs:195-240`; `src/skills/mod.rs:440-470`;
`src/export.rs:330-372`; `tests/corpus/` whole; `README.md:18-30`, `180-192`, `200-250`;
`SECURITY.md:20-92`, `128-142`; `IMPLEMENTATION.md:3030-3065`, `3368-3395`, `3735-3749`; build
ticket 87's `## What` and 81's outcome lines 36-42; and the diff of `fad1b98` itself, including the
new test in `tests/pty_tee.rs`. `fad1b98` added two lines to `IMPLEMENTATION.md` near line 300 and
ten to `SECURITY.md` near line 79, so rows G1 to G40 that cite those files below those points are
off by that much at `fad1b98`; I left their `d264867` numbers as checked.

Not done: I did not run the suite, build the binary or launch a provider, so G1 is confirmed by
reading both sides of the call and not by observing a child's environment. The same holds for
G41, G42 and G43: each is confirmed by reading the code on both sides, and I did not repoint a
checkout's `HEAD` and reap it, plant an editing build step in a rung-two pass, or plant a variable
before an admission run. Those three reproductions are the tests the fixes need anyway. I did not check the
`agy` reader's first-panel rows F4, F5, F13 and F14, since it cited no lines. I did not read
`docs/rungs-spec.md`, `src/face.rs`, the per-provider normalizers, or build tickets 77 to 86 other
than 81. Citation counts in the verdict table are from a grep over each review and are
approximate.
