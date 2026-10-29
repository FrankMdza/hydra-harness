# Hydra

Hydra is a router over four agent CLIs (`claude`, `cursor-agent`, `agy`, `codex`), plus Rungs, a
self-improvement loop that reads what the router records. The router core is Rust. `hermes` was a
fifth in the early map until ticket 22 unrouted it; its guardrail rules are still copied, its binary is not launched.

This repo holds a finished design, three documents in `docs/`, written from a wayfinder map that
closed on 2026-09-05 with 38 of 38 tickets resolved and all four seam checks passing. It also holds
the Rust that implements all eleven of that design's commands, `evolve` last, on 2026-09-08.

**Read `docs/` first, not the map. Then read `IMPLEMENTATION.md`**, which is the gap between the
design and the code. The code is deliberately not spec-complete: all four providers are routed and
gated, with skills and MCP injected into each, so that file says what a record does and does not
carry.

The three documents are the design. `docs/harness-spec.md` is how Hydra launches, feeds and records
the providers; `docs/run-record-contract.md` is the format it writes and Rungs reads;
`docs/rungs-spec.md` is both rungs of self-improvement. They are meant to be read together, and
nothing crosses between the harness and Rungs except the run record and the skill store.

## The map

`.scratch/hydra/map.md` is the history: the destination, the settled decisions D1 to D16, a one-line
index of the closed tickets, what is still fog, and what is out of scope. Read it when you need to
know **why** a document says what it says. The ticket behind any decision holds the reasoning the
document compressed.

**The map stands at 125 resolved of 136 tickets on 2026-10-27, and the build stands at 370 resolved of 373 tickets, with 130, 294 and 377 open.** Map ticket 68 answered the immune-loop question with an attack corpus in `tests/corpus/`, and 69 to 71 followed from it; 72 and 73 came from that panel. The map reached its destination
on 2026-09-05 and closed; build tickets 07, 08 and 12 then hit five things that need a decision
rather than an implementation, which is the one condition for reopening it. Tickets 39 to 43 are
those, three of them about what a policy scope is and who gets to know, one about provider drift.
All of them, plus 44 to 46, closed on 2026-09-06, and build tickets 13 to 16 carry their code.
The build closed at 16 of 16 on 2026-09-07 and opened 47 and 48, two grillings, on the way, then
reopened with ticket 17 the same day. Tickets 47 to 50 closed on 2026-09-07 and 2026-09-08 with
the human, and build tickets 17 to 28 carry their code: thinking text, `hydra show`, the Herdr
bridge and graph, `base_tree`, the face admitted, the interactive mode and its harvest. The
map's own paragraph under `## Destination` says what each left. **If a build ticket finds a
decision rather than an implementation, it reopens the map with a new ticket rather than deciding.**

**Wayfinder is still not how the build continues**: it charts decisions, its tickets resolve
questions rather than execute slices, and its own instructions say the pull to just do the work is
the signal you have reached the edge of the map. Take a map ticket only when the work is a decision:

```
/mattpocock-skills:wayfinder .scratch/hydra/map.md
```

## The build

Since build 305 the session runs `cargo nextest run --release --workspace` and then `cargo test --release --workspace --doc`, which together take half of what `cargo test --release --workspace` takes; that one still works and is what CI runs.

The dev profile carries line tables only; `CARGO_PROFILE_DEV_DEBUG=2 cargo build` restores full debuginfo for a session that needs the debugger. Since build 310 the release profile sets `lto = "off"`, which cut the warm edit build from 15.3 s to 12.3 s.

`.scratch/hydra/build/plan.md` indexes the build and `.scratch/hydra/build/NN-slug.md` are its
tickets, same format as the map's, `Type: build`. Start a session like this:

```
./target/release/hydra-puppets frontier build
```

Take a takeable ticket, set `Status: claimed` in its file **before** doing any work, read the spec
sections it names, then:

```
/mattpocock-skills:implement .scratch/hydra/build/NN-slug.md
```

Nothing else is needed to resume. The tickets carry the context, so a session that has read `docs/`,
`IMPLEMENTATION.md` and one ticket knows what a session that wrote them knew. There is no handoff
document and there should not be one.

## Layout

