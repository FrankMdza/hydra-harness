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

use std::path::{Path, PathBuf};

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

/// A process whose cwd or an open file sits under a path Hydra is about to
/// change.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Holder {
    pub pid: u32,
    pub command: String,
}

/// Processes, other than this one, that have a cwd or an open file under any
/// of `roots`.
///
/// Build ticket 373. Reap calls this before it sweeps a private git directory
/// or runs git on a checkout. A pid this user cannot inspect is skipped: the
/// run's processes share this uid, and a pid that belongs to someone else is
/// not one of them. On macOS the list is `proc_listpids` with
/// `proc_pidinfo` and `proc_pidfdinfo`. On Linux it is `/proc/<pid>/cwd` and
/// `/proc/<pid>/fd`.
pub fn holders<P: AsRef<Path>>(roots: &[P]) -> Result<Vec<Holder>> {
    let roots = real_roots(roots);
    if roots.is_empty() {
        return Ok(Vec::new());
    }
    let mut found = scan_holders(&roots)?;
    found.sort_by_key(|h| h.pid);
    found.dedup_by_key(|h| h.pid);
    Ok(found)
}

fn real_roots<P: AsRef<Path>>(roots: &[P]) -> Vec<PathBuf> {
    roots
        .iter()
        .map(|root| {
            let root = root.as_ref();
            std::fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf())
        })
        .filter(|root| !root.as_os_str().is_empty())
        .collect()
}

fn holds(path: &Path, roots: &[PathBuf]) -> bool {
    let path = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    roots
        .iter()
        .any(|root| path == *root || path.starts_with(root))
}

#[cfg(target_os = "macos")]
fn scan_holders(roots: &[PathBuf]) -> Result<Vec<Holder>> {
    let self_pid = std::process::id();
    let mut found = Vec::new();
    for pid in macos_pids()? {
        if pid == self_pid || pid == 0 {
            continue;
        }
        let Ok(raw) = i32::try_from(pid) else {
            continue;
        };
        if macos_holds(raw, roots) {
            found.push(Holder {
                pid,
                command: macos_command(raw),
            });
        }
    }
    Ok(found)
}

#[cfg(target_os = "macos")]
fn macos_pids() -> Result<Vec<u32>> {
    const PROC_UID_ONLY: u32 = 4;
    // SAFETY: getuid reads the calling process and touches no memory of ours.
    let uid = unsafe { libc::getuid() };
    let mut buf = vec![0i32; 1024];
    loop {
        let bytes = buf.len() * std::mem::size_of::<i32>();
        // SAFETY: `buf` is a live vector of `pid_t` values and `bytes` is its
        // length. proc_listpids writes at most that many bytes of pids.
        let wrote = unsafe {
            libc::proc_listpids(PROC_UID_ONLY, uid, buf.as_mut_ptr().cast(), bytes as i32)
        };
        if wrote <= 0 {
            let err = std::io::Error::last_os_error();
            bail!("proc_listpids: {err}");
        }
        let n = wrote as usize / std::mem::size_of::<i32>();
        if n >= buf.len() {
            buf.resize(buf.len() * 2, 0);
            continue;
        }
        return Ok(buf[..n]
            .iter()
            .copied()
            .filter(|&pid| pid > 0)
            .map(|pid| pid as u32)
            .collect());
    }
}

#[cfg(target_os = "macos")]
fn macos_holds(pid: i32, roots: &[PathBuf]) -> bool {
    if macos_cwd(pid).is_some_and(|cwd| holds(&cwd, roots)) {
        return true;
    }
    macos_vnodes(pid).iter().any(|path| holds(path, roots))
}

#[cfg(target_os = "macos")]
fn macos_cwd(pid: i32) -> Option<PathBuf> {
    // SAFETY: proc_vnodepathinfo is a C struct of integers and byte arrays.
    // All-zero is a valid bit pattern, and the kernel overwrites it before we
    // read a field.
    let mut info = unsafe { std::mem::zeroed::<libc::proc_vnodepathinfo>() };
    // SAFETY: `info` is a live proc_vnodepathinfo. The kernel writes at most
    // its size, and we only read it when the call copied a positive length.
    let wrote = unsafe {
        libc::proc_pidinfo(
            pid,
            libc::PROC_PIDVNODEPATHINFO,
            0,
            std::ptr::from_mut(&mut info).cast(),
            std::mem::size_of::<libc::proc_vnodepathinfo>() as i32,
        )
    };
    if wrote <= 0 {
        return None;
    }
    c_path(&info.pvi_cdir.vip_path)
}

#[cfg(target_os = "macos")]
fn macos_vnodes(pid: i32) -> Vec<PathBuf> {
    // SAFETY: a null buffer with size 0 is the documented way to ask
    // proc_pidinfo how many bytes PROC_PIDLISTFDS needs.
    let needed =
        unsafe { libc::proc_pidinfo(pid, libc::PROC_PIDLISTFDS, 0, std::ptr::null_mut(), 0) };
    if needed <= 0 {
        return Vec::new();
    }
    let count = needed as usize / std::mem::size_of::<libc::proc_fdinfo>();
    let mut fds = vec![
        libc::proc_fdinfo {
            proc_fd: 0,
            proc_fdtype: 0,
        };
        count
    ];
    // SAFETY: `fds` holds `count` proc_fdinfo slots, `needed` is that many
    // bytes, and the kernel writes at most that.
    let wrote = unsafe {
        libc::proc_pidinfo(
            pid,
            libc::PROC_PIDLISTFDS,
            0,
            fds.as_mut_ptr().cast(),
            needed,
        )
    };
    if wrote <= 0 {
        return Vec::new();
    }
    let n = wrote as usize / std::mem::size_of::<libc::proc_fdinfo>();
    fds[..n]
        .iter()
        .filter(|fd| fd.proc_fdtype == libc::PROX_FDTYPE_VNODE as u32)
        .filter_map(|fd| macos_vnode_path(pid, fd.proc_fd))
        .collect()
}

