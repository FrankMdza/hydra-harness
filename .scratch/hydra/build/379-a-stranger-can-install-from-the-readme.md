# 379 - A stranger can install from the README, and its numbers are true

Type: build
Status: resolved
Blocked by:

## Objective

Two readers who had never seen the repository read it on 2026-11-01 as a visitor would, from `.scratch/hydra/research/134b-reader-prompt.md`. Runs `01a19945-a50a` (Opus) and `01a19946-4f95` (GPT). They agreed on what a visitor hits first. The session also built a fresh clone of master from zero (34 s) and launched one recorded run from it with `HYDRA_ROOT` set, which ended `completed`. So the install works. What fails is the text around it.

## Scope

Prose and one script. No Rust changes.

1. `docs/install.md` lines 10 to 20 and the README's `## Install` block: `hydra doctor` comes before the line that puts `~/.local/bin` on `PATH`, so on a default Mac it fails with "command not found". Put `export PATH="$HOME/.local/bin:$PATH"` inside the block, before `hydra doctor`, with one sentence saying to add it to the shell profile. Name the two prerequisites a fresh Mac lacks: the Command Line Tools (`xcode-select --install`, which also provides `/usr/bin/python3` and `git`) and a Rust toolchain through rustup.
2. The README's `## Install` block lacks the `masterofpuppets` symlink that `## Quick start` uses. Copy the `ln -sf` line from `docs/install.md`.
3. README lines 6 to 14: build ticket numbers, "map 119" and "stand-in provider" arrive before anything defines them. Cut the paragraph to two or three plain sentences: measured on macOS; on Linux it compiles, the suite runs in CI minus named tests, launches run under bubblewrap, and no real provider has been measured there yet. Link `docs/roadmap.md`. `.github/workflows/ci.yml` says the README names the nine skipped tests. If the README no longer names them, move the list into `docs/install.md`'s Linux or ports section and change the `ci.yml` comment to point there. `tests/workflows.rs` must stay green.
4. README line 93: "only `claude` takes `--policy` there until build ticket 130" reads as done. Ticket 130 is open. Say that the other three wait for it.
5. README `## Status`: every count is stale. On 2026-11-01 it says 41 test files under `tests/` and the tree has 42; 356 build tickets opened and 353 resolved; 134 map tickets and 122 resolved. Count each one from the tree and say how you counted in the Outcome (`./target/release/hydra-puppets frontier` and `frontier build` print the resolved counts). Date the table. If a test in the repository already pins one of these numbers, update it with the prose.
6. README line 162 links into `.scratch/hydra/research/129-ready-to-go-public.md`, while line 200 says a reader needs only `README.md`, `SECURITY.md` and `docs/`. Keep the link and mark it as design history, or drop it. Rewrite the sentence about `.scratch/` at lines 197 to 200 so it says what is true: those files reference notes outside the repository, in `~/.claude/handoffs/`, that are not published. "Point at the author's private notes in places" reads as "there are private things in here".
7. `llms.txt` contradicts the README. It says "macOS only" and "Linux untested", and builds with `cargo build --release` without `--workspace`, which skips `hydra-puppets`. Make it match the README's install line and its Linux sentence.
8. `integrations/copilot/bin/masterofpuppets` lines 7 to 10: the usage examples name `~/private-repo`, another private project. Use `~/some-repo`.

Leave alone: `config/export-allowlist.json`, which is protected; `docs/demo/corpus.cast` and the gifs; the three design documents in `docs/`, except to move the test list in item 3; `CLAUDE.md`.

## Checks

`cargo fmt --all --check`, `cargo clippy --release --workspace --all-targets -- -D warnings`, `cargo check --release --target x86_64-unknown-linux-gnu`, and `cargo nextest run --release --workspace --no-fail-fast`.

## Outcome

Prose and one script, no Rust.

1. The README's `## Install` block and `docs/install.md` now run `export PATH="$HOME/.local/bin:$PATH"` before `hydra doctor`, then tell the reader to add it to `~/.zshrc`. Both name the Command Line Tools through `xcode-select --install`, which brings `git` and `/usr/bin/python3`, and rustup for the Rust toolchain.
2. The README's install block gained the `masterofpuppets` `ln -sf` line from `docs/install.md`.
3. The README's opening Linux paragraph is three sentences with no ticket numbers. The nine skipped test names moved to `docs/install.md` under `## Ports`, and the `ci.yml` comment now points there. `tests/workflows.rs` passes.
4. README line 93 now says only `claude` takes `--policy` in interactive mode and the other three wait for build ticket 130, which is open.
5. The `## Status` table is dated 2026-11-01 and recounted. `ls tests/*.rs` gives 42 and `ls crates/*/tests/*.rs` gives 25. `ls .scratch/hydra/build/` gives 376 numbered ticket files, and `grep -l '^Status: resolved'` gives 374 once this ticket closes. `hydra-puppets frontier build` prints 369 of 372 because `frontier.rs` line 88 takes only all-digit ids and skips 318a, 318b, 340a and 340b. The table uses the file count. `hydra-puppets frontier` prints 128 of 136, which matches the 136 files under `issues/`. The records row keeps its 2026-10-27 count: `~/hydra/runs/` is outside what this run may read, and the table says so. No test pins any of these numbers.
6. The link to `research/129-ready-to-go-public.md` stays, marked as design history. The `.scratch/` paragraph now says that some of those files cite handoff notes in `~/.claude/handoffs/`, which are not published.
7. `llms.txt` builds with `--workspace`, says measured on macOS, and carries the README's Linux sentence instead of "Linux untested".
8. `masterofpuppets` usage examples name `~/some-repo`.

`IMPLEMENTATION.md` needed no change. It makes no claim about the README's install block, the counts or the skip list.

False assumption: the ticket's counts assumed the frontier and the tree agree. They differ by four, the lettered tickets. CLAUDE.md's "372 resolved of 374" matches neither and is protected from this run.

Checks: `cargo fmt --all --check`, clippy with `-D warnings`, and `cargo check` for `x86_64-unknown-linux-gnu` are clean. `cargo nextest run --release --workspace --no-fail-fast` ran 1547 tests with 1547 passed and 4 skipped.

The session, on 2026-11-01: `hydra doctor --walk` checked 1,056 records, and the README's records row now says so. CLAUDE.md's counts now read 374 of 376, with a note that `hydra-puppets frontier` skips lettered ids.
