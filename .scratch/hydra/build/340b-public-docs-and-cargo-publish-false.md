# Build 340b - Public documents and `Cargo.toml` `publish = false`

Status: claimed
Size: haiku

## Goal

Update `Cargo.toml`, `README.md`, `CONTRIBUTING.md`, `CHANGELOG.md`, `CLAUDE.md`, `docs/install.md`, `docs/roadmap.md` and `docs/who-wrote-the-code.md` so that every public document in the repository matches the codebase at build 361 and the root package is marked `publish = false`. Do not touch `SECURITY.md`, `IMPLEMENTATION.md`, or any `.rs` file. Follow `~/.claude/skills/unslop/SKILL.md`: no em dashes in new prose.

## What to change

1. **`Cargo.toml`:**
   - Under `[package]`, add `publish = false` right after `version = "0.3.0"`.

2. **`README.md`:**
   - **Line 29:** Clarify that a run's git worktree is seeded from your tree under `<parent>/.hydra-worktrees/<run-id>/` for an external repo and `~/hydra/worktrees/<run-id>/` for `~/hydra` itself.
   - **Lines 37 to 39:** Note that `src/env.rs` is tested across all four providers, `Provider::Codex` and `OPENAI_API_KEY` included since build ticket 359.
   - **Lines 83 to 85:** Replace `The interactive mode, where the vendor draws its own interface and Hydra harvests the transcript afterwards, is claude and codex today, and only claude takes a policy there.` with: `The interactive mode, where the vendor draws its own interface and Hydra harvests the transcript afterwards, runs ungated on all four providers since builds 255, 266 and 267, and only claude takes --policy there until build ticket 130.`
   - **Line 103:** Replace `the only path anything leaves this machine by` with `the only path by which corpus records leave this machine`.
   - **Line 109:** Replace `with --session claude the ungated chat in the left panel` with `with --session claude or --session codex the ungated chat in the left panel`.
   - **Lines 137 to 139 and 152 to 153:** Soften `What is not on the shelf is the record.` and `Hydra has the record` so it states what Hydra writes and replays without claiming no other tool writes a record: at line 137 say `What Hydra adds is a contract-checked record per run whose outcome is derived from the event stream and replayed against the run's compiled gate.`, and at line 152 say `Hydra writes the contract-checked record and derives the outcome from the stream`.
   - **Lines 171 to 177:** Update the `## Status` table and paragraph below it:
     - `Test files`: `41 under tests/, 25 under crates/*/tests/, run with cargo nextest`
     - `Records on the author's machine`: `997 on 2026-10-27, every one checked by hydra doctor --walk`
     - `Build tickets`: `356 opened, 353 resolved, every resolved one ending in an ## Outcome`
     - `Design tickets`: `134 on the map, 122 resolved`
     - At line 177, change `6,000 lines of them` to `over 6,400 lines of them`.

3. **`CONTRIBUTING.md`:**
   - **Lines 18 to 19:** Add a sentence warning that `hydra show <run-id> --events` prints raw event content without running `hydra export`'s twelve-class secret redactor, so inspect the output before pasting it into a public issue.
   - **Lines 23 to 30:** Update the four commands and prose to match builds 126, 296 and 305:
     ```
     cargo fmt --all --check
     cargo clippy --workspace --all-targets -- -D warnings
     cargo build --release --workspace
     cargo nextest run --release --workspace
     cargo test --release --workspace --doc
     ```
     Explain that the test suite runs in parallel since build ticket 296 and reads the checked-in synthetic fixtures under `tests/fixtures/records/`, while `hydra doctor --walk` walks the live records under `~/hydra/runs/` since build ticket 126.
   - **Lines 34 to 40:** Update the paragraph to say `hydra doctor --walk` walks every `.jsonl` under `~/hydra/runs/` (or `$HYDRA_ROOT/runs`), while the integration tests in `tests/` (`record_contract.rs`, `normalize.rs`, `outcome.rs`) walk `tests/fixtures/records/`.
   - **Lines 45 to 47:** Replace `A test that sets a process-wide variable belongs in an integration test file of its own with a lock around it, the way tests/git_config.rs does` with: `No test mutates the process environment since build ticket 296; pass environment variables on the child Command instead of calling std::env::set_var.`
   - **Lines 51 to 54:** Change `The five protected configs` to `The six protected configs of docs/harness-spec.md section 15.1: policies/, the environment allowlist, the export allowlist, config/preamble.md, config/mcp.md and config/panels.json (build ticket 138).`

