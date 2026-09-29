# 145 - `hy` asks for the policy, and takes a provider by name

Type: build
Status: resolved
Blocked by:

## What

Build 129 made `hy` ask for the provider and the model. It does not ask for the policy. In
`integrations/herdr/hy.zsh`, with no arguments `hy` sets `policy="standard"`, and with arguments it
sets `provider="claude"` and reads `[model] [policy]`. So `hy opus read-only` works, no menu offers
a policy, and no spelling of the command names another provider. The human asked on 2026-09-29 for
a session he starts by picking provider, model and policy.

The list of providers with a measured interactive gate stays what build 129 made it:
`integrations/herdr/interactive-gate-providers.txt`, which holds `claude`. This ticket adds no
provider. Build 130 does that.

## Do

1. With no arguments, after the provider and the model, `hy` asks for the policy. The choices are
   the policy files in `policies/` beside the checkout, by name, without `floor.json`, which is the
   pattern floor and not a policy. `standard` is the default on an empty answer. A name that is not
   a policy file is refused in one sentence that lists the valid names.
2. `hy <provider> <model> <policy>`, three arguments, launches that provider. A provider that is
   not on the measured list is refused with the sentence build 129 already prints.
3. `hy <model> [policy]`, one or two arguments, keeps launching `claude` exactly as it does today.
   The first argument of a one or two argument call is never read as a provider to launch; build
   129's refusal of a bare provider name there stays.
4. `integrations/herdr/README.md`: the paragraph on `hy` names the three forms and says where the
   policy list comes from.

Do not reformat what you do not change. Check `git log -3 -- integrations/herdr/hy.zsh` first.

## Done when

- Tests in `integrations/herdr/tests/test_hy.py` drive the prompt with piped input, with `herdr`
  and `hydra-tui-in-pane` stubbed on `PATH`, launching no Herdr and no provider: picking `claude`,
  `opus` and `read-only` builds `--provider claude --model opus --policy read-only`; an empty
  policy answer builds `--policy standard`; `hy claude opus judge` builds the same line the prompt
  would; `hy opus read-only` builds what it builds today; `hy codex gpt-6-astra standard` refuses;
  a policy name with no file refuses and lists the valid names. Each test failed before the change.
- `bash -n` and `zsh -n` on `hy.zsh` pass, and the existing Herdr tests pass.
- The report says whether `hydra run --interactive` was seen to accept each policy name, or that
  the run could not check it and where it looked.

## Outcome

Resolved 2026-09-29. Built by `composer-2.5` on run 01a0eba6-54a0 under `standard`, about 0.29M
tokens. The session applied the diff with one change.

With no arguments `hy` asks for the provider, the model and then the policy, listing the files in
`policies/` without `floor.json`. An empty answer is `standard`, and a number picks from the list.
`hy <provider> <model> <policy>` launches that provider when it is on the measured list, and
refuses `codex`, `cursor-agent` and `agy` with build 129's sentence. `hy <model> [policy]` still
launches `claude`. Every form now refuses a policy name with no file and lists the valid ones:
`curate`, `judge`, `permissive`, `read-only`, `standard`.

The session's change: a policy is a name and never a path. `hy opus ../config/panels` passed the
file test as the run wrote it, because `policies/../config/panels.json` exists. A name with a
slash is now refused.

The run added six tests to `integrations/herdr/tests/test_hy.py` and changed the piped input of
one. It showed three of them failing before the change. It did not run two of the others on the
old tree, and `test_hy_opus_read_only_still_claude` passed before, as a guard on the two argument
form. So "each test failed before the change" holds for three of six. On master `bash -n` and
`zsh -n` pass and the Herdr tests are 31 of 31.

Not confirmed: whether `hydra run --interactive` accepts each of the five policies. The run
launched nothing. `IMPLEMENTATION.md` records `read-only` measured, and `standard` ran live on
2026-09-29 (run 01a0eb99). `curate`, `judge` and `permissive` are offered by the menu and not
measured in a session.
