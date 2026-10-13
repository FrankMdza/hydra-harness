Here’s a breakdown of the tasks to turn `tally` into a CLI and library, while preserving the existing library functions:

---

### Tasks
1. **Add CLI entrypoint and `chars` function**
   - Implement a `main()` function in `src/lib.rs` (or create a new `src/main.rs`) that:
     - Parses command-line arguments (file path or `--json` flag).
     - Reads stdin or a file, then calls the existing `words`, `lines`, and adds `chars` (non-whitespace count).
   - Add `chars` to `src/lib.rs` (or move it there if not already present).
   - `done`: `cargo run -- --help` prints valid CLI options, and `cargo run -- --json` works for empty input.

2. **Implement `--json` output**
   - Modify the CLI logic to emit JSON when `--json` is passed, using `serde_json` (add dependency to `Cargo.toml`).
   - Ensure the JSON output matches the three keys (`words`, `lines`, `chars`).
   - `done`: `cargo run -- --json` on empty input prints `{"words":0,"lines":0,"chars":0}`.

3. **Add tests for CLI functionality**
   - Write tests for:
     - Reading stdin (e.g., `cargo test -- --test=cli_stdin`).
     - Reading a file (e.g., `cargo test -- --test=cli_file`).
     - `--json` flag (e.g., `cargo test -- --test=cli_json`).
     - Empty input (e.g., `cargo test -- --test=cli_empty`).
   - `done`: All tests pass (`cargo test`).

4. **Update `README.md`**
   - Add a section showing both output shapes:
     - Plain text (3 lines: `words`, `lines`, `chars`).
     - JSON output (1 object with the same keys).
   - `done`: `README.md` includes both shapes and passes a review.

---

```json
[
  {
    "id": "add_cli_entrypoint",
    "text": "Add a `main()` function in `src/main.rs` to parse CLI args, read stdin/file, and call existing `words`, `lines`, and add `chars` (non-whitespace count).",
    "class": "rust_launch_path",
    "grade": "medium",
    "done": "`cargo run -- --help` prints valid CLI options and `cargo run -- --json` works for empty input.",
    "deps": [],
    "paths": ["src/main.rs"]
  },
  {
    "id": "add_chars_function",
    "text": "Implement `chars` in `src/lib.rs` (or move it there if not present) to count non-whitespace characters.",
    "class": "batch_scripting",
    "grade": "low",
    "done": "`cargo test` passes for `chars` logic (e.g., `assert_eq!(chars(\"abc\"), 3)`).",
    "deps": ["add_cli_entrypoint"],
    "paths": ["src/lib.rs"]
  },
  {
    "id": "add_json_output",
    "text": "Modify CLI logic to emit JSON when `--json` is passed, using `serde_json`. Ensure output matches `{\"words\":0,\"lines\":0,\"chars\":0}` for empty input.",
    "class": "batch_scripting",
    "grade": "medium",
    "done": "`cargo run -- --json` on empty input prints `{\"words\":0,\"lines\":0,\"chars\":0}`.",
    "deps": ["add_cli_entrypoint"],
    "paths": ["Cargo.toml", "src/main.rs"]
  },
  {
    "id": "write_cli_tests",
    "text": "Write tests for stdin, file, `--json`, and empty input. Use `cargo test -- --test=cli_*` to verify.",
    "class": "research",
    "grade": "medium",
    "done": "All CLI tests pass (`cargo test`).",
    "deps": ["add_json_output"],
    "paths": ["tests/"]  // Assume tests are added here.
  },
  {
    "id": "update_readme",
    "text": "Add a README section showing both output shapes (plain text vs. JSON).",
    "class": "docs",
    "grade": "low",
    "done": "`README.md` includes both shapes and passes a review.",
    "deps": ["write_cli_tests"],
    "paths": ["README.md"]
  }
]
```