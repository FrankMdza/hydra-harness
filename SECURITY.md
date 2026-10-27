# Security

## What Hydra promises

A run launched by Hydra acts inside its own git worktree, under a named policy compiled into
the vendor's own pre-tool hook. Every tool call the hook sees is recorded with the decision
Hydra made about it.

The agent's environment is built from an allowlist, `src/env.rs`, plus the few names Hydra sets
itself, so a key exported in your shell is not in it. Since build ticket 97 the allowlist also
bounds the watcher, the Hydra process that owns the child's pty and sits outside the seatbelt
for the whole run: `hydra run` starts it from the child's environment rather than its own. That
does not keep secrets away from a run under `standard`. A shell command can still read a dotfile
on disk, and a same-uid `ps -E` still reads the environment of your own terminals.

## What Hydra does not promise, on purpose

- **The gate does not interpret shell.** A command that spells a protected path is refused
  under every policy. A command that reaches the same path through a variable, a subshell, a
  `cd` followed by a relative path, or a nested `sh -c` naming a sibling under a parent this
  run owns a piece of, is not. Neither is a path split by a quote, assembled by concatenation,
  percent-encoded, or spelled with `..`, `//` or `/./` inside an interpreter or nested-shell
  token. The gate refuses a protected path only when its prefix appears whole and in canonical
  form. `docs/harness-spec.md` section 7.4 states this and `tests/gate.rs` fails if someone
  claims to have closed it. A shell command `shlex` cannot split skips the token pass in
  `gate.py`'s `protected_path_named_by`, so only the raw prefix pass reads it for a protected
  path; the pattern floor then refuses it as unparseable, and `--allow-floor-escape` lifts that
  refusal with the rest of the floor.
- **An MCP argument's relative path resolves, but its fields do not join.** Since build ticket 348
  a string in an MCP call that looks like a path, or a word of one, resolves against the run's
  cwd, and a refusal names the resolved path and any link crossed. Not caught: two arguments
  that join into one path, a path a server assembles from its own variables or percent-encoding,
  and a relative path that the MCP server resolves against a directory other than the cwd the
  hook reports.
- **A shell bounded to a command list is a filter, and the seatbelt is the guarantee, since build
  ticket 139.** A policy may grant `shell` with a `commands` list, which map ticket 82 gives the
  seat that reads files it was not handed. The gate then refuses a command whose segments do not
  each start with a listed entry's words, or that carry a flag the entry refuses
  (`find -delete`, `find -exec`, `git diff --output`, a GNU-style abbreviation included). It
  refuses a redirect, a subshell, a background `&`, a `$` expansion or a backquote, an unquoted
  glob or brace expansion, a newline, an unbalanced quote, and any argument that resolves outside
  the policy's `file_read` scopes, a symlink out included. It splits on a pipe, `;`, `&&` and
  `||` and on nothing else. The list does not make a listed command safe, which is why `sed` is off the list the
  session proposed: its own script writes (`w`, `W`) and reads (`r`) files no argument names. For
  a write a listed command could still make, what stops it on macOS is the profile: under a bounded shell the
  worktree is denied to writes, and file contents and listings under Hydra's whole tree are
  denied to reads except the run's own worktree, skill snapshot, spawn directory and the parent
  repo's `.git`. That profile does not cover the rest of `HOME`. A listed command that writes
  outside `~/hydra` without naming a path the gate resolves reaches the disk, and `tests/gate.rs`
  refuses only the spellings it can read. Linux has no seatbelt, so there the list is all there is.
- **The gate folds case ASCII only, and Hydra refuses a root it would fold wrong.** The macOS
  filesystem folds `É` too, so `hydra run` refuses when its data root or its checkout resolves
  to a path outside ASCII. The launching repository's `.git` is folded the same way and is not
  checked: under a repository whose path holds a cased non-ASCII letter, the denial of writes to
  that `.git` misses the other case, and on macOS the seatbelt is what still refuses them.
