# 124 - An unhooked row keeps its capability, and the replay judges it

Type: build
Status: resolved
Blocked by:

## What

Map ticket 79, resolved 2026-09-24. Read its `## Answer` first.

Today `unhooked(name)` in `src/policy/table.rs` near line 69 builds a `Tool` whose capability is
`Capability::Unhooked`. The compiler skips those rows (`src/policy/mod.rs` near line 821). The
Cursor reader (`src/normalize/cursor.rs` near lines 518 and 536) keeps them out of the replay
and files each call as `Allow`, decider `Provider`, reason `hydra/unhooked: ...`, under every
policy. So a Cursor run under `judge`, which grants no `network_read`, can call `WebSearch` and
the record shows an ordinary allowed call. `CODEX` has an `unhooked("WebSearch")` row too.

## Do

1. `Tool` gets a flag for "the hook never fires for this". `unhooked(name)` takes the real
   capability as a second argument. Once no row uses `Capability::Unhooked`, remove the
   variant, and its `"unhooked"` string in `src/policy/mod.rs` near line 98, unless a record
   field or a doc depends on that string. `grep -rn '"unhooked"\|Unhooked' src tests docs`
   and read each hit.
2. Capabilities: `WebSearch` is `NetworkRead` on both Cursor and codex. For `ReadLints`,
   `getMcpToolsToolCall` and `awaitToolCall`, the map answer proposes `FileRead` of the run's own
   data for the first and `ProviderNative` for the other two. Check each against the comment
   above it and a measured record (`grep -l ReadLints runs/*.jsonl | head`) before choosing. If
   `FileRead` needs a path the record does not carry, say so and pick `ProviderNative`, with a
   comment saying why.
3. The compiler keys its skip on the flag, not the capability.
4. The Cursor reader: an unhooked call asks the policy about the row's capability. The policy
   has no path to check (these rows carry none), so it answers from the capability alone.
   Allowed: the decision stays as it is today. Denied: the decision is `Deny`, decider
   `Provider`, and a finding says the provider ran a tool the policy denies without asking the
   hook. Keep the existing Disagreement arm for the case where the hook did fire. Pick the
   `Finding` variant that fits, or add one, and justify the choice in a comment.
5. The codex reader: find how a `WebSearch` item reads today (`src/normalize/codex.rs`, the
   calls to `codex_capability`) and give it the same treatment.
6. `IMPLEMENTATION.md`, where it describes map 51's unhooked rows: say what changed.
   `SECURITY.md`: if it says an unhooked tool sits outside every policy, correct it.

## Done when

- A fixture twin under `tests/fixtures/records/cursor-agent@.../`, made the way
  `tests/fixture_corpus.rs` makes the others: a `judge` run with one `WebSearch` call. Its
  normalized record carries the new finding and the call is not a plain `Allow`. The same call
  under `standard` reads exactly as it does today. A codex twin too if the codex fixture corpus
  has a shape a `WebSearch` item fits.
- The unit test near `src/policy/table.rs:660` asserts the flag and the capability of each
  unhooked row, and the one near line 704 asserts codex's.
- With your reader change reverted, the `judge` fixture test fails. Paste the failure line
  into `## Outcome`.
- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --lib`, `cargo
  test --test normalize --test gate --test fixture_corpus --test outcome` pass. `outcome` has
  one known red on this machine's live walk (build ticket 125); say whether it is the only one.

## Outcome

Resolved 2026-09-24. Built by `composer-2.5` on run 01a0d572-9dee under `standard`, about 5.36M
tokens, 5.20M of them cache reads. The session read the diff, left out the `integrations/puppets/`
files the worktree had inherited from master's uncommitted state, and changed one decision.

`Tool` has an `unhooked` flag, and `unhooked(name, capability)` keeps the real capability.
`Capability::Unhooked` is gone. The compiler skips rows on the flag. Rows: `ReadLints` is
`FileRead` (it carries a `paths` list, not one path field, so the replay judges the capability
alone), `getMcpToolsToolCall` and `awaitToolCall` are `ProviderNative`, and `WebSearch` is
`NetworkRead` on Cursor and on codex. `Policy::allows_capability` answers with no path, and
treats `ProviderNative` as outside the five policies. `judge` allows nothing, so a `ReadLints`
under `judge` is now a finding too, which is correct.

The Cursor and codex readers file `Finding::UnhookedDenied` (tool, call id, a
`hydra/capability: policy X does not allow Y` reason) when the policy denies the row's
capability. `show` prints it as `unhooked_denied`, and rung one's corpus counts it with the
disagreements. The existing Disagreement arm, for a hook that did fire, stays.

The session's change: step 4 of this ticket asked for a `Deny` decision with decider `Provider`,
and the agent built that. It said the call was refused when the provider ran it, and the agent
had to narrow the gated-outcome walk in `tests/outcome.rs` to keep a `judge` fixture from
reading as `gated`. Map 79's answer asks only for a finding naming the provider. The decision is
now `Allow`, decider `Provider`, reason `hydra/unhooked: ...`, as before, and the policy's view is
the finding beside it. The `tests/outcome.rs` change was dropped, and the walks pass without it.

codex gained a `web_search` item arm. Its shape (`query`, `results`) is map ticket 58's reading
of the hooks reference, not measured on a Hydra run, so there is no codex fixture. The first
real codex web search should be checked against it.

Fixture 12 under `cursor-agent@2026.09.02-c22c1a3/` is a `judge` run with one `webSearchToolCall`.
`a_cursor_websearch_under_judge_is_a_finding_the_hook_never_saw` and
`the_judge_websearch_fixture_reads_as_unhooked_denied` in `tests/normalize.rs` read the finding.
With the reader's finding switched off on master, both fail (normalize.rs:1546 and 1574); the
agent saw `left: Allow, right: Deny` on its own version before the change. The two table unit
tests assert flag and capability per row.

On master with the live corpus: lib 210, normalize 92, gate 99, fixture_corpus 3, outcome 47,
record_contract 21, show 3, all passing, and fmt and clippy are clean. No `judge` Cursor record
on this machine calls an unhooked tool, so the live walk gains no finding. IMPLEMENTATION.md and
SECURITY.md say unhooked calls are judged by capability now. The fixture count in README.md and
IMPLEMENTATION.md now says twelve.
