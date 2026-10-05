# 292 - The models each harness offers, read from the CLIs twice a day

Type: build
Status: claimed
Blocked by:

## What

Frank, 2026-10-04: one table of every model available in each harness, taken from each CLI's own
command, kept on this machine, refreshed twice a day (04:00 and 16:00) by a recurring job.

Measured by the session the same day:

- `agy models` prints `id<TAB>display name` lines after a `Fetching available models...` line.
- `cursor-agent models` prints `Available models`, a blank line, then `id - display name` lines
  (some names end in zero-width characters; strip them).
- `codex debug models` prints JSON: `{"models":[{"slug", "display_name", "description",
  "default_reasoning_level", "supported_reasoning_levels":[{"effort", ...}]}]}`.
- `claude` has no listing command. Its rows come from the aliases Hydra already knows
  (`integrations/puppets/aliases.json`), marked `source: aliases`, never invented.

Build:

1. `hydra models refresh`: runs the three commands with the launch environment's allowlist and a
   timeout each, parses them, and writes `models.json` under the data root atomically:
   per provider, `fetched_at`, `source` (the command, or `aliases`), `ok` or the error, and the
   rows (`id`, `name`, and for codex the reasoning levels). A provider whose command fails keeps
   its previous rows with the error and the old `fetched_at`, so one broken CLI does not empty
   the table.
2. `hydra models [--provider P] [--json]`: prints the table from the file and its age; it launches
   nothing. With no file it says to run `refresh`.
3. A launchd template `integrations/models/launchd/co.hephaestuslab.hydra-models.plist` that runs
   `target/release/hydra models refresh` at 04:00 and 16:00 (`StartCalendarInterval`, two
   entries), with the README lines to install and remove it, on the pattern of
   `integrations/grader/README.md`. The human installs it.
4. `--model` validation is out of scope here; a later ticket may warn when a launch names a model
   the table does not hold.

## Done when

- Tests parse recorded samples of the three outputs (hand-made, from the shapes above), keep the
  old rows when a stub command fails, and check the plist's two times. No test runs a real CLI.
  fmt, clippy over the workspace, `cargo test --lib` and the new test file clean.
  `IMPLEMENTATION.md`: one paragraph.

## Outcome
