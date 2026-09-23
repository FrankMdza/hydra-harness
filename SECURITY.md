# Security

## What Hydra promises

A run launched by Hydra acts inside its own git worktree, under a named policy compiled into
the vendor's own pre-tool hook, with an environment built from an allowlist so no credential
in your shell reaches the agent. Every tool call the hook sees is recorded with the decision
Hydra made about it.

## What Hydra does not promise, on purpose

- **The gate does not interpret shell.** A command that spells a protected path is refused
  under every policy. A command that reaches the same path through a variable, a subshell, a
  `cd` followed by a relative path, or a nested `sh -c` naming a sibling under a parent this
  run owns a piece of, is not. Neither is a path split by a quote, assembled by concatenation,
  percent-encoded, or spelled with `..`, `//` or `/./` inside an interpreter or nested-shell
  token. The gate refuses a protected path only when its prefix appears whole and in canonical
  form. `docs/harness-spec.md` section 7.4 states this and `tests/gate.rs` fails if someone
  claims to have closed it.
- **The gate folds case ASCII only, and Hydra refuses a root it would fold wrong.** The macOS
  filesystem folds `É` too, so `hydra run` refuses when its data root or its checkout resolves
  to a path outside ASCII. The launching repository's `.git` is folded the same way and is not
  checked: under a repository whose path holds a cased non-ASCII letter, the denial of writes to
  that `.git` misses the other case, and on macOS the seatbelt is what still refuses them.
- **There is no container, and on macOS there is a sandbox around the process tree.** The
  vendor CLI needs the network to reach its model, so network cannot be cut, and the run has your
  real `HOME` because a wrong one logs the vendor out. Since 2026-09-21 every recorded launch on
  macOS runs under the kernel's own seatbelt with the same user and `HOME`: for the launched
  process and every descendant, writes under Hydra's directories and reads of the corpus and of
  any other run's spawn directory fail with "operation not permitted" whatever the spelling, a
  variable, an interpreter or a symlink included, measured on all four providers and pinned in
  `tests/spawn.rs`. The profile is `(allow default)` plus those denials, so it is defence behind
  the gate and not a boundary: it does not restrict process launching or mach services, and a
  process that a system agent starts on the run's behalf, launchd for one, does not inherit it.
  One `launchctl submit` probe on 2026-09-22 did not reproduce that class; nothing says a second
  spelling would not. Map ticket 67 holds the question of what the seatbelt is for. The gate
  still decides what a policy allows and writes the record; the kernel is what refuses the
  write. A gated interactive launch, `hydra run --interactive --policy`, runs under the same
  profile since build ticket 73. Linux launches unwrapped.
- **The deny list is Hydra's runtime directories, the checkout the binary was built from, and five
  config files.** Build ticket 74 added `hydra_home()` whole, with the run's own worktree and a
  `curate` pass's checkout carved back out. Under `standard` a shell command still reads and writes
  the rest of the machine: `~/.ssh`, your own `~/.claude` hooks and settings, and any directory on
  `PATH` ahead of `/usr/bin`, which is where the vendor binary is resolved from. The honest
  sentence is still the one the 2026-09-14 review ended on: the gate bounds what a run may do to
  Hydra, not what it may do to you. Rung two's admission gate runs `cargo fmt --check`, `clippy`,
  `build` and `test` on a proposer's worktree under the proposer's own seatbelt profile since
  build ticket 75, with `HYDRA_ROOT` pointed at an empty scratch root, before any judge reads the
  patch; that profile bounds what the steps may do to Hydra and not what they may do to you, so a
  hostile test can still read `~/.ssh`, and the pass's harness record names the profile sha and
  the root the steps ran under.
- **Inside the seatbelt a run can stage its work and cannot move a branch.** The worktree's
  private git directory under `~/hydra/.git/worktrees/<run-id>` and the shared object store are
  carved out of the denial since build ticket 70, so `git status` and `git add` work; `refs/` and
  `HEAD` stay denied, so `git commit` and `git stash` fail with "operation not permitted" and no
  branch moves from inside a run. `hydra reap` commits from outside.
- **Some vendor tools never ask the hook.** Cursor runs `ReadLints`, `getMcpToolsToolCall`
  and `awaitToolCall` without consulting it. The record marks those `unhooked`.
- **Hydra reads records that agents wrote.** Anything in a run record is data, never
  instructions, and Rungs treats it that way; if you find a path by which text in a
  record changes what Hydra does, that is a vulnerability.
- **The checkout's own instruction files reach the run unread.** `CLAUDE.md`, `AGENTS.md`,
  `.cursorrules` and their kind are ordinary files in the repository Hydra seeds, and each
  provider loads them as configuration. Hydra neither strips nor records them, and `hydra reap`
  commits whatever a run leaves in them. The policy bounds what a run so instructed can do.
