# 40 - Where the `skill_store` scope points, now that two documents disagree

Type: grilling
Status: open

## Question

Graduated from `## Not yet specified`, "The curator's own worktree of the skill store",
which said the open detail was where that worktree lives and that it was sharp enough to
confirm rather than decide. Build ticket 12 built it and the confirming never happened, so
the code now answers a question the documents answer two ways.

**Is the `skill_store` scope `~/hydra/skills/` literally, or the run's own checkout of
it?** Harness spec 7.2 says the first: "`curate` writes to `~/hydra/skills/`, outside its
own worktree". `docs/ouroboros-spec.md` 3.6 says the second, and 9.1 flags it as a
refinement to confirm rather than assume.

What the code does, as of 2026-09-06: `Layout::skill_store_scope(run_id, policy)` returns
`~/hydra/skill-worktrees/<run-id>/` for `curate` and `~/hydra/skills/` for everything else.
Build ticket 12 chose that and had a reason the map should hear, because it is a better
reason than either document gives:

**The scope has to be a function both sides can compute.** Contract 7.7 re-derives every
`permission_decision` by running the run's policy again at read time. No field on the
record carries a scope. So a scope that lived only in the compiled gate would make every
curate run read back as a run whose gate leaked on its own writes, which is exactly what
happened before the fix: ten `Finding::GateLeak` on one pass, one per file the curator
touched in its own checkout. That is ticket 42's problem seen from one instance.

## Settle

- **Which document moves.** One of 7.2 and 3.6 is wrong and they cannot both stand. 7.2 is
  a table row in the harness spec and 3.6 is a paragraph in the Ouroboros spec, so this is
  a one-line edit in whichever loses.
- **Whether the answer keys on the policy name.** `skill_store_scope` branches on the
  string `curate`, which makes a policy name load-bearing in the layout. The alternative is
  a field on the policy, which is a policy schema change, or a per-run argument, which is
  what ticket 12 removed to make the replay work.
- **`~/hydra/skill-worktrees/` is a sixth place and harness spec 5 names four.** Ticket 12
  put it beside `worktrees/` rather than in the spawn directory on the grounds that `hydra
  reap` removes a spawn directory whole and a pass's uncommitted proposal is work. That
  reasoning is sound and the table still does not have the row.
- **Who removes one.** Nothing does. `hydra reap` sweeps worktrees, spawn directories and
  now skills snapshots; a curate checkout survives every command in the surface.

## Blocks

Ticket 41 needs this answered first: the order of the curate checkout and the store pin
depends on where the checkout is.

## Where the evidence is

`src/paths.rs`, `Layout::skill_store_scope`. `IMPLEMENTATION.md` on `hydra curate` and on
the skill store. Build tickets 07 and 12, both `## Outcome`.

## Prepared for the human, 2026-09-06

Not an answer. This is the half of the ticket that does not need you: what the code
does on this machine, what each option costs, and the questions left over.

Everything below was run against a throwaway `~/hydra` root under
`/private/tmp/.../t40/`, from a scratch crate depending on this library by path.
Gate scripts ran under `/usr/bin/python3` with the payload shape `tests/gate.rs`
uses. Nothing was written under `~/hydra/runs/`, `~/hydra/skills/`,
`~/hydra/skill-worktrees/`, `~/hydra/skills-snapshots/`, `~/hydra/worktrees/` or
`~/hydra/spawn/`.

### What was measured

**The scope, per policy name.** `Layout::skill_store_scope(run_id, policy)`:

| Policy | Resolves to |
|---|---|
| `judge` | `<root>/skills` |
| `curate` | `<root>/skill-worktrees/<run-id>` |
| `read-only` | `<root>/skills` |
| `standard` | `<root>/skills` |
| `permissive` | `<root>/skills` |

The arm at `src/paths.rs:116` matches the raw string, so `"Curate"`, `"curate "` and
`""` all fall through to the store. Nothing here fails loudly on a name outside the
five; `policy::NAMES` in `src/policy/mod.rs` is what stops a typo reaching it, and
the two lists sit in different files.

