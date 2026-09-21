# 61 - Codex sits on the judge panel

Type: build
Status: resolved
Blocked by:

## What

Map ticket 63 closed on 2026-09-21: `judge` grants no tool to any provider, so a judge never
reads a file and the diff arrives in the prompt. The one thing keeping `codex` off the `evolve`
panel is the pre-flight refusal build ticket 48 put in for `read-only` and `judge` together.
`read-only` stays refused; `judge` opens.

1. `codex::unsupported_policy` in `src/provider/codex.rs` refuses `read-only` alone. The
   message keeps naming build ticket 48 item 4 for `read-only` and says `judge` is routable
   since this ticket. Its unit test changes with it.
2. The judges list in `src/ouroboros/evolve.rs` gains `(Provider::Codex, "codex")` after `agy`.
   A `codex` judge takes no `--model`, the way the other two non-`claude` judges do. Every
   place in that file and in `src/ouroboros/review.rs` that assumes three answers, three run
   ids or "one per provider" as a count of three takes four. The commit message the pass
   writes lists four judges.
3. `tests/evolve.rs` and `tests/gate.rs`: whatever asserts the panel's size or the `judge`
   refusal on `codex` moves with it, and one new test shows a `judge` launch on `codex` is not
   refused at pre-flight by `unsupported_policy`.

Out of scope: `docs/`, `IMPLEMENTATION.md`, `README.md` and `CLAUDE.md`, which say "panel of
three" in several places; the orchestrating session rewrites those after the measurement.
Do not launch `hydra evolve` and do not launch `codex`; the session measures a real pass.

Verify with `cargo build --release`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt
--check`, and `cargo test --release --test evolve --test gate` plus the unit tests of the files
touched. Do not run the whole suite.
## Outcome

Resolved 2026-09-21. `cursor-agent` on Composer 2.5, run 01a0c4a2-d6b4 under `standard`, 1.0M
tokens, five files: the refusal narrows to `read-only`, the judges list is four, the panel
verdict needs four answers, and the tests move with it. Applied unchanged. The measurement is
the ninth `evolve` pass, launched the same day with a directed task and `codex` on the panel;
its verdict is in the rung-two section of `IMPLEMENTATION.md`.

**Measured.** Pass ten, 01a0c4b4-81ec, on 2026-09-21: the `codex` judge, run 01a0c4c9-9ad1 under
`judge`, answered `{"verdict":"pass","findings":[]}` in 27.7k tokens with no prose around it, and
the panel of four offered the patch, the first offerable patch in ten passes. Pass nine before it
was blocked at admission by a corpus test whose selector the proposer's own record matched; the
selector is fixed in the same change. `rungs-spec.md`, `IMPLEMENTATION.md` and `README.md` say
four judges and ten passes.
