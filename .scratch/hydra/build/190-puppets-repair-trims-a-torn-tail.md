# 190 - puppets repair trims a torn tail

Type: build
Status: claimed
Blocked by: 188

## What

Build 188 made every writer refuse a goal log whose last line has no newline (`UnterminatedLog`,
exit 1), which is safe but leaves the log unwritable after a writer died mid-line until a human
trims the tail by hand. There is no command for that.

## Do

Tests first in `integrations/puppets/tests/`, each shown failing before the change. Stdlib only.

1. `puppets.py repair <goal>` prints the unterminated tail (cut to 300 characters) and what it
   would do, and writes nothing. `puppets.py repair <goal> --yes` does it, under the log's lock:
   moves the tail into `<goal>.torn-<UTC timestamp>.txt` beside the log, then truncates the log to
   its last newline, so nothing is lost. A log with no torn tail says so and exits 0 either way.
2. `UnterminatedLog`'s message names `puppets.py repair <goal>`.
3. `integrations/puppets/README.md`, one paragraph.

## Done when

- Each test failed before the change; the puppets suite passes.
