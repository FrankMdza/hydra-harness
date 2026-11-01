# 134 synthesis: what is left before the repository goes public

Panel of 2026-11-01, sat because the human asked for ticket 134 to move forward with a panel. Seats, all under `standard`, read only. The prompt carried the session's measurement for map 99, taken outside any seatbelt.

- Opus, `claude-opus-5-5`, run `01a198bc-2c3d`: 99 (a), 110 and 114 stated, no new fix-before.
- GPT, codex `gpt-6-luna`, run `01a198bc-e2c4`: 99 (a), 110 stated, 114 fix before.
- Flash, agy `gemini-3.8-flash-high`, run `01a198be-4391`: 99 (a), 110 and 114 stated, no new fix-before.
- Grok, cursor `grok-4.7-high`, run `01a198bd-909d`, ended `auth_failed` and gave no answer. Three seats split nowhere evenly, so no judge sat.

## The measurement behind question 1

On 2026-11-01 the session ran `claude` 2.1.312 under `(allow default)` plus a write deny on `^~/.claude\.json` and, in the last launch, on `^~/.claude/history\.jsonl`. A shell write to the file under the same profile was refused. `claude -p` answered. An interactive session through a pty answered in `~/hydra`, and in a fresh `/tmp` directory it showed the trust dialog, took "Yes" and answered. Neither file's sha nor mtime changed. A session longer than a minute and a launch through `hydra run --interactive` were not measured.

## Question 1, map 99: (a), deny both files, three of three

The reason build 250 left `~/.claude.json` writable was that nobody knew whether `claude` starts without it, and it does. Opus found that the fix is a deletion: `copilot_block` already denies writes under the whole home directory (`src/sandbox.rs:869`), and the regex allow loop at `src/sandbox.rs:898-906` is what carves the two files back out. Against (b), Flash noted that `CLAUDE_CONFIG_DIR` is in `NEVER_SET` in `src/env.rs`, because setting it changes how `claude` finds its keychain entry. Against (c), Opus noted that it detects the server after the next ungated session may already have started it.

What it costs the human, from Opus and Flash: a gated session in a fresh worktree path asks for trust once per session, its prompts never reach up-arrow history in later sessions, and `/mcp` approvals or `claude mcp add` made inside it do not persist. Opus asked for one more measurement as the build's acceptance: a gated session of about 30 minutes typed by the human, with both files' sha compared before and after.

`history.jsonl`, map 99's third question, goes with it. The same measurement covered it.

## Question 2, maps 110 and 114: state both and publish with them open

- 110, three of three. Nothing under `src/` sends a diff off the machine. `crates/grader/src/jev.rs:1` is the probe without `ask`, and `hydra export` writes a local file and starts no process. `SECURITY.md` already says map 110 governs any return of `jev ask`.
- 114, two against one. `hydra bench` refuses without `--dry-run` at `src/main.rs:303-308`, pinned in `tests/bench.rs`, so the open scoring cannot produce a wrong number today. GPT voted fix before, because the scoring still counts `completed` as an attack that got through. The majority's answer is that code nobody can run without `--dry-run` publishes no rate.

## Question 3, the audit: no new fix-before, two of three

The seats checked fifteen claims between them: the environment allowlist, the seat socket limits, the MCP name and relative-path refusals, `TIOCSTI`, the approval store deny, the Linux admission refusal, the private git sweep, `Workflow` mapped to `subagent_spawn`, and gated interactive mode being `claude` only. They held. GPT's "yes" named maps 99 and 114, both already answered above, and no new claim.

Two stale sentences, found by Opus and checked by the session:

1. `SECURITY.md:422-423`: "`hydra bench` counts a `completed` run as a successful attack". Since `src/main.rs:303` the bench refuses to launch rather than miscount.
2. `SECURITY.md:81-82`: a gated session "writes nothing under your home directory outside its worktree and `claude`'s session state". The profile also allows `~/Library/Caches/claude-cli-nodejs` (`src/sandbox.rs:888-891`), which the sentence does not name.

Both are prose and go into the map 99 build, which rewrites `SECURITY.md:76-85` anyway.

## Question 4, what the human is handed

`config/mcp.md` lines 40 to 43, merged from the three seats and the session's draft:

> It decides which runs are handed the server. It does not decide whether they may call it: the
> delegation policy does that. `permissive` grants `mcp_call` for every server, and `standard`
> for `hydra` alone. So a server admitted to `standard` under any other name starts, its tools
> reach the model, and the gate refuses every call to it. Admitting a server to a policy that
> cannot call it costs a process and some context, and buys nothing.

Topics, eight, taking the ones at least two seats named and filling from the rest: `agent-harness`, `ai-agents`, `claude-code`, `codex-cli`, `cursor-agent`, `sandboxing`, `macos-seatbelt`, `rust`.

Homepage: none, three of three. The README is the page.

## Tickets

1. Map 99 resolved with (a).
2. Build 378: drop the two regex allows, flip the tests that pin them, rewrite `SECURITY.md:76-85` and `:422-423`, amend `IMPLEMENTATION.md`. Acceptance includes the human's long gated session.
3. Maps 110 and 114 stay open, marked fix after in 134's answer.
