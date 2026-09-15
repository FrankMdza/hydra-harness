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

## Known exposures, stated before someone else does

- **A run's own hook config is denied to it, since 2026-09-12.** On `cursor-agent` the config
  Hydra writes is `<worktree>/.cursor/hooks.json`, and `standard` may write anywhere in the
  worktree, so before build ticket 44 a run could rewrite the file that points at its gate.
  Those paths are refused under every policy now, on every provider's gate. The skill roots
  Hydra links are denied to writes and stay readable, because a shell command naming one is how
  two of the three providers read a skill.
- **The gate is only as good as the vendor build.** It lives in the vendor's own hook dialect,
  and vendors have shipped bugs where hooks did not fire in print mode. The record proves the
  gate was consulted on every call it saw; it does not prove nothing else ran. `tests/drift.rs`
  is where a confirmed change becomes a failing test.

## Vendor terms

Hydra spawns each vendor's unmodified binary under your own subscription and never reads,
stores or forwards its token. That is the documented path for scripted use on all three. Two
things the terms do forbid, and Hydra cannot enforce for you: training a competing model on a
vendor's outputs, which is what `hydra export` could feed; and volume beyond "ordinary,
individual usage", which is the stated reason for the 2026 cutoffs of unattended loops. Read
the terms before running Ouroboros overnight.

## Reporting

Open a private security advisory on the repository, or email the address in `NOTICE`.
Include the run id and the record if you can; the record is usually the whole report.
Vulnerabilities in the vendor CLIs themselves go to their vendors.
