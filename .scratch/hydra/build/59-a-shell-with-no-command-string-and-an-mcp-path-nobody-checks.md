# 59 - Two shell tools carry no command, and an MCP path never meets the deny list

Type: build
Status: resolved
Blocked by:

## What

Two findings that share a cause: `evaluate` in `src/policy/gate.py` reaches the path denials
of 7.5 through two doors only, a `file_write` with a path and a `shell` with a command string.

1. **Cursor's `WriteShellStdin` and `agy`'s `send_command_input` are `shell` rows with no
   `command_field`**, so `raw_command` is empty and both the floor and the path check are
   skipped. `table.rs` already notes the floor's blind spot here; the larger half is that
   "denied under every policy" goes with it. Either deny these rows under every policy until the
   gate can see what they feed, or map them to a capability no policy grants.
2. **`mcp_call` never meets `protected_root`.** Under `permissive` an MCP tool writing the
   corpus is allowed. `config/mcp.md` admits no writing server today, so nothing has fired;
   the gun is loaded.

Found by the adversarial review.

## Outcome

Both halves, and they wanted different answers.

**The two shell rows: a capability no policy grants.** The ticket offered "deny under every policy"
or "map to a capability no policy grants", and the second is the first said in the language the gate
already speaks. `Capability::OpaqueShell` carries `WriteShellStdin` and `send_command_input`, the
compiler emits the row rather than skipping it, and the gate answers `hydra/capability` by name
instead of `hydra/unknown-tool`. Five policies times two providers, all refused.

**The pair split.** `send_command_input` and `command_status` shared `Capability::Shell` and a
comment saying so. They are not the same risk: writing into a running command is a command the gate
never saw, and reading its output is not. `command_status` stays `shell`, where a policy that denies
shell still denies it and one that grants shell still grants it, and a test pins both directions.

**The cost, named rather than buried.** A run under any policy cannot answer an interactive prompt
on `cursor-agent` or `agy` now. That is a functional loss and `IMPLEMENTATION.md` says so. It comes
back when the gate can read what is being typed, which is a change to the two providers' payloads
rather than to Hydra.

**The MCP door: the raw pass, applied again.** `protected_path_named_in` walks every string in a
call's arguments at any depth and searches each for a denied spelling, the same coarse match build
ticket 56 gave the shell check and with the same residual, which is that nothing is resolved so a
relative path walks through. Measured: disable it and `mcp__writer__put` with the corpus as its
`path` comes back `hydra/allow: policy permissive allows mcp_call`. A call naming a path inside the
run's own worktree is still allowed, because `denied_prefixes` applies the own carve-out.

Gate suite 71 of 71, plus build ticket 58's one ignored test.
