# Local OpenTelemetry traces

Run with Python 3 and an installed Hydra binary (or set `HYDRA_BIN`). No Python
packages are required.

```sh
python3 integrations/otel/record_to_otlp.py record.jsonl --out trace.json
python3 integrations/otel/record_to_otlp.py --all fixture-runs --out-dir traces
python3 -m unittest discover -s integrations/otel/tests -v
```

Without `--out`, a single request is printed to stdout. Each output is an OTLP JSON
`ExportTraceServiceRequest`, containing a `hydra.run` span, child tool spans, and
instantaneous `hydra.seat_request` spans with links to the requested seat runs.
Provider, recorded model, policy, derived outcome, token counts and usage provenance
are root attributes when available. The task is redacted first and then cut to 2,000
characters. Every string attribute and span name uses the grader's secret redactor.
Missing values are omitted, not guessed.

The translator calls `hydra show --events` on a temporary copy of just the input
record, with `HYDRA_ROOT` set to that scratch directory. It never discovers the
user's runs. Outcome, usage, tools and gate decisions come from that normalized
view. Gate decisions are replayed by the installed Hydra and can differ when its
policies differ from those pinned by the record. Raw envelopes supply timestamps
and seat requests, which `hydra show --events` currently omits.

Run times are `run_start.ts` through `run_end.ended_at`. Tool times are call and
matched result envelope timestamps. A missing tool result gives a zero-duration
span with `timing.complete=false`. A missing run end uses the last committed
envelope timestamp and `run.complete=false`; this is an observation boundary, not
an inferred completion. Torn trailing lines are ignored; malformed complete lines
fail conversion. Seat requests have no recorded duration, so their spans are
instantaneous. Times are decimal nanoseconds, IDs are deterministic hexadecimal
hashes, and integer attributes use OTLP's decimal string encoding.

The trace ID is derived from the session root run ID. The root span of a child run
has its parent's deterministic run span ID as `parentSpanId`. Ancestors are read
from sibling record files. If an ancestor is missing, its ID is the highest known
ancestor and is used for the trace ID. Supply the complete session beside the input
to obtain the same trace ID across every depth. Cycles fail conversion.

Batch mode recursively reads `.jsonl` files, writes `<run-id>.json` and a local
`<run-id>.size.json` byte-length stamp, and skips existing outputs whose stamp
matches the input length. Same-length edits intentionally do not invalidate this
cache. Outputs and stamps are replaced atomically, output first.

Messages, thinking, corrections, tool arguments and tool output text are omitted
to keep traces small and avoid carrying the trajectory or file contents. Tools
carry only their names, gate decision and decider, result status, output byte size
and timing completeness. Permission reasons, seat tasks and refusal reasons are
omitted because they can contain paths or task content. Process IDs, auth and
subscription metadata, worktree paths, skills, signatures, secret flags, raw frames,
normalizer findings and replay scope metadata are omitted because this is a trace
summary, not a lossless export or an audit record. Usage provenance is retained so
unavailable or estimated counts are not presented as measured counts.

This integration writes files and makes no network calls.
`hydra export` is the only way out of the machine.
