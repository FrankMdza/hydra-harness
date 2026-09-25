# 129 - `hy` asks which provider and which model, and offers only a measured gate

Type: build
Status: resolved
Blocked by:

## What

Map ticket 76-F, resolved 2026-09-24. Read its `## Answer, the rest` first.

`hy` in `integrations/herdr/hy.zsh` takes `[model] [policy]` and always launches
`hydra-tui-in-pane --provider claude`. The human wants `hy` to ask for the provider and the
model. The answer limits the choice: `hy` offers only providers whose gate has been measured in
their own interactive TUI. Today that list is `claude` alone. `hydra run --interactive` refuses
every other provider in `src/main.rs` near line 26.

## Do

1. One list of providers with a measured interactive gate, in one place that both `hy` and the
   tests read. A plain file under `integrations/herdr/` is fine. It holds `claude` today, and
   builds 130 to 132's pattern adds a line per provider. A comment says what "measured" means:
   a build ticket's Outcome that shows a gate refusal arriving in that provider's TUI.
2. `hy` with no arguments prompts for the provider (from that list) and then the model, with
   `sonnet` as the default for `claude`. A provider's model list comes from
   `integrations/puppets/vendors.json` rows for that provider, plus free text. `hy <model>
   [policy]` keeps working as it does now for `claude`, so muscle memory does not break.
3. A provider not on the list is not offered. If someone types one anyway, `hy` says why in one
   sentence and names the ticket that would add it.
4. `integrations/herdr/README.md`: the paragraph on `hy` says it asks, and what the list is.

This file is the other session's territory (Master of Puppets, `hy`, the mascot). Check
`git log -3 -- integrations/herdr/hy.zsh` before starting, and do not reformat what you do not
change.

## Done when

- A test under `integrations/herdr/tests/` drives the prompt with piped input: picking `claude`
  and a model builds the same `hydra-tui-in-pane` command line `hy sonnet standard` builds today,
  and naming `cursor-agent` refuses with the reason. No Herdr and no provider are launched; stub
  `herdr` and `hydra-tui-in-pane` on `PATH`.
- `bash -n` and `zsh -n` on `hy.zsh` pass, and the existing Herdr tests pass.

## Outcome

Resolved 2026-09-24. Built by `composer-2.5` on run 01a0d618-3015 under `standard`, about 0.61M
tokens. The session applied the diff with one change.

`integrations/herdr/interactive-gate-providers.txt` is the list of providers with a measured
interactive gate, one per line, and holds `claude`. Its comment says what "measured" means and
that build 130 adds the others. `hy` with no arguments prompts for a provider from that list and
then a model, listing that provider's rows from `integrations/puppets/vendors.json` and accepting
free text. `hy <model> [policy]` still launches `claude` as before. Naming `cursor-agent`, `agy`
or `codex` refuses in one line that points at build 130. `HYDRA_HERDR` overrides the directory so
the tests can run from a worktree.

The session's change: an empty model answer fell back to `sonnet` for every provider. It now
defaults only for `claude`, and asks for a name otherwise, so the first provider build 130 adds
does not launch with a Claude model name.

`integrations/herdr/tests/test_hy.py` stubs `herdr` on `PATH`. `hy sonnet standard` builds the
same `hydra-tui-in-pane` line as before; the prompt path picking `claude` and a model builds it
too; `hy cursor-agent` refuses. The refusal test failed before the change, when `hy cursor-agent`
took `cursor-agent` as a model name and launched. On master: 22 Herdr tests pass, and `zsh -n` and
`bash -n` pass on `hy.zsh`.
