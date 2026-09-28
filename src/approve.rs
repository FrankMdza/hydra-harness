//! `hydra approve`, map ticket 82 answer 4 and build ticket 141.
//!
//! A seat request that matches no shape in `config/panels.json` waits for a
//! human. The launcher, outside the seatbelt, writes it into the store as
//! `<id>.pending.json` and keeps the face's call open. The human types
//! `hydra approve <id>` in a terminal outside the session, which renames the
//! file to `<id>.approved.json`; the launcher sees the rename, writes a
//! `seat_approval` line on the session's record, launches, and answers the face
//! with the marked results. A request nobody approves in time, or whose session
//! ends first, is renamed to `<id>.withdrawn.json` by the launcher.
//!
//! **Every change of state is one `rename`, so the two writers cannot both
//! win.** The human renames `pending` to `approved`, the launcher renames
//! `pending` to `withdrawn`, and whichever rename finds the file gone lost.
//!
//! **The kernel is the guard, not this command's refusal.** The store is under
//! `~/hydra` and every run's profile denies it reads and writes
//! (`src/policy/mod.rs`, `src/sandbox.rs`), so a session cannot approve its own
//! request whatever it types. The command also refuses inside a seatbelt, as
//! the delegate face does, so a model running it gets a refusal by name rather
//! than a kernel error.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use crate::paths::Layout;
use crate::seats::AskedSeat;

/// The command was typed inside a seatbelt, which is where every model runs.
pub const INSIDE_SEATBELT: &str = "hydra/approve-inside-seatbelt";
/// No request by that id is waiting.
pub const NO_PENDING_REQUEST: &str = "hydra/no-pending-request";
/// The request was approved already.
pub const ALREADY_APPROVED: &str = "hydra/already-approved";
/// The request stopped waiting before anybody approved it.
pub const WITHDRAWN: &str = "hydra/request-withdrawn";

const PENDING: &str = "pending";
const APPROVED: &str = "approved";
const WITHDRAWN_STATE: &str = "withdrawn";

/// One request waiting for a human: what the listing shows before the human
/// types its id.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pending {
    pub request_id: String,
    /// The session that asked, by run id.
    pub session: String,
    /// The session's policy, which bounds every seat it asks for.
    pub session_policy: String,
    pub task: String,
    pub seats: Vec<AskedSeat>,
    pub asked_at: String,
}

/// What `hydra approve` does, typed by a human: approve `request_id`, or list
/// what waits when there is none. Refused inside a seatbelt either way.
pub fn command(layout: &Layout, request_id: Option<&str>) -> Result<String> {
    if crate::sandbox::inside_seatbelt() {
        bail!(
            "{INSIDE_SEATBELT}: hydra approve runs in a terminal outside every session. \
             This process is inside a seatbelt, which is where a model runs, and no \
             model approves a seat request (map ticket 82). Nothing was written."
        );
    }
    match request_id {
        Some(id) => {
            let approved = approve(layout, id)?;
            Ok(format!(
                "approved {}: session {} launches it now\n",
                approved.request_id, approved.session
            ))
        }
        None => Ok(listing(&pending(layout)?)),
    }
}

/// Approves one waiting request. The caller has already refused a seatbelt;
/// [`command`] is the entry point that does.
pub fn approve(layout: &Layout, request_id: &str) -> Result<Pending> {
    let dir = layout.approvals();
    if !is_request_id(request_id) {
        bail!("{NO_PENDING_REQUEST}: {request_id:?} is not a request id");
    }
    let from = entry(&dir, request_id, PENDING);
    let to = entry(&dir, request_id, APPROVED);
    // Read before the rename so the answer can say whose request it was. A
    // file that vanishes in between is caught by the rename.
    let request = read(&from).ok();
    // A launcher killed outside its own shutdown leaves its requests pending
    // with nobody waiting on them. Approving one would launch nothing, and
    // the human would read "approved" all the same.
    if let Some(request) = &request
        && !crate::seats::launcher_present(layout, &request.session)
        && let Ok(Settled::Withdrawn) = withdraw(&dir, request_id)
    {
        bail!(
            "{WITHDRAWN}: request {request_id}'s session {} has no launcher any more, so \
             nothing is waiting to launch it. It is withdrawn.",
            request.session
        );
    }
    match std::fs::rename(&from, &to) {
        Ok(()) => {}
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            if to.exists() {
                bail!("{ALREADY_APPROVED}: request {request_id} was approved already");
            }
            if entry(&dir, request_id, WITHDRAWN_STATE).exists() {
                bail!(
                    "{WITHDRAWN}: request {request_id} stopped waiting before it was \
                     approved, because its session ended or nobody approved it in time. \
                     The session has to ask again."
                );
            }
            bail!("{NO_PENDING_REQUEST}: no request {request_id} is waiting for approval");
        }
        Err(err) => {
            return Err(err).with_context(|| format!("approving {}", from.display()));
        }
    }
    match request {
        Some(request) => Ok(request),
        None => read(&to),
    }
}

