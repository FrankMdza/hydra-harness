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