- **There is no container, and on macOS there is a sandbox around the process tree.** The
  vendor CLI needs the network to reach its model, so network cannot be cut, and the run has your
  real `HOME` because a wrong one logs the vendor out. Since 2026-09-21 every recorded launch on
  macOS runs under the kernel's own seatbelt with the same user and `HOME`: for the launched
  process and every descendant, writes under Hydra's directories and reads of the corpus, of
  pruned records (since build ticket 93) and of any other run's spawn directory fail with "operation not permitted" whatever the spelling, a
  variable, an interpreter or a symlink included, measured on all four providers and pinned in
  `tests/spawn.rs`. Since build ticket 137 the profile also refuses the `TIOCSTI` ioctl, so a
  process in it cannot push keystrokes into its controlling terminal, which in a gated interactive
  run is yours; `tests/sandbox_tty.rs` probes that on a pty the test owns. The profile is `(allow default)` plus those denials, so it is defence behind
  the gate and not a boundary: it does not restrict process launching or mach services, and a
  process that a system agent starts on the run's behalf, launchd for one, does not inherit it.
  One `launchctl submit` probe on 2026-09-22 did not reproduce that class; nothing says a second
  spelling would not. Map ticket 67 holds the question of what the seatbelt is for. The gate
  still decides what a policy allows and writes the record; the kernel is what refuses the
  write. A gated interactive launch, `hydra run --interactive --policy`, runs under the same
  profile since build ticket 73. A plain `--interactive` run, with no `--policy`, compiles no
  gate and gets no seatbelt or worktree. Linux launches unwrapped.
- **The deny list is Hydra's runtime directories, the checkout the binary was built from, and six
  config files.** Build ticket 74 added `hydra_home()` whole, with the run's own worktree and a
  `curate` pass's checkout carved back out. Under `standard` a shell command still reads and writes
  the rest of the machine: `~/.ssh`, your own `~/.claude` hooks and settings, and any directory on
  `PATH` ahead of `/usr/bin`, which is where the vendor binary is resolved from, except in a gated
  interactive session since build 250, which writes nothing under your home directory outside its
  worktree and `claude`'s session state. That state includes `~/.claude.json`, where Claude Code
  keeps user-scope MCP servers, and `~/.claude/history.jsonl*`. A gated interactive `claude`
  session can add a server to `~/.claude.json`, and the next ungated `claude` session starts it.
  Map ticket 99 holds that question. The gate allows
  one read outside the worktree on `claude`: the run's own `~/.claude/projects/<slug>/`
  directory, for tool output Claude Code wrote there; every other path under `~/.claude` stays
  refused on a gated `Read`. The honest
  sentence is still the one the 2026-09-14 review ended on: the gate bounds what a run may do to
  Hydra, not what it may do to you.
- **Rung two's admission gate runs proposer code, on a copy of the tree the judges read.** It
  runs `cargo fmt --check`, `clippy`, `build` and `test` before any judge reads the patch, under
  a seatbelt profile since build ticket 75, with `HYDRA_ROOT` pointed at an empty scratch root.
  Since build ticket 90 each step starts from a cleared environment and gets only
  `ADMISSION_ENV_ALLOWLIST` in `src/env.rs` (`PATH`, `HOME`, `TMPDIR`, `USER`, `LANG`,
  `RUSTUP_HOME`, `RUSTUP_TOOLCHAIN`) plus `HYDRA_ROOT` and a scratch `CARGO_HOME`; an unset or
  empty name is skipped and a `HOME` that is not the account's is dropped. Before that, the steps
  inherited the shell that ran `hydra evolve`, API keys and agent sockets included. Since build
  ticket 98 the steps run on a fresh copy of the read tree, written byte for byte from the object
  store after the proposer's run ends, and not in the proposer's checkout. On macOS the
  proposer's seatbelt cannot write that copy, and neither can any process the proposer left
  running, because the profile is inherited. Cargo gets an empty cargo home, and the admission
  profile denies reads of cargo's configuration files in every `.cargo` above the copy, and of
  the `.cargo` name, so a link planted in its place resolves nowhere. A `runner` or
  `rustc-wrapper` the proposer wrote into `~/.cargo/config.toml` is not read. `cargo` is the
  toolchain's own binary, found before the proposer ran, and is called by absolute path. What is
  not closed: on this machine the toolchain lives in a Homebrew directory the user owns, which a
  `standard` run can rewrite, and `cc`, `git` and `python3` still resolve through `PATH`. The
  profile bounds what the steps may do to Hydra and not what they may do to you, so a hostile
  test can still read `~/.ssh`. The pass's harness record names the profile sha and the root the
  steps ran under. On Linux the steps run bare.
