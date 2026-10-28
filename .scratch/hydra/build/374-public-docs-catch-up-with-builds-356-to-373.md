# 374 - The public documents catch up with builds 356 to 373 and maps 108, 135 and 136

Type: build
Status: claimed
Blocked by: 373

## Objective

Builds 340a and 340b updated the public documents before builds 318a and 318b landed, and nothing has updated them since. Bring them up to date with what master holds on 2026-10-28. Read the `## Outcome` of every build ticket from 356 to 373 in `.scratch/hydra/build/`, and the answers of map tickets 108, 135 and 136 in `.scratch/hydra/issues/`. Build 370 is partial; read its Outcome closely.

The one claim to get right is Linux. Since 318b a Linux launch runs under bubblewrap, and build 370 launched one in Docker: it started, recorded `sandbox_kind: bubblewrap`, and its deny paths held. But git did not work inside that launch, and the cause is open. So the documents say that Linux has a bubblewrap wrapper, that one launch was measured, and that Linux launches are not ready for real work until 370 closes. Do not claim more than 370's Outcome measured.

## Files

- `README.md` lines 6 to 10, which say no sandbox wraps a Linux launch.
- `SECURITY.md`: the Linux paragraph around line 402, which says launches are hook-only until 318 lands; the `git push` floor paragraph, which build 365 already amended, so check only that it reads as one paragraph; what a run can write in its private git directory, and the sweep of builds 367 and 371; the secret classes after builds 368 and 369; and what survives a run after builds 372 and 373.
- `docs/install.md`, where it describes Linux, and the requirement for `bubblewrap` with user namespaces.
- `docs/roadmap.md`: move what landed out of the open list.
- `CHANGELOG.md` under `## Unreleased`: one dated entry for 2026-10-27 covering builds 356 to 373 and maps 108, 135 and 136, in the style of the entries already there, with run ids where a ticket names them.
- `.github/workflows/ci.yml` line 61, which says "twelve" unit tests; drop the count.
- `CLAUDE.md`'s layout paragraph only where it is now false: the crates list if it names crates, and `procinfo.rs` if a file description needs it. Do not touch its counts line; the session keeps that.

## Rules

Prose follows plain style: no em dashes, no parentheses for asides, sentence case, short sentences, name the mechanism, not the feeling. Every claim points at the build or map ticket that carries it. Write only what a ticket's Outcome measured or a file in `src/` shows.

Run `cargo test --release --workspace --doc` and `git diff --check`. Nothing else changes code.

## Outcome
