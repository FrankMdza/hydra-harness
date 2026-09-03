# 25 - Cursor's usage and thinking fields

Ticket: [25-cursor-usage-and-thinking.md](../issues/25-cursor-usage-and-thinking.md)
Corrects: [09-run-record-schema.md](../issues/09-run-record-schema.md)

Run 2026-09-02 on this machine. `cursor-agent --version` printed `2026.08.31-4057e58` at 16:51
local, which is the same build ticket 14 probed on 2026-09-01. The binary did not move under me
this time. Everything below is either a frame I captured live or bundle source I read at
`/Users/USER/.local/share/cursor-agent/versions/2026.08.31-4057e58/`.

## Summary

Ticket 14 was right. Cursor emits real token counts, and it emits reasoning text. Both facts hold
in both JSON output formats, and the numbers move when the input moves.

Three things ticket 14 did not have, and they change the shape of the fix:

1. **The four fields are not agy's four.** Cursor gives `inputTokens`, `outputTokens`,
   `cacheReadTokens`, `cacheWriteTokens`. No reasoning-token count and no total. The internal
   protobuf does carry a `reasoningTokens` field, and the print-mode writer drops it on the floor.
   So ticket 09's normalized `usage` event gets a cache-write count from Cursor that `agy` does not
   give, and loses the thinking-token count that `agy` does.
2. **`inputTokens` is net of cache, and it clamps at zero.** The emitter computes
   `max(inputTokens - cacheReadTokens - cacheWriteTokens, 0)`. On a warm enough cache the field
   Hydra would call "input tokens" reads 0 while the turn really consumed tens of thousands. This
   is the zero trap the ticket warned about, except it lands on one field rather than the block.
3. **Reasoning text needs a hidden flag in `json` and no flag in `stream-json`.** The two formats
   expose it by different mechanisms and neither is in `--help`.

## Method

Every probe ran in
`/private/tmp/claude-501/-Users-USER/f2c096c9-40e2-4c31-bdb1-bdcf8d1f4683/scratchpad/t25/`.
Nothing was written inside `~/hydra` except this file and the three edits ticket 25 asks for.

I read the bundle first so the live runs could confirm a specific claim rather than fish. The
print-mode writer is `6041.index.js`; the option table is `index.js`.

Six runs. One was refused before any model call, five reached the API. Prompts were kept to one
sentence except the one that had to carry real input.

| probe | command | why |
|---|---|---|
| P0 | `cursor-agent -p "Reply with exactly: OK" --output-format json` | trust gate, no `--trust` |
| P1 | same plus `--trust` | is `usage` in `json` at all |
| P2 | same prompt, `--output-format stream-json --trust` | does `stream-json` differ |
| P3 | `-p "What is 17*23? Reply with the number only." --output-format json --show-thinking --trust` | the hidden thinking flag |
| P4 | `-p "<75KB corpus> …reply with the checksum on line 0742"`, `--output-format stream-json --trust` | do the numbers move |
| P5 | `-p "Run the shell command 'exit 3', then read ./nope.txt…"`, `--output-format stream-json --trust` | usage on the rejected path |

P4's corpus is 900 generated lines, 74,813 bytes, each line carrying a distinct checksum. Asking
for the checksum on one line proves the model actually read the input rather than guessing, so a
token count that moved cannot be explained by anything else. It answered `34662`, which is correct.

## Where usage appears

**Only on the `result` frame, in both `json` and `stream-json`.** I scanned every frame of all five
captures for a `usage` key or any token-shaped field outside the result frame and found none.
Cursor has no per-step usage, so it does not behave the way `agy` does.

`--output-format text` emits no usage. The text branch writes the answer string and stops.

The three result-frame writers in `6041.index.js` sit behind these flags:

```js
ne = true === L.streamPartialOutput
oe = "stream-json" === L.outputFormat
re = "json"        === L.outputFormat
ie = "text"        === L.outputFormat
se = re && true === L.showThinking
```

and every one of them appends usage the same conditional way:

```js
we ? {usage: we} : {}
```

That conditional matters for the schema. `we` is set only if a `turnEnded` update carried at least
one of the four token fields. A successful run can therefore emit a `result` frame with no `usage`
key at all. I never saw it happen in seven captures, but the adapter has to treat the block as
optional on the success path, not merely on the failure path.

### P1, `--output-format json`, verbatim

One line of stdout, nothing else:

