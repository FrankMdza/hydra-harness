# 55 - `Ungated` and `GateLeak` mean different things on each provider

Type: build
Status: open
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
