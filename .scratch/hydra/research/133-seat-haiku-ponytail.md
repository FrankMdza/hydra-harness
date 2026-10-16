## 1. What ponytail is

Ponytail is a prompt skill, not a program. Its core is one file, `skills/ponytail/SKILL.md`, plus a compact `AGENTS.md` for agents that read rules files. It tells the agent to write only what the task needs and never to cut validation, error handling, security or accessibility. https://github.com/DietrichGebert/ponytail

Installs go through a Claude Code plugin, a Codex plugin, an npm package, or a copied rule file. The Pi listing at https://pi.dev/packages/@dietrichgebert/ponytail says its package ships an extension that can execute code. The npm page returned 403 to me.

The claimed problem is over-building. The v5 README reports 53 percent fewer lines across 39 tasks. The v4.9.0 README reports 54 percent across 12 tasks, four runs per arm. Both are the maintainer's own numbers. https://cdn.jsdelivr.net/npm/@dietrichgebert/ponytail@4.9.0/README.md

## 2. Maturity

MIT licence, copyright 2026 DietrichGebert. https://github.com/DietrichGebert/ponytail/blob/main/LICENSE

The repository shows 164.5k stars and 8.9k forks. That is high for a project whose first release dates from 12 June 2026, and I could not explain it. https://github.com/DietrichGebert/ponytail

26 releases across three pages, from v1.0.0 to v5.2.0 on 13 October. https://github.com/DietrichGebert/ponytail/releases

347 commits, newest on 15 October 2026. https://github.com/DietrichGebert/ponytail/commits/main

Eight open issues. https://github.com/DietrichGebert/ponytail/issues

Contributor count unreadable.

A `tests/` directory exists. `test.yml` runs on push and pull request. `publish.yml` publishes to npm on tags. https://github.com/DietrichGebert/ponytail/tree/main/.github/workflows

## 3. Attachment points in Hydra

**Injected skill, the best fit.** A human copies `SKILL.md` into `~/hydra/skills/ponytail/` with a `hydra.toml` recording origin and pin. `src/skills/mod.rs` links it into the worktree from a per-sha snapshot the run cannot write, after a collision check. `standard` and `permissive` grant `skill_invoke`. The record carries `skills.store_sha`, `skills.linked` and `skills.resolved`. No run writes the store.

**MCP server, not a fit.** `src/mcp.rs` writes a per-run `mcp.json` from the canonical document, which is protected config 5 and which a human edits. The top-level tree shows no MCP server, though release notes mention one. Under `standard`, `mcp_call` is granted for the `hydra` server only, so calls to a ponytail server would be refused.

**Tool on PATH, not recommended.** `src/env.rs` passes the human's `PATH`, so the tool would need an absolute path and would run through `shell`.

**Hook, not recommended.** `src/spawn_config.rs` writes one gate hook per run. A ponytail hook would sit beside it in a protected config. I did not confirm whether Claude loads plugin hooks under Hydra's strict settings.

## 4. Trial

Use eight to twelve identical tickets on one base commit, each with hidden tests. Compare no skill against the pinned `SKILL.md`, with the same provider, model and policy, alternating arm order. Run at least five per arm, since the maintainer used four and five.

Measure lines changed from `git diff --stat` on the branch, since the record does not carry size. Add hidden-test pass rate and wall time. `skills.linked` and `skills.resolved` show which arm got the skill and what the model saw. `skills.store_sha` pins the bytes. Tokens come from `usage`. I did not read contract sections 5.1 or 7.8 in full, so the usage field names are unverified.

## 5. Risks

**Supply chain.** The npm package is small with no dependencies, but its extension runs code. Installing the package or the repository brings manifests for about fifteen other agents. Copy only `skills/ponytail/SKILL.md`, at a commit sha, after reading it.

**Data egress.** I did not read the hook or extension scripts, so I cannot say what leaves the machine. `/ponytail-gain` suggests local statistics, which is unverified.

**Licence.** MIT fits Apache-2.0 if the copyright notice travels with the copied file. That is my reading, not a legal review.

## Pending from me

Decide whether to run the trial. Someone with npm access should read the package page, since it returned 403 here. Contributor count and usage field names remain open.

## Changed

Wrote this memo to `.scratch/hydra/research/133-seat-haiku-ponytail.md`. Nothing else changed, nothing is committed, and no file from the ponytail repository was cloned or run.

## Found

The two benchmarks differ in task count and model, so do not merge them. Release notes mention an MCP server that the top-level tree does not show. The star count is far above what the first release date suggests.

`src/mcp.rs` says `mcp_call` is granted by `permissive` alone, but `policies/standard.json` grants it for the `hydra` server. One of the two is stale, and I did not decide which.
