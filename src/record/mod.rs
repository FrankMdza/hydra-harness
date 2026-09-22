//! The run record: `~/hydra/runs/<run-id>.jsonl`, one file per run.
//!
//! Contract sections 2 to 6. Every line is a Hydra envelope. A provider line
//! wraps one provider frame verbatim in `raw`; a Hydra line carries a body Hydra
//! authored and has no `raw` field. There are exactly two Hydra lines, `run_start`
//! and `run_end`, and everything else in the file came off the provider's stream.
//!
//! Nothing normalized is written here. The ten event types in contract section 7
//! are derived at read time by `src/normalize/`, which is why an adapter bug found
//! next month fixes last month's records.

mod lines;
pub mod secrets;
pub mod validate;

/// An exclusive `flock(2)` on `path`, released when the handle drops.
///
/// Advisory and kernel-held, so a crashed holder leaves nothing to clear. The
/// harvest of an abandoned interactive run takes it on the record itself:
/// `hydra status` harvests, the Herdr bridge polls `hydra status`, and two
/// harvesters that both pass a `has_run_end` check append the transcript
/// twice, which the validator then refuses as a lost line and a second
/// `run_end`.
pub fn lock_exclusive(path: &std::path::Path) -> anyhow::Result<std::fs::File> {
    use anyhow::Context;
    use std::os::fd::AsRawFd;
    let file = std::fs::OpenOptions::new()
        .read(true)
        .open(path)
        .with_context(|| format!("opening {} to lock it", path.display()))?;
    loop {
        // SAFETY: `flock` takes a descriptor this handle owns and touches no
        // memory of ours.
        if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX) } == 0 {
            return Ok(file);
        }
        let err = std::io::Error::last_os_error();
        if err.kind() != std::io::ErrorKind::Interrupted {
            return Err(err).with_context(|| format!("locking {}", path.display()));
        }
    }
}

pub use lines::{Auth, Refusal, RunEnd, RunStart, WorktreeRef};

use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::{SecondsFormat, Utc};
use serde::Serialize;
use serde_json::value::RawValue;

/// The contract version this binary writes. Contract section 15: an integer on
/// `run_start`, versioning the normalized vocabulary rather than the file format.
pub const SCHEMA_VERSION: u32 = 1;

/// One past the last `seq` a run file already used.
///
/// `seq` is gapless from 0, so counting non-empty lines answers it without
/// parsing any of them. A second writer taking the file over, which is exactly
/// what a harvest is, needs this to continue the sequence rather than restart
/// it.
pub fn next_seq(path: &Path) -> Result<u64> {
    let text =
        std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    Ok(text.lines().filter(|line| !line.trim().is_empty()).count() as u64)
}

/// Six fields on every line, plus the body.
#[derive(Serialize)]
struct Envelope<'a> {
    run_id: &'a str,
    parent_run_id: Option<&'a str>,
    seq: u64,
    ts: String,
    provider: Option<&'a str>,
    model: Option<&'a str>,
    secret_flag: Option<Vec<&'static str>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    raw: Option<&'a RawValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    hydra: Option<&'a RawValue>,
}

/// The only process that appends to a run file.
///
/// Contract section 2: at most one writer at a time, and the child is never that
/// process. Ownership transfers only to a process that has proved the recorded pid
/// is gone, which this slice never does because it harvests nothing.
pub struct Writer {
    file: File,
    path: PathBuf,
    run_id: String,
    parent_run_id: Option<String>,
    provider: Option<String>,
    model: Option<String>,
    seq: u64,
}

impl Writer {
    /// Creates the run file. Refuses to open one that exists, because a second
    /// writer on one file is the thing the contract spends section 2 forbidding.
    pub fn create(
        path: impl Into<PathBuf>,
        run_id: &str,
        parent_run_id: Option<&str>,
        provider: Option<&str>,
        model: Option<&str>,
    ) -> Result<Self> {
        let path = path.into();
        let file = File::options()
            .write(true)
            .create_new(true)
            .open(&path)
            .with_context(|| format!("creating {}", path.display()))?;
        Ok(Self {
            file,
            path,
            run_id: run_id.to_string(),
            parent_run_id: parent_run_id.map(str::to_string),
            provider: provider.map(str::to_string),
            model: model.map(str::to_string),
            seq: 0,
        })
    }

