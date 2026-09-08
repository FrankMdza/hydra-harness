# 29 - A harvested run that met the rate limit derives `rate_limited`

Type: build
Status: resolved
Blocked by: 28

## Goal

Contract 8 says `rate_limited` is a run the provider refused for its own limit, and rung one
never learns from one. On a recorded `claude` run the outcome comes off `api_error_status: 429`
in the `result` frame. A harvested run has no `result` frame at all. Run 01a07f6c-ba21, an
interactive `claude` session harvested on 2026-09-08, has exactly one assistant turn, the
sentence "You've hit your session limit · resets 1:50am (America/Bogota)", and derives
`completed` with that sentence as `result_text`. Make the harvest path file it as
`rate_limited`.

## Where the design says it

- `docs/run-record-contract.md` section 8, the outcome table and "recorded and learned-from are
  different questions".
- `src/outcome.rs`: `provider_refused` reads the 429, `cursor_stderr_refusal` is the precedent
  for a text match written down as one, and `derive` already knows a harvested body by
  `body_source: "harvest"`.
- `IMPLEMENTATION.md`, "Rough edges worth knowing", the bullet that names this.

## The slices

**The rule.** In `src/outcome.rs`, a harvested `claude` run whose last assistant `message` text
starts with `You've hit your` and contains `limit` derives `Outcome::RateLimited`. It sits
beside `cursor_stderr_refusal`, is keyed on `provider == "claude"` and on the body being a
harvest, and its doc comment says it is a text match, why it exists, and that a reworded
sentence sends the run back to `completed`. It runs before the `completed` fallback and after
the gate-refused-everything check, so a run that was gated and then rate-limited stays `gated`.

**The projection.** `outcome_source` on such a run stays `derived`. `result_text` keeps the
sentence.

**Tests.** `tests/harvest.rs`: a harvested transcript whose only assistant turn is that sentence
derives `rate_limited`; the same transcript with an ordinary sentence still derives
`completed`. A unit test in `src/outcome.rs` beside the existing ones covers the
recorded-run case not changing: a `claude` run with a `result` frame and the same sentence in a
message still comes off `api_error_status`, not the text.

## Done when

- `hydra show 01a07f6c-ba21-7622-b5e1-813a9d9c60db` prints `"outcome": "rate_limited"`.
- The tests above pass and the rest of the suite is untouched.
- The orchestrating session moves the rough-edge bullet in `IMPLEMENTATION.md` into the
  "Deriving an outcome" section as a rule.

## Outcome

Resolved 2026-09-08. `cursor-agent` on Composer 2.5, run 01a07ffe-372a under `standard`:
1.15M tokens, 1.09M of them cache reads, three files, 148 insertions, nine minutes. The rule
landed exactly where the ticket put it, `claude_harvest_limit_refusal` beside
`cursor_stderr_refusal`, and `hydra show 01a07f6c-ba21` prints `rate_limited`. Two harvest
tests and one outcome test, the latter pinning that a streamed body still reads the 429 and
that the sentence alone does nothing there. The orchestrating session applied the diff to
master, ran the suite, and moved the rough-edge bullet into the rate-limited section of
`IMPLEMENTATION.md`.

**What the run found on the way.** Its `ReadLints` call carried `paths`, a list, which the
table and the normalizer both read under another name; the replay derived a deny and the
stream shows Cursor running the tool anyway. Map ticket 51 holds it, and the corpus walk in
`tests/normalize.rs` skips this run's record by id until that ticket closes. The model also
tried `ls ~/hydra/runs/` to look at the record the ticket cites, and the protected path refused
it, which is the design working and a reason to quote a record's shape in a ticket rather than
point at it.
