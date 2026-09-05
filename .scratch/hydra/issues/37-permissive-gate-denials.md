# 37 - Which gate denials hold under `permissive`?

Type: grilling
Status: resolved
Blocked by:

## Question

The seam check found the Ouroboros spec relying on four run-time guarantees the harness spec does
not make. Both documents are internally consistent and they disagree with each other.

**What Ouroboros claims.** Section 2.2 lists five things neither rung may ever do, then says: "The
first three are enforced by the gate on every run under every policy, `permissive` included." Those
three are calling `hydra prune` or `hydra reap`, calling `hydra export`, and writing anywhere under
`~/hydra/runs/`. It then says the fourth, writing any of the five protected configs, "is enforced
twice: the gate denies the writes at run time, and a rung-two patch touching one of those files is
refused before review."

**What the harness spec says.** Section 7.5 names exactly one universal exception: "The gate denies
writes to the delegation policy directory and to the generated script under every policy,
`permissive` and `curate` included. That is the single place `permissive` stops being permissive."
Section 15.2's reconciled table agrees and is narrower still. Of eleven protected paths, entries 5
and 9 carry "Denied at run time: yes, under every policy" and the other nine carry "no".
`~/hydra/runs/` is not in that table at all, and section 7.2 defines `permissive` as "everything".

So on the harness spec as written, a `permissive` run may call `hydra export`, may call `hydra
prune`, and may write into `~/hydra/runs/`. Three of the five protected configs get no run-time
denial either: the environment allowlist, the export allowlist, the instruction preamble and the
canonical MCP document are admission-refused only.

**The harness spec asserts the same conclusions elsewhere and names no mechanism.** Section 10 says
"Ouroboros cannot call either at either rung" of prune and reap. Section 14 says of export, "No run,
no curator and no delegation policy can reach it." Section 17 repeats it. None of the three says
what enforces it, and none of them is in 15.2's table, so a builder implementing the table builds a
gate that permits all three.

## What to decide

Which document moves. Two shapes, and they are not equally cheap.

**Widen the gate.** Three more universal denials: the `~/hydra/runs/` path, the `hydra` binary as a
`shell` target for the `export`, `prune` and `reap` subcommands, and the remaining protected config
paths. They go in 15.2's run-time column and in section 7.5 beside the two that are already there.
This is the shape that makes `evidence_is_read_only` in the Ouroboros checklist check something real.
The cost is that "`permissive` allows everything" stops being true in five more places, and the
denial on a subcommand of `hydra` is a pattern-floor style match on argv, which section 7.4 already
admits is catchable by spelling.

**Narrow the claim.** Ouroboros 2.2 stops saying the gate enforces the first three, and says what
actually does. For prune and reap that may be nothing but the fact that no Ouroboros code path calls
them, which is a claim about Hydra's own source rather than about a run, and belongs to the
`evidence_is_read_only` checklist item rather than to the gate. The cost is that a `permissive`
proposer run, which is a model editing Rust, has nothing stopping it running `hydra export`.

Note that the two rungs are differently exposed. Rung one runs under `curate` and rung two's proposer
under `standard` per the Ouroboros spec's open question 9.2, and neither is `permissive`. So the hole
is not one either rung walks into today. It is a hole in what the checklist item can promise, and the
whole point of that item is to keep the property true under a self-edit.

## Also here

**Read blocking has one mechanism named in one document.** Ouroboros 4.3 says the gate keeps the
proposer out of `~/.claude`, `~/.cursor`, `~/.agents` and `~/hydra/runs`, and that "Read blocking is
confirmed on all three providers, `beforeReadFile` on Cursor and `PreToolUse` on the other two."
`beforeReadFile` appears nowhere in the harness spec, whose section 6.6 describes the gate as a
`PreToolUse` hook and gives Cursor one config file. The harness owns the gate, so the harness spec is
where the second hook event belongs. Same shape as the finding above: Ouroboros naming a mechanism
its sibling does not describe.

## The five claims are not one kind of thing, 2026-09-05

Written after filing, and it changes the shape of the answer. "Widen the gate" and "narrow the claim"
read as two sides of one decision above. They are not, because the five things Ouroboros 2.2 claims
split by mechanism, and only some of them are a gate's job at all.

**Two are path blocks and the gate can deliver them outright.** Writing under `~/hydra/runs/`, and
writing the four protected configs that today carry only an admission refusal. Ticket 26 measured a
hook blocking a file write before it reaches disk on all three providers, probed on Cursor with the
path and the content in hand and no file afterwards, and `agy` reports the attempted arguments in its
own error string. So the hook sees the path and can refuse on it. These belong in 15.2's run-time
column beside entries 5 and 9, and they cost a row each.

