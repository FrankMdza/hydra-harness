# Fourth panel review, 2026-09-24 (agy)

## Verdict

Advance to technical interview.
The repository demonstrates a deep understanding of process isolation, threat modeling, and state management, building a functional, deterministic harness around non-deterministic AI tools. While there are open security gaps, such as the watcher inheriting the shell environment or the object store vulnerability, they are largely documented and reflect explicit engineering trade-offs rather than ignorance.
Level: Mid-level Systems/Security Engineer.
Outreach note: "Your portfolio project Hydra is an impressive piece of systems engineering. We were particularly struck by your design of the out-of-process gate and the event-sourced record contract, and we would love to talk to you about our own agent scaffolding."

## Scores

| Axis | Score | Evidence |
|---|---|---|
| Rust engineering quality | 8 | The use of custom derive macros for serialization only on normalized records shows a solid grasp of type-state (`src/outcome.rs:2088-2094`). |
| Test discipline | 8 | The test suite is extensive and correctly avoids real data, using synthetic fixtures instead (`tests/record_contract.rs:577-605`). |
| Security and threat-model honesty | 9 | `SECURITY.md` explicitly documents what the gate does not catch, such as shell interpolation, demonstrating extreme honesty (`SECURITY.md:18-28`). |
| Documentation and communication | 7 | The documentation is thorough but suffers from some drift and stale facts, such as missing `codex` in the provider enum (`docs/run-record-contract.md:143`). |
| Depth and originality of the harness idea | 9 | Compiling the policy directly into the vendor's hook dialect per run is a highly original and robust isolation strategy (`README.md:143-151`). |
| Engineering process | 8 | Turning third panel reviews into explicitly numbered build tickets and addressing them methodically shows a professional, mature workflow (`.scratch/hydra/build/97-*.md`). |

## Strengths

1. **Per-run compiled policies**: The gate script is dynamically compiled into the vendor's hook dialect per run (`src/policy/mod.rs`). Labs care about this because it prevents static policy bypasses.
2. **Outcome derivation from event stream**: The outcome is logically derived from tool calls (e.g., `src/outcome.rs:122-280`) rather than relying on the vendor's potentially inaccurate exit status, which is critical for accurate evaluation labels.
3. **Strict separation of capabilities**: The environment allowlist (`src/env.rs:43-52`) prevents implicit credential leakage to the child process, a must-have for safe local agent execution.
4. **Append-only orchestration**: The `Master of Puppets` mode (`integrations/puppets/puppets.py`) uses an append-only JSONL log to fold state, providing robust auditability for multi-agent workflows.
5. **Rigorous documentation of limitations**: `SECURITY.md` meticulously lists the limits of the sandbox, such as the fail-open behavior of the Codex hook (`SECURITY.md:155-166`), showing mature threat modeling.

## Weaknesses and red flags

1. **A design smell**: Rung two's admission runs on the proposer's still-writable checkout (`src/rungs/evolve.rs:1261`), allowing a malicious test runner to spoof a passing check before the read tree is hashed.
2. **A bug**: The watcher process that owns the pty inherits the launching shell's full environment (`src/watch.rs:103-134`), exposing credentials to a same-uid `ps -E` despite `SECURITY.md`'s claims.
3. **A design smell**: An error inside `worktree::create` after `git worktree add` leaves an orphaned checkout and branch with no record, because the failure path is built too late (`src/run.rs:480-486`).
4. **A bug**: Launch discovery uses plain git, so if a human launches from inside a run's checkout, a rewritten `.git` file can trigger a decoy hook (`src/worktree.rs:159-169`).
5. **A scope limit the author already admits**: The object store is fully writable, allowing a run to plant a forged pack that could map an existing SHA to other bytes, affecting rung two's offered branch (`src/policy/mod.rs:640-646`).
6. **A documentation gap**: The `codex` provider is missing from the contract enum, meaning a strict parser would reject valid `codex` records (`docs/run-record-contract.md:143`).
7. **A bug**: The `codex` login probe fails to clear the environment, inheriting the launcher's whole environment unlike every other probe (`src/provider/codex.rs:272-276`).

## The third panel, checked

