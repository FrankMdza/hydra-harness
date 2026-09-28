//! Spawning a provider on a pty Hydra owns, and teeing what comes back.
//!
//! Hydra is the parent process rather than a client of a terminal. Ticket 02
//! settled that: it has to be the parent to tee anyway, and Ghostty exposes no pid
//! and no exit status. `portable-pty` is the one dependency that carries this.
//!
//! Two things a pipe would not have shown, measured on `claude` 2.1.261 on
//! 2026-09-05. The tty translates every `\n` to `\r\n`, so a line arrives with a
//! carriage return the frame did not contain. And `claude` writes a bare
//! `ESC[?25h` after the last frame, with no trailing newline, which is a line the
//! record has to hold rather than drop.

use std::io::ErrorKind;
use std::time::Duration;

use anyhow::{Context, Result};
use portable_pty::{CommandBuilder, MasterPty, PtySize, native_pty_system};

/// 16 MiB per line. Frames of 200 KB are tested and fine; a provider that
/// streams one enormous frame with no newline would otherwise grow the tee
/// buffer without limit. When `pending` reaches this with no newline, the tee
/// writes what it holds as a frame, counts it, and keeps reading so the rest
/// of the line becomes the next frame. Nothing is dropped.
pub const LINE_CAP: usize = 16 * 1024 * 1024;

/// After the child exits, how long the tee waits for the pty's EOF before giving
/// up. A descendant that inherited the slave can hold it open without writing,
/// which used to block `read` forever; this bound is long enough for the kernel
/// to flush buffered output and short enough that a sleeping grandchild does not
/// pin a record open.
pub const PTY_EOF_DEADLINE: Duration = Duration::from_secs(2);

/// How often the tee polls while the child is still alive. Without a timeout,
/// `poll` would not return when the child exits and a grandchild keeps the slave
/// open with no more data to read.
const POLL_WHILE_ALIVE: Duration = Duration::from_millis(100);

/// Trailing half of the `run_end.error` clause for lines split at [`LINE_CAP`].
/// The count and a space precede it, the same shape as the lossy UTF-8 note:
/// `"1 line(s) the child wrote were split by the tee"`.
pub const CAPPED_CLAUSE: &str = "line(s) the child wrote were split by the tee";

/// The `run_end.error` sentence for `count` lines the tee split.
pub fn capped_line_note(count: usize) -> String {
    format!("{count} {CAPPED_CLAUSE}")
}

/// How many capped lines `run_end.error` names, if it names any.
pub fn capped_count(error: &str) -> Option<usize> {
    error.split("; ").find_map(|part| {
        let rest = part.strip_suffix(CAPPED_CLAUSE)?;
        rest.trim().parse().ok()
    })
}

pub struct Spawned {
    pub pid: u32,
    child: Box<dyn portable_pty::Child + Send + Sync>,
    /// Held so the pty stays open for as long as the run does.
    _master: Box<dyn MasterPty + Send>,
    /// Lines the child wrote that were not valid UTF-8.
    pub lossy_lines: usize,
    /// Times the tee split a line at [`LINE_CAP`] because no newline had arrived.
    pub capped_lines: usize,
    /// The child exited but a descendant still held the slave when the tee gave up.
    pub pty_still_held: bool,
    pty_fd: libc::c_int,
    /// The signal the child died on, once [`Spawned::wait`] has returned, or
    /// `None` for a child that exited.
    ///
    /// Kept beside the exit code because the pty reports a signal death as exit
    /// code 1, which on `cursor-agent` is also what an orderly failure exits
    /// with. Without this the two would be one shape on the record.
    pub signal: Option<String>,
}

