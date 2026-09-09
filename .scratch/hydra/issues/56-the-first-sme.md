# 56 - The first SME: a skill in the store, launched and measured

Type: prototype
Status: resolved
Blocked by:

## Question

Ticket 54 found the training channel built and empty. This ticket puts one skill through it
and reports what actually arrives at the model, so ticket 55 argues from a measurement rather
than from the code's promise.

Not a decision. If it finds one, it says so and stops.

## Outcome

Measured 2026-09-09.

**The channel delivers, end to end.** `~/hydra/skills/hydra-self-edit/` was written with a
`SKILL.md` and a `hydra.toml` carrying `origin = "human"` and `pinned = false`. Its content is
the four rules the panel has taught across passes 6 and 7, each naming the pass that taught it.
A probe run of `claude` on haiku under `standard`, run `01a086aa-b63b`, recorded:

- `skills.store_sha` moved from the empty repo to `c44b43d`, so the launch committed the dirty
  store and pinned it, as harness spec 6.4 says it should.
- `skills.linked: ["hydra-self-edit"]`, the first non-empty `linked` any run on this machine
  has carried.
- `<worktree>/.claude/skills/hydra-self-edit` is a symlink into
  `~/hydra/skills-snapshots/c44b43d.../hydra-self-edit`, and the `SKILL.md` reads back through
  it intact.
- `skills.resolved` carries `hydra-self-edit` fourth of 27, so `claude`'s own discovery found
  it and offered it to the model.

Every wire from the store to the model's available-skills list works, unmodified, first try.

**The model still did not open it, and the reason is worth writing down.** The probe asked the
run to report what the skill says about a red test. It went looking for
`/Users/USER/.claude/skills/hydra-self-edit/SKILL.md`, the human's own skill root, got the
refusal `standard` owes that path, and answered that the skill "isn't available in my context
window". It never tried the Skill tool and never tried the snapshot, both of which `standard`
grants: the policy carries `skill_invoke` and a `file_read` scoped to `skill_snapshot`.

Half of that is the probe's fault. The skill's description triggers on editing Hydra's Rust,
and the probe asked a question *about* the skill, which matches no trigger. So the model was
not auto-loaded with it and had to go hunting. The other half is a real caveat for training: a
skill that reaches the listing has still not reached the model, and on a weak model the gap
between those two is where a training signal disappears.

**What this does not answer.** Whether the skill changes behaviour on a task that does match
its trigger. That needs an evolve pass with the skill in the store, which is the experiment
ticket 55's question 4 needs and which passes 7 and 8 have already set up: same target, same
proposer model, one variable moved at a time.

**No decision was found.** The store's design held everywhere it was touched.
