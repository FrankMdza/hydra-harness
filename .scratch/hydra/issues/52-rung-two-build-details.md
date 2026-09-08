# 52 - Rung two's three open details, before `hydra evolve` gets a build ticket

Type: grilling
Status: open
Blocked by:

## Question

`hydra evolve` is the one command of harness spec 17's eleven that is not built, and rung two
is the only part of the design with no code. `docs/ouroboros-spec.md` section 9 holds three
details that nothing on the map decided, and the map's fog says the first is sharp enough to
ticket today while the other two are build work for a rung that ships later. On 2026-09-08 the
human asked to try everything pending, and this is the last pending thing. Three questions, and
the answers go under `## Answer` as decisions.

1. **The proposer's policy, 9.2.** Reads and writes in its own worktree plus a shell for
   `cargo` is `standard` as the five policies stand. Does the proposer run as `standard`, or
   does a sixth policy arrive with a reason?
2. **Fitting a review prompt, 9.3.** The reference budgets 920K tokens per reviewer and
   injects whole documents. Hydra's reviewers are `judge` runs whose diff rides the prompt.
   What happens when the diff does not fit: a fit note and a path manifest, a refusal to review
   at all, or a cap on the proposer's diff size so the question never arises?
3. **Tolerant parsing, 9.4.** A reviewer's response has to be told apart from a refusal, and
   the failure mode is an unreviewed self-edit recorded as reviewed. Does the reviewer answer
   in a fixed JSON shape the parser rejects on any deviation, with a rejected parse counting as
   a block, or does a looser parse with a sentinel stand in?
