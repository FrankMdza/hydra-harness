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

use std::io::Read;

use anyhow::{Context, Result};
use portable_pty::{CommandBuilder, MasterPty, PtySize, native_pty_system};

/// 16 MiB per line. Frames of 200 KB are tested and fine; a provider that
/// streams one enormous frame with no newline would otherwise grow the tee
/// buffer without limit. When `pending` reaches this with no newline, the tee
/// writes what it holds as a frame, counts it, and keeps reading so the rest
/// of the line becomes the next frame. Nothing is dropped.
pub const LINE_CAP: usize = 16 * 1024 * 1024;

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
    reader: Box<dyn Read + Send>,
    /// Held so the pty stays open for as long as the run does.
    _master: Box<dyn MasterPty + Send>,
    /// Lines the child wrote that were not valid UTF-8.
    pub lossy_lines: usize,
    /// Times the tee split a line at [`LINE_CAP`] because no newline had arrived.
    pub capped_lines: usize,
    /// The signal the child died on, once [`Spawned::wait`] has returned, or
    /// `None` for a child that exited.
    ///
    /// Kept beside the exit code because the pty reports a signal death as exit
    /// code 1, which on `cursor-agent` is also what an orderly failure exits
    /// with. Without this the two would be one shape on the record.
    pub signal: Option<String>,
}

/// Launches `binary` with `argv` on a new pty.
///
/// The environment is exactly what the caller passes. `CommandBuilder::new` starts
/// from an empty environment rather than inheriting this process's, which is what
/// makes the allowlist in [`crate::env`] a boundary rather than a filter.
pub fn spawn(
    binary: &str,
    argv: &[String],
    cwd: &std::path::Path,
    env: impl Iterator<Item = (String, String)>,
    sandbox_profile: Option<&std::path::Path>,
) -> Result<Spawned> {
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
    for (name, value) in env {
        cmd.env(name, value);
    }

    let child = pair
        .slave
        .spawn_command(cmd)
        .with_context(|| format!("spawning {binary}"))?;
    let pid = child.process_id().context("the child reported no pid")?;

    let reader = pair
        .master
        .try_clone_reader()
        .context("cloning the pty reader")?;
    // The child holds the only other slave fd now. Without this drop the master
    // never reaches EOF and the tee loop waits for a writer that already exited.
    drop(pair.slave);

    Ok(Spawned {
        pid,
        child,
        reader,
        _master: pair.master,
        lossy_lines: 0,
        capped_lines: 0,
        signal: None,
    })
}

impl Spawned {
    /// Reads the child's output to EOF, calling `on_line` once per line.
    ///
    /// A trailing fragment with no newline is delivered too. `claude` ends every
    /// print-mode run with one.
    pub fn tee(&mut self, mut on_line: impl FnMut(&str) -> Result<()>) -> Result<()> {
        let mut pending: Vec<u8> = Vec::new();
        let mut chunk = [0u8; 65536];

        loop {
            let read = match self.reader.read(&mut chunk) {
                Ok(0) => break,
                Ok(n) => n,
                // A pty master reports EIO rather than EOF on some platforms once
                // the last slave closes. Both mean the same thing here.
                Err(err) if err.raw_os_error() == Some(libc::EIO) => break,
                Err(err) if err.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(err) => return Err(err).context("reading the child's pty"),
            };
            pending.extend_from_slice(&chunk[..read]);
            self.flush_pending(&mut pending, &mut on_line)?;
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