```json
{"type":"result","subtype":"success","is_error":false,"duration_ms":6867,"duration_api_ms":6867,
 "result":"OK","session_id":"bc4e6a43-e684-4c64-b99b-da0a39249c4c",
 "request_id":"53cbd53d-da95-4016-8ac8-e4712a24bdd7",
 "usage":{"inputTokens":14629,"outputTokens":27,"cacheReadTokens":2304,"cacheWriteTokens":0}}
```

### P2, `--output-format stream-json`, same prompt, all eight frames

```json
{"type":"system","subtype":"init","apiKeySource":"login","cwd":"/private/tmp/…/t25","session_id":"16a68032-…","model":"Auto","permissionMode":"default"}
{"type":"user","message":{"role":"user","content":[{"type":"text","text":"Reply with exactly: OK"}]},"session_id":"16a68032-…"}
{"type":"thinking","subtype":"delta","text":"The user wants a reply","session_id":"16a68032-…","timestamp_ms":1788386102066}
{"type":"thinking","subtype":"delta","text":" containing exactly ","session_id":"16a68032-…","timestamp_ms":1788386102071}
{"type":"thinking","subtype":"delta","text":"\"OK\".","session_id":"16a68032-…","timestamp_ms":1788386102071}
{"type":"thinking","subtype":"completed","session_id":"16a68032-…","timestamp_ms":1788386102072}
{"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"OK"}]},"session_id":"16a68032-…"}
{"type":"result","subtype":"success","duration_ms":9632,"duration_api_ms":9632,"is_error":false,"result":"OK","session_id":"16a68032-…","request_id":"d257e658-…","usage":{"inputTokens":11557,"outputTokens":22,"cacheReadTokens":5376,"cacheWriteTokens":0}}
```

Worth noticing against research 01's gap 10: `timestamp_ms` is on the thinking frames and, from P5,
on the tool-call frames. It is not on `system/init`, not on `user`, not on `assistant` and not on
`result`. Ticket 14 said Cursor now stamps "most frames". Measured here it stamps some frames, and
the two that bracket a run are not among them. Hydra still has to stamp every line itself.

## The four fields, and what they count

`inputTokens` is not the raw input. The emitter rewrites it before anyone sees it:

```js
we = (function (e) {
  return Object.assign(Object.assign({}, e), {
    inputTokens: Math.max(e.inputTokens - e.cacheReadTokens - e.cacheWriteTokens, 0)
  });
})({inputTokens: …, outputTokens: …, cacheReadTokens: …, cacheWriteTokens: …});
```

Two runs of the identical prompt prove the arithmetic from outside the binary. P1 reported
`14629 + 2304`, P2 reported `11557 + 5376`. Both sum to **16933**. Same prompt, same total raw
input, different cache split, and the reported `inputTokens` swung by 3,072 tokens between them.
P3's slightly longer prompt sums to 16,940 the same way.

So the reconstruction rule for the record is
`raw_input = inputTokens + cacheReadTokens + cacheWriteTokens`, and the reported `inputTokens` is
the uncached remainder. Research 14 read this transform out of the bundle. This is it confirmed
against emitted numbers.

Multi-turn runs are aggregated, not per-turn. The bundle sums all four fields across every
`turnEnded` before the result frame is written, which is why P5's three turns report one block.

**Against `agy`, field by field.** `agy`'s result carries `input_tokens`, `output_tokens`,
`thinking_tokens`, `cache_read_tokens`, `total_tokens`. Cursor and `agy` overlap on input, output
and cache-read only, and even there the semantics differ, since `agy` reports full input with cache
read beside it while Cursor subtracts. Cursor adds `cacheWriteTokens`, which `agy` does not report.
Cursor has no total and, more importantly for ticket 09, **no reasoning-token count**.

That last one is a deliberate drop rather than an absence upstream. The internal
`TurnEndedUpdate` message is constructed with five fields:

```js
turnEnded(e) { … new f.TurnEndedUpdate(e ? {
  inputTokens: BigInt(e.inputTokens), outputTokens: BigInt(e.outputTokens),
  cacheReadTokens: BigInt(e.cacheReadTokens), cacheWriteTokens: BigInt(e.cacheWriteTokens),
  reasoningTokens: BigInt(e.reasoningTokens ?? 0)
} : {}) …}
```

`reasoningTokens` appears exactly once in the whole bundle, in that constructor. The print-mode
writer never reads it. So Cursor counts reasoning tokens internally and does not tell you.

## Where thinking appears

Reasoning is exposed two ways, by different mechanisms, and neither is documented in `--help`.

**In `stream-json`, with no flag.** `thinking/delta` frames carry a `text` chunk and a
`timestamp_ms`. `thinking/completed` carries neither text nor duration; it is a boundary marker and
nothing else. A run can open and close several thinking blocks, and P5 produced three.

