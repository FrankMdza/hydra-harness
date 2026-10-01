# 95 - The pieces left: what is ported, what is dropped, and where shared code lives

Type: task
Status: open
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
