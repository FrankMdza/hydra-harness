# 313 — Stale claims in IMPLEMENTATION.md

Read-only audit of the current worktree, 2026-10-13. `CLAUDE.md` was read first, followed by all 6,129 lines of `IMPLEMENTATION.md` in order. Only this report was written. No build, test, provider launch or access outside the worktree was used.

Line numbers refer to the unchanged `IMPLEMENTATION.md`. Quotes collapse line wrapping and omit bold emphasis; wording is retained. Each numbered finding identifies the current code or configuration that contradicts the quoted assertion and proposes replacement prose. A later correction in the same document does not cure an earlier unqualified present-tense claim. Explicitly scoped port history and dated measurements are not treated as current claims.

The checked unit is one quoted sentence or standalone assertion, including a few compound sentences and short heading assertions. The totals count those units once, not every grammatical clause. This is a documented partial verification: the second list records what remains unverified.

## Stale claims, in source order

### 1. Line 22 — What is built

> A record whose `policy` is null was written before the gate reached a provider.

Evidence: `src/interactive.rs::run_ungated` and `src/rungs/evolve.rs::write_harness_start` write `policy: None` today.

Replacement:

> A null `policy` can name a pre-gate record, an ungated interactive run or a harness pass; read `launch_mode` and the envelope's provider before deciding which.

### 2. Line 167 — What the gate does about `shell`, and what it still does not

> Whether either provider reloads hooks mid-session is still unmeasured; the file is committed by `hydra reap` either way and inherited by the next run seeded from that branch.

Evidence: `src/reap.rs::commit_then_remove` excludes `provider::own_worktree_paths`; `src/provider/mod.rs::own_worktree_paths` includes both hook files.

Replacement:

> Whether either provider reloads hooks mid-session is still unmeasured; reap excludes Hydra's hook configs from its commit.

### 3. Line 169 — What the gate does about `shell`, and what it still does not

> `protected_worktree_paths` in `src/policy/mod.rs` is the list, seven entries built from the provider modules: the two hook configs, the two MCP configs beside them, and the three project-scope files `claude` reads from cwd (`.claude/settings.json`, `.claude/settings.local.json`, `.mcp.json`).

Evidence: `src/policy/mod.rs::protected_worktree_paths` builds six Cursor, agy and Codex paths plus Claude's three.

Replacement:

> `protected_worktree_paths` holds nine entries: the hook and MCP configs for Cursor, agy and Codex, and Claude's three project config files.

### 4. Line 175 — What the gate does about `shell`, and what it still does not

> The three skill roots are a second list, `write_denied_worktree_paths`, read by the `file_write` check alone: the first version of this ticket put them on the whole-command list, the corpus walk caught `ls -la .claude/skills/` on run `01a09405-f451` reading as a leak, and a shell command naming a skill root is how two of the three providers read a skill, so the roots are denied to writes and to nothing else.

Evidence: `src/policy/mod.rs::write_denied_worktree_paths` enumerates all four providers.

Replacement:

> The four skill roots are a second list, `write_denied_worktree_paths`, checked for file writes; naming a root in a shell read does not by itself refuse the command.

### 5. Line 197 — What the gate does about `shell`, and what it still does not

> `paths::hydra_home` answers where `policies/` and `config/` live, the variable first and the compile-time `CARGO_MANIFEST_DIR` second, and five call sites go through it.

Evidence: `rg -n 'hydra_home\(' src` finds callers in policy, MCP, panels, exports, doctor, cockpit, bench, main and launch paths, already more than five; `src/paths.rs::hydra_home` retains the stated precedence.

Replacement:

> `paths::hydra_home` answers where the shipped files live, `HYDRA_HOME` first and the compile-time `CARGO_MANIFEST_DIR` second; launch, inspection and integration readers share it.

### 6. Line 449 — `hydra stop`, since build ticket 169

> `--all` marks every run it found, scans again, and repeats until a scan finds nothing new, so a seat that appeared after the first scan is stopped before its parent.

Evidence: `src/stop.rs::all_marked` bounds rescans by `MAX_PASSES = 10`.

Replacement:

> `--all` marks and rescans up to ten times, stopping early when no new live run appears; later launches may need another stop.

### 7. Line 527 — Cleanup: `hydra reap` and `hydra prune`

> `commit_then_remove` in `src/reap.rs` generalizes what used to be `remove_checkout` to serve both directories: it takes the branch Hydra made, `hydra/<run-id>` for the worktree and `ouroboros/curate/<run-id>` for the checkout, and the commit message, `hydra reap: what curate run <run-id> left uncommitted` for the checkout.

Evidence: `src/reap.rs::one` passes `rungs/curate/{run_id}` to `commit_then_remove`.

Replacement:

> `commit_then_remove` serves both directories, expecting `hydra/<run-id>` for the worktree and `rungs/curate/<run-id>` for the skill checkout.

