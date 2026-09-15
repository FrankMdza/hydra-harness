#!/usr/bin/env bash
# asciinema records docs/demo/demo.sh; agg renders the cast to a GIF. Needs both on PATH.
#
# `--window-size` is the flag, not `--cols/--rows`: asciinema 3 dropped those two
# and ignores them without a word, so the checked-in cast was recorded at the
# terminal's own 80x24 while `agg` was told to paint 118x30. Measured on
# asciinema 3.2.1, build ticket 52.
set -e
cd "$(dirname "$0")/../.."
asciinema rec --overwrite --window-size 118x30 --command "bash docs/demo/demo.sh" docs/demo/hydra.cast
agg --cols 118 --rows 30 --font-size 15 --idle-time-limit 2.5 --theme monokai docs/demo/hydra.cast docs/demo/hydra.gif
ls -la docs/demo/hydra.gif
