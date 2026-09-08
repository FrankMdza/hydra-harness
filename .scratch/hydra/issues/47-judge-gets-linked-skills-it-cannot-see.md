# 47 - A `judge` run is linked skills it cannot list, read or invoke

Type: grilling
Status: resolved
Blocked by:

## Question

Harness spec 6.4 links the pinned snapshot into every run that has a checkout, `judge` included.
Build ticket 16 measured on `claude` 2.1.263, 2026-09-06, that `--restricted` drops every skill the
filesystem offers along with the servers settings define: a scratch `HOME` with one user-scope
skill and a cwd with one project-scope skill gave a `skill_listing` of 14 names with no flags and
the binary's own 12 under the flag. So a `judge` run cannot list the skills Hydra linked, `judge`
grants no `skill_invoke` so it could not invoke them, and it grants no `file_read` so it could not
read them either. The links are there and nothing can reach them.

The probe now answers `skills.resolved: null` on `judge`, which is contract 5.3's stance on an
unanswered listing, and the collision check skips its third test on a null. That is what keeps a
`judge` launch on a machine with a non-empty store from being refused with a remediation telling
the human to rename a skill. It is a workaround for a question nobody asked: **should a `judge` run
be linked skills at all?**

## The two shapes

**Stop injecting on `judge`.** 6.4 gains an exception, `skills` is absent on a `judge` record the
way it is absent on a run with no checkout, and the probe under `judge` has nothing to answer for.
Costs: 6.4's "every run" loses a word, and a policy name reaches the injection code for the first
time. `curate` already reaches `write_spawn_config` by name, so there is a precedent.

**Keep injecting and accept the null.** Nothing changes. Costs: a record that says `skills.store_sha`
and `resolved: null` for a run that could touch none of it, and a reader that cannot tell that null
from a crashed probe without the stderr line ticket 16 added.

## Where the evidence is

Build ticket 16's `## Outcome`, "What the ticket assumed that turned out false". `src/provider/claude.rs`,
`resolve_in`, the branch that answers `None` when the probe ran restricted. `IMPLEMENTATION.md`,
the rewritten `--restricted` section. Harness spec 6.4 and 6.7.

## Answer

Resolved 2026-09-07 with the human. **Stop injecting on `judge`.** The policy allows nothing, the
diff a judge reads arrives in its prompt, and `skill_invoke` and `file_read` are both absent, so
the links were never reachable and a record that names a `store_sha` for such a run says
something false. 6.4 gains the exception, `skills` is absent on a `judge` record the way it is on
a run with no checkout, and the probe under `judge` has nothing to answer for. The human's
question, whether `judge` loses anything, has the answer no: a judge that needs guidance gets it
in the task string, which is where the review checklist of ouroboros spec 5 belongs the day
something reads it. Build ticket 25.