**The compiled `curate` gate, both ways.** Same policy, same throwaway root, one gate
compiled with `skill_store` pointing at the checkout and one pointing at
`~/hydra/skills/`:

| Call | Scope = the checkout | Scope = the store |
|---|---|---|
| `Write` under `skill-worktrees/<run-id>/` | allow | deny |
| `Write` under `skills/` | deny | allow |
| `Write` under `worktrees/<run-id>/` | deny | deny |
| `Read` under `skill-worktrees/<run-id>/` | allow | deny |
| `Read` under `skills/` | deny | allow |
| `Read` under `runs/` | allow | allow |

The reason strings, verbatim:

- `hydra/allow: policy curate allows file_write.`
- `hydra/out-of-scope: policy curate allows file_write inside skill_store and this call is outside it.`
- `hydra/out-of-scope: policy curate allows file_read inside corpus or skill_store and this call is outside it.`

The checkout-scoped gate exited 0 on all seven probes, allow and deny alike, so the
refusal rides in the body the way 6.6 requires.

**Who may write into `~/hydra/skills/` today, across all five policies.** One, and it
is not `curate`:

| Policy | `Write` into the store | Reason |
|---|---|---|
| `judge` | deny | `hydra/capability: policy judge does not allow file_write (Write).` |
| `curate` | deny | `hydra/out-of-scope: policy curate allows file_write inside skill_store and this call is outside it.` |
| `read-only` | deny | `hydra/capability: policy read-only does not allow file_write (Write).` |
| `standard` | deny | `hydra/out-of-scope: policy standard allows file_write inside worktree and this call is outside it.` |
| `permissive` | allow | `hydra/allow: policy permissive allows file_write.` |

Through `shell` the answer is different again: `standard` and `permissive` both allow
`echo x > ~/hydra/skills/naming/SKILL.md`, because the store is not one of 7.5's seven
denied paths and `shell` carries no scope. `curate` has no shell at all.

**The one real curate record, replayed both ways.** Run
`01a07513-baa3-7cf0-b20a-ded11ede6a38`, 21 tool calls, driven through both compiled
gates. Ten answers differ, all of them `Read` calls inside
`~/hydra/skill-worktrees/01a07513-.../`, and all ten flip from allow to deny when the
scope is the store. That is `IMPLEMENTATION.md`'s remembered "ten `Finding::GateLeak`"
measured rather than recalled. The pass proposed nothing, so all ten are reads and
none is a write. Most of the ten name files a curator was guessing at,
`manifest.json`, `SKILLS.md`, `skills.json`, `index.md`, which is what looking around
an empty checkout produces.

`cargo test --test normalize` passes today, 32 tests in 1.09 s, including
`no_record_on_this_machine_shows_a_gate_that_leaked_or_stayed_silent`. That test is
what would break.

**What is on disk.** `git -C ~/hydra/skills worktree list` returns one line, the store
itself, at `b0541a8` on `master`. Five commits, two of them
`hydra: the store was dirty at launch, so it was committed before pinning`. The
working tree is clean and holds no skill: the store is `.git` and nothing else.
`ls ~/hydra/skill-worktrees/` is empty. So the one real pass took its own checkout
back, which is `git worktree remove` and `git branch -d` doing their job.

Of the 16 records under `~/hydra/runs/`, 13 carry `policy_sha`
`536974998142fa4eb0e2ecdc533e2af39eb02d9412913b581b39d788a7ccf811` and 3 predate the
gate. One names `policy: curate`.

**What reap and prune do to a curate checkout.** Measured on a throwaway root holding
a run worktree with an uncommitted file, a spawn directory, a snapshot, and a curate
checkout holding an uncommitted `proposal.md`. `hydra reap` committed the run's work
to `hydra/<run-id>`, removed the worktree, removed the spawn directory and swept the
snapshot. The checkout and `proposal.md` survived. A named `hydra reap <run-id>` left
them too. `hydra prune` reported `nothing to prune`. Afterwards the store's
`git worktree list` still carried the checkout on `ouroboros/curate/<run-id>`.

