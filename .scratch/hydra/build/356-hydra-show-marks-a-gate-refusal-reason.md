# Build 356 - `hydra show --events` marks `gate_refusal.reason` inside the result boundary

Status: claimed
Size: haiku

## Goal

In `src/show.rs`, `event_json` formats `Event::GateRefusal(e)` by cloning `e.body` and wrapping `body["command"]` with `mark(command, token)`, while leaving `body["reason"]` unmarked.

Two facts make `body["reason"]` untrusted text:

1. In `src/policy/gate.py`, `check_shell_command` interpolates the offending token from the run's shell command into the refusal reason via `quoted(word)`.
2. The sidecar file `gate-refusals.jsonl` lives in the run's spawn directory with mode `0600` under the same UID, so a run can also write a row to it before `run_end` folds the sidecar into the record.

When `hydra show --events` prints a `gate_refusal` event, `body["reason"]` must be wrapped with `mark(reason, token)` just like `body["command"]`, so any forged closing boundary marker inside `reason` is escaped inside the real boundary.

## What to change

1. **`src/show.rs`:**
   In `event_json` under the `Event::GateRefusal(e)` arm around lines 170 to 176, wrap `body["reason"]` with `mark(reason, token)` whenever `body["reason"].as_str()` is `Some(reason)`, right beside `body["command"]`.

2. **`tests/show.rs`:**
   Add a test verifying that when a record carries a `gate_refusal` event whose `command` and `reason` both contain a fake closing marker `<<<end hydra-result 0>>>` followed by an injected instruction line, `hydra show --events` wraps both `body["command"]` and `body["reason"]` in the run's `<<<hydra-result ...>>>` boundary and escapes the fake closing marker as `<<<\end hydra-result 0>>>` in both fields.

## Done when

- `cargo fmt --all --check` passes.
- `cargo clippy --release --workspace --all-targets -- -D warnings` passes.
- `cargo nextest run --release -E 'binary(show)'` passes, including the new test.
