Here’s a breakdown of the tasks to turn `tally` into a CLI and library while preserving existing library functions:

---

### Tasks Plan

1. **Add CLI entrypoint and `chars` function**
   - Implement a `main()` function in `src/lib.rs` (or move it to `src/main.rs`) that reads from stdin or a file argument, computes `words`, `lines`, and `chars` (non-whitespace characters), and prints the results in the default format (3 lines).
   - Add a `chars()` function to `src/lib.rs` (if not already present) that counts non-whitespace characters.
   - *Done*: `cargo run -- --help` shows a valid CLI usage, and `cargo run -- --help` prints usage instructions.

2. **Implement `--json` flag**
   - Parse the `--json` flag in `main()` and print the results as a JSON object (e.g., `{"words":0,"lines":0,"chars":0}`).
   - *Done*: `cargo run -- --json` on empty input prints `{"words":0,"lines":0,"chars":0}`.

3. **Add tests for CLI functionality**
   - Write tests for stdin/file input, `--json` flag, and empty input in `tests/` (e.g., `tests/cli.rs`).
   - *Done*: `cargo test` passes, and tests cover all cases.

4. **Update `README.md`**
   - Document both output shapes (default 3-line format and JSON output) in the README.
   - *Done*: `README.md` includes examples for both formats.

5. **Add `chars` to library exports**
   - Ensure `chars()` is exported in `src/lib.rs` (if not already) so it can be used by the CLI and tests.
   - *Done*: `cargo check` passes without warnings for `chars()`.

---

```json
[
  {
    "id": "add_cli_entrypoint",
    "text": "Add a `main()` function in `src/main.rs` that reads from stdin/file, computes `words`, `lines`, and `chars`, and prints the default 3-line output. Add a `chars()` function to `src/lib.rs` if missing.",
    "class": "rust_launch_path",
    "grade": "medium",
    "done": "`cargo run -- --help` prints valid CLI usage and `cargo run -- --help` shows usage instructions.",
    "deps": [],
    "paths": ["src/main.rs", "src/lib.rs"]
  },
  {
    "id": "add_json_flag",
    "text": "Parse the `--json` flag in `main()` and print results as a JSON object. Ensure `chars()` is used for consistency.",
    "class": "batch_scripting",
    "grade": "medium",
    "done": "`cargo run -- --json` on empty stdin prints `{\"words\":0,\"lines\":0,\"chars\":0}`.",
    "deps": ["add_cli_entrypoint"],
    "paths": ["src/main.rs"]
  },
  {
    "id": "write_cli_tests",
    "text": "Write tests for stdin/file input, `--json` flag, and empty input in `tests/cli.rs`.",
    "class": "research",
    "grade": "low",
    "done": "`cargo test` passes and all test cases are covered.",
    "deps": ["add_cli_entrypoint", "add_json_flag"],
    "paths": ["tests/cli.rs"]
  },
  {
    "id": "update_readme",
    "text": "Document both output shapes (default 3-line format and JSON output) in `README.md`.",
    "class": "docs",
    "grade": "low",
    "done": "`README.md` includes examples for both formats.",
    "deps": ["add_cli_entrypoint", "add_json_flag", "write_cli_tests"],
    "paths": ["README.md"]
  }
]
```