That the checkout survives is measured. That nothing anywhere could have removed it is
read: `skill_worktree` has no caller in `src/reap.rs`, `src/prune.rs`, `src/status.rs`
or `src/cli.rs`, and its only callers are `src/run.rs:658`,
`src/normalize/claude.rs:483`, `src/ouroboros/curate.rs:93` and `tests/curate.rs`.

**Ticket 41's second ordering costs nothing at compile time.** A gate compiled while
`skill-worktrees/` does not exist yet produces a byte-identical `scopes` block to one
compiled after the checkout is there, and both allow a write inside it. The caveat is
`resolved()`'s: it canonicalizes the longest existing ancestor and appends the rest, so
a symlinked component that appears only after compilation would leave the two strings
apart. `Layout::discover` reads the passwd home, and `/Users/USER/hydra` has no such
component.

**The two sentences.**

`docs/harness-spec.md:661`:

> `curate` writes to `~/hydra/skills/`, outside its own worktree.

`docs/ouroboros-spec.md:264`:

> **The curator writes into its own git worktree of the store, never into `~/hydra/skills/` itself.**

### What the ticket claimed that turned out different

**The disagreement is four places, not two.** Beside 7.2 and 3.6, `docs/harness-spec.md:846`
says a request to write into the store "is refused because the gate denies writes to
the store under every policy but `curate`". Measured, that is wrong in both
directions: `curate` is denied and `permissive` is allowed. And `src/policy/mod.rs:70`
documents `Scope::SkillStore` as "`~/hydra/skills/`. `curate` writes here, outside its
own worktree", which is 7.2's reading sitting in the code that no longer does it.

**Nothing gated can write to the store at all.** The ticket frames this as where one
scope points. The consequence is larger: with the scope on the checkout, the store's
own working tree has no gated writer, only `permissive`, `shell`, and Hydra's own
`Store::pin`. Whether that is the intended shape is part of question 1.

**Harness spec 5 names four rows and says three.** The ticket's "harness spec 5 names
four" is right about the table, lines 107 to 110. Line 102 opens the section with
"Three places." `Layout` has seven directories: `runs`, `worktrees`, `spawn`,
`skills`, `skills-snapshots`, `skill-worktrees`, `pruned`. So the count was already
off before this checkout existed.

**A leftover checkout is visible in Hydra's own `git status`.** `.gitignore` anchors
six of the seven and has no `/skill-worktrees/` line. Reproduced in a throwaway repo
with the same shape: a checkout under `skill-worktrees/` shows as `?? skill-worktrees/`,
and `git add -A` warns `adding embedded git repository`. It is invisible today only
because git does not track an empty directory, and `~/hydra` is this checkout on this
machine.

**The existing gate test does not test this rule.** `tests/gate.rs`'s
`curate_writes_inside_the_skill_store_and_nowhere_else` builds its own `Spawn` and
passes a directory it named `skills`, so it never calls `skill_store_scope` and passes
under either reading. `tests/curate.rs` covers the compiled string and the replay;
nothing drives the compiled gate at a real checkout path.

### The options, costed

#### Which document moves

**A. The checkout wins. 7.2 moves, the code stands.**

Prose only, about 20 lines, no logic. `docs/harness-spec.md:661` becomes something
like: "`curate` writes into its own checkout of `~/hydra/skills/`, outside both that
store's working tree and the run's own worktree. It and `permissive` are the two
policies that break the worktree-write assumption, which section 7.5 handles." Line
846's clause is corrected in the same pass. `docs/ouroboros-spec.md` 9.1, ten lines at
861 to 870, collapses to a sentence saying it was confirmed and where. The two code
doc comments follow.

Tests: none break. One worth adding, roughly 25 lines in `tests/gate.rs`, the shape
run by hand above: a `curate` gate compiled from `Layout::skill_store_scope` allows a
write in the checkout and refuses one in the store.

