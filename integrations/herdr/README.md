# The Herdr bridge

Map ticket 44 made Herdr the interactive layer and said nothing crosses from Hydra to it. This
directory is the other direction: a Herdr plugin that launches `hydra run` in a pane and labels
the pane from the run record. Nothing under `src/` imports it, and Hydra works with Herdr
stopped. If Herdr changes its API, this directory is the only thing that breaks.

## What is here

- `herdr-plugin.toml`, the manifest. One action, `run`, and one plugin pane, `bridge`.
- `bin/hydra-in-pane`, run inside a pane. Takes `hydra run` arguments verbatim, launches the run
  with `--timeout 0`, puts `run_id`, `provider` and `policy` on the pane as tokens, reports
  `working`, and then replaces itself with `hydra logs --follow` so the pane shows the stream.
- `bin/hydra-bridge`, a poller. Every two seconds it lists the panes carrying a `run_id` token,
  reads `hydra status` for each, and reports on change. The state table at the top of the file
  is the whole policy, and map ticket 49 names the two rows it may move.
- `tests/test_mapping.py`, the table checked without Herdr or Hydra.

## Install

```
herdr plugin link /Users/USER/hydra/integrations/herdr
herdr plugin list
```

Both scripts expect `herdr` on the `PATH` of the pane, and `hydra` either on the `PATH` or named
by `HYDRA_BIN`, since a shell alias does not reach a script. The bridge pane is opened from
Herdr's plugin pane list, or by hand:

```
herdr plugin pane open --plugin hydra.bridge --entrypoint bridge --placement split
```

## Launch a run from a pane

Inside any Herdr pane whose cwd is the repo you want the run to work on:

```
hydra-in-pane --provider claude --model haiku --task "..."
```

The pane's cwd decides whether the run gets a worktree. From a directory that is not a repo the
run records `worktree: null` and there is no branch for Hunk to open.

## How the state reaches the pane

Herdr holds four pane states, `idle`, `working`, `blocked` and `unknown`, and refuses any other
name. Hydra's outcomes go on as display labels over one of those four, so the pane says
`gated` while Herdr's own state is `blocked`. Herdr produces no reading of its own for a Hydra
run, because the provider's interface lives on a pty Hydra owns, so this bridge is the only
source of the pane's state rather than a correction of Herdr's.

## Test

```
python3 -m unittest discover -s integrations/herdr/tests
```

## Smoke test

`HYDRA_BIN=$PWD/target/release/hydra integrations/herdr/tests/smoke.sh` drives the bridge against a live Herdr session. It splits a pane, launches one `claude` haiku run through `hydra-in-pane`, waits for the bridge to label the pane `completed`, and closes what it opened. It spends one haiku call and needs Herdr running.
