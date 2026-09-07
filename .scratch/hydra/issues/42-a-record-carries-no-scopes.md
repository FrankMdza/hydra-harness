# 42 - A record pins the policy and not the paths it resolved to

Type: grilling
Status: resolved

## Question

Contract 11 derives `permission_decision` at read time by running the run's policy again,
which is what makes an adapter fix reach last month's records. Contract 5.1 keeps
`policy_sha` on every `run_start` so a reader a month later has the rules exactly as they
were.

**The rules are not only the policy files.** A policy grants `file_read` scoped to
`worktree`, `skill_store` or `corpus`, and what those three names resolve to is computed
at compile time from `Layout` and stored nowhere. So the sha pins half of what decided a
call.

**Should a record carry the paths its gate was compiled against?**

Seen for real on 2026-09-06, and it is why the question is here rather than in the fog.
Two branches ran side by side. One pointed `skill_store` at a per-run worktree of the
store; the other resolved it to `~/hydra/skills/`. Replaying the first branch's run on the
second branch turned ten in-scope reads into ten `Finding::GateLeak`, one per file the
curator touched in its own checkout. No policy file had changed, so `policy_sha` did not
move and nothing warned. Both builds were correct. The record could not tell them apart.

Build ticket 12's fix was to make the scope a function of the run id and the policy name,
which both the launch and the replay compute. That works and it is the reason
`Layout::skill_store_scope` exists. It also means every future change to where a scope
points silently rewrites history, and the next one may not be caught by a corpus-wide test
that happened to exist.

## Settle

- **Whether `run_start` gains a `scopes` object.** Three names and three resolved paths,
  beside `policy` and `policy_sha`. It is a contract change and contract 15 versions the
  vocabulary, so it costs a schema bump and a reader that handles its absence on every
  record written before it.
- **Or whether the rule stays derived and gets a guard instead.** A test that says every
  scope is a pure function of `Layout`, the run id and the policy name, so a change that
  breaks the property fails the suite rather than the corpus. Cheaper, and it holds only as
  long as someone keeps the property true.
- **What a replay should do when it cannot know.** Today it silently derives a different
  answer. A finding that says "this record's scopes are not recomputable" is a third
  option, and it is the one that keeps the corpus honest without changing the format.
- **Whether this is the same question for `worktree`.** That scope is
  `~/hydra/worktrees/<run-id>/`, which `IMPLEMENTATION.md` already flags as a location no
  closed ticket names. Moving it would do to every record what the incident above did to
  ten calls.

## What it costs to leave open

Nothing, until someone moves a scope. Then it costs a corpus that reads differently than
it did, with no field saying so, which is the failure contract 11 spends a section
promising cannot happen.

## Where the evidence is

`IMPLEMENTATION.md`, "Rough edges worth knowing", the entry beginning "A record does not
carry the paths its scopes resolved to". `src/paths.rs` and `src/policy/mod.rs` for where
the resolution happens.

## Prepared for the human, 2026-09-06

Evidence, costs and questions. Nothing below decides anything. The four bullets under
`## Settle` are still open, and this is a grilling ticket.

### What was measured

**Every place a scope path is computed.** Launch side:

- `src/run.rs:153` creates the checkout at `Layout::worktree(run_id)`.
- `src/run.rs:640`, `gate_script` is `Layout::gate_script(run_id)`.
- `src/run.rs:650`, the no-repo fallback for `worktree`, `~/hydra/spawn/<run-id>/no-worktree`.
- `src/run.rs:658`, `skill_store` is `Layout::skill_store_scope(run_id, policy)`.
- `src/run.rs:664`, `worktree` is the path `git worktree add` created, or the fallback above.
- `src/run.rs:666`, `corpus` is `Layout::runs()`.
- `src/policy/mod.rs:396` writes all three into the compiled config and
  `src/policy/mod.rs:466` canonicalizes and case-folds each one.

Read side:

