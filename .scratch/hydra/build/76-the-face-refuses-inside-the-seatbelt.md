# 76 - The delegate face refuses in Hydra's words from inside a seatbelt

Type: build
Status: resolved
Blocked by:

## What

Map ticket 64, resolved 2026-09-22: delegation from inside a macOS run is not available, because
the delegated child is Hydra itself and the profile the run inherited denies it the corpus. Today
a `delegate` call from inside a run fails on the first corpus read with "Operation not permitted"
inside a tool result, which is the kernel's sentence and not Hydra's. `sandbox::inside_seatbelt`
says whether the face's process is inside one.

## Done when

`face::answer` checks `sandbox::inside_seatbelt()` before it calls `run::run` and answers a tool
error whose text starts with a `hydra/` code (`hydra/seatbelt-delegation` or similar, added to
`normalize::GATE_CODES` so the reader knows it) saying that delegation from inside a macOS run is
not available, that the orchestrator outside the run is where a child is launched from, and that
map ticket 64 holds the decision. The refusal reaches the parent's record as the tool result the
provider streamed back, the way every face answer does. `tests/face.rs` drives the built binary
under `/usr/bin/sandbox-exec` with a minimal `(allow default)` profile and reads the refusal back;
the same request outside a profile still reaches `run::run` (the existing tests cover it).
`SECURITY.md`'s delegation bullet and the face section of `IMPLEMENTATION.md` say the refusal is
Hydra's now; harness spec 9 gets one sentence. This ticket may edit `SECURITY.md` and that one
sentence of `docs/harness-spec.md` and nothing else under `docs/`.

## Outcome

`face::call` checks `sandbox::inside_seatbelt()` after argument parsing and before `run::run`,
answering `hydra/seatbelt-delegation` with map ticket 64's decision. `seatbelt-delegation` is on
`normalize::GATE_CODES`; the unit test there now reads the delegate face as a fourth source beside
`gate.py` and the three hook wrappers. `tests/face.rs::delegation_from_inside_a_seatbelt_is_refused_in_hydras_words`
drives the built binary under `/usr/bin/sandbox-exec` when outside a seatbelt, or through the
inherited seatbelt when the test process is already inside one, because nested `sandbox-exec` is
refused on this machine. `SECURITY.md`, harness spec 9 and `IMPLEMENTATION.md` updated. Measured
with `cargo test --release --test face`: 8 passed. `cargo test --release --lib
the_known_codes_are_exactly_the_ones_the_gate_and_its_wrappers_write`: 1 passed. `cargo clippy
--all-targets -- -D warnings`: clean. The ticket assumed the check belonged before argument parsing;
after `read()` is correct so invalid provider and unknown-argument refusals still reach the model
when the face is inside a seatbelt.

Built by the Claude Code session by hand, with no agent run, so there is no run id or
token count.
