# Hydra

One command runs `claude`, `cursor-agent`, `agy` or `codex` in its own git worktree, behind a
deny-by-default policy, and leaves one JSONL record per run that says what actually happened.

macOS only, today. The four vendor binaries exist on Linux and the port is small; the section
on ports says what it needs.

![hydra run on agy: the vendor reports SUCCESS, the record says gated](docs/demo/hydra.gif)

## Why

Four coding-agent CLIs, four output formats, four permission systems, and none of them
tells you when it failed. Hydra exists because of one measurement: all four exit 0 and report
success on runs where every tool call was refused, measured again on 2026-09-22 on the versions
`hydra doctor` prints. In the demo above `agy` says `result.status='SUCCESS'`; the record says
`gated` and names the refused call.

So the record decides, not the vendor.

- **The outcome is derived from the event stream.** A run whose every tool call the gate
  refused is `gated`. One refused call among calls that ran is a `permission_decision` on the
  record, not a run outcome. A run that reached its own terminator is `failed` when that frame's
  `is_error` is true, `agy`'s `status` is not `SUCCESS` or `codex` ended on `turn.failed`, and
  `completed` otherwise; a `codex` turn that failed on its usage limit is `rate_limited`. A harvested
  run has no terminator and goes by its exit code, `completed` when there is none; a streamed run
  that stopped without one is `failed` or `crashed`. A failed tool call does not fail the run.
  Four vendors, one contract, one outcome column.
- **Every run gets its own git worktree.** Seeded from your tree, so the agent sees your
  uncommitted work and cannot touch it. `hydra reap` commits what it did to `hydra/<run-id>`
  and removes the checkout. If the checkout is on another branch, reap refuses and leaves it
  and its spawn directory intact. Your branch never moves.
- **A policy is compiled into the vendor's own hook.** Five named policies, from `judge`
  (nothing) to `permissive`. The gate speaks each vendor's hook dialect and refuses if spoken
  to in the wrong one. Any shell command that spells a protected path is refused under every
  policy; one that assembles the path is not, and `SECURITY.md` says what walks past. Measured
  detail: Cursor runs three tools without asking its hook; the record marks them `unhooked`
  rather than pretending. And on `codex` a gate that crashes before it answers lets the call
  run, measured again on 2026-09-24; `SECURITY.md` says why Hydra accepts that.
- **The agent's environment is an allowlist, not your shell.** The child's environment is built
  from `HOME`, `USER`, `PATH`, `TERM`, plus `SHELL` and `TMPDIR` for `claude`,
  `NODE_EXTRA_CA_CERTS` for the two Node providers, the proxy variables when set, and the names
  Hydra sets itself: the run id, `CURSOR_DATA_DIR` on `cursor-agent`, and the run's own git
  object directories. `src/env.rs` is the whole list. The API key exported in your `.zshrc`
  stays out of the child's `env`, and since build ticket 97 out of the watcher that owns its pty.
  `NODE_OPTIONS` and `PYTHONPATH` are never on the list. What the agent can read off the disk
  under `standard` is a different question, and `SECURITY.md` answers it.
- **No daemon, no database.** `~/hydra/runs/*.jsonl` is the source of truth. Nothing has to
  be running for the harness to exist, and `hydra status` reconciles live pids against open
  records on demand.
- **Records leave by one door.** `hydra export` is the only path Hydra itself sends anything
  off this machine by: one line per eligible run, and `preference.jsonl` beside it since build
  ticket 65, behind a repo allowlist, a human sign-off manifest and a secret scan that aborts
  before either file is created. The vendor CLI talks to its own API on every run, and the
  record shows what it was sent. The contract keeps records lossless so a training set can be
  built from them elsewhere; the SFT shape the contract describes is not written yet, and
  training itself is out of scope (map decision D12).
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