/// Opens a pty, spawns `binary` on its slave, and returns the spawned child
/// with the slave handle still open.
///
/// The environment is exactly what the caller passes. `CommandBuilder::new` copies
/// this process's environment, so it is cleared before the caller's names go in;
/// that is what makes the allowlist in [`crate::env`] a boundary rather than a
/// filter. Until 2026-09-23 this comment said the builder started empty and the
/// clear was missing, so every recorded run inherited the shell that launched
/// `hydra run` through the watcher; the second hiring panel found it and
/// `tests/pty_tee.rs::the_child_gets_the_environment_it_is_handed_and_nothing_else`
/// fails without the clear.
fn open(
    binary: &str,
    argv: &[String],
    cwd: &std::path::Path,
    env: impl Iterator<Item = (String, String)>,
    sandbox_profile: Option<&std::path::Path>,
) -> Result<(Spawned, Box<dyn portable_pty::SlavePty + Send>)> {
    let (binary, argv) = match sandbox_profile {
        Some(profile) => {
            let (wrapped, argv) = crate::sandbox::wrap_command(binary, argv, profile);
            (wrapped, argv)
        }
        None => (binary, argv.to_vec()),
    };
    let pty = native_pty_system();
    let pair = pty
        .openpty(PtySize {
            // Wide enough that nothing a provider prints can wrap. Frames are
            // written with one console.log each and do not consult the width, but a
            // wrapped frame would be an unreadable record and the cost here is zero.
            rows: 40,
            cols: 200,
            pixel_width: 0,
            pixel_height: 0,
        })
        .context("opening a pty")?;

    let mut cmd = CommandBuilder::new(binary);
    for arg in argv {
        cmd.arg(arg);
    }
    cmd.cwd(cwd);
    cmd.env_clear();
    for (name, value) in env {
        cmd.env(name, value);
    }

    let child = pair
        .slave
        .spawn_command(cmd)
        .with_context(|| format!("spawning {binary}"))?;
    let pid = child.process_id().context("the child reported no pid")?;

    let pty_fd = pair
        .master
        .as_raw_fd()
        .context("the native pty has no fd")?;

    Ok((
        Spawned {
            pid,
            child,
            _master: pair.master,
            lossy_lines: 0,
            capped_lines: 0,
            pty_still_held: false,
            pty_fd,
            signal: None,
        },
        pair.slave,
    ))
}

/// Spawns `binary` on a fresh pty and drops this process's slave handle.
///
/// The child holds the only other slave fd then. Without the drop the master
/// never reaches EOF and the tee loop waits for a writer that already exited.
pub fn spawn(
    binary: &str,
    argv: &[String],
    cwd: &std::path::Path,
    env: impl Iterator<Item = (String, String)>,
    sandbox_profile: Option<&std::path::Path>,
) -> Result<Spawned> {
    let (spawned, slave) = open(binary, argv, cwd, env, sandbox_profile)?;
    drop(slave);
    Ok(spawned)
}

/// Keeps the slave open after the child exits. Integration tests use this to
/// mimic a grandchild that inherited the slave and outlived the provider.
pub struct HeldSlave {
    _slave: Box<dyn portable_pty::SlavePty + Send>,
}

/// Like [`spawn`], but the returned [`HeldSlave`] keeps the slave open until it
/// is dropped.
pub fn spawn_with_held_slave(
    binary: &str,
    argv: &[String],
    cwd: &std::path::Path,
    env: impl Iterator<Item = (String, String)>,
    sandbox_profile: Option<&std::path::Path>,
) -> Result<(Spawned, HeldSlave)> {
    let (spawned, slave) = open(binary, argv, cwd, env, sandbox_profile)?;
    Ok((spawned, HeldSlave { _slave: slave }))
}