/// Every request waiting for a human, oldest first.
pub fn pending(layout: &Layout) -> Result<Vec<Pending>> {
    let dir = layout.approvals();
    let entries = match std::fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(err).with_context(|| format!("reading {}", dir.display())),
    };
    let suffix = format!(".{PENDING}.json");
    let mut out = Vec::new();
    for entry in entries {
        let path = entry?.path();
        let named = path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.ends_with(&suffix));
        if !named {
            continue;
        }
        match read(&path) {
            Ok(request) => out.push(request),
            // Approved or withdrawn between the listing and the read.
            Err(_) if !path.exists() => {}
            Err(err) => return Err(err),
        }
    }
    out.sort_by(|a, b| a.request_id.cmp(&b.request_id));
    Ok(out)
}

/// The listing `hydra approve` prints with no id, so the human reads what a
/// request asks for before typing its id.
pub fn listing(pending: &[Pending]) -> String {
    if pending.is_empty() {
        return "nothing is waiting for approval\n".to_string();
    }
    let mut out = String::new();
    for request in pending {
        out.push_str(&format!(
            "request {}\n  asked at {} by session {} under {}\n",
            request.request_id, request.asked_at, request.session, request.session_policy
        ));
        for (index, seat) in request.seats.iter().enumerate() {
            out.push_str(&format!(
                "  seat {index}: {} model {} policy {}\n",
                seat.provider,
                seat.model.as_deref().unwrap_or("(the provider's default)"),
                seat.policy
            ));
        }
        out.push_str("  task:\n");
        for line in request.task.lines() {
            out.push_str(&format!("    {line}\n"));
        }
        out.push_str(&format!(
            "  to approve: hydra approve {}\n\n",
            request.request_id
        ));
    }
    out
}

/// How a wait for a human ended.
pub(crate) enum Settled {
    Approved,
    Withdrawn,
}

/// Writes `request` into the store as waiting. The launcher's half.
pub(crate) fn hold(dir: &Path, request: &Pending) -> Result<()> {
    use std::io::Write;
    use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(dir)
        .with_context(|| format!("creating {}", dir.display()))?;
    // Written whole under a name no listing reads, then renamed in, so a
    // human never lists half a request.
    let partial = dir.join(format!(".{}.partial", request.request_id));
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&partial)
        .with_context(|| format!("creating {}", partial.display()))?;
    file.write_all(serde_json::to_string_pretty(request)?.as_bytes())?;
    file.sync_all()?;
    let path = entry(dir, &request.request_id, PENDING);
    std::fs::rename(&partial, &path).with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}

/// Whether the human has approved `request_id`.
pub(crate) fn approved(dir: &Path, request_id: &str) -> bool {
    entry(dir, request_id, APPROVED).exists()
}

/// Stops `request_id` waiting, unless the human approved it first.
pub(crate) fn withdraw(dir: &Path, request_id: &str) -> Result<Settled> {
    let from = entry(dir, request_id, PENDING);
    match std::fs::rename(&from, entry(dir, request_id, WITHDRAWN_STATE)) {
        Ok(()) => Ok(Settled::Withdrawn),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound && approved(dir, request_id) => {
            Ok(Settled::Approved)
        }
        Err(err) => Err(err).with_context(|| format!("withdrawing {}", from.display())),
    }
}

fn entry(dir: &Path, request_id: &str, state: &str) -> PathBuf {
    dir.join(format!("{request_id}.{state}.json"))
}

fn read(path: &Path) -> Result<Pending> {
    let text =
        std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    serde_json::from_str(&text).with_context(|| format!("parsing {}", path.display()))
}

/// A request id's alphabet, so the id a human types cannot name a path outside
/// the store.
fn is_request_id(text: &str) -> bool {
    !text.is_empty() && text.chars().all(|c| c.is_ascii_hexdigit() || c == '-')
}
