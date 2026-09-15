# 55 - `Ungated` and `GateLeak` mean different things on each provider

Type: build
Status: resolved
Blocked by:

## What

The three readers apply different bars to the same question, and nothing in the contract says
so, which makes a count compared across providers silently wrong.

| | `claude` | `cursor-agent` | `agy` |
|---|---|---|---|
| Tool-table check | every name in `system/init.tools` | only names actually called, since Cursor's init carries no tool list | every name in `init.tools` |
| `Ungated` | every allowed call with no Hydra voice | only where another hook refused | same as Cursor |
| Bar for a leak | the call's fate is known | result present, `no-path` excused | result status `Ok` only |

So a renamed Cursor tool stays invisible until something calls it, and `agy`'s leak count is a
floor where `claude`'s is not. Either the findings mean one thing everywhere, or contract 7.7
says what each means per provider and the reader reports which bar it used.

Also in scope, one file's own inconsistency: `claude.rs` recognises Hydra's voice through
`hydra_voice`, which validates against `GATE_CODES`, on the stream path, and accepts any text
starting `hydra/` on the harvest path. `GATE_CODES` is missing `unhooked` and
`capped-argument`, which the other two readers write.

Found by the conformance audit.

## Outcome

**The second arm, and it is the right one.** The ticket offered "the findings mean one thing
everywhere" or "contract 7.7 says what each means per provider and the reader reports which bar it
used". None of the three differences is a choice: `cursor-agent`'s init frame carries no tool list,
so the tool-table check has only called names to read, and `agy` reports a leak on the one result
status that is evidence because its other statuses are not. Making the three agree would mean
lowering `claude` to the floor the other two sit at, which throws away real findings to make a
column line up.

So contract 7.7 carries the table of three axes, and it says the sentence that matters: a reader
comparing counts across providers is comparing a ceiling against two floors. `note_finding_bar` in
`src/normalize/mod.rs` puts that on the record itself, one `Limit` per `cursor-agent` and `agy`
record naming which bar it held. Nothing is emitted on `claude`, because it is the reference and a
line saying "this count is complete" on 79 records buys nothing.

**The file's own inconsistency was a real bug, not a style difference.** `claude.rs`'s harvest path
tested `output.starts_with("hydra/")` while its stream path went through `hydra_voice`, which has
validated the code against `GATE_CODES` since ticket 01. So the same reader disagreed with itself
about what counts as Hydra speaking, and a tool error reading `hydra/runs/x.jsonl: permission
denied`, which is a guard quoting a path, was recorded as a gate that denied the call. Both paths go
through `hydra_voice` now.

**`unhooked` and `capped-argument` do not belong on `GATE_CODES` and the ticket was wrong about
that.** They are written by the reader into a decision's `reason`, not by the gate into a refusal.
`the_known_codes_are_exactly_the_ones_the_gate_and_its_wrappers_write` asserts that list equals what
`gate.py` and the three hook wrappers write, and it passes; adding two reader-written codes would
break it and would make a reader's own sentence readable as the gate's. The contract says the two
namespaces are separate.

`tests/common/mod.rs` gained `named_its_bar` beside `rebuilt_the_scopes`, so a test asking whether a
call raised anything still gets the rest of the list. Suites: normalize 79 of 79, outcome 40 of 40.
