# 292 - The models each harness offers, read from the CLIs twice a day

Type: build
Status: resolved
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
- `claude` lists its models only through `/model` inside the TUI (Frank, 2026-10-04), which no
  script can call; it has no listing command. Its rows come from the aliases Hydra already knows
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

Built 2026-10-04 by a Grok 4.6 run (`01a109c7`), reviewed, applied and run by the session.

`hydra models refresh` (`src/models.rs`) runs the three listing commands under the launch
environment with a 60 s timeout each, reads `claude` from `aliases.json`, and writes
`models.json` under the data root atomically; a provider whose command fails keeps its old rows
and `fetched_at` with the error. `hydra models [--provider P] [--json]` prints the file and its
age and launches nothing. The launchd template and its README are in `integrations/models/`,
04:00 and 16:00, no `RunAtLoad`. 16 tests in `tests/models.rs`, none running a real CLI.

**Run for real by the session:** `cursor-agent` 246 models, `agy` 18, `codex` 10, `claude` 3.
The gated run could not write the file (its worktree may not write the data root), which is the
design.

**Assumed and false:** `cursor-agent models` ends with a line that is not `id - name`; the parser
skips such lines once it has rows. **Left open:** `claude` shows only `haiku`, `sonnet` and
`opus`, because `aliases.json` has no canonical row for Fable; adding one is a change to the
alias table, which the grader and puppets also read. The launchd job is not installed; that is
the human's.
