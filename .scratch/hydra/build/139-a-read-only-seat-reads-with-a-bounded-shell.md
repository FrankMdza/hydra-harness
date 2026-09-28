# 139 - A read-only seat reads with a bounded shell under a write-denying seatbelt

Type: build
Status: resolved
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

Run `01a0e79c` (claude opus, `standard`). The policy gains an optional `commands` list on `shell`
(`src/policy/mod.rs`, `ShellGrant`, containment ordered), the gate a bounded-shell pass
(`hydra/shell-list`: quote-aware hazard walk, split on the floor's segments, exact words per
entry, refused flags whole or abbreviated, every argument resolved inside the `file_read` scopes),
and the profile, under a bounded shell only, a worktree write deny and `file-read-data` denied
under Hydra's tree with the seat's own paths reopened. Tests failed first where they could.

The session changed two things before applying. **`sed` is off the list.** Its own script writes
(`w`, `W`) and reads (`r`) files no argument names, so the gate cannot see them, and this profile
does not deny writes outside Hydra's tree; `head`, `tail` and `grep -n` cover what a reading seat
needs. `sed -n 'w /tmp/x' f` is now a refused case in `tests/gate.rs`. **The live seatbelt probe
failed on master**, where it runs (inside the run it skips, nested `sandbox-exec` being refused):
the seat could not read its own worktree. A later `(allow file-read* ..)` does not lift an earlier
`(deny file-read-data ..)`; only an allow naming `file-read-data` reopens it, measured on three
hand-written profiles. `compose` now emits both for each reopened path under a bounded shell.

What the ticket assumed that was false, from the run: the seatbelt cannot deny every write and
every read outside the worktree, because it wraps the provider, which reads its install and writes
its session state; which paths each of the four needs is unmeasured, so the profile covers
Hydra's tree only and `SECURITY.md` says the rest of `HOME` is not covered. Binding `HOME` is a
map question. No provider has started under this profile yet: that is the first thing 140 must
measure. `codex` stays refused under `read-only` by name (`src/provider/codex.rs:327`).
`policies/README.md` needs a line on `commands` when the human writes the JSON; the proposed
`policies/read-only.json` (the test fixture, without `sed`) is at
`~/.claude/handoffs/2026-09-28-builds/PROPUESTA-policies-read-only.json`.

Master: fmt, clippy, lib 213, gate 105, sandbox_read_only 3 (live probe included), spawn 35,
sandbox_tty 2, panels 8, delegation 14, normalize 87, doctor 3, evolve 42, face 8.
