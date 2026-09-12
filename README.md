# Hydra

One command runs `claude`, `cursor-agent` or `agy` in its own git worktree, behind a
deny-by-default policy, and leaves one JSONL record per run that says what actually happened.

![hydra run on agy: the vendor reports SUCCESS, the record says gated](docs/demo/hydra.gif)

## Why

Three coding-agent CLIs, three output formats, three permission systems, and none of them
tells you when it failed. Hydra exists because of one measurement: all three exit 0 and report
success on runs where every tool call was refused. In the demo above `agy` says
`result.status='SUCCESS'`; the record says `gated` and names the refused call.

So the record decides, not the vendor.

- **The outcome is derived from the event stream.** A run with a denied tool inside is
  `gated`, a run whose tools failed is `failed`, whatever the exit code said. Three vendors,
  one contract, one outcome column.
- **Every run gets its own git worktree.** Seeded from your tree, so the agent sees your
  uncommitted work and cannot touch it. `hydra reap` commits what it did to `hydra/<run-id>`
  and removes the checkout. Your branch never moves.
- **A policy is compiled into the vendor's own hook.** Five named policies, from `judge`
  (nothing) to `permissive`. The gate speaks each vendor's hook dialect and refuses if spoken
  to in the wrong one. Any shell command that names a protected path is refused under every
  policy. Measured detail: Cursor runs three tools without asking its hook; the record marks
  them `unhooked` rather than pretending.
- **No secret reaches the agent.** The child's environment is built from an allowlist:
  `HOME`, `USER`, `PATH`, `TERM`. The API key in your `.zshrc` stays in your `.zshrc`.
  `NODE_OPTIONS` and `PYTHONPATH` are stripped as injection paths.
- **No daemon, no database.** `~/hydra/runs/*.jsonl` is the source of truth. Nothing has to
  be running for the harness to exist, and `hydra status` reconciles live pids against open
  records on demand.
- **Records leave by one door.** `hydra export` is the only path anything leaves this
  machine by: one line per eligible run, behind a repo allowlist and a secret scan that aborts
  the whole export on one hit. The contract keeps records lossless so a training set can be
  built from them elsewhere; the SFT and preference shapes the contract describes are not
  written yet (map ticket 59).
- **It reads its own records.** Ouroboros rung one proposes skill edits from the failure
  classes it finds; rung two proposes patches to Hydra's own Rust, behind an admission gate
  and a panel of three judges, one per vendor. Nothing lands without a human merge.

## Install

macOS, a Rust toolchain, and at least one of the three CLIs installed and logged in with its
own subscription. Hydra spawns the real binaries and never touches their credentials.

```
git clone <this repo> ~/hydra && cd ~/hydra
cargo build --release
alias hydra=~/hydra/target/release/hydra
```

Hydra writes its runs, worktrees and skill store under `~/hydra/`, so the checkout and the
data root are one directory on purpose. Measured against `claude` 2.1.267, `cursor-agent`
2026.09.10 and `agy` 1.2.1; a drift test in `tests/drift.rs` fails when `claude` changes
what the gate relies on.

## Quick start

```
hydra status                                            # live 0
hydra run --provider claude --policy standard --task "add a test for the resolver"
hydra show 01a09                                        # a prefix is enough
hydra reap 01a09                                        # commit to hydra/<run-id>, remove the worktree
```

`--policy` defaults to `standard`: reads, writes, shell and network reads inside the run's own
worktree, plus the skills Hydra linked. `--timeout` degrades the call to a handle rather than
killing the child; `hydra logs <id> --follow` tails it.

## What one run does

| Step | What happens |
|---|---|
| Pre-flight | auth, concurrency cap, skill collisions, MCP servers the policy admits. Anything missing refuses the launch |
| Spawn | worktree from your tree, an immutable snapshot of the skill store linked in, the gate script compiled and written outside the checkout |
| Run | a detached process owns the pty and tees every frame, verbatim, into the record |
| End | `run_end` with the derived outcome, token usage, and the diff against the seeded tree |