- `src/normalize/claude.rs:471-476`, `worktree` is `run_start.worktree.path`, falling
  back to `run_start.cwd`, falling back to `Layout::worktree(run_id)`.
- `src/normalize/claude.rs:477`, `gate_script` is `Layout::gate_script(run_id)`.
- `src/normalize/claude.rs:483`, `skill_store` is `Layout::skill_store_scope(run_id, policy)`.
- `src/normalize/claude.rs:484`, `corpus` is `Layout::runs()`.
- `src/policy/replay.rs:88` then hands that `Spawn` to the same `policy::compile`.

A third caller, `src/ouroboros/curate.rs:93`, checks the store out at
`Layout::skill_store_scope(run_id, "curate")`, so the checkout and the scope cannot drift
apart.

**Three of the four are one function called twice. `worktree` is two functions that agree
by luck.** `skill_store`, `corpus` and `gate_script` each go through one `Layout` method
that both sides call. `worktree` does not. The launch takes the path git made and the
reader takes the path off line 0. On all sixteen records here they match, because
`run_start.worktree.path` is `~/hydra/worktrees/<run-id>` on every one of them. They part
company on a run with no repo, and that is live now (below).

**The flip, reproduced.** A throwaway `Layout`, no provider launched, both gates compiled
from the same `policies/` at `policy_sha`
`536974998142fa4eb0e2ecdc533e2af39eb02d9412913b581b39d788a7ccf811`. Same payload, same
sha, two resolutions of `skill_store`, opposite answers:

```
Read   scope = the checkout : allow  hydra/allow: policy curate allows file_read.
       scope = ~/hydra/skills: deny  hydra/out-of-scope: policy curate allows file_read
                                     inside corpus or skill_store and this call is outside it.
Write  scope = the checkout : allow  hydra/allow: policy curate allows file_write.
       scope = ~/hydra/skills: deny  hydra/out-of-scope: policy curate allows file_write
                                     inside skill_store and this call is outside it.
```

**On the real record it is ten calls, which is the number the ticket claims.** Run
`01a07513-baa3-7cf0-b20a-ded11ede6a38` made 21 tool calls. Ten flip when `skill_store`
moves from the checkout to the store, eleven do not, and all ten flippers are `Read` of a
file under `~/hydra/skill-worktrees/01a07513-.../`. The pass proposed nothing, so no
`Write` is among them.

**The reader then reports two findings per call, not one.** A synthetic curate
record whose reads land outside the resolved scope, with the gate's own `allow` in the
stream, produces both:

```
Disagreement { call_id: "toolu_00", derived: Deny, derived_code: Some("out-of-scope"),
               gate_said: Allow, gate_code: Some("allow") }
GateLeak { call_id: "toolu_00", tool: "Read",
           reason: "hydra/out-of-scope: policy curate allows file_read inside corpus or
                    skill_store and this call is outside it." }
```

`tests/normalize.rs`'s corpus check filters on `GateLeak`, `SubagentLeak` and `Ungated`,
so it caught the leak half. Nothing corpus-wide asserts the absence of a `Disagreement`.

**The cost is not confined to a test.** `src/ouroboros/corpus.rs:390` and `:396` turn both
findings into `signals.disagreements`, which `curate.rs:441` prints into the curator's
brief as "the record and the policy disagree", which `corpus.rs:116` weights at 2 apiece,
and which `corpus.rs:108` reads as a clean-looking failure. Today run `01a07513` is one of
seven learnable records with no disagreements. Under the old resolution it would carry
twenty and sort to the top of the next pass's brief, as a run whose gate leaked twenty
times. It did not leak once.

