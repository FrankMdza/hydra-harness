# 26 - The face gets its parent, and the document admits it

Type: build
Status: resolved
Blocked by:

## Goal

Map ticket 48 decided how the delegate face learns which run spawned it: Hydra recognizes its
own binary. A server in the canonical MCP document whose `command` has the basename `hydra` gets
`--parent <run-id>` appended to its args when Hydra writes the run's config. The document's
format does not change. And the human admitted the face under `standard` and `permissive`, so
the orchestrating session writes the `[[server]]` block into `config/mcp.md` in the same change.

## Where the design says it

- Map ticket 48's `## Answer`. Harness spec 9 on the face and its depth cap, 6.5 on the document.
- `src/mcp.rs::write_config`, which copies servers into the per-run file. `src/face.rs` for
  what `--parent` does. `tests/mcp.rs` for how a fixture document drives the writer.

## The slices

**The rule.** In `write_config`, a server whose `command`'s last path component is `hydra` has
`--parent <run-id>` appended to `args` in the written file, and only there: the document and
`mcp.injected` on the record stay as declared. The rule is written once, named, and its doc
comment says why a string is the key and why the basename rather than the whole path.

**The face's depth.** With the flag in place, a delegated child's `depth` is the parent's plus
one and the cap in section 9 holds. `tests/delegation.rs` already drives the cap by hand;
add one test that starts from a written config: the face command in the file carries the flag
with the right id.

**Tests.** `tests/mcp.rs`: a fixture document with one server `command = "hydra"` and one with
`command = "/opt/homebrew/bin/hydra"` both get the flag; a server whose command is
`hydra-something` or `python3` does not. The written file for `read-only`, `judge` and
`curate` does not carry the face at all, since the document admits it to the other two.

## Done when

- The written config for a `standard` run names the face with `--parent <that run's id>`.
- The tests above pass and the suite is untouched.
- `config/mcp.md` carries the block and a paragraph on the rule, and `IMPLEMENTATION.md`'s
  delegation section says the face is admitted since ticket 26.

## Outcome

Resolved 2026-09-07. `claude` on Sonnet 5, run 01a07f37-9461 under `standard`: 2.65M tokens,
2.5M cache reads. The rule lives in `src/mcp.rs`, named, keyed on the basename of `command`, and
appends `--parent <run-id>` only in the written file. Eight tests across `tests/mcp.rs` and
`tests/delegation.rs`: bare and absolute `hydra` both get the flag, `hydra-something` and
`python3` do not, and the face is absent from the files written for `read-only`, `judge` and
`curate`. The orchestrating session wrote the `[[server]]` block into `config/mcp.md` with the
human's authorization and the test that pinned an empty document now pins this one.

**Measured twice on a real `standard` run.** With `command = "hydra"` the face came up
`failed` in `claude`'s init frame and `mcp.resolved` said `[]`, because a recorded run's `PATH`
is the human's and the binary is not on it. With the absolute path the init frame says
`connected`, the tool list carries `mcp__hydra__delegate`, and the record says `resolved:
["hydra"]`. The launch check of 6.1 item 6 did not catch the first case: for `claude` a server
is deliverable when it rides the command line, and whether its command exists is not a
question anything asks before the spawn. Worth a line on the map's fog rather than a ticket
today.
