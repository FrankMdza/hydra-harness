# 68. An attack corpus, and whether Hydra gets an immune loop

Type: research
Status: resolved
Blocked by: none
Opened: 2026-09-22, by the human, during the study session before publishing.

## The question

The human asked for an immune system: an agent that researches the current methods for
attacking coding agents, tries each one against Hydra, and builds a barrier, on a schedule,
patching what it finds. Two settled decisions say no to the shape as asked. D10 says nothing
has to be running for the harness to work, and `docs/rungs-spec.md` section 1 says both rungs
stop at a branch a human merges, which is the sentence that document calls its most important.
A loop that patches its own security is the loop section 3.6 says walks around every gate on
this map. Rung one is invoked rather than scheduled (3.1), and scheduling is a graduation
condition (section 8), not a default.

So the ticket asks the narrower thing that fits the seam:

1. **What are the attack classes against coding-agent harnesses today**, from primary sources
   (vendor advisories, the OWASP agentic work, arXiv, the Claude Code, Cursor, Codex and
   Antigravity changelogs and security notes, published CVEs on MCP servers and hooks)? Prompt
   injection through files, tool results and MCP tool descriptions; tool poisoning and rug
   pulls on MCP servers; hook and settings tampering; environment injection; symlink and path
   assembly escapes; exfiltration through fetch tools and the vendor's own API; unicode and
   casefold spellings; parallel-call races; escalation through a relaunched binary.
2. **Which of those Hydra already answers, by which mechanism, and where the test is.** The
   gate (`src/policy/gate.py`, `tests/gate.rs`), the seatbelt (`src/sandbox.rs`,
   `tests/spawn.rs`), the allowlist (`src/env.rs`), containment (`src/containment.rs`), the
   protected paths (harness spec 15.2), the marking of a child's result (`src/face.rs`).
   `SECURITY.md` and `docs/reviews/2026-09-14-the-gate-under-attack.md` are the prior art
   inside the repo and both must be read before anything is claimed new.
3. **Which of those Hydra does not answer**, split into the ones `SECURITY.md` already states
   as limits and the ones nobody has written down.
4. **What an attack corpus looks like as a mechanism Hydra can own.** The shape that fits is
   `tests/gate.rs`'s: one probe per class, driven against the compiled gate and, on macOS,
   against the real profile, launching no provider, with a probe that pins an open hole as a
   passing test and a comment telling whoever closes it to rewrite the test. Propose the file
   layout, the probe format, and how a new class enters the corpus. Say what a probe needs that
   the current test harness does not give it.
5. **How a fix flows.** Through `hydra evolve` with the corpus in the admission gate, so a patch
   that closes a probe is reviewed by the panel and merged by a human, or through a build
   ticket a human writes. Say which, and what the pass needs to see.
6. **Whether a research agent that updates the corpus is a rung, a `hydra run` under an
   existing policy, or a thing outside Hydra**, and what it reads and writes. Name what it
   would need that the five policies do not grant.

## Rules for whoever takes this

- Read `SECURITY.md`, `docs/harness-spec.md` sections 6.2, 6.6, 7, 8, 9 and 15,
  `docs/rungs-spec.md` sections 1, 2 and 4, `docs/reviews/2026-09-14-the-gate-under-attack.md`,
  `docs/reviews/2026-09-22-seven-readers-as-a-hiring-panel.md`, `src/policy/gate.py`,
  `src/sandbox.rs`, `src/env.rs`, `src/containment.rs` and the test names in `tests/gate.rs`
  and `tests/spawn.rs` before writing a claim. Cite a file and a line for every claim about
  the code.
- Every source outside the repo gets a URL and the date read. A claim with no source is a
  hypothesis and says so.
- **Try a method only where trying it launches no provider and touches nothing outside a
  scratch directory.** `tests/gate.rs` shows how: compile a gate into a scratch layout and feed
  it a payload through `/usr/bin/python3`. Do not run a probe against the real corpus, the real
  `HOME`, `~/.ssh` or any credential. Do not launch `hydra run`.