### 8. Line 541 — Cleanup: `hydra reap` and `hydra prune`

> `provider::own_worktree_paths` lists the six files Hydra writes into a checkout, built from the two providers' `gitignore` functions rather than retyped: `.cursor/.gitignore`, `.cursor/hooks.json`, `.cursor/mcp.json`, `.agents/.gitignore`, `.agents/hooks.json`, `.agents/mcp_config.json`.

Evidence: `src/provider/mod.rs::own_worktree_paths` also reads `codex::gitignore`; `own_worktree_paths_are_the_names_each_gitignore_lists` asserts nine entries.

Replacement:

> `provider::own_worktree_paths` lists nine files, built from Cursor's, agy's and Codex's ignore lists; the Codex entries are `.codex/.gitignore`, `.codex/hooks.json` and `.codex/config.toml`.

### 9. Line 579 — Cleanup: `hydra reap` and `hydra prune`

> A child that checked out one of the human's own branches gets its work committed there and the branch left alone.

Evidence: `src/reap.rs::commit_then_remove` bails on `branch != hydra_branch` before staging or committing.

Replacement:

> A checkout on any branch but the run's own is refused before staging, and its checkout and spawn directory stay.

### 10. Line 636 — Cleanup: `hydra reap` and `hydra prune`

> That test reads every `.jsonl` under `~/hydra/runs/` and returns quietly when there are none, so a prune sweep costs the check silently.

Evidence: `tests/record_contract.rs::every_fixture_meets_the_contract` reads `common::fixture_corpus` and asserts it is nonempty.

Replacement:

> The contract test reads the checked-in fixture corpus, so pruning live records does not empty it; `hydra doctor --walk` is the check over the machine's records.

### 11. Line 649 — The policy set and the gate, on a run

> `tests/gate.rs` feeds the compiled script synthetic hook payloads and asserts its answers, 23 of them when ticket 02 closed and 72 tests today; `tests/spawn.rs` checks that a run writes the script where the run will find it.

Evidence: `rg -c '^#\[test\]' tests/gate.rs` prints `117`; this counts source test declarations, without running them.

Replacement:

> `tests/gate.rs` has 117 test declarations feeding compiled gates synthetic payloads; `tests/spawn.rs` checks the spawn artifacts.

### 12. Line 689 — The policy set and the gate, on a run

> Nineteen of `claude`'s twenty-nine tools answer to no capability the five policies speak in.

Evidence: `src/policy/table.rs::CLAUDE` has 32 rows, 19 `ProviderNative`; `Agent`, `Glob` and `Grep` expanded the old table.

Replacement:

> Nineteen of the 32 named rows in Claude's capability table are `provider_native`; the table keeps old spellings so old records still read.

### 13. Line 710 — The policy set and the gate, on a run

> Two protected configs are still paths rather than files.

Evidence: `rg --files config` lists `config/export-allowlist.json`, and no `config/env-allowlist.json`; `src/env.rs` implements the latter's role.

Replacement:

> Of the two paths named here, only `config/env-allowlist.json` still has no file; the export allowlist exists.

### 14. Line 735 — The policy set and the gate, on a run

> The file does not exist on this machine because the human writes it, so until they do every request for seats waits for approval.

Evidence: The worktree contains `config/panels.json`, with shape `review`, asker `standard`, three seats and cap two per hour; `src/interactive.rs::askable_shapes` and `src/face.rs::panel_tool` expose approved shapes.

Replacement:

> The shipped `config/panels.json` defines a three-seat `review` shape for `standard`, capped at two requests per hour; matching requests use that approval.

### 15. Line 886 — The policy set and the gate, on a run

> Item 5 is built and not granted.

Evidence: `policies/standard.json` grants `mcp_call` with `servers: ["hydra"]`; `src/policy/gate.py::evaluate` enforces server-scoped grants.

Replacement:

> Item 5 is built and granted for Hydra's own MCP server under `standard`.

### 16. Line 886 — The policy set and the gate, on a run

> The capability table had no way to scope `mcp_call`, and `policies/standard.json` grants no `mcp_call` at all, so today the gate refuses `mcp__hydra__delegate` to a `standard` session and the channel is unreachable through the face.

Evidence: `policies/standard.json` grants the `hydra` server; the scoped-server branch of `src/policy/gate.py::evaluate` accepts matching names, subject to other checks.

Replacement:

> `standard` grants `mcp_call` to the `hydra` server, so the face is reachable through the gate; the launcher still checks containment, shapes and approvals.

### 17. Line 1322 — The canonical MCP document, and what a run is handed

> `mcp_call` is granted by `permissive` alone (7.2), so a server admitted to `standard` connects, its tools reach the model, and the gate refuses every call to it.

Evidence: `policies/standard.json` grants Hydra's server, while `permissive` has the unscoped grant.

