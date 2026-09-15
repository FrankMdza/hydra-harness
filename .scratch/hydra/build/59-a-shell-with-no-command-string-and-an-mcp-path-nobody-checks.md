# 59 - Two shell tools carry no command, and an MCP path never meets the deny list

Type: build
Status: open
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
