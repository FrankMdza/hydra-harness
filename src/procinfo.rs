//! The kernel's start time for a pid.
//!
//! Contract 5.2: a run is alive when a process matching both the pid and the start
//! time still exists. The start time is not optional, because macOS recycles pids
//! and a bare pid can point at something unrelated an hour later.
//!
//! `libc` carries no `kinfo_proc` for Apple targets, so this reads the sysctl
//! buffer directly. `p_starttime` sits at offset 0: `kinfo_proc` opens with
//! `kp_proc`, `extern_proc` opens with a union whose second arm is that `timeval`.
//! Verified on Darwin 25.2.0 against `ps -o lstart=`, and the test below keeps it
//! verified.
//!
//! On Linux the same answer comes from `/proc`: field 22 of `/proc/<pid>/stat`
//! counts clock ticks since boot, and `btime` in `/proc/stat` is the boot in
//! seconds since the epoch. Build ticket 315, which compiles it and has not run
//! it against a live Linux kernel.

use anyhow::{Result, bail};
use chrono::{DateTime, TimeZone, Utc};

#[cfg(target_os = "macos")]
const CTL_KERN: libc::c_int = 1;
#[cfg(target_os = "macos")]
const KERN_PROC: libc::c_int = 14;

/// Reads the start time the kernel recorded for `pid`.
///
/// Returns `Ok(None)` when no such process exists, which is the answer a liveness
/// check wants rather than an error.
#[cfg(target_os = "macos")]
pub fn start_time(pid: u32) -> Result<Option<DateTime<Utc>>> {
    let mut mib = [CTL_KERN, KERN_PROC, libc::KERN_PROC_PID, pid as libc::c_int];
    let mut len: libc::size_t = 0;

    // First call sizes the buffer. A live pid gives one kinfo_proc, 3888 bytes on
    // this machine; a dead one gives zero.
    // SAFETY: `mib` is a four-element stack array; `len` is a live `size_t` out-param
    // and `oldp` is null so this call only sizes the buffer.
    let rc = unsafe {
        libc::sysctl(
            mib.as_mut_ptr(),
            mib.len() as libc::c_uint,
            std::ptr::null_mut(),
            &mut len,
            std::ptr::null_mut(),
            0,
        )
    };
    if rc != 0 {
        let err = std::io::Error::last_os_error();
        if err.raw_os_error() == Some(libc::ESRCH) {
            return Ok(None);
        }
        bail!("sysctl sizing kinfo_proc for pid {pid}: {err}");
    }
    if len < std::mem::size_of::<TimeVal>() {
        return Ok(None);
    }

    let mut buf = vec![0u8; len];
    // SAFETY: `buf` holds `len` bytes from the sizing call and `len` is the in-out
    // length; `mib` is unchanged and still valid.
    let rc = unsafe {
        libc::sysctl(
            mib.as_mut_ptr(),
            mib.len() as libc::c_uint,
            buf.as_mut_ptr().cast(),
            &mut len,
            std::ptr::null_mut(),
            0,
        )
    };
    if rc != 0 {
        let err = std::io::Error::last_os_error();
        if err.raw_os_error() == Some(libc::ESRCH) {
            return Ok(None);
        }
        bail!("sysctl reading kinfo_proc for pid {pid}: {err}");
    }
    if len < std::mem::size_of::<TimeVal>() {
        // The process died between the two calls.
        return Ok(None);
    }

    let tv = TimeVal::from_prefix(&buf);
    match Utc.timestamp_opt(tv.sec, tv.usec as u32 * 1_000) {
        chrono::LocalResult::Single(t) => Ok(Some(t)),
        _ => bail!("kernel start time for pid {pid} is not a valid instant: {tv:?}"),
    }
}

/// Reads the start time the kernel recorded for `pid`, from `/proc`.
///
/// A missing `/proc/<pid>` is `Ok(None)`, the answer the Darwin body gives for
/// a pid that is gone. `btime` has whole-second resolution and the tick count
/// does not change, so two reads for one process give the same instant, which
/// is what [`is_alive`]'s equality needs.
#[cfg(target_os = "linux")]
pub fn start_time(pid: u32) -> Result<Option<DateTime<Utc>>> {
    use anyhow::Context;

    let line = match std::fs::read_to_string(format!("/proc/{pid}/stat")) {
        Ok(line) => line,
        Err(err)
            if err.kind() == std::io::ErrorKind::NotFound
                || err.raw_os_error() == Some(libc::ESRCH) =>
        {
            return Ok(None);
        }
        Err(err) => return Err(err).with_context(|| format!("reading /proc/{pid}/stat")),
    };
    let stat = std::fs::read_to_string("/proc/stat").context("reading /proc/stat")?;
    let Some(btime) = stat
        .lines()
        .find_map(|l| l.strip_prefix("btime "))
        .and_then(|v| v.trim().parse::<i64>().ok())
    else {
        bail!("/proc/stat has no btime line");
    };
    // SAFETY: `sysconf` reads a configuration value and touches no memory of ours.
    let hz = unsafe { libc::sysconf(libc::_SC_CLK_TCK) };
    if hz <= 0 {
        bail!("sysconf(_SC_CLK_TCK) answered {hz}");
    }
    start_from_stat(&line, btime, hz)
        .with_context(|| format!("parsing /proc/{pid}/stat"))
        .map(Some)
}

