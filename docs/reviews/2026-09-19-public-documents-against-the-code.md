# Public documents against the code

Written by `cursor-agent` on Composer 2.5, run 01a0bcdb-3071 under `standard`, 2.8M tokens.
Every finding below is fixed in the commit that publishes this report, so the detail stays.

Date: 2026-09-19. Method: read `README.md`, `CLAUDE.md`, `SECURITY.md`, `CONTRIBUTING.md`, `CHANGELOG.md`, `policies/README.md`, `config/README.md`, and `docs/reviews/README.md`; spot-check every claim in them that names a number, file, command, flag, version, or count against `src/`, `tests/`, `policies/*.json`, `config/`, `.github/`, and `Cargo.toml`; read `IMPLEMENTATION.md` and `.scratch/hydra/build/*.md` outcomes where README cites them; build the binary and run `hydra status` with `HYDRA_ROOT` set; run `claude --version`, `cursor-agent --version`, and `agy --version` on this machine. Did not run the full test suite.

## Findings

| Id | Rating | Summary |
|---|---|---|
| F1 | wrong | README says `HYDRA_ROOT` moves everything; policies stay at the checkout |
| F2 | wrong | README commands table says export writes SFT and preference JSONL |
| F3 | wrong | README cites map ticket 59 for missing SFT export shapes |
| F4 | wrong | `config/README.md` says `mcp.md` declares no server |
| F5 | wrong | README omits `HYDRA_HOME`, the variable that moves `policies/` and `config/` |
| F6 | stale | `IMPLEMENTATION.md` counts six evolve passes; README counts eight |
| F7 | stale | `IMPLEMENTATION.md` provider version table lags README and this machine |
| F8 | stale | README measured provider versions lag the binaries on this machine |
| F9 | wording | README lists four env vars; `src/env.rs` passes more per provider |
| F10 | wording | README policy ordering with `<` hides that `curate` and `standard` are incomparable |

## F1. `HYDRA_ROOT` does not move everything

README line 67: "`HYDRA_ROOT=/some/dir` moves everything for one command."

`src/paths.rs` lines 73 to 85: `HYDRA_ROOT` overrides only `Layout::discover`, which sets the root for runs, worktrees, skills, and spawn. `hydra_home` in lines 24 to 50 reads `HYDRA_HOME` when set, otherwise `CARGO_MANIFEST_DIR`, and `PolicySet::discover` loads policies from `hydra_home().join("policies")`.

Measured: `HYDRA_ROOT=/tmp/hydra-test-root ./target/release/hydra status` printed `root /tmp/hydra-test-root` and `policies` still under this worktree checkout.

Believe the code. README line 67 is wrong for policies and config. Line 68 is closer but still incomplete without naming `HYDRA_HOME`.

## F2. Export output is not SFT and preference JSONL

README line 108: "`export` | SFT and preference JSONL, the only path anything leaves this machine by."

README lines 39 to 41, same file: "the SFT and preference shapes the contract describes are not written yet."

`src/export.rs` lines 307 to 323: each eligible run becomes one JSON line `{"events": [...]}` plus a header `export count=... allowlist_sha=...`. No `sft.jsonl`, no `preference.jsonl`, no second file.

Believe the code and README line 39. Line 108 is wrong; a reader would look for two training files the binary never writes.

## F3. Map ticket 59 is not about SFT export shapes

README line 41: "the SFT and preference shapes the contract describes are not written yet (map ticket 59)."

`.scratch/hydra/map.md` line 287: ticket 59 is "The three documents against the code", a read-only audit of doc claims.

Believe the map. README misattributes the gap. The gap is real; the ticket number is not.

## F4. `mcp.md` declares a server

`config/README.md` line 12: "`mcp.md` | #5, the canonical MCP document | yes, and it declares no server."

`config/mcp.md` lines 60 to 72: an active `[[server]]` block names `hydra`, stdio transport, `delegate-face`, admitted under `standard` and `permissive`.

Believe `mcp.md`. Build ticket 26 added this block; `IMPLEMENTATION.md` lines 2581 to 2587 say so. `config/README.md` is wrong.

## F5. `HYDRA_HOME` moves policies and config; README never names it

README lines 67 to 68 mention compile-time resolution and `HYDRA_ROOT` but not `HYDRA_HOME`.

`src/paths.rs` lines 24 to 37: "`HYDRA_HOME` when set, otherwise the compile-time `CARGO_MANIFEST_DIR`."

`.scratch/hydra/build/50-the-binary-is-not-relocatable.md` outcome lines 22 to 27: ticket 50 added `HYDRA_HOME` and `hydra status` prints the policies path.

Believe the code and build 50 outcome. A reader who clones away from `~/hydra` and sets only `HYDRA_ROOT` still reads policies from the checkout unless they also set `HYDRA_HOME`.

## F6. Evolve pass count in `IMPLEMENTATION.md` is low

README line 185: "Rung two has run eight passes and approved none."

`IMPLEMENTATION.md` lines 2716 and 3232: "Six passes have run on this machine."

