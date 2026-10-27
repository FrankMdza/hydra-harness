# Build 361 - `hydra doctor --walk` classifies `RejectedBeforeHook` rows by tool and error shape

Status: claimed
Size: haiku

## Goal

Build 334 separated `Finding::RejectedBeforeHook { call_id, tool, error }` from `Finding::Ungated` on `claude`, and left a note in `IMPLEMENTATION.md` saying the machine-wide walk breakdown required the driving session because a worktree run cannot read `~/hydra/runs/`.

Today `src/walk.rs` counts only `rejected_before_hook: usize` and `src/main.rs` prints `RejectedBeforeHook: 58; Ungated: 4` with no breakdown of which tools and error shapes make up those 58 rows. Adding a per-shape map `rejected_by_shape: BTreeMap<String, usize>` to `walk::Report` and printing it in `hydra doctor --walk` makes the walk self-describing on any corpus without needing manual inspection of `~/hydra/runs/`.

## What to change

1. **`src/walk.rs`:**
   - Add `pub rejected_by_shape: BTreeMap<String, usize>` to `Report`.
   - Add a helper `rejected_shape_label(tool: &str, error: &str) -> String` that classifies the first error line into a concise bucket prefixed by `tool`:
     - If `error.contains("InputValidationError")`: `format!("{tool}: InputValidationError")`
     - Else if `error.contains("No such tool available")`: `format!("{tool}: No such tool available")`
     - Else if `error.starts_with("<tool_use_error>Blocked:")`: `format!("{tool}: <tool_use_error>Blocked")`
     - Else if `error.starts_with("<tool_use_error>File has not been read yet")`: `format!("{tool}: <tool_use_error>File has not been read yet")`
     - Else if `error.starts_with("<tool_use_error>File does not exist")`: `format!("{tool}: <tool_use_error>File does not exist")`
     - Else if `error.starts_with("<tool_use_error>String to replace not found")`: `format!("{tool}: <tool_use_error>String to replace not found")`
     - Else if `error.starts_with("<tool_use_error>No task found with ID")`: `format!("{tool}: <tool_use_error>No task found with ID")`
     - Else if `error.starts_with("<tool_use_error>Error calling tool")`: `format!("{tool}: <tool_use_error>Error calling tool")`
     - Else if `error.starts_with("<tool_use_error>Sibling tool call errored")`: `format!("{tool}: <tool_use_error>Sibling tool call errored")`
     - Else if `error.starts_with("Not run: the response that made this tool call was stopped")`: `format!("{tool}: safety classifier stopped call")`
     - Else: `format!("{tool}: {}", error.chars().take(80).collect::<String>())`
   - In `walk_paths`, when matching `NormalizeFinding::RejectedBeforeHook { tool, error, .. }`, increment `*rejected_by_shape.entry(rejected_shape_label(&tool, &error)).or_insert(0) += 1`.

2. **`src/main.rs`:**
   Right after `println!("RejectedBeforeHook: {}; Ungated: {}", report.rejected_before_hook, report.ungated);`, iterate over `&report.rejected_by_shape` and print `println!("  RejectedBeforeHook {shape}: {count}");`.

3. **`tests/doctor.rs`:**
   In `walk_counts_an_edit_rejected_before_the_hook_separately`, also assert that `stdout` contains `"  RejectedBeforeHook Edit: <tool_use_error>String to replace not found: 1"`.

## Done when

- `cargo fmt --all --check` passes.
- `cargo clippy --release --workspace --all-targets -- -D warnings` passes.
- `cargo nextest run --release -E 'binary(doctor)'` passes.