- **A hole this finds open is held, not published.** `docs/reviews/README.md` forbids
  committing a scenario a reader could use before the fix. Name the class and the mechanism in
  the research document; put the exact spelling, payload or command in a separate file named
  `68-open-holes-<reader>.md` beside it, which the session moves out of the repository before
  anything is committed.
- Do not decide. This is a research ticket; the decision it feeds is the human's, on this
  ticket's `## Answer`. Where two options exist, cost both in files, lines and tests.
- Prose follows `~/.claude/skills/unslop/SKILL.md`, which is not readable from inside a run,
  so the rules in one line: no em dashes, no bold-label lists, sentence case, plain words, say
  what the mechanism is rather than how it feels.

## Deliverables

One document per reader under `.scratch/hydra/research/`:

- `68-attack-corpus-opus.md`, from a Claude Code subagent on Opus reading the checkout.
- `68-attack-corpus-codex.md`, from a `codex` run under `standard` in its own worktree.

Each with these sections, in this order: `## Sources`, `## Attack classes` (a table: class,
source, what it needs, which providers of the four), `## What Hydra answers` (a table: class,
mechanism, file and line, test), `## What Hydra does not answer` (stated limit against
unstated), `## The corpus as a mechanism`, `## How a fix flows`, `## The research agent`,
`## What I tried` (each probe, what it did, what came back, launching nothing),
`## Questions for the human`.

## Answer

Resolved 2026-09-22 with the human, from the Opus reader's seven questions. The `codex` reader
refused the task on its wording (run `01a0ca7d-f703`, `failed`, reaped) and does not get a second
research pass; it reviews the fixes instead, with the task written as a defensive review.

1. **H4, the non-ASCII root.** Hydra refuses a `hydra_home()` whose path is not ASCII, at pre-flight
   and in `hydra doctor`. Build ticket 60's ASCII fold stays, and the refusal makes its assumption
   a checked precondition rather than a residual. Inode comparison fails for paths that do not
   exist yet, which is every new write. Build ticket 78.
2. **H1 to H3, the raw pass.** The claim gets narrowed to what the code does. The docstring at
   `src/policy/gate.py:293` stops saying the corpus, store, snapshots and spawn directory are
   covered whatever the spelling; `SECURITY.md` names a path assembled inside an interpreter token
   or laundered through `..` as part of "does not interpret shell"; H1 to H3 are pinned as open
   the way `tests/gate.rs:1766` pins the sibling-worktree residual. No normalization step: it
   closes spellings and not the class, which is 7.4's failure mode in a second place. Build
   ticket 77.
3. **H5 and H6, payload shape.** Fixed in the gate. `evaluate` reads the payload in the dialect
   the gate was compiled for and refuses one that carries another dialect's envelope, the same
   rule argv already follows at `src/policy/gate.py:858`. A `tool_input` that is not an object is
   refused as `hydra/bad-payload`, like the non-string command at `:782`. The probes go in the
   attack corpus, which runs in CI, not in `tests/drift.rs`, which skips without the binary.
   Build ticket 77.
4. **`network_read`.** The floor's Slack rules also run under `network_read` now (build ticket
   77). Whether `network_read` gets a scope or a domain list is a design question with a cost per
   provider, and it goes to map ticket 69.
5. **`tests/` and rung two.** Option A now: `tests/corpus` joins `PROTECTED_CODE_PATHS` (build
   ticket 79). Option B, baseline-test provenance as a mechanism, is build ticket 80, open.
6. **Classes that cannot apply.** A row in the corpus README saying why, and no test. An empty
   test asserts something that is not true.
7. **The research agent.** An ordinary `standard` run, with the corpus left out of its evidence.
   Whatever it needs from a record, the session quotes into its task. No sixth policy, no two-run
   split.

What stays out of the repository: the exact spellings in `68-open-holes-opus.md`, H7 among them,
which is the floor's documented limit two characters from a working payload. The research
document enters the repository once build ticket 77 has written H1 to H3's class into
`SECURITY.md`, because from then on it names a stated limit rather than an unstated hole.

The human also asked for a scheduled immune loop, and the answer to that half is no, for the
reasons under `## The question`: D10, rungs spec 1 and 3.6. What exists instead is a corpus a
human or a research run adds a class to, a build ticket per fix, and rung two unable to edit the
corpus it would be graded by.
