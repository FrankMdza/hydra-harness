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

**Answered the same day, and the answer is about the model rather than about Hydra.** The
probe was rerun on a task that does match the trigger: edit `reap`'s doc comment in the
worktree, change no behaviour and no test, then say which guidance you consulted. Twice, same
policy, same store sha, same worktree shape, one variable.

| Run | Model | Tool calls | Skill invoked | What it said it consulted | Tokens |
|---|---|---|---|---|---|
| 01a086b2-5102 | haiku | Read, Edit, Bash | no | `CLAUDE.md` | 146,315 |
| 01a086b3-ca7c | sonnet | **Skill**, Read, Edit, Bash | yes, first call | `hydra-self-edit`, naming two of its rules | 190,528 |

Sonnet's first tool call is `Skill { skill: "hydra-self-edit" }`, before it opens a single file.
haiku never reaches for it, on a task whose first sentence is the skill's trigger, and when
asked point blank it names the repo's `CLAUDE.md` instead.

**So a skill in the store is not training on its own.** The channel delivers the skill to the
model's menu and a weak model does not order from it. Whether a specialist can be trained
through the store is a property of the model the specialist runs on, and every rung-two pass so
far has run its proposer on haiku.

**The contrast with the brief is the useful half.** The same knowledge, written into the
proposer's prompt as a clause in `brief()`, changed haiku's behaviour on pass 8: it obeyed both
contracts the task named. Written into a linked skill, it did not reach haiku at all. Prompt
text works on the weak model and the store does not, which is exactly backwards from what
training wants, because the prompt is frozen in Rust and the store is the versioned, proposable,
mergeable thing.

**No decision was found.** The store's design held everywhere it was touched. What the
measurement changes is ticket 55's question 1: an SME may be a skill, and it is only a skill on
a model that opens skills.