    /// Reopens a run file to append the harvest, contract 10's "ownership
    /// transfers to whoever proves the previous owner is dead."
    ///
    /// The caller has already made that proof: [`crate::live::has_run_end`] said
    /// there is no closing line yet, which is as far as a liveness test can go
    /// before the pid check one level up decides the owner is gone. `seq`
    /// continues from [`next_seq`] rather than from zero, so a harvester never
    /// repeats or skips a number the file already used.
    pub fn resume(
        path: impl Into<PathBuf>,
        run_id: &str,
        parent_run_id: Option<&str>,
        provider: Option<&str>,
        model: Option<&str>,
        next_seq: u64,
    ) -> Result<Self> {
        let path = path.into();
        let file = File::options()
            .append(true)
            .open(&path)
            .with_context(|| format!("opening {} to append", path.display()))?;
        Ok(Self {
            file,
            path,
            run_id: run_id.to_string(),
            parent_run_id: parent_run_id.map(str::to_string),
            provider: provider.map(str::to_string),
            model: model.map(str::to_string),
            seq: next_seq,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Backfills the model the provider reported.
    ///
    /// Contract 4: `model` is what the provider reported, or what Hydra passed when
    /// it reports none. Line 0 is already on disk by the time a provider says which
    /// model answered, and 5.1 settles that shape for `provider_session_id`:
    /// backfill in memory, never rewrite line 0.
    pub fn set_model(&mut self, model: String) {
        self.model = Some(model);
    }

    /// One Hydra line: `run_start` or `run_end`, nothing else.
    pub fn hydra_line(&mut self, body: &impl Serialize) -> Result<u64> {
        let body = serde_json::value::to_raw_value(body)?;
        self.append(None, Some(&body))
    }

    /// One provider frame, verbatim.
    ///
    /// `raw` holds the frame's own bytes: `RawValue` round-trips the text rather
    /// than re-serializing a parsed value, so key order and number spelling survive.
    /// A line the provider wrote that is not valid JSON is stored as a JSON string
    /// instead of dropped, because never-discard has no exception for noise.
    pub fn provider_line(&mut self, line: &str) -> Result<u64> {
        match serde_json::from_str::<&RawValue>(line) {
            Ok(raw) => {
                let flags = secrets::scan(line);
                self.append_with_flags(Some(raw), None, flags)
            }
            Err(_) => {
                let raw = serde_json::value::to_raw_value(line)?;
                let flags = secrets::scan(line);
                self.append_with_flags(Some(&raw), None, flags)
            }
        }
    }

    fn append(&mut self, raw: Option<&RawValue>, hydra: Option<&RawValue>) -> Result<u64> {
        self.append_with_flags(raw, hydra, None)
    }

    fn append_with_flags(
        &mut self,
        raw: Option<&RawValue>,
        hydra: Option<&RawValue>,
        secret_flag: Option<Vec<&'static str>>,
    ) -> Result<u64> {
        let seq = self.seq;
        let envelope = Envelope {
            run_id: &self.run_id,
            parent_run_id: self.parent_run_id.as_deref(),
            seq,
            ts: Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
            provider: self.provider.as_deref(),
            model: self.model.as_deref(),
            secret_flag,
            raw,
            hydra,
        };
        let mut line = serde_json::to_string(&envelope)?;
        line.push('\n');
        self.file.write_all(line.as_bytes())?;
        // Contract 2: append line-buffered, one sync per line, and readers tail.
        // Agent events arrive at single digits per second, so this is noise on an
        // SSD. Revisit it with evidence from a real load and not before.
        self.file.sync_data()?;
        self.seq += 1;
        Ok(seq)
    }
}