Hydra never handles an API key. You log in once with each vendor's own CLI, and that stored
login is the only way a run authenticates. The child's environment is built from an allowlist,
and a test in `src/env.rs` checks that `ANTHROPIC_API_KEY`, `GEMINI_API_KEY`, `CURSOR_API_KEY`
and `CURSOR_AUTH_TOKEN` do not reach it. The `run_start` line of every provider launch carries an `auth` block with
`mechanism: subscription-oauth`, `source: keychain` and `allow_billed: false`. Hydra writes
those three values itself, the same for all four providers, from the fact that it passed no
key (`src/run.rs`). It does not ask the vendor where the login is stored or which pool the
run bills to, so read the block as what Hydra did, not as what the vendor charged.

| Provider | Login | What Hydra does about it |
|---|---|---|
| `claude` | its own login, read from the macOS keychain | passes the real `HOME` and never sets `CLAUDE_CONFIG_DIR`, which would lose the login |
| `cursor-agent` | its own login; its init frame reports `apiKeySource: "login"` | passes no Cursor key or token |
| `agy` | its Google login | never sets `AGY_ADC_AUTH` and passes no Google key |
| `codex` | `codex login` on a ChatGPT plan | runs `codex login status` before the launch and refuses when it says logged out |

The record's `subscription` field names who pays: `anthropic`, `cursor`, `google` or `openai`.

```
git clone https://github.com/FrankMdza/hydra-harness ~/hydra && cd ~/hydra
cargo build --release --workspace   # Rust 1.88 or newer; --workspace also builds hydra-puppets
mkdir -p ~/.local/bin && ln -sf ~/hydra/target/release/hydra ~/.local/bin/hydra
ln -sf ~/hydra/integrations/copilot/bin/masterofpuppets ~/.local/bin/masterofpuppets
hydra doctor            # its "hydra on PATH" line should say "this binary"
```

Check that `~/.local/bin` is on your `PATH` (`echo $PATH`), and add it to your shell profile if
not. A shell alias is not enough. The delegate face is spawned by the vendor's CLI, which sees
`PATH` and knows nothing about your aliases. Hydra writes its own path into a run's face config,
so a run works either way, but `hydra` typed at a prompt needs the symlink.

**Hydra writes its runs, worktrees and skill store under `~/hydra/`, wherever the checkout
lives.** The install path above makes the two the same directory on purpose. Clone it somewhere
else and the binary still reads and writes `~/hydra`, which is somebody else's corpus if they
have one: `hydra status` prints the root it read on its first line and the policy directory
on its second. `HYDRA_ROOT=/some/dir` moves the runs, worktrees and skill store for one
command; `HYDRA_HOME=/some/checkout` moves `policies/` and `config/`, which otherwise resolve
through the source directory at compile time, so the binary is not relocatable without both.
Measured against `claude` 2.1.268, `cursor-agent` 2026.09.18-9a7762b, `agy` 1.2.8 and `codex`
0.154.0 on 2026-09-22, which is what `hydra doctor` compares the installed binaries against.
`tests/drift.rs` re-measures what the `judge` launch relies on in `claude` and whether `codex
login status` still answers; the hook dialects themselves are measured by hand and recorded by
run id in `IMPLEMENTATION.md`.

## Quick start

```
hydra status                                            # live 0
hydra run --provider claude --policy standard --task "add a test for the resolver"
hydra show 01a09                                        # a prefix is enough
hydra reap 01a09                                        # commit to hydra/<run-id>, remove the worktree
```

`--policy` defaults to `standard`: reads, writes, shell and network reads inside the run's own
worktree, plus the skills Hydra linked. `hydra run` prints one JSON object with the `run_id`.
`--timeout` defaults to `10m` and degrades the call to a handle rather than killing the child;
`hydra logs <id> --follow` tails it. `--timeout 0` launches and returns at once, so several
agents can run side by side, and `hydra wait <id>...` blocks until they have all ended.

```
masterofpuppets ~/some-repo       # the orchestrator: a chat that runs a goal through plan, table and diffs
hydra cockpit                     # the viewer: tasks, runs by state, one run's detail
```