Replacement:

> Admission still differs from permission: `standard` may call the admitted `hydra` server, while other servers need a grant of their own; `permissive` grants MCP without a server list.

### 18. Line 1551 — `cursor-agent`, the second provider

> `WriteShellStdin` is `shell` with no command in its input, so the pattern floor cannot see what it feeds a background shell, which is 7.4's failure mode on a second spelling and is the reason the row says so.

Evidence: `src/policy/table.rs::CURSOR` maps `WriteShellStdin` to `OpaqueShell`.

Replacement:

> `WriteShellStdin` is `opaque_shell`, which no shipped policy grants, because the gate cannot judge what it feeds a running shell.

### 19. Line 1628 — `cursor-agent`, the second provider

> Four tools Cursor runs without asking the hook, and the table says so, since build ticket 35.

Evidence: `src/policy/table.rs::CURSOR` has five `unhooked` rows: `ReadLints`, `getMcpToolsToolCall`, `awaitToolCall`, `WebSearch`, `updateTodos`.

Replacement:

> Five Cursor rows are marked unhooked, including `updateTodos` since build ticket 286.

### 20. Line 1695 — `cursor-agent`, the second provider

> Not measured through `hydra run`, because `config/mcp.md` admits no server and is protected config #5; the two real runs each listed nothing, wrote `{"mcpServers":{}}`, and recorded `injected: []` beside `resolved: []`.

Evidence: `config/mcp.md` declares `hydra`, admitted under `standard` and `permissive`; the historical run observations were not rechecked.

Replacement:

> The two measured runs predated the shipped `hydra` server; the current document admits it under `standard` and `permissive`.

### 21. Line 1831 — `agy`, the third provider

> It keys on the payload rather than on the dialect, so the same script answers a live hook and contract 7.7's read-time replay, which sends the normalized shape for every provider.

Evidence: `src/policy/gate.py::as_tool_call` reads `HYDRA_CONFIG["dialect"]`; `evaluate` first rejects a foreign envelope.

Replacement:

> It unwraps only the configured dialect's envelope, and also accepts the normalized shape the replay sends.

### 22. Line 1868 — `agy`, the third provider

> Three of the fifty-seven are `shell` and carry no command for the floor to read.

Evidence: `src/policy/table.rs::AGY` maps `send_command_input` to `OpaqueShell`, leaving `command_status` and `notebook_execution` as `Shell`.

Replacement:

> Of these three commandless rows, `command_status` and `notebook_execution` remain `shell`; `send_command_input` is `opaque_shell` and refused by every shipped policy.

### 23. Line 2022 — `agy`, the third provider

> The 16,338-token floor on a resumed `--conversation` run was not measured, and neither `--conversation` nor `--continue` is reachable from Hydra, which mints a fresh run id per resume.

Evidence: `src/provider/agy.rs::resume_args` returns `--conversation <id>`; `src/resume.rs::resume_in` carries the provider session into a new run. The token measurement is outside this audit.

Replacement:

> The resumed usage floor remains unmeasured; `hydra resume` reaches agy's `--conversation <id>` while giving the continuation a fresh Hydra run id.

### 24. Line 2096 — `codex`, the fourth provider, since build ticket 48

> Routable under `standard` and `permissive` only.

Evidence: `src/provider/codex.rs::unsupported_policy` refuses only `read-only`; the adjacent test explicitly allows `judge`.

Replacement:

> Codex's policy pre-flight refuses only `read-only`; `judge` is routable since build ticket 61.

### 25. Line 2126 — What the record does not carry

> `skills` is present on every run with a checkout and absent on the rest.

Evidence: `src/run/mod.rs::inject_into_checkout` returns before injection on `judge`; `src/interactive.rs::run_ungated` also writes no skills.

Replacement:

> A recorded non-judge run with a checkout gets the skills group; judges and ungated interactive runs do not.

### 26. Line 2193 — What the record does not carry

> That test also walks every record under `~/hydra/runs/` on this machine and fails if any of them drifted.

Evidence: `tests/record_contract.rs::every_fixture_meets_the_contract` walks fixtures; `src/walk.rs` implements the live walk.

Replacement:

> The test walks the checked-in records; `hydra doctor --walk` checks the machine's corpus.

### 27. Line 2213 — Reading a record back

> Eight of the ten types have a producer.

Evidence: `src/normalize/mod.rs::Event` has ten variants; `src/normalize/claude/harvest.rs::splice_corrections` produces `Correction` in addition to the eight listed.

Replacement:

> Nine of the ten event types have a producer; only `message_delta` remains without one.

### 28. Line 2283 — Reading a record back