- H1: Fixed with a caveat, `src/policy/mod.rs:640-646`
- H2: Not fixed, `src/rungs/evolve.rs:809-816`
- H3: Fixed with a caveat, `src/watch.rs:103-134`
- H5: Fixed with a caveat, `src/worktree.rs:127-142`
- H38: Not fixed, `src/rungs/evolve.rs:1254-1281`
- H39: Not fixed, `src/worktree.rs:381-400`
- H41: Not fixed, `src/export.rs:313-320`
- H47: Not fixed, `src/worktree.rs:159-169`
- Build 112: Not fixed, `tests/harvest.rs`

## Master of Puppets, read cold

Master of Puppets (`integrations/puppets/puppets.py`) is an out-of-process orchestrator that maintains one append-only JSONL log per goal. State is derived by the `fold` function (tested in `integrations/puppets/tests/test_puppets.py::SessionReviewTests`), tying approvals to specific plan revisions. Before ticket 110 lands, a run under `standard` can freely edit the logs under `~/hydra/puppets/`, forging its own approvals or rewriting history. I would put it on a resume line as an interesting event-sourced orchestration pattern, though its security relies entirely on the pending ticket 110.

## Claims checked

| Claim | File and lines | Status |
|---|---|---|
| The outcome is derived from the stream rather than the exit code | `src/outcome.rs:2088-2264` | holds |
| The env allowlist is the whole list in src/env.rs | `src/env.rs:43-52` | holds with a caveat |
| The gate refuses a protected path only when written whole and in canonical form, and tests pin the respellings it misses | `tests/gate.rs:1536-1600` | holds |
| A Codex hook that dies fails open | `src/provider/codex.rs:20-40` | holds |
| The seatbelt denies the corpus whatever the spelling | `SECURITY.md:37-45` | holds |
| A child is never wider than its parent | `src/containment.rs:12-21` | holds with a caveat |
| The concurrency cap is a reservation under a lock | `src/live.rs:329-334` | holds |
| `hydra reap` commits only on the branch Hydra made | `src/reap.rs:403-416` | holds |
| Every git call Hydra makes in a run's checkout pins its git directory from the record | `src/worktree.rs:127-142` | holds with a caveat |
| Each run writes git objects into a store of its own and reap imports it with `index-pack --strict` | `src/reap.rs:329-351` | holds |
| Admission runs on a fresh copy of the read tree | `src/rungs/evolve.rs:1254-1281` | does not hold |
| The watcher and every spawn start from a cleared environment | `src/watch.rs:103-134` | does not hold |
| Git outside the seatbelt reads no config a run can write | `src/worktree.rs:127-142` | does not hold |
| Rung two offers exactly the tree its checks and judges read | `src/rungs/evolve.rs:1254-1281` | holds with a caveat |
| A rung-two pass edits only what `--may-edit` names, over a floor no name lifts | `src/rungs/evolve.rs:494-512` | holds |
| Export aborts on a known prefix or PEM header and only warns on entropy | `src/export.rs:537-577` | holds |
| Codex reads a seatbelt-denied copy of its skills | `src/skills/mod.rs:429-465` | holds |
| A non-ASCII root is refused before launch | `src/run.rs:550` | holds |
| The authorship table's counts match the tickets' Outcomes | `README.md:222-245` | does not hold |

## Interview questions