`docs/ouroboros-spec.md` line 33: "eight passes have run as of 2026-09-12."

Believe README and `docs/ouroboros-spec.md`. Pass 7 and pass 8 appear in map tickets 57 and 56. `IMPLEMENTATION.md` has not been updated since pass six.

## F7. Provider version table in `IMPLEMENTATION.md` lags README

README lines 68 to 69: "Measured against `claude` 2.1.267, `cursor-agent` 2026.09.10 and `agy` 1.2.1."

`IMPLEMENTATION.md` lines 78 to 82: `claude` 2.1.263, `cursor-agent` 2026.09.02-c22c1a3, `agy` 1.1.26 then 1.1.27, all dated 2026-09-06.

Believe README as the newer public claim. `IMPLEMENTATION.md` is stale relative to README. Neither file is checked into the repo as a live probe; `tests/drift.rs` re-measures behavior, not version strings.

## F8. README measured provider versions lag this machine

README lines 68 to 69 name the versions above.

This machine on 2026-09-19: `claude` 2.1.268, `cursor-agent` 2026.09.18-9a7762b, `agy` 1.2.3.

Believe the version commands. README's measurement date is implicit and already stale for two of three vendors that self-update. Rating stale, not wrong, because README frames them as a measurement rather than a runtime check.

## F9. Environment allowlist is broader than four names

README lines 31 to 32: "The child's environment is built from an allowlist: `HOME`, `USER`, `PATH`, `TERM`."

`src/env.rs` lines 21 to 82: `BASE` is those four; `claude` also receives `SHELL`, `TMPDIR`, and `NODE_EXTRA_CA_CERTS` when set; all providers receive proxy variables from `PROXY` when set; Hydra may set `CURSOR_DATA_DIR` and `HYDRA_RUN_ID` via `ChildEnv::set`.

Believe `src/env.rs`. The secret-scrubbing claim holds: `NODE_OPTIONS` and `GEMINI_API_KEY` do not reach the child, covered in tests at lines 174 to 194. The four-name list is an oversimplification, not a false deny list.

## F10. Policy ordering in README is not a chain

README line 115: "`judge` < `curate`, `read-only` < `standard` < `permissive`."

`src/policy/mod.rs` lines 1093 to 1101, test `curate_and_standard_are_incomparable`: neither `curate` nor `standard` contains the other under `Policy::contains`.

Believe the code. The two comma-separated chains do not falsely claim `curate` < `read-only`, but a reader may treat `<` as total order. Wording, not a numeric error.

## Checked and aligned

| Claim | Where checked | Result |
|---|---|---|
| Eleven commands on the public surface | `src/cli.rs` lines 34 to 207, `docs/harness-spec.md` section 17 | match |
| Hidden `watch` and `delegate-face` | `src/cli.rs` lines 210 to 224 | match |
| `--policy` defaults to `standard` on recorded runs | `src/main.rs` line 43 | match |
| 23 test files | `tests/*.rs` glob | 23 files |
| Rust 1.88, edition 2024 | `Cargo.toml` lines 4 to 5, README line 59 | match |
| Five named policies plus `floor.json` | `policies/*.json`, `src/policy/mod.rs` line 240 | match |
| `standard` grants read, write, shell, network_read, skill_invoke | `policies/standard.json` | match |
| Three Cursor tools marked unhooked | `SECURITY.md` lines 18 to 19, `src/policy/table.rs` lines 149 to 160 | match |
| Shell variable bypass still allowed | `README.md` line 134, `SECURITY.md` lines 12 to 15, `tests/gate.rs` lines 1737 to 1762 | match |
| `WriteShellStdin` and `send_command_input` denied | `SECURITY.md` lines 32 to 38, `tests/gate.rs` lines 2312 to 2332 | match |
| `HYDRA_RUN_ID` containment marker | `SECURITY.md` lines 40 to 48, `src/containment.rs` line 38 | match |
| Auth pre-flight specified, not built | README line 89, `src/record/lines.rs` line 240 | match |
| Export refuses without allowlist file | `config/README.md` lines 11 to 12, `src/export.rs` lines 36 to 39 | match |
| `config/export-allowlist.json` absent; example ships | `config/` directory | match |
| CONTRIBUTING test recipe matches CI | `CONTRIBUTING.md` lines 22 to 26, `.github/workflows/ci.yml` lines 21 to 24 | match |
| Corpus walks honor `HYDRA_ROOT` | `CONTRIBUTING.md` lines 33 to 36, `tests/common/mod.rs` lines 868 to 873 | match |
| Who wrote the code table vs build outcomes | tickets 17 to 38, 50, 51, 60 outcomes | match; 25 of 60 agent-written |
| Ticket 27 token count 54.7M | `.scratch/hydra/build/27-interactive-claude-and-harvest.md` lines 69 to 70 | match |
| `llms.txt` exists | repo root | present |
| Provider drift issue template | `.github/ISSUE_TEMPLATE/provider-drift.md` | present |
| CHANGELOG build ticket 40 admission | `CHANGELOG.md` line 10, build ticket 40 | match |
