# 123 - The run's git pointers reach every repository the run makes, not only its checkout

Type: grilling
Status: open
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
