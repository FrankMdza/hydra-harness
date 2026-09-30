//! Terminal input injection under the seatbelt, build ticket 137.
//!
//! `TIOCSTI` pushes bytes into a terminal's input queue as if typed. A process
//! inside the seatbelt whose controlling terminal is the human's could type a
//! command into the shell that reads that terminal after the run ends. The test
//! opens a pty of its own, makes it the controlling terminal of a probe that
//! runs under the profile `compose` writes, and reads the pty back.
//!
//! The probe is this test binary run again with [`PROBE_ENV`] set, so it needs
//! no compiler, no interpreter and no path outside the scratch layout.

#![cfg(target_os = "macos")]

use std::io::Read;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use hydra::paths::Layout;
use hydra::policy::PolicySet;
use hydra::provider::Provider;
use hydra::run::write_spawn_config;

mod corpus;
use corpus::seatbelt_exec_or_skip;

const PROBE_ENV: &str = "HYDRA_TIOCSTI_PROBE";
const PROBE_TEST: &str = "tiocsti_probe";

/// The probe itself. A no-op in a normal run of the suite; with [`PROBE_ENV`]
/// set, its stdin is the pty, already its controlling terminal, and it tries to
/// push `y` and a newline into it.
#[test]
fn tiocsti_probe() {
    if std::env::var_os(PROBE_ENV).is_none() {
        return;
    }
    for byte in *b"y\n" {
        // SAFETY: fd 0 is open for the probe's lifetime, and TIOCSTI reads
        // exactly one byte through the pointer, which points at a live local.
        let rc = unsafe { libc::ioctl(0, libc::TIOCSTI, &byte as *const u8) };
        let err = std::io::Error::last_os_error();
        println!("tiocsti rc={rc} errno={}", err.raw_os_error().unwrap_or(0));
    }
}

fn scratch() -> (Layout, PathBuf) {
    let root = std::env::temp_dir().join(format!("hydra-sandbox-tty-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let layout = Layout::at(&root);
    layout.ensure().unwrap();
    (layout, root)
}

/// A fresh pty pair, master first.
fn open_pty() -> (OwnedFd, OwnedFd) {
    let mut master = -1;
    let mut slave = -1;
    // SAFETY: both out-pointers are live locals, and null name, termios and
    // winsize ask openpty for defaults.
    let rc = unsafe {
        libc::openpty(
            &mut master,
            &mut slave,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    assert_eq!(rc, 0, "openpty: {}", std::io::Error::last_os_error());
    // Raw mode, so the injected bytes are not echoed into an output queue that
    // nobody drains: with echo on, the probe blocks at exit waiting for it.
    // SAFETY: `slave` is the open descriptor openpty returned and `tio` is a
    // live local that tcgetattr fills before cfmakeraw and tcsetattr read it.
    unsafe {
        let mut tio: libc::termios = std::mem::zeroed();
        assert_eq!(libc::tcgetattr(slave, &mut tio), 0);
        libc::cfmakeraw(&mut tio);
        assert_eq!(libc::tcsetattr(slave, libc::TCSANOW, &tio), 0);
    }
    // SAFETY: openpty succeeded, so both are open descriptors nothing else owns.
    unsafe { (OwnedFd::from_raw_fd(master), OwnedFd::from_raw_fd(slave)) }
}

/// Runs the probe under `profile` on a fresh pty and answers what the probe
/// printed and what the pty's input queue held afterwards.
fn probe_under(profile: &Path) -> (String, Vec<u8>) {
    let (_master, slave) = open_pty();
    let stdin = slave.try_clone().unwrap();
    let mut child = Command::new("/usr/bin/sandbox-exec");
    child
        .arg("-f")
        .arg(profile)
        .arg(std::env::current_exe().unwrap())
        .args(["--exact", PROBE_TEST, "--nocapture", "--test-threads=1"])
        .env(PROBE_ENV, "1")
        .stdin(Stdio::from(stdin))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    // SAFETY: setsid and ioctl are async-signal-safe, and the closure touches
    // no memory shared with the parent.
    unsafe {
        child.pre_exec(|| {
            if libc::setsid() == -1 {
                return Err(std::io::Error::last_os_error());
            }
            if libc::ioctl(0, libc::TIOCSCTTY.into(), 0) == -1 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    let out = child.output().unwrap();
    let printed = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    // SAFETY: the descriptor is ours and open; the flags are plain integers.
    unsafe {
        let flags = libc::fcntl(slave.as_raw_fd(), libc::F_GETFL);
        libc::fcntl(slave.as_raw_fd(), libc::F_SETFL, flags | libc::O_NONBLOCK);
    }
    let mut queued = vec![0u8; 64];
    let n = std::fs::File::from(slave).read(&mut queued).unwrap_or(0);
    queued.truncate(n);
    (printed, queued)
}

#[test]
fn the_seatbelt_refuses_terminal_input_injection() {
    if !seatbelt_exec_or_skip("the_seatbelt_refuses_terminal_input_injection") {
        return;
    }
    let (layout, root) = scratch();
    let set = PolicySet::discover().unwrap();
    let worktree = root.join("worktree");
    std::fs::create_dir_all(&worktree).unwrap();

    // The control: under a profile with no rules the probe must inject, or the
    // refusal below would pass on a probe that never worked.
    let bare = root.join("bare.sb");
    std::fs::write(&bare, "(version 1)\n(allow default)\n").unwrap();
    let (printed, _) = probe_under(&bare);
    assert!(
        printed.contains("tiocsti rc=0"),
        "the probe did not inject under a bare profile: {printed}"
    );

    let spawn = write_spawn_config(
        &layout,
        "r1",
        &set,
        Provider::Claude,
        "standard",
        Some(&worktree),
        None,
        None,
        false,
    )
    .unwrap();
    let profile = spawn.sandbox_profile.expect("macOS spawn writes a profile");
    let (printed, queued) = probe_under(&profile);
    assert!(
        printed.contains("tiocsti rc=-1") && !printed.contains("tiocsti rc=0"),
        "TIOCSTI was not refused under the spawn's profile: {printed}"
    );
    assert!(
        queued.is_empty(),
        "the pty's input queue holds {queued:?} after the probe: {printed}"
    );
    let _ = std::fs::remove_dir_all(&root);
}
