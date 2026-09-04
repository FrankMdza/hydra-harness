# 30 - When two hooks answer, does deny win?

Type: research
Status: resolved
Blocked by: 26

## Question

Ticket 27's composition decision rests entirely on this and nobody has measured it.

Hydra writes a gate config per run. The user's own hooks load too, and Hydra cannot stop them on two
of three providers: `cursor-agent` always reads `~/.cursor/hooks.json` and `~/.claude/settings.json`,
and `agy`'s global and shared config live in the user's home. Ticket 21 ruled out moving `HOME`,
since a wrong one silently logs three providers out. Only `claude --restricted` gives a clean slate.

Ticket 27 accepted composition on the grounds that the user's hooks can only tighten Hydra's floor
and never widen it. **Ticket 26 probed one hook at a time and never two at once, so that grounds is
an assumption.**

Answer for `claude` 2.1.257, `cursor-agent` and `agy`, re-reading every version first since two of
the three self-update:

- **Two hooks on one tool call, one denying and one allowing.** Which answer the provider takes, and
  whether the order they are declared in changes it.
- **Whether scope changes the outcome.** A user-scope allow against a project-scope deny, and the
  reverse. Cursor's loader has seven config locations with a precedence order; find out whether that
  order decides the permission or only the loading.
- **Whether a hook can widen at all.** On `claude` a `PreToolUse` deny beats `bypassPermissions`
  (ticket 26). Find out whether a `PreToolUse` allow beats a denial that would otherwise have come
  from the permission layer, because a hook that can only ever subtract is a different security
  object from one that can add.
- **What Cursor's dedupe does to this.** Ticket 26 found Cursor drops any Claude-format hook whose
  command string matches one already declared in a Cursor-format config. Two hooks that share a
  command string but sit in different scopes may collapse to one, which would decide the question by
  accident.
- **`agy`'s `hook_deny_unless_prior_grant`.** Ticket 26 found `ask` and `force_ask` collapse to deny
  for a detached child. Check whether a prior grant from the user's own config counts as the grant
  that reopens them.

## Why this blocks ticket 16

If deny wins, ticket 27's answer stands as written and the harness spec says the user's hooks
compose with Hydra's.

If an allow can win, ticket 27 inverts. Composition stops being safe, `--restricted` becomes
mandatory on `claude`, and ticket 22's rule takes over for the other two: a provider Hydra cannot
gate is either given a config root Hydra owns or is not routed. That would cut the routable set to
one provider or force a config-root mechanism nobody has designed, which is a spec change rather
than a paragraph.

Read the `## Answer` on ticket 27 and
[research/26-hook-level-action-gate.md](../research/26-hook-level-action-gate.md) first. Ticket 26's
probe rig and its exact commands are in there; this is the probe it did not run.

Output is a findings file at `research/30-hook-precedence-deny-vs-allow.md`. Do not modify the
user's real hook configs; `~/.claude/settings.json` is live and holds their own guards.

## Answer, 2026-09-03

**Deny wins on all three, and the result is stronger than composition needed: a hook cannot grant
anything on any provider.** A hook is a veto and never a key. Ticket 27 stands as written.
`--restricted` does not become mandatory, ticket 22's rule is not triggered, and the routable set
stays at three.

Every command, the raw frames and the per-provider per-question detail are in
[research/30-hook-precedence-deny-vs-allow.md](../research/30-hook-precedence-deny-vs-allow.md),
43 KB, measured 19:00 to 21:45 local. Versions were read before the first probe and after the last
and were identical both times: `claude` 2.1.257, `cursor-agent` 2026.09.02-c22c1a3, `agy` 1.1.25.
Ticket 26 probed `agy` 1.1.24, so every `agy` claim here was re-measured rather than inherited.

### Deny against allow

- `claude` 2.1.257: a hook `allow` loses to a `permissions.deny` rule and to a `permissions.ask`
  rule. The shipped code says why. A hook deny returns immediately, a hook allow is still checked
  against the rules, and the comment beside it reads `"but deny rule overrides"`. Under
  `--restricted` the allow is discarded before it is considered at all.
- `cursor-agent` 2026.09.02-c22c1a3: no code path reads `"allow"`. The merge computes it and nothing
  consults the result.
- `agy` 1.1.25: a hook allow did not lift the headless auto-deny on a file write.

Declaration order does not change it. Two of the three ship the rule as readable code and both spell
it the same way: claude's `switch` assigns `deny` unconditionally and guards every other branch,
Cursor's merge is a symmetric `deny > ask > allow` ternary. `agy` is a Go binary and was measured
instead, with deny winning in both handler orders and short-circuiting the rest.

**Both live counterparty runs came out the right way.** The user's real `~/.claude/settings.json`
guards, which deny by exit code 2, beat a command-line JSON allow inside `claude` and beat a
project-scope allow inside `cursor-agent`, which reads that same file.

### Probe hygiene

Everything created for this ticket lives under the session scratchpad, prefixed `t30-`. No user-scope
config was written. Verified independently after the run: `~/.claude/settings.json` still carries its
2026-08-31 22:57 mtime and sha1 `7fd1e3d3637f3f62dc77d77268afebba015ae4a5`, and neither
`~/.cursor/hooks.json` nor `~/.gemini/config/hooks.json` exists.

### Six things it opened

1. **`--setting-sources project` drops the user's hooks while keeping all 25 tools**,
   `bypassPermissions` and Hydra's own `--settings` hooks. Ticket 27 knew only about `--restricted`,
   which strips Bash, and chose composition partly because isolation cost too much. Isolation on
   `claude` is now a choice rather than a trade. Graduated as ticket 31.
2. **The recorded decider is not always Hydra's.** On `agy` the first deny short-circuits, so Hydra's
   hook may never fire and the reason belongs to the user's, and ordering across separately named
   hooks is not file order. On Cursor every hook's message is concatenated, and one probe reached the
   model as `Rejected: t30 CUR-PROJ says allow`, quoting the hook that allowed. Ticket 09 gains a
   decider field Hydra sets rather than one parsed out of a reason string.
3. **Ticket 26's Cursor path table has a wrong row.** The team hook file is
   `~/.cursor/managed/active-team-hooks/hooks.json`, built from `homedir()` in `4274.index.js`, not
   from the workspace. Only three of the seven locations sit inside a directory Hydra creates.
   Corrected on ticket 26.
4. **Ticket 26's "`ask` collapses to deny" needs narrowing.** That is the permission layer's answer
   under `request-review`, not the hook's. With `--dangerously-skip-permissions` an `ask` resolved to
   allow and the command ran, while a deny still beat it. Corrected on ticket 26.
5. **Cursor accepts Claude's response dialect on `preToolUse`**, so the shared script answers in two
   shapes rather than three. Passing the dialect explicitly stays right for `agy` and for claude's
   fail-open.
6. **`agy` names one action two ways**, `write_to_file` as a step and `write_file` as a permission
   rule. Ticket 27's capability table needs both.

### Unprobed, with reasons

User-scope allow hooks on `claude` and Cursor, Cursor's enterprise and team scopes, `agy`'s
home-directory hook and settings files, and an `agy` prior grant from the user's own
`permissions.allow`. All three permission scopes are empty today and all of these live in the user's
home, which this ticket was barred from writing. Each is covered either by source that carries no
notion of a hook's source, or by an equivalent probe from a directory Hydra owns. The findings file
lists them one by one.
