# 326 - `hydra reap` decides what a staged nested repository becomes: the files committed and the gitlink dropped, or a refusal naming the path

Type: build
Status: resolved
Blocked by:
Opened: 2026-10-15, by the session, from map 123's answer, question 2. The panel put the fix in
reap, outside the seatbelt, where Hydra already rewrites the index before it commits
(`unstage_missing_objects`, `src/worktree.rs` line 743). Two treatments are on the table and the
human has not picked yet: commit the files and drop the gitlink, saying so in reap's output, or
refuse with a message naming the path and leave the checkout for the human. The session's
suggestion is the first, because a refusal leaves the checkout behind as today's exit 128 does.
**This ticket is not takeable until the human's word is written here.**

Decision: refuse by name, before any commit. 2026-10-15, the human on the session's recommendation
from panel 130 ("sigue tus recomendaciones"), which reversed the suggestion above. Two facts
decided it, both from Grok's seat and checked: `unstage_missing_objects` leaves mode 160000 alone
on purpose because a legitimate submodule is also a gitlink, so flattening every gitlink flattens a
pin a run set to an upstream commit; and `import_run_objects` runs before `commit_everything`, so
the nested objects already arrive in the parent and "`git fsck --unreachable` empty" cannot hold
under the commit treatment. A refusal is the smaller change, stops the gitlink reaching the branch
(today's unstaged path already publishes one), leaves the nested repository intact, and keeps
flattening available later as an explicit flag with its own ticket.

## What

Before `import_run_objects` and `commit_everything`, reap finds every index entry of mode 160000
and every path under the checkout named `.git`, directory or file (a `gitdir:` pointer from a
worktree or a submodule), skipping the checkout's own. If any is found, reap exits non-zero before
any commit, prints one line per path, and leaves the checkout, its index and its files untouched.
The run record's reap event, if it has one, carries the same paths. `hydra status` keeps listing
the checkout as waiting for a reap.

Build 325's two cases change their assertions: no commit on the branch (`git rev-parse` of the
branch unchanged), the checkout still present with the nested `.git` intact, and the message naming
`sub`. Add a case with two nested repositories (both named), a case where `.git` is a file, and a
case with a legitimate submodule staged by the run, which must also refuse rather than flatten.
Measure that the parent's object store gains no object from the refused reap (`git count-objects
-v` before and after).

## Do

1. The scan and the refusal in `src/reap.rs`, before the import.
2. 325's cases updated; the three new cases.
3. `IMPLEMENTATION.md` beside the `run_object_env` paragraph says what reap does now, and that
   flattening is not offered.

## Done means

`cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo
nextest run --release --workspace`, `cargo test --release --workspace --doc` green, and the
decision named in the Outcome.

Rules inside the run: relative paths only, never look outside your own worktree, write files with
your file tool and not with shell heredocs, and the test suite is green inside a run since build
302 without any `env -u`.

## Outcome

Implemented the human's decision: refuse by name before any commit; flattening is not offered.
`src/reap.rs` reads the trusted checkout index with NUL-delimited `ls-files --stage`, gathers
every mode 160000 entry, and walks the checkout for nested `.git` entries, including ignored
paths and gitdir files. It skips the checkout's own pointer, Git metadata contents and directory
symlinks. Sorted, deduplicated repository paths are escaped onto one report line each. The scan
runs before pointer repair, private-log removal, injected-ignore removal, object import and
index repair, so a refusal leaves the checkout and its index untouched.

Updated build 325's two tests deliberately: staged and unstaged nested repositories now return
`Reaped::Failed` by name before importing or committing, rather than pinning the old git-status
failure or the old gitlink commit followed by removal failure. Added four tests: two nested
repositories (one ignored and deeper), a gitdir file, a legitimate submodule staged with the
run's object pointers, and an index-only gitlink with no on-disk marker. All six check the exact
report paths, unchanged branch SHA, byte-identical index and checkout pointer, surviving nested
markers and injected ignore, retained spawn directory, and status's waiting checkout list.

Measurements: the cleanup suite passed 52/52 tests. `git count-objects -v` before and after
each refusal was identical: count 4, size 16 KiB in five cases; count 5, size 20 KiB in the
legitimate-submodule case; in-pack, packs, size-pack, prune-packable, garbage and size-garbage
all 0 in every case. Newly unreachable objects: 0 in all six. One path is reported in five
cases, two in the two-repository case; failures() is 1, which the CLI maps to exit 1.

No new false assumption in the decided ticket. The earlier flattening suggestion and build
325's unstaged-files expectation are superseded by the recorded human decision. Reap has no
run-record event to extend: it does not append to another process's record, as harness spec 10
requires. `IMPLEMENTATION.md` now records the refusal and measurements beside the object-store
paragraph. No gate refusal occurred.

Required validation, all exit 0:
- `cargo fmt --all --check`: no output.
- `cargo clippy --workspace --all-targets -- -D warnings`: Finished `dev` profile
  [unoptimized + debuginfo] target(s) in 29.46s.
- `cargo nextest run --release --workspace`: Summary [47.332s] 1431 tests run:
  1431 passed, 4 skipped.
- `cargo test --release --workspace --doc`: test result: ok. 0 passed; 0 failed;
  0 ignored; 0 measured; 0 filtered out; finished in 0.00s.
