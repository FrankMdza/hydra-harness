# 294 - Kev-0.8B beside system-one, on this machine

Type: build
Status: open
Blocked by:

## What

Map 109's Kev note (`.scratch/hydra/research/109b-kev.md`): Kev 1.0 is an open, Apache-2.0
decision model family with Jev's interface, and its 0.8B size is stated to run on any Apple
Silicon Mac through an MLX server. Measure it before building on it.

1. **By the session, first, not by a gated run**: read Kev's GitHub README and model cards (the
   research run did not open them), install its server, start Kev-0.8B on loopback, and record
   its memory on this 8 GB Mac and its latency. If it does not fit, stop here and say so.
2. A `hydra-grader` subcommand (name it) that asks a loopback Kev server one `noul` question per
   labelled run ("did this run end without completing?") with the task text and the record
   features `system-one` uses as the document, through `hydra_records::loopback_http`, loopback
   only, and prints Brier, log loss and AUC on the same time split and the same 559 test rows as
   `system-one`, beside the base rates. Tests use a stub server; none reaches a real one.
3. Outcome: the table of baseline, `system-one` and Kev-0.8B, and whether a fine-tune on Hydra's
   records (the post has a section on it) is the next step or `system-one` already wins.

## Done when

- The session's measurements of step 1 under Outcome; tests for step 2 on a stub; fmt, clippy,
  `cargo test -p hydra-grader` clean; `IMPLEMENTATION.md` one paragraph.

## Outcome
