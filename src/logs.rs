//! `hydra logs`, harness spec 10's one view.
//!
//! **Observation is read-only.** Tailing the run file is the only way to watch an
//! unwatched child: no attach, no input, no answering a permission prompt from
//! outside. D5 caps the crossings at two, two of three providers cannot be asked
//! anything mid-run anyway, and the gate already decided every permission question
//! before the process started. So this prints bytes and does nothing else.
//!
//! What it prints is the run file verbatim, one contract-3 line at a time. It is
//! not the normalized view: `hydra show --events` is that, and it is a different
//! ticket.

use std::io::{Read, Seek, SeekFrom, Write};

use anyhow::{Context, Result, bail};

use crate::paths::Layout;

/// How often `--follow` looks for more.
const POLL: std::time::Duration = std::time::Duration::from_millis(100);

/// `out` is where the bytes go, which is stdout for the command and a buffer for
/// the suite. Nothing here formats, so the two are the same run of bytes.
pub fn logs(layout: &Layout, run_id: &str, follow: bool, out: &mut dyn Write) -> Result<()> {
    let path = layout.run_file(run_id);
    if !path.is_file() {
        bail!("no run record at {}", path.display());
    }

    let mut file =
        std::fs::File::open(&path).with_context(|| format!("opening {}", path.display()))?;
    let mut at = copy_from(&mut file, 0, out)?;
    if !follow {
        return Ok(());
    }

    // The run may have closed between the read above and this test, so the end
    // condition is checked after each pass rather than before the first.
    loop {
        if crate::live::has_run_end(&path)? {
            copy_from(&mut file, at, out)?;
            return Ok(());
        }
        // A run whose writer died leaves a file that will never gain a closing
        // line. Following it forever would be following nothing, so the liveness
        // test ends the tail the same way the closing line does, and says which
        // of the two happened.
        //
        // **A dead pid is not yet an answer.** The writer reaps the child and
        // then writes the closing line, so every ordinary run passes through a
        // window where the pid is gone and the record has not ended. It is a
        // `write_all` and an `fsync` wide against a 100ms poll, which is small
        // and not rare: giving up inside it would drop the `run_end` off the tail
        // and tell the reader the run was abandoned when it finished. So the
        // liveness test only opens the question and the file answers it.
        if !crate::live::read_view(&path)?.is_live() {
            std::thread::sleep(POLL);
            at = copy_from(&mut file, at, out)?;
            if crate::live::has_run_end(&path)? {
                copy_from(&mut file, at, out)?;
                return Ok(());
            }
            eprintln!(
                "hydra: this run has no closing line and its process is gone. \
                 `hydra status {run_id}` has the rest."
            );
            return Ok(());
        }
        std::thread::sleep(POLL);
        at = copy_from(&mut file, at, out)?;
    }
}

/// Copies from `at` to the end of the file, returning the new offset.
///
/// It seeks rather than reopening, so a run file that grows under the tail is
/// followed rather than reread. Partial lines are not held back: a frame arrives
/// in one `write_all` followed by an `fsync`, so a torn read is a race with a
/// single write rather than a normal state.
fn copy_from(file: &mut std::fs::File, at: u64, out: &mut dyn Write) -> Result<u64> {
    file.seek(SeekFrom::Start(at))?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    if !buffer.is_empty() {
        out.write_all(&buffer)?;
        out.flush()?;
    }
    Ok(at + buffer.len() as u64)
}
