# 254 - The cockpit drops the gated chat

Type: build
Status: resolved
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

Resolved 2026-10-03 by run `01a103a7-f5ce` (`codex` gpt-6.1-sol, `standard`, 1.04M tokens). The
session removed what the agent left in place and fixed one sentence the change made false.

Built: `hydra cockpit --session <p> --policy <name>` exits 2 at argument parsing, before a layout
is found or anything starts, with `error: gated chat uses hydra run --interactive --provider <p>
--policy <name>`. The cockpit's launcher no longer passes `--policy`, and the chat title no longer
says "under". `COCKPIT.md` has two modes. Test: `cockpit_policy_refuses_before_starting` (exit
code, exact text, no stdout, no run file); the agent saw it fail before the change. The help
overlay never described the gated mode, so it did not change.

What the session changed: the agent kept `policy` on `cockpit::Session` and a second refusal in
`cockpit::run` that nothing could reach; both are gone, and `main.rs` drops the flag's value.
`integrations/copilot/COPILOT.md` (build 253) still named three cockpit modes; it names two.

Left: `~/hydra-oficina/empezar.sh` uses the removed form and now exits 2. It is outside the
repository and the human's.
