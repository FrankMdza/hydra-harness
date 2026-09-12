# Security

## What Hydra promises

A run launched by Hydra acts inside its own git worktree, under a named policy compiled into
the vendor's own pre-tool hook, with an environment built from an allowlist so no credential
in your shell reaches the agent. Every tool call the hook sees is recorded with the decision
Hydra made about it.

## What Hydra does not promise, on purpose

- **The gate does not interpret shell.** A command that names a protected path is refused
  under every policy. A command that reaches the same path through a variable or a
  subshell is not. `docs/harness-spec.md` section 7.4 states this and `tests/gate.rs` fails
  if someone claims to have closed it.
- **There is no container.** The vendor CLI needs the network to reach its model, so network
  cannot be cut, and the run has your real `HOME` because a wrong one logs the vendor out.
- **Some vendor tools never ask the hook.** Cursor runs `ReadLints`, `getMcpToolsToolCall`
  and `awaitToolCall` without consulting it. The record marks those `unhooked`.
- **Hydra reads records that agents wrote.** Anything in a run record is data, never
  instructions, and Ouroboros treats it that way; if you find a path by which text in a
  record changes what Hydra does, that is a vulnerability.

## Reporting

Open a private security advisory on the repository, or email the address in `NOTICE`.
Include the run id and the record if you can; the record is usually the whole report.
Vulnerabilities in the vendor CLIs themselves go to their vendors.