- **Inside the seatbelt a run can stage its work and cannot move a branch.** Under the parent
  repository's git directory, only the private `worktrees/<run-id>/index` and `index.lock` are
  writable, since build ticket 95; its `HEAD`, `commondir`, `gitdir` and `logs/`, and the shared
  refs, stay denied at the gate and the seatbelt. `git status` and `git add` work, `git commit`
  fails, and no branch moves from inside a run. `hydra reap` commits from outside, only on
  `hydra/<run-id>` or `rungs/curate/<run-id>`, and refuses any other branch before it stages
  anything. Under bubblewrap the rule is weaker: since build ticket 364 the whole private
  directory is writable, with only its existing `HEAD`, `commondir`, `gitdir` and `logs` bound
  read-only. A run there can add files host git reads, such as a `COMMIT_EDITMSG` link, a
  `MERGE_HEAD`, `locked` or `config.worktree`, and it can replace `index` with a fifo or a link.
  SBPL allows writes to the literal `index` path, so the same may hold on macOS. Nobody has
  measured it.
- **Hydra sweeps the private git directory before it runs git there, since build ticket 367.**
  Every `CheckoutGit` constructor keeps `HEAD`, `commondir` and `gitdir` and removes every other
  entry, the index and `logs` included, and follows no link. It refuses the checkout, naming the
  path, when one of the three is not a regular file with one link. Since build ticket 371 it
  then rebuilds the index with `git read-tree HEAD`, so host git never reads an index the run
  wrote: a skip-worktree bit on a planted blob had made reap commit bytes the checkout did not
  hold. A rebuilt index has no skip-worktree bits, so `hydra run` refuses a parent with sparse
  checkout on. Reap, the diff stat at a run's end, `hydra status`, rung one's proposal and rung
  two's patch read all go through it. A run `hydra status` finds live gets the type checks
  alone and keeps its index, because removing files under a running child races it.
  Launch-time git runs before the child starts and skips the sweep. Every plant was measured
  with plain git on macOS. None was measured under bubblewrap.
- **Git that Hydra runs outside the seatbelt trusts neither the checkout's `.git` file nor any
  configuration a run can write.** Every git call Hydra makes in a run's checkout sets
  `GIT_DIR`, `GIT_COMMON_DIR` and `GIT_WORK_TREE` from the run record's
  `scopes.parent_git_dir`, or from launch data saved beside the checkout under the deny list,
  and turns off hooks and fsmonitor. Before build ticket 95 a run could rewrite that file and
  point Hydra's own git, in reap and in the diff stat at the end of a run, at a configuration it
  wrote, found by the second hiring panel's Opus reader. Since build ticket 99 every git call
  Hydra makes anywhere sets `GIT_CONFIG_GLOBAL=/dev/null` and `GIT_CONFIG_NOSYSTEM=1`, so a
  filter, an external diff or a hook path a run writes into `~/.gitconfig`, or into a system
  file the user owns, runs nowhere. Four values still come from that configuration, each read as
  data and passed back with `-c`: `user.name` and `user.email`, so commits keep their author, and
  since build ticket 105 `core.excludesFile` and every `safe.directory`. `core.attributesFile` is
  pinned to `/dev/null`, because git reads `~/.config/git/attributes` even with global
  configuration off. Hydra's commits are unsigned. A launch from under `~/hydra/worktrees/` or
  `~/hydra/skill-worktrees/` is refused before any git runs there, and so is `hydra evolve`. One
  exception: `git worktree remove` reads the checkout's `.git` file, so reap rewrites it to the
  trusted directory first. A checkout with neither trusted source is left for a human, and reap
  says how to remove it.
