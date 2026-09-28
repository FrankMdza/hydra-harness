# 139 - A read-only seat reads with a bounded shell under a write-denying seatbelt

Type: build
Status: open
Blocked by:

## What

Map 82, answer 3, which amends map 63. A seat that must read files it was not handed runs under
`read-only` (`policies/read-only.json`: file reads in the worktree and skill snapshot, skill
invoke, no shell). It gains a shell limited to read-only commands, and its seatbelt denies every
write and every read outside the worktree, so a gap in the list opens nothing.

The policy JSON is protected config #1 and the human's to write. This ticket builds what the gate
and the profile need, drives it from a fixture policy in the tests, and ends with the exact JSON
the human should put in `policies/read-only.json`, in the report and nowhere else.

## Do

Tests first, each shown failing before the code:

1. Gate tests in `tests/gate.rs` with a fixture policy granting `shell` scoped to a command list:
   - allowed: `grep -rn x src`, `rg x`, `find . -name '*.rs'`, `sed -n 1,20p f`, `cat f`,
     `head f`, `tail f`, `wc -l f`, `ls`, `git log`, `git show HEAD`, `git diff`;
   - refused, each with a reason naming the part: `sed -i s/a/b/ f`, `find . -delete`,
     `find . -exec rm {} \;`, `git checkout f`, `rm f`, `cat f > g`, `cat f | sh`,
     `echo $(rm f)`, `` `rm f` ``, `xargs rm`, `bash -c 'cat f'`, `python3 -c 'print(1)'`, a
     command the gate cannot parse.
   Pipes between allowed commands (`grep x f | head`) are allowed; anything the parser cannot
   split is refused, never guessed.
2. A seatbelt test beside `tests/sandbox_tty.rs`'s pattern: under the profile for this policy,
   a probe's write inside its own worktree is refused, and a read outside it is refused.
3. The list is data in the policy, not code: the gate reads it from the compiled policy. The
   shell pass today is a substring search (`src/policy/gate.py` near 279); this scope needs a
   real split of the command. Read how 63 and the pattern floor are built before writing a new
   parser, and reuse what exists.

Then `SECURITY.md` and `IMPLEMENTATION.md`: what the bounded shell refuses and that the seatbelt,
not the list, is the guarantee. Harness spec 7.2's row for `read-only`.

No tests run under this policy: `cargo test` writes `target/`. Say so in the spec row.

## Outcome