**In `json`, only with `--show-thinking`.** The flag is registered with `.hideHelp()`, which is why
it is missing from the help output. Its own description is
`"Include model thinking blocks in the final JSON result (only works with --print and --output-format json)"`.
Passing it with `stream-json` exits 1 with `Error: --show-thinking requires --output-format json`.
P3, verbatim:

```json
{"type":"result","subtype":"success","is_error":false,"duration_ms":13889,"duration_api_ms":13889,
 "result":"391",
 "thinking_blocks":[{"text":"Computing 17 × 23.\n\nThe answer is 391.","duration_ms":1}],
 "session_id":"13bf55ca-…","request_id":"208a9715-…",
 "usage":{"inputTokens":7340,"outputTokens":38,"cacheReadTokens":9600,"cacheWriteTokens":0}}
```

The two paths carry the same bytes. `se` accumulates the very same `thinkingDelta` text into `ue`,
then flushes it into `de` on `thinkingCompleted` along with `thinkingDurationMs`. Streaming gives
you the text as it arrives and no duration; the final block gives you the text in one piece with a
duration. Choosing `stream-json` costs the per-block duration, which is the only thing the JSON
path has that the stream path does not.

**Is it the model's reasoning or a summary of it?** I cannot answer that from here, and neither can
anyone reading the CLI. The text is first person, present tense and specific to the prompt, which
reads like a reasoning trace rather than a post-hoc gloss. But it arrives already shaped by the
provider, `model` reports `"Auto"` rather than a model id, and I have no way to compare against the
raw trace. Record it as what it is, provider-reported reasoning text, and do not claim more.

## The failure and rejection paths

**A rejected tool still reports usage.** P5 ran `exit 3` under `--trust` alone, which the shell
runner refused, plus a read of a file that does not exist. The run finished `exit 0`,
`subtype: "success"`, `is_error: false`, and carried a full usage block:

```json
{"type":"result","subtype":"success","duration_ms":14082,"duration_api_ms":14082,"is_error":false,
 "result":"…`exit 3` was rejected by the shell runner; `./nope.txt` was not found.",
 "session_id":"af4c5621-…","request_id":"d3bcbd44-…",
 "usage":{"inputTokens":8032,"outputTokens":437,"cacheReadTokens":43776,"cacheWriteTokens":0}}
```

The two tool outcomes stayed distinct on the stream, which reconfirms ticket 14 against research
01:

```json
{"type":"tool_call","subtype":"completed","tool_call":{"shellToolCall":{"result":{"rejected":{"command":"exit 3","workingDirectory":"/private/tmp/…/t25","reason":"","isReadonly":false}}},"toolCallId":"call-a087796d-…-0\nfc_605af469-…_0","startedAtMs":"1788386209660","completedAtMs":"…"}}
{"type":"tool_call","subtype":"completed","tool_call":{"readToolCall":{"result":{"error":{"errorMessage":"File not found"}}},"toolCallId":"call-a087796d-…-1\nfc_605af469-…_1","startedAtMs":"1788386209661","completedAtMs":"1788386210102"}}
```

`rejected.reason` came back empty again, so ticket 14's open question stays open. The model then
retried the rejected shell command once on its own, which is a second thing the record will show as
two identical rejections rather than one.

**A run that fails hard reports nothing.** The `catch` around the whole print-mode loop writes no
frame to stdout. It stores the token usage into telemetry (`we && xe.setTokenUsage(we)`), prints
the error string to stderr and exits 1. So on the failure path the counts exist inside the process
and never reach the record. Hydra sees an empty stdout, a stderr line and an exit code.

P0 is the cheap instance of that. Without `--trust` the run is refused before any model call: exit
1, zero bytes of stdout, and this on stderr.

```
⚠ Workspace Trust Required
  Cursor Agent can execute code and access files in this directory.
  Do you trust the contents of this directory?
    /private/tmp/claude-501/…/t25
  To proceed, you can either:
    • Run 'agent' interactively to decide
    • Pass --trust, --yolo, or -f if you trust this directory
```

## Do the numbers move

Yes, decisively, and only in the field that should have moved.

| probe | prompt | `inputTokens` | `outputTokens` | `cacheReadTokens` | `cacheWriteTokens` | raw input sum |
|---|---|---|---|---|---|---|
| P1 | tiny, `json` | 14,629 | 27 | 2,304 | 0 | 16,933 |
| P2 | tiny, `stream-json` | 11,557 | 22 | 5,376 | 0 | 16,933 |
| P3 | tiny, `json --show-thinking` | 7,340 | 38 | 9,600 | 0 | 16,940 |
| P4 | 74,813 bytes of corpus | 37,352 | 59 | 512 | 0 | 37,864 |
| P5 | two tools, three turns | 8,032 | 437 | 43,776 | 0 | 51,808 |

