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

## Prepared for the human, 2026-09-06

Evidence and costing, not an answer. Every bullet under `## Settle` still needs a person.

### What was measured

`hydra::policy::write_gate` compiled a real gate for each of the five policies into a
throwaway layout, and each gate answered synthetic `PreToolUse` payloads on
`/usr/bin/python3` 3.9.6, the interpreter the shebang names. The worktree, the snapshot,
the store and the corpus were throwaway directories under
`/private/tmp/claude-501/-Users-USER-hydra/a02d3c0e-4f14-4519-af18-21e7c0db1613/scratchpad/t39/`.
`<worktree>/.claude/skills/hydra-marker` was a real symlink into the snapshot, the shape
`skills::inject` makes. `curate`'s `skill_store` scope came from
`Layout::skill_store_scope`, so it points where a real curate run's gate points. No
provider launched and no quota spent.

| Call | `judge` | `curate` | `read-only` | `standard` | `permissive` |
|---|---|---|---|---|---|
| `Skill` on a linked name | deny | deny | deny | deny | allow |
| `ToolSearch` | deny | deny | deny | deny | allow |
| `Read` through the link | deny | deny | deny | deny | allow |
| `Read` the snapshot path itself | deny | deny | deny | deny | allow |
| `Read` `~/hydra/skills/` | deny | deny | deny | deny | allow |
| `Write` or `Edit` through the link | deny | deny | deny | deny | allow |
| `Bash cat` through the link | deny | deny | deny | **allow** | allow |
| `Read` an ordinary file in the checkout | deny | deny | allow | allow | allow |
| `Read` the curate store checkout | deny | allow | deny | deny | allow |

The reason strings, verbatim:

- `hydra/capability: policy standard does not allow provider_native (Skill).`
- `hydra/capability: policy judge does not allow file_read (Read).`
- `hydra/out-of-scope: policy standard allows file_read inside worktree and this call is outside it.`
- `hydra/out-of-scope: policy read-only allows file_read inside worktree and this call is outside it.`
- `hydra/out-of-scope: policy curate allows file_read inside corpus or skill_store and this call is outside it.`
- `hydra/out-of-scope: policy standard allows file_write inside worktree and this call is outside it.`
- `hydra/allow: policy standard allows shell.`

Seven things the measurement corrects or adds.

**The `Skill` denial is `hydra/capability`, not `hydra/out-of-scope`.** Run
`01a07512-d36e-7651-b8d9-e9e279e5730b`, seq 11, carries the gate's own words:
`hydra/capability: policy standard does not allow provider_native (Skill).`
`hydra/out-of-scope` only fires for a scoped capability whose path fell outside, so it
could never have been the answer here. The reason code is the thing a reader keys on
(`policy::replay::reason_code`), so quoting the wrong one in the ticket that decides this
would have sent someone to the scope machinery for a capability problem.

**`provider_native` covers seventeen tools, not nineteen.** Counted off
`src/policy/table.rs`: 25 rows, of which `Bash`, `Read`, `Write`, `Edit`, `NotebookEdit`,
`WebFetch`, `WebSearch` and `Task` answer to the six named capabilities and the remaining
17 do not. The "nineteen" comes from subtracting six capabilities from twenty-five tool
names. `IMPLEMENTATION.md`, `src/policy/table.rs`'s own doc comment, build ticket 01, and
the test name `the_nineteen_provider_native_tools_belong_to_permissive_alone` all carry
the wrong number.

**"Four of the five" is right about invoking and wrong about reading the body.** Under
`standard` a run reads the linked `SKILL.md` with `Bash cat`, measured allow. `shell` is
unscoped by design (`Capability::is_scoped` excludes it, and 7.4 opens by saying so), and
the gate's shell check refuses only the seven protected paths, which the snapshot is not.
So the body is out of reach under three policies, `judge`, `read-only` and `curate`, and
in reach under `standard` and `permissive`. Only the `Skill` tool is out of reach under
four.

