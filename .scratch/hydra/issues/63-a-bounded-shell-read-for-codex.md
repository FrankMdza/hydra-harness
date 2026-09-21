# 63 - Does a provider without a read tool get a bounded shell read under read-only and judge?

Type: grilling
Status: resolved
Blocked by:

## Question

Build ticket 48 routed `codex` on 2026-09-20 and found that it has no read tool: every read is a
`Bash` call, and `read-only` and `judge` grant no shell. The human decided the same day that
those two policies refuse a `codex` launch at pre-flight, so `codex` runs under `standard` and
`permissive` only and cannot sit on the `evolve` panel. That is the interim state. This ticket is
the other option, left open on purpose.

1. **Is a bounded shell read a capability or a hole?** The shape would be a closed list of
   commands, `cat`, `head`, `sed -n`, `grep`, `rg`, `ls`, with no redirection, no pipe and no
   path outside the worktree, granted by the `codex` rows alone under `read-only` and `judge`.
   Harness spec 7.4 says the floor declines to interpret shell, and the adversarial review of
   2026-09-14 measured why; a parser that admits `sed -n` has to say what `sed -n 'w /x'` is.
2. **What does a `judge` run on `codex` need to read?** The whole diff arrives in the prompt.
   If a judge never has to open a file, `judge` on `codex` may need no read at all and only
   the shell denial stands in the way, which is a narrower question than item 1.
3. **Who else is in this class?** Map ticket 58 called it "a provider without a hook"; this is
   a provider with a hook and without a file tool. If a fifth provider lands in the same shape,
   the answer here is the rule.

The measurement for items 1 and 2 needs a logged-in `codex`, which is build ticket 48 item 5.

## Answer

Resolved 2026-09-21 with the human. The question dissolved on a fact the ticket did not have:
`judge` grants no tool to any provider, `policies/judge.json` says `"allows": "nothing"`, so no
judge on any provider reads a file, and the diff arrives in the prompt. Item 2 was the whole of
it. A `codex` judge needs no shell read; the only thing keeping it off the panel was the
pre-flight refusal build ticket 48 put in for both narrow policies at once.

Decision: `judge` opens to `codex` and the `evolve` panel becomes four, one per routable
provider, measured with a real pass. `read-only` stays refused on `codex`, because there a
read is the point and `codex` only reads by shell. The bounded command list of item 1 is not
built and not scheduled: the floor declines to interpret shell on purpose, harness spec 7.4,
and a `read-only` `codex` run is not something anyone has asked for. Item 3's rule, for a
fifth provider in this shape: a policy that grants nothing routes it as is; a policy that
grants reads and not shell refuses it at pre-flight. Build ticket 61 carries the code.
