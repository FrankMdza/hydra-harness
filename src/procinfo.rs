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

use anyhow::{Result, bail};
use chrono::{DateTime, TimeZone, Utc};

const CTL_KERN: libc::c_int = 1;
const KERN_PROC: libc::c_int = 14;

/// Reads the start time the kernel recorded for `pid`.
///
/// Returns `Ok(None)` when no such process exists, which is the answer a liveness
/// check wants rather than an error.
pub fn start_time(pid: u32) -> Result<Option<DateTime<Utc>>> {
    let mut mib = [CTL_KERN, KERN_PROC, libc::KERN_PROC_PID, pid as libc::c_int];
    let mut len: libc::size_t = 0;

    // First call sizes the buffer. A live pid gives one kinfo_proc, 3888 bytes on
    // this machine; a dead one gives zero.
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

/// Whether a process matching both the pid and the recorded start time exists.
///
/// This is contract 5.2's liveness test, and nothing else in Hydra is allowed to
/// answer the question.
pub fn is_alive(pid: u32, recorded_start: DateTime<Utc>) -> Result<bool> {
    Ok(start_time(pid)? == Some(recorded_start))
}

#[derive(Debug)]
#[repr(C)]
struct TimeVal {
    sec: i64,
    usec: i32,
}

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

    #[test]
    fn liveness_needs_both_halves() {
        let pid = std::process::id();
        let real = start_time(pid).unwrap().unwrap();
        assert!(is_alive(pid, real).unwrap());
        assert!(!is_alive(pid, real - chrono::Duration::seconds(1)).unwrap());
    }
}