**Two are shell-argv blocks and the gate cannot deliver them as absolutes.** `hydra prune`, `hydra
reap` and `hydra export` are a binary a run invokes through `shell`. That is the pattern floor's
shape, and the harness spec's own section 7.4 already says what the floor is worth: "It is a floor
and never a gate, and the spec says so out loud... it inherits that shape's failure mode: silent
permission on the spelling nobody wrote down." `./target/debug/hydra export`, an alias, or a
`subprocess` call from a Python one-liner all walk past an argv match. So a gate rule here is worth
having and is not worth stating as a guarantee, and Ouroboros 2.2 states it as one.

**So the likely answer is that both documents move and the claims split by mechanism**, rather than
one document winning. The runs directory and the remaining protected configs become real run-time
denials under every policy. The three `hydra` subcommands stop being a gate claim and become what
they already are in the checklist: `evidence_is_read_only` bounds Hydra's own source, so no Ouroboros
code path calls them, which is a property of the binary rather than of a run. That split is cleaner
than either option above, because the gate bounds what a run may do and the checklist bounds what the
source may become, and the current text has the gate carrying both.

Worth grilling rather than adopting. The counter is that a floor nobody can state as a guarantee is
still the only thing standing between a `permissive` proposer and `hydra export`, and dropping it
from the gate on the grounds that it is imperfect is how a partial defence becomes no defence.

## Answer, 2026-09-05

**Both documents move, and the five claims split by mechanism.** Neither "widen the gate" nor "narrow
the claim" was right on its own, because the five things Ouroboros 2.2 forbids are three different
kinds of thing and only two of them are a gate's job.

**The gate's universal denial set is now seven paths**, in `docs/harness-spec.md` section 7.5: the
five protected configs, the generated gate script, and everything under `~/hydra/runs/`. All seven
are path blocks, and ticket 26 already measured a hook refusing a file write before it reaches disk on
all three providers, probed on Cursor with the path and content in hand, so the gate delivers them
outright rather than aspirationally. Section 7.5 gives each of the three groups its own reason: the
policy directory and the script close the induction, the other four configs were protected against a
rung-two patch and against nothing else, and the runs directory is the evidence a learning loop must
not be able to edit.

**`hydra prune`, `reap` and `export` are not gate claims and both documents now say so.** They are a
binary reached through `shell`, so any rule against them is an argv match with the pattern floor's
own failure mode, which section 7.4 states in the harness spec's own words: silent permission on the
spelling nobody wrote down. `./target/debug/hydra export` walks past it. What holds them is that no
Ouroboros code path calls them, a property of Hydra's source rather than of a run, and
`evidence_is_read_only` is the checklist item that keeps it true under a self-edit.

That item is now load-bearing rather than belt-and-braces, so it gained a clause telling a reviewer
not to defer to the gate on those three. A critical item that is the only mechanism has to say so, or
a reviewer reads a redundant check and treats it as advisory.

**Stating the weaker mechanism was the point.** A floor described as a guarantee is section 2.1's
last rule with the roles swapped, which is the failure this whole design is named after.

### What changed

- **Harness 7.5** rewritten: seven paths, three groups, each with its reason, plus an explicit
  paragraph on what the gate does not do and why.
- **Harness 15.1** gains the run-time denial for all five configs, and the sentence that admission
  refusal and the gate answer different questions.
- **Harness 15.2** grows to twelve. Entries 3, 8, 10 and 11 move from "no" to "yes, under every
  policy". Entry 12 is the run records directory, admission not applicable. The heading changes from
  "eleven, not nine" to twelve, keeping the arithmetic correction and the new decision separate.
- **Harness 6.6** gains `beforeReadFile` on Cursor. Reads are a capability, `read-only` and `standard`
  both scope them, and Cursor routes file reads through a hook event `preToolUse` does not cover.
  Ouroboros 4.3 relied on this and the harness spec described a `PreToolUse`-only gate.
- **Harness 8** updated where it named the two old paths in the worked example.
- **Ouroboros 2.2** gains an "Enforced by" column and three paragraphs on why the rows do not share a
  mechanism.
- **Ouroboros 4.4** now says ten of the twelve are what admission refuses, since two entries carry a
  run-time denial and no admission refusal.
- **Ouroboros 5.3** amends `evidence_is_read_only`, and **5.4** gains it as a fifth amendment.

### One thing found while fixing

**Section 5.4 was headed "Three items amended" and listed four.** In a document whose section 5 opens
by complaining that every count in its source tickets is wrong by one in the same direction. Now five,
counted from the paragraphs, with the miscount named rather than quietly corrected.