- `src/`, the binary. `run.rs` is the launch sequence and the place to start reading; `watch.rs` is
  the detached half that owns the pty and the record; `live.rs` is the pid liveness test everything
  else reads; `reap.rs` and `prune.rs` are the two cleanup commands, typed by a human and
  reached from nowhere but `main`; `resolver.rs` turns a run-id prefix into the run it
  names, for `show` and `reap`, and never looks up a full uuid; `record/` is the run record and the validator the tests
  drive; `env.rs` is the allowlist; `pty.rs` owns the child's pty; `provider/claude.rs`,
  `provider/cursor.rs`, `provider/agy.rs` and `provider/codex.rs` hold every fact that is true
  of one binary and not of a model, and the four files beside them in `normalize/` read each
  one's records back; `policy/` loads the policy set, owns the capability table per provider, and compiles
  the gate script;
  `skills/` is the store, the per-sha snapshot, the read-only index over the user's own skill
  roots and the collision check;
  `mcp.rs` reads the canonical MCP document and decides what one launch is handed;
  `spawn_config.rs` writes the gate's hook config and, on macOS, the seatbelt profile, and is
  harness spec 15.2's row 4; `sandbox.rs` composes that profile and wraps the launch in it;
  `doctor.rs` is `hydra doctor`; `export_preference.rs` reads preference pairs, corrections and
  judge verdicts, behind the human's sign-off manifest;
  `panels.rs` loads the approved panel shapes of protected config #6; `seats.rs` is the launcher a gated interactive session asks for seats over its own socket, and `approve.rs` the store and the human-typed `hydra approve` for a request no shape covers (map 82);
  `preamble.rs` composes what the run is told; `interactive.rs` is the second launch mode and
  the harvest that completes it; `show.rs` prints a record normalized; `face.rs` is the MCP delegate face of harness
  spec 9, a stdio server a run's own provider spawns from the MCP document, and the one place
  a child's text is marked on the way back; `rungs/` holds both rungs of self-improvement,
  `hydra curate` and `hydra evolve`, and is the only place Rungs lives, which is what makes "it calls neither cleanup command" a
  property you can check by reading one directory.
- `policies/`, the five policies of harness spec 7.2 and the pattern floor of 7.4. Protected config
  #1, and `policy_sha` on every run record is its sha. `policies/README.md` is the format.
- `config/`, protected configs of harness spec 15.1. `preamble.md`, `mcp.md`, `panels.json` and
  `export-allowlist.json` live there. The environment allowlist lives in `src/env.rs`, and
  `export-signoff.json` is optional. `config/README.md` says what each file is for.
- `IMPLEMENTATION.md`, what is built, what is not, and every departure from the design with its
  reason. Amend it in the same change that narrows the gap.
- `integrations/herdr/`, the Herdr plugin: a launcher that puts the run id on the pane, a poller
  that labels the pane from the record, a graph of one run in the terminal, and a smoke test
  against a live Herdr. Nothing under `src/` imports it, and its README says how to link it.
- `tests/`, twenty-seven files on 2026-09-24, more since (`panels.rs`, `seats.rs`, `sandbox_tty.rs`, `sandbox_read_only.rs` among them). `record_contract.rs` checks a written record against the contract and walks
  the fixture corpus; since build ticket 126 the walk over every record on this machine is
  `hydra doctor --walk`, its assertions live in `src/walk.rs`, and `doctor.rs` drives it on a scratch
  corpus; `fixture_corpus.rs` writes and guards `tests/fixtures/records/`,
  the synthetic records the fixture walks in `record_contract.rs`, `normalize.rs` and
  `outcome.rs` read; `git_config.rs` plants a global and a system git config and checks that
  nothing they name runs from Hydra's git, and that the excludes file, `safe.directory`, the
  XDG attributes file and a fifo include are handled; `launch_phases.rs` fails a launch after
  its worktree exists and reads the `never_started` record back; `pty_tee.rs` drives a stub provider so the pty path costs no quota;
  `worktree.rs` covers the seeding; `gate.rs` compiles a real gate per policy and feeds it synthetic
  hook payloads, launching nothing; `spawn.rs` checks that a run writes that gate where the run will
  find it; `normalize.rs` and `outcome.rs` read synthetic records back; `detached.rs` drives the
  detached half on a stub and checks it outlives its launcher; `status.rs` builds the three liveness
  states by hand; `cleanup.rs` drives `hydra reap` and `hydra prune` against a throwaway repo
  and checks that reap's commit reaches the branch before the checkout goes, and that neither
  command touches a launch in progress; `skills.rs` drives the store, the snapshot, the links and
  the snapshot sweep against a throwaway repo and launches nothing; `mcp.rs` generates a run's
  `mcp.json` from a fixture document, refuses the launch shape that cannot deliver a server it
  admits, and reads that refusal back; `curate.rs` builds a corpus a record at a time, checks
  which runs rung one is told about, and commits a proposal into a throwaway store;
  `delegation.rs` builds parent records by hand and drives the depth cap and the containment
  refusal, launching nothing; `face.rs` drives the built binary's delegate face over stdin and
  reads its refusals back as tool results; `show.rs` and `harvest.rs` read a normalized record and a harvested one back; `usage.rs` tallies
  two synthetic records and reads a missing one's error back; `export.rs` audits four synthetic
  records against a two-entry allowlist and aborts on a planted secret; `resume.rs` builds each
  provider's resume argv and drives the four refusals, launching nothing; `evolve.rs` drives the
  size cap, the protected-path refusal, the closed-shape parser and the harness record on
  synthetic patches and answers, launching nothing;
  `resolver.rs` feeds the prefix resolver a scratch corpus and launches nothing;
  `drift.rs` runs `claude`'s free probe under `judge`'s
  flags with stub servers in a scratch `HOME`, `codex login status`, and a paid `codex` probe of
  a hook that dies, skips without the binary, and holds the only tests in the suite that launch
  a provider. `cleanup.rs` also plants a forged pack, an alternates file, a linked fan-out
  directory and a hard link in a run's object store and reads the parent back.
