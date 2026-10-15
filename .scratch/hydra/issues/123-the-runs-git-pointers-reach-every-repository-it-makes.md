# 123 - The run's git pointers reach every repository the run makes, not only its checkout

Type: grilling
Status: resolved
Blocked by:
Opened: 2026-10-13, by the session, from build 309's Outcome ("Found on the way") and builds
295, 296 and 302 before it. Grilling: the human answers, the session only asks and brings
what it measured.

## Question

Map 74 hands every run `GIT_OBJECT_DIRECTORY` and `GIT_ALTERNATE_OBJECT_DIRECTORIES` so the
run's git writes into the run's own object store and `hydra reap` can import what it wrote.
The two variables are process environment, so they reach every `git` the run's shell starts,
including one that makes a new repository under the worktree: `git init` there creates a
`.git` with no objects of its own, `git commit` writes the commit into the run's store, and a
reader that drops the variables (Hydra's own git does, and a test fixture does since 302)
finds a repository with no history, walks up, and reaches the parent checkout. On 2026-10-13
that reader was Hydra's `worktree::create` under test inside a run, and it tried to write a
ref into `~/hydra/.git`, which the seatbelt refused. Four builds this week had to be told the
`env -u` incantation, and 302 moved it into every test fixture.

1. Is this the design? Map 74 wanted the run's checkout to write into the run's store. A
   repository the run creates is not the run's checkout, and its objects land in a store that
   reap imports into the parent as unreferenced objects. Is that what reap should carry, or
   should a run's new repository be its own and travel in the worktree as files?
2. If not the design, where is the fix? Three places: the allowlist could hand the pointers
   through a git config the checkout alone reads (`extensions.objectFormat` cannot do it, but
   `core.alternateRefsCommand` cannot either; `GIT_DIR`-scoped config would mean a wrapper), a
   `git` shim on the run's PATH that sets the pointers only when the cwd's `--git-common-dir`
   is the checkout's, or nothing in the product and a sentence in the preamble telling a run
   that its git writes to the run's store wherever it runs.
3. The test suite is already independent of the pointers since 302. Does the product need to
   change at all, or is this a documentation line in `IMPLEMENTATION.md` and `config/preamble.md`
   (protected, so yours)?

The session's own view, which is not an answer: the shim is the only option that keeps map
74's property and fixes the sub-repository case without a human reading a preamble line, and
it is a new binary on every run's PATH, which is a seatbelt and allowlist question before it
is a build.

## Answer

Resolved 2026-10-15 by the human, who took the panel's recommendation ("sigo con las
decisiones del panel").

1. The pointers reaching every repository is the cost build 103 wrote into `run_object_env`'s
   doc comment, and it stays. A sub-repository's history landing in the parent as unreferenced
   objects is harmless: checked by `index-pack --strict`, capped, pruned by `git gc`. What breaks is
   `hydra reap` on a checkout where the run staged a nested repository: a gitlink of mode 160000
   that reap's pointer-less git cannot resolve, exit 128, the checkout left behind.
2. No shim and no allowlist change. The shim cannot see `git init` coming (before it runs, the cwd
   belongs to the checkout) and costs 7.3 ms per git call. The fix is in reap, outside the
   seatbelt, where Hydra already rewrites the index before it commits: every index entry of mode
   160000 and every directory holding a `.git` under the checkout gets one treatment. Which of the
   two, commit the files and drop the gitlink, or refuse naming the path, is the one word still the
   human's; the session's suggestion is the first, said in reap's output, because a refusal leaves
   the checkout behind as today does.
3. The suite needs nothing more since build 302. `IMPLEMENTATION.md` carries the measured
   consequences since 78f9de2. A sentence in `config/preamble.md` is protected and the human's.

Tickets: build 325 (the `tests/cleanup.rs` case, no decision needed), build 326 (reap's
treatment, after the word).

## Panel

Two seats sat on 2026-10-13, Opus 5.5 and Gemini 3.8 Flash; the GPT seat died on codex's usage limit at its first turn and there is no Grok seat while Cursor is on the free plan. Memos: `research/123-seat-opus.md`, `123-seat-gemini.md`. The session's synthesis with a recommendation per question: `research/123-synthesis.md`. Its first finding: the leak is written down in `src/worktree.rs` as an accepted cost, and the half that breaks is `hydra reap` on a repository nested in a checkout, exit 128 when the run staged it. Nothing under Answer is written; the human answers.