Corpus: zero. `policy_sha` does not move, the 13 records that carry one derive exactly
as they do now.

Ticket 41: both orderings stay live, and the measurement above says the second one
compiles identically. Ticket 42: unchanged, and this becomes its worked example.

**B. The store wins. 3.6 moves, the code changes.**

`src/paths.rs` loses lines 86 to 120, 35 lines. `src/run.rs:658` and
`src/normalize/claude.rs:483` lose a call and a comment each, about 14 lines together.
`src/ouroboros/curate.rs` mentions the checkout on 42 lines: `check_out`,
`undo_checkout`, the `Done::Handed` arm, `propose`'s parameter and two paragraphs of
`brief()`. Call it 100 lines. `tests/curate.rs` mentions it 21 times, and the two
tests at 589 to 684, 96 lines, are about this rule and would invert or go.

Corpus: the ten reads above become ten `Finding::GateLeak` on record
`01a07513-baa3-7cf0-b20a-ded11ede6a38`, and `tests/normalize.rs` fails on the next
run. Fixing that means pruning the record or writing an exception, and both are ticket
42 arriving early through the door it warned about.

Run time: the curator writes into the live store, so a concurrent `hydra run` that
pins a dirty store runs `git add -A` over a half-written proposal and commits it to
`master` under "the store was dirty at launch". That collision is the whole reason 3.6
exists. Rung one's "nothing lands" also loses one of the three mechanisms
`src/ouroboros/curate.rs`'s module doc names, and the first one is the only gate among
them.

Ticket 41: dissolves. No checkout, no ordering, and it should be closed with a note
rather than answered. It is replaced by the pin-versus-curator race above, which has
no ticket. Ticket 42: narrows. `skill_store` stops being per-run, so `worktree` is the
only scope carrying a run id and 42's fourth bullet becomes the whole of 42.

**C. Both stand, and the name changes.** The checkout gets its own scope name,
`skill_checkout`, and `skill_store` keeps meaning `~/hydra/skills/`. 3.6 stands
unedited; 7.2's row becomes "reads the corpus, writes only inside its own checkout of
the skill store".

`policies/curate.json` changes one line, and it is protected config #1, so a human
edits it. `Scope` gains a variant and an `as_str` arm, 6 lines. `compile()` emits a
fourth entry in `scopes`, 1 line. `src/paths.rs` resolves per scope name and the
`match` on `"curate"` goes, about 10 lines. `src/run.rs` and `src/normalize/claude.rs`
pass two paths instead of one, 2 lines each. `src/policy/gate.py` needs nothing:
`scopes_for` already looks names up in a dict. Containment in `Policy::contains` is
unaffected, because `anywhere` still subsumes a scoped pair.

The cost is one number. Editing anything in `policies/` moves `policy_sha`, and 13 of
the 16 records carry today's. Every one then reports `Finding::StalePolicy`
permanently, since nothing recovers an old policy directory from a sha. No decision
flips; the corpus just carries 13 findings saying the rules moved.

#### Whether the answer keys on the policy name

Status quo is the string match at `src/paths.rs:116`, measured above to be silent on
anything that is not exactly `curate`. Option C removes the branch and makes a scope a
function of `Layout`, the run id and the scope name, which is one fewer thing for
ticket 42's guard to assert. A per-run argument is what build ticket 12 removed: it
costs 42's first bullet outright, a `scopes` object on `run_start`, a schema bump under
contract 15, and a reader that handles its absence on all 16 existing records.

#### The sixth place

Leave the table alone, or add the row. Adding it is three lines: the row itself plus a
sentence saying why it is not in the spawn directory, and the "Three places" at line
102 fixed in the same edit since it is already wrong by one.

Moving the checkout into the spawn directory is what section 9.1 suggested and build
ticket 12 refused. The reap measurement above is the evidence: one sweep removed the
spawn directory whole and left the checkout with its uncommitted proposal.

Separately, `/skill-worktrees/` in `.gitignore` is one line. `.gitignore` is not one of
15.1's five protected configs.