4. **`CHANGELOG.md`:**
   - Under `## Unreleased`, add a dated bullet `- 2026-10-27.` summarizing builds 338, 339, 348 to 361, 340a, 340b and 341 with their build numbers and run ids:
     - `codex` 0.160.0's `webrun` is a hooked `network_read` row in the capability table, replayed from `web_search` `open_page` items with a fixture in the corpus and a `Limit` finding when an `open_page` item carries no string URL (builds 338, 353, 354, 355; runs `01a1609d-7626`, `01a160ac-ed7f`, `01a160be-72cd`, `01a161e9-0f35`).
     - Every denied tool call appends a `gate_refusal` row (`arguments_sha256` when there is no shell command), `hydra show --events` wraps `gate_refusal.reason` inside the result boundary marker, and `Outcome::Gated` plus `walk::check_gated_write_denials` count a `gate_refusal` row in Hydra's voice when no stream call got through (builds 339, 356, 360; runs `01a142ae-f739`, `01a17c8a-3c0b`, `01a17ccf-c6a7`).
     - Relative paths in MCP tool arguments resolve against the run's `cwd` before the protected-path check (build 348, run `01a142af-2a26`).
     - The pattern floor refuses a bare `git push` from a protected branch, `git push origin HEAD` and `@`, `-C`/`--git-dir`/`--work-tree` before `push`, `-c` push refspecs and `push.default=matching`, `--config-env`, and `refs/heads/` branch prefixes (builds 349, 357; runs `01a142af-5e94`, `01a17c8c-342a`).
     - `claude` 2.1.301's `Workflow` maps to `subagent_spawn`, `LSP` stays unknown and refused, and a `hook_started` frame one `seq` before its call joins by order so the final `WebFetch` no longer reads `Ungated` (build 350, run `01a160a8-e887`).
     - `pairs` keeps only answers recorded in `attempts.jsonl` (build 351, run `01a15fe9-7a33`), cutover tests retry `ETXTBSY` on Linux (build 352, run `01a16088-20d3`), `hydra doctor` runs `/usr/bin/python3 -c "import json, shlex"` (build 358, run `01a17c9e-b5b7`), `src/env.rs` tests `Provider::Codex` and `OPENAI_API_KEY` (build 359, run `01a17ca3-d339`), and `hydra doctor --walk` classifies all 59 `RejectedBeforeHook` rows by tool and error shape (build 361, run `01a17cba-1b5f`).
     - Public documentation and `IMPLEMENTATION.md` brought in line with build 361 (builds 340a, 340b, 341; run `01a17cd1-f166`).

5. **`docs/install.md`:**
   - **Line 3:** Mention `/usr/bin/python3` with standard-library `json` and `shlex` alongside Rust 1.88+, and note that `hydra doctor` checks `/usr/bin/python3` since build ticket 358.
   - **Lines 23 to 29:** Correct the foreign-checkout claim: since build ticket 67, launching from a checkout that is not `~/hydra` refuses unless `HYDRA_ROOT` is set, rather than silently reading and writing `~/hydra`.
   - **Lines 35 to 36:** Add `OPENAI_API_KEY` to the list of API keys tested in `src/env.rs` across all four providers (build ticket 359).
   - **Lines 80 to 83:** Update `## Ports` to state that Linux compiles and runs the test suite in CI since builds 315 and 331, while an OS sandbox on Linux waits on build ticket 318 (bubblewrap, map 119).

6. **`docs/roadmap.md`:**
   - **Lines 6 to 9 (`Linux`):** Update to state that builds 315 and 316 are resolved (Hydra compiles on Linux, runs CI on `ubuntu-latest`, and stamps `host` and `sandbox_kind`), and build 318 (the bubblewrap backend) is what remains before Linux launches are sandboxed.
   - **Lines 10 to 14:** Delete the two bullets for builds 319 to 321 and builds 322 to 324, since all six build tickets are resolved.
   - **Lines 22 to 23 (`hydra show --verdict`):** Update to note that build 335 took `--verdict` for the puppets goal-log verdict, so a command comparing the vendor's own verdict beside the record's needs a different flag name (`docs/demo/verdict.py` today).

7. **`docs/who-wrote-the-code.md`:**
   - **Lines 40 to 42:** Add a sentence after build 312's report noting that build ticket 328 stopped reading pasted tables as ticket attributions and widened the outcome phrase matcher over 329 tickets (`131 applied as is, 34 corrected, 13 finished by hand, 9 rejected and 142 unsaid`).

8. **`CLAUDE.md`:**
   - **Lines 4 to 5:** Change `hermes was a fourth until ticket 22 unrouted it` to `hermes was a fifth in the early map until ticket 22 unrouted it`.
   - **Lines 13 to 14:** Remove `and which of the three has never written one` from the end of the sentence, since all four routed providers write records.
   - **Line 28:** Update the status sentence so it reflects the current map and build numbers (`The map stands at 122 resolved of 134 tickets on 2026-10-27, and the build stands at 353 resolved of 356 tickets, with 130, 294 and 318 open.`).
   - **Lines 95 to 97:** Update `rungs/ is rung one, hydra curate` to `rungs/ holds both rungs of self-improvement, hydra curate and hydra evolve`.
   - **Lines 100 to 101:** Update to state that `preamble.md`, `mcp.md`, `panels.json` and `export-allowlist.json` live in `config/` while the environment allowlist lives in `src/env.rs` and `export-signoff.json` is optional.

## Done when

- Only `Cargo.toml`, `README.md`, `CONTRIBUTING.md`, `CHANGELOG.md`, `CLAUDE.md`, `docs/install.md`, `docs/roadmap.md`, `docs/who-wrote-the-code.md`, and `.scratch/hydra/build/340b-public-docs-and-cargo-publish-false.md` are modified.
- No em dashes (`—`) are introduced.
- `git diff --check` passes.
