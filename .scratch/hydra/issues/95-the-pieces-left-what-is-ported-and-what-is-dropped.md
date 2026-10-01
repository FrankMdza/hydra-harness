# 95 - The pieces left: what is ported, what is dropped, and where shared code lives

Type: task
Status: resolved
Blocked by:
Opened: 2026-10-01, by the session, from a read-only plan of the ports that are left
(`.scratch/hydra/research/95-the-plan-for-the-pieces-left.md`). `puppets` and `grader` are in
Rust; map 90 orders `herdr`, `apprentice`, `otel` and `local-llm` next, scripts last. The plan is
seventeen tickets and five questions that decide which of them exist.

## Question

**A. The mascot.** `integrations/herdr/mascot/hydrapet.py` draws two skins. The compact pet is a
24 by 24 byte grid with `math.sin`, and the eleven frames the cockpit reads were generated from
it once and committed. The HD dragon is a 160 by 160 float32 canvas in numpy, the default skin
of the Herdr watch pane, and matching numpy's float32 pairwise sums to the byte is unlikely.

- A1. Port the compact pet only; drop the HD skin and `hydrapet.py`'s standalone modes.
- A2. Port both, the HD skin within a stated tolerance.
- A3. Keep `hydrapet.py` as a frame generator; the pane shows committed frames; `herdr` keeps
  one Python file and numpy.

**B. The shell.** `hy.zsh` and the two bash launchers call `python3` for one JSON field each.

- B1. Port their logic into `hydra-herdr`; `hy.zsh` stays as a one-line function.
- B2. Keep them shell and replace only the `python3` snippets.
- B3. Leave them alone.

**C. Shared code.** `records`, `redact`, `isotime` and `judge_http` live in `crates/grader`; the
apprentice, otel and local-llm need them.

- C1. The new crates depend on `hydra-grader` as a library.
- C2. Move them to a shared crate first (into `hydra-pyjson` or a new one), one ticket in front.

And the judge's client refuses every host but loopback, where the Python of the apprentice and
of local-llm goes wherever `HYDRA_LOCAL_LLM_URL` points: (i) the same rule for all three, (ii)
loopback only for the judge.

**D. What is not worth porting.** `bitnet_serve.py` is a local inference shim whose binary does
not work today. `frontier.py` starts every session. `verify35.py`, `docs/logo/*.py`,
`docs/demo/verdict.py` and a test fixture server are one-offs.

- D1. Drop `bitnet_serve.py`; port `frontier.py` as a subcommand; delete the one-offs that
  nothing calls and keep the fixture server.
- D2. Port everything.
- D3. Keep all of it in Python.

**E. `src/policy/gate.py`.** 1,320 lines of Python compiled into every gate, Python on purpose
because the gate runs as a hook inside each provider. Map 90 called `src/` all Rust.

- E1. Out of scope: it is the product's gate, not an integration, and it stays Python.
- E2. A map ticket of its own later.

## Answer

Resolved 2026-10-01 by a panel of three under the human's delegation, codex, cursor-agent
grok-4.7-high and agy gemini-3.1-pro-high, on four of the five questions. The first is left to
the human. Answers in his decision log, `decisiones/2026-10-01/answer-95-*.md`. The plan is
`.scratch/hydra/research/95-the-plan-for-the-pieces-left.md`.

- **A, the mascot: not settled here.** Two seats chose to port the compact pet only and drop the
  HD dragon (cursor 0.66, agy 0.90); codex chose to keep `hydrapet.py` as a frame generator
  (0.66). Both cursor and codex name the same thing that reverses dropping it: the human using
  the HD skin in the watch pane. It is that pane's default skin, so the condition very likely
  holds, and what the mascot looks like is his to say. The mascot's tickets wait on him. The
  option the seats pointed at without its being on the list: generate the HD frames once, commit
  them as the eleven cockpit frames are, show those, and then delete the renderer and numpy.
- **B, the shell: stays shell** (codex 0.83, cursor 0.61; agy chose to port it at 0.85).
  `hy.zsh` and the two launchers keep their logic, and their three `python3` snippets become a
  call to `hydra-herdr`. The live smoke test has to pass after.
- **C, shared code: a shared crate first, and the loopback rule for every client** (unanimous:
  0.89, 0.78, 0.95). The record reader, the redactor, the date parser and the HTTP client move
  out of `crates/grader` in one ticket. The apprentice and `local-llm` refuse any host but this
  machine's loopback, as the judge does, and say so: they send tasks, diffs and results from
  the human's records. Reversed by one remote model he means to reach; the default is
  `127.0.0.1`.
- **D, not ported** (unanimous: 0.80, 0.84, 0.90): `bitnet_serve.py` is dropped, its binary
  does not work; `frontier.py` becomes a subcommand; the one-off scripts under `docs/` that
  nothing calls are deleted, by the human or with his word, since `docs/` is his; the fixture
  server stays.
- **E, the gate: out of scope** (unanimous: 0.94, 0.58, 0.75). `src/policy/gate.py` stays
  Python: it runs as a hook inside each provider's process, and a Python hook is what all four
  can run. `IMPLEMENTATION.md` and the README say so in one sentence, because map 90's brief
  called `src/` all Rust. Reversed by all four providers taking a native binary as a hook.

Carried by builds 224 to 239. The mascot's tickets are not written.