impl Spawned {
    /// Reads the child's output to EOF, calling `on_line` once per line.
    ///
    /// A trailing fragment with no newline is delivered too. `claude` ends every
    /// print-mode run with one.
    ///
    /// After the child's pid is dead, the tee waits at most [`PTY_EOF_DEADLINE`]
    /// for EOF. A descendant that inherited the slave can keep it open without
    /// writing; without the bound the read would block forever and the record
    /// would never close.
    pub fn tee(&mut self, mut on_line: impl FnMut(&str) -> Result<()>) -> Result<()> {
        self.pty_still_held = false;
        set_nonblocking(self.pty_fd)?;
        let mut pending: Vec<u8> = Vec::new();
        let mut chunk = [0u8; 65536];
        let mut child_dead_since: Option<std::time::Instant> = None;

        loop {
            let timeout_ms = if let Some(since) = child_dead_since {
                let left = PTY_EOF_DEADLINE.saturating_sub(since.elapsed());
                left.as_millis().min(i32::MAX as u128) as i32
            } else {
                POLL_WHILE_ALIVE.as_millis().min(i32::MAX as u128) as i32
            };

            let mut pfd = libc::pollfd {
                fd: self.pty_fd,
                events: libc::POLLIN,
                revents: 0,
            };
            // SAFETY: `pty_fd` is the master fd of `_master`, open for the life of
            // `Spawned`; `pfd` is one live `pollfd` for the call.
            let polled = unsafe { libc::poll(&mut pfd, 1, timeout_ms) };
            if polled < 0 {
                let err = std::io::Error::last_os_error();
                if err.kind() == ErrorKind::Interrupted {
                    continue;
                }
                return Err(err).context("polling the child's pty");
            }

            if pfd.revents & (libc::POLLIN | libc::POLLHUP | libc::POLLERR) != 0 {
                // SAFETY: `pty_fd` is the same open master as poll; `chunk` is
                // 65536 bytes and its pointer is valid for `chunk.len()`.
                let read = unsafe {
                    libc::read(
                        self.pty_fd,
                        chunk.as_mut_ptr() as *mut libc::c_void,
                        chunk.len(),
                    )
                };
                if read == 0 {
                    if self.child_exited(&mut child_dead_since)? {
                        if self.persistent_eof(&mut pending)? {
                            break;
                        }
                        continue;
                    }
                    continue;
                }
                if read < 0 {
                    let err = std::io::Error::last_os_error();
                    // A pty master reports EIO rather than EOF on some platforms once
                    // the last slave closes. Both mean the same thing here.
                    if err.raw_os_error() == Some(libc::EIO) {
                        if self.child_exited(&mut child_dead_since)? {
                            if self.persistent_eof(&mut pending)? {
                                break;
                            }
                            continue;
                        }
                        break;
                    }
                    if err.raw_os_error() == Some(libc::EAGAIN) {
                        continue;
                    }
                    if err.kind() == ErrorKind::Interrupted {
                        continue;
                    }
                    return Err(err).context("reading the child's pty");
                }
                let n = read as usize;
                pending.extend_from_slice(&chunk[..n]);
                self.flush_pending(&mut pending, &mut on_line)?;
            }

            if self.child_exited(&mut child_dead_since)? {
                let left = PTY_EOF_DEADLINE.saturating_sub(
                    child_dead_since
                        .expect("child_dead_since is set when child_exited returns true")
                        .elapsed(),
                );
                if left.is_zero() {
                    self.pty_still_held = true;
                    break;
                }
            }
        }

        if !pending.is_empty() {
            let trailing = std::mem::take(&mut pending);
            self.deliver(&trailing, &mut on_line)?;
        }
        Ok(())
    }

    /// Emits complete lines and any prefix that has hit [`LINE_CAP`] with no
    /// newline. Leftover bytes stay in `pending` for the next read.
    fn flush_pending(
        &mut self,
        pending: &mut Vec<u8>,
        on_line: &mut impl FnMut(&str) -> Result<()>,
    ) -> Result<()> {
        loop {
            match pending.iter().position(|b| *b == b'\n') {
                Some(at) if at >= LINE_CAP => self.emit_capped(pending, on_line)?,
                Some(at) => {
                    let line: Vec<u8> = pending.drain(..=at).collect();
                    self.deliver(&line[..line.len() - 1], on_line)?;
                }
                None if pending.len() >= LINE_CAP => self.emit_capped(pending, on_line)?,
                None => return Ok(()),
            }
        }
    }

