# 08 - MCP injection on `claude`

Type: build
Status: resolved
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

## Outcome, 2026-09-06

All four bullets are built, on `claude` 2.1.263. `config/mcp.md` is protected config #5 and
the only place a server is declared, `src/mcp.rs` reads it and decides what one launch gets,
`src/provider/claude.rs` turns that into `~/hydra/spawn/<run-id>/mcp.json`, and both flags
ride a non-`judge` recorded run. `tests/mcp.rs` is new and drives the whole path without a
provider. 290 tests pass, up from 261.

**The document's format.** Markdown with prose and exactly one fenced `toml` block, and the
block is the data. Markdown because a human edits this file by hand and the rules belong
beside the servers; one block rather than one per server, so a broken fence is an error
rather than a launch that silently admits nothing. A server carries `name`, `transport`
(`stdio`, `http` or `sse`), `admit` as a list of policy names, and then `command`/`args`/`env`
or `url`/`headers`. Four things are refused at parse time and each would otherwise be silent:
an unknown field, an `admit` entry that is not one of the five policies, a name carrying `__`,
and a server naming both a command and a url. Ticket 23's per-provider differences all fit:
the stdio body is shared, and `transport` is what tells `type`+`url` from `url` from
`serverUrl` when tickets 09 and 10 generate their files.

**`doc_sha` is a sha256 of the whole file, prose included.** So `shasum -a 256 config/mcp.md`
reproduces what a record carries. The alternative, hashing only the block so a wording fix does
not move the value, is what `policy_sha` does for `policies/README.md`; it loses here because
there the prose is a separate file, and a sha over part of a file cannot be checked by hand.

**What the document holds on this machine: no servers.** Nobody has an MCP server configured
here, so it ships with the format and one commented example per transport. Two tests assert
that emptiness so the day a human adds one, the suite says which tests then describe something
else.

**The shapes that make a server undeliverable, and there is one today.** `judge`. 6.7 gives it
`--restricted` and no `--mcp-config`, so nothing Hydra writes can reach it, and a document
admitting a server to `judge` fails the launch with `mcp_unavailable` before anything spawns.
Everything else delivers what it admits. Two shapes that turned out not to be shortfalls: a run
with no checkout, since on `claude` the servers ride the command line rather than the worktree,
and a server admitted to a policy that does not grant `mcp_call`, which is four of the five.
That last one delivers the server and refuses every call to it, which is ticket 27's decision
rather than a shortfall, and it is now on a record.

**`run_start.mcp` carries `doc_sha`, `injected` and `resolved`, and one probe answers both
`resolved` fields.** Ticket 07's free probe already ran per launch; it now carries the run's own
`--mcp-config` and `--strict-mcp-config`, and its transcript's `deferred_tools_delta` attachment
holds the answer. `addedNames` carries one entry per MCP tool as `mcp__<server>__<tool>`, which
is why the document refuses a server name containing `__`, and the same attachment carries
`pendingMcpServers`, `needsAuthMcpServers` and `failedMcpServers`. Measured on 2.1.263: 1.8s with
an empty config, 1.9s with one server, against ticket 07's 1.8s with no MCP flags.

**Line 0 does not wait for the init frame, and does not have to.** The ticket asked what happens
if the only honest source arrives after line 0 is written. It does not: the probe runs before
line 0 and answers under the run's own MCP flags. The run's init frame turned out to carry
`mcp_servers` with a `status` per server as well, which corroborates the probe from the record
rather than replacing it. Deciding `--strict-mcp-config` on the probe was the open question
ticket 07 left; it is taken, because without it the probe resolves the human's servers and the
record would claim them for a run that will not have them. The cost is that an admitted server
starts twice per launch.

**`resolved` is null on a `judge` run**, a second producer for that null beside a probe that
failed. The probe is not `--restricted`, so it cannot reproduce what a `judge` run resolves, and
an empty array would be a claim rather than a measurement.

**Measured on two real runs, `--model haiku`, 2026-09-06.** Run
`01a077c3-e32f-7612-bb19-89227bf6f531`, `standard`, on the shipped document: both flags
accepted, `mcp.json` was `{"mcpServers":{}}`, the init frame said `mcp_servers: []`, the record
carried the sha and two empty lists, 6.8s end to end. Run
`01a077c4-68e9-7203-9cc9-030044661116`, `standard`, with one stdio server added to the document
by hand for the measurement and taken out again: `injected` and `resolved` both
`["hydra-probe"]`, the init frame said `[{"name":"hydra-probe","status":"connected"}]` and
listed `mcp__hydra-probe__ping`, and the model's call to that tool came back
`hydra/capability: policy standard does not allow mcp_call (mcp__hydra-probe__ping).` 12.6s end
to end. The server is `tests/fixtures/mcp-probe-server.py`, which cost about twenty minutes to
write and answers the MCP handshake and nothing else.

**One thing the ticket assumed that turned out false, and it needs a decision.**
`--restricted` does not drop MCP on 2.1.263. The ticket, 6.5 and 6.7 all rest on ticket 23's
measurement of 2.1.257, and the probe now shows `claude --restricted --mcp-config <file>`
resolving the server in that file; 2.1.263's own help says "add --strict-mcp-config to skip MCP
servers too". So a `judge` run, which carries neither flag, keeps whatever the human has
configured. The gate refuses every `mcp__` call under `judge`, so what is left is server
processes starting and their tool descriptions reaching the context, which is ticket 27's
residue arriving on the provider 6.5 says is free of it. Not measurable further here, because
this machine has no user-scope server and writing one into the human's config is theirs to do.
The fix is one flag on the `judge` argv and 6.7 is what writes that argv, so it is left for the
map. `IMPLEMENTATION.md` has it under its own heading beside the code.

**Two smaller things.** The refusal path is now one function, `hydra::run::refuse`, shared by
6.1's items 5 and 6, which is what let a test drive the `mcp_unavailable` record without a
provider. And the contract validator now checks both injection groups' three fields rather than
only that neither is null, so `tests/record_contract.rs` says something about 5.3 rather than
about absence alone.