- `.scratch/hydra/build/plan.md`, the build index, and `build/NN-slug.md`, one build ticket each.
- `.scratch/hydra/map.md`, the closed design map. One load gives you the whole design at low
  resolution.
- `.scratch/hydra/issues/NN-slug.md`, one ticket each. `Type:` is `research`, `prototype`,
  `grilling` or `task`. `Status:` is `open`, `claimed` or `resolved`. `Blocked by:` lists ticket
  numbers. A resolved ticket carries its decision under `## Answer`.
- `.scratch/hydra/research/NN-slug.md`, findings from resolved research tickets. Long. Read one
  only when its ticket is relevant to what you are doing.

No issue tracker is configured, so this is the local-markdown tracker. Run
`/setup-matt-pocock-skills` to move it to GitHub issues.

## The frontier

```
./target/release/hydra-puppets frontier          # the design map
./target/release/hydra-puppets frontier build    # the build
```

Prints which tickets are takeable now and which are still blocked. Claim one by setting
`Status: claimed` **before** doing any work, so a concurrent session skips it.

## Closing a ticket

A build ticket:

1. Set `Status: resolved`, and append under `## Outcome` what was built, what was measured, and
   anything the ticket assumed that turned out false.
2. **Amend `IMPLEMENTATION.md` in the same change.** A ticket is not done while that file still
   claims the gap is open, and it is the file the next session reads to know what is real.
3. Amend any later ticket the work changed, so the next session starts from the finding.
4. If the work found a decision rather than an implementation, stop and say so. That is the edge of
   the design, and it belongs on the map rather than in a commit.

A design ticket on the closed map:

1. Append the decision under `## Answer` in the ticket file, and set `Status: resolved`.
2. Add one line to `## Decisions so far` in the map: a gist plus a link. Never restate the
   decision there, the ticket holds it.
3. Amend any ticket the answer changed, so the next session starts from the finding rather than
   rediscovering it.
4. Graduate anything the answer made specifiable out of `## Not yet specified` into a real
   ticket, and clear it from the fog.

## House rules

- Prose follows `~/.claude/skills/unslop/SKILL.md`. No em dashes.
- Grilling tickets need the human. Never answer your own questions on one.
- The originating research document, `~/Downloads/Harness CLI Unificado para Cursor, Gemini y
  Claude.md`, is a starting point and not a source. Several of its claims turned out wrong.

## Nothing here is enforced

**Unless your cwd is under `~/hydra/worktrees/`.** Then Hydra launched you, you are the first
population, and the gate is real: a shell command naming `~/hydra/runs/` is refused under every
policy, and that refusal is the design working. So is a refused `Read` of `~/.claude/`: the skills
this file names by slash command live there, outside your worktree, and under `standard` the read
tool cannot reach them. Your ticket carries what you need; do not go looking for the skill.

**`cat ~/.claude/x` is not refused, and that is the honest version of the sentence above.** Under
`standard` the path scopes bind the file tools, and the shell check reads the pattern floor and the
denied write paths and nothing else. The adversarial review of 2026-09-14 measured it. The deny list
covers `~/hydra`'s runtime directories and six config files; the rest of the machine, `~/.ssh` and
the user's own guards included, is reachable by shell under `standard`. The rest of this section is
for the third.

Hydra's gate applies to runs Hydra launches, not to you. You are an agent working on Hydra's own
source, which is the one population the harness does not gate: no compiled policy, no hook config,
no injected skills, no preamble. Ticket 28 found three populations rather than two, and this is the
third.

The six protected configs in `docs/harness-spec.md` section 15.1 are protected against Hydra's own
runs. This file is what stands in their place for you. Treat them as read-only, and if a change
genuinely needs one of them, say so and let a human make it.
