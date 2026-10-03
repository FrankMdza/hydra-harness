# 254 - The cockpit drops the gated chat

Type: build
Status: claimed
Blocked by:

## What

Map 102 D6, unanimous on 2026-10-03: the gated chat leaves the cockpit. The cockpit is for the
person who orchestrates, through the ungated copilot or the viewer. `hydra run --interactive
--policy <name>` stays on the command line with build 250's sandbox, unchanged.

## Do

1. `hydra cockpit --session <p> --policy <name>` exits 2 before launching anything, with one line
   that names the command-line form: `hydra run --interactive --provider <p> --policy <name>`.
   Find where `src/cli.rs` and `src/cockpit/` handle `--policy` for `--session`, and remove the
   cockpit's gated path rather than leaving it unreachable. Keep anything `hydra run
   --interactive` itself still needs.
2. The help overlay and the start line no longer describe a gated cockpit session.
3. `COCKPIT.md`: "The three ways to run the cockpit" becomes two, and one sentence points at the
   command-line form for a gated chat. `IMPLEMENTATION.md`: one paragraph.
4. Tests: the refusal and its text; the ungated session and the viewer still start as before
   (use whatever test already covers them, and say which). The refusal test fails with the
   change reverted; say so.

## Done when

- fmt, clippy over the workspace, `cargo test --lib` and the cockpit tests are clean.
  `~/hydra-oficina/empezar.sh` uses the removed form; say so in the report, do not edit it.

## Outcome
