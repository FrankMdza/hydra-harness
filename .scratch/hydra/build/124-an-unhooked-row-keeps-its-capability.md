# 124 - An unhooked row keeps its capability, and the replay judges it

Type: build
Status: open
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
