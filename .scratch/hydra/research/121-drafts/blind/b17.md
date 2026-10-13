Here’s a breakdown of the tasks to turn `tally` into a CLI and library while preserving the existing library functions. The goal is modular, with dependencies flowing logically.

---

### Tasks

1. **Add CLI entry point and basic CLI parsing**
   - Implement a `main()` function in `src/main.rs` that reads from stdin or a file argument and delegates to the library functions.
   - Add a `parse_args()` function to handle `--json` flag and file/stdin input.
   - "Done": `cargo run -- --help` prints usage, and `cargo run -- --json` on empty input prints `{"words":0,"lines":0,"chars":0}`.

2. **Add `chars` counting to the library**
   - Modify `src/lib.rs` to include a `chars` function that counts non-whitespace characters.
   - "Done": `cargo test` passes, and `tally::chars("")` returns `0`.

3. **Implement JSON output in the CLI**
   - Update `main()` to conditionally print JSON when `--json` is passed.
   - "Done": `cargo run -- --json` on empty input prints `{"words":0,"lines":0,"chars":0}`.

4. **Write tests for CLI functionality**
   - Add tests for stdin, file input, `--json`, and empty input in `tests/cli.rs`.
   - "Done": `cargo test` passes all tests, including edge cases.

5. **Update `README.md` to document CLI and JSON output**
   - Add examples for both text and JSON output formats.
   - "Done": `README.md` includes clear examples for both shapes.

---

```json
[
  {
    "id": "add-cli-entry-point",
    "text": "Create `src/main.rs` with a `main()` function that reads from stdin or a file argument and delegates to `tally::words`, `tally::lines`, and `tally::chars`. Add a `parse_args()` function to handle `--json` and file/stdin input.",
    "class": "rust_launch_path",
    "grade": "medium",
    "done": "`cargo run -- --help` prints usage, and `cargo run -- --json` on empty input prints `{\"words\":0,\"lines\":0,\"chars\":0}`",
    "deps": [],
    "paths": ["src/main.rs"]
  },
  {
    "id": "add_chars_to_library",
    "text": "Extend `src/lib.rs` with a `chars` function that counts non-whitespace characters. Ensure it works with empty input.",
    "class": "docs",
    "grade": "low",
    "done": "`cargo test` passes, and `tally::chars(\"\")` returns `0`",
    "deps": ["add-cli-entry-point"],
    "paths": ["src/lib.rs"]
  },
  {
    "id": "json_output_in_cli",
    "text": "Modify `src/main.rs` to print JSON when `--json` is passed. Use the existing library functions to construct the JSON object.",
    "class": "batch_scripting",
    "grade": "medium",
    "done": "`cargo run -- --json` on empty input prints `{\"words\":0,\"lines\":0,\"chars\":0}`",
    "deps": ["add-chars-to-library"],
    "paths": ["src/main.rs"]
  },
  {
    "id": "write_cli_tests",
    "text": "Write tests for stdin, file input, `--json`, and empty input in `tests/cli.rs`. Ensure all edge cases are covered.",
    "class": "research",
    "grade": "high",
    "done": "`cargo test` passes all tests, including edge cases",
    "deps": ["json-output-in-cli"],
    "paths": ["tests/cli.rs"]
  },
  {
    "id": "update_readme",
    "text": "Update `README.md` to document both text and JSON output formats with clear examples.",
    "class": "docs",
    "grade": "low",
    "done": "`README.md` includes examples for both shapes",
    "deps": ["json-output-in-cli"],
    "paths": ["README.md"]
  }
]
```