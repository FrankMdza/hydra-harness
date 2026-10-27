# Build 358 - `hydra doctor` checks that `/usr/bin/python3` runs `import json, shlex`

Status: resolved
Size: haiku

## Goal

Every compiled gate script in `src/policy/mod.rs` begins with `#!/usr/bin/python3` via `SHEBANG_PYTHON` and imports `json` and `shlex`. Today `src/doctor.rs` checks the data root, the checkout, the policy directory, the four vendor binaries, `hydra` on `PATH`, the policy sha, and the seven protected configs, and never checks `/usr/bin/python3`.

On macOS `/usr/bin/python3` can also exist on disk as an `xcrun` stub that exits non-zero when the Command Line Tools are missing, so checking `Path::is_file` alone is not enough. `hydra doctor` must execute the interpreter with `-c "import json, shlex"` and report whether it is `present`, `missing`, or `broken`, setting `Report::missing = true` on `missing` or `broken`.

## What to change

1. **`src/doctor.rs`:**
   - Add a helper `gate_python_line(out: &mut String, python: &Path, missing: &mut bool)` that checks `python`:
     - If `!python.is_file()`, set `*missing = true` and append `format!("gate python {} missing\n", python.display())`.
     - Otherwise run `std::process::Command::new(python).args(["-c", "import json, shlex"]).output()`. If it exits with `status.success()`, append `format!("gate python {} present\n", python.display())`. Otherwise set `*missing = true` and append `format!("gate python {} broken\n", python.display())`.
   - In `run`, resolve the interpreter path from `std::env::var_os("HYDRA_DOCTOR_PYTHON").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(crate::policy::SHEBANG_PYTHON))` and call `gate_python_line(&mut out, &python, &mut missing)` right after the `path_line(&mut out, "policies", &policies, &mut missing)` line.
   - Add a unit test in `src/doctor.rs` under `#[cfg(test)] mod tests` testing `gate_python_line` on `/usr/bin/python3`, on a missing path, and on `/usr/bin/false`.

2. **`tests/doctor.rs`:**
   - Add an integration test verifying that `hydra doctor` prints `gate python /usr/bin/python3 present` by default, prints `gate python <missing> missing` and exits non-zero when `HYDRA_DOCTOR_PYTHON` points to a missing path, and prints `gate python /usr/bin/false broken` and exits non-zero when `HYDRA_DOCTOR_PYTHON` points to `/usr/bin/false`.

## Done when

- `cargo fmt --all --check` passes.
- `cargo clippy --release --workspace --all-targets -- -D warnings` passes.
- `cargo nextest run --release -E 'binary(doctor)'` and `cargo test --release --lib doctor` pass.

## Outcome

Built. `hydra doctor` prints `gate python <path> present`, `missing` or `broken` right after
the `policies` line, and sets exit 1 on `missing` or `broken`. `broken` means the interpreter
ran `-c "import json, shlex"` and exited non-zero.

- `src/doctor.rs`: `gate_python_line` and the `GATE_PYTHON` constant. `run` reads
  `HYDRA_DOCTOR_PYTHON` and falls back to the constant. Unit test
  `a_gate_python_line_passes_on_a_working_interpreter_and_fails_otherwise` covers
  `/usr/bin/python3`, a missing path and `/usr/bin/false`.
- `tests/doctor.rs`: `doctor_runs_the_gate_python_and_reports_present_missing_or_broken`
  covers the default, a missing path and `/usr/bin/false`, and checks exit status on the
  last two.
- `IMPLEMENTATION.md`: one paragraph under the gate interpreter section.

Measured on this machine: `cargo fmt --all --check` passes. `cargo clippy --release
--workspace --all-targets -- -D warnings` exits 0. `cargo nextest run --release -E
'binary(doctor)'` runs 13 tests and passes all 13. `cargo test --release --lib doctor`
runs 3 tests and passes all 3.

Assumption that was false: the ticket names `crate::policy::SHEBANG_PYTHON`. No such
constant exists. The interpreter appears as a literal in `src/policy/mod.rs`, in
`src/policy/replay.rs` as `GATE_PYTHON`, and in the shebang of `src/policy/gate.py`. The
ticket said to touch nothing it did not name, so `doctor.rs` keeps its own `GATE_PYTHON`
constant, and the four copies are not unified.