> A key the record does not carry is rebuilt the way build ticket 14 left it: `worktree` off `run_start.worktree.path` and otherwise the launch's own `spawn/<run-id>/no-worktree` placeholder; `corpus`, `skill_store` and `gate_script` from the reader's own `Layout`; `skill_checkout` from the record's `policy` field, `Some` on `curate` alone; and `parent_git_dir` from `git rev-parse --git-common-dir` inside the record's worktree, through the same `crate::worktree::git_common_dir` the launch calls from the parent.

Evidence: `src/normalize/mod.rs::ReplaySpawn::of` recovers a missing parent through `CheckoutGit::for_run`; `src/worktree.rs::CheckoutGit::for_run_from` reads the saved `.git-context.json` instead of discovering through writable `.git`.

Replacement:

> For old records without scopes, the replay rebuilds layout paths and recovers `parent_git_dir` only through the trusted saved checkout context; a missing context becomes a limit, not a git discovery inside the run's checkout.

### 29. Line 2357 — Reading a record back

> And `src/policy/replay.rs` runs the gate with the reader's own environment and interpreter, setting only `PWD`, while the gate expands `~` and variables from its environment.

Evidence: `src/policy/replay.rs::Replay::run_gate` calls `env_clear`, restores `gate_env_names(provider)` with reader values and sets `PWD`.

Replacement:

> The replay clears its environment, passes only the provider allowlist with the reader's values, and sets `PWD`; path resolution still depends on the reader's filesystem and allowed values.

### 30. Line 2369 — Reading a record back

> One place still writes `gate_sha: null`.

Evidence: `src/rungs/evolve.rs::write_harness_start` sets `gate_sha: None`, but `src/record/lines.rs::RunStart::gate_sha` has `#[serde(skip_serializing_if = "Option::is_none")]`; no null field is emitted by that writer.

Replacement:

> A harness pass compiles no gate and omits `gate_sha`; the serializer also omits it on any other launch with no compiled gate.

### 31. Line 2380 — Reading a record back

> Seven of them: a gate leak, subagents spawned under a policy that denies them, a disagreement between the derivation and the gate's recorded answer, a gated call the gate never answered, a tool name the capability table does not know, a `policy_sha` that is not today's, and a bare limit for anything else.

Evidence: `src/normalize/mod.rs::Finding` has eight variants, including `UnhookedDenied`.

Replacement:

> There are eight finding types; `UnhookedDenied` names a provider running a denied tool without asking its hook, beside the seven listed here.

### 32. Line 2609 — The detached spawn, the timeout, and what `hydra status` reconciles

> `src/watch.rs` is the half that owns the pty and the record, `src/run.rs` is the half a human talks to, `src/live.rs` is contract 5.2's test, and `src/status.rs` and `src/logs.rs` are the two commands that read what the test says.

Evidence: `src/run/mod.rs` defines the launch; `src/run/record.rs`, `lineage.rs` and `refuse.rs` hold its helpers; `src/run.rs` is absent.

Replacement:

> `src/watch.rs` owns the pty and record, `src/run/mod.rs` owns the launch, and `src/live.rs` supplies the liveness reading used by status and logs.

### 33. Line 2648 — The detached spawn, the timeout, and what `hydra status` reconciles

> `git` and the read-time gate replay still inherit, and build ticket 97's Outcome has the table of every `Command::new` in `src/`.

Evidence: `src/policy/replay.rs::Replay::run_gate` now calls `env_clear` before copying allowlisted names.

Replacement:

> Git still begins from the caller's environment with its configured removals; since build ticket 286, gate replay starts empty and copies only the provider allowlist.

### 34. Line 2675 — The detached spawn, the timeout, and what `hydra status` reconciles

> Nothing kills a child, ever.

Evidence: `src/stop.rs::terminate` sends SIGTERM and SIGKILL; `src/watch.rs` also propagates a stopped parent's marker to a late child.

Replacement:

> Timeouts do not kill a run; `hydra stop` sends SIGTERM and, after the grace period, SIGKILL.

### 35. Line 2748 — The detached spawn, the timeout, and what `hydra status` reconciles

> The diff stat's base is recovered from the seed index rather than persisted.

Evidence: `src/run/record.rs::result` prefers `run_start.worktree.base_tree` over the caller's recovered seed; `run_start` persists the worktree reference.

Replacement:

> The diff stat takes `base_tree` from the record, falling back to the seed index for older records.

### 36. Line 3028 — Delegation, and the face

> A child reads two fields off its parent's record and nothing else.

Evidence: `src/run/lineage.rs::Lineage::of` also reads `command` and `launch_mode` to recognize a harness parent.

Replacement:

> Lineage reads the parent's depth and policy, plus `command` and `launch_mode` to recognize an unbounded harness parent.

### 37. Line 3028 — Delegation, and the face

> `run::Lineage::of` takes `depth` and `policy` off line 0 of `~/hydra/runs/<parent>.jsonl` and refuses three shapes before a run id exists: a parent this machine has no record of, a parent whose `policy` is null, which is a pre-gate record or an interactive one, and a child that would land past the cap.

