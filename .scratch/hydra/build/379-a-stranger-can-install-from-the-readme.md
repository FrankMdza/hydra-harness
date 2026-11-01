# 379 - A stranger can install from the README, and its numbers are true

Type: build
Status: claimed
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
