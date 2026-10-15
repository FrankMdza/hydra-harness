# 315 - Hydra compiles on Linux: a `/proc` start-time reader and a CI job that proves it

Type: build
Status: resolved
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

## Outcome

**The list was one error.** `rustup target add x86_64-unknown-linux-gnu`, then `cargo check
--workspace --all-targets --target x86_64-unknown-linux-gnu`: `E0425, cannot find value
KERN_PROC_PID in crate libc` at `src/procinfo.rs:24`, in the lib and the lib test. With that
fixed, the next check surfaced a single dead-code warning, `BUILD_CACHE` in `worktree.rs`, now
gated `macos` like the two functions that read it. The end state is a clean cross check, and
`cargo clippy --workspace --all-targets --target x86_64-unknown-linux-gnu -- -D warnings`
is clean too. No site needed the `bail!("not built on linux yet, map 119")` body.

**`procinfo.rs`.** The Darwin `start_time`, its two sysctl constants and `TimeVal` sit behind
`cfg(target_os = "macos")`. The Linux `start_time` reads `/proc/<pid>/stat` (NotFound or ESRCH
is `Ok(None)`), `btime` from `/proc/stat` and `sysconf(_SC_CLK_TCK)`, then calls
`start_from_stat(line, btime, hz)`, a pure function compiled on Linux and under `test`. It
splits at the last `)` and takes index 19 of what follows. `is_alive` is shared. Three unit
tests run on macOS: a `sleep` line at 100 Hz giving `btime + 8123 s + 450 ms`, the same line
with the command name `(a b) S 1 2 3 (x))`, and refusals for an unclosed name, a short line
and a non-numeric field 22. `btime` is whole seconds and the tick count is fixed, so two reads
of one process give the same instant, which `is_alive`'s exact equality needs.

**`clonefile` was already handled.** Build 309 left a `cfg(not(target_os = "macos"))`
`clone_build_cache` that returns `Ok(())`, meaning no clone at all, and I kept it. A recursive
copy of `target/` would cost gigabytes of real writes per run on a filesystem without reflinks,
which is more than what it saves, and `cp --reflink` on btrfs or XFS is a later ticket if a
Linux VM ever runs builds.

**CI.** `.github/workflows/ci.yml` gained a `linux` job on `ubuntu-latest`: the same
`HYDRA_ROOT` scratch root, `cargo build --release --workspace`, then `cargo test --release
--workspace --no-fail-fast -- --skip sandbox`. The macOS job is unchanged. Excluded: every
test whose path holds `sandbox`, which is the profile unit tests in `src/sandbox.rs` and
`tests/spawn.rs::a_spawn_writes_a_sandbox_profile_that_denies_hydra_paths`, because they
describe a wrapper Linux does not apply. The four `tests/sandbox_*.rs` files were already
`#![cfg(target_os = "macos")]` and compile to nothing there. The launchd template tests skip
on their own without `plutil`, and the evolve admission tests skip without `exec_available()`.

**Measured on macOS, arm64, Darwin 25.2.0.** `cargo nextest run --release --workspace`:
1391 passed, 4 skipped, 63.6 s. The six `procinfo` tests pass, `our_own_start_time_matches_ps`
included, so the Darwin body still reads what `ps` reads.

**What the ticket assumed that was false.** It expected `worktree.rs` to be a second error site;
it was already gated and compiles. And the job's test step has never run: nothing in this
session ran a Linux binary, so whether `--skip sandbox` is a sufficient filter is unmeasured.
The step fails hard, so the first push to GitHub gives the answer, and any test that fails
there is this ticket's unfinished list rather than a reason to widen the filter blind.
