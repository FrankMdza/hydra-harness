#!/usr/bin/env bash
# Records the demo. Needs asciinema on PATH, and agg and tmux for the scripted part.
#
#   bash docs/demo/record.sh                  scripted: demo.sh into hydra.cast and hydra.gif
#   bash docs/demo/record.sh copilot [repo]   by hand: the copilot, into copilot.cast
#   bash docs/demo/record.sh puppets <repo>   by hand: Master of Puppets, into puppets.cast
#
# The scripted part launches two runs, one on `agy` and one on `claude` Haiku, and
# drives the cockpit with tmux send-keys. The other two are chats with a model, so
# nothing scripts them: asciinema records what the author types until the session
# ends. `suprema` and `masterofpuppets` are the links to
# integrations/copilot/bin/hydra-copilot and integrations/copilot/bin/masterofpuppets.
#
# Run it from the real checkout in a terminal of your own, not from inside a run.
#
# `--window-size` is the flag, not `--cols/--rows`: asciinema 3 dropped those two
# and ignores them without a word, so the checked-in cast was recorded at the
# terminal's own 80x24 while `agg` was told to paint 118x30. Measured on
# asciinema 3.2.1, build ticket 52.
set -e
cd "$(dirname "$0")/../.."
need() { command -v "$1" >/dev/null || { echo "record.sh: $1 is not on PATH" >&2; exit 1; }; }
need asciinema
case "${1:-scripted}" in
  scripted)
    need agg; need tmux
    asciinema rec --overwrite --window-size 118x30 --command "bash docs/demo/demo.sh" docs/demo/hydra.cast
    agg --cols 118 --rows 30 --font-size 15 --idle-time-limit 2.5 --theme monokai docs/demo/hydra.cast docs/demo/hydra.gif
    ls -la docs/demo/hydra.gif
    ;;
  copilot)
    need suprema
    # hydra-copilot takes [model] [repo] [provider]. An empty model keeps its default.
    # With no repo it orchestrates this checkout.
    if [ -n "${2:-}" ]; then
      asciinema rec --overwrite --window-size 118x30 --command "suprema '' '$2'" docs/demo/copilot.cast
    else
      asciinema rec --overwrite --window-size 118x30 --command "suprema" docs/demo/copilot.cast
    fi
    ;;
  puppets)
    need masterofpuppets
    [ -n "${2:-}" ] || { echo "usage: record.sh puppets <repo>" >&2; exit 1; }
    asciinema rec --overwrite --window-size 118x30 --command "masterofpuppets '$2'" docs/demo/puppets.cast
    ;;
  *)
    echo "usage: record.sh [scripted | copilot [repo] | puppets <repo>]" >&2
    exit 1
    ;;
esac
