# 338 - `codex` 0.160.0's web tool `webrun` is in the capability table, so a web lookup under `permissive` is allowed and recorded instead of refused as unknown

Type: build
Status: open
Size: haiku
Blocked by:
Opened: 2026-10-15, by the session, from run `01a14290-c0a1`: a `codex` research seat under
`permissive` asked for a web lookup and the gate answered `hydra/unknown-tool: webrun is not in
Hydra's capability table for this provider`. The table (`src/policy/table.rs`, `CODEX`) was read
off 0.154.0 and knows `WebSearch` as an unhooked `network_read` row; the installed binary is
0.160.0 and names the tool `webrun`. The refusal is the design working, 7.1: an unknown name fails
closed under every policy. The table is what is stale.

## What

One row, measured, not guessed. A free probe first: launch `codex exec` the way
`src/provider/codex.rs` does for its login probe, with the hook pointed at a script that logs the
payload and allows, ask it to fetch one URL, and read the payload: the tool's name, whether the
hook fired at all (if it did not, the row is `unhooked`, as `WebSearch` is), and the field that
carries the URL. Then the row in `CODEX`, with the version and the run id in its comment, and the
`is_unhooked` and `capability` facts pinned in `table.rs`'s tests beside the `WebSearch` ones.
IMPLEMENTATION.md's `codex` paragraph gains the sentence; the drift table of build 317 gains
0.160.0 if it keys on version.

## Contract

Input: a hook payload for provider `codex` with `tool_name` and `tool_input`. Output: the gate's
decision and reason. Latency: the gate's own, unchanged.

## Cases

Written before the row, in `tests/gate.rs` beside the `codex` cases, each a synthetic payload fed
to the compiled gate; the first two go red on master today:

1. `webrun` with a `https://` URL under `permissive` is allowed with `hydra/allow`.
2. `webrun` under `read-only` and `judge` is refused and the reason starts `hydra/no-network` or
   whatever `network_read` refusals read today, not `hydra/unknown-tool`.
3. `webrun` naming `file:///Users/USER/hydra/runs/x.jsonl` is refused the way `WebFetch` on
   `claude` is (A11 in `tests/corpus/README.md`).
4. A name the table does not hold, `webrun2`, is still refused `hydra/unknown-tool` under
   `permissive`.

## Do

1. The probe, its payload quoted in the Outcome.
2. The cases, red.
3. The row, the cases green, `no_codex_tool_appears_twice` still green.
4. IMPLEMENTATION.md and, if it exists for `codex`, the drift fixture.

## Done means

`cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
`cargo nextest run --release --workspace`, `cargo test --release --workspace --doc` green, and the
Outcome quotes the probe's payload.
