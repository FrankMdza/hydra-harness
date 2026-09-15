# 52 - The demo scripts only work in the original checkout

Type: build
Status: open
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