## Commands

| | |
|---|---|
| `run` | launch a provider on a recorded run |
| `status` | live pids against open records; abandoned worktrees |
| `logs` | tail a record |
| `show` | the full record, normalized; `--events` for the stream |
| `resume` | continue a finished run's vendor session under a fresh run id |
| `reap` | commit a worktree to its branch, remove it |
| `prune` | move records out of `runs/`; deletes only behind a flag |
| `curate` | Ouroboros rung one: propose skill edits from the corpus |
| `evolve` | Ouroboros rung two: a proposer, the admission gate, a panel of three, a branch |
| `usage` | tokens across the corpus |
| `export` | SFT and preference JSONL, the only path anything leaves this machine by |

Two more exist and clap hides them: `watch`, the detached half, and `delegate-face`, an MCP
server that gives Claude Code a typed `delegate` tool over `hydra run`.

## Policies

`judge` < `curate`, `read-only` < `standard` < `permissive`. A child is never wider than its
parent. `policies/README.md` has the capability table; `policies/*.json` are the policies, and
every record carries the sha of the set it ran under.

## What Hydra is not

- Not a chat UI and not an agent. The vendor's model thinks; Hydra decides where it may act
  and writes down what it did.
- Not a container. The gate refuses commands that name a protected path and declines to
  interpret shell, so `P=$HOME; echo x > $P/runs/x` passes. `tests/gate.rs` breaks on purpose
  if someone "closes" that. Network cannot be cut because the CLI needs its model.
- Not a trainer. Records export losslessly; weights are trained elsewhere.
- Not Linux yet, and not tested outside this machine.

## Works with

- [Herdr](https://herdr.dev), an agent multiplexer that lives in your terminal, is the interactive layer: `integrations/herdr/` is a plugin that
  launches `hydra run` in a pane and labels the pane from the record, and a graph of one run in
  the terminal. Nothing under `src/` knows it exists.
- [Hunk](https://hunk.dev), a review-first diff viewer for agent-authored changes, is how you read a run: `hunk show hydra/<run-id>` opens what a run
  changed, and the record's `diff_stat.base_tree` separates that from what was seeded.

## Roadmap

- **Codex CLI as the fourth provider.** It signs in with a ChatGPT account, which is the
  auth model Hydra routes: a vendor binary on a subscription, never an API key. Whether it
  exposes a pre-tool hook the gate can compile into is the open question (map ticket 58).
- More vendor CLIs that log in rather than take a key.
- OpenTelemetry emission from the record, so existing viewers can read it.
- Linux.

## Status, in numbers

Eleven of eleven commands. 23 test files, green. Rung one has run once against the real
corpus and proposed one skill. Rung two has run eight passes and approved none; every pass
found something real, and `IMPLEMENTATION.md` says what. Read that file before trusting a
record: it is every departure from the design with its reason.

## Design

`docs/harness-spec.md` is how Hydra launches, feeds and records the providers.
`docs/run-record-contract.md` is the format it writes and Ouroboros reads.
`docs/ouroboros-spec.md` is both rungs of self-improvement. They are meant to be read
together, and nothing crosses between the harness and Ouroboros except the run record and
the skill store.

`.scratch/hydra/map.md` is the design history: 57 decision tickets with the reasoning each
document compressed. `CLAUDE.md` is what an agent reads first.

## Contributing and support

`CONTRIBUTING.md` says how a change gets in and what is not open to one. `SECURITY.md` says
what the gate promises and what it deliberately does not. The most useful thing to report is
provider drift, and there is an issue template for it.

If Hydra saves you a bad afternoon, the sponsor button on this repository is how to say so.
Vendor tokens are the one cost this project cannot avoid.

## License

Apache-2.0. See `LICENSE` and `NOTICE`.