**"Under `permissive` it works" is unmeasured.** No run on this machine has ever had a
`Skill` call allowed. Sixteen records: eight `read-only`, four `standard`, one `curate`,
three with no policy. The gate says the call passes. Whether the provider then loads the
body, and whether that load goes through a tool call the gate would see, is a claim off
reading the gate rather than off a record, which is the position the map's own note warns
about. It is also the fact that decides whether splitting the capability is sufficient on
its own.

**The listing carries no path.** The `skill_listing` attachment on the paid run is 7,950
characters over 28 entries, each `- <name>` or `- <name>: <description>`, and five entries
rendered bare, `hydra-marker` and `hydra-second` among them. No entry carries a file path,
so a model that wants the snapshot has to find it. Under `standard` that costs one
`ls -l .claude/skills/`, or nothing at all, since `cat .claude/skills/<name>/SKILL.md`
resolves the link in the shell and the gate allows it.

**`curate` reads every store skill already, through a different door.** Its `skill_store`
scope resolves to `~/hydra/skill-worktrees/<run-id>/`, its own checkout of the store, and
`ouroboros::curate::brief` names that path in the task string. Reading it is allowed,
measured. So rung one reaches the same bytes the links point at without following a link,
and what it actually loses is the ability to invoke a skill. That weakens the fourth
`## Settle` bullet considerably.

**No curate run has been launched since injection and the curator met.** Both records
carrying a `skills` object were written at 23:56 and 23:57 local on 09-05, before the merge
at 00:21 on 09-06. The one curate record has a worktree and no `skills` object, because it
came off the branch that had no injection. "A curate run is linked skills it cannot
follow" is derived from the merged code and has never been seen in a record.

Two more findings, neither of them this ticket's business, both worth a reader knowing.

`claude` 2.1.263's `init` frame lists 29 tools and the table has 25. `TaskCreate`,
`TaskGet`, `TaskList` and `TaskUpdate` are new since the 2.1.261 reading, and each denies
with `hydra/unknown-tool` under every policy, `permissive` included. That is 7.1 working
as specified, and it is also the measurement that says how stable the `provider_native`
bucket is: four names in two patch releases.

The store has been empty since `bc3868f`, and `inject_skills` runs the 1.8 second listing
probe on every run with a checkout regardless of how many links it made. So today every
recorded run pays 1.8 seconds to ask the provider about zero linked skills.

### The options, costed

Every line count below is off the call sites, not off a written patch.

#### Whether `provider_native` should be split

**Leave it.** Zero lines. A `standard` or `read-only` run that reaches for a skill burns a
tool call on the refusal and derives `gated`; run `01a07512` spent $0.033 finding that
out. The body stays reachable under `standard` through `Bash`, so the capability boundary
and the practical boundary already disagree.

**Grant `provider_native` to `standard`.** One line in `policies/standard.json`. It also
grants `CronCreate`, `CronDelete`, `CronList`, `DesignSync`, `EnterWorktree`,
`ExitWorktree`, `ListAgents`, `Monitor`, `PushNotification`, `RemoteTrigger`,
`ReportFindings`, `ScheduleWakeup`, `SendMessage`, `TaskOutput`, `TaskStop` and
`ToolSearch`. `CronCreate`, `ScheduleWakeup` and `RemoteTrigger` schedule work that
outlives the run and gets no run record, which is the objection 7.6 makes to in-process
subagents, arriving through a different door.

**One new capability for `Skill` alone.** `src/policy/mod.rs` gains an enum arm and an
`as_str` row, 3 lines; `src/policy/table.rs` changes one row; each policy that grants it
gains one line, and `permissive.json` must gain it too or
`permissive_contains_all_five_and_judge_contains_only_itself` fails. `policies/README.md`
gains a name. `tests/gate.rs`'s `the_nineteen_provider_native_tools_belong_to_permissive_alone`
asserts `Skill` is denied under `standard` and breaks; a replacement is about 20 lines.
Roughly 15 lines of source and 25 of test. Harness spec 7.1 gains a sentence and 7.2's
"what it allows" column changes for each policy that gains it. The record contract needs
nothing: 7.6 says `tool_call` carries "the capability Hydra's own table maps the name to"
and enumerates none, so a new capability name costs no schema bump.

