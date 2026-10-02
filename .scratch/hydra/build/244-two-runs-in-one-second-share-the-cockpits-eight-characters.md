# 244 - Two runs launched in one second share the eight characters the cockpit shows

Type: build
Status: open
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
