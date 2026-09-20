# 63 - Does a provider without a read tool get a bounded shell read under read-only and judge?

Type: grilling
Status: open
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
