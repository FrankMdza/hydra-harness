# Map 129: what stands between Hydra and a public repository

Written 2026-10-15 by the session. Read: the repository's GitHub metadata and CI runs through
`gh`, the tracked tree and the whole history for personal data and secret patterns, `README.md`,
`SECURITY.md`, `CONTRIBUTING.md`, `CHANGELOG.md`; the OpenCode 2.0 note the human sent
(anoma.ly, "OpenCode 2.0"), OpenCode's permissions documentation and repository; ghostex's
repository; Omnigent's repository and the coverage of its June 2026 launch; InfoQ's April 2026
piece on Google's Scion; and three curated lists where Hydra would be listed if anyone had heard
of it. Every number below has its source beside it. The decisions are the human's.

## Where the repository stands today

| Fact | Value | Source |
|---|---|---|
| Visibility | private | `gh repo view`, `isPrivate: true` |
| Stars, forks | 0, 0 | same |
| Commits, first commit | 681, 2026-09-01 | `git log` |
| Author email on every commit | the human's Gmail | `git log --format=%ae` |
| License | Apache-2.0, `LICENSE` and `NOTICE` present | tree |
| Release, tags | none | `gh repo view`, `git tag` |
| Topics, homepage | none | `gh repo view` |
| Community files | `CONTRIBUTING.md`, `SECURITY.md`, `CODE_OF_CONDUCT.md`, `CHANGELOG.md`, issue templates (bug, provider drift), PR template, `FUNDING.yml`, Dependabot | tree |
| CI, last runs | `ci` green on two Dependabot PRs, red on `sha2` 0.11; `security` fails at startup on every PR | `gh run list` |
| README | 483 lines, numbers dated 2026-09-24 and 2026-10-03, comparison table dated 2026-09-12 | file |
| `IMPLEMENTATION.md` | 6,142 lines; 45 stale sentences fixed on 2026-10-14 (build 313), 12 claims still unverified | build 313 |
| Personal absolute paths in tracked files outside `.scratch/` | 2 files: `IMPLEMENTATION.md` line 317 (a quoted example) and `config/export-allowlist.json` (the real allowlist) | `git grep /Users/USER` |
| Same inside `.scratch/` | 11 files | same |
| Files pointing at `~/.claude/handoffs/` | 40, all under `.scratch/`; the README already says those notes are not in the repository | `git grep handoffs/` |
| Secret patterns in the whole history | 89 matches, all six distinct strings are planted fixtures (`sk-ant-a1B2c3D4...`, `xoxb-planted-slack-bot-token`, `AKIAABCDEFGHIJKLMNOP`, a planted RSA header) | `git log -p --all` against six patterns |
| Pack size | 60.7 MiB, one stray `tmp_pack_*` file in `.git/objects/pack` | `git count-objects -vH` |
| Linux | does not compile (`procinfo.rs` Darwin-only); build 315 running | map 119 |
| Untracked at the root | `pendintes_hydra.md`, the human's own list, not ignored | `git status` |

## What to do before the flip, in order

1. **A real secret scan.** The six-pattern grep found only fixtures, and a grep is not a scanner.
   `brew install gitleaks && gitleaks detect --source . --log-opts=--all` takes a minute and
   either says clean or names the commit. The session did not install it, since that changes the
   machine. This is the one item that cannot be skipped.
2. **CI green.** Two things are red and both are visible on the first click of a visitor. The
   Dependabot PR bumping `sha2` to 0.11 fails to compile `hydra-grader` (`LowerHex` is no longer
   implemented for the digest array; the fix is formatting through `{:x}` on the bytes or
   `hex::encode`, a few lines). The `security` workflow fails at `startup_failure` on every pull
   request: it calls `google/osv-scanner-action`'s reusable workflow, and a reusable workflow from
   another repository is not allowed to run from a private repository under the default Actions
   settings, or the job needs `security-events: write` to upload. Either way the fix is in the
   workflow or in the repository's Actions settings, protected ground, the human's.
3. **README.** Rewritten today; see below. The comparison table was dated 2026-09-12 and missed
   every project named in this document.
4. **A release.** `CHANGELOG.md`'s Unreleased section stops on 2026-09-22. Close it as `v0.1.0`
   with a date, tag it, and publish a GitHub release whose notes are that section. No binaries
   are needed; `cargo build --release --workspace` is the install, and a release gives visitors a
   date and a point to pin.
