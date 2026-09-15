# 56 - The shell check only saw whole tokens

Type: build
Status: resolved
Blocked by:

## What

Harness spec 7.5 says a command that names a denied path is refused whole.
`protected_path_named_by` tokenized with `shlex` and resolved each token, so a denied path
spelled in full inside a larger token was invisible: an interpreter's `-c` argument is one
token. Verified against the real compiled gate of run `01a0a07b-e8a6`: `cat <corpus>/x.jsonl`
refused, `sh -c 'cat <corpus>/x.jsonl'` allowed. The corpus, the store, the snapshots, a
sibling's spawn directory and the parent repository's `.git` were all reachable that way.

Not the documented limit. That one is a path the command never writes down, assembled from a
variable; here the path is written down in full and the parser missed it.

## Outcome

A raw pass runs before the token pass: the command, case-folded, is searched for each denied
path as a substring. `denied_spellings` in `src/policy/mod.rs` emits two forms per path, the
resolved one and the literal one, because on macOS `/var/folders/...` and the
`/private/var/folders/...` it resolves to are both things a command can write down and a
substring search resolves nothing. A gate compiled before the key falls back to one spelling
each.

**The residual, and why it is one.** A denied path with one of the spawn's own paths under it
cannot go on the raw list: `~/hydra/worktrees` is denied while the run's own worktree sits
inside it, and a substring search cannot say "this one and not that one". A sibling run's
worktree spelled inside a larger token still reaches the token pass and still walks through.
The corpus, the store, the snapshots, the spawn directory and the per-run config files carry no
own path and are covered. `gate.py` says this where the code is rather than in a ticket nobody
will read.

Two tests in `tests/gate.rs`: five shapes of the bypass refused, five ordinary commands still
allowed, including the run's own worktree and a skill read. Gate suite 67 of 67.

**What it broke, and what that became.** The corpus walk went red on a record from 2026-09-08
whose command the gate of that day allowed. Third time a change to the gate's own code turned
older records into leaks, so build ticket 49 was done the same evening.
