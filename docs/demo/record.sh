#!/usr/bin/env bash
# asciinema records docs/demo/demo.sh; agg renders the cast to a GIF. Needs both on PATH.
set -e
cd "$(dirname "$0")/../.."
asciinema rec --overwrite --cols 118 --rows 30 --command "bash docs/demo/demo.sh" docs/demo/hydra.cast
agg --cols 118 --rows 30 --font-size 15 --idle-time-limit 2.5 --theme monokai docs/demo/hydra.cast docs/demo/hydra.gif
ls -la docs/demo/hydra.gif
