# Changelog

Dates are when the change landed on `master`. Run ids are the evidence.

## Unreleased

- `hydra evolve --task` gives the proposer a target; the first directed pass produced the
  first patch a panel reviewed as a patch (2026-09-09).
- `hydra show` and `hydra reap` take a run-id prefix (2026-09-10).
- Admission refuses patches to the code rows harness spec 15.2 names, not only `policies/`
  and `config/` (2026-09-12, build ticket 40).
- A curate pass no longer tries to delete the proposal branch it just made (2026-09-12,
  build ticket 43).
- README rewritten around the measurement Hydra exists for; `llms.txt` added; demo GIF
  recorded from a real `agy` run (2026-09-12).

## 0.2.0, 2026-09-08

All eleven commands of harness spec 17. Three routed providers, five policies, the run record
contract, the skill store with per-run snapshots, the MCP delegate face, the Herdr bridge,
both Ouroboros rungs. `IMPLEMENTATION.md` carries every departure from the design.