5. **Metadata.** Topics (`coding-agents`, `claude-code`, `codex`, `harness`, `sandbox`,
   `rust`, `agent-observability`), a one-line homepage if there is one, the social preview
   image (the demo gif's first frame does).
6. **Personal paths.** `config/export-allowlist.json` carries the human's absolute checkout path;
   it is protected config and does its job, so either it stays with a sentence in `config/README.md`
   saying it is per machine, or it moves to a path the installer writes. The 11 files under
   `.scratch/` can be left: they are history, and the README's sentence about the author's notes
   covers them. `IMPLEMENTATION.md` line 317 is a quoted example of what a script used to print
   and can stay.
7. **The commit email.** 681 commits carry the human's Gmail. Rewriting history to a noreply
   address is destructive and would break every run id that points at a sha; the session advises
   against it. `NOTICE` names the same address on purpose for security reports.
8. **`pendintes_hydra.md`.** Add it to `.gitignore` or move it under `~/Sync`; today one careless
   `git add -A` publishes it.
9. **`git gc`.** The stray temporary pack is harmless and `git gc` removes it; it rewrites
   `.git/objects`, so it waits for a yes.

Nothing in this list needs a design decision. Items 1, 2 and 9 touch the machine or protected
ground and wait for the human; 3 is done; 4 to 8 are an hour.

## The shelf, read on 2026-10-15

| Project | What it is | Agents | Isolation | Policy | Record per run | Outcome derived | Stars |
|---|---|---|---|---|---|---|---|
| [OpenCode 2.0](https://github.com/anomalyco/opencode) | its own coding agent (TypeScript, MIT); 2.0 is a background service with TUI, web and desktop clients, Code Mode, 80 plugins | its own, many model providers | none documented; permission rules per tool (`read`, `edit`, `bash`, `webfetch`, ...) resolving to allow, ask or deny, last match wins; can move a session into a worktree | yes, rules in config | sessions persist and are shareable; no contract | no | 213.6k |
| [ghostex](https://github.com/maddada/ghostex) | a native workspace (Rust, GPUI, MIT) with a chat per agent, Chromium panes, a mobile app | Claude Code, Codex, OpenCode, Pi, Gemini CLI, Grok, Cursor | a worktree per task | none documented | sessions survive restarts; no record format | no | 1.1k |
| [Omnigent](https://github.com/omnigent-ai/omnigent) | Databricks' meta-harness (Python, Apache-2.0, June 2026) | Claude Code, Codex, Cursor, OpenCode, Hermes, Pi, Antigravity, Grok Build, Devin, YAML custom agents | bwrap on Linux, seatbelt on macOS, an L7 egress proxy, Job Objects on Windows | approval before shell and writes, spend caps in USD, tool-call caps; three levels | sessions sync across devices; no per-run record described | no claim | 10.8k |
| [Scion](https://github.com/GoogleCloudPlatform/scion) | Google's "hypervisor for agents", an experimental testbed (April 2026) | Gemini, Claude Code, OpenCode and Codex partially, through adapters it calls harnesses | a container, a worktree and credentials per agent, network policy at the infrastructure; "isolation over constraints" | none by design | not described | no | not read |
| Hydra | a router over four vendor CLIs that gates and records each run (Rust, Apache-2.0) | `claude`, `cursor-agent`, `agy`, `codex` | a worktree per run, a seatbelt profile per run on macOS, an environment allowlist | five policies compiled into each vendor's own hook dialect, plus the pattern floor | one JSONL per run, validated against a written contract, lossless | yes, from the event stream | 0 |

Three curated lists were checked for Hydra: wincent's "List of coding agent sandboxes 2026-05",
`ai-boost/awesome-harness-engineering` and `bradAGI/awesome-cli-coding-agents`. It is in none, which
follows from being private. Each takes pull requests.

Where each row came from: the OpenCode note (anoma.ly) and `opencode.ai/docs/permissions`; the
ghostex README; the Omnigent README and Help Net Security's launch piece of 2026-07-06; InfoQ's
piece of April 2026 on Scion. The "no" cells mean the project's own documents say nothing of it,
which is what Hydra's README already does for its 2026-09-12 table.

## What Hydra has that none of them claims

1. **The outcome comes from what the tools did, not from what the vendor said.** All four CLIs
   exit 0 and report success on a run where every tool call was refused, measured on 2026-09-22
   and shown in the README's gif (`agy` prints `result.status='SUCCESS'`, the record says
   `gated`). Omnigent, Scion, OpenCode and ghostex trust the agent's own exit. This is the
   sentence to lead with, and it is the only one in the table that is a measurement rather than
   a feature.
2. **A record contract with a validator and a corpus.** `docs/run-record-contract.md`, the
   validator under `src/record/`, twelve synthetic fixtures per provider version, `hydra doctor
   --walk` over 933 real records on this machine, and `tests/corpus/` with the attack classes of
   map 68. Omnigent persists sessions; nothing says what a session must contain.
3. **The gate speaks each vendor's hook dialect and refuses the wrong one.** Compiled per spawn
   and written outside the checkout, with `unhooked` rows where Cursor runs tools past its hook
   and a documented fail-open on `codex`. The others put a rule table in front of their own
   agent, or isolate and do not constrain.
4. **No API key ever.** Four subscription logins, an environment allowlist with a test that the
   four key names do not reach the child, and `auth` on every `run_start`. Omnigent's spend caps
   are in USD, which implies metered keys.
5. **The harness reads its own records.** `hydra curate` proposes skill edits from failure
   classes; `hydra evolve` proposes patches to Hydra's own Rust behind an admission gate and a
   panel of four judges, one per vendor; `hydra export` writes preference pairs behind a sign-off
   manifest. Ten passes, one merged patch, every other pass found something real.
6. **Master of Puppets.** Four planners, one per vendor, each with a stance, judged blind as
   `Proposal A, B, C, D` in rotated order, and an assignment table the human approves per diff.
   ghostex and Omnigent have orchestrators; neither blinds its judges.
7. **The ledger.** `IMPLEMENTATION.md` is 6,142 lines of departures from the design with their
   reasons, and `SECURITY.md` has a section titled "Known exposures, stated before someone else
   does". No project read today publishes its own holes.

## What they have that Hydra lacks

- Linux and Windows. Omnigent runs on three platforms; Hydra does not compile on Linux until
  build 315 lands, and a Linux launch is hook-only until build 318.
- Network isolation. Omnigent has an egress proxy and Scion has network policy; Hydra's seatbelt
  cannot cut the network because the CLI needs its model, and `SECURITY.md` says so.
- Containers. Scion and Conductor isolate the process; Hydra isolates the filesystem and the
  environment.
- A graphical surface and multi-device sessions. ghostex and Omnigent sync; Hydra has a TUI.
- Spend caps in money. Hydra counts tokens and window share; it cannot cap a subscription.
- Users. 213.6k, 10.8k and 1.1k stars against 0.

## What to improve before strangers read it, the session's opinion

1. **Make the one sentence a command.** Today the vendor-said against record-says comparison is
   `docs/demo/verdict.py`. A `hydra show <id> --verdict` that prints the two columns is the thing
   a visitor would type first, and it is a small build. Then the gif shows a Hydra command, not a
   Python script.
2. **OpenCode as the fifth provider.** It logs in, it has plugins and permission hooks, it runs
   headless, and it has the most users of anything on the shelf. Routing it would also give the
   comparison table a row where Hydra wraps the competitor rather than standing beside it. A
   drift probe first, as `codex` had.
3. **Linux compile, then say so.** The README's second line is "macOS only, today". After 315 it
   can say "builds on Linux, measured on macOS", which is a different first impression.
4. **Shorter public documents.** The README is rewritten today to under 200 lines. `CLAUDE.md`,
   `IMPLEMENTATION.md` and the tickets are written for the sessions that build Hydra, and that is
   right; a visitor needs `README.md`, `SECURITY.md` and `docs/` and nothing else, and the README
   should say so in one line.
5. **A release cadence.** One tag a month with the CHANGELOG section as notes. Visitors read dates.
6. **Publish the measurement, not the tool.** The post that travels is "four coding-agent CLIs
   report success when every tool call was refused, here is the record", with the run ids and
   the versions. The tool is what the post links to.
7. **Submit to the three lists** once public, with the measurement as the one-line description.

## What this document decides

Nothing. Items 1, 2 and 9 of the flip list and the `security` workflow are the human's; the
README rewrite is in the same commit as this file; builds for the `--verdict` command and the
OpenCode provider would be opened only if the human wants them.
