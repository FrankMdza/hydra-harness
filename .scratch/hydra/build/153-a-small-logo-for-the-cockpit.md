# 153 - A small logo for the cockpit and the README

Type: build
Status: resolved
Blocked by:

## What

The human asked on 2026-09-30 for a minimal logo, in the spirit of the one-line marks that
`agy` and `claude` print when they start, built from the Hydra mascot, to sit in the cockpit's
header and at the top of the README. The mascot is the three-headed HydraPet dragon
(`integrations/herdr/MASCOTS.md`, `integrations/herdr/mascot/`, `integrations/herdr/sprites/`).

This ticket is art with constraints. It decides nothing; the human picks.

## Do

1. Look at what the two CLIs print at start (`claude --help` and `agy --help` show their marks
   near the top; if not, say what you found) and at the mascot's sprites and the HD skin, so the
   logo is the same creature.
2. Draw six candidates in `docs/logo/candidates.md`, each in a fenced block, each with a one-line
   note on where it fits:
   - two of one line, at most 24 columns, ASCII only, for a status line or a prompt;
   - two of three lines, at most 32 columns, ASCII only, for the cockpit header;
   - two of three to five lines using box or block characters, for the README and a splash.
   Every candidate shows three heads, however small, and the word `hydra` in lowercase.
3. A script `docs/logo/render.py`, stdlib only, that prints any candidate by name so the human
   can see each in his terminal, and a test that every candidate fits its column limit and uses
   only the characters its class allows.
4. Nothing else changes. No README edit yet, no cockpit code.

## Done when

- `docs/logo/candidates.md` with six candidates and their notes, `docs/logo/render.py` and its
  test passing under `.venv/bin/python` if the worktree has it, else `python3`.
- Your report shows all six rendered, says which two you would pick and why in one sentence
  each, and starts with what you could not confirm.

## Outcome

Resolved 2026-09-30. A first run on `cursor-agent` gpt-5.6-sol-high died at start with "socket
hang up" (run 01a0f064-dbd8). Built by `claude` sonnet on run 01a0f06e-531b under `standard`,
about 0.34M tokens. The session applied it with one change: the two ASCII heads carried doubled
backslashes and slashes (`\\` and `//`), which draw as two characters; the session made them
single so the necks line up.

Six candidates in `docs/logo/candidates.md`: `line-eyes`, `line-fangs`, `head-necks`,
`head-crest`, `splash-blocks`, `splash-box`, with `render.py` to print any by name and a test of
the column and character limits. Neither `claude --help` nor `agy --help` prints a mark, so the
candidates come from the mascot alone. The human picks; no README or cockpit change yet.
