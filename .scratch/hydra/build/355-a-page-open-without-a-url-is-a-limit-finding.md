# 355 - A codex page open without a URL reads as a `webrun` call with nothing to check, and the record says so

Type: build
Status: resolved
Size: haiku
Blocked by:
Opened: 2026-10-21, by the session, from build 353's Outcome. `src/normalize/codex.rs::web_search`
(lines 353 to 387) copies `item["action"]["url"]` into `{"open": [{"ref_id": <url>}]}` whatever it
holds. When an `open_page` item carries no `url`, or a `url` that is not a string, the call reads
as `webrun` with `ref_id: null`. The Slack floor reads only string URLs, so that call passes the
floor with no URL to check, and the record carries no finding that says the reader had nothing to
check. A replay that says `allow` on a call it could not read is the one thing the reader must
not do in silence; every other hole in this file is a `Limit` finding (lines 244, 283, 686, 751).
No probe has shown this shape on 0.160.0, so this is a guard, not a measured case.

## What

In `web_search`, when `action.type` is `open_page` and `action.url` is not a string, the call is
still named `webrun` with capability `network_read`, its arguments are `{"open": []}` rather than
a list holding a null, and the reader pushes one `Finding::Limit` naming the item id and saying
the page open carried no URL, so the replay's decision was made without one. The decision join
stays what it is: with no refusal row the call reads as `allow` from `hydra_policy`, and the
finding beside it is what tells a reader of the record that this allow checked nothing. An
`open_page` item with a string `url` is unchanged from build 353. The finding is pushed once per
such item, not deduplicated across items, since each one is a separate call the floor did not see.

The reason for `{"open": []}` over `[{"ref_id": null}]`: the gate script's `fetch_urls` reads
`open[].ref_id` strings and skips the rest, so both shapes give the floor nothing. The empty list
is the honest one. It says no page was named, where a null says a page was named badly.

Out of scope: a `Limit` for an `open_page` item whose `url` is a string that is not a URL. The
floor already reads such a string and the policy decides. Also out of scope: the `item.started`
half with `action.type: "other"`, dropped today at line 224 and still dropped.

## Contract

Input: a `codex` record line whose `raw` is an `item.completed` frame of type `web_search` with
`action: {"type": "open_page"}` and no `url`, or `"url": 7`. Output: one `ToolCall` named `webrun`
with `arguments` equal to `{"open": []}`, capability `network_read`, one `ToolResult` from
`results`, one `Finding::Limit` whose text holds the item id and the words `no url`, and a
`permission_decision` of `allow` under `permissive`. The page-open cases of build 353 and the
fixture 13 test of build 354 still pass unchanged.

## Cases

In `tests/normalize.rs` after `a_codex_search_item_still_reads_as_websearch` (line 3439),
written first:

1. `a_codex_page_open_without_a_url_reads_as_webrun_with_nothing_open_and_a_limit`: the two
   frames of `codex_page_open` with the `url` key removed from `action` and `query` set to `""`;
   one call named `webrun`, `arguments["open"]` an empty array, capability `network_read`, one
   result, exactly one `Limit` in `common::other_findings` whose text contains the item id and
   `no url`, decision `allow` under `permissive`.
2. `a_codex_page_open_whose_url_is_not_a_string_reads_the_same_way`: `"url": 7`; the same
   assertions as case 1.
3. `a_codex_page_open_with_a_url_still_carries_no_limit`: the existing
   `a_codex_page_open_reads_as_a_webrun_call_with_the_url_under_open` already asserts
   `other_findings` empty, so this case is that test, run again. Do not add a copy.

## Do

1. Read `src/normalize/codex.rs` lines 196 to 262 and 353 to 387, and `tests/normalize.rs` lines
   3339 to 3460 for the `codex_page_open` helper and the three cases of build 353.
2. Cases 1 and 2, red.
3. The change in `web_search`: match on `item["action"]["url"].as_str()` inside the `open_page`
   arm; a string gives the arguments of today, anything else gives `{"open": []}` and the
   finding. Rewrite the doc comment's last sentence to say what a missing URL reads as.
4. One sentence at the end of the paragraph `**The replay reads \`webrun\` the way the hook saw
   it.**` in `IMPLEMENTATION.md` (line 1754), saying a page open with no URL reads as `webrun`
   with `open: []` and a `Limit`, build 355.
5. `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
   `cargo nextest run --release --workspace --no-fail-fast`, `cargo test --release --workspace
   --doc`, then `## Outcome`.

## Outcome

Built. `web_search` in `src/normalize/codex.rs` matches `action.url` as a string inside the
`open_page` arm. A string gives the same `webrun` call as build 353. Anything else gives
`{"open": []}` and one `Finding::Limit` reading "page open <id> named no url, so its decision
checked nothing". The doc comment's last sentence now says what a missing URL reads as.

Tests: `a_codex_page_open_without_a_url_reads_as_webrun_with_nothing_open_and_a_limit` and
`a_codex_page_open_whose_url_is_not_a_string_reads_the_same_way` in `tests/normalize.rs`, written
red first. Both failed on `arguments["open"]` before the change. The build 353 and 354 page-open
tests pass unchanged, and the search-item test passes.

Checks: `cargo fmt --all --check` clean, `cargo clippy --workspace --all-targets -- -D warnings`
clean, `cargo nextest run --release --workspace --no-fail-fast` 1474 passed and 4 skipped, and
`cargo test --release --workspace --doc` passed.

Assumed and not measured: no probe has produced a page open without a string URL on 0.160.0.
The test shape is synthetic. `IMPLEMENTATION.md` says so.

Not done, as the ticket scoped it out: a `Limit` for an `open_page` URL that is a string but not a
URL, and the `item.started` half with `action.type: "other"`, which is still dropped.
