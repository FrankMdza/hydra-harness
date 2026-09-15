# 52 - The demo scripts only work in the original checkout

Type: build
Status: claimed
Blocked by:

## What

`docs/demo/record.sh` cds to its own checkout, then `demo.sh` does
`export PATH="$HOME/hydra/target/release:$PATH"` and `cd "$HOME/hydra"`, so running it from a
clone records the original `~/hydra` instead. `verdict.py` builds its record path from
`~/hydra/runs/<id>.jsonl` and ignores `HYDRA_ROOT`.

Also measured: the checked-in `hydra.cast` header says 80x24 while `record.sh` passes
`--cols 118 --rows 30`, so the cast in the repository is not the one the published GIF came
from. Re-record after the scripts work anywhere.

Make all three read the checkout they live in and honour `HYDRA_ROOT`. Found by the cold-clone
review.

## Progress

The three scripts read their own checkout now. `demo.sh` derives it from `BASH_SOURCE` instead of
hardcoding `$HOME/hydra`, `verdict.py` takes the data root from `HYDRA_ROOT` and falls back to
`~/hydra`, and `record.sh` already cd'd relative to itself.

**The ticket was wrong about the cast.** The 80x24 header is not evidence the checked-in cast is
stale. asciinema 3.2.1 dropped `--cols` and `--rows` and ignores both without a word: measured with
`asciinema rec --cols 118 --rows 30 --command "tput cols; tput lines"`, which records a pty that
answers 80 and 24 and writes a header saying the same. The flag is `--window-size 118x30`, and
`record.sh` passes that now. So the cast in the repository is the one the GIF came from, and what
lied was the recording script telling `agg` to paint 118x30 over an 80-wide recording.

Left open: re-record with the corrected script. It needs a live `agy` run of about three minutes
and a quiet machine, and this one was carrying five other runs at the time.
