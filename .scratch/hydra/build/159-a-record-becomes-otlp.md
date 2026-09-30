# 159 - A record becomes OpenTelemetry, on disk and nowhere else

Type: build
Status: claimed
Blocked by:

## What

The README's roadmap says "OpenTelemetry emission from the record, so existing viewers can read
it". The map says a viewer reads `runs/` and is never the source of truth. The human wants his
traces in a viewer, LangSmith among them, and the rule stands that nothing leaves the machine
except through `hydra export`. So: a translator from a run record to OTLP JSON, written to a
file. Sending it anywhere is `hydra export`'s job and not this ticket's.

## Do

A new directory `integrations/otel/`, Python, stdlib only, like the grader. Nothing under `src/`
changes. Tests in `integrations/otel/tests/` on the fixture records under
`tests/fixtures/records/`, each shown failing before the change.

1. `record_to_otlp.py <record.jsonl> [--out file.json]` reads one record (the contract is
   `docs/run-record-contract.md`; the normalized view is what `hydra show --events` prints, and
   `integrations/grader/records.py` shows how to call it) and writes one OTLP JSON
   `ExportTraceServiceRequest`: a root span for the run (`run_start` to `run_end`, attributes
   provider, model, policy, outcome, tokens), one child span per tool call with its name, its
   gate decision and its timing, one span per `seat_request` with the seat run ids as links, and
   the `parent_run_id` as the trace's parent so a session and its seats share a trace id
   derived from the session's run id.
2. Secrets: every string attribute goes through `integrations/grader/redact.py`. The task text
   is an attribute cut at 2,000 characters; no tool output text goes in, only names, decisions
   and sizes.
3. `record_to_otlp.py --all <runs dir> --out-dir <dir>` does every record, skipping the ones
   already written with the same file length.
4. `integrations/otel/README.md`: what is written, what is left out on purpose, and the two
   sentences that this writes files and `hydra export` is the only way out.

## Done when

- The tests, each failed before the change, on the fixture records; the suite passes under
  `.venv/bin/python` if the worktree has it, else `python3`.
- The report shows one span of the output and says which fields of the contract are not
  carried and why.