- **A run writes git objects only into its own store, since build ticket 103.** A run with a
  checkout gets `GIT_OBJECT_DIRECTORY` pointed at `~/hydra/spawn/<run-id>/objects` and reads the
  parent's store as an alternate. The parent's `.git` is denied whole to the seatbelt and to the
  gate, `objects/` included. A pack a run forges in its own store lies only to its own git. Reap
  moves the run's objects into the parent from outside the seatbelt with `git pack-objects` and
  `git index-pack --strict`, which names each object from its bytes, so a forged name never
  reaches the parent or a later worktree. Reap copies the run's store into `reap-import` beside
  it and runs `pack-objects` on the copy, which the run cannot write after reap has started, so
  a process the run left running cannot plant `info/alternates` or a link in the directory git
  reads. A store over 1 GiB imports nothing. An index entry whose object did not arrive is
  hashed again from the checkout. No other git Hydra runs reads the run's store: each one drops
  the two variables and hashes the checkout itself. Until build ticket 103 `objects/` in the
  parent kept a full write grant, and a run could plant a pack that mapped an existing sha to
  other bytes, which git served. That window is closed, build ticket 112.
- **Some vendor tools never ask the hook.** Cursor runs `ReadLints`, `getMcpToolsToolCall`,
  `awaitToolCall`, `WebSearch` and, measured on 2026.10.01, `updateTodos` without consulting it.
  `codex` runs its hosted `WebSearch` without consulting the hook too. Under a policy that denies
  `network_read`, that call cannot be blocked before it runs. Normalization files it as
  `UnhookedDenied`, and the record marks an allowed one `unhooked`, instead of treating the tool
  as outside every policy. `codex`'s `webrun` is hooked since build ticket 338. On `claude`
  2.1.301, `LSP` is unknown to the table and refused, and `Workflow` maps to `subagent_spawn` by
  reading, build ticket 350.
- **Hydra reads records that agents wrote.** Anything in a run record is data, never
  instructions, and Rungs treats it that way; if you find a path by which text in a
  record changes what Hydra does, that is a vulnerability.
- **The checkout's own instruction files reach the run unread.** `CLAUDE.md`, `AGENTS.md`,
  `.cursorrules` and their kind are ordinary files in the repository Hydra seeds, and each
  provider loads them as configuration. Hydra neither strips nor records them, and `hydra reap`
  commits whatever a run leaves in them. The policy bounds what a run so instructed can do.
- **Under `standard` a run holds all three legs of the lethal trifecta.** It reads untrusted
  text, it reads private data, and `network_read` carries no scope and no domain list, so it can
  send what it read anywhere. The floor refuses Slack's posting methods by shell and by fetch
  URL and nothing else. A domain list would bind the fetch tools and not `curl`, since the gate
  does not interpret shell, and the seatbelt filters by address rather than host name, so
  map ticket 69 chose to state the exposure rather than build half a fence. `read-only`, `judge`
  and `curate` grant no network.
- **Nothing pins what an MCP server advertises.** The record carries the servers a launch was
  handed by name, not their tool descriptions, so a server that changes its descriptions after
  admission is not detected.
- **A run's words reach the orchestrator marked, and marking is advice.** The orchestrator that
  reads `hydra show` and the delegate face is the one session no gate binds. Since build ticket
  272 both print every field the run's model or tools wrote (`result_text`, and the message,
  thinking and tool output text `--events` prints) inside `<<<hydra-result TOKEN>>>` and
  `<<<end hydra-result TOKEN>>>`. The token is drawn after the text was on disk, and a run that
  writes a boundary of its own has it escaped with a backslash. That tells the orchestrator which
  bytes are a report. It does not stop the orchestrator obeying them. `error` stays plain, since
  only Hydra writes it, and so do a tool call's `arguments`, which are JSON rather than text.
  `hydra wait` prints no text a run wrote. Since build ticket 356, `hydra show --events` also wraps
  `gate_refusal.reason` in `<<<hydra-result TOKEN>>>` and `<<<end hydra-result TOKEN>>>`. `gate.py`
  interpolates the refused token into `reason`, and the sidecar that holds it is writable inside
  the run's seatbelt.

## Known exposures, stated before someone else does

- **Until 2026-09-23 the environment allowlist did not reach recorded runs.** The probes and the
  interactive launch cleared the environment, and the pty that starts a recorded run did not:
  portable-pty's `CommandBuilder::new` copies the environment of the process that calls it, and
  the watcher calling it had inherited the shell that ran `hydra run`. So every recorded launch
  before build ticket 88 received that shell's whole environment, `GEMINI_API_KEY`,
  `SSH_AUTH_SOCK` and Claude Code's messaging token among what the test measured, while this
  file said no credential in your shell reaches the agent. The second hiring panel found it by
  reading the dependency's source. `src/pty.rs` clears the environment before applying the
  allowlist now, and `tests/pty_tee.rs` fails without the clear. Records from before carry an
  `env_allowlist` that named what Hydra meant to pass, not what the child got. The watcher kept
  the shell's whole environment one day longer, readable by a same-uid `ps -E` for the length of
  the run, until build ticket 97 started it from the child's environment; the third hiring panel
  found it. `tests/detached.rs` reads a running watcher's environment from the kernel and fails
  on any name the child was not handed.
