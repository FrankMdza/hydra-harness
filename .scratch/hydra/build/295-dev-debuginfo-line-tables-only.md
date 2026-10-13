# 295 - Dev debuginfo drops to line tables, measured before and after

Type: build
Status: open
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