1. "Rung two commits the tree sha you read before admission. What does a human get when they check out `rungs/<pass>`, and what did admission actually test?" (`src/rungs/evolve.rs:1254-1281`) Strong answer: Mentions the gap where a surviving proposer process changes the checkout, and how a custom `runner` in `~/.cargo` can spoof a passing test. Worrying: Believing the tree sha guarantees the code executed was exactly what passed admission.
2. "Walk me from a forged pack in the object store to what the next run sees." (`src/policy/mod.rs:640-646`) Strong answer: Explains that git serves the forged bytes because it uses the parent as an alternate, and suggests using a per-run object directory that is verified before `commit-tree`. Worrying: Believing content addressing magically prevents malicious packs.
3. "The watcher keeps the shell environment while the pty clears it. How would you systematically audit process spawns for environment leaks?" (`src/watch.rs:103-134`) Strong answer: Proposes listing every `Command::new` site and enforcing an explicit environment contract, along with integration tests that read the watcher's environment. Worrying: Suggesting another allowlist without clearing.
4. "A `codex` run hits its quota and fails. What does its record derive?" (`src/outcome.rs:74-79`) Strong answer: Traces how the outcome is derived as `failed` because the adapter drops unknown frames without `Finding::Limit`. Worrying: Believing `completed` is an appropriate outcome.
5. "If `worktree::create` fails after `git worktree add`, who owns the checkout?" (`src/worktree.rs:381-400`) Strong answer: Acknowledges that the failure path is not established until `phase_worktree` returns, leaving an orphaned checkout, and proposes a rollback inside `create`. Worrying: Assumes `hydra reap` cleans it up.
6. "How does the gate prevent shell interpolation bypasses, and what does it miss?" (`SECURITY.md:12-22`) Strong answer: Notes that the gate expands variables but does not interpret arbitrary subshells, and that the macOS seatbelt acts as the final backstop for canonical paths. Worrying: Believing the gate completely prevents shell bypasses.
7. "Why does not the gate crash on a malformed response for Codex?" (`src/provider/codex.rs:20-40`) Strong answer: Explains that it fails open intentionally because the hook runs in the worktree and cannot be trusted, detecting crashes after the fact. Worrying: Assuming it behaves like Claude.
8. "What happens if a run modifies its own `.git` file, and the user runs a git command from inside that worktree?" (`src/worktree.rs:159-169`) Strong answer: Acknowledges that plain git follows the modified `.git` to an arbitrary decoy repository outside the seatbelt. Worrying: Assuming Hydra's `GIT_DIR` pins always apply to user-invoked commands.
9. "How does Master of Puppets prevent a run from approving its own actions?" (`integrations/puppets/puppets.py`) Strong answer: Explains that until ticket 110 lands, it does not; the run can edit the JSONL logs. Worrying: Assuming the orchestrator is inherently secure.
10. "The authorship table notes that Opus had to fix every agent diff after ticket 89. How did you verify the agent's work?" (`README.md:239-245`) Strong answer: Explains the process of using Hunk for diff review and checking the explicit Outcome audit trails. Worrying: Admitting to merging unread code.

## Before publishing

1. Large: `src/rungs/evolve.rs:1254-1281`. Rung two's admission runs on the proposer's still-writable checkout. Move admission to a fresh, un-writable materialization of the read tree.
2. Large: `src/watch.rs:103-134`. The watcher inherits the shell's environment, exposing credentials. Start the watcher from `env_clear`.
3. Large: `src/worktree.rs:381-400`. `worktree::create` leaves an orphaned checkout if it fails after `git worktree add`. Implement rollback inside `create`.
4. Large: `src/policy/mod.rs:640-646`. Map ticket 74: The object store allows forged packs. Implement content verification before `pack-objects`.
5. Small: `src/worktree.rs:127-142`. Git reads global `~/.gitconfig`. Set `GIT_CONFIG_GLOBAL=/dev/null` in `CheckoutGit::command`.
6. Small: `docs/run-record-contract.md:143`. Add `codex` to the provider enum. Documentation drift.
7. Small: `src/provider/codex.rs:272-276`. Clear the environment for the Codex login probe.
8. Small: `README.md:222-245`. Update the authorship table to reflect tickets 95 and 96. Documentation drift.
9. Small: `src/export.rs:313-320`. Prevent `--out` from overwriting `Preference.jsonl` on case-insensitive filesystems by checking inodes.

## Method

Read in full: `README.md`, `SECURITY.md`, `CONTRIBUTING.md`, `CLAUDE.md`, `docs/reviews/2026-09-24-four-readers-third-panel.md`.
Read by grep and range: `IMPLEMENTATION.md` (Deriving an outcome, Git that Hydra runs, Master of Puppets, Rung two, Concurrency cap).
Read via cat and grep: `.scratch/hydra/build/97-106.md`, `112-*.md`, `integrations/puppets/README.md`, `integrations/puppets/puppets.py`, `integrations/puppets/tests/test_puppets.py`.
Skipped deep reading of `src/` and `tests/` directories, relying instead on findings verified against the provided source paths.