- **A run's own hook config is denied to it, since 2026-09-12.** On `cursor-agent` the config
  Hydra writes is `<worktree>/.cursor/hooks.json`, and `standard` may write anywhere in the
  worktree, so before build ticket 44 a run could rewrite the file that points at its gate.
  Those paths are refused under every policy now, on every provider's gate. The skill roots
  Hydra links are denied to writes. A shell command naming the snapshot under `~/hydra` is
  refused, so since build ticket 87 `codex`, which reads only by shell, gets read-only copies in
  its worktree instead of links; on macOS the seatbelt denies writes to them, and elsewhere a run
  can `chmod` its own copy, which the watcher notes at exit. `agy` reads a skill through its file
  tool. `hydra reap` never commits an entry Hydra injected.
- **A run cannot plant a plugin, since build ticket 257.** A plugin is code a provider loads into
  its own process, and on `claude` one can approve a call the gate refused. Map 103 C answered
  it in two parts. The human's half is a managed settings file that keeps users' plugins out of
  every `claude` on the machine. Hydra's half refuses writes to `~/.claude/plugins/`,
  `~/.claude/skills/`, and inside the worktree `.claude/skills/`, `.claude/settings.json`,
  `.claude/settings.local.json` and `.agents/plugins/`, plus any path with a `.claude-plugin`
  component, under every policy. The gate refuses the file tools. On macOS the seatbelt refuses
  everything else, shell included, in headless and interactive launches alike. Off macOS a shell
  command can still write there, because the gate does not deny shell commands that name
  `.claude/skills/`: that is how two providers read a skill.
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
  same live run writes a `containment` refusal instead of starting. Until build ticket 97, on
  2026-09-23, the marker reached no recorded child: the launcher set it on its own copy of the
  child's environment, and the watcher rebuilt the child's environment without it. So from build
  ticket 57 to 97 a recorded run that relaunched the binary read as a human at depth 0, and only
  interactive launches, which set the marker on the child directly, were checked. On macOS the
  seatbelt below still denied such a child the corpus. A recorded `claude` run printed its own
  run id from `$HYDRA_RUN_ID` after the fix, run 01a0d1fa-c301. **A child that unsets the
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
- **A gated interactive session asks its own launcher for seats over a unix socket, since build
  ticket 140.** Map ticket 82 admitted one channel: `seats.sock` in the run's spawn directory,
  bound by `hydra run --interactive` outside the seatbelt before the provider starts. Before this
  ticket any run could connect to any socket in a sibling's spawn directory, because the read
  denial on the spawns directory does not stop `connect`. Every run's profile now denies connecting
  to a socket under the spawns directory and allows only its own `seats.sock`; measured on
  2026-09-28 with hand-written profiles, a sibling's socket answers EPERM under every spelling
  tried. The run cannot unlink the socket or bind its own, because the spawn directory is denied
  to its writes. The kernel scopes the socket by path, so the session's own shell reaches it as
  well as the face does. The launcher is the guard: it launches only a shape the human approved in
  `config/panels.json`, never a seat wider than the session's policy, at most the shape's cap per
  hour, writes every request to the session's record before it launches anything, and marks each
  child's text itself. Everything else waits for a human. Since build ticket 142 the launcher also
  holds against the session itself. It answers at most 60 requests an hour per session, whatever
  it decides about them, and writes one `hydra/request-cap` line when that runs out and nothing
  after it, so a loop of refused requests cannot grow the record. A request line is at most 64 KB
  and has 10 seconds to arrive whole. At most four connections are answered at once and the fifth
  is refused and closed, and a reply that is not read within 10 seconds is dropped. When the
  session ends the launcher waits up to 60 seconds for a seat still inside `run::run` to reach its
  detached watcher before the process exits. A seat launch prints nothing on the human's terminal.
  The `mcp_call` server scope matches the server part of a tool name exactly: `mcp__hydra___x`
  and `mcp__hydra__evil__x` name servers `hydra_` and `hydra__evil`, and both are refused.
