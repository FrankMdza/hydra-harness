# 353 - The codex replay names a `webrun` call by the name the hook saw, and reads the page it opened

Type: build
Status: resolved
Size: haiku
Blocked by:
Opened: 2026-10-21, by the session, from build 338's Found: `src/normalize/codex.rs::web_search`
(lines 347 to 375) names every `web_search` stream item `WebSearch`, reads `query` and `results`,
and its comment still says hosted search never reaches the hook on 0.154.0. On 0.160.0 the item a
`webrun` call leaves is a `web_search` item too, with `action.type: "open_page"`, and the hook did
see that call as `webrun` (build 338, one paid turn). So a replay of a run that opened a page
re-evaluates the call under the unhooked `WebSearch` row and the Slack floor never sees the URL,
while the gate judged it as `webrun` with the URL under `open[].ref_id`. The two readings of one
call disagree, and the record cannot show it.

## What

In `web_search`, an item whose `action.type` is `open_page` becomes a call named `webrun` with
arguments `{"open": [{"ref_id": <action.url>}]}`, the shape the hook payload carried, so the
replay re-evaluates it under the hooked `network_read` row and the Slack floor reads the same URL
the gate read. The output stays what `results` holds, as today. An item with no `action`, or
with `action.type` of `search`, stays `WebSearch` with `query`. The `item.started` frame of the
same call arrives with `query: ""` and `action.type: "other"` and is ignored already (line 224),
so nothing changes there. The decision join follows what the reader does for a hooked
`command_execution` with no refusal row (lines 683 to 735): read it and say in the Outcome what an
allowed `webrun` reads as, since codex writes no hook frame for an allow.

The measured stream, from the probe of 2026-10-21, `codex-cli` 0.160.0, thread
`01a1609f-d4ad`, two frames for one call:

```
{"type": "item.started", "item": {"id": "exec-2cc2aa80-74a6-40c4-8ee4-4a44974c5951", "type": "web_search", "query": "", "action": {"type": "other"}}}
{"type": "item.completed", "item": {"id": "exec-2cc2aa80-74a6-40c4-8ee4-4a44974c5951", "type": "web_search", "query": "https://example.com", "action": {"type": "open_page", "url": "https://example.com"}, "results": [{"type": "text_result", "domain": "example.com", "ref_id": "turn0view0", "snippet": "Total lines: 1", "title": "Example Domain", "url": "https://example.com/"}]}}
```

The hook payload for the same call carried `tool_name: "webrun"`, `tool_use_id:
"exec-2cc2aa80-74a6-40c4-8ee4-4a44974c5951"` and `tool_input: {"open": [{"ref_id":
"https://example.com"}], "response_length": "short"}` (build 338's Outcome). The item id and
the hook's `tool_use_id` are the same string, which is what joins them.

Out of scope: a real record. No Hydra run has used `webrun` yet, so the corpus under
`tests/fixtures/records/codex@0.154.0` has no such item and the case below is synthetic. A
recorded `codex` run under `permissive` that opens one page is one paid turn the session can
launch later, and the fixture corpus takes it then.

## Contract

Input: a `codex` record line whose `raw` is the `item.completed` frame above. Output: one
`ToolCall` named `webrun`, `arguments.open[0].ref_id` equal to `action.url`, capability
`network_read`, one `ToolResult` with the `results` text, and the replay's decision for it under
`permissive` is `allow` with the Slack floor applied to that URL. An item with
`action.type: "search"` and a `query` still reads as `WebSearch`.

## Cases

In `tests/normalize.rs` beside `a_codex_command_reads_its_exit_code_and_a_failure_is_not_a_refusal`,
written first:

1. `a_codex_page_open_reads_as_a_webrun_call_with_the_url_under_open`: the two frames above as
   two record lines; one call named `webrun`, its arguments as the contract says, one result,
   no `Limit` finding, and the derived decision under `permissive` is `allow`.
2. `a_codex_page_open_of_a_slack_method_replays_as_a_floor_refusal`: the same frames with
   `action.url` set to `https://slack.com/api/files.upload`; the replay under `standard` derives
   a `hydra/floor` refusal for that call.
3. `a_codex_search_item_still_reads_as_websearch`: an item with `action.type: "search"` and a
   `query`, no `url`; the call is `WebSearch` with `{"query": ...}`, as the existing reader did.

## Do

1. Read `src/normalize/codex.rs` lines 196 to 262, 347 to 375 and 683 to 735, and `tests/normalize.rs`
   around line 3154 for how a synthetic codex record is built.
2. The three cases, red.
3. The change in `web_search`, with the doc comment rewritten: hosted search stays unhooked on
   0.160.0, a page open is `webrun` and hooked, measured 2026-10-21 by build 338.
4. One paragraph in `IMPLEMENTATION.md` after build 338's at line 1734, saying the replay now
   follows `webrun` and what an allowed one reads as.
5. `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
   `cargo nextest run --release --workspace --no-fail-fast`, `cargo test --release --workspace
   --doc`, then `## Outcome`.

## Outcome

Built. `web_search` in `src/normalize/codex.rs` now reads an item whose `action.type` is
`open_page` as a `webrun` call with `{"open": [{"ref_id": <action.url>}]}`. Its capability is
`network_read`, from the existing `webrun` row. The replay sends it to the hooked path, so the
Slack floor reads the same URL the gate read. Search items keep the `WebSearch` reading with
`query`. The output is still what `results` holds.

Measured. The two page-open cases in `tests/normalize.rs` went red first, then green. The
search case passed before the change, as the old reader did, and it still passes. The
page-open cases use the probe frames from 2026-10-21 as two record lines. The Slack case gives
a `hydra/floor:` deny under `standard`. The `other_findings` check runs beside the scope
limit that every synthetic record gets, which the existing codex tests filter the same way.
`cargo fmt --all --check` passes, `cargo clippy --workspace --all-targets -- -D warnings` is
clean, `cargo nextest run --release --workspace --no-fail-fast` gives 1467 passed and 4
skipped, and `cargo test --release --workspace --doc` passes.

What an allowed `webrun` reads as. Codex writes no hook frame for an allow, so the replay
derives it. With no refusal row in the record, the call reads as `permission_decision` with
`decision: allow` and `decider: hydra_policy`. Its reason is the policy's verdict. A refusal
reads through the result text like any hooked call.

Assumption that turned out only partly true. The ticket treated `action.url` as always
present on an `open_page` item. The code copies whatever `action.url` holds, so a missing URL
gives `ref_id: null`. The floor reads only string URLs, so such a call would pass the Slack
check without a URL to check. No probe has shown this shape, and the code does not flag it.

IMPLEMENTATION.md. The build 338 paragraph said the replay did not follow `webrun` yet. That
sentence now points at build ticket 353, and a new paragraph after it records the reader's
behavior.

## Pending from me

Nothing for this ticket. A recorded `codex` run under `permissive` that opens one page would
add a real fixture to the corpus. That is one paid turn, and the ticket leaves it for later.

## Changed

- `src/normalize/codex.rs`: `web_search` branches on `action.type` and rewrites its doc comment.
- `tests/normalize.rs`: a `codex_page_open` helper and three cases at the end of the file.
- `IMPLEMENTATION.md`: one sentence in build 338's paragraph corrected, and one new paragraph
  after it.
- This ticket: `Status: resolved` and this Outcome.
- `cargo fmt --all` ran across the workspace. It changed nothing outside the test file above.
