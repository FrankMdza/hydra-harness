The library already has `words` and `lines`, and the goal forbids rewriting them, so the first task only adds `chars` beside them with its own unit test. Everything else hangs off that.

The binary is the one piece with real decisions in it, so it gets its own task and a medium grade. I'd keep it dependency-free. The JSON object has three fixed keys and three integers, so a `format!` produces it exactly, and pulling in serde for that is more than the job needs. Argument parsing is the same story: one flag and one optional path, handled from `std::env::args` by hand. The goal says "three lines" without fixing their shape, so the plan fixes it here as `words: N`, `lines: N`, `chars: N`, in that order. The tests and the README both depend on that choice, and writing it down once stops them from drifting apart.

Tests and README can run in parallel once the binary exists. The CLI tests go in `tests/cli.rs` and drive the built binary through `env!("CARGO_BIN_EXE_tally")`, so they exercise the real stdin and file paths rather than a refactored helper. The README task copies output from a real run instead of typing it.

Four tasks. The last two touch disjoint files, so neither waits on the other.

```json
[
  {
    "id": "lib-chars",
    "text": "Add `pub fn chars(text: &str) -> usize` to src/lib.rs that counts non-whitespace characters (`text.chars().filter(|c| !c.is_whitespace()).count()`), with a doc comment in the style of the others. Add a unit test in the existing `tests` module covering \"\", \"  \\n\", \"ab c\" (3) and a multibyte case such as \"héllo\" (5). Do not change `words`, `lines` or `bump`, or their tests.",
    "class": "rust_off_launch_path",
    "grade": "low",
    "done": "`cargo test --lib` passes, including the new chars test; `git diff src/lib.rs` shows only additions",
    "deps": [],
    "paths": ["src/lib.rs"]
  },
  {
    "id": "bin-main",
    "text": "Create src/main.rs for a binary named `tally` (add an explicit `[[bin]] name = \"tally\" path = \"src/main.rs\"` to Cargo.toml). Parse `std::env::args` by hand, with no new dependencies: an optional `--json` flag and at most one positional file path, in any order. Read the whole file if a path is given, otherwise all of stdin. Compute counts with `tally::words`, `tally::lines` and `tally::chars`. Default output is exactly three lines: `words: N`, `lines: N`, `chars: N`. With `--json`, print one line `{\"words\":N,\"lines\":N,\"chars\":N}` built with `format!`, keys in that order, no spaces. An unreadable file prints `tally: <path>: <error>` to stderr and exits 1. An unknown flag or a second path prints a one-line usage to stderr and exits 2.",
    "class": "rust_off_launch_path",
    "grade": "medium",
    "done": "`printf '' | cargo run --quiet -- --json` prints `{\"words\":0,\"lines\":0,\"chars\":0}`; `printf 'a b\\nc\\n' | cargo run --quiet` prints `words: 3`, `lines: 2`, `chars: 3`; `cargo run --quiet -- /nonexistent` exits 1",
    "deps": ["lib-chars"],
    "paths": ["src/main.rs", "Cargo.toml"]
  },
  {
    "id": "cli-tests",
    "text": "Add tests/cli.rs with integration tests that spawn the built binary via `std::process::Command::new(env!(\"CARGO_BIN_EXE_tally\"))`. Cover: text on stdin with the three-line output; a file argument (write a temp file under `std::env::temp_dir()` with a unique name, no new dev-dependencies) with the three-line output; `--json` on non-empty stdin; empty stdin in both shapes, asserting the JSON is exactly `{\"words\":0,\"lines\":0,\"chars\":0}`; and a missing file exiting with status 1. Assert on full stdout, not substrings.",
    "class": "rust_off_launch_path",
    "grade": "low",
    "done": "`cargo test` is green and `cargo test --test cli -- --list` shows at least five tests",
    "deps": ["bin-main"],
    "paths": ["tests/cli.rs"]
  },
  {
    "id": "readme-cli",
    "text": "Add a `## Command line` section to README.md after the existing paragraph. Show reading stdin and reading a file, then both output shapes: the three-line default and the `--json` object. Generate every example output by actually running the binary (`echo ... | cargo run --quiet` and `cargo run --quiet -- --json`) and paste the real output. Say in one sentence that `chars` counts non-whitespace characters. Leave the existing paragraph alone.",
    "class": "docs",
    "grade": "low",
    "done": "README.md has a `## Command line` section containing a `words: ` / `lines: ` / `chars: ` block and a `{\"words\":` JSON line, and each shown output matches what the binary prints for the shown input",
    "deps": ["bin-main"],
    "paths": ["README.md"]
  }
]
```
