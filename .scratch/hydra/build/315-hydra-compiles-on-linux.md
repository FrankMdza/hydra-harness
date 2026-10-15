# 315 - Hydra compiles on Linux: a `/proc` start-time reader and a CI job that proves it

Type: build
Status: claimed
Blocked by:
Opened: 2026-10-15, by the session, from map 119's answer. The panel found that Hydra has never
been built for Linux and does not build there: `src/procinfo.rs` line 24 calls `sysctl` with
`libc::KERN_PROC_PID`, a Darwin constant, and the module has no `cfg(target_os)` gate. `src/lib.rs`
declares it for every target and `live.rs`, `watch.rs` and `interactive.rs` call it, so every
liveness answer goes through it. Map 119 wants runs on a Linux VM one day; nothing on that path
starts until `cargo build` succeeds there.

## What

`procinfo.rs` exposes two functions, `start_time(pid) -> Result<Option<DateTime<Utc>>>` and
`is_alive(pid, recorded_start) -> Result<bool>`. Keep the signatures and give them a Linux body:
field 22 of `/proc/<pid>/stat` is the start time in clock ticks since boot, `btime` in
`/proc/stat` is the boot time in seconds since the epoch, and `sysconf(_SC_CLK_TCK)` is the tick
rate. Parse the stat line from the right of the last `)`, because the command name in field 2
may contain spaces and parentheses. A missing `/proc/<pid>` is `Ok(None)`, the same answer the
Darwin body gives for a pid that is gone.

Everything else that is Darwin-only has to be found: `grep -rn 'libc::' src crates` and
`grep -rn 'cfg(target_os' src crates` list the candidates. `sandbox.rs` already gates on macOS
(`wraps_launches()`, `inside_seatbelt()`); `worktree.rs`'s `clonefile` from build 309 is a
likely second site and needs a fallback (a plain recursive copy or no clone at all on Linux, say
which in the Outcome and why). Do not change what any of these do on macOS.

## Do

1. `rustup target list --installed`; if `x86_64-unknown-linux-gnu` is missing, `rustup target add
   x86_64-unknown-linux-gnu`. Then `cargo check --workspace --all-targets --target
   x86_64-unknown-linux-gnu` and collect every error. That is the list of this ticket.
2. Gate the Darwin body of `procinfo.rs` behind `#[cfg(target_os = "macos")]` and write the Linux
   body behind `#[cfg(target_os = "linux")]`, with the parse of the stat line as a pure function
   that takes the line text and the two numbers, so it has unit tests that run on macOS with
   sample lines (one with a command name holding spaces and a `)`).
3. Fix the rest of the list the same way, smallest change that compiles. Where a macOS-only
   call has no Linux counterpart worth writing today, the Linux body returns an error that names
   the ticket (`bail!("not built on linux yet, map 119")`), never a silent success.
4. `.github/workflows/ci.yml`: add a job on `ubuntu-latest` that runs `cargo build --release
   --workspace` and `cargo test --release --workspace --no-fail-fast -- --skip sandbox` or an
   equivalent filter; list in the Outcome which tests you excluded and why (the seatbelt tests
   cannot pass without `sandbox-exec`). Keep the macOS job as it is.
5. `cargo check --target x86_64-unknown-linux-gnu` clean at the end, and the macOS suite green.

## Done means

The cross check passes; the unit tests of the stat parser; `cargo fmt --all --check`, `cargo
clippy --workspace --all-targets -- -D warnings`, `cargo nextest run --release --workspace` and
`cargo test --release --workspace --doc` green on macOS. Amend `IMPLEMENTATION.md` where it says
what platforms Hydra runs on, or add the sentence if none does: Linux compiles and is untested
at runtime, `wraps_launches()` is false there, and recorded runs stay on the Mac (map 119).

Rules inside the run: relative paths only, never look outside your own worktree, write files with
your file tool and not with shell heredocs, and the test suite is green inside a run since build
302 without any `env -u`.
