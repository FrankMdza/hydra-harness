# 46 - Parallel calls to one tool on `claude` leave the first of them `Ungated`

Type: build
Status: resolved
Blocked by:

## What

Run `01a095e6-626f`, `claude` 2.1.268 on sonnet under `standard`, emitted three `tool_use`
frames in a row, two `WebFetch` and one `WebSearch`, at seq 190, 191 and 192, and only then
three `hook_started` and three `hook_response` frames, at 193 to 198. The hook answered every
call. The corpus walk still reported the first `WebFetch` as `Ungated`.

`decide` in `src/normalize/claude.rs` joins a response to its call through the `hook_started`
frames inside the call's own window, which closes at the next call. That rested on a 6.6
measurement: the start fires the moment the call is emitted, before the next call of the same
tool. On 2.1.268 a batch of parallel calls emits every call first and every start after, so
the first call's window holds no start at all, and the second's holds the wrong one.

Join starts to calls first-in, first-out per tool instead: a call of tool T takes the earliest
untaken `hook_started` of T with a later seq. That is the same answer as the window in the
pipelined case ticket 22 measured, since starts are emitted in call order even when responses
are not, and it is the right answer in the parallel case. A synthetic record in
`tests/normalize.rs` with two same-tool calls before their two starts must yield two
decisions and no `Ungated`. Found by the corpus walk on 2026-09-12; map ticket 59's research
named the vendor bug class this looked like, and it was not that.

## Outcome

`group_starts` in `src/normalize/claude.rs` keeps the window join as the first pass and adds a
second: a call whose window holds no `hook_started` borrows the earliest start from a later
window of the same tool that holds more than one. A plain first-in, first-out join was tried
first and broke run `01a07f37-8a67`, where `Read` calls the binary refused on their input, with
no hook ever started, stole the one start the next real `Read` owned. Borrowing only from a
surplus keeps that record clean and fixes `01a095e6-626f`. Test
`parallel_calls_to_one_tool_each_get_their_own_hook_answer` builds the batch shape and asserts
two decisions and no `Ungated`.

What the ticket assumed that was false: that first-in, first-out was the same answer as the
window wherever the window had one. It is not when a call legitimately has no start.