**The ticket's fourth bullet is wrong, and `IMPLEMENTATION.md` already says so.** Moving
`~/hydra/worktrees/` would not do to every record what the incident did to ten calls,
because `worktree` is the one scope the reader takes off the record
(`src/normalize/claude.rs:476`, and "Two of the three scopes are rebuilt from the layout
and the third from the record"). All sixteen records carry `worktree.path`, so their
`worktree` scope is already pinned. Three things are unpinned rather than one, and the
other two are not on the ticket:

- **A run with no repo.** `run_start.worktree` is null, the launch scoped `worktree` at
  `~/hydra/spawn/<run-id>/no-worktree` and the reader falls back to `cwd`. Measured: the
  gate at launch says `deny hydra/out-of-scope`, the replay says
  `allow hydra/allow: policy standard allows file_write`, for the same `Write` in the
  directory the human was standing in. No record on this machine has that shape yet, so
  this is latent rather than live in the corpus.
- **`gate_script`, which is a function of `~/hydra/spawn/`.** Measured: a `permissive`
  `Write` to `~/hydra/spawn/<run-id>/gate` is `deny hydra/protected-path` with the spawn
  directory where it is, and `allow hydra/allow: policy permissive allows file_write` with
  it moved. Same failure, different path.
- **The five protected configs**, which `src/policy/mod.rs:342` builds from
  `CARGO_MANIFEST_DIR`. An installed binary resolves them somewhere else than this
  checkout does.

**Contract 5.1's reason for not carrying a gate sha is false.** It says "the script is a
pure function of the policy source and the Hydra version, both of which are here".
`policy::compile` takes six more inputs: the policy name, `worktree`, `skill_store`,
`corpus`, `gate_script` and `floor_lifted`. The record carries one of the six
(`worktree`, and null on a no-repo run), and `floor_lifted` has no field at all, which
`IMPLEMENTATION.md` already records. So this ticket and
`IMPLEMENTATION.md`'s "`policy_sha` does not cover the gate script" are two readings of
one wrong sentence.

**Byte cost of a `scopes` object.** Three names and three resolved absolute paths, compact
JSON, real values from run `01a07513`:

```
{"scopes":{"corpus":"/Users/USER/hydra/runs","skill_store":"/Users/USER/hydra/skill-worktrees/01a07513-baa3-7cf0-b20a-ded11ede6a38","worktree":"/Users/USER/hydra/worktrees/01a07513-baa3-7cf0-b20a-ded11ede6a38"}}
```

214 bytes standalone, 213 inserted into line 0 with its comma. Line 0 on the sixteen
records here runs 888 to 5,579 bytes, median 1,759, so it is about 12% of a median line 0
and 1.2% of a median run file, which is 18,278 bytes. Across the whole corpus, 3,408 bytes
against 565,239, or 0.6%.

**The validator does not refuse an unknown field, and nothing else does either.**
`src/record/validate.rs` checks for the presence of the fields in `RUN_START_FIELDS` and
never for the absence of others. Measured on run `01a07513`'s real line 0 with a `scopes`
object spliced in: `ok=true, problems=[]`, and `ok=true` without it. `tests/record_contract.rs`
walks the corpus through the same function and adds no field-set check.
`src/normalize/claude.rs:160-165` reads line 0 by indexing into a `serde_json::Value`, so
a reader that predates the field cannot break on it. Nothing in `src/` uses
`deny_unknown_fields`. Adding the field is therefore compatible in code. Whether it is a
`schema_version` bump is a contract question, and contract 15 does not answer it. It
versions the normalized event vocabulary and says nothing about a `run_start` field.

One thing would break. Putting `"scopes"` in `RUN_START_FIELDS` makes
`every_record_on_this_machine_meets_the_contract` fail on all sixteen records at once.

**What the sixteen records carry.** Thirteen have a policy, `read-only` 8, `standard` 4
and `curate` 1. Three have `policy: null` and predate the gate. All
thirteen carry `policy_sha` `53697499...`, the same one `policies/` hashes to today, so no
record reports `StalePolicy`. All sixteen carry a `worktree` whose `path` is
`~/hydra/worktrees/<run-id>` and a `cwd` equal to it. Every one says
`hydra_version: "0.2.0"`, which is what `Cargo.toml` says today, and nothing in `src/`
reads that field. Re-deriving the three scopes for any of them needs the run id, the
policy name, and `~/hydra` being where it is; two of those are on the record and the third
is `Layout::discover()` reading the passwd entry at read time.

**One thing found on the way that belongs to somebody.** The curate pass's record says
`command: "run"`. Contract 5.1 says "A curator run reads `command: curate`", and
`src/run.rs:712` hard-codes the string. It matters here because `command` is the field a
scope rule could key on instead of the policy name, which is ticket 40's second bullet, and
right now that field says the wrong thing.

### The options, costed

**Bullet 1, whether `run_start` gains a `scopes` object.**

*Store all three.* `src/record/lines.rs` gains an `Option<Scopes>` field and a struct,
about 12 lines. `src/run.rs` composes it, about 10, and the three paths are already in
scope in `write_spawn_config` so either that returns them or `run_start` recomputes them.
`src/normalize/claude.rs:471-484` reads them off line 0 and derives only what is missing,
about 15 lines. Four `RunStart` struct literals gain a line each (`src/run.rs:708`,
`tests/record_contract.rs:32`, `tests/detached.rs:40`, `tests/pty_tee.rs:24`). Six places
build a line 0 as raw JSON, five of them in the tests (`tests/common/mod.rs`,
`cleanup.rs`, `record_contract.rs`, `skills.rs`, `status.rs`) and one in `src/run.rs:773`,
and they gain a field only if the reader prefers the record over the derivation. No test
breaks unless `RUN_START_FIELDS` gains the name, which breaks sixteen at once. Contract 5.1's table gains a row and its `policy_sha` paragraph is
rewritten either way, since it is wrong today. Contract 5.3's rule ("a sha without names
does not say what was selected") is the shape the object should follow. Contract 5.5 and
5.6 need it absent rather than null on a harness run and an interactive one. Contract 11
should say out loud that this is a stored input and not a stored derived value, or a later
reader will count it as a fourth exception. 213 bytes per record, and a read gets cheaper,
since three `Layout` calls become three string reads.

*Store the gate script's sha instead.* About 80 bytes, one field. `src/run.rs:678`
already computes that sha for the hook config. It detects the whole class, `floor_lifted`
included, and recovers nothing. A reader would know its replay is against a different gate
and still could not say what the run's gate answered. It needs a new finding to be worth
anything, which is bullet 3.

*Store `skill_store` alone.* About 90 bytes and one string. It closes the incident that
happened and leaves `corpus`, `gate_script` and the no-repo `worktree` open. It also
pins the answer to ticket 40 into every record written before 40 is decided.

*Ticket 40 interaction.* Storing scopes takes the weight off 40's second bullet, since the
launch would be free to compute the scope however it likes and the record would carry the
answer. It settles none of 40's other three bullets, and it does not need 40 answered
first. Independent, and it makes 40 cheaper.

**Bullet 2, the rule stays derived and gets a guard.**

*A differential test.* Compile through `hydra::run::write_spawn_config` for each of the
five policies and each run shape, rebuild the `Spawn` the way `src/normalize/claude.rs`
does, and assert the two compiled configs' `scopes` objects are byte-identical. About 80
lines in `tests/spawn.rs` or a new file, no source change, no contract change, zero bytes
per record.

*The property the ticket names cannot be written as written.* "Every scope is a pure
function of `Layout`, the run id and the policy name" is already false for `worktree`. The
reader takes it off the record by design, and on a no-repo run the two sides disagree
today, measured above. So the guard has to be a differential test rather than a purity
test, and it fails on the no-repo case the day it is written. That is an argument for it
rather than against it, and it means writing the guard costs a bug fix as well.

*What it does not do.* It makes the next move loud in CI and does nothing for a corpus
already written under the old resolution. The incident happened between two branches, and
a guard on one branch says nothing about the other's records.

**Bullet 3, what a replay should do when it cannot know.**

*A finding saying the scopes are not recomputable.* It has no trigger today. Nothing on
the record disagrees with the derivation, because nothing on the record says anything
about a scope, so there is no signal to key on. This option only exists downstream of
storing something.

*Key it on `hydra_version` instead.* The scope rule lives in the binary, `hydra_version` is
on every record, and nothing reads it. A `Finding::StaleBuild { recorded, current }` in the
shape of the existing `StalePolicy` costs a variant in `src/normalize/mod.rs`, about 15
lines in `src/normalize/claude.rs`, and one arm in `src/ouroboros/corpus.rs:389`'s match, routing it
to `signals.limits` rather than `signals.disagreements`. Getting that arm wrong is what
feeds the curator the noise this ticket is about. Contract 15 already licenses the
argument for `provider_version`, "a normalizer can branch on it". It is coarse in the
direction `StalePolicy` is coarse, firing on every version bump whether or not a scope
moved.

*Do nothing.* Measured cost on the one record that has the shape: ten calls re-derive the
other way, twenty findings, and the run reaches a future curator's brief top-ranked as a
clean-looking failure whose gate leaked. Nothing on the other fifteen.

**Bullet 4, whether this is the same question for `worktree`.**

*It is not, as the ticket states it.* `worktree` is on the record and the reader uses it.
Moving `~/hydra/worktrees/` changes future runs and rewrites nothing.

*The real version of the bullet is three paths.* The no-repo `worktree` fallback, which
diverges today; `gate_script`, which is `~/hydra/spawn/` and flips a `permissive` decision
when moved; and the `config/` prefix under `CARGO_MANIFEST_DIR`. Fixing the first is about
6 lines in `src/normalize/claude.rs` plus a test, and it is a bug rather than a decision if
the answer is "the reader should reproduce the launch". Fixing the second and third means
either recording them or accepting them, which is the same question as bullet 1 with two
more strings on it, around 100 more bytes per record.

*Ticket 40 interaction.* None. 40 is about where the skill store checkout lives; this
bullet is about `~/hydra/worktrees/`, `~/hydra/spawn/` and `config/`.

### Questions for the human

1. **Does harness spec 5's own test apply to the compiled gate?** It says "if reading a
   spawn artifact is needed to interpret a run, the record is not the source of truth and
   is missing a field", and the gate script is a spawn artifact reaped with the worktree
   that holds the only copy of the resolved scopes. Yes means bullet 1 is a contract debt
   and bullets 2 and 3 become follow-ons. No means the gate is exempt because the policy is
   re-derivable, and bullets 2 and 3 are the whole answer. *My opinion: it applies, because
   the paragraph was written about exactly this shape and the gate is the artifact it
   forgot to name.*
2. **If a field lands, is it the three resolved paths or the gate script's sha?** Recover
   against detect. *My opinion: the paths, because a sha tells a reader its answer is
   wrong and leaves it with no way to get the right one.*
3. **If the record carries scopes, does the reader prefer them or only check them?** And
   what happens on the sixteen records that predate the field: derive silently, or derive
   and report a limit. *My opinion: prefer where present, derive where absent, and report a
   limit on the derived path, because that keeps the existing corpus readable instead of
   quietly wrong.*
4. **Is a new `run_start` field a `schema_version` bump?** Contract 15 versions the
   normalized vocabulary and says nothing about `run_start`'s field list, and this will be
   asked again for `floor_lifted` and for anything else. *My opinion: not a bump, and 15
   should say so in a sentence, because the reader indexes into a `Value` and the validator
   ignores unknown fields, both measured.*
5. **If the rule stays derived, is the guard the differential test, and who owns the
   no-repo divergence it will fail on?** It is a live bug either way, and it is smaller than
   this ticket. This one is moot if 1 is yes and 3 says the reader prefers the record.
6. **Does the answer cover `gate_script` and the `config/` prefix, or only the three named
   scopes?** *My opinion: cover `gate_script`, because a moved spawn directory flips a
   `permissive` decision the same way a moved store flipped ten reads, and leaving it out
   means doing this again.*
7. **Contract 5.1's paragraph on why no gate sha is carried is false as written. Does it
   get rewritten here, or in whichever build ticket implements the answer?** It is wrong
   today whatever is decided above.
8. **`command: "run"` on the curate record contradicts contract 5.1. Is that this ticket's,
   ticket 40's, or a build ticket's?** It bears on 40's second bullet, since `command` is
   the field a scope rule would key on if not the policy name.

### What resolving this changes elsewhere

- **Ticket 40** loses weight on its second bullet if scopes go on the record. Its other
  three are untouched. **Ticket 41** is unaffected either way; it is an ordering question.
- **Contract 5.1** loses the last sentence of its `policy_sha` paragraph whatever is
  decided, and its table gains a row if a field lands.
- **Contract 5.3** is the precedent for the object's shape and the source of the argument
  for it. **5.5** and **5.6** need the absent-not-null rule applied to a harness run and an
  interactive one.
- **Contract 11** should say the new field is a stored input rather than a fourth exception
  to never-store, or a later reader will read it as one.
- **Contract 15** gains a sentence on what adding a `run_start` field costs, whichever way
  question 4 goes.
- **Contract 7.7** gains the sentence saying what a re-evaluation resolves its scopes
  against, which is the thing it currently leaves to the reader to find in the code.
- **Harness spec 5**'s "the record therefore answers what did the gate refuse on its own"
  is the claim this ticket tests. **7.5** gains the list of what the script is compiled
  against.
- **`IMPLEMENTATION.md`**: the "Rough edges" entry beginning "A record does not carry the
  paths its scopes resolved to" is replaced, "Reading a record back"'s paragraphs on the
  three scopes and on `policy_sha` not covering the gate script are both amended, and the
  departures list gains or loses the `command: "run"` entry depending on question 8.
- **`tests/normalize.rs`'s corpus-wide leak check and `tests/curate.rs`'s
  `a_curate_runs_own_record_does_not_read_back_as_a_gate_that_leaked`** are the two tests
  that pin today's behaviour, and both are where a new guard belongs.
- **The map** gains one line under `## Decisions so far` and drops 42 from the reopened
  four.

## What ticket 39 left you, added 2026-09-06

**A fourth scope name exists, and it makes your argument easier.** `skill_snapshot` resolves from
`hydra.skills.store_sha`, which sits on line 0 of every record that linked a skill, so a reader a
month later recomputes it without being told. It is the first scope with that property. The
question you settle is whether the other three get it too, or whether the record names its paths.

**Built on 2026-09-06 by build ticket 13, and the property is real.** `Layout::skill_snapshot_scope`
is the whole rule and both sides call it: `src/run.rs` with the sha it pinned, both normalizers with
the sha they read off line 0. `tests/normalize.rs` has the pair that shows it working, a record with
the sha whose read derives allow and the same record without it whose read derives a deny and a
`GateLeak`. So the option you are costing has one worked example in the tree, and its cost was one
function plus one field on the normalizer's `Walk`. A run that linked nothing carries no `skills`
object at all, and the scope then resolves to `<spawn dir>/no-snapshot`, which is the case a rule
recomputed from the record has to answer for and a stored path would not.

**One more thing the build turned up, and it is yours rather than ticket 40's.** A cross-check
between a record's own gate answers and a replay is only sound while `policy_sha` has not moved.
Build ticket 13 was the first policy edit since records existed, and it made one run report a
`Disagreement` that was pure artifact: the old gate refused a `Skill` call correctly and today's
policy allows it. `normalize::policy_moved` now downgrades a disagreement to a limit when the sha
moved. `GateLeak` has the same exposure in the other direction and was left alone, because
suppressing a leak is the unsafe half of the same fix: an edit that narrows a policy would make the
replay deny a call the old gate allowed, and that reads as a leak that never happened. Nothing fires
today because this edit only widened. Whatever you decide about storing scopes, a record that
carried its own resolved paths would not fix this: the paths are not the part that moved, the rules
are.

## What ticket 40 left you, added 2026-09-06

**Your question 8 no longer bears on ticket 40.** You asked whether `command: "run"` on the curate
record is 40's, since `command` is the field a scope rule would key on if not the policy name. 40
removed the keying entirely: `skill_store` and `skill_checkout` are two names now, each resolving one
way under every policy, so `src/paths.rs`'s match on the string `"curate"` goes. No scope keys on a
policy name or on `command` any more. The `command: "run"` contradiction with contract 5.1 is still
real and it is a build ticket's.

**Your second bullet gets easier to write.** The property is now "every scope is a pure function of
`Layout`, the run id, the scope name and, for `skill_snapshot`, the store sha off the record". No
policy name in it. Whether that guard is worth writing instead of a `scopes` object is still yours.

**The vocabulary is five scope names, not three.** `worktree`, `skill_store`, `skill_checkout`,
`skill_snapshot`, `corpus`. Your byte costing counted three at 213 bytes; a curate run would carry
four of the five and the others three, since a run with no checkout has no `skill_checkout`.

**Three of the five are now denied paths as well as scopes.** Ticket 40 put `~/hydra/skills/` and
`~/hydra/skills-snapshots/` into harness spec 7.5's deny list, which is `denied_write_paths` in
`src/policy/mod.rs` rather than anything under `policies/`, so no sha moved. That does not change
what you decide, but a reader of a replayed record now sees a `protected-path` refusal where it used
to see an `out-of-scope` one, and the two carry different reason strings.

## Answer

Resolved 2026-09-06 with the human, two rounds from the brief above. Seven decisions.

**1. Harness spec 5's test applies to the compiled gate.** A record that needs the gate script to
be interpreted is missing a field. The script held the only copy of the paths the scopes resolved
to, and spec 5 now names it.

**2. `run_start` gains a `scopes` object of resolved absolute paths, keyed by scope name.** The
paths, not the gate script's sha: a sha would tell a reader its replay was wrong and leave it no
way to get it right. Shape follows contract 5.3's rule for `skills`, a sha without names does not
say what was selected.

**3. The keys are `worktree`, `skill_store`, `skill_checkout`, `corpus` and `gate_script`.**
`skill_snapshot` stays derived from `skills.store_sha`, which is already on the record. A key the
run did not have is absent, never null. The whole object is absent on a harness run and an
interactive run. The `config/` prefix stays out, because it moves only with a release and
`hydra_version` covers that. `gate_script` is in because a moved spawn directory flips a
`permissive` write from `protected-path` to `allow`, measured.

**4. `floor_lifted` lands on `run_start` as a boolean beside `policy`**, in the same change. It is
the sixth input to `policy::compile`, harness spec 16 already says a per-run human flag lands on
the record, and ticket 03 had left it out only because inventing a field was a contract change.

**5. The reader prefers stored scopes, derives where absent, and reports a limit on the derived
path.** Never a disagreement, because a disagreement reaches the curator's brief weighted as a
clean-looking failure, which is the cost the brief measured. Contract 11 says the two fields are
stored inputs rather than a fourth exception to never-store. This also closes the no-repo
`worktree` divergence, since the reader stops guessing the fallback path.

**6. Neither field is a `schema_version` bump.** Contract 15 gains the rule: an added `run_start`
field a reader may ignore is not a version change, a removed or re-typed one is. Measured basis: the
reader indexes into a `Value`, the validator checks presence and never absence.

**7. Contract 5.1's sentence on why no gate sha is carried was false and is rewritten here**, to
list what the script is compiled from. Harness spec 7.5 carries the same list.

**What moved.** Contract 5.1, 5.5, 5.6, 7.7, 11 and 15; harness spec 5 and 7.5; three paragraphs and
one rough edge in `IMPLEMENTATION.md`. Build ticket 15 carries the code, blocked by 14 because the
key set is 14's. No source changed.

**Left where it was.** The `GateLeak` exposure ticket 13 flagged when `policy_sha` moves is not
this ticket's, as 13 said: the paths are not the part that moves there, the rules are. The
`command: "run"` on the curate record is a build ticket's, per ticket 40's note.
