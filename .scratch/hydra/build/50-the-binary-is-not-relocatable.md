# 50 - The binary is not relocatable and nothing says so at runtime

Type: build
Status: open
Blocked by:

## What

`policies/` and `config/` resolve through `env!("CARGO_MANIFEST_DIR")`, baked in at compile
time. `strings target/release/hydra | grep -c <checkout path>` returns 3. Copy the binary
anywhere and move or rename the source directory, and it fails on a path nobody told the user
about. The README now says so; the binary does not.

Make the resolution explicit and checkable: keep the compile-time path as the default, let an
environment variable or a flag override it, and have the launch refuse with a sentence naming
the missing directory rather than failing deeper in. `hydra status` should report where it read
the policy set from, beside the root it already prints. Found by the cold-clone review,
`docs/reviews/2026-09-14-first-ten-minutes.md`.