Evidence: `src/run/lineage.rs::Lineage::of` explicitly accepts `command: evolve`, null `launch_mode`, null `policy` as a harness parent.

Replacement:

> Lineage refuses a missing parent, an unusable depth, a depth beyond the cap or a null-policy parent other than a harness pass; an evolve parent is accepted without a policy bound.

### 38. Line 3067 — Delegation, and the face

> One tool, `delegate(provider, task, files?, policy?)`, and everything the model gets wrong comes back as a tool result rather than a protocol error.

Evidence: `src/face.rs::tools` always lists `delegate` and conditionally adds `panel` when the parent has a seat socket.

Replacement:

> Every face lists `delegate(provider, task, files?, policy?)`; a parent with a seat socket also gets `panel(shape, task, files?)`, and bad tool arguments return tool errors.

### 39. Line 3121 — Delegation, and the face

> `permissive` is the only admission worth writing, because `mcp_call` is granted by that policy alone (7.2), so a face admitted to `standard` would start, list its tool, and have every call to it refused by the gate.

Evidence: `policies/standard.json` grants `servers: ["hydra"]`; `config/mcp.md` admits the face to both policies.

Replacement:

> The face is admitted under `standard` and `permissive`; `standard` grants calls only to the `hydra` server.

### 40. Line 3330 — `judge` is not injected, and the face is admitted

> The command in the document is the absolute path to the binary, because a recorded run's `PATH` is the human's and `hydra` is not on it: measured on 2026-09-07, a bare `hydra` came up `failed` in the init frame and the absolute path came up `connected` with `mcp__hydra__delegate` on the tool list.

Evidence: The active TOML in `config/mcp.md` says `command = "hydra"`; `src/mcp.rs` loads that declaration. The 2026-09-07 observation is not disputed.

Replacement:

> The shipped document now says `command = "hydra"`, so the child's PATH must resolve it; the earlier absolute-path probe remains a dated measurement.

### 41. Line 3685 — Master of Puppets, since build tickets 108 and 109

> A run record that is not UTF-8 raises before any line is read.

Evidence: `crates/puppets/src/launch.rs::run_file_has_end` reads bytes, splits lines, and skips a line that fails UTF-8 decoding.

Replacement:

> The wait reader scans bytes line by line and skips invalid UTF-8, so a later `run_end` can still finish the wait.

### 42. Line 4469 — `hydra cockpit`, since build ticket 150

> An ended record is normalized once and kept by run id and file length, because normalizing replays the gate, a Python process per recorded tool call.

Evidence: `src/policy/replay.rs::Replay::decide_all` batches all calls and falls back to per-call processes only on batch failure.

Replacement:

> An ended record is cached by run id and file length; gate replay normally uses one Python process per record and falls back to one per call if the batch fails.

### 43. Line 4711 — `hydra cockpit`, since build ticket 150

> Interactive runs still support only `claude`; other providers are refused by the child in the panel.

Evidence: `src/interactive.rs::refuse_provider` accepts every provider when ungated; `src/cockpit/mod.rs` starts `Launch::hydra_run` without a policy.

Replacement:

> The cockpit's ungated interactive chat supports all four providers; only gated interactive launches remain Claude-only.

### 44. Line 4990 — `hydra export`, since build ticket 34

> Nothing on this machine has been exported: the allowlist does not exist, and `hydra export --dry-run` says so.

Evidence: The worktree contains `config/export-allowlist.json`; `src/export.rs::allowlist_path` names it. Export history cannot be established within the worktree.

Replacement:

> The export allowlist exists in this checkout; export still checks each run's eligibility, and whether an export has actually been made is not established here.

### 45. Line 5164 — Rung two: `hydra evolve`, since build ticket 36

> A pass is a harness run with four children.

Evidence: `src/rungs/evolve.rs::evolve` starts the proposer; `run_judge_panel` iterates `src/rungs/review.rs::PANEL_ORDER`, which has four providers.

Replacement:

> A full pass is a harness run with five children, one proposer and four judges; an early refusal may stop it before the judges.

### 46. Line 5168 — Rung two: `hydra evolve`, since build ticket 36

> `--model` names the proposer's model and nothing chooses one on the human's behalf.

Evidence: `src/rungs/evolve.rs::run_judge_panel` copies `proposer_model` into the Claude judge request as well.

Replacement:

> `--model` names the proposer's model and the Claude judge's; the other judges leave their model unspecified.

### 47. Line 5226 — Rung two: `hydra evolve`, since build ticket 36

> Each refusal lands on `ouroboros/blocked/<pass-id>` with its reason in the commit message, spec 4.9.

Evidence: `src/rungs/evolve.rs::land_proposer_patch` constructs `rungs/blocked/{pass_run_id}`.

Replacement:

> Each blocked patch lands on `rungs/blocked/<pass-id>` with its reason in the commit message.

