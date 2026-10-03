# 247 - A session is told its mode at start, and two refusals name the way out

Type: build
Status: open
Blocked by:

## What

Map ticket 96, D3 (resolved 2026-10-03): both halves. In the usability probes of 2026-10-02 three
of five sessions asked for a tool of the other mode, and nothing told them which mode they were
in.

## Do

1. `hydra run --interactive` gives `claude` one line at start in both modes, through the flag the
   provider already takes for appended system text (check `claude --help`; do not touch the
   preamble, which is protected config #2 and reaches gated launches only). Gated: "You are a
   gated Hydra session under <policy>. You cannot launch agents yourself or with `hydra run`; you
   can ask for an approved panel with the `panel` tool: <shape names from config/panels.json>."
   Ungated: "You are an ungated Hydra copilot. Launch agents with `hydra run` from a shell in the
   repository; each one is gated and recorded. The `panel` and `delegate` tools are not here."
   Read the shape names at launch; never hard-code them.
2. The `hydra/seatbelt-delegation` refusal (`src/face.rs:57`) ends with what to do instead: the
   `panel` tool when this run has a launcher listening, else asking the human to launch from
   outside. The `subagent_spawn` refusal in the gate does the same, naming `panel` only when the
   launch is a gated interactive session (the gate knows its launch mode from its config; if it
   does not, add it as a field rather than guessing).
3. Tests: the two start lines from the argv builder without launching (`tests/` as
   `claude::interactive_argv` is tested today); each refusal's new tail; a gated launch whose
   `panels.json` has two shapes names both.

## Done when

- The tests pass and the refusal tests fail with the change reverted. fmt, clippy and
  `cargo test` pass. `gate_sha` changes; the Outcome says so.
