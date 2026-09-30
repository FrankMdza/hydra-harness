#!/bin/bash
# Print the six round-three hydras in colour, one after another, with a title each.
# Run it in a real terminal: Claude Code's `!` and a plain pipe drop the colour codes.
cd "$(dirname "$0")" || exit 1
PY="${PY:-$(command -v python3)}"
[ -x ../../../.venv/bin/python ] && PY=../../../.venv/bin/python
for d in claude/design-1 claude/design-2 claude/design-3; do
  printf '\n\033[1m%s\033[0m\n' "$d"; "$PY" render.py "$d.txt" | tail -n +2
done
for d in codex/design-1 codex/design-2 codex/design-3; do
  printf '\n\033[1m%s\033[0m\n' "$d"; "$PY" render_codex.py "$d.txt"
done