/// The start time a `/proc/<pid>/stat` line names, given the boot time in
/// seconds since the epoch and the clock ticks per second.
///
/// Field 2 is the command name in parentheses and may itself hold spaces and
/// `)`, so the fields are counted from the right of the last `)`. Field 3, the
/// state, is the first one there, which puts field 22 at index 19.
#[cfg(any(target_os = "linux", test))]
fn start_from_stat(line: &str, btime: i64, hz: i64) -> Result<DateTime<Utc>> {
    let Some((_, rest)) = line.rsplit_once(')') else {
        bail!("no `)` closes the command name");
    };
    let Some(field) = rest.split_whitespace().nth(22 - 3) else {
        bail!("fewer than 22 fields");
    };
    let ticks: i64 = field
        .parse()
        .map_err(|_| anyhow::anyhow!("field 22 is not a tick count: {field:?}"))?;
    let secs = btime + ticks / hz;
    let nanos = (ticks % hz) * 1_000_000_000 / hz;
    match Utc.timestamp_opt(secs, nanos as u32) {
        chrono::LocalResult::Single(t) => Ok(t),
        _ => bail!("start time {secs}s {nanos}ns is not a valid instant"),
    }
}

/// Whether a process matching both the pid and the recorded start time exists.
///
/// This is contract 5.2's liveness test, and nothing else in Hydra is allowed to
/// answer the question.
pub fn is_alive(pid: u32, recorded_start: DateTime<Utc>) -> Result<bool> {
    Ok(start_time(pid)? == Some(recorded_start))
}

#[cfg(target_os = "macos")]
#[derive(Debug)]
#[repr(C)]
struct TimeVal {
    sec: i64,
    usec: i32,
}

#[cfg(target_os = "macos")]
impl TimeVal {
    fn from_prefix(buf: &[u8]) -> Self {
        let sec = i64::from_ne_bytes(buf[0..8].try_into().expect("sized above"));
        let usec = i32::from_ne_bytes(buf[8..12].try_into().expect("sized above"));
        Self { sec, usec }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn our_own_start_time_matches_ps() {
        // `/bin/ps` is setuid, and a seatbelt refuses to exec a setuid binary
        // whatever its profile allows, so inside one this test has no witness.
        // Rung two's admission runs the suite inside one. Build ticket 104.
        if crate::sandbox::inside_seatbelt() {
            eprintln!(
                "skipped our_own_start_time_matches_ps: inside a seatbelt, which refuses to \
                 exec setuid /bin/ps"
            );
            return;
        }
        let pid = std::process::id();
        let ours = start_time(pid).unwrap().expect("we are running");

        let out = std::process::Command::new("ps")
            .args(["-p", &pid.to_string(), "-o", "lstart="])
            .output()
            .unwrap();
        let lstart = String::from_utf8(out.stdout).unwrap();
        let lstart = lstart.trim();

        // ps prints local time to the second. Compare the rendered second.
        let ours_local = ours.with_timezone(&chrono::Local);
        let rendered = ours_local.format("%a %b %e %H:%M:%S %Y").to_string();
        assert_eq!(
            rendered, lstart,
            "sysctl offset 0 is not p_starttime any more"
        );
    }

    #[test]
    fn a_reaped_pid_reports_no_start_time() {
        // Not pid 0: that is kernel_task, and sysctl answers for it.
        let mut child = std::process::Command::new("sleep")
            .arg("5")
            .spawn()
            .unwrap();
        let pid = child.id();
        let start = start_time(pid).unwrap().expect("sleep is running");
        assert!(is_alive(pid, start).unwrap());

        child.kill().unwrap();
        child.wait().unwrap();
        assert_eq!(start_time(pid).unwrap(), None);
        assert!(!is_alive(pid, start).unwrap());
    }

    // A line from Linux 6.8 for `sleep 5`, its tick count 812345 at 100 Hz.
    const SLEEP_STAT: &str = "4242 (sleep) S 4100 4242 4100 34816 4242 4194304 98 0 0 0 0 0 \
                              0 0 20 0 1 0 812345 5640192 256 18446744073709551615 1 1 0 0 0 \
                              0 0 0 0 0 0 0 17 3 0 0 0 0 0";

    #[test]
    fn the_stat_parser_adds_ticks_to_the_boot_time() {
        let start = start_from_stat(SLEEP_STAT, 1_760_000_000, 100).unwrap();
        assert_eq!(start.timestamp(), 1_760_000_000 + 8123);
        assert_eq!(start.timestamp_subsec_nanos(), 450_000_000);
    }

    #[test]
    fn the_stat_parser_counts_from_the_last_paren() {
        // A command name may hold spaces and `)`, and the kernel does not escape them.
        let line = SLEEP_STAT.replace("(sleep)", "(a b) S 1 2 3 (x))");
        let start = start_from_stat(&line, 1_760_000_000, 100).unwrap();
        assert_eq!(start.timestamp(), 1_760_000_000 + 8123);
    }

    #[test]
    fn the_stat_parser_refuses_a_short_or_unclosed_line() {
        assert!(start_from_stat("4242 (sleep S 4100", 0, 100).is_err());
        assert!(start_from_stat("4242 (sleep) S 4100 4242", 0, 100).is_err());
        let garbled = SLEEP_STAT.replace(" 812345 ", " soon ");
        assert!(start_from_stat(&garbled, 0, 100).is_err());
    }

    #[test]
    fn liveness_needs_both_halves() {
        let pid = std::process::id();
        let real = start_time(pid).unwrap().unwrap();
        assert!(is_alive(pid, real).unwrap());
        assert!(!is_alive(pid, real - chrono::Duration::seconds(1)).unwrap());
    }
}
