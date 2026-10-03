# 247 - A session is told its mode at start, and two refusals name the way out

Type: build
Status: resolved
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

## Outcome

Resolved 2026-10-03 by run `01a101a0-b0bd` (`claude` opus 5.5, `standard`, 4.05M tokens),
applied by the session unchanged, with the run's two deviations accepted.

Built: `claude::mode_line`, passed on `--append-system-prompt` to every interactive launch. An
ungated one is told to launch agents with `hydra run` and that `panel` and `delegate` are not
there; a gated one is told its policy and the shapes of `config/panels.json` whose `askers`
include that policy (`interactive::askable_shapes`), or that there are none. The
`hydra/seatbelt-delegation` refusal now ends by sending the model to the human and `hydra run`.
The gate's `subagent_spawn` refusal names the `panel` tool when the session's `seats.sock`
exists beside the gate, else the human.

Deviations the session accepted: (1) the gate decides by the socket's existence at fire time
(`seats_socket` in the compiled config) instead of a launch-mode field, since the socket is bound
after the gate is compiled and the face makes the same test before it lists `panel`; adding the
field would have touched about 20 call sites. (2) The start line names only the shapes this
policy may ask for. And the ticket's conditional tail on the seatbelt refusal cannot fire: the
face asks a listening launcher before it reaches that refusal, so the tail always names the human.

Measured by the session on master: with the tail call removed from `gate.py`, the two new gate
tests fail; restored, they pass. The run measured its four start-line tests failing without the
`--append-system-prompt` push. `--test gate` 115, `--test face` 8, `--test spawn` 36, `--lib` 234;
fmt and clippy clean. `gate_sha` changes for every run.

Not measured: a live `claude` receiving the start line (the flag is a general option in
`claude --help`), and the gate seeing the socket from inside a real seatbelt. Both are for the
human's next gated session, after a release build.