**A trap inside that option.** Grant it to `read-only` and not to `standard` and the two
stop being comparable, so a `standard` parent can no longer delegate a `read-only` child.
`Policy::contains` is set containment over capability-scope pairs and `standard` contains
`read-only` today only because `read-only`'s single grant is a subset. Whatever gets this
capability, `standard` has to get it too, or 7.3 quietly loses a delegation that works.

**What every one of these costs at run time.** Any edit under `policies/` moves
`policy_sha`. Thirteen of the sixteen records on this machine carry today's sha, and each
would then normalize with `Finding::StalePolicy`, which reaches rung one as "the decisions
answer for another policy". That number only grows.

**The measurement that should come first.** One `permissive` run on haiku with a marker
skill whose body carries a sentinel, then look for the sentinel in the transcript and for
any `Read` the gate was asked about. It answers whether granting the tool delivers the
body, which is what decides whether this bullet alone is enough. About $0.03 and one
minute, against the run that already exists as a template.

#### Whether the snapshot is inside a run's read scope

**Leave it.** Zero lines. Under `standard` the shell reaches the body anyway, so the
practical loss falls on `read-only` and `curate`.

**A `skill_snapshot` scope, resolved per run from the pinned sha.** `Scope` gains an arm
and an `as_str` row; `Spawn` gains a field, which touches all four `Spawn` literals
(`src/run.rs`, `src/policy/mod.rs` tests, `src/normalize/claude.rs`, `tests/gate.rs`);
`compile`'s `scopes` map gains one entry; `write_spawn_config` gains a parameter, which
touches 11 call sites, one in `src/run.rs` and ten across `tests/spawn.rs` and
`tests/curate.rs`; `inject_skills` has to return the snapshot path it already computes.
`src/normalize/claude.rs` recomputes the same path at read time from
`hydra.skills.store_sha`, about 8 lines, and `src/live.rs` already reads that field so the
rule exists. Three policy files gain a line if `read-only`, `standard` and `curate` all
get it. Roughly 45 lines of source and 60 of test. The ordering is already right:
`inject_skills` runs at `src/run.rs:181` and the gate is compiled at 222, so nothing has
to move.

The property worth noticing: the snapshot path is a pure function of a field the record
already carries, so this is the one scope option that does not make ticket 42 worse.

**Copy instead of link.** About 10 lines in `skills::inject`, no policy edit, no
`policy_sha` move, and the `worktree` scope covers the files for free. Measured cost: a
copied `SKILL.md` under `<worktree>/.claude/skills/` is writable by the run,
`hydra/allow: policy standard allows file_write`, where a write through the link is
refused today with `hydra/out-of-scope`. So a run could edit its own instructions
mid-flight. Harness spec 8 currently gets that guarantee by construction rather than by
rule, and copying would spend it. Disk is the smaller cost.

**Widen `worktree` to cover the injected root's link targets.** Cheapest in lines and most
expensive in what it gives up. The gate sees one path and resolves it; distinguishing
"reached through the run's own injected root" from "reached through any symlink" means
judging the call by both the pre-resolution and the post-resolution path. That turns
`a_symlink_out_of_the_worktree_does_not_widen_the_scope` from a rule into a rule with an
exception the compiled script has to carry correctly on every call.

#### What harness spec 6.4 should say either way

Wherever this lands, the sentence goes right after "The links point into a snapshot, not
into the store", which is where a reader who has just learned about the snapshot asks
whether a run may follow one.

If a listing entry is the whole delivery: "A run is not given a way to open what the links
point at. Under every policy but `permissive` the snapshot sits outside the run's read
scope and the `Skill` tool is denied, so injection delivers a name and a description into
the provider's listing and the model decides what to do with them."

If the snapshot becomes readable: "The run's own snapshot is a read scope of its own,
named `skill_snapshot`, so a run may read what it was linked and may write nowhere near
it. It is resolved from the sha on the run's own record, which is what lets contract 7.7
re-derive the same answer at read time."