P4 against the P1 to P3 baseline is the answer to the ticket's question. Adding 74,813 bytes of
input moved the raw input sum from ~16,935 to 37,864, a jump of 20,929 tokens, or about 3.6 bytes
per token on that text. `outputTokens` tracked output too: 22 tokens for `"OK"`, 437 for P5's
three-turn run with two tool calls and a paragraph of explanation. These are not placeholders.

**One field is always zero.** `cacheWriteTokens` read 0 in all seven captures, mine and ticket 14's
both. I never got it to move. Either this account's traffic never writes cache through this path or
the field is not wired up. That is the exact hazard the ticket named, scoped down to one field
rather than the block, and it is enough to matter: an estimator keyed on "does the provider report
cache writes" would read a real zero and a broken zero the same way.

## What changes in ticket 09

**`usage_source` for Cursor moves from `estimated` to `provider`.** The premise ticket 09 wrote it
on, "Cursor emits no token counts at all", is false on this build in both JSON formats, on the
success path and on the tool-rejection path. Nothing has to be estimated when a result frame
arrives.

**The estimator keeps a producer, and it is a narrower one.** Three cases still have no
provider-reported counts, so `usage_estimator` does not become dead code:

- a run that fails hard, or is killed, and never writes a result frame, which is every Cursor
  failure by design;
- `--output-format text`, if Hydra ever runs it, though the record contract has no reason to;
- a success whose result frame omits `usage`, which the `we ? {usage:we} : {}` conditional permits
  and I did not observe.

There is a fourth, and it is the one to argue about. `inputTokens` clamping to 0 on a warm cache is
a provider-reported number that is arithmetically correct and useless as "input tokens". Hydra
should store the three parts and derive the total rather than trust the one field, which keeps the
estimator out of it.

**The `thinking` sentence needs rewriting.** "never from Cursor" is wrong. Cursor is the provider
that gives reasoning text in print mode with no flag, which is more than ticket 09 credited any
provider with. What it does not give is a reasoning-token count, and ticket 09 does not currently
distinguish reasoning text from reasoning tokens. It should, because the four providers split
differently on the two: `agy` reports `thinking_tokens` and Cursor reports thinking text, and
neither reports both.

## What I could not establish

- **Whether `cacheWriteTokens` can ever be non-zero.** Seven captures, always 0. I did not find the
  write path in the bundle and I did not construct a workload that would obviously trigger one.
- **Whether the thinking text is the model's own trace or a provider summary.** Unanswerable from
  the CLI. `model` reports the display name `"Auto"`, so I cannot even name the model to compare
  against.
- **Whether a successful run can omit `usage` entirely.** The code allows it. I never saw it.
- **Whether usage survives a mid-run API error.** The `catch` path writes no result frame, so the
  answer from source is no, but I did not trigger a live API failure to confirm it. Triggering one
  means forcing a rate limit or a network cut, which I judged not worth the quota.
- **Whether a partially-written result line can appear.** Ticket 14's SIGKILL probes left whole
  lines over a pipe. Hydra runs a pty. Still untested, still ticket 09's gap.
- **Anything about a pinned model.** Every run used the default, which resolves to `"Auto"`. A run
  with `--model` pinned might report different fields. I did not spend a run on it.

## What the runs cost

Five paid runs, one turn each except P5, which took three. All on the existing Cursor subscription,
so the cost is quota rather than money. Cursor reports no cost field at all, which research 14
already noted, so there is no dollar figure to give.

| | total |
|---|---|
| `inputTokens` reported | 78,910 |
| `cacheReadTokens` | 61,568 |
| raw input (sum of the two) | 140,478 |
| `outputTokens` | 583 |
| wall clock, five paid runs | 69.0 s |

P4 alone is 37,864 of the 140,478. It is the run that answers the ticket's central question and I
would spend it again. The other four are small.

P0 cost nothing. It never reached the API.

## Housekeeping

No API key was read and no billed call was made, so D16 holds. Every probe ran on the paid Cursor
subscription.

`~/.cursor/chats/` gained five session entries, which I left in place rather than delete, since
deleting means writing into Cursor's own config directory. My scratch, including every captured
frame and the corpus, is at
`/private/tmp/claude-501/-Users-USER/f2c096c9-40e2-4c31-bdb1-bdcf8d1f4683/scratchpad/t25/`.
Nothing outside `.scratch/hydra/` was touched.
