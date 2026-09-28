# 140 - A gated interactive session asks its own launcher for seats

Type: build
Status: resolved
Blocked by: 138, 139

## What

Map 82, answers 1, 2 and 5, which amend map 64. In a gated interactive launch
(`src/interactive.rs`, `run_gated`), `hydra run --interactive` waits outside the seatbelt while
`claude` runs inside it. The face (`src/face.rs`, `call`, near 234) refuses today with
`hydra/seatbelt-delegation`. After this ticket, for an interactive session only, the face hands
the request to that waiting launcher over one channel, the launcher checks it against
`config/panels.json`, launches each seat as a child run with the session as `parent_run_id`, and
returns the results marked as foreign (`marked`, near 255), exactly as the face does outside the
seatbelt.

The channel is the second channel D5 rules on, admitted by 82 for this one use: per run, from
that run's face to that run's launcher, carrying a launch request and returning a result, nothing
else. Pick the mechanism (a unix socket in the run's spawn directory, created by the launcher
before exec, is the obvious one) and the seatbelt rule that lets only the face reach it. If the
seatbelt cannot scope it to one socket path, stop and say so.

A request that matches no shape is not launched in this ticket: the face answers
`hydra/approval-required` with a request id and records the request. 141 builds the approval.

## Do

Tests first, each shown failing before the code, all on stubs, no paid provider:

1. A launcher test: a stub interactive session's face sends a request matching a fixture shape;
   the launcher starts the seats as child runs of the session (their `run_start` carries the
   parent), respects the shape's cap per hour, and the face returns each child's text inside the
   boundary.
2. A request for a shape the session's policy may not ask for, or a seat above the session's
   policy, is refused by name and launches nothing.
3. A request matching no shape comes back `hydra/approval-required` with an id, and the request is
   on the session's record.
4. From inside the seatbelt, the socket of another run is unreachable (kernel refusal), and a
   non-interactive run's face still answers `hydra/seatbelt-delegation`.
5. The session's policy gains `mcp_call` scoped to `mcp__hydra__*` only. Say whether the capability
   table supports that scope today; if not, build it with a gate test that `mcp__claude-in-chrome__*`
   stays refused.

Harness spec 9 (the face) and 10 (interactive mode) and D5's note, with `IMPLEMENTATION.md`.

## Measured before launch, 2026-09-28 (session research, hand-written profiles, `sandbox-exec`)

- **SBPL scopes connect to one socket.** After `(allow default)`:
  `(deny network-outbound (remote unix-socket (subpath "<spawns>")))` then
  `(allow network-outbound (remote unix-socket (path-literal "<own spawn>/seats.sock")))`. The own
  socket connects; a sibling's gets EPERM, also by the `/private`-less spelling, a relative path,
  `..`, a symlink, and a hardlink made outside. Do not deny all unix sockets: that kills
  `/var/run/mDNSResponder`, which is DNS.
- **Today any run can connect to any socket in a sibling's spawn directory.** The existing
  file-read deny on the spawns directory does not stop connect. The deny rule above closes it;
  emit it for every run, not only interactive ones.
- **Bind inside a write-denied directory is refused** to the sandboxed process, and so is
  unlinking the launcher's socket.
- **The face is inside the seatbelt** (`gated_exec_argv`, `src/interactive.rs:229`, and the face
  is a child of `claude`). It can derive the socket path from the `--parent <run_id>` it already
  gets (`src/mcp.rs` near 310); the run cannot rewrite `mcp.json`.
- **`sun_path` is 104 bytes.** The real path is about 72; a test root under `$TMPDIR` overflows.
  Tests bind under a short root in `/private/tmp`, or bind by a relative path after `chdir`.
- **The launcher blocks in `record_then_wait`** (`src/interactive.rs:548`); serve the socket on
  its own thread, and seat runs must not inherit the TTY.

## Decided by the session, 2026-09-28 (delegated by the human)

The kernel scopes the channel to a path, not a process, so the session's own shell reaches
`seats.sock` too, and the gate's `mcp_call` scope is not what guards it. Accepted: anything the
shell can send, the face can send, and the launcher is the guard either way. It checks
`config/panels.json`, holds everything else for `hydra approve`, marks every child's text itself
(not the face), and writes each request to the session's record. What the kernel must guarantee is
that no run reaches another run's socket, which it does. Item 5 of Do stays, scoped as defence
for the MCP path only; do not claim it guards the channel.

## Outcome

Run `01a0e7ae` (claude opus, `standard`), applied as written plus the contract amendment it was not
allowed to make. `src/seats.rs`: one JSON line each way over `seats.sock`, a `Launcher` that checks
askers, containment and the cap per hour, writes a `seat_request` line under the record's lock
before launching, launches seats as children of the session, and marks their text itself; the
face asks it first and gains a `panel(shape, task, files?)` tool; `run_gated` binds before exec and
stops before the harvest. Every profile denies unix-socket connects under the spawns directory and
allows the run's own `seats.sock`. `Allow` refuses unknown keys now (it silently dropped them, so
a misspelt narrowing would have loaded as the widest grant) and gains `servers` for `mcp_call`.

Found by the run: the shipped `standard` grants no `mcp_call`, so the face cannot reach the channel
from a session until the human adds `{ "capability": "mcp_call", "servers": ["hydra"] }`
(`tests/fixtures/policies/standard.json` shows the file); the shell can reach it either way.
Launcher output lands on the session's screen (142, item 4). On Linux, with no seatbelt, a gated
session's `delegate` now goes through the launcher's check.

The session ran the three live-kernel tests on master (they skip inside a run): a sibling's socket
is refused, a non-interactive face still refuses, an interactive face reaches its launcher. The
record contract gains `seat_request` as the one exception to "exactly two Hydra lines". An
adversarial review found no bypass of shapes, containment, socket scope or marking, and five
robustness gaps a hostile session can reach; they are build 142, and the release binary waits for
it. Master: fmt, clippy, lib 217, seats 12, gate 110, spawn 35, face 8, harvest 12, delegation 14,
record_contract 20, sandbox_read_only 3, sandbox_tty 2, panels 8, normalize 87, doctor 3,
detached 9, pty_tee 4, show 3.
