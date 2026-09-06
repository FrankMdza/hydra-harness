# 07 - The skill store, the snapshot, and per-run injection

Type: build
Status: resolved
Blocked by: 02

## Goal

One canonical store, snapshots pinned per run, links into the worktree, and the two
collision checks. `claude` first.

## Where the design says it

- Harness spec 6.4, all of it. The store layout with `hydra.toml` beside each
  `SKILL.md`, injection per run into the worktree and never into the user's home, the
  self-ignoring `.gitignore` holding `*`, snapshots at a pinned sha, committing a dirty
  store before pinning, the read-only index over user-scope roots, the two collision
  checks, and the write-time YAML validator.
- 6.1 items 5, the launch-time collision check as its own run outcome.
- Contract 5.3, `skills.store_sha`, `skills.linked` and `skills.resolved`, and why the
  third exists.

## Done when

- `~/hydra/skills/` is a git repo, snapshots are refcounted per sha, and links point at
  a snapshot rather than the live tree.
- A dirty store is committed before pinning, with authorship in the commit trailer.
- The launch check asks the provider what it resolved rather than comparing strings,
  and a hit fails the run as `skill_collision`.
- A description containing a colon followed by a space is refused at write time.
  `~/.claude/skills/handoff-frank` fails this today.
- `run_start.skills` stops being absent and carries all three fields.

## Watch for

**Injection has a measured side effect on `claude` and the spec states it rather than
hiding it.** Claude Code fits its whole skill listing into a character budget and renders
the overflow as bare names. Nine injected links moved bare entries from 8 to 14, and
three of the six newly stripped were the user's own plugin skills. Nothing warned. That
is why `skills.resolved` exists and why the record cannot claim what the model saw
without it.

**There is no shared notion of a skill's identity.** `claude` keys on the directory
name, `agy` on the frontmatter `name`, and Cursor's listing has no name field at all.
One probe directory named one thing and declaring another is a different skill to each
of them.

## What ticket 01 left you, added 2026-09-05

**`curate`'s write scope is `~/hydra/skills/` literally**, which is what harness spec
7.2 says. `docs/ouroboros-spec.md` 3.6 has the curator working in its own git worktree
of the store instead, and 9.1 flags that as a refinement to confirm rather than assume.
Beside it: 3.7 has the curator commit to `ouroboros/curate/<run-id>` in the store's
repo, and `curate` grants no `shell`, so either the supervisor commits or the policy
needs something it does not have. Ticket 01 did not decide it. If this ticket settles
where the store's worktree lives, the `skill_store` scope in `src/policy/mod.rs` moves
with it.

## What ticket 06 left you, added 2026-09-05

**Reap does not touch `~/hydra/skills-snapshots/`, and deciding who does is yours.**
Harness spec 5's table gives the snapshot a different lifetime from the other three
rows: created per sha, shared and refcounted, reaped "when no live run references it"
with no command named. `src/reap.rs` removes a run's checkout and its spawn directory
and stops there, so a snapshot left behind by this ticket's work has no reaper until you
write one. `hydra::live::scan` is how you ask what still references one, the same
function reap and `hydra status` use.

**`~/hydra/worktrees/` and `~/hydra/spawn/` are empty now**, so a skills link you find
inside a checkout is one this ticket made rather than one an old run left.

## Outcome

Built for `claude`, 2026-09-06, on 2.1.263. Four of the five "Done when" items hold
outright and the fifth is a rule with no caller, said plainly below.
`IMPLEMENTATION.md`'s new section "The skill store, the snapshot and what a run is
linked" is the long version of everything here.

**What was built.** `src/skills/` is the store, the per-sha snapshot, the read-only
index over both scopes, the write-time validator and the three collision tests.
`~/hydra/skills/` is a git repo created on first launch with an empty root commit, so
there is always a sha to pin. A dirty store is committed before pinning, committer
`hydra <hydra@localhost>`, trailer `Hydra-Pinned-By: <run-id>`. Links point into
`~/hydra/skills-snapshots/<sha>/`, a detached git worktree shared per sha.
`<worktree>/.claude/skills/` gets a `.gitignore` of `*` and one symlink per admitted
skill. `run_start.skills` carries `store_sha`, `linked` and `resolved` on every run with
a checkout, and is absent on a run with none, because 6.4 puts injection in the worktree
and nowhere else. `tests/skills.rs` is fifteen offline tests; the validator, the index,
the three collision tests and the two pure halves of the probe are unit tests beside
their code.

**Who reaps a snapshot, which ticket 06 left open.** `hydra reap`, in both of its modes,
because the refcount is global. The count is the corpus rather than a file:
`skills::referenced_shas` reads `skills.store_sha` off every live run's line 0. Two
guards, and the second is the one worth remembering: a checkout younger than the launch
grace whose record has not spoken stands the whole sweep down, because a launch reuses an
existing snapshot without touching it and age cannot see that one.

**What was measured.**