#[cfg(target_os = "macos")]
fn macos_vnode_path(pid: i32, fd: i32) -> Option<PathBuf> {
    const PROC_PIDFDVNODEPATHINFO: i32 = 2;
    // SAFETY: VnodeFdInfoWithPath is a C struct of integers and byte arrays.
    // All-zero is a valid bit pattern, and the kernel overwrites it before we
    // read a field.
    let mut info = unsafe { std::mem::zeroed::<VnodeFdInfoWithPath>() };
    // SAFETY: `info` is a live vnode_fdinfowithpath. The kernel writes at most
    // its size, and we only read the path when the call copied a positive length.
    let wrote = unsafe {
        libc::proc_pidfdinfo(
            pid,
            fd,
            PROC_PIDFDVNODEPATHINFO,
            std::ptr::from_mut(&mut info).cast(),
            std::mem::size_of::<VnodeFdInfoWithPath>() as i32,
        )
    };
    if wrote <= 0 {
        return None;
    }
    c_path(&info.pvip.vip_path)
}

#[cfg(target_os = "macos")]
fn macos_command(pid: i32) -> String {
    let mut buf = [0u8; libc::MAXCOMLEN + 1];
    // SAFETY: `buf` is a live stack array and proc_name writes at most
    // `buffersize` bytes of a C string into it.
    let wrote = unsafe { libc::proc_name(pid, buf.as_mut_ptr().cast(), buf.len() as u32) };
    if wrote <= 0 {
        return String::new();
    }
    let end = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    String::from_utf8_lossy(&buf[..end]).into_owned()
}

#[cfg(target_os = "macos")]
fn c_path(chunks: &[[libc::c_char; 32]; 32]) -> Option<PathBuf> {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;
    let bytes: Vec<u8> = chunks
        .iter()
        .flatten()
        .map(|&c| c as u8)
        .take_while(|&b| b != 0)
        .collect();
    if bytes.is_empty() {
        None
    } else {
        Some(PathBuf::from(OsString::from_vec(bytes)))
    }
}

/// `sys/proc_info.h` `vnode_fdinfowithpath`. libc has the nested path struct
/// and not this wrapper.
#[cfg(target_os = "macos")]
#[repr(C)]
struct VnodeFdInfoWithPath {
    pfi: ProcFileInfo,
    pvip: libc::vnode_info_path,
}

#[cfg(target_os = "macos")]
#[repr(C)]
struct ProcFileInfo {
    fi_openflags: u32,
    fi_status: u32,
    fi_offset: i64,
    fi_type: i32,
    fi_guardflags: u32,
}

#[cfg(target_os = "linux")]
fn scan_holders(roots: &[PathBuf]) -> Result<Vec<Holder>> {
    let self_pid = std::process::id();
    let mut found = Vec::new();
    let proc = match std::fs::read_dir("/proc") {
        Ok(dir) => dir,
        Err(err) => bail!("reading /proc: {err}"),
    };
    for entry in proc.flatten() {
        let Ok(pid) = entry.file_name().to_string_lossy().parse::<u32>() else {
            continue;
        };
        if pid == self_pid || pid == 0 {
            continue;
        }
        if let Some(holder) = linux_holder(pid, roots) {
            found.push(holder);
        }
    }
    Ok(found)
}

#[cfg(target_os = "linux")]
fn linux_holder(pid: u32, roots: &[PathBuf]) -> Option<Holder> {
    match std::fs::read_link(format!("/proc/{pid}/cwd")) {
        Ok(cwd) if holds(&cwd, roots) => {
            return Some(Holder {
                pid,
                command: linux_comm(pid),
            });
        }
        Err(err) if skip_inspect(&err) => return None,
        _ => {}
    }
    let fds = match std::fs::read_dir(format!("/proc/{pid}/fd")) {
        Ok(dir) => dir,
        Err(err) if skip_inspect(&err) => return None,
        Err(_) => return None,
    };
    for fd in fds.flatten() {
        match std::fs::read_link(fd.path()) {
            Ok(path) if holds(&path, roots) => {
                return Some(Holder {
                    pid,
                    command: linux_comm(pid),
                });
            }
            Err(err) if skip_inspect(&err) => continue,
            _ => continue,
        }
    }
    None
}

#[cfg(target_os = "linux")]
fn linux_comm(pid: u32) -> String {
    std::fs::read_to_string(format!("/proc/{pid}/comm"))
        .unwrap_or_default()
        .trim()
        .to_string()
}

#[cfg(target_os = "linux")]
fn skip_inspect(err: &std::io::Error) -> bool {
    matches!(
        err.kind(),
        std::io::ErrorKind::PermissionDenied | std::io::ErrorKind::NotFound
    ) || matches!(
        err.raw_os_error(),
        Some(libc::EACCES | libc::EPERM | libc::ESRCH | libc::ENOENT)
    )
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn scan_holders(_roots: &[PathBuf]) -> Result<Vec<Holder>> {
    Ok(Vec::new())
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
