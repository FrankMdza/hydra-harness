# 31 - `hydra usage recount`

Type: build
Status: resolved
Blocked by: 18

## Goal

Harness spec 17 lists eleven commands and this build has seven. `hydra usage recount` is the
smallest of the four missing: the spec says it has no work to do in v1, because no provider
produces `usage_source: estimated`, and it stays in the surface so the tool exists the day one
does. Build the command so it walks the corpus, says what each run's usage source is, and says
there is nothing to recount.

## Where the design says it

- `docs/harness-spec.md` section 17, the row and the paragraph "has no work to do in v1".
- `docs/run-record-contract.md` section 7.8 for `usage_source` and its three values.
- `src/show.rs` for how a command reads one record back through the normalizer, and
  `src/normalize/mod.rs::UsageSource`.

## The slices

**The command.** `hydra usage recount [<run-id>]`, a `Usage` subcommand in `src/cli.rs` with a
nested `Recount` subcommand, dispatched in `src/main.rs`, implemented in a new `src/usage.rs`
with `pub fn recount(layout: &Layout, run_id: Option<&str>, out: &mut dyn Write) -> Result<()>`
and `pub mod usage;` in `src/lib.rs`. Without a run id it reads every `*.jsonl` under
`layout.runs` in filename order; with one, that record alone. Each record is read the way
`show.rs` reads one and its projection's `usage_source` is tallied.

**The output.** One line per source with its count, in the order `provider`, `estimated`,
`unavailable`, then one closing line. When the `estimated` count is zero the closing line is
`nothing to recount: no run carries usage_source estimated`. When it is not zero the closing
line is `N runs carry usage_source estimated and no tokenizer is built to recount them`, and
the command exits 0 either way. A record that fails to parse is reported on stderr by filename
and skipped.

**Tests.** A new file `tests/usage.rs`, built the way `tests/show.rs` builds a record: two
synthetic records, one with a provider usage frame and one with none, tallied as one
`provider` and one `unavailable` with the nothing-to-recount line; and the one-record form
naming a run id that does not exist fails with an error that names the path.

## Done when

- `hydra usage recount` on this machine prints the tally and the nothing-to-recount line.
- `hydra usage recount <run-id>` prints the same for one record.
- The tests above pass and the rest of the suite is untouched.
- The orchestrating session amends `IMPLEMENTATION.md`'s command list and `CLAUDE.md`'s test
  file count.

## Outcome

Resolved 2026-09-08, on the second launch. The first, run 01a07ffe-4808 on `agy`
`gemini-3.8-flash-high` under `standard`, the first `agy` run under that policy ever, read
fifty-five steps and 3.2M tokens without writing a file and died on the provider's own
"timeout waiting for response" while `cargo test --release` compiled a cold worktree; it
found the `grep_search` field name wrong on the way, fixed in master the same hour. The
second, run 01a08008-ba33 on `cursor-agent` Composer 2.5 under `standard`: 1.21M tokens, five
files, three tests, thirteen minutes, every slice as written. `src/usage.rs` reads each record
through `normalize::read_with` and `outcome::Usage::of`, the same path `show` takes. On this
machine: `provider: 58`, `estimated: 0`, `unavailable: 5`, nothing to recount. Nineteen test
files now. The orchestrating session amended the command list, the not-built list and
`CLAUDE.md`'s counts.

**What the second run found on the way.** One more `ReadLints` past the gate, the same leak as
ticket 29's run, so map ticket 51's skip in the corpus walk is by tool rather than by record.
The model also tried to `tail` its own provider's log under `~/hydra/spawn/`, and the protected
path refused it.