All four providers take a recorded run. The interactive mode, where the vendor draws its own
interface and Hydra harvests its transcript afterwards, is narrower.

| Provider | Recorded run | `--interactive`, ungated | `--interactive --policy` | Copilot chat |
|---|---|---|---|---|
| `claude` | yes | yes, transcript harvested | yes | yes, with `Edit`, `Write` and `NotebookEdit` disallowed |
| `codex` | yes, not under `read-only` | yes since build ticket 255, session file harvested | refused | yes, and it keeps its edit tools |
| `cursor-agent` | yes | refused | refused | no |
| `agy` | yes | refused | refused | no |

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
| `wait <run>... [--any] [--timeout 30m]` | block until selected runs end; exit 2 on timeout |
| `logs` | tail a record |
| `show` | the full record, normalized; `--events` for the stream |
| `resume` | continue a finished run's vendor session under a fresh run id |
| `stop` | end a live run, `--all` of them, or `--session` and its seats; the record says `stopped` |
| `reap` | commit a worktree to its branch, remove it |
| `prune` | move records out of `runs/`; deletes only behind a flag |
| `curate` | rung one: propose skill edits from the corpus |
| `evolve` | rung two: a proposer, the admission gate, a panel of four, a branch |
| `usage recount` | tokens across the corpus, recounted from the records |
| `export` | one JSON line of events per eligible run, and `preference.jsonl` beside it; the only path anything leaves this machine by |
| `approve` | approve a seat request no shape covers; with no id, list what waits |
| `deny` | deny one; the session hears `hydra/request-denied` and nothing launches |
| `doctor` | the root, the binaries against the measured versions, the policy sha, which protected configs are missing |
| `cockpit` | one goal's tasks, the runs in four lanes, one run's detail, in a full-screen view that refreshes itself; answers seat requests, approves Puppets steps, stops seats; with `--session claude` or `--session codex`, that provider's ungated chat in the left panel; `COCKPIT.md` at the repo root is the help file a session can read when you ask from the overlay |

