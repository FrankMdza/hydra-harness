# 125 - The new `agy` seats measured on what we already know: GPT-OSS 120B against Gemini 3.8 Flash

Type: research
Status: resolved
Blocked by:
Opened: 2026-10-15, by the session, from the human's note of the same day: a new model on `agy`
"no es muy útil y alucina mucho"; measure it on a panel-style task whose answer we already know,
compare it with the Gemini seat, and decide which of the two keeps the `agy` seat on panels.
`agy models` on 2026-10-15 lists Gemini 3.8 Flash (high, medium, low), Claude Opus 5.5, Claude
Sonnet 5.5 and GPT-OSS 120B (medium), and no Gemini Pro; the one entry new since the memory of
2026-09-08 that fits the human's description is GPT-OSS 120B, so it is the one measured, against
`gemini-3.8-flash-high`. If the human meant another model, this ticket measures that one too.

## Question

Three probes per model, each a recorded run under `standard` in a Hydra worktree, the same task
text for both:

1. Twelve closed questions about this repository with answers the session computed on master
   first (file counts, a constant's value, a doc comment's wording, a function's return, two with
   a false premise). Score: right, wrong, invented (a path, line or value that does not exist),
   and whether the model said the false premise was false.
2. The panel question of map 120, question 1, whose answer is known since 2026-10-13: does the
   floor refuse a shell command that names the run's own worktree by absolute path? The model
   measures on its own live gate and reads the code. Score against the synthesis: allowed, the
   carve-out at `src/policy/mod.rs` line 964 and `gate.py`'s `protected_root`.
3. A bounded code task: a word-frequency script with tests in the worktree's scratch
   directory, run by the model. Score: does it run, do the tests pass, how many turns.

Cost per probe in tokens and seconds from the record. The decision the numbers feed: which model
sits in the `agy` seat, and what GPT-OSS 120B is for, if anything.

## Findings

Resolved 2026-10-15 by the session, from six recorded runs under `standard`, three per model, the
same task text for both. `agy models` on this day lists no Argon and no Gemini Pro; the one new
entry that fits "not very useful, hallucinates a lot" is GPT-OSS 120B, so that is what was
measured. If the human meant another model, this ticket reopens for it.

| Probe | Gemini 3.8 Flash high | GPT-OSS 120B medium |
|---|---|---|
| 1, twelve closed facts | 12 of 12 right, both false premises named, line cites correct to within two lines; 677K tokens (`01a13e02-8111`) | first launch died after a 42 s `ls -R` on a 503 "No capacity available" (391K, `01a13dff-47d9`); second: 9 of 12, neither false premise named, two values invented (`start_time` as the Linux `/proc` reader; `CLONE_NOFOLLOW = 0x00000100`, real value `0x0001`), one misread (`tests/cleanup.rs:533` holds `hermes` as a refused unknown provider, not a launch); 619K (`01a13e05-0f14`) |
| 2, map 120 question 1 on the live gate | allowed, as the synthesis found; carve-out read at `own_write_paths` line 876 and `denied_spellings`, both real; 286K (`01a13e0c-7966`) | allowed; the code paragraph cites lines 772 to 779 loosely and adds agy's own `.system_generated` refusal as if it were Hydra's; 225K (`01a13e0c-cb01`) |
| 3, word-frequency script with five tests | 5 of 5 pass, 0 fix rounds, 28 and 95 lines, tie-break and split correct on reading; 338K (`01a13e12-4094`) | first launch died on the same 503 (91K, `01a13e12-9d84`); second: 5 of 5 pass, 1 round, 40 and 60 lines, correct on reading, `Counter` and a usage line; 123K (`01a13e18-ebac`) |

What the numbers decide:

- The `agy` seat on panels stays `gemini-3.8-flash-high`. GPT-OSS invented two values out of
  twelve and swallowed both false premises, which is the one failure a panel seat cannot have; it
  also lost two launches in five to server capacity.
- GPT-OSS 120B is usable for a bounded code task with tests attached: it passed the same probe as
  Flash at a third of the tokens. That is its only measured use. Not for facts, not for reviews, not
  for anything whose answer the session cannot check mechanically.
- Flash's own cost is the finding against Flash: 677K tokens to answer twelve greps, against 49K
  for a one-question probe on 2026-09-08. It reads whole files where a grep would do. A probe to
  Flash should name the command to run.
