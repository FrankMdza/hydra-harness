# 295 - Dev debuginfo drops to line tables, measured before and after

Type: build
Status: resolved
Blocked by:
Opened: 2026-10-13, by the session, from `~/cerebro/research/security-and-compile.md` section 2,
change 1, which the human put on Hydra's list on 2026-10-12.

## What

Cargo's dev default is `debug = true`, full debuginfo. This workspace links 59 test binaries, each
against a 78 MB `libhydra.rlib`, and most of what the linker copies is debuginfo nobody reads: a
panic in a test names its file and line without it, and nobody here steps through a Hydra test in
lldb. `line-tables-only` keeps backtraces with file and line and drops the rest; dependencies get
no debuginfo at all. The research note says this is the change most likely to cut both link time
and the size of `target/debug`. This ticket makes the change and measures it, because a compile
speed claim that is only read is the kind map ticket 34 warned about.

## Do

1. In the root `Cargo.toml`:

   ```toml
   [profile.dev]
   debug = "line-tables-only"

   [profile.dev.package."*"]
   debug = false
   ```

2. Measure before and after, in this worktree, from a clean `target/`: `cargo clean`, then
   `cargo build --all-targets --timings`, and record from the timings report the total wall time,
   plus `du -sh target/debug`. Twice: once on the tree as you found it, once with the change. The
   second build is the one that matters; the first is the baseline. Report both numbers and the
   machine's load if something else was building at the time.
3. Confirm a backtrace still names files: `RUST_BACKTRACE=1 cargo test --lib <any test that you
   make fail on purpose, then restore>` shows `src/...rs:<line>` frames.
4. One sentence in `CLAUDE.md` under "The build": the dev profile carries line tables only, and
   `CARGO_PROFILE_DEV_DEBUG=2 cargo build` restores full debuginfo for a session that needs the
   debugger. One paragraph in `IMPLEMENTATION.md` with the two numbers.

## Done when

- The two timings and the two sizes are in the Outcome and in `IMPLEMENTATION.md`; fmt, clippy
  and `cargo test --workspace -- --test-threads=1` clean on the changed profile; the backtrace
  check in the report.

## Outcome

Built by `codex` `gpt-6.1-sol` on Hydra run `01a13402-9d49-7411-8f95-23835693536b` under
`standard`, 1.31M tokens, applied to master on 2026-10-13 after the diff was read whole. The
profile change and the `CLAUDE.md` sentence are the builder's; the numbers below are the
session's, because the builder's pair (25.2 s before, 29.1 s after, load average 16 at the end)
ran while build 293's test suite and a panel seat were on the same 8 GB, and it said so.

Re-measured by the session in the same worktree with one other run on the machine, `cargo
clean` then `cargo build --all-targets`, wall clock:

| Profile | clean build | `target/debug` |
|---|---|---|
| dev defaults | 23 s | 1.3 GB |
| line tables, no dependency debuginfo | 22 s | 1.1 GB |

The time claim from the research note did not hold: one second is noise. The disk claim held at
15 percent. The change stays because it costs nothing and panics still name their frames, which
the builder checked with a deliberately failing test (`src/paths.rs:450:9` in the backtrace,
test restored). The builder stopped short of resolving because three `run::tests` failed in its
worktree; they failed on the original profile too and pass on master, and the cause is the
run's own environment, which hands every run `GIT_OBJECT_DIRECTORY` and
`GIT_ALTERNATE_OBJECT_DIRECTORIES` from the allowlist. Build 293's builder hit the same thing.
A test that spawns `git` inside a gated run inherits a pointer to the parent's objects, so a
ticket whose suite must be green inside a run has to say `env -u` on both, and 296's task text
does.