If the tool is granted: "`Skill` is its own capability rather than one of the seventeen
tools that answer to `provider_native`, because a run that is linked a skill and cannot
invoke it was given a listing entry rather than a skill."

Section 7.2's table and `policies/README.md`'s capability and scope lists move with any of
the three. Section 8 and `config/preamble.md` move only under the first, since both tell
every run that "the skills linked at the pinned sha" are its instructions, and under
`standard` today the run is told that and then refused the tool. Run `01a07512` shows the
model working it out: "The policy compiled for this run does not allow the Skill tool. I
cannot use hydra-marker."

#### Whether `curate` is a special case

The premise is weaker than the ticket assumed. A curate pass reads every store skill
through its own checkout, at a path its brief names, so it is not shut out of skill
content. What it cannot do is invoke one, and that is the first bullet rather than this
one. If the answer to the first bullet gives `standard` a skill capability, giving it to
`curate` costs one line and no new mechanism. If the answer is "leave it", `curate` needs
nothing, and this bullet closes with the finding rather than with a change.

### Questions for the human

Ordered so an early answer moots later ones.

1. **Is a linked skill meant to be usable by the run, or is a name and a description in
   the listing the whole delivery?** Pick the second and questions 2 to 5 are moot: the
   documents move, no code does, and 6.4 gains the first candidate sentence above.
   My read: it is meant to be usable, because harness spec 8 and the preamble both call
   the linked skills a run's instructions, and an instruction the run cannot open is a
   name.
2. **Do you want the `permissive` sentinel run measured before answering 3?** One haiku
   run, about $0.03. It says whether granting the tool delivers the body or whether a
   read scope is needed as well.
   My read: yes, because every option under question 3 is priced against an unmeasured
   assumption without it.
3. **Which of these three: `Skill` gets its own capability, `standard` gets
   `provider_native` whole, or nothing moves?** If a capability is added, name which
   policies grant it, remembering that `read-only` cannot have it unless `standard` does.
   My read: its own capability, granted to `read-only`, `standard`, `curate` and
   `permissive`, because the alternative hands an ordinary coding run `ScheduleWakeup`
   and `CronCreate` to buy one tool.
4. **If a run should be able to read a skill body as a file, does the snapshot become a
   named scope, or do the links become copies?** Pick one.
   My read: the named scope, because the copy makes a run's instructions writable by the
   run under `standard`, which is measured above and is a worse trade than 45 lines.
5. **Is `curate` a special case, now that it reads the whole store through its own
   checkout?** Yes or no.
6. **Does the preamble keep its third instruction as written?** It says the skills linked
   at the pinned sha instruct the run. If the answer to question 1 is "a listing entry",
   the preamble is promising more than the gate delivers, and it is protected config #4,
   so only you can change it.

### What resolving this changes elsewhere

**Map tickets.** Ticket 40 shares the scope machinery: if a `skill_snapshot` scope is
added, both tickets are editing `Scope` and `Spawn` in the same week and should be
sequenced. Ticket 42 gains a fourth scope name to argue about, and the argument gets
easier, since the snapshot path is recomputable from `skills.store_sha` on line 0. Ticket
41 is untouched.

**Documents.** `docs/harness-spec.md` 6.4 in every outcome; 7.1 and 7.2 if a capability is
added; section 8 and `config/preamble.md` if the answer narrows what injection delivers.
`docs/ouroboros-spec.md` 3.6 only if question 5 answers yes. `docs/run-record-contract.md`
needs nothing, since 7.6 names no capability list.

**Build tickets.** Build 07's `## Outcome` and `IMPLEMENTATION.md`'s section "A linked
skill is unreachable under four of the five policies" both need the corrections above
whatever is decided: the reason code, the count of seventeen, the shell route under
`standard`, and that `permissive` is inferred rather than observed. Build tickets 09 and
10 build the `cursor-agent` and `agy` capability tables, so a split decided now sets the
shape for two more tables; decided later, it is three edits instead of one.
