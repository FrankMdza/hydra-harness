# 138 - Approved panel shapes are protected config six

Type: build
Status: open
Blocked by:

## What

Map 82, answer 2. A session may ask its launcher for seats without a prompt only when the request
matches a shape the human approved. Shapes live in `config/panels.json`, protected config #6 in
harness spec 15.1, written by the human only. This ticket builds the file's format, its loader
and its protection. Nothing launches from it yet; 140 does that.

## Do

Tests first, each shown failing before the code:

1. A loader test: a fixture `panels.json` with one shape parses into typed seats (provider from
   the four, model optional, policy by name from the loaded set, role free text), a cap per hour,
   and a list of session policies allowed to ask. Unknown provider, unknown policy, a seat whose
   policy grants more than any allowed asker's policy (answer 3: never more than the session), or
   an unknown field refuse to load with the field named. A missing file loads as no shapes.
2. A gate test in `tests/gate.rs`: under every policy, a write to `config/panels.json` is refused,
   like the other five (`denied_write_paths` near `src/policy/mod.rs:572`).
3. A seatbelt test in `tests/spawn.rs`, like the existing protected-config rows: the kernel denies
   the write.
4. `hydra doctor` lists `panels.json` as present or missing beside the other five
   (harness spec 2, near line 141).

Then:
- `config/README.md` gets the format with one example shape, the fixed three-seat panel
  (claude opus `judge` reviewer, codex `standard` adversary, cursor-agent composer-2.5 `judge`
  security). Do not create `config/panels.json` itself: the human writes it.
- Harness spec 15.1 becomes six configs, with the failure mode "a run widens which seats it may
  summon without a human"; 15.2's count follows. `IMPLEMENTATION.md` in the same change.

"Never more than the session" needs an order over policies. If the policy set has none you can
compute from the grants (a policy's grants are a subset of another's), say so and stop: that is
a decision.

## Outcome
