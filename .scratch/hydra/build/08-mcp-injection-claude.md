# 08 - MCP injection on `claude`

Type: build
Status: open
Blocked by: 02

## Goal

One canonical MCP document generating the per-provider file, `claude` first, with the
per-policy admission field and the launch check.

## Where the design says it

- Harness spec 6.5. The canonical document that is none of the three files, the
  admission field keyed on a policy name, `--mcp-config` plus `--strict-mcp-config` on
  recorded runs and without the strict flag interactively, and the two accepted costs.
- 6.1 item 6, the launch check, and `mcp_unavailable` as its own outcome.
- 15.1 config #5: an MCP server definition is a command Hydra spawns, so this document
  is protected and a run that edits it runs code inside every future run.
- Contract 5.3, `mcp.doc_sha`, `mcp.injected` and `mcp.resolved`, which are identical on
  a recorded `claude` run and can differ everywhere else.

## Done when

- The canonical document exists, is protected at run time under every policy, and
  generates `~/hydra/spawn/<run-id>/mcp.json`.
- A server whose admission field excludes the run's policy is not written.
- The launch check fails the run when the policy admits a server this launch cannot
  deliver.
- `run_start.mcp` stops being absent.

## Watch for

**Claude's MCP config goes on the command line and its skills go in the worktree, and
the asymmetry is deliberate.** A reader who expects the two to match will go looking for
a bug. `--strict-mcp-config` is what makes the server set an argument Hydra wrote rather
than the state of five directories at spawn time, which is what lets the record claim
the set and be right.

**A recorded `claude` run cannot reach the human's own user-scope servers at all.** That
is one of the two accepted costs, not a bug to fix later.

## What ticket 02 left you, added 2026-09-05

**The spawn directory exists and `mcp.json` is the file that is missing from it.**
`Layout::spawn(run_id)` is the directory, and ticket 02 writes `gate` and
`settings.json` into it through `hydra::run::write_spawn_config`. That function is where
the MCP config joins them.

**`--mcp-config` and `--strict-mcp-config` are held back on purpose**, not forgotten.
`src/provider/claude.rs` has a note saying so, and its `the_mcp_flags_wait_for_ticket_08`
test fails the moment you add them, which is the reminder to delete the test with the
same change. Both go on a non-`judge` run only: `--restricted` drops MCP anyway, so the
`judge` shape stays as it is.

**Protected config #5 has no file yet.** The gate denies writes to `config/mcp.md` under
every policy and nothing has written it. `config/README.md` is the table, and ticket 02
wrote `config/preamble.md` next to it as the worked example of what one of these looks
like once it is real.

**The hook command is no longer just a path.** It is a shell one-liner that checks the
gate against a sha the config carries and then runs it, so a rewritten script fails
closed. `claude::gate_command` builds it. Nothing about MCP touches that, but a reader
expecting `settings.json` to name the script directly will not find it.

## What ticket 07 left you, added 2026-09-06

**A pre-flight that refuses a launch now has a shape to copy.**
`hydra::run::refuse_for_collision` writes line 0, writes `RunEnd::refused`, reads the
record back through the normalizer and returns contract 17's projection. Your
`mcp_unavailable` check is the same three steps with a different code, and 6.1 puts it
one item after the skill collision check. `run.rs` already runs both halves of that
sequence in order: skills, then the refusal branch, then `write_spawn_config`.

**The launch already runs one free `claude` process before the spawn.** Ticket 07 asks
the provider what skills it resolved with `claude -p x --model <unknown> --session-id
<uuid>` in the checkout, measured at 1.8s on 2.1.263. That probe loads the user's
settings, so it starts whatever MCP servers they have configured. Nothing on this machine
has any, so the 1.8s is a floor rather than the number you will see once `--mcp-config`
exists. If the probe becomes slow, `--strict-mcp-config` on the probe is the obvious
lever and it changes what the probe measures, so decide it rather than reach for it.

**`skills.resolved` is the pattern `mcp.resolved` should follow.** Contract 5.3 asks both
groups the same question and ticket 07 answered it for skills by asking the provider and
storing null when it could not be asked. An empty array would claim the run got nothing,
which is a different fact.