    fn emit_capped(
        &mut self,
        pending: &mut Vec<u8>,
        on_line: &mut impl FnMut(&str) -> Result<()>,
    ) -> Result<()> {
        let held: Vec<u8> = pending.drain(..LINE_CAP).collect();
        self.capped_lines += 1;
        self.deliver(&held, on_line)
    }

    fn deliver(&mut self, line: &[u8], on_line: &mut impl FnMut(&str) -> Result<()>) -> Result<()> {
        // ONLCR put the carriage return there, not the provider.
        let line = line.strip_suffix(b"\r").unwrap_or(line);
        if line.is_empty() {
            return Ok(());
        }
        let text = match std::str::from_utf8(line) {
            Ok(text) => std::borrow::Cow::Borrowed(text),
            Err(_) => {
                self.lossy_lines += 1;
                String::from_utf8_lossy(line)
            }
        };
        on_line(&text)
    }

    /// Whether the master still sees EOF after the child has exited. A single
    /// zero is often spurious while a descendant holds the slave; two in a row
    /// mean the slave really closed.
    fn persistent_eof(&self, pending: &mut Vec<u8>) -> Result<bool> {
        let mut pfd = libc::pollfd {
            fd: self.pty_fd,
            events: libc::POLLIN | libc::POLLHUP,
            revents: 0,
        };
        // SAFETY: `pty_fd` is the open master while `_master` lives; one `pollfd`
        // for the call.
        let polled = unsafe { libc::poll(&mut pfd, 1, 0) };
        if polled <= 0 {
            return Ok(false);
        }
        let mut byte = [0u8; 1];
        // SAFETY: `pty_fd` is the open master; `byte` is one byte and its pointer
        // is valid for `byte.len()`.
        let read = unsafe {
            libc::read(
                self.pty_fd,
                byte.as_mut_ptr() as *mut libc::c_void,
                byte.len(),
            )
        };
        if read == 0 {
            return Ok(true);
        }
        if read < 0 {
            let err = std::io::Error::last_os_error();
            if err.raw_os_error() == Some(libc::EAGAIN) {
                return Ok(false);
            }
            if err.raw_os_error() == Some(libc::EIO) {
                return Ok(true);
            }
            return Ok(false);
        }
        // The probe read a real byte. It is the child's output and goes where
        // every other byte goes; the next flush delivers it.
        pending.push(byte[0]);
        Ok(false)
    }

    /// Records that the child has exited and returns whether the tee is still
    /// waiting for EOF on a slave a descendant may hold.
    fn child_exited(&mut self, child_dead_since: &mut Option<std::time::Instant>) -> Result<bool> {
        if self.child.try_wait()?.is_none() {
            return Ok(false);
        }
        if child_dead_since.is_none() {
            *child_dead_since = Some(std::time::Instant::now());
        }
        Ok(true)
    }

    /// Waits for the child and returns its exit status.
    ///
    /// Process exit is the authoritative end of a run. The provider's own terminator
    /// frame is optional enrichment: `claude` always writes one, `cursor-agent`
    /// writes one only on success, and no contract survives a frame two of three
    /// providers withhold on the path that matters most.
    pub fn wait(&mut self) -> Result<Option<i32>> {
        let status = self.child.wait().context("waiting for the child")?;
        self.signal = status.signal().map(str::to_string);
        Ok(Some(status.exit_code() as i32))
    }
}

fn set_nonblocking(fd: libc::c_int) -> Result<()> {
    // SAFETY: `fd` is an open pty master the caller owns for this call.
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    if flags < 0 {
        return Err(std::io::Error::last_os_error()).context("reading the pty flags");
    }
    // SAFETY: same open `fd`; `flags` came from `F_GETFL` on it.
    let rc = unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) };
    if rc < 0 {
        return Err(std::io::Error::last_os_error()).context("making the pty non-blocking");
    }
    Ok(())
}