### 48. Line 5300 — Rung two: `hydra evolve`, since build ticket 36

> The verdict lives in the commit message, spec 4.9, on `ouroboros/<pass-id>` when the panel passed and `ouroboros/blocked/<pass-id>` when it did not, naming the pass, the proposer and the four judges by run id, with the four answers in the body.

Evidence: `src/rungs/evolve.rs::land_proposer_patch` and `review_blocked_in` write `rungs/` branches; legacy names remain in lookup helpers only.

Replacement:

> The verdict lives in the commit message on `rungs/<pass-id>` or `rungs/blocked/<pass-id>`; readers also accept the older `ouroboros/` names.

### 49. Line 5376 — Rung two: `hydra evolve`, since build ticket 36

> The admission gate runs `cargo test`, this suite walks every record under `~/hydra/runs/`, and the proposer's own record is one of them, live, while the gate reads it.

Evidence: `src/rungs/evolve.rs::admission_gate` runs the suite with a scratch root, and `tests/record_contract.rs::every_fixture_meets_the_contract` plus the normalization walks use fixtures.

Replacement:

> That coupling described the early passes; admission now runs against a scratch root and the suite walks checked-in fixtures, while the live corpus check is `hydra doctor --walk`.

### 50. Line 5422 — Rung one: `hydra curate`

> `corpus.rs` decides which runs the curator may learn from, `curate.rs` composes what it is told, launches it as an ordinary recorded run under `curate`, and commits whatever that run wrote into its checkout of the skill store onto `ouroboros/curate/<run-id>`.

Evidence: `src/rungs/curate.rs::curate` sets `branch = format!("rungs/curate/{run_id}")`.

Replacement:

> `corpus.rs` selects the records and `curate.rs` launches the curator, then proposes its skill-store edits on `rungs/curate/<run-id>`.

### 51. Line 5460 — Rung one: `hydra curate`

> Two of the five classes have no producer in this build at all, human rejection and the retry loop, and the brief names them as that rather than printing a zero somebody could read as a measurement.

Evidence: `src/normalize/claude/harvest.rs::derive_corrections` produces corrections; `src/rungs/corpus.rs::signals` counts them and `Selection::classes` counts parented runs as retries. `curate::brief` still has stale prose about human rejection, but prints the retry count as implemented.

Replacement:

> All five classes have code paths: harvested corrections supply human rejection, and parented records supply the retry count; the brief's no-interactive-producer wording is itself stale.

### 52. Line 5613 — Choices this build made that no ticket settled

> Minting a fifth is a contract change rather than an implementation, so the refusal goes to stderr and no run id is minted.

Evidence: `src/run/mod.rs::reserve_on_layout` creates the UUID before calling `live::reserve`, whose cap check may fail.

Replacement:

> The cap refusal writes no run record, although the launcher has already minted the id it tried to reserve.

### 53. Line 5671 — Choices this build made that no ticket settled

> A `PATH` without `/usr/bin` refuses the launch before a run id exists, with an error naming the silent logout it would otherwise cause.

Evidence: `src/run/mod.rs::reserve_on_layout` mints the id before `prepare_machine` calls `compose_child_env`; `src/env.rs::ChildEnv::build` performs the PATH check.

Replacement:

> A `PATH` without `/usr/bin` refuses the recorded launch when the child environment is built, after an id is minted but before a record is written.

### 54. Line 5788 — Not built

> No record on this machine carries `mcp_unavailable`: `tests/mcp.rs` drives the refusal against a fixture document, and the shipped document admits nothing to anybody, so producing one for real means a human adding a server.

Evidence: `config/mcp.md` admits `hydra` to `standard` and `permissive`; live record history is unavailable within this audit.

Replacement:

> The shipped document admits `hydra` under `standard` and `permissive`; `tests/mcp.rs` checks `mcp_unavailable` with a fixture that requests a server the launch cannot deliver.

### 55. Line 6094 — Rough edges worth knowing

> Killing the detached half mid-run produces one, and nothing completes it: `hydra status` reports it as abandoned and stops there, because completing it means harvest and no run on this machine has a vendor transcript to harvest from.

Evidence: `src/status.rs::all_in` calls `harvest_abandoned_interactive` and rescans; `is_abandoned_interactive` accepts Claude, Codex, Cursor and agy interactive envelopes. Recorded stream runs still have no equivalent recovery. No machine-wide transcript history was read.

Replacement:

> A missing end does not make a record corrupt. Status attempts harvest for abandoned interactive runs from all four providers; an abandoned recorded stream run stays open.

### 56. Line 6101 — Rough edges worth knowing

> nothing in this build can end it.

Evidence: `src/stop.rs::command`, `stop` and `terminate` implement the human's stop command.

Replacement:

> `hydra stop` can end it, while a timeout or Ctrl-C of the recorded foreground view leaves it running.

