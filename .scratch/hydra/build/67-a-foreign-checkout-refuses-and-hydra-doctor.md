# 67 - A launch from a foreign checkout refuses, and `hydra doctor` says what a launch would find

Type: build
Status: resolved
Blocked by: 66

## What

Map ticket 61, resolved 2026-09-21.

1. `hydra run` from a checkout that is not the data root, with `HYDRA_ROOT` unset, refuses before
   writing anything, in Hydra's words: the root it would have written into, the checkout it was
   run from, and that setting `HYDRA_ROOT`, to that root or another, is what says the writer
   meant it. The check lives beside the other pre-flight checks in `src/run.rs`, reads
   `Layout::discover` and `paths::hydra_home`, and never writes a record, because a refusal that
   writes into the root is the surprise itself. `tests/spawn.rs` drives it with a scratch layout
   and a scratch checkout.
2. `hydra doctor`, the twelfth command, prints: the data root and the policy path with whether
   each exists; the four vendor binaries with the version each reports and the version the code
   was measured against, from one table in `src/provider/mod.rs` that `README.md` and
   `tests/drift.rs` can share; the policy set sha; and each of the five protected configs with
   present or missing. Exit 0 when nothing is missing, 1 otherwise. It launches nothing and runs
   only `--version` on each binary. `src/cli.rs` gains the command and the help text says what it
   does not do.

Do not edit `docs/`, `README.md`, `IMPLEMENTATION.md` or `.scratch/`.

Verify with `cargo build --release`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt
--check` and `cargo test --release --test spawn --test status`. Do not run the whole suite.

## Outcome

Resolved 2026-09-21. `cursor-agent` on Composer 2.5, run 01a0c61d-0639 under `standard`, 1.9M
tokens: `refuse_foreign_checkout` in `run.rs`, called after `Layout::discover` and before
`layout.ensure`, refuses when `HYDRA_ROOT` is unset and the checkout is not the root, naming both
and the variable, and writes nothing; `src/doctor.rs` and the `Doctor` command print the root and
policy path, the four binaries against `provider::MEASURED_VERSIONS`, the policy sha and the five
protected configs, exit 1 when anything is missing. Applied by copying its seven files. Run once
on this machine: exit 1, because `env-allowlist.json` and `export-allowlist.json` are missing,
which is the truth. `spawn` 20, `status` 15. Nothing the ticket assumed turned out false; the
interactive launch does not go through the refusal yet and the outcome says so.
