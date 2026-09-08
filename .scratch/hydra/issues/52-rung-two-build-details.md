# 52 - Rung two's three open details, before `hydra evolve` gets a build ticket

Type: grilling
Status: resolved
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

## Answer

Decided with the human on 2026-09-08, in one round.

1. **The proposer runs as `standard`.** Nothing `standard` grants is more or less than a
   proposer needs, and a sixth policy arrives the day something `standard` allows turns out
   to matter here, with that reason written. The read-only brief of spec 4.3 stays a
   convention the supervisor keeps when it composes the prompt.
2. **A diff that would not fit never reaches a reviewer.** The proposer's patch has a size cap,
   a line count named in the build ticket and adjustable in one place; a patch over it is
   recorded as rejected for size before review, and the proposer is told to split it. No fit
   note, no path manifest, no reviewing part of a patch blind: a `judge` always sees the whole
   diff or none.
3. **The reviewer answers in one fixed JSON shape and any deviation is a block.** A closed
   schema, parsed whole; a response that does not parse is recorded as a block with the parse
   error, never as a pass and never retried silently. Silence is not consent, ticket 17's rule,
   now spelled for the parser. No sentinel, no tolerant match.

Build ticket 36 carries `hydra evolve` with all three. `docs/ouroboros-spec.md` section 9 says
each is closed and points here.
