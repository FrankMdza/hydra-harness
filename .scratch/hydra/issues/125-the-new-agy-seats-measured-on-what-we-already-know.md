# 125 - The new `agy` seats measured on what we already know: GPT-OSS 120B against Gemini 3.8 Flash

Type: research
Status: claimed
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