- The free names oracle survives on 2.1.263. `claude -p x --model <unknown> --session-id
  <uuid>` in the checkout costs 1.8s, no API call, and writes a transcript carrying a
  `skill_listing` attachment with `skillCount` and `names`. `--session-id` was added to
  ticket 24's recipe so the read is a predictable path rather than the newest file in a
  directory another session may be writing. The project slug rule was measured rather
  than guessed: every character that is not a letter, a digit or a hyphen becomes one.
- A real gated run with two links wrote all three fields, and both linked names came back
  in `resolved`, which was 28 names against 26 without them.
- The listing side effect reproduced and landed on Hydra's own skills. 7,950 characters,
  28 entries, 5 rendered bare, and **both injected store skills were among the five**. So
  the run's model was shown two names with no description, and `store_sha` plus `linked`
  would have claimed otherwise. That is the argument for `skills.resolved` arriving on the
  first run that used it.
- The collision check refused a real launch in 2.3s: `outcome: skill_collision`,
  `outcome_source: refusal`, null exit code, no child. Contract 8's `skill_collision` has
  a producer now.
- Reap swept both snapshots through git, so the store can check the same sha out again
  rather than refusing a path it still thinks it owns.

**The fifth "Done when" is a rule without a caller, and the ticket wrote it as
though the caller existed.** "A description containing a colon followed by a space
is refused at write time" needs a write. 6.4 gives the write to the curator, and no
code path in this build writes to the store: the curator writes through its own
gated tool calls and the step that turns those into a store commit is rung one's.
`skills::admit` is the check, with a test and no caller, in the shape
`record::Refusal` was written in before any of its four checks existed. The launch
check is not a substitute for it: it fails the whole run, which 6.4 calls harsh and
justifies by what the write check does to the odds. Ticket 12's curate pass commits
what the run left in the store, and that is where the caller belongs.

**Two more corrections to what the first draft of this Outcome claimed.**

The validator holds two rules beyond the file format, not one. The frontmatter
`name` must equal the directory name, because `claude` keys on the directory and
`agy` on the frontmatter and a store entry where they disagree is a different skill
to each. And `disable-model-invocation: true` is refused. Both are stricter than 6.4
says in words; both follow from what it asks the store to do.

`skills.resolved` answers identity and not what the model saw. Contract 5.3 wants
the field because the listing renders overflow as bare names, and the free probe
strips every filesystem description, so the field cannot carry description survival.
The paid run's own transcript can and nothing reads it. That is a smaller field than
5.3's paragraph implies, and it is the largest one that costs nothing.

**Two things worth a reader's attention that this ticket did not create.**

The probe makes `claude` write `~/.claude/projects/<slug>/<session>.jsonl` per
launch and nothing reaps it. D9 says Hydra never writes outside a path it created,
and Hydra does not write this: the provider does, as it already does for every
recorded run in the same directory. The probe adds a second file per run to a
directory ticket 02's runs were already filling.

A recorded run with no checkout leaves `skills` absent, which is a record shape
contract 5.5 and 5.6 do not enumerate. It follows from 6.4 putting injection in the
worktree and nowhere else, and from the absent-against-null rule those two sections
set. Worth confirming rather than inheriting.

**What the ticket assumed that turned out false.**

`~/.claude/skills/handoff-frank` does not fail the YAML rule any more. Ticket 24 measured
that on 31 August and the file has been edited since; its description is 656 characters
and parses cleanly. Measured with the validator itself, nothing in `~/.claude/skills`
fails the YAML rule today, so the proof is a fixture. Two files do carry `: ` and both
quote it, which is the case a regex would have refused and a parser does not, so the
spec's insistence on a parser earned itself twice over.

The other surprise is a rule the ticket did not ask for. Eight of the eleven skills in
`~/.claude/skills` set `disable-model-invocation: true` and none of the eight appears in
`claude`'s listing. A store skill setting it would be linked and then indistinguishable
from one a collision swallowed, so the validator refuses it.

**What this ticket did not settle, and it is a decision rather than an implementation.**

A linked skill is unreachable under four of the five policies, measured with a real run
and with that run's own gate. `Skill` is `provider_native`, which only `permissive`
grants, so the run cannot invoke it. And the link resolves into the snapshot, outside the
checkout, so `file_read` scoped to `worktree` denies reading the `SKILL.md` too. Both
denials are correct against the documents as written: 6.4 puts the links in a snapshot,
7.2 scopes `worktree` to the checkout, and no closed ticket connects them. Injection
therefore delivers a name and a description into the system prompt and nothing else,
except under `permissive`.

Beside it, the question ticket 01 handed over is still open and was deliberately not
answered here. `Scope::SkillStore` still points at `~/hydra/skills/` literally, which is
harness spec 7.2. `docs/ouroboros-spec.md` 3.6 has the curator working in its own
worktree of the store and 9.1 flags that as a refinement to confirm. Moving the scope is
the same decision as the paragraph above, seen from the write side.

Both belong on the map.
