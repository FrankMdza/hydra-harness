The goal splits into four pieces of work and one order between them. `chars` goes into the library first, because the binary and its tests both call it. The binary comes next, then the tests that drive it and the README that shows its output, and those two can run side by side.

`chars` is an addition to `src/lib.rs`. `words`, `lines` and `bump` stay as they are, byte for byte, because the goal forbids rewriting them. The existing `use super::{...}` line in the test module is the one place an existing line may change, to pull in `chars`.

The binary needs no new dependency. Three integer values with fixed keys don't justify serde, so `main.rs` formats the JSON by hand and parses its arguments by hand: `--json`, an optional path, nothing else. Cargo finds `src/main.rs` on its own and names the binary after the package, so `Cargo.toml` stays untouched. I pin the plain output as `words: N`, `lines: N`, `chars: N` in that order, and the JSON as `{"words":N,"lines":N,"chars":N}` with no spaces, since the done check compares that exact string. The README task has to copy whatever the binary really prints, which is why it waits on the binary rather than on the tests.

The integration tests use `env!("CARGO_BIN_EXE_tally")` and `std::process::Command`, so they also need no new crate. Every task here is ordinary Rust, nowhere near Hydra's launch path, and the grades are low because each change is small and checkable by one command.

```json
[
  {
    "id": "lib-chars",
    "text": "Add `pub fn chars(text: &str) -> usize` to src/lib.rs, counting characters that are not whitespace (`text.chars().filter(|c| !c.is_whitespace()).count()`), with a doc comment in the style of the others. Do not change `words`, `lines` or `bump`. Add a unit test `chars_skips_whitespace` covering \"\", \"  \\n\", \"ab c\" (3) and a non-ASCII case such as \"é é\" (2), and extend the test module's `use super::{...}` line to import `chars`. Update the crate doc line to mention characters.",
    "class": "rust_off_launch_path",
    "grade": "low",
    "done": "`cargo test --lib` passes with four tests, including `chars_skips_whitespace`; `git diff src/lib.rs` shows no removed lines inside `words`, `lines` or `bump`.",
    "deps": [],
    "paths": ["src/lib.rs"]
  },
  {
    "id": "bin-main",
    "text": "Create src/main.rs for a binary named `tally` (Cargo auto-discovers it; leave Cargo.toml alone unless the build says otherwise). Parse arguments by hand: `--json` sets JSON output, at most one other argument is a file path, anything else (a second path or an unknown flag starting with `-`) prints a usage line to stderr and exits with code 2. Read the whole file as UTF-8 if a path is given, otherwise read all of stdin. A file that cannot be read prints the error with the path to stderr and exits 1. Compute `tally::words`, `tally::lines` and `tally::chars`. Default output is exactly three lines: `words: N`, `lines: N`, `chars: N`. With `--json`, print one line `{\"words\":N,\"lines\":N,\"chars\":N}` with no spaces, built with `format!`, no serde. No new dependencies.",
    "class": "rust_off_launch_path",
    "grade": "low",
    "done": "`printf '' | cargo run --quiet -- --json` prints `{\"words\":0,\"lines\":0,\"chars\":0}`; `printf 'a b\\nc\\n' | cargo run --quiet` prints `words: 3`, `lines: 2`, `chars: 3`; `cargo run --quiet -- Cargo.toml` prints three lines; `cargo run --quiet -- missing.txt` exits 1; `Cargo.toml` has no new `[dependencies]` entry.",
    "deps": ["lib-chars"],
    "paths": ["src/main.rs"]
  },
  {
    "id": "cli-tests",
    "text": "Create tests/cli.rs with integration tests that run the built binary through `std::process::Command::new(env!(\"CARGO_BIN_EXE_tally\"))`. Cover: text on stdin with the plain output, compared line by line; a file argument, written to a temporary path under `std::env::temp_dir()` with a unique name and removed after; `--json` on non-empty stdin, compared as an exact string; empty stdin both plain (all three zero) and with `--json` (exactly `{\"words\":0,\"lines\":0,\"chars\":0}` plus a newline); a missing file exits non-zero. No new dev-dependencies.",
    "class": "rust_off_launch_path",
    "grade": "low",
    "done": "`cargo test` is green and its output lists the tests from tests/cli.rs, at least one each for stdin, a file argument, `--json` and empty input.",
    "deps": ["bin-main"],
    "paths": ["tests/cli.rs"]
  },
  {
    "id": "readme-cli",
    "text": "Add a `## Command line` section to README.md after the existing paragraph. Say that `tally` reads stdin or one file named as an argument and counts words, lines and non-whitespace characters. Show one example invocation with its plain three-line output and one with `--json` and its single-line output, each in a fenced block, with the numbers taken from actually running the binary on that input. Fix the existing paragraph's 'tallys' to 'tallies' and add characters to what it counts. Leave the sentence about Hydra's bench goals as it is.",
    "class": "docs",
    "grade": "low",
    "done": "README.md has a `## Command line` section showing both output shapes; piping each example's input through `cargo run --quiet` (with and without `--json`) reproduces the output the README shows.",
    "deps": ["bin-main"],
    "paths": ["README.md"]
  }
]
```
