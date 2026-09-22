# 66 - Rung two's admission gate runs the proposer's code before any judge sees it

Type: grilling
Status: open
Blocked by:

## Question

`rungs::evolve::admission_gate` runs `cargo fmt --check`, `cargo clippy`, `cargo build` and
`cargo test` in the proposer's worktree, from the `hydra evolve` process, which is the human's
shell with the human's environment and no seatbelt. The two checks before it are the line-count
cap and `protected_path_hit`, whose list names source rows and not `tests/`, `build.rs`,
`Cargo.toml` or `.cargo/`. So a proposer under `standard` can add a test, or a build script, that
`cargo test` executes with the human's credentials outside the gate and outside the sandbox
before a single judge reads the patch. Two of the seven readers on 2026-09-22 called it the most
serious thing in the repository, and `SECURITY.md` states it. Rungs spec 4.5 puts admission
before review on purpose, so that judges never read a patch that does not build; that ordering is
what makes the hole.

1. **Does admission run under the seatbelt the proposer ran under?** The profile is in the
   proposer's spawn directory already. It bounds writes to Hydra and reads of the corpus, and
   nothing else, so a hostile test still reads `~/.ssh`; and the corpus walks in `tests/` read
   `~/hydra/runs/`, which the profile denies, so admission would have to point `HYDRA_ROOT` at an
   empty root the way CI does, and then a pass whose patch breaks a corpus reader passes
   admission. Pass 9 was blocked by exactly a corpus test.
2. **Or does `cargo test` move after the panel?** Judges read a patch that may not build; the
   checklist has a `builds` item they cannot answer without running it. A pass would then have
   two verdicts, the panel's and the suite's, and spec 4.5's order flips.
3. **Or does the protected-path list grow** to `tests/`, `build.rs`, `Cargo.toml` and `.cargo/`?
   Then a proposer cannot add a test, and map ticket 53 decided the judge sees the edited
   baseline test because proposers edit tests.
4. **What does the record say?** A harness record of a pass names the admission steps and their
   exit codes. Whatever runs the suite, the record should say under what.

Until this closes, `SECURITY.md` says admission runs proposer-authored code on this machine
outside the seatbelt, and `hydra evolve` is a command to type with that in mind.