#### Who removes one

**Nothing, as today.** Bounded rather than unbounded: a pass that ends inside its call
removes its own checkout, so only a handed-off pass (`Done::Handed`) or a failed
removal leaves one. Cost is a directory per leak, a stale entry in the store's
`git worktree list`, and the `git status` noise above.

**`hydra reap` sweeps them, commit first.** Commit the checkout to
`ouroboros/curate/<run-id>`, then `git worktree remove` without `--force`, which is
exactly `remove_checkout`'s existing shape and the same guarantee: a commit that did
not happen stops the removal. Roughly 40 lines in `src/reap.rs` and 30 in
`tests/cleanup.rs`. It answers build ticket 12's objection, since the proposal survives
on its branch rather than being taken. It does not break harness spec 10's "Ouroboros
cannot call either", because reap stays a command a human types.

**`hydra status` reports one and removes nothing.** Roughly 15 lines. Keeps every
removal in reap and puts the orphan where the human already looks.

**The next `hydra curate` tidies an older one.** Worth naming so it can be refused:
`hydra curate` has no liveness test for another pass, so it would take work from a
curator still running.

### Questions for the human

Ordered so an early answer can moot a later one. I have not answered any of them.

**1. Which reading is the design: the run's own checkout of the store, or
`~/hydra/skills/` literally?** Pick one. If the store literal wins, questions 2 to 5
mostly evaporate and ticket 41 loses its subject.

*My opinion:* the checkout, because `~/hydra/skills/` is the one
directory a concurrent `Store::pin` commits without asking, so a curator writing there
hands its half-written proposal to whichever run launches next.

**2. If the checkout wins, does `skill_store` stay one name resolved per policy, or
does the checkout get its own scope name?** Keep the branch in `src/paths.rs`, or spend
a `policy_sha` change and 13 permanent `StalePolicy` findings on `skill_checkout`.

*My opinion:* keep the branch now and take the fourth scope name the next time
something else already needs a policy edit, because a comment buys most of the clarity
and 13 findings on a 16-record corpus is a real price on the only evidence there is.

**3. Does harness spec 5's table gain a row for `~/hydra/skill-worktrees/<run-id>/`,
and does the section stop saying "three places"?** Yes or no to each.

*My opinion:* yes to both, in one edit, since the count is wrong today regardless of
what this ticket decides.

**4. Who removes a leftover checkout: nothing, `hydra reap` with the commit-first rule,
or `hydra status` reporting it?**

*My opinion:* `hydra reap`, because commit-first is already the rule that makes reap
safe on a run's worktree and this is the same shape of work with the same failure mode.

**5. Does `.gitignore` gain `/skill-worktrees/`?**

*My opinion:* yes, and it is one line.

### What resolving this changes elsewhere

**Unblocks 41.** If the checkout wins, both of 41's orderings stay on the table and the
byte-identical compile above removes the objection to the second. If the store wins, 41
has no subject and should be closed with a pointer here rather than answered.

**Narrows or feeds 42.** 42's second bullet, a test asserting every scope is a pure
function of `Layout`, the run id and the policy name, gets written against whatever
this ticket picks. Option C would change that sentence to "the scope name" and remove
one input.

**Documents to amend:** `docs/harness-spec.md` section 5 (the table and its opening
count), 7.2's `curate` row, and the clause at line 846; `docs/ouroboros-spec.md` 3.6
and 9.1.

**Code comments carrying the losing reading:** `src/policy/mod.rs:70`,
`src/paths.rs:88-94` and `src/paths.rs:103-120`.

**`IMPLEMENTATION.md`:** the two paragraphs under "Ouroboros rung one" beginning "Where
that scope points" and "The checkout lives at", and the "Rough edges" entry that points
at 42 and here.

**Build ticket 12's outcome:** its bullet saying section 9.1's spawn-directory
suggestion does not hold becomes a settled decision rather than a build's choice.

**The map:** one line under `## Decisions so far`, and the reopened four become three.
