The library already has `words` and `lines`, so the only library change is a new `chars` function that counts non-whitespace characters. `words`, `lines` and `bump` stay as they are. Everything else is new files: `src/main.rs` for the binary, `tests/cli.rs` for the end-to-end tests, and a section in `README.md`. Cargo picks up `src/main.rs` as a binary named `tally` on its own, because `autobins` is still on even with the explicit `[lib]` table, so `Cargo.toml` should not need an edit.

I'm fixing the text format here so the binary, the tests and the README can't drift apart. Plain output is three lines in the order `words`, `lines`, `chars`, each `name: N`, for example `words: 3`. `--json` prints `{"words":N,"lines":N,"chars":N}` on one line, in that key order and with no spaces, which is the exact string the goal's done check expects. The keys are fixed and the values are integers, so the binary writes the JSON with `format!` and no `serde` dependency. One positional argument is a file path, and no argument means stdin. An unreadable file, an unknown flag or a second positional argument prints a message to stderr and exits 2.

The plan is five tasks. `chars` goes first because the binary calls it. The tests and the README both depend on the binary but not on each other, so they can run in parallel. The last task is a probe that runs the goal's own done checks from a clean build. Every task below repeats the output format, because the tests and README agents run in parallel and each has to match the binary without seeing the other's work.

```json
[
  {
    "id": "lib-chars",
    "text": "Add `pub fn chars(text: &str) -> usize` to src/lib.rs that returns the number of non-whitespace characters, `text.chars().filter(|c| !c.is_whitespace()).count()`, with a one-line doc comment in the style of the others. Add a unit test `chars_skips_whitespace` in the existing tests module covering \"\" -> 0, \"  \\n\" -> 0, \"ab c\" -> 3 and a multibyte case like \"né\" -> 2, and add `chars` to the module's `use super::{...}` line. Do not change `words`, `lines` or `bump` or their tests.",
    "class": "rust_off_launch_path",
    "grade": "low",
    "done": "`cargo test --lib` passes with `chars_skips_whitespace` listed, and `git diff src/lib.rs` shows only additions plus the edited `use` line.",
    "deps": [],
    "paths": ["src/lib.rs"]
  },
  {
    "id": "bin-main",
    "text": "Create src/main.rs, a binary named `tally` (Cargo auto-detects it; leave Cargo.toml alone unless the build says otherwise). Parse `std::env::args()` by hand, no clap: `--json` is the only flag, at most one positional argument is a file path, no positional means read all of stdin. Read input into a String with `std::fs::read_to_string` or `io::stdin().read_to_string`. Compute `tally::words`, `tally::lines` and `tally::chars`. Without `--json`, print exactly three lines: `words: N`, `lines: N`, `chars: N`, in that order. With `--json`, print exactly `{\"words\":N,\"lines\":N,\"chars\":N}` followed by a newline, built with `format!`, no serde. On an unknown flag, a second positional argument, or a read error, print a one-line message to stderr and exit with status 2. Do not touch the library functions.",
    "class": "rust_off_launch_path",
    "grade": "medium",
    "done": "`printf '' | cargo run --quiet -- --json` prints `{\"words\":0,\"lines\":0,\"chars\":0}`; `printf 'a b\\nc\\n' | cargo run --quiet` prints `words: 3`, `lines: 2`, `chars: 3` on three lines; `cargo run --quiet -- --bogus </dev/null; echo $?` prints a stderr message and `2`.",
    "deps": ["lib-chars"],
    "paths": ["src/main.rs"]
  },
  {
    "id": "cli-tests",
    "text": "Create tests/cli.rs with integration tests that run the built binary through `std::process::Command::new(env!(\"CARGO_BIN_EXE_tally\"))`, piping stdin where needed. Cover: (1) stdin input `a b\\nc\\n` gives stdout `words: 3\\nlines: 2\\nchars: 3\\n`; (2) a file argument, written to a temp path under `std::env::temp_dir()` with a unique name and removed afterwards, gives the same counts as the same text on stdin; (3) `--json` on that input gives exactly `{\"words\":3,\"lines\":2,\"chars\":3}\\n`; (4) empty stdin gives `words: 0\\nlines: 0\\nchars: 0\\n`, and empty stdin with `--json` gives `{\"words\":0,\"lines\":0,\"chars\":0}\\n`; (5) a missing file path exits with status 2. No new dev-dependencies.",
    "class": "rust_off_launch_path",
    "grade": "medium",
    "done": "`cargo test --test cli` runs at least five tests and all pass; temporarily breaking the JSON key order in src/main.rs makes the `--json` tests fail.",
    "deps": ["bin-main"],
    "paths": ["tests/cli.rs"]
  },
  {
    "id": "readme-cli",
    "text": "Add a `## Command line` section to README.md after the existing paragraph. Say the binary reads stdin or one file path, and that `chars` counts non-whitespace characters. Show two examples with their real output, copied from running the binary: `printf 'a b\\nc\\n' | cargo run --quiet` printing `words: 3`, `lines: 2`, `chars: 3`, and the same input with `-- --json` printing `{\"words\":3,\"lines\":2,\"chars\":3}`. Mention the exit status 2 on bad arguments or an unreadable file. Leave the existing paragraph unchanged, including its typo.",
    "class": "docs",
    "grade": "low",
    "done": "README.md has a `## Command line` section with both output shapes, and running each shown command reproduces the shown output byte for byte.",
    "deps": ["bin-main"],
    "paths": ["README.md"]
  },
  {
    "id": "goal-check",
    "text": "From a clean checkout of the combined result, run `cargo clean`, then `cargo test`, then `printf '' | cargo run --quiet -- --json`, and check the README shows both shapes. Report each command's output. Change nothing; if a check fails, name the task whose output is wrong.",
    "class": "probe",
    "grade": "low",
    "done": "`cargo test` exits 0 with the lib and cli tests passing, the empty-stdin command prints exactly `{\"words\":0,\"lines\":0,\"chars\":0}`, and README.md contains both the three-line and the JSON example.",
    "deps": ["lib-chars", "bin-main", "cli-tests", "readme-cli"],
    "paths": []
  }
]
```
