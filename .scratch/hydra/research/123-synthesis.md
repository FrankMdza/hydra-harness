# Map 123, the session's synthesis for the human

Two seats sat on 2026-10-13: Opus 5.5 on `claude` (`123-seat-opus.md`) and Gemini 3.8 Flash on
`agy` (`123-seat-gemini.md`). The GPT seat died at its first turn on codex's usage limit; no Grok
seat while Cursor is on the free plan. Both seats measured inside their own gated worktree with the
two pointers live, which is the right place: Opus against scratch object stores it pointed the
variables at, Gemini against a sub-repository it initialized under `.scratch/`. The session checked
the code facts they cite.

## A finding before the questions: the leak is written down, and reap is the half that breaks

Both seats found the sentence the ticket did not quote: `src/worktree.rs` lines 366 and 367, the
doc comment of `run_object_env`, ends "The cost is that git inside the run, in any repository,
reads and writes this store", and `IMPLEMENTATION.md` says the same beside it. So this is not an
unintended side effect nobody saw; build 103 saw it and shipped it as an accepted cost. Map 74 did
not decide it. Opus calls it "the design by acceptance, never by decision", which is the accurate
name.

Opus then measured what the ticket did not ask: what `hydra reap` does with a repository nested in
a checkout. A `git init` plus commit under the pointers makes a `.git` with no `objects/`; if the
run never staged it, reap's pointer-less `git add -A` stores the files as plain files and the
history arrives in the parent as unreferenced objects (bounded by `RUN_STORE_IMPORT_CAP`, 1 GiB,
`src/worktree.rs` line 398, and checked by `index-pack --strict`). If the run ran `git add -A`
itself, which agents do, the run's git stages a gitlink, mode 160000, and reap's pointer-less
`git status --porcelain` and `git add -A` exit 128, "not recognized as a git repository";
`unstage_missing_objects` skips 160000 entries on purpose (line 758, confirmed), so reap fails at
`commit_everything` and leaves the checkout behind. Opus reproduced this with reap's own git
commands by hand, not with the reap binary, and says a `tests/cleanup.rs` case should confirm it
first. Gemini did not look at reap.

## Where the two agree

- **Question 1.** The pointers reaching every repository is not a decision anyone took, and a
  sub-repository's objects landing in the parent as unreferenced objects is harmless in itself:
  checked, capped, pruned by `git gc`.
- **Question 2.** No shim. Both seats measured the same bootstrapping failure in the ticket's
  proposed condition: before `git init` or `git clone` runs, the cwd is an ordinary directory of
  the checkout and `git rev-parse --git-common-dir` answers the checkout's, so the shim would set
  the pointers for exactly the commands it should not. Opus measured the cost too: 7.3 ms per
  extra `rev-parse`, about 60 percent on a cheap git call. And both note that a shim is a new
  executable on every run's `PATH`, an allowlist change (`src/env.rs` line 21 allows `HOME`,
  `USER`, `PATH`, `TERM`) and a seatbelt read grant, to fix a case the run's own git already
  handles consistently. No git config key moves an object directory either.
- **Question 3, the suite.** Nothing more: build 302 made every fixture strip the pointers, and
  Gemini ran the whole suite inside its run with the pointers live, 1,388 of 1,388.
- **The preamble.** `config/preamble.md` is protected config #4 and both seats would keep git
  plumbing out of it, Gemini firmly, Opus leaning the other way with one sentence ("Git in this
  run writes objects to the run's own store in every repository, including one you create or
  clone; a tool that does not read git's environment will not see them"). The session's view:
  this week four builders needed exactly that sentence, so a line somewhere is cheap; the human
  decides whether the somewhere is the preamble or the task text.

## Where they split, and the recommendation

1. **Whether the product changes at all.** Gemini: no, document it in `IMPLEMENTATION.md` and the
   repo skills and leave the runtime alone. Opus: yes, in reap, outside the seatbelt, where Hydra
   already rewrites the index before it commits: find every index entry of mode 160000 and every
   directory holding a `.git` under the checkout and decide once what happens to them, either
   "commit the files and drop the gitlink" or "refuse with a message naming the path". The
   session sides with Opus, because Gemini's "no change" leaves a measured path where `hydra
   reap` exits 128 and the human's cleanup command stops working on a checkout an agent staged.
   Which of the two candidates reap picks is the human's, and that is the real question 123 should
   have asked.
2. **Should a run's new repository own its objects.** Gemini's recommendation for question 1 is
   that a sub-repository should be self-contained and travel as files. Opus measured that case
   too: a self-contained sub-repository makes the parent's `git add -A` stage a gitlink and none of
   the files, and `git worktree remove` on a checkout with a submodule needs `--force`, which reap
   does not pass (that last claim is from git's documentation, unmeasured, because the gate refused
   `git worktree`). So "its own objects" does not give "travels as files" either; reap decides what
   travels in both cases. Recommendation: keep the pointers; put the decision on reap.

Fable was not called as judge: the one split resolves on a measurement Gemini did not make.

## What would become tickets if the human agrees

- Build, first and no decision needed: a `tests/cleanup.rs` case that makes a checkout, `git init`
  and commits a sub-directory under the run's pointers, stages it from the run, and reaps; it
  should fail today with exit 128 and pin the behaviour the human then chooses.
- Build, after the human picks: reap's handling of 160000 entries and nested `.git` directories,
  commit the files or refuse by name.
- One line in `IMPLEMENTATION.md` beside the `run_object_env` paragraph naming the measured
  consequences (no `objects/`, silent walk-up to the checkout), which the session can write
  without a decision.
- Protected, the human's: one sentence in `config/preamble.md`, or none.
- Nothing for the shim, the allowlist or the suite.