- **A seat request no shape names waits for `hydra approve`, typed by the human, since build
  ticket 141.** The launcher writes it to `~/hydra/approvals/` and holds the face's call open for
  up to ten minutes; the human's `hydra approve <id>` renames the file, and the launcher
  launches. The command refuses inside a seatbelt, which is only courtesy: a model that skips it
  still meets the gate's `hydra/protected-path` and the kernel, because the store is on the
  denied write paths of every policy and every profile denies it reads too, since a waiting
  request carries its session's task. Those denials are in the profile text on every spawn; the
  live-kernel probe of a write, a rename and a read in the store
  (`the_seatbelt_denies_a_write_to_the_approval_store`) skips inside a seatbelt and has not run
  on the machine that built it. A waiting request holds one of the session's four connections.
  A launcher killed outside its own shutdown leaves its requests pending; `hydra approve` finds
  the session's socket gone and withdraws such a request rather than approving it into nothing.
- **On `codex` the gate rides the command line, and a hook that dies without answering fails
  open, since 2026-09-21.** Codex reads no project hook file inside a git worktree, and every
  Hydra run is one, so the gate is handed over as a `-c` override naming a Python file in the
  spawn directory, which every policy denies writes to, and the launch carries
  `--dangerously-bypass-hook-trust` because a per-run hook can never be a hash a human trusted.
  Measured on 0.154.0: a hook that exits non-zero with no JSON answer lets the call through. The
  sha check answers a tamper with a deny rather than an exit code for that reason, and a Python
  crash inside the gate on this provider is a call that runs. Since 2026-09-22 the gate falls back
  to the worktree for a payload whose `cwd` is not a string and answers a deny in the caller's
  dialect for a shell row whose command is not a string, a document nested past the interpreter's
  limit, and anything `evaluate` raises; before that a `cwd: 5` was a traceback. The run record still shows every
  call the hook answered; it cannot show the one it crashed on. Under the seatbelt `codex`
  runs with `--sandbox danger-full-access`, because its own seatbelt does not nest inside
  Hydra's; its `workspace-write` would have bounded writes to the workspace, and Hydra's
  profile bounds them to everything but Hydra's own directories, so on macOS a `codex` run's
  writes reach wider than the vendor's default.
  `tests/drift.rs::a_codex_hook_that_dies_without_answering_lets_the_call_run` re-measures the
  fail-open with a hook that raises before it answers. It costs one turn, runs only under
  `HYDRA_DRIFT_PAID=1`, and fails if `codex` starts refusing the call. Its first run, on
  2026-09-24, passed: a `codex` hook that dies still lets the call run.
  Since build ticket 358, `hydra doctor` runs `/usr/bin/python3 -c "import json, shlex"`, so a
  missing or broken system Python shows up before launch. Its provider lines print the installed
  version beside the one `MEASURED_VERSIONS` holds, so a `claude` or `codex` that has moved past
  it is visible before launch. `unmeasured` appears only when the table has no row for the
  platform.
- **The gate is only as good as the vendor build.** It lives in the vendor's own hook dialect,
  and vendors have shipped bugs where hooks did not fire in print mode. The record proves the
  gate was consulted on every call it saw; it does not prove nothing else ran. `tests/drift.rs`
  is where a confirmed change becomes a failing test.
- **A stored `gate_refusal` row is hook evidence, not tamper-proof provenance against a process
  that can write the sidecar through the kernel's necessary write allowance.** That is contract
  7.7's sentence, since build 321. The hook appends refused shell commands to
  `gate-refusals.jsonl` under the run's own seatbelt profile, and the allowance that lets the hook
  write lets the run's shell write too. The human declined a signature on 2026-10-15, since any
  secret the hook can read the shell can read. Build 332 measured the forgery:
  `tests/detached.rs` has a stub append a well-formed row for a call no frame names, and the
  record carries it as a `gate_refusal` event, `hydra show --events` prints its command inside
  the untrusted-content boundary, and the derived `permission_decision` rows do not change,
  because those come from the provider's stream. A damaged line in the sidecar is skipped and
  counted on `run_end.error`, and the record still closes. `hydra doctor --walk` reads each
  row's `call_id` against the same record's stream and prints `Uncorroborated` for a row that
  names no call, a call with no result, or a call that succeeded. It does not fail on one,
  because a row is a claim about the run and not a leak. Since build ticket 339 the hook appends a
  `gate_refusal` row for every denied call, not only shell commands. A row for a call with no
  command text carries `arguments_sha256` in its place, and `reason` still names a refused path.
  A deny whose payload has no string `tool_name` writes no row.

