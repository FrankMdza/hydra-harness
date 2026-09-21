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
- **Two shell facilities are denied under every policy, since 2026-09-15, and the cost is a feature.**
  `WriteShellStdin` on `cursor-agent` and `send_command_input` on `agy` write to the stdin of a
  command that is already running. The command was judged when it started and what is typed into it
  afterwards was not, because neither row carries a command string for the gate to read. They were
  ordinary shell rows until build ticket 59, which meant a run under `permissive` could feed a
  protected path to a shell it had already opened. No policy grants them now, so a run on those two
  providers cannot answer an interactive prompt. That is a real loss and it is the right side of the
  trade until the gate can see what is being typed.
- **A run cannot relaunch the binary and name its own policy, since 2026-09-15, and the stop is a
  marker rather than a wall.** Harness spec 7.3 says a child is never wider than its parent and
  that a human at depth 0 is the only thing that widens anything. Until build ticket 57 nothing
  enforced it: `standard` grants shell, shell reaches the binary, and `hydra run --policy
  permissive` with no `--parent` was a child bounded by nothing. No forging was needed, the flag
  was simply absent, and the adversarial review measured it working. Hydra sets `HYDRA_RUN_ID` in
  every gated child's environment now, and a launch carrying it without a `--parent` naming the
  same live run writes a `containment` refusal instead of starting. **A child that unsets the
  variable walks through this**, because a variable is a thing a process removes from its own
  children's environment. What the check closes is the absent flag, which was the whole of the
  attack as it was measured; what it does not close is a child that means it. Map ticket 62 asks
  whether this class wants a second uid and file modes instead, which would be the first thing in
  this design to need something from the machine.
- **On `codex` the gate rides the command line, and a hook that dies without answering fails
  open, since 2026-09-21.** Codex reads no project hook file inside a git worktree, and every
  Hydra run is one, so the gate is handed over as a `-c` override naming a Python file in the
  spawn directory, which every policy denies writes to, and the launch carries
  `--dangerously-bypass-hook-trust` because a per-run hook can never be a hash a human trusted.
  Measured on 0.154.0: a hook that exits non-zero with no JSON answer lets the call through. The
  sha check answers a tamper with a deny rather than an exit code for that reason, and a Python
  crash inside the gate on this provider is a call that runs. The run record still shows every
  call the hook answered; it cannot show the one it crashed on.
- **The gate is only as good as the vendor build.** It lives in the vendor's own hook dialect,
  and vendors have shipped bugs where hooks did not fire in print mode. The record proves the
  gate was consulted on every call it saw; it does not prove nothing else ran. `tests/drift.rs`
  is where a confirmed change becomes a failing test.

## Vendor terms

Hydra spawns each vendor's unmodified binary under your own subscription and never reads,
stores or forwards its token. That is the documented path for scripted use on all four. Two
things the terms do forbid, and Hydra cannot enforce for you: training a competing model on a
vendor's outputs, which is what `hydra export` could feed; and volume beyond "ordinary,
individual usage", which is the stated reason for the 2026 cutoffs of unattended loops. Read
the terms before running Ouroboros overnight.

## Reporting

Open a private security advisory on the repository, or email the address in `NOTICE`.
Include the run id and the record if you can; the record is usually the whole report.
Vulnerabilities in the vendor CLIs themselves go to their vendors.
