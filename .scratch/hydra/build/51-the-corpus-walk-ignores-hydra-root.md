# 51 - The corpus walk reads the real home and ignores `HYDRA_ROOT`

Type: build
Status: open
Blocked by:

## What

`tests/record_contract.rs:575` builds its path from `real_home()` and appends `hydra/runs`,
so the two corpus walks validate whatever is in the developer's own `~/hydra` whatever
`HYDRA_ROOT` says. `.github/workflows/ci.yml` sets `HYDRA_ROOT` and its comment claims "the
corpus walks read HYDRA_ROOT", which is false; CI passes because the runner's home has no
corpus, not because the variable works.

`CONTRIBUTING.md` tells a contributor to use `Layout::at` and never `Layout::discover()`, and
never mentions the variable. So a contributor's first full suite run reads their own records
and can fail on a record Hydra never wrote.

Make the walks honour `HYDRA_ROOT`, fix the CI comment, and say in `CONTRIBUTING.md` that the
suite reads a corpus and which one. Found by the cold-clone review.