- **The export verifier aborts on the redactor's classes and warns on the rest.** `hydra export`
  redacts twelve classes of secret since build ticket 128. They include AWS `AKIA` access keys,
  `aws_secret_access_key=` values, Slack `xoxb-` bot tokens and `hooks.slack.com` webhooks, and
  GitHub `github_pat_` and `gho_` tokens, alongside OpenAI keys and PEM headers. OpenAI keys
  match by a matcher of their own, not a prefix. The export then re-reads every line with a scan
  of its own that aborts on a PEM header or a known prefix, including one hidden in a JSON key or
  a `\u` escape. The scan checks the redactor's prefixes, so a hit means the redactor missed one.
  A long high-entropy token that matches no class is reported after the export by field, class
  and length, and the export still happens. Aborting on entropy stopped a third of this machine's
  records on digests that are not secrets. JWTs are not a named class. The write-time sweep in
  `src/record/secrets.rs` scans only the five alarm classes in the shared table in
  `crates/secret-classes`. Harvest writes through the same `provider_line`, so each frame gets one
  scan. The cockpit hides flagged lines whole and masks spans from the full table on other lines,
  even after reveal. Export redacts the full table, including the OpenAI span rule. A secret
  split by a space or URL-encoded reaches neither check, and `~/hydra/runs/` sits on this laptop
  beside the dotfiles that hold them. Since build ticket 100 `hydra export` refuses an
  `--out` that is the same file as `preference.jsonl` beside it, compared by device and inode
  before a byte is written. That covers `Preference.jsonl` on a case-insensitive volume and a
  symbolic or hard link by any name.
- **`git push` floor and its remaining shell limits, builds 349, 357, 362 and 365.** Since build
  tickets 349 and 357 the pattern floor refuses a bare `git push` from a protected branch. It
  judges `git push origin HEAD` and `@` by the branch the checkout names, with any `-C`,
  `--git-dir` or `--work-tree` before `push` taken into account. It refuses a push to a protected
  branch named with a `refs/heads/` prefix, a `-c remote.<name>.push` refspec to one, `-c
  push.default=matching`, and any `--config-env`. A `cd` earlier in the same shell command still
  reaches a protected branch, because the gate does not interpret shell. Build ticket 362 reads
  `push.default` and `remote.<name>.push` from `.git/config` and from the shared config a linked
  worktree's `commondir` names. A `push.default = matching` there refuses a push with no refspec.
  So does a `remote.<name>.push` refspec to a protected branch, when the push names that remote or
  none. Build ticket 365 takes the safe side of a line reader. The floor reads a dotted
  `[remote.origin]` header, a key on the header line, a comment after the header, and a value
  continued with a backslash. It refuses a target that contains `*`, an empty `:` refspec, any
  `[include]` or `[includeIf]` section, `-c include.path` and `-c includeIf.*.path`, and a
  `GIT_CONFIG`, `GIT_CONFIG_COUNT`, `GIT_CONFIG_PARAMETERS`, `GIT_CONFIG_GLOBAL`,
  `GIT_CONFIG_SYSTEM` or `GIT_CONFIG_KEY_<n>` assignment before `git`. It does not follow
  includes. `config.worktree` is still not read.
- **What remains open across map 134.** The interactive gate on `cursor-agent`, `agy` and `codex`
  is unmeasured, so `hydra run --interactive --policy` refuses those three until build ticket 130.
  Linux compiles and runs CI since builds 315 and 331, but launches hook-only without an OS sandbox
  until build ticket 318 lands. `hydra doctor --walk` walks the live corpus on the author's
  machine, not in CI. `hydra bench` counts a `completed` run as a successful attack, map ticket
  114. Nothing sends diffs off the machine today, and map ticket 110 governs any future return of
  `jev ask`.

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