### 57. Line 6101 — Rough edges worth knowing

> That is deliberate rather than missing: nothing here signals a child, and the design gives removal to `hydra reap`, which a human types.

Evidence: `src/stop.rs::terminate` signals the child or its process group; reap is a separate cleanup command.

Replacement:

> Stopping and cleanup are separate human commands: `hydra stop` signals the run and `hydra reap` commits and removes its checkout.

## Claims not verified

1. **Live corpus and provider observations.** Claims about what no record on this machine contains, actual exports, provider hook failures, measured tools and vendor transcript availability cannot be established from this checkout. I checked the associated local configuration or code where cited above, not the machine-wide conclusion. No live records, vendor homes or parent directories were accessed. Dates, timings and token measurements were excluded, as requested.

2. **Remaining numerical and equivalence claims in the ports (lines 3486–4336, 6011–6021).** I did not reproduce the Python/Rust differential comparisons, count the compiled test binaries (including the 76-binary assertion), or verify every Unicode, CSV, floating-point and exception-parity claim. That would require wider source tracing or execution; no build or test was run.

3. **No reachable NaN score (lines 3986–3987).** The claim that no command reaches log loss with NaN needs a complete trace of CLI inputs, model loading and arithmetic. A text search cannot prove that reachability assertion, and it is not certified here.

4. **Dead scratch-link path under label (lines 4135–4139).** The statement about Python show_projection being dead under label needs the historical Python callers and their input invariants. The current Rust command alone does not establish that historical no-caller claim.

5. **UI and interactive lifecycle detail (lines 4337–4840).** Only the provider-support and replay-process assertions listed above were checked in detail. Key handling, rendering dimensions, pane resizing, recorder detach/reconnect and terminal restoration were not exhaustively traced or exercised.

6. **Security and export completeness (especially lines 31–194, 740–930, 4840–4996, 5903–5977 and 6110–6116).** The report checks named policy rows and selected control paths; it does not prove every containment, symlink, redaction or fail-open statement. In particular, the claim that nothing checks whether the gate interpreter can start needs a full launch-path audit, not merely the absence of a named preflight function.

7. **Remaining present-tense code claims and repeated references.** Reading all 6,129 lines did not amount to separately proving every sentence. Claims absent from both ledgers remain unchecked, including detailed grader/export/cache behavior and further mentions of old module paths. The count below is the explicitly evidenced set, not a claim of exhaustive verification.

## Checked assertions retained

These are the other checked units, recorded so the denominator is reviewable. A confirmed local code fact does not revalidate a vendor observation used to explain it.

1. **Line 6:** The dev profile carries line tables only, build ticket 295.

   Evidence: `Cargo.toml`, `[profile.dev]` and package override.

2. **Line 13:** The root `[profile.release]` sets only `lto = "off"`.

   Evidence: `Cargo.toml`, `[profile.release]` has only that setting.

3. **Line 223:** They carry `Capability::OpaqueShell` now, which no policy grants, so the gate answers `hydra/capability` by name rather than letting a `permissive` run feed a protected path to a shell it had already opened.

   Evidence: `src/policy/table.rs` maps both stdin tools to `OpaqueShell`; the shipped policies have no such grant.

4. **Line 248:** `tests/fixtures/records/` holds twelve synthetic records in five directories named by the binary that wrote each shape: `claude@2.1.261`, `cursor-agent@2026.09.02-c22c1a3`, `agy@1.1.26`, `codex@0.154.0` and `harness`.

   Evidence: `rg --files tests/fixtures/records` lists twelve JSONL files under those five directories.

5. **Line 466:** What holds it here is that nothing in `src/` calls `reap::reap` or `prune::prune` except `main`.

   Evidence: `rg -n 'reap::reap|prune::prune' src` finds only `src/main.rs`.

6. **Line 511:** An absent run is still benign and the test that pins it now says so with a well-formed uuid.

   Evidence: `src/reap.rs::one` returns `Reaped::Done` with empty removal fields when no worktree, skill checkout or spawn exists; the resolver lets a full UUID through.

7. **Line 571:** The branch survives whatever it carries, decided by the same mechanism.

   Evidence: `src/reap.rs::drop_branch` uses `-d` by default; `commit_then_remove` retries `-D` only under the explicit flag.

8. **Line 588:** A detached HEAD is refused rather than reaped.

   Evidence: `src/reap.rs::commit_then_remove` returns an error before staging.

9. **Line 1200:** The store's validator parses the frontmatter with a real YAML parser rather than scanning for `: `, because a scan is wrong in both directions.

   Evidence: `src/skills/frontmatter.rs::parse` calls `yaml_rust2::YamlLoader`.