- **Nothing pins what an MCP server advertises.** The record carries the servers a launch was
  handed by name, not their tool descriptions, so a server that changes its descriptions after
  admission is not detected.

## Known exposures, stated before someone else does

- **A run's own hook config is denied to it, since 2026-09-12.** On `cursor-agent` the config
  Hydra writes is `<worktree>/.cursor/hooks.json`, and `standard` may write anywhere in the
  worktree, so before build ticket 44 a run could rewrite the file that points at its gate.
  Those paths are refused under every policy now, on every provider's gate. The skill roots
  Hydra links are denied to writes. They were meant to stay readable by shell, which is how
  `codex` and `agy` read a skill, and since build ticket 74 denied the checkout whole they are
  not: the shell check refuses any command naming the snapshot under `~/hydra`, so on those two
  providers a skill is out of reach by shell: `codex` cannot open one at all, and `agy` only
  through its own file tool. Map ticket 71 holds what to do about it.
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
  attack as it was measured; what it does not close is a child that means it. Map ticket 62 asked
  whether this class wants a second uid, and the answer on 2026-09-21 was the seatbelt above
  instead: a child that unsets the variable still cannot write a record into the corpus, because
  the kernel denies the directory to its whole process tree. Since 2026-09-22 it cannot move the
  corpus either: `HYDRA_ROOT` is refused from inside a seatbelt, because a child that unset the
  marker and pointed the root at `/tmp` was writing a `permissive` run's record where the kernel
  does not deny it, and `sandbox_check(3)` is the one fact about its ancestry a run cannot shed.
- **Delegation from inside a run does not work under the seatbelt, since 2026-09-21.** The profile
  is inherited by every descendant, and a delegated child is Hydra itself, which reads its parent's
  record and writes its own into the corpus the profile denies. The human chose to keep the corpus
  denied. Map ticket 64 holds the decision. Since build ticket 76 a `delegate` call from inside a
  macOS run is refused in Hydra's words with `hydra/seatbelt-delegation` before any corpus read,
  and the parent's record carries the tool result the provider streamed back.
- **On `codex` the gate rides the command line, and a hook that dies without answering fails
  open, since 2026-09-21.** Codex reads no project hook file inside a git worktree, and every
  Hydra run is one, so the gate is handed over as a `-c` override naming a Python file in the
  spawn directory, which every policy denies writes to, and the launch carries
  `--dangerously-bypass-hook-trust` because a per-run hook can never be a hash a human trusted.
  Measured on 0.154.0: a hook that exits non-zero with no JSON answer lets the call through. The
  sha check answers a tamper with a deny rather than an exit code for that reason, and a Python
  crash inside the gate on this provider is a call that runs. Since 2026-09-22 the gate answers
  a deny in the caller's dialect for a payload whose `cwd` is not a string, a shell row whose
  command is not a string, a document nested past the interpreter's limit, and anything
  `evaluate` raises; before that a `cwd: 5` was a traceback. The run record still shows every
  call the hook answered; it cannot show the one it crashed on. Under the seatbelt `codex`
  runs with `--sandbox danger-full-access`, because its own seatbelt does not nest inside
  Hydra's; its `workspace-write` would have bounded writes to the workspace, and Hydra's
  profile bounds them to everything but Hydra's own directories, so on macOS a `codex` run's
  writes reach wider than the vendor's default.
- **The gate is only as good as the vendor build.** It lives in the vendor's own hook dialect,
  and vendors have shipped bugs where hooks did not fire in print mode. The record proves the
  gate was consulted on every call it saw; it does not prove nothing else ran. `tests/drift.rs`
  is where a confirmed change becomes a failing test.

- **The export verifier aborts on the redactor's classes and warns on the rest.** `hydra export`
  redacts five classes, then re-reads every line with a scan of its own that aborts on a PEM
  header or a known prefix, including one hidden in a JSON key or a `\u` escape. A long
  high-entropy token that matches no class is reported after the export by field, class and
  length, and the export still happens: aborting on entropy stopped a third of this machine's
  records on digests that are not secrets. AWS keys, Slack tokens and JWTs are not a named class,
  a secret split by a space or URL-encoded reaches neither check, and `~/hydra/runs/` sits on
  this laptop beside the dotfiles that hold them.

## Vendor terms

Hydra spawns each vendor's unmodified binary under your own subscription and never reads,
stores or forwards its token. That is the documented path for scripted use on all four. Two
things the terms do forbid, and Hydra cannot enforce for you: training a competing model on a
vendor's outputs, which is what `hydra export` could feed; and volume beyond "ordinary,
individual usage", which is the stated reason for the 2026 cutoffs of unattended loops. Read
the terms before running Rungs overnight.

## Reporting

Open a private security advisory on the repository, or email the address in `NOTICE`.
Include the run id and the record if you can; the record is usually the whole report.
Vulnerabilities in the vendor CLIs themselves go to their vendors.
