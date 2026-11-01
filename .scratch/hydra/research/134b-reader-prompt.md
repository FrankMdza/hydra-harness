# A stranger reads the repository before it goes public

You are one of two readers. Read only: do not edit, create or delete a file in the repository, and do not commit. Scratch work under `/tmp` is fine. Your seat name is in your task line.

The repository goes public soon. You have never seen it. Read it in the order a visitor would, and stop where a visitor would stop:

1. `README.md`, top to bottom.
2. `docs/install.md`, then whichever of `docs/harness-spec.md`, `docs/run-record-contract.md` and `docs/rungs-spec.md` the README sends you to first. Skim, do not read all three in full.
3. `SECURITY.md`, `CONTRIBUTING.md`, `CHANGELOG.md`'s first screen, `LICENSE` and `NOTICE`.
4. The root listing and `.scratch/` as a visitor sees them on GitHub: what is this directory, and does the README explain it?

Do not read `IMPLEMENTATION.md` in full or anything else under `.scratch/`.

## Answer

1. In the first 30 seconds of the README: what does Hydra do, and for whom? Say it in one sentence, in your words. If you cannot, say where the README lost you.
2. Everything a visitor would find broken, stale, contradictory or embarrassing. For each: file and line, what it says, why it is wrong, and the smallest fix. Check numbers and dates against the tree when you can (test counts, ticket counts, versions, file names the prose names). Typos count. Internal jargon used before it is defined counts.
3. Does `docs/install.md` work as written on macOS? Say which command you would expect to fail, and why. Do not run builds.
4. Anything that should not be public: a personal detail, a path, another project's private information, a note to self.
5. The three changes that would most improve a stranger's first visit, smallest first.

At most 900 words. Rank item 2 by how likely a visitor is to hit it. End with `## Pending from me`, `## Changed` and `## Found`.
