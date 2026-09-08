# 34 - `hydra export`

Type: build
Status: resolved
Blocked by: 31

## Goal

Harness spec 14: the one path by which anything leaves this machine, typed by a human, default
deny, allowlisted by path, redacted in four fixed classes, verified before anything is written.
Build it as the audit the spec says is implementable today.

## Where the design says it

- `docs/harness-spec.md` section 14 in full, and 15.1's row for protected config #3, the export
  allowlist.
- `docs/run-record-contract.md` section 16, the export test: 16.2 says what a record has to
  answer, and the exclusions beside it; section 5.2 names the fields export drops; section 14 for
  the prefix sweep and `secret_flag`; the line near 973 on harness runs and judge runs.
- `config/README.md`: `config/export-allowlist.json` does not exist and a human writes it.
- `src/normalize/` for the tool-call adapter that yields every path a run touched;
  `src/ouroboros/corpus.rs` for how rung one already walks the corpus and excludes runs.

## The slices

**The allowlist.** `config/export-allowlist.json` is a JSON array of absolute repo paths. Read
at export time only. When the file is missing, `hydra export` refuses with one sentence saying a
human writes it and where, and exports nothing. Do not create the file: it is protected config #3.

**Eligibility, per run.** A run is eligible when every path its tool calls read or wrote sits
under an allowlisted repo, when no call was `unclassified`, when `outcome_source` is not
`unknown`, when it is not a harness run, when it carries no `attachment` content (16.2's third
rule), when `secret_flag` is null on every line, and when every run beneath it by `parent_run_id`
is eligible too. Each reason has a name, and `--dry-run` prints one line per run with its
reasons.

**Redaction.** The four classes of section 14, each a fixed pattern applied to every string in
the exported run: private repository and project names taken from the allowlist entries that
are not the exported run's own, customer data inside a fixture is out of scope for a pattern and
is named in the output header as the residue, secrets by the same prefixes the write-time sweep
uses, and the stdout of the user's own hooks, which are the `hook_response` frames, dropped.
Section 5.2's fields are dropped.

**The verification scan and the output.** After redaction, the same prefix sweep runs over the
output; one hit aborts the whole export with exit 1 and nothing written. Otherwise one JSON line
per exported run, the normalized events, to the path in `--out <file>`, and a header line first
naming the count, the allowlist sha, and the residue classes.

**Tests.** `tests/export.rs`: a scratch layout with a two-entry allowlist and four synthetic
records, one eligible, one reading outside the allowlist, one with a child that reads outside,
one with `secret_flag` set; the eligible one is the only line written; `--dry-run` names each
reason; a missing allowlist refuses; a planted `sk-ant-` string in an eligible run's text aborts
with nothing written.

## Done when

- `hydra export --dry-run` on this machine, with no allowlist, refuses and names the file.
- The tests above pass and the rest of the suite is untouched.
- If any slice needs a decision the documents do not make, stop at that slice, finish the
  others, and say so in the report.

## Outcome

Resolved 2026-09-08. `cursor-agent` on Composer 2.5, run 01a0801d-7811 under `standard`:
4.96M tokens, eight files, four tests, thirty-two minutes, in a Herdr pane. Every slice landed
and the model found one false assumption of its own: paths have to be canonicalized on macOS
or every run looks out of scope. The orchestrating session corrected two things on the way
to master. The model excluded every outcome rung one never learns from, and contract 16.2
excludes one outcome, `auth_failed`; `Reason::AuthFailed` replaced `NeverOutcome`. And the
run's own record carried a `Write` gate leak that was an `editToolCall` with no `args` and
"Invalid arguments" as its result, so the cursor normalizer now files a `no-path` deny with a
provider error as a limit rather than a leak. The run also used `awaitToolCall`, a Cursor
tool the table did not know, refused by the gate at run time; noted in the Cursor section.
No decision surfaced: the allowlist stays a human's file and does not exist on this machine.