`hydra cockpit` has three views. The main one has three panels. The left one is
the task table of a Master of Puppets goal (id, status, done, provider, model, run, grade and
class, dropped from the right as the pane narrows; once a task's run ends, status shows the
run's outcome). The middle one sorts every run into `live`, `waiting approval`, `ended ok` and
`ended not ok`, with the seat requests and Puppets steps that wait on a human in the second.
A run's row carries its id, provider, model, goal task, role, policy, tokens and elapsed time, and
the model is the recorded one, else the one the provider's init frame names, else `default`.
The right one shows what you select: a run's task, or its question when it is a panel seat,
then its outcome, tokens and final text, then its last fifteen tool calls, with a
secret-flagged call's input hidden and other calls shown as a short head until `i` reveals the
whole input. `Enter` opens a run as a full page, the second view. `t` opens the third, the
goal's table as a full page: the revision, whether the plan and the table are approved, each
panel's verdict and its source, every seat's provider, model, role and vote, and per task its
text, class, grade, provider, model, policy, deps, status, run and launch approval. At 80
columns or fewer the three panels stack. The header's dragon counts the live seats of the
watched session (every live run without one), and lists each live provider's tokens per
minute and share of its window. The cockpit rereads the disk every two seconds and, without
`--session`, launches no provider. Run labels are the shortest unique prefix of at least eight
characters.

| Key | Does |
|---|---|
| arrows, `Tab`, `Shift-Tab` | move within a panel, or between panels |
| `Enter`, `Esc` | open the selected run as a full page, or a request or step in the detail pane; `Esc` closes or leaves the page |
| `Left`, `Backspace` | on a full-page run, back one step |
| `t` | open the shown goal's task table as a full page; `t` again or `Esc` returns |
| `PageUp`, `PageDown`, `Space`, `b`, `Home`, `g`, `End`, `G` | scroll the detail or full-page run by a screen, or to either end |
| `k` | swap the four lanes by state for one lane per agent, the waiting approvals last; with `--session`, only the session and its seats |
| `r` | swap the detail between tool calls and the raw last twenty record lines |
| `i` | reveal the whole input of the selected call or line |
| `a`, `y` | approve the selected seat request, or run the argv `hydra-puppets pending` listed for a step |
| `d`, `y` | deny the selected seat request; a step has no deny |
| `v`, `y` | start `hydra-puppets review` on the selected task: three judge runs, which count against quota |
| `m`, `y` | stop the selected live seat, the same stop as `hydra stop`; with `--session`, leave an unsent note about it in the chat |
| `p` | policy pane: the set's sha and the live runs' shas; `e` opens the selected file in `$EDITOR` |
| `?` | help: every key the cockpit answers; `Esc` or `?` closes; with `--session`, `a` there types a question about `COCKPIT.md` into the chat, unsent |
| `F2`, `Ctrl-q` | with `--session`: `F2` moves the keys between the chat and the cockpit; `Ctrl-q` quits once the chat has exited |
| `q` | quit |

Each write waits for a `y`: a seat approval or denial, which is what `hydra approve <id>` or
`hydra deny <id>` writes; a Master of Puppets approval, through the `hydra-puppets` helper; a stop.
The cockpit runs the `hydra-puppets` beside its own binary, else the one on `PATH`. That helper
is the only writer of a goal log, and the cockpit writes no policy file, since the editor is the
human's. Approvals, denials and stops refuse inside a seatbelt. The mouse clicks,
selects and scrolls.

```
hydra cockpit                    # the goal log that changed last
hydra cockpit --goal <goal-id> --interval 5
hydra cockpit --session claude --model claude-opus-5-5
```

With `--session claude` or `--session codex` the chat moves inside: the cockpit runs `hydra
run --interactive` for that provider, with `--model` passed on, and draws it in the left panel
while the table, lanes and detail stack on the right. The chat is ungated and recorded. Since
build ticket 254 the cockpit takes no `--policy`: it exits 2 and names the replacement, `hydra
run --interactive --provider <p> --policy <name>`, which is the gated chat and is `claude`
only. `F2` moves the keys between the chat and the cockpit, and with the chat focused every
key goes to it, `q` included. `Ctrl-q` quits once the chat has exited; while it is alive the
cockpit asks, and `y` hangs it up. When the chat exits, its last screen stays and the panel's
border names the run and its outcome.

Two more exist and clap hides them: `watch`, the detached half, and `delegate-face`, an MCP
server that gives Claude Code a typed `delegate` tool over `hydra run`.

## Local model

`hydra-local-llm` is a Rust stdio MCP server with one tool, `ask`, backed by local Ollama.
The workspace release build above produces `target/release/hydra-local-llm`. To admit it,
the human adds this block to the fenced TOML in `config/mcp.md`, adjusting `command` to
the binary path on their machine:

```toml
[[server]]
name = "local-llm"
transport = "stdio"
command = "/absolute/path/to/hydra/target/release/hydra-local-llm"
args = []
admit = ["permissive"]
```

See [the local model documentation](integrations/local-llm/README.md) for environment
variables, tool arguments and tests.

## Policies

`judge` < `read-only` < `standard` < `permissive`, and `judge` < `curate`, which grants a
different set and is neither inside nor outside `standard`. A child is never wider than its
parent. `docs/harness-spec.md` section 7.2 has the capability table and `src/policy/table.rs`
is its code; `policies/*.json` are the policies, and
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

A `claude` mod or plugin is JavaScript inside the `claude` process, and one that approves tool
calls can approve a call the gate's hook refused. Build ticket 251 found no flag in `claude`
2.1.288 that drops mods and keeps the hook, so the block has two halves. The human installs
`/Library/Application Support/ClaudeCode/managed-settings.json`, root-owned, with
`allowManagedModsOnly`, and `claude` then refuses every mod that is not the organization's.
Hydra does not write or check that file. Hydra's half, build ticket 257, keeps a run from
planting a plugin for a later launch. The gate refuses a file-tool write to
`~/.claude/plugins`, `~/.claude/skills`, the worktree's `.claude/skills`,
`.claude/settings.json`, `.claude/settings.local.json` and `.agents/plugins`, and to any path
with a `.claude-plugin` component, under every policy. On macOS the seatbelt profile denies
the same paths to every process of the run, shell included.

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
Rows other than Hydra's come from each project's own docs as read on 2026-09-12.

| | Works with several vendor CLIs | Worktree per run | Record per run, contract-checked | Gate compiled per run |
|---|---|---|---|---|
| [ccmanager](https://github.com/kbwo/ccmanager) | yes | yes | no | no |
| [vibe-kanban](https://github.com/BloopAI/vibe-kanban) | yes | yes | no | no |
| [Cupcake](https://github.com/eqtylab/cupcake) | yes | no | no | yes |
| [Langfuse coding-agent tracing](https://langfuse.com/resources/engineering/coding-agent-tracing) | yes | no | no | no |
| Hydra | yes, four | yes | yes, outcome derived from the stream | yes, five policies |

Where each "yes" comes from, re-read on 2026-09-24. A "no" means the project's README says
nothing of it, which cannot be quoted.

- ccmanager: "Run multiple AI assistant sessions in parallel across different Git worktrees", for
  Claude Code, Gemini CLI, Codex CLI, Cursor Agent and five more. Its experimental auto-approval
  asks Claude Haiku whether a prompt needs a human, which is not a compiled gate.
- vibe-kanban: the README lists Claude Code, Codex, Gemini CLI, Cursor and six more, and its
  worktrees show only in a variable that disables "git worktree cleanup". The project now says it
  is sunsetting.
- Cupcake: "Cupcake intercepts agent events and evaluates them against user-defined rules written
  in Open Policy Agent (OPA) Rego", for Claude Code, Cursor, Factory AI and OpenCode.
- Langfuse: traces Claude Code, OpenAI Codex, Cursor and six more, and says of itself "treat
  hook-based tracing as telemetry, not enforcement".

ccmanager has the live dashboard and container isolation Hydra lacks. Cupcake has the policy
language. Langfuse has the UI and cross-run search. None derives the outcome from what the tools
did, which is the one thing Hydra was built around.

## Works with

The orchestrator chat is `integrations/copilot/bin/hydra-copilot [model] [repo] [provider]`,
which `masterofpuppets` starts in its Master of Puppets mode. It starts `hydra cockpit --session` on `claude`, or on `codex` when asked, from
`~/.hydra-copilot/`, outside any repository, where a short document rendered with the
repository's path replaces that repository's own instructions. The repository is any git work
tree, and this checkout when none is named. The chat is ungated and every agent it launches is
gated. It launches them with `hydra run --timeout 0`, waits with `hydra wait`, and reads them
with `hydra show`. On `claude` the chat runs with `Edit`, `Write` and `NotebookEdit`
disallowed, so file changes go through `hydra run`; it keeps its shell. The default model on
`claude` is Opus, and `codex` keeps its own default. Map ticket 97 measured one Opus copilot
on 2026-10-03, 44 minutes and five agents: 12.5M tokens against the agents' 10.6M, 1.18 times,
or 1.09 times with cache reads weighted at a tenth. The ticket's own bar, a copilot that costs
less than its agents, is not met.

Master of Puppets runs on any repository as `masterofpuppets <repo> [model] [provider]`. It
is the same copilot started from `~/.hydra-puppets-copilot/` with the `puppets` skill
installed, and `hydra-puppets` (`crates/puppets/`) keeps the goal's log and refuses a step the
human has not approved: the plan, the assignment table and each diff. The skill seats four
planners, one per provider, and each gets its own stance appended to its brief: `minimal`,
`risk`, `contrarian` or `builder`. `choose --judge` hands the proposals to the judges as
`Proposal A`, `B` and so on, each seat reading them in a different order, and maps the answers
back to providers afterwards. On a split, `claude` and `codex` break the tie under `judge`,
the second reading in reverse order, and the verdict is theirs only when the two agree.

- [Herdr](https://herdr.dev), an agent multiplexer that lives in your terminal, is optional:
  `integrations/herdr/` is a plugin that launches `hydra run` in a pane and labels the pane
  from the record, and a graph of one run in the terminal. Hydra runs with Herdr stopped, and
  `hydra cockpit` shows the same runs without it. Two things in the Rust still name it:
  `src/cockpit/mascot.rs` reads the dragon's frames from `integrations/herdr/mascot/frames/`,
  and `hydra-puppets` opens a review in a Herdr pane when it runs inside one
  (`crates/puppets/src/panel.rs`).
- [Hunk](https://hunk.dev), a review-first diff viewer for agent-authored changes, is how you read a run: `hunk show hydra/<run-id>` opens what a run
  changed, and the record's `diff_stat.base_tree` separates that from what was seeded.

## Roadmap

- **Codex under `read-only`.** Routed since build ticket 48 under
  `standard` and `permissive`, on the judge panel since 61, writing through `apply_patch` since
  69 because the gate reads the paths out of the patch body. It has no read tool, so map ticket
  63 refuses `read-only` until a bounded shell read exists. Resume is supported since build
  ticket 71.
- A gated interactive session on `codex`, `cursor-agent` and `agy`. Build ticket 130 is open,
  and `integrations/herdr/interactive-gate-providers.txt` lists `claude` alone as measured.
- More vendor CLIs that log in rather than take a key.
- OpenTelemetry emission from the binary. Today `hydra-otel`, a separate binary, converts a
  record to OTLP JSON files.
- Linux.

## Ports wanted

Hydra is measured on macOS and nowhere else. A Linux port is small: `setsid`, the process
start time the liveness test reads, and the four vendor binaries all exist there, so most of
the work is running `tests/drift.rs` and fixing what it names. A Windows port is larger: the
pty crate supports ConPTY, but the liveness test, the gate's shell parsing (POSIX today) and the
hook paths all assume a Unix shell. Both are welcome as forks or pull requests; open an issue
first so the design map gets a ticket and the record contract does not fork with the code.

## Status, in numbers

Seventeen commands a human types on 2026-10-03, the command table above, and two hidden ones.
38 Rust test files under `tests/` and six under `crates/*/tests/`, plus the Python suites
under `integrations/`. The
Master of Puppets helper is Rust, `hydra-puppets`; the Python it was ported from was deleted on
2026-10-01, after its output on every test input was recorded as the fixture the helper is now
compared with. Since builds 252 and 262 the helper's judge and plan prompts differ from that
Python on purpose, so the recorded comparison skips those cases and unit tests cover them. The
grader is Rust too, `hydra-grader`, and its Python was deleted the same way (build ticket 220);
it reads a goal by running `hydra-puppets show`. Still Python: the Herdr plugin, the apprentice,
the OTLP translator and the local-LLM server under `integrations/`, with the record scan and the
redactor the apprentice and the translator load. CI never walks the live corpus under
`~/hydra/runs/`; that is `hydra doctor --walk` on the machine that launched the runs. The fixture
walks in `cargo test` run the same assertions over `tests/fixtures/records/`, twelve synthetic
records filed by the provider version whose shapes they reproduce, and the CI job summary prints
how many fixtures it checked. A green CI says the readers handle every shape in the fixtures, not
that every record on this laptop still meets the contract. Rung one has
run once against the real corpus and proposed one skill. Rung two has run ten passes and
offered one patch, a two-word one, on the tenth, and a human merged it; the other nine found
something real, and `IMPLEMENTATION.md` says what. Read that file before trusting a record:
it is every departure from the design with its reason.

## Who wrote the code

Part of Hydra was written by the agents Hydra launches, through `hydra run`, while a Claude
Code session wrote the tickets, reviewed each worktree diff and applied it to master by hand.
The corpus in `~/hydra/runs/` and the git history are the sources for this table, which covers
the 122 build tickets resolved by 2026-09-24, 01 to 122. 80 has no row of its own, because
build ticket 96 closed it. Each ticket an agent wrote names its run id and token count under
`## Outcome`.

| Written by | Build tickets |
|---|---|
| `cursor-agent` on Composer 2.5 | 20, 24, 28, 29, 31, 32, 34, 36, 48, 50, 51, 60, 61 to 75, 77 to 79, 86, 87, 90, 92, 93, 108, 110, 111, 113, 114, 116 to 122 |
| `cursor-agent` on Grok 4.6 Fast | 21, 30, 33, 35, 37, 38 |
| `cursor-agent`, no model flag, so the record says none | 17 |
| `claude` on Sonnet 5 | 18, 22, 23, 26, 27 |
| `claude` on Haiku 4.5 | 25, 82 |
| `claude` on Opus 5.5 | 91, 96 |
| `agy` on Gemini 3.1 Pro | 19, 94, and the drift test of 17 |
| `agy` on Gemini 3.8 Flash | 115 |
| `codex`, no model flag, so the record says none | 81, 83, 85, 89, 95, 109 |
| a Claude Code subagent on Opus 5.5, in a git worktree of its own and not through `hydra run` | 97 to 106, 112 |
| the Claude Code session, by hand | 01 to 16, 39 to 47, 49, 52 to 59, 76, 84, 88, 107 |

On 77, 78, 81, 82, 85, 86 and 87 an Opus subagent fixed the agent's diff in the worktree before
the session applied it, and on 82 it rebuilt Haiku's version. From 89 to 96 an Opus subagent
reviewed each diff and the session made the fixes on master; every one of those eight needed at
least one, and on 92 a second Composer run made them. That is 72 of the 121 build tickets on an
agent run through `hydra run`, none of them merged unread. Tickets 97 to 106, the third hiring
panel's, went to subagents working on the source directly. Two Opus reviews of their merged code
opened 105 and 106, and 112 holds what those two left. Ticket 27, the
interactive mode and its harvest, was the largest single run at 54.7M tokens, 54.0M of them
cache reads. Ticket 25 was the first Rust ticket given to the cheapest model, and it held.
The design documents, the policies, the record contract and this README are the session's.

## Design

`docs/harness-spec.md` is how Hydra launches, feeds and records the providers.
`docs/run-record-contract.md` is the format it writes and the two rungs read.
`docs/rungs-spec.md` is both rungs of self-improvement. They are meant to be read
together, and nothing crosses between the harness and the rungs except the run record and
the skill store.

`.scratch/hydra/map.md` is the design history: 103 decision and research tickets on
2026-10-03, with the reasoning each document compressed. `.scratch/hydra/build/plan.md`
indexes the build, 264 tickets on the same day, 245 of them resolved, and every resolved one ends in an `## Outcome`
that says what the session changed in the agent's diff before applying it.
`hydra-puppets frontier` and `hydra-puppets frontier build` print
today's counts. `CLAUDE.md` is what an agent reads first.

Many tickets point at `~/.claude/handoffs/...`, and some at paths under `/Users/USER`. Those are
the author's private working notes on the author's own machine: session handoffs, panel briefs, launch
scripts and raw proposals. They are not in this repository, and nothing here needs them to build or
to run. Where a ticket's reasoning depended on one, the ticket or its research file says what it
found.

## Contributing and support

`CONTRIBUTING.md` says how a change gets in and what is not open to one. `SECURITY.md` says
what the gate promises and what it deliberately does not. The most useful thing to report is
provider drift, and there is an issue template for it.

If Hydra saves you a bad afternoon, the sponsor button on this repository is how to say so.
Vendor tokens are the one cost this project cannot avoid.

## License

Apache-2.0. See `LICENSE` and `NOTICE`.
