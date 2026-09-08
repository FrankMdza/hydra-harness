# Hydra

Hydra is a router over three agent CLIs (`claude`, `cursor-agent`, `agy`), plus Ouroboros, a
self-improvement loop that reads what the router records. The router core is Rust. `hermes` was a
fourth until ticket 22 unrouted it; its guardrail rules are still copied, its binary is not launched.

This repo holds a finished design, three documents in `docs/`, written from a wayfinder map that
closed on 2026-09-05 with 38 of 38 tickets resolved and all four seam checks passing. It also holds
the Rust that implements six of that design's eleven commands: `run`, `status`, `logs`, `reap`,
`prune` and `curate`.

**Read `docs/` first, not the map. Then read `IMPLEMENTATION.md`**, which is the gap between the
design and the code. The code is deliberately not spec-complete: all three providers are routed and
gated, with skills and MCP injected into each, so that file says what a record does and does not
carry and which of the three has never written one.

The three documents are the design. `docs/harness-spec.md` is how Hydra launches, feeds and records
the providers; `docs/run-record-contract.md` is the format it writes and Ouroboros reads;
`docs/ouroboros-spec.md` is both rungs of self-improvement. They are meant to be read together, and
nothing crosses between the harness and Ouroboros except the run record and the skill store.

## The map

`.scratch/hydra/map.md` is the history: the destination, the settled decisions D1 to D16, a one-line
index of the closed tickets, what is still fog, and what is out of scope. Read it when you need to
know **why** a document says what it says. The ticket behind any decision holds the reasoning the
document compressed.

**The map is open again at 46 of 49, and nothing open blocks the build.** It reached its destination
on 2026-09-05 and closed; build tickets 07, 08 and 12 then hit five things that need a decision
rather than an implementation, which is the one condition for reopening it. Tickets 39 to 43 are
those, three of them about what a policy scope is and who gets to know, one about provider drift.
All of them, plus 44 to 46, closed on 2026-09-06, and build tickets 13 to 16 carry their code.
The build closed at 16 of 16 on 2026-09-07 and opened 47 and 48, two grillings, on the way, then
reopened with ticket 17 the same day; 49 is the Herdr bridge's two decisions. The
map's own paragraph under `## Destination` says what each left. **If a build ticket finds a
decision rather than an implementation, it reopens the map with a new ticket rather than deciding.**

**Wayfinder is still not how the build continues**: it charts decisions, its tickets resolve
questions rather than execute slices, and its own instructions say the pull to just do the work is
the signal you have reached the edge of the map. Take a map ticket only when the work is a decision:

```
/mattpocock-skills:wayfinder .scratch/hydra/map.md
```

## The build

`.scratch/hydra/build/plan.md` indexes the build and `.scratch/hydra/build/NN-slug.md` are its
tickets, same format as the map's, `Type: build`. Start a session like this:

```
python3 .scratch/hydra/frontier.py build
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
  reached from nowhere but `main`; `record/` is the run record and the validator the tests
  drive; `env.rs` is the allowlist; `pty.rs` owns the child's pty; `provider/claude.rs`,
  `provider/cursor.rs` and `provider/agy.rs` hold every fact that is true of one binary and
  not of a model, and the three files beside them in `normalize/` read each one's records
  back; `policy/` loads the policy set, owns the capability table per provider, and compiles
  the gate script;
  `skills/` is the store, the per-sha snapshot, the read-only index over the user's own skill
  roots and the collision check;
  `mcp.rs` reads the canonical MCP document and decides what one launch is handed;
  `preamble.rs` composes what the run is told; `face.rs` is the MCP delegate face of harness
  spec 9, a stdio server a run's own provider spawns from the MCP document, and the one place
  a child's text is marked on the way back; `ouroboros/` is rung one, `hydra curate`, and
  the only place Ouroboros lives, which is what makes "it calls neither cleanup command" a
  property you can check by reading one directory.
- `policies/`, the five policies of harness spec 7.2 and the pattern floor of 7.4. Protected config
  #1, and `policy_sha` on every run record is its sha. `policies/README.md` is the format.
- `config/`, protected configs 2 to 5 of harness spec 15.1. `preamble.md` and `mcp.md` exist;
  `config/README.md` says what the other two are waiting on.
- `IMPLEMENTATION.md`, what is built, what is not, and every departure from the design with its
  reason. Amend it in the same change that narrows the gap.
- `tests/`, sixteen files. `record_contract.rs` checks a written record against the contract and walks
  every record on this machine; `pty_tee.rs` drives a stub provider so the pty path costs no quota;
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
  reads its refusals back as tool results; `drift.rs` runs `claude`'s free probe under `judge`'s
  flags with stub servers in a scratch `HOME`, skips without the binary, and holds the only
  tests in the suite that launch a provider.
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
python3 .scratch/hydra/frontier.py          # the design map, 46 of 49 resolved
python3 .scratch/hydra/frontier.py build    # the build
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

Hydra's gate applies to runs Hydra launches, not to you. You are an agent working on Hydra's own
source, which is the one population the harness does not gate: no compiled policy, no hook config,
no injected skills, no preamble. Ticket 28 found three populations rather than two, and this is the
third.

The five protected configs in `docs/harness-spec.md` section 15.1 are protected against Hydra's own
runs. This file is what stands in their place for you. Treat them as read-only, and if a change
genuinely needs one of them, say so and let a human make it.
