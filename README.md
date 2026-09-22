# Hydra

One command runs `claude`, `cursor-agent`, `agy` or `codex` in its own git worktree, behind a
deny-by-default policy, and leaves one JSONL record per run that says what actually happened.

macOS only, today. The four vendor binaries exist on Linux and the port is small; the section
on ports says what it needs.

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
  `HOME`, `USER`, `PATH`, `TERM`, plus `SHELL` and `TMPDIR` for `claude`, the proxy
  variables when set, and the two names Hydra sets itself. `src/env.rs` is the whole list.
  The API key in your `.zshrc` stays in your `.zshrc`. `NODE_OPTIONS` and `PYTHONPATH` are
  stripped as injection paths.
- **No daemon, no database.** `~/hydra/runs/*.jsonl` is the source of truth. Nothing has to
  be running for the harness to exist, and `hydra status` reconciles live pids against open
  records on demand.
- **Records leave by one door.** `hydra export` is the only path anything leaves this
  machine by: one line per eligible run, behind a repo allowlist and a secret scan that aborts
  the whole export on one hit. The contract keeps records lossless so a training set can be
  built from them elsewhere; the SFT and preference shapes the contract describes are not
  written yet, and training itself is out of scope (map decision D12).
- **It reads its own records.** Rung one (`hydra curate`) proposes skill edits from the
  failure classes it finds; rung two (`hydra evolve`) proposes patches to Hydra's own Rust,
  behind an admission gate and a panel of four judges, one per vendor. Nothing lands without
  a human merge. Ten passes so far and one offerable patch, on 2026-09-21, the first time all
  four judges said pass; before it, one critical fail from any judge vetoed every patch, by
  design, and every pass found something real. The loop was called Ouroboros
  until 2026-09-21; [razzant/ouroboros](https://github.com/razzant/ouroboros) is earlier,
  unrelated work under that name, so this one is the rungs.

## Install

macOS, a Rust toolchain, and at least one of the four CLIs installed and logged in with its
own subscription. Hydra spawns the real binaries and never touches their credentials; that is
the documented path for scripted use on all four, and `SECURITY.md` says what their terms
still forbid.

```
git clone https://github.com/FrankMdza/hydra-harness ~/hydra && cd ~/hydra
cargo build --release   # needs Rust 1.88 or newer, edition 2024
alias hydra=~/hydra/target/release/hydra
```

**Hydra writes its runs, worktrees and skill store under `~/hydra/`, wherever the checkout
lives.** The install path above makes the two the same directory on purpose. Clone it somewhere
else and the binary still reads and writes `~/hydra`, which is somebody else's corpus if they
have one: `hydra status` prints the root it read on its first line and the policy directory
on its second. `HYDRA_ROOT=/some/dir` moves the runs, worktrees and skill store for one
command; `HYDRA_HOME=/some/checkout` moves `policies/` and `config/`, which otherwise resolve
through the source directory at compile time, so the binary is not relocatable without both.
Measured against `claude` 2.1.267, `cursor-agent` 2026.09.10 and `agy` 1.2.1 on 2026-09-15;
a drift test in `tests/drift.rs` fails when `claude` changes what the gate relies on.

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
| Pre-flight | the binary is on PATH, the concurrency cap, skill name collisions, and the MCP servers the policy admits. Anything missing refuses the launch. An auth check is built for `codex` only, so a logged-out `claude`, `cursor-agent` or `agy` fails in its own words rather than Hydra's |
| Spawn | worktree from your tree, an immutable snapshot of the skill store linked in, the gate script compiled and written outside the checkout, and on macOS a seatbelt profile the vendor binary runs under |
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
| `curate` | rung one: propose skill edits from the corpus |
| `evolve` | rung two: a proposer, the admission gate, a panel of four, a branch |
| `usage recount` | tokens across the corpus, recounted from the records |
| `export` | one JSON line of events per eligible run, and `preference.jsonl` beside it; the only path anything leaves this machine by |
| `doctor` | the root, the binaries against the measured versions, the policy sha, which protected configs are missing |

Two more exist and clap hides them: `watch`, the detached half, and `delegate-face`, an MCP
server that gives Claude Code a typed `delegate` tool over `hydra run`.

## Policies

`judge` < `read-only` < `standard` < `permissive`, and `judge` < `curate`, which grants a
different set and is neither inside nor outside `standard`. A child is never wider than its
parent. `policies/README.md` has the capability table; `policies/*.json` are the policies, and
every record carries the sha of the set it ran under.

The policy is compiled into the vendor's hook rather than written into the prompt, and two
outside measurements say why. ["When 'Do Not' Is Not Deny"](https://arxiv.org/abs/2608.23550)
read 481 public `CLAUDE.md` files and found a built-in control behind 4.4% of their security
rules under the strictest match, 16% under the loosest; an instruction is not a guardrail.
["Before the Tool Call"](https://arxiv.org/abs/2603.20953), the Open Agent Passport paper,
authorizes each action before it runs and signs a record of the decision, and in a live
adversarial testbed measured 0% attack success under a restrictive policy against 74.6%
under a permissive one. Hydra's gate is that second idea carried down into the CLIs people
already use, one compiled script per spawn in each vendor's hook dialect.

## What Hydra is not

- Not a chat UI and not an agent. The vendor's model thinks; Hydra decides where it may act
  and writes down what it did.
- Not a container. The gate refuses commands that name a protected path and declines to
  interpret shell, so `P=$HOME; echo x > $P/runs/x` walks past the gate; on macOS the kernel's
  seatbelt around the run refuses that write anyway, measured on all four providers, and
  `tests/gate.rs` breaks on purpose if someone claims the gate itself closed it. Network cannot
  be cut because the CLI needs its model, and Linux runs without the seatbelt.
- Not a trainer. Records export losslessly; weights are trained elsewhere.
- Not Linux yet, and not tested outside this machine.

## How it compares

Worktree-per-run over several vendor CLIs is a crowded shelf. What is not on it is the record.

| | Runs several vendor CLIs | Worktree per run | Record per run, contract-checked | Gate compiled per run |
|---|---|---|---|---|
| [ccmanager](https://github.com/kbwo/ccmanager) | 9 | yes | no | container and auto-approve |
| [vibe-kanban](https://github.com/BloopAI/vibe-kanban) | 10+ | yes | no | no |
| [Cupcake](https://github.com/eqtylab/cupcake) | 4, via hooks | no | no | yes, OPA/Rego, richer than Hydra's |
| [Langfuse coding-agent tracing](https://langfuse.com/resources/engineering/coding-agent-tracing) | 3 | no | traces, hosted, no outcome | no |
| Hydra | 4, `codex` under two policies | yes | yes, outcome derived from the stream | yes, five policies, three dialects |

ccmanager has the live dashboard and container isolation Hydra lacks. Cupcake has the policy
language. Langfuse has the UI and cross-run search. None derives the outcome from what the tools
did, which is the one thing Hydra was built around.

## Works with

- [Herdr](https://herdr.dev), an agent multiplexer that lives in your terminal, is the interactive layer: `integrations/herdr/` is a plugin that
  launches `hydra run` in a pane and labels the pane from the record, and a graph of one run in
  the terminal. Nothing under `src/` knows it exists.
- [Hunk](https://hunk.dev), a review-first diff viewer for agent-authored changes, is how you read a run: `hunk show hydra/<run-id>` opens what a run
  changed, and the record's `diff_stat.base_tree` separates that from what was seeded.

## Roadmap

- **Codex under `read-only` and `judge`.** Routed since build ticket 48 under `standard` and
  `permissive`, measured on 0.154.0 with the gate refusing a protected path on a real run. It
  has no read tool, so the two narrower policies wait on map ticket 63, and under `standard`
  its `apply_patch` is refused because the tool names no path; writes go through the shell.
- More vendor CLIs that log in rather than take a key.
- OpenTelemetry emission from the record, so existing viewers can read it.
- Linux.

## Ports wanted

Hydra is measured on macOS and nowhere else. A Linux port is small: `setsid`, the process
start time the liveness test reads, and the three vendor binaries all exist there, so most of
the work is running `tests/drift.rs` and fixing what it names. A Windows port is larger: the
pty crate supports ConPTY, but the liveness test, the gate's shell parsing (POSIX today) and the
hook paths all assume a Unix shell. Both are welcome as forks or pull requests; open an issue
first so the design map gets a ticket and the record contract does not fork with the code.

## Status, in numbers

Eleven of eleven commands. 23 test files, green. Rung one has run once against the real
corpus and proposed one skill. Rung two has run ten passes and offered one patch, a two-word
one, on the tenth, and a human merged it; the other nine found something real, and
`IMPLEMENTATION.md` says what. Read that file before trusting a
record: it is every departure from the design with its reason.

## Who wrote the code

Part of Hydra was written by the agents Hydra launches, through `hydra run`, while a Claude
Code session wrote the tickets, reviewed each worktree diff and applied it to master by hand.
The corpus in `~/hydra/runs/` is the source for this table, and each ticket's `## Outcome`
names the run id and the token count.

| Written by | Build tickets |
|---|---|
| `cursor-agent` on Composer 2.5 | 20, 24, 28, 29, 31, 32, 34, 36, 48, 50, 51, 60 |
| `cursor-agent` on Grok 4.6 Fast | 21, 30, 33, 35, 37, 38 |
| `cursor-agent`, no model flag, so the record says none | 17 |
| `claude` on Sonnet 5 | 18, 22, 23, 26, 27 |
| `claude` on Haiku 4.5 | 25 |
| `agy` on Gemini 3.1 Pro | 19, and the drift test of 17 |
| the Claude Code session, by hand | 01 to 16, 39 to 47, 49, 52 to 59 |

That is 26 of 60 build tickets on an agent run, none of them merged unread. Ticket 27, the
interactive mode and its harvest, was the largest single run at 54.7M tokens, 54.0M of them
cache reads. Ticket 25 was the first Rust ticket given to the cheapest model, and it held.
The design documents, the policies, the record contract and this README are the session's.

## Design

`docs/harness-spec.md` is how Hydra launches, feeds and records the providers.
`docs/run-record-contract.md` is the format it writes and the two rungs read.
`docs/rungs-spec.md` is both rungs of self-improvement. They are meant to be read
together, and nothing crosses between the harness and the rungs except the run record and
the skill store.

`.scratch/hydra/map.md` is the design history: 62 decision tickets with the reasoning each
document compressed. `CLAUDE.md` is what an agent reads first.

## Contributing and support

`CONTRIBUTING.md` says how a change gets in and what is not open to one. `SECURITY.md` says
what the gate promises and what it deliberately does not. The most useful thing to report is
provider drift, and there is an issue template for it.

If Hydra saves you a bad afternoon, the sponsor button on this repository is how to say so.
Vendor tokens are the one cost this project cannot avoid.

## License

Apache-2.0. See `LICENSE` and `NOTICE`.
