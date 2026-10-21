# 350 - `claude` 2.1.301's `LSP` and `Workflow` tools are in the capability table, and the last `WebFetch` of two haiku runs that reads `Ungated` is explained

Type: build
Status: claimed
Size: haiku
Blocked by:
Opened: 2026-10-15, by the session, from `hydra doctor --walk` after the three builds of that night
landed. Two `claude` haiku 5.5 runs under `permissive` on Claude Code 2.1.301 (`01a14290-f30d`,
`01a14295-9d3f`) each raise one `Ungated { tool: "WebFetch" }`, the last `WebFetch` of the run in
both cases (seq 304 of the first), while `hydra show --events` prints a derived `allow` and an
`unclassified` result for the same call id; the nineteen earlier `WebFetch` calls of the first run
carry hook evidence. The same two records raise `unknown_tool` findings for `LSP` and `Workflow`,
two tool names the `CLAUDE` table (`src/policy/table.rs`) does not hold, so a run that calls either
is refused `hydra/unknown-tool` the way build 338's `webrun` was. The walk's totals moved from
`Ungated: 5` to `Ungated: 7` in one night on account of these two.

## What

Two rows and one explanation. The rows: `LSP` and `Workflow` as 2.1.301 names them, with the
capability a free probe measures (a `-p` run with the hook pointed at a logging script, one call
each, the payload quoted in the Outcome), the version in the row's comment, and the drift fixture
of build 317 moved to 2.1.301 if it keys on version. The explanation: read the two records around
the last `WebFetch`, the hook sidecar's rows for that call id if any, and the `run_end` timing, and
say which of three it is: the hook's evidence line arrived after the record closed, the provider
ran the call without the hook (the shape map 123 found for `Edit`), or the normalizer joins the
decision to the call and the finding to a different id. Then either a fix in the normalizer, a
`RejectedBeforeHook`-style variant that names the shape, or a sentence in IMPLEMENTATION.md that
says why the last call of a run can read `Ungated` and what a reader should do with it.

## Contract

Input: a `claude` hook payload naming `LSP` or `Workflow`; and the two records above. Output: the
gate's decision for the first; for the second, one named shape and the walk's totals after.

## Cases

In `tests/gate.rs` beside the `claude` table cases, written first: `LSP` and `Workflow` under
`standard` and `read-only` answer with the measured capability's reason and not
`hydra/unknown-tool`; a third name, `Workflow2`, still does. In `tests/normalize.rs`, a synthetic
record with the two runs' shape around a last `WebFetch`, asserting the finding the explanation
names, written after the reading and before the fix.

## Do

1. The probe and the two rows.
2. The reading of the two records, in the Outcome with seqs.
3. The normalizer case and whatever it needs.
4. IMPLEMENTATION.md's `claude` paragraph.

## Done means

The four checks green, `hydra doctor --walk` on this machine prints `Ungated: 5` or says in the
Outcome why the two stay, and the Outcome quotes both hook payloads.
