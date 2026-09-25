# The Herdr bridge

Map ticket 44 made Herdr the interactive layer and said nothing crosses from Hydra to it. This
directory is the other direction: a Herdr plugin that launches `hydra run` in a pane and labels
the pane from the run record. Nothing under `src/` imports it, and Hydra works with Herdr
stopped. If Herdr changes its API, this directory is the only thing that breaks.

## What is here

- `herdr-plugin.toml`, the manifest. Two actions, `run` and `interactive`, and two plugin panes,
  `bridge` and `mascot`.
- `bin/hydra-in-pane`, run inside a pane. Takes `hydra run` arguments verbatim, launches the run
  with `--timeout 0`, puts `run_id`, `provider` and `policy` on the pane as tokens, reports
  `working`, and then replaces itself with `hydra logs --follow` so the pane shows the stream.
- `bin/hydra-tui-in-pane`, run inside a pane. Takes `hydra run --interactive` arguments verbatim,
  keeps the terminal for the vendor's own interface, tees stderr to catch the run id, and puts the
  same tokens on the pane plus `launch_mode=interactive`. The record stays thin until the harvest
  at exit, so the bridge shows the pane alive with no tool names until then.
- `hy.zsh`, a shell function to source from `~/.zshrc`: with no arguments `hy` asks for a provider
  and a model; `hy [model] [policy]` still launches `claude` directly for muscle memory. Only
  providers in `interactive-gate-providers.txt` are offered, those whose interactive gate has been
  measured in their own TUI (`claude` today). Defaults are `sonnet` and `standard`.
- `bin/hydra-bridge`, a poller. Every two seconds it lists the panes carrying a `run_id` token,
  reads `hydra status` for each, and reports on change. The state table at the top of the file
  is the whole policy. Resolved 2026-09-07, map ticket 49.
- `bin/hydra-mascot`, the HydraPet dragon. With no mode flag it prints one pet frame; `--watch`
  lists Herdr panes once a second and drives the three heads from their tokens. It never calls
  `hydra status` and never reads `~/hydra/runs/`. Interactive panes (`launch_mode=interactive`)
  keep a head awake but never fire, because the record stays thin until harvest.
- `tests/test_mapping.py`, the table checked without Herdr or Hydra.
- `tests/test_mascot.py`, the watch mapping checked without Herdr or Hydra.

## Install

```
herdr plugin link /Users/USER/hydra/integrations/herdr
herdr plugin list
```

Both scripts expect `herdr` on the `PATH` of the pane, and `hydra` either on the `PATH` or named
by `HYDRA_BIN`, since a shell alias does not reach a script. The bridge and mascot panes are opened from Herdr's plugin pane list, or by hand:

```
herdr plugin pane open --plugin hydra.bridge --entrypoint bridge --placement split
herdr plugin pane open --plugin hydra.bridge --entrypoint mascot --placement split
```

The mascot pane needs the bridge pane running on a live run so the pane tokens carry `denials`
and `tool_rate`. Split a run pane beside them with `hydra-in-pane`, wait for the bridge to label
it `live`, and one head should wake.

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

The bridge puts these tokens on every pane it watches: `run_id`, `provider`, `policy`, `state`,
`outcome` once there is one, `alive` as `yes` or `no`, `last_tool` as the latest tool name and
seq from `hydra show --events`, `denials` as the count of `permission_decision` events with
`decision: deny`, `tool_rate` as the number of `tool_call` events in the last thirty seconds,
`elapsed` as seconds since `started_at`, and `tokens` as the projection total once the run ended.
It also sets a title from the provider, the short run id, and the outcome or the last tool name.

On a live interactive `claude` run the record stays thin until harvest, so the bridge tails that
session's append-only transcript under `~/.claude/projects/<slug>/` read-only, counting error
`tool_result` lines that carry Hydra's gate voice the same way the normalizer does. A missing or
unreadable transcript leaves `denials` unset. A Claude Code format change breaks the mascot's
refusal pose, not Hydra's record or gate.

## Test

```
python3 -m unittest discover -s integrations/herdr/tests
```

## Smoke test

`HYDRA_BIN=$PWD/target/release/hydra integrations/herdr/tests/smoke.sh` drives the bridge against a live Herdr session. It splits a pane, launches one `claude` haiku run through `hydra-in-pane`, waits for the bridge to label the pane `completed`, and closes what it opened. It spends one haiku call and needs Herdr running.

## After editing the bridge

A plugin pane runs the script it was opened with. After a change to `bin/hydra-bridge`, close
the pane and open it again:

```
herdr plugin pane close <pane_id>
herdr plugin pane open --plugin hydra.bridge --entrypoint bridge --placement split --env HYDRA_BIN=/Users/USER/hydra/target/release/hydra
```

The smoke test opens a bridge pane only if none is open, and closes the one it opened.
