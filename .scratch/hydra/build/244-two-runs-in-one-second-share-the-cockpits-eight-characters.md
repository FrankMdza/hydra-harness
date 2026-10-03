# 244 - Two runs launched in one second share the eight characters the cockpit shows

Type: build
Status: resolved
Blocked by:

## What

Usability probe of 2026-10-02. A copilot session and the first agent it launched were
`01a0fe73-1b67-...` and `01a0fe73-5a9c-...`. The cockpit lists both as `01a0fe73`, and
`hydra show 01a0fe73` refuses, correctly, with `"01a0fe73" matches 2 runs`. A uuid v7 puts 48
bits of milliseconds first, so eight hex characters are 65 seconds wide: every copilot and the
agents it launches in its first minute collide. Pairs seen the same afternoon: `01a0fe63`
(three runs), `01a0fe6a`, `01a0fe71`, `01a0fe73`.

Also from the same sessions, one run to look at: `01a0fe6e-995e` (`hydra cockpit --session
claude`, opus, no policy) ended `failed` with no tokens at 21:04Z.

## Do

1. The cockpit's lanes, table and lanes-by-agent show the shortest prefix that is unique among
   the runs on screen, never fewer than eight characters. The resolver (`src/resolver.rs`)
   already knows how to count a prefix's matches.
2. The refusal for an ambiguous prefix prints, for each match, enough characters to tell them
   apart, plus provider and policy, so the next command can be typed from it.
3. Read `01a0fe6e-995e`'s record and say in the Outcome why it failed. Fix nothing for it in this
   ticket unless the cause is in the cockpit's launch.

## Done when

- A cockpit test with two cards sharing eight characters reads two different labels back from
  the buffer. A resolver test reads the longer prefixes back from the refusal.
- fmt, clippy and `cargo test` pass. `README.md` and `IMPLEMENTATION.md` say it in one sentence.

## Outcome

Resolved 2026-10-03 by run `01a1019b-62ac` (`codex` gpt-6.1-sol, `standard`, 1.54M tokens),
applied by the session unchanged.

Built: `resolver::unique_prefix`, the shortest prefix of at least eight characters that no other
id in a set shares. The cockpit's lanes, task table and lanes by agent label each run with it
among the runs on screen, and the table's run column widens to fit. An ambiguous prefix's refusal
lists each match as its distinguishing label, provider, policy and full id. Tests:
`colliding_run_labels_are_distinct_in_each_view` (in `src/cockpit/app.rs`) and a collision case in
`tests/resolver.rs`.

Measured by the session on master: with `unique_prefix` forced back to eight characters both new
tests fail; restored, they pass. `cargo test --test resolver` 9, `--lib` 228, `--test cockpit`
114 passed and 1 ignored; fmt and clippy clean.

Not done: step 3, the `failed` run `01a0fe6e-995e`. The run could not read it; its record is now
in `~/hydra/pruned/` with the other usability probes. The ticket's premise about the refusal was
half stale: it already printed full ids, and lacked provider and policy.
