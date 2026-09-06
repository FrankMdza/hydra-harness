# 39 - A linked skill is unreachable under four of the five policies

Type: grilling
Status: open

## Question

Build ticket 07 wired skill injection and then measured what a run can do with an injected
skill. The answer is nothing, under four of the five policies, and both halves of that are
the documents working as written.

**Is injection supposed to deliver a skill the run can use, or a listing entry the model
can read about?** Today it delivers the second, and no document says which was meant.

Measured 2026-09-06, `claude` 2.1.263, run
`01a07512-d36e-7651-b8d9-e9e279e5730b` under `standard` with two skills linked and a task
naming one of them:

- The gate refused `Skill` with `hydra/out-of-scope`, the model replied "The policy
  compiled for this run does not allow the Skill tool", and the run derived `gated`.
  `Skill` maps to `provider_native` in `src/policy/table.rs`, which only `permissive`
  grants.
- Driving that run's own compiled gate by hand, a `Read` of
  `<worktree>/.claude/skills/hydra-marker/SKILL.md` is refused too. The link resolves into
  `~/hydra/skills-snapshots/<sha>/`, outside the checkout, and `file_read` scoped to
  `worktree` denies it. The gate resolves symlinks before comparing, deliberately, and
  `tests/gate.rs` calls that `a_symlink_out_of_the_worktree_does_not_widen_the_scope`.

So under `judge`, `read-only`, `standard` and `curate` a linked skill reaches the model as
a name and a description in the system prompt and no further. Under `permissive` it works,
because that policy grants `provider_native` and `file_read` anywhere.

## Settle

- **Whether `provider_native` should be split.** `src/policy/table.rs` maps nineteen tools
  to one capability so that `permissive` can allow everything without inventing nineteen
  categories, and `IMPLEMENTATION.md` says splitting it is a design decision the build did
  not make. `Skill` and `ToolSearch` are the two that cost an ordinary coding run
  something. If they move, decide what they move to and which policies grant it.
- **Whether the snapshot is inside a run's read scope.** The alternatives are a scope that
  names the run's own snapshot beside the worktree, a link into the worktree that copies
  rather than points, or leaving it as it is and accepting that a skill is a listing entry.
  A copy defeats the sharing the snapshot exists for; a second scope path is a change to
  what `worktree` means; leaving it costs the feature.
- **What harness spec 6.4 should say either way.** It spends four paragraphs on how the
  links are made and none on whether the run may follow one. Whatever this settles, that
  section is where a reader will look for it.
- **Whether `curate` is a special case.** Rung one is the population most likely to want a
  skill it was linked, and it is one of the four that cannot reach one.

## What it costs to leave open

Injection is built and measured and every run pays 1.8 seconds for the probe that records
what the provider resolved. If the answer is "a listing entry is the whole point", that is
a fine answer and the cost is honest. If it is not, every run between now and the fix
carries a feature that does not work under the policy people actually use.

## Where the evidence is

`IMPLEMENTATION.md`, the section "A linked skill is unreachable under four of the five
policies", and build ticket 07's `## Outcome`.