10. **Line 1212:** The frontmatter `name` must equal the directory name, because `claude` keys on the directory and `agy` on the frontmatter and a store entry where they disagree is a different skill to each of them.

   Evidence: `src/skills/frontmatter.rs::parse` rejects a different name; provider behavior is historical context, not newly measured.

11. **Line 1215:** And `disable-model-invocation: true` is refused, because it hides the skill from the provider's own listing, which is the answer the launch check reads.

   Evidence: `src/skills/frontmatter.rs::parse` rejects the boolean; the vendor explanation was not remeasured.

12. **Line 1309:** The document is prose with one fenced `toml` block in it, and the block is the data.

   Evidence: `src/mcp.rs::Document::parse` calls `fenced_toml`, which requires exactly one closed block.

13. **Line 1350:** `mcp.doc_sha` is a sha256 of the whole file, prose included.

   Evidence: `src/mcp.rs::Document::parse` hashes `text.as_bytes()` before extracting TOML.

14. **Line 3037:** The cap is `live::MAX_DEPTH` and it is 2.

   Evidence: `src/live.rs::MAX_DEPTH = 2`; `src/run/lineage.rs::Lineage::of` compares before incrementing.

15. **Line 3859:** `hydra-grader jev build`, `patches` and `score`; `ask` is not ported (map 94), went with `jev_probe.py` in build 220, and no TLS is added.

   Evidence: `crates/grader/src/jev_cli.rs::main` dispatches the three local commands and returns 2 with the dropped-command explanation for `ask`; this confirms the absence of an ask implementation.

16. **Line 3939:** The label queue, `label-queue` and `label-import`, is the other half of `baseline.py` and is not here: the binary says so and exits 2.

   Evidence: The paragraph at lines 3836–3838 explicitly scopes these subsections to their port dates. The queue is implemented now (`crates/grader/src/baseline_cli.rs::main`), and the surrounding section says ticket 217 added it. Retained as historical, not a current absence claim.

17. **Line 4148:** `records.iter_records` has no caller in the grader and is not ported.

   Evidence: `rg -n 'iter_records' crates/grader/src` finds only the module documentation, not a function or caller.

18. **Line 4796:** The source record is never rewritten, and the continuation is a run of its own with a fresh id, worktree and gate.

   Evidence: `src/resume.rs::resume_in` reads the source and builds a new `run::Request` with `resumed_from` and no reused run id.

19. **Line 4965:** The interactive launch does not pass through the refusal yet.

   Evidence: `src/interactive.rs::run_gated` and `run_ungated` call the non-ASCII check but not `refuse_foreign_checkout`.

20. **Line 5591:** The concurrency cap is eight.

   Evidence: `src/live.rs::LIVE_RUN_CAP = 8`, passed by `src/run/mod.rs::reserve_on_layout`.

21. **Line 5599:** Since build ticket 81 the cap is a reservation, not a count.

   Evidence: `src/live.rs::reserve` holds the reservation-directory lock while inspecting pending reservations and live records.

22. **Line 5612:** A run refused by the cap writes no record.

   Evidence: `src/run/mod.rs::reserve_on_layout` propagates `live::reserve` errors before record creation.

23. **Line 5695:** The store's pin commit is attributed to `hydra <hydra@localhost>` with the run id in a trailer.

   Evidence: `src/skills/mod.rs::Store::commit` builds `Hydra-Pinned-By` and supplies the identity.

24. **Line 6020:** CI remains on cargo test; changing its runner is left to the human.

   Evidence: `.github/workflows/ci.yml` invokes `cargo test --release --workspace --no-fail-fast`.

25. **Line 6028:** All actions in the security and existing CI workflows are pinned to commit SHAs with their reference in a comment.

   Evidence: Every `uses:` value in `.github/workflows/ci.yml` and `security.yml` is a full SHA followed by a version comment.

26. **Line 6029:** The existing fmt, clippy, build and test job stays on `macos-latest` with `CARGO_INCREMENTAL=0`.

   Evidence: `.github/workflows/ci.yml`, `jobs.check` and its environment.

27. **Line 6037:** No test calls `std::env::set_var` or `std::env::remove_var`; the admission environment probe in `src/rungs/evolve.rs` already used a child.

   Evidence: `rg -n 'set_var\(|remove_var\(' src tests crates -g '*.rs'` returned no matches; the admission probe re-executes.

## Pendiente de mí

The seven groups above remain unverified. Applying any replacement is outside this ticket; `IMPLEMENTATION.md` and the ticket are unchanged.

## Cambiado

Only `.scratch/hydra/research/313-stale-claims.md` was added, using the edit tool.

## Encontrado

Checked **84 sentence/assertion units**: **57 stale**, **27 retained**. These counts cover the explicit ledgers, not every checkable sentence in the document.

The three sections with the most stale assertions are:

- **The policy set and the gate, on a run: 6.**
- **Reading a record back: 5.**
- **Rung two: `hydra evolve`, since build ticket 36: 5.**

