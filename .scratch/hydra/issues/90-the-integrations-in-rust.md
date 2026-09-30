# 90 - The integrations in Rust

Type: task
Status: resolved
Blocked by:
Opened: 2026-09-30, by the session, at the human's request ("is there a way to make all of those
Rust based?").

## Question

`src/` is all Rust (74,861 lines tracked). `integrations/` holds 15,626 lines of Python, stdlib
only except the dragon's renderer (numpy): `puppets` 7,024, `grader` 4,838, `herdr` 1,847,
`apprentice` 1,379, `otel` 303, `local-llm` 235; plus `.scratch/hydra/frontier.py` and logo
scripts. About 8,000 of the Python lines are tests. The rule since the plan of 2026-09-29: the
learning loop lives outside `src/` and Hydra works with it stopped. The cockpit already
reimplements `puppets.py`'s `fold` in Rust (`src/cockpit/goal.rs`) and shells out to
`puppets.py` for `pending`, `approve` and `review`, and ports `sheet.py`'s window share. Where to
put the Rust, in what order, how to know a port changed nothing, and what stays Python.

## Answer

Resolved 2026-09-30 by a panel of three under the human's delegation (brief and answers in the
human's decision log, `decisiones/2026-09-30/brief-90.md` and `answer-90-*.md`).

- **Where: a Cargo workspace** (unanimous A; codex 0.86, cursor-agent 0.67, agy 0.85). `hydra`
  stays the router crate; each integration becomes its own crate with its own binary under
  `crates/`, and the router crate never depends on one. Reversed by a workspace build that needs
  more than 8 GB to compile one integration.
- **Order: `puppets` first** (A, 2 to 1; cursor-agent chose `grader` first at 0.55), then
  `grader`, `herdr`, `apprentice`, `otel` and `local-llm`, scripts last. Each piece finishes, and
  its Python is deleted, before the next starts, so the repository is publishable after every step.
- **Proof: differential** (unanimous A; 0.94, 0.84, 0.95). Python and Rust run on the same inputs
  (the committed fixtures, the real goal logs and run records, read-only) and must give identical
  data output; the Python tests are ported too; the Python goes only then. If two runs of the
  Python on the same input already differ, that output falls back to ported tests plus an explicit
  comparison.
- **The cockpit keeps a subprocess** (B, 2 to 1; cursor-agent chose linking at 0.60). It runs the
  `hydra-puppets` binary as it runs `puppets.py` today, so there is still one writer process of a
  goal log. Reversed by a measured subprocess cost that breaks a cockpit interaction's latency.

Stays Python: training for a rented GPU, which the plan puts on PyTorch and which does not exist
yet. Carried by builds 193 to 196 for `puppets`; the other pieces get their own tickets when
`puppets` is done.
