# 10 - Route `agy`

Type: build
Status: open
Blocked by: 03, 08

## Goal

The third provider: `-p=` with the equals sign, `--add-dir` carrying three separate
things, the hook config that fails closed, and its rows in the normalizer.

## Where the design says it

- Harness spec 6.7's `agy` block, including why a bare `-p` silently drops the prompt
  and why `--dangerously-skip-permissions` is a launch decision rather than a run-time
  one.
- 6.4, 6.5 and 6.6 for the three things `--add-dir` carries, without which `agy` reports
  `hooks: []` rather than an error.
- 6.5 for the inherited-server residue this provider keeps, and why Hydra never calls
  `agy mcp disable`.
- 6.6 for `overwrite`, which is forbidden, and for the first-deny short circuit.
- Contract 7.3 for `text_delta`, which is a buffered flush wearing a streaming name, and
  9.2 for `result` usage being per conversation rather than per run.

## Done when

- A recorded `agy` run writes a record that passes the contract checker.
- The normalizer concatenates `text_delta` over `step_index` to produce one `message`,
  reproducibly from `raw`.
- `usage` comes off the step's `DONE` frames, not off `result`, on a resumed run.
- The three unverified facts in harness spec 19.3 are either measured or still carried
  as caveats, and the ticket says which.

## Watch for

**`agy` writes where it likes.** With `--dangerously-skip-permissions` a probe that wrote
`probe.txt` put it in `~/.gemini/antigravity-cli/scratch/`, not the launch directory.

**It has moved four times in four days.** Re-read the version and re-measure rather than
inherit. Ticket 26 probed the hook gate on 1.1.24 and ticket 30 re-measured on 1.1.25.

**A permission denial and a file-not-found get the same `state: ERROR` and the same
`error.type: TOOL_ERROR`, and both messages contain the word "permission".** The adapter
returns `unclassified` here rather than guessing. Guessing produces a curator that
rewrites a skill because a file was missing.

## What ticket 03 left you, added 2026-09-05

**The seam is `normalize::read_with(text, policies, layout)`, and it dispatches on
nothing today.** `src/normalize/mod.rs` holds the ten types and calls
`claude::normalize` directly, because one provider is routed. Your column needs an
`agy.rs` beside it and a branch on the envelope's `provider`, which every line carries
(contract 4).

**Two fields on the types exist for you.** `Message::block` is documented as the
`step_index` your concatenation runs over, and `Counts::cache_write_tokens` and
`Counts::thinking_tokens` are `Option` because your provider reports one and not the
other. `Counts::total_tokens()` deliberately does not match your own `total_tokens`, and
the reason is in its doc comment: yours excludes cache read, so copying it would make a
warm-cache run look cheap.

**`usage` is one event per run and you have per-step counts.** 9.2 says a run's usage is
the sum of its own `step_update` frames and that `result.usage` is recorded and not used
for it. The `claude` adapter emits its single `Usage` event when it reaches the `result`
frame; yours has to sum first and emit once, and the `seq` field on `Usage` is an
`Option<u64>` that will not name a single frame honestly. Say so in the outcome rather
than pointing it at the last step.

**`ResultStatus::Unclassified` is written and reached.** 7.6's warning about your
`TOOL_ERROR` is already the code's behaviour, and the `claude` adapter reaches the same
value from a different direction: a successful `Write` with no `is_error` field.

## What ticket 07 left you, added 2026-09-06

**Your resolved listing is free and it is one command.** Ticket 24:
`agy -p "/skills" --output-format json --add-dir <worktree>` returns `num_turns: 0`,
`total_tokens: 0` and one record per surviving skill. Fill in
`Provider::resolved_skills`, beside `skill_root` (`.agents/skills`, which needs
`--add-dir <worktree>` or it is invisible) and `user_skill_roots`
(`~/.gemini/config/skills` and nothing else).

**You are the provider that keys on the frontmatter `name`**, and the store's validator
now requires the frontmatter name to equal the directory name, so a store skill has one
identity for you and for `claude` both. `hydra::skills::index` keys on both anyway,
because the user's own skills are not held to that rule.

**You collapse two skills declaring one name and `claude` does not**, which ticket 24
measured. The launch check will see that as a linked name you did not list, which is a
collision, which is the right answer.

## What ticket 08 left you, added 2026-09-06

**The canonical document exists and your file is generated from it.** `config/mcp.md` is
protected config #5: Markdown prose around exactly one fenced `toml` block, one `[[server]]`
table per server, carrying `name`, `transport` (`stdio`, `http` or `sse`), `admit` as a list of
policy names, and then `command`/`args`/`env` or `url`/`headers`. `hydra::mcp::Document` parses
it, `hydra::mcp::Server` is the shape you generate from, and `hydra::mcp::write_config` matches
on the provider to pick both the path and the body. Yours is `<worktree>/.agents/mcp_config.json`
with `serverUrl` on a remote server, and it needs `--add-dir <worktree>` or it is invisible.

**The document already refuses the pair you cannot express.** A server naming both a command and
a url fails at parse time, with your name in the error, so no document can ask you to write one.

**Nothing calls `agy mcp disable` and nothing here changed that.** 6.5's inherited-server residue
is yours alone now: `claude` closes it with `--strict-mcp-config`, Cursor closes it under the
data-directory redirect, and you have neither. One thing did move: `--restricted` no longer drops
MCP on `claude` 2.1.263, so that provider keeps the residue on `judge` runs too.
`IMPLEMENTATION.md` has the measurement, and it is a decision rather than a fix.

**`Provider::resolved_skills` is now `Provider::resolve`**, answering contract 5.3's two
`resolved` fields from one probe: `Resolved { skills, mcp, failed_mcp }`, each `Option` where the
provider did not say. It takes a `hydra::mcp::Probe`, which is either the run's own config file
or `Unanswerable`. Your skill listing is free through `agy -p "/skills"`; ticket 23 found no free
route to your resolved MCP set, so `mcp: None` is the honest answer until one turns up, and
contract 5.3 requires a reader to treat it as unanswered rather than as an empty listing.
