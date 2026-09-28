//! The seat channel: a gated interactive session asks its own launcher for
//! seats. Map ticket 82, answers 1, 2 and 5, and build ticket 140.
//!
//! In a gated interactive launch `hydra run --interactive` waits outside the
//! seatbelt while `claude` runs inside it, and the delegate face is a child of
//! `claude`, so the face cannot launch anything: the child would be Hydra
//! itself under a profile that denies the corpus (map ticket 64). This module
//! is the one hop map 82 admits past D5. The launcher binds `seats.sock` in the
//! run's own spawn directory before it execs the provider, the face connects,
//! sends one [`Ask`] and reads one [`Reply`], and nothing else crosses.
//!
//! **The kernel scopes the socket to a path, not to a process.** Every run's
//! profile denies connecting to a socket under the spawns directory and allows
//! its own `seats.sock` (`src/sandbox.rs`), so no run reaches another run's
//! launcher. The session's own shell reaches this one as well as its face does,
//! and that was accepted: the launcher is the guard either way. It checks every
//! request against `config/panels.json`, holds everything else for a human,
//! marks every child's text itself, and writes each request to the session's
//! record before it launches anything.
//!
//! **The request names seats, never flags.** A seat's provider, model and
//! policy come from a shape a human approved, or are matched against one. The
//! task is the only free text, and it lands on the session's record verbatim
//! and in each seat's `run_start` with the seat's role in front of it.

use std::collections::BTreeSet;
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, anyhow, bail};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::panels::Shape;
use crate::paths::Layout;
use crate::policy::{Policy, PolicySet};
use crate::provider::Provider;
use crate::record::Writer;
use crate::run::{Call, Request};

/// The socket's name in the run's spawn directory.
pub const SOCKET: &str = "seats.sock";

/// A request that matches no shape. Build ticket 141 is the approval.
pub const APPROVAL_REQUIRED: &str = "hydra/approval-required";
const UNKNOWN_SHAPE: &str = "hydra/unknown-shape";
const NOT_ASKABLE: &str = "hydra/shape-not-askable";
const ABOVE_SESSION: &str = "hydra/seat-above-session";
const PANEL_CAP: &str = "hydra/panel-cap";
const PANELS_UNREADABLE: &str = "hydra/panels-unreadable";
const BAD_REQUEST: &str = "hydra/seat-request";
const SESSION_ENDED: &str = "hydra/session-ended";
const REQUEST_CAP: &str = "hydra/request-cap";
const BUSY: &str = "hydra/seat-channel-busy";

/// The most requests one session makes in an hour, whatever the launcher
/// decides about them. Build ticket 142: every request is a line on the
/// session's record, so without this a loop of refused requests grows the
/// record without bound, and each line reads the whole file for its `seq`.
pub const REQUESTS_PER_HOUR: usize = 60;

/// The most a request line may hold. A task is prose, and 64 KB of it is
/// already more than any panel anybody meant to ask for.
pub const MAX_REQUEST: usize = 64 << 10;

/// How long [`Served::stop`] waits for a seat still inside `run::run` to hand
/// off to its detached watcher. The human has left the session by then and is
/// waiting on this, so it is bounded; a seat still launching after it dies with
/// the process and is named on stderr.
pub const HANDOFF_BUDGET: Duration = Duration::from_secs(60);

/// How long the accept loop rests after `accept` fails, so a failure that
/// repeats (out of descriptors, say) does not spin a core.
const ACCEPT_BACKOFF: Duration = Duration::from_millis(50);

const HOUR: Duration = Duration::from_secs(3600);

/// `seats.sock` in `run_id`'s spawn directory.
pub fn socket_path(layout: &Layout, run_id: &str) -> PathBuf {
    layout.spawn(run_id).join(SOCKET)
}

/// What the face sends: a shape by name, or the seats it wants.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ask {
    pub task: String,
    #[serde(default)]
    pub shape: Option<String>,
    #[serde(default)]
    pub seats: Option<Vec<AskedSeat>>,
}

/// One seat as a request names it. A seat names no role: the role is the
/// shape's, and a request that matches no shape has none to take.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AskedSeat {
    pub provider: String,
    #[serde(default)]
    pub model: Option<String>,
    pub policy: String,
}

/// What the launcher answers, which the face hands the model as a tool result.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Reply {
    pub text: String,
    pub is_error: bool,
}

/// Starts one seat. `run::run` in a real session; a stub in the tests.
pub type LaunchSeat = dyn Fn(Request) -> Result<Call> + Send + Sync;

/// The session a launcher serves.
pub struct Session {
    pub run_id: String,
    /// The session's policy, which bounds every seat (map 82 answer 3).
    pub policy: String,
    /// The model on the session's envelope, repeated on every line it adds.
    pub model: Option<String>,
    pub run_file: PathBuf,
    /// Where each seat's worktree is seeded from: the repo the human launched
    /// the session in.
    pub working_dir: PathBuf,
}

/// The half of a gated interactive launch that answers `seats.sock`.
pub struct Launcher {
    session: Session,
    policies: PathBuf,
    panels: PathBuf,
    launch: Box<LaunchSeat>,
    /// When each shape was launched, per shape name, for the cap per hour.
    /// In memory because the cap is per session and this lives exactly as
    /// long as the session does.
    launched: Mutex<HashMap<String, Vec<Instant>>>,
    /// Every request in the last hour, for [`REQUESTS_PER_HOUR`].
    requests: Mutex<Meter>,
    /// Whether the record takes lines, and which seats are still launching.
    flight: Arc<Flight>,
}

/// The requests a session made in the last hour, and whether the record
/// already says the cap is spent.
#[derive(Default)]
struct Meter {
    at: Vec<Instant>,
    said: bool,
}

/// What [`Launcher::meter`] found.
enum Metered {
    Within,
    Spent { first: bool },
}

/// The record's gate and the seats between their `seat_request` line and
/// their handoff, under one lock so a seat is either counted before the
/// record closes or never launched.
#[derive(Default)]
struct Flight {
    state: Mutex<FlightState>,
    landed: Condvar,
}

#[derive(Default)]
struct FlightState {
    /// Set by [`Launcher::close`] before the harvest writes `run_end`.
    closed: bool,
    /// Run ids minted for seats whose launch has not yet handed off.
    launching: BTreeSet<String>,
}

impl Flight {
    fn lock(&self) -> std::sync::MutexGuard<'_, FlightState> {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// `run_id` handed off, or its launch returned. Either may come first,
    /// and the second finds nothing to do.
    fn land(&self, run_id: &str) {
        if self.lock().launching.remove(run_id) {
            self.landed.notify_all();
        }
    }

    /// Waits up to `budget` for every seat to land, and names those that did
    /// not.
    fn settle(&self, budget: Duration) -> Vec<String> {
        let deadline = Instant::now() + budget;
        let mut state = self.lock();
        while !state.launching.is_empty() {
            let Some(left) = deadline.checked_duration_since(Instant::now()) else {
                break;
            };
            state = self
                .landed
                .wait_timeout(state, left)
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .0;
        }
        state.launching.iter().cloned().collect()
    }
}

/// What the launcher decided about one request.
enum Decision {
    Launch(Shape),
    Refuse { code: &'static str, text: String },
    Pending,
}

/// The seats a request asked for, as the record carries them.
#[derive(Serialize)]
struct RecordedSeat {
    provider: String,
    model: Option<String>,
    policy: String,
    role: Option<String>,
}

/// The session record's line for one request.
#[derive(Serialize)]
struct SeatRequest<'a> {
    #[serde(rename = "type")]
    kind: &'static str,
    request_id: &'a str,
    shape: Option<&'a str>,
    seats: Vec<RecordedSeat>,
    task: &'a str,
    /// `launched`, `refused` or `approval_required`.
    decision: &'static str,
    /// The refusal's code, and `hydra/approval-required` for a pending one.
    reason: Option<&'static str>,
    /// The run ids minted for the seats, before any of them exists.
    runs: &'a [String],
}

impl Launcher {
    /// `policies` is the policy directory and `panels` is `config/panels.json`.
    /// Both are read at every request, so a shape the human approves while the
    /// session runs is one the session can ask for.
    pub fn new(
        session: Session,
        policies: PathBuf,
        panels: PathBuf,
        launch: Box<LaunchSeat>,
    ) -> Self {
        Self {
            session,
            policies,
            panels,
            launch,
            launched: Mutex::new(HashMap::new()),
            requests: Mutex::new(Meter::default()),
            flight: Arc::new(Flight::default()),
        }
    }

    /// Answers one request: decides it, writes it to the session's record,
    /// and launches its seats when a shape admits it.
    ///
    /// **Nothing launches unless the record took the line first.** A panel
    /// that is not on the session's record is a panel the corpus cannot find,
    /// which is what map 82 set out to fix.
    pub fn answer(&self, ask: &Ask) -> Reply {
        let request_id = Uuid::now_v7().to_string();
        if let Metered::Spent { first } = self.meter() {
            return self.spent(ask, &request_id, first);
        }
        let (shape_name, seats, decision) = self.weigh(ask);
        let runs: Vec<String> = match &decision {
            Decision::Launch(shape) => shape
                .seats
                .iter()
                .map(|_| Uuid::now_v7().to_string())
                .collect(),
            _ => Vec::new(),
        };
        let (kind, reason) = match &decision {
            Decision::Launch(_) => ("launched", None),
            Decision::Refuse { code, .. } => ("refused", Some(*code)),
            Decision::Pending => ("approval_required", Some(APPROVAL_REQUIRED)),
        };
        let line = SeatRequest {
            kind: "seat_request",
            request_id: &request_id,
            shape: shape_name.as_deref(),
            seats,
            task: &ask.task,
            decision: kind,
            reason,
            runs: &runs,
        };
        if let Err(err) = self.record(&line, &runs) {
            return Reply {
                text: format!(
                    "{SESSION_ENDED}: request {request_id} could not be written to the \
                     session's record, so nothing was launched: {err:#}"
                ),
                is_error: true,
            };
        }
        match decision {
            Decision::Refuse { text, .. } => Reply {
                text,
                is_error: true,
            },
            Decision::Pending => Reply {
                text: format!(
                    "{APPROVAL_REQUIRED}: request {request_id} matches no shape in \
                     config/panels.json, so it waits for a human. No model approves it, \
                     this session included. The request is on this session's record; \
                     `hydra approve {request_id}`, typed outside the session, is build \
                     ticket 141 and does not exist yet, so for now nothing launches it."
                ),
                is_error: true,
            },
            Decision::Launch(shape) => self.launch(&request_id, &shape, &ask.task, &runs),
        }
    }

    /// Stops the record taking lines. Called before the harvest, which writes
    /// `run_end`, so nothing lands after it.
    pub fn close(&self) {
        self.flight.lock().closed = true;
    }

    /// Counts one request against [`REQUESTS_PER_HOUR`]. A spent cap counts
    /// nothing, so it frees up an hour after the requests that spent it.
    fn meter(&self) -> Metered {
        let mut meter = self
            .requests
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let now = Instant::now();
        meter.at.retain(|at| now.duration_since(*at) < HOUR);
        if meter.at.len() >= REQUESTS_PER_HOUR {
            let first = !meter.said;
            meter.said = true;
            return Metered::Spent { first };
        }
        meter.at.push(now);
        meter.said = false;
        Metered::Within
    }

    /// The refusal once the cap is spent. The first refused request is a line
    /// on the record, so the corpus sees the session hit the cap; the rest
    /// write nothing, which is what the cap is for.
    fn spent(&self, ask: &Ask, request_id: &str, first: bool) -> Reply {
        if first {
            let line = SeatRequest {
                kind: "seat_request",
                request_id,
                shape: ask.shape.as_deref(),
                seats: asked(ask),
                task: &ask.task,
                decision: "refused",
                reason: Some(REQUEST_CAP),
                runs: &[],
            };
            // The refusal stands whether or not the line lands.
            let _ = self.record(&line, &[]);
        }
        Reply {
            text: format!(
                "{REQUEST_CAP}: this session has made {REQUESTS_PER_HOUR} requests of its \
                 launcher in the last hour, refused ones included, which is the most one \
                 session may. Nothing was launched or written."
            ),
            is_error: true,
        }
    }

    /// The decision, with what the record should say was asked.
    fn weigh(&self, ask: &Ask) -> (Option<String>, Vec<RecordedSeat>, Decision) {
        let asked = asked(ask);
        let refuse = |code, text| Decision::Refuse { code, text };

        if ask.task.trim().is_empty() {
            let text = format!(
                "{BAD_REQUEST}: the task is empty, and a seat told nothing has nothing to do"
            );
            return (ask.shape.clone(), asked, refuse(BAD_REQUEST, text));
        }
        let policies = match PolicySet::load(&self.policies) {
            Ok(policies) => policies,
            Err(err) => {
                let text = format!("{BAD_REQUEST}: the policy set does not load: {err:#}");
                return (ask.shape.clone(), asked, refuse(BAD_REQUEST, text));
            }
        };
        let session = match policies.get(&self.session.policy) {
            Ok(session) => session,
            Err(err) => {
                let text = format!("{BAD_REQUEST}: the session's own policy: {err:#}");
                return (ask.shape.clone(), asked, refuse(BAD_REQUEST, text));
            }
        };
        let panels = match crate::panels::load(&self.panels, &policies) {
            Ok(panels) => panels,
            Err(err) => {
                let text = format!(
                    "{PANELS_UNREADABLE}: config/panels.json does not load, so no shape is \
                     approved and nothing launches: {err:#}"
                );
                return (ask.shape.clone(), asked, refuse(PANELS_UNREADABLE, text));
            }
        };

        match (&ask.shape, &ask.seats) {
            (Some(name), None) => {
                let Some(shape) = panels.shapes.iter().find(|shape| shape.name == *name) else {
                    let names: Vec<&str> = panels
                        .shapes
                        .iter()
                        .map(|shape| shape.name.as_str())
                        .collect();
                    let text = format!(
                        "{UNKNOWN_SHAPE}: no approved shape is named {name:?}. The approved \
                         ones are: {}. Ask for seats with `delegate` to have a human look \
                         at anything else.",
                        if names.is_empty() {
                            "none".to_string()
                        } else {
                            names.join(", ")
                        }
                    );
                    return (Some(name.clone()), asked, refuse(UNKNOWN_SHAPE, text));
                };
                let decision = self.admit(shape, &policies, session);
                (Some(name.clone()), recorded(shape), decision)
            }
            (None, Some(seats)) if !seats.is_empty() => {
                for (index, seat) in seats.iter().enumerate() {
                    if let Err(err) = seat.provider.parse::<Provider>() {
                        let text = format!("{BAD_REQUEST}: seat {index}: {err:#}");
                        return (None, asked, refuse(BAD_REQUEST, text));
                    }
                    let policy = match policies.get(&seat.policy) {
                        Ok(policy) => policy,
                        Err(err) => {
                            let text = format!("{BAD_REQUEST}: seat {index}: {err:#}");
                            return (None, asked, refuse(BAD_REQUEST, text));
                        }
                    };
                    if !session.contains(policy) {
                        let text = above_session(index, &seat.policy, &session.name);
                        return (None, asked, refuse(ABOVE_SESSION, text));
                    }
                }
                let matching: Vec<&Shape> = panels
                    .shapes
                    .iter()
                    .filter(|shape| same_seats(shape, seats))
                    .collect();
                if let Some(shape) = matching
                    .iter()
                    .find(|shape| shape.askers.contains(&self.session.policy))
                {
                    let decision = self.admit(shape, &policies, session);
                    return (Some(shape.name.clone()), recorded(shape), decision);
                }
                if let Some(shape) = matching.first() {
                    let text = not_askable(shape, &self.session.policy);
                    return (Some(shape.name.clone()), asked, refuse(NOT_ASKABLE, text));
                }
                (None, asked, Decision::Pending)
            }
            _ => {
                let text = format!(
                    "{BAD_REQUEST}: a request names a shape or lists its seats, one of the two"
                );
                (ask.shape.clone(), asked, refuse(BAD_REQUEST, text))
            }
        }
    }

    /// Whether `shape` launches for this session now: its askers, its seats
    /// against the session's policy, and its cap per hour, in that order.
    fn admit(&self, shape: &Shape, policies: &PolicySet, session: &Policy) -> Decision {
        if !shape.askers.contains(&self.session.policy) {
            return Decision::Refuse {
                code: NOT_ASKABLE,
                text: not_askable(shape, &self.session.policy),
            };
        }
        // `panels::load` already refused a shape whose seats exceed one of its
        // askers. Checked again because the policy directory is read afresh.
        for (index, seat) in shape.seats.iter().enumerate() {
            let within = policies
                .get(&seat.policy)
                .is_ok_and(|policy| session.contains(policy));
            if !within {
                return Decision::Refuse {
                    code: ABOVE_SESSION,
                    text: above_session(index, &seat.policy, &session.name),
                };
            }
        }
        let mut launched = self
            .launched
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let times = launched.entry(shape.name.clone()).or_default();
        let now = Instant::now();
        times.retain(|at| now.duration_since(*at) < HOUR);
        if times.len() >= shape.cap_per_hour as usize {
            return Decision::Refuse {
                code: PANEL_CAP,
                text: format!(
                    "{PANEL_CAP}: shape {} may be asked for {} times an hour and this \
                     session has asked for it {} times in the last hour. The cap is the \
                     human's, in config/panels.json.",
                    shape.name,
                    shape.cap_per_hour,
                    times.len()
                ),
            };
        }
        times.push(now);
        Decision::Launch(shape.clone())
    }

    /// Launches every seat of `shape` at once, as children of the session,
    /// and marks each child's text once all of them have stopped.
    fn launch(&self, request_id: &str, shape: &Shape, task: &str, runs: &[String]) -> Reply {
        let requests: Vec<Request> = shape
            .seats
            .iter()
            .zip(runs)
            .map(|(seat, run_id)| Request {
                provider: seat.provider,
                task: seat_task(&seat.role, task),
                model: seat.model.clone(),
                policy: seat.policy.clone(),
                floor_lifted: false,
                timeout: crate::face::TIMEOUT,
                parent: Some(self.session.run_id.clone()),
                run_id: Some(run_id.clone()),
                command: "run".to_string(),
                resumed_from: None,
                resume_session_id: None,
                working_dir: Some(self.session.working_dir.clone()),
                quiet: true,
                handed_off: Some({
                    let flight = self.flight.clone();
                    let run_id = run_id.clone();
                    Box::new(move || flight.land(&run_id))
                }),
            })
            .collect();
        let calls: Vec<Result<Call>> = std::thread::scope(|scope| {
            let handles: Vec<_> = requests
                .into_iter()
                .zip(runs)
                .map(|(request, run_id)| {
                    scope.spawn(move || {
                        let call = (self.launch)(request);
                        self.flight.land(run_id);
                        call
                    })
                })
                .collect();
            handles
                .into_iter()
                .map(|handle| {
                    handle
                        .join()
                        .unwrap_or_else(|_| Err(anyhow!("the seat's launch panicked")))
                })
                .collect()
        });

        // Harness spec 8: drawn after every child stopped, so no child could
        // have written it.
        let token = crate::face::token();
        let seats: Vec<Value> = shape
            .seats
            .iter()
            .zip(calls)
            .map(|(seat, call)| {
                let result = match call {
                    Ok(Call::Ended(result)) => crate::face::marked(&result, &token)
                        .unwrap_or_else(|err| json!({ "error": format!("{err:#}") })),
                    Ok(Call::Handle(handle)) => {
                        serde_json::to_value(&handle).unwrap_or(Value::Null)
                    }
                    Err(err) => json!({ "error": format!("{err:#}") }),
                };
                json!({
                    "role": seat.role,
                    "provider": seat.provider.binary(),
                    "model": seat.model,
                    "policy": seat.policy,
                    "result": result,
                })
            })
            .collect();
        let body = json!({
            "request_id": request_id,
            "shape": shape.name,
            "seats": seats,
        });
        Reply {
            text: serde_json::to_string_pretty(&body).unwrap_or_else(|_| body.to_string()),
            is_error: false,
        }
    }

    /// Appends one line to the session's record, under the record's lock,
    /// and counts `runs` as launching once it has.
    fn record(&self, line: &SeatRequest<'_>, runs: &[String]) -> Result<()> {
        let mut flight = self.flight.lock();
        if flight.closed {
            bail!("the session has ended and its record takes no more lines");
        }
        let run_file = &self.session.run_file;
        let _lock = crate::record::lock_exclusive(run_file)?;
        if crate::live::has_run_end(run_file)? {
            bail!("the session's record already carries its run_end");
        }
        let seq = crate::record::next_seq(run_file)?;
        let mut writer = Writer::resume(
            run_file,
            &self.session.run_id,
            None,
            Some(Provider::Claude.binary()),
            self.session.model.as_deref(),
            seq,
        )?;
        writer.hydra_line(line)?;
        flight.launching.extend(runs.iter().cloned());
        Ok(())
    }
}

/// The seats a request listed, as the record carries them.
fn asked(ask: &Ask) -> Vec<RecordedSeat> {
    ask.seats
        .iter()
        .flatten()
        .map(|seat| RecordedSeat {
            provider: seat.provider.clone(),
            model: seat.model.clone(),
            policy: seat.policy.clone(),
            role: None,
        })
        .collect()
}

fn recorded(shape: &Shape) -> Vec<RecordedSeat> {
    shape
        .seats
        .iter()
        .map(|seat| RecordedSeat {
            provider: seat.provider.binary().to_string(),
            model: seat.model.clone(),
            policy: seat.policy.clone(),
            role: Some(seat.role.clone()),
        })
        .collect()
}

/// Whether `asked` is `shape`'s seats, in any order, role aside.
fn same_seats(shape: &Shape, asked: &[AskedSeat]) -> bool {
    let mut theirs: Vec<(String, Option<String>, String)> = shape
        .seats
        .iter()
        .map(|seat| {
            (
                seat.provider.binary().to_string(),
                seat.model.clone(),
                seat.policy.clone(),
            )
        })
        .collect();
    let mut ours: Vec<(String, Option<String>, String)> = asked
        .iter()
        .map(|seat| match seat.provider.parse::<Provider>() {
            Ok(provider) => (
                provider.binary().to_string(),
                seat.model.clone(),
                seat.policy.clone(),
            ),
            Err(_) => (
                seat.provider.clone(),
                seat.model.clone(),
                seat.policy.clone(),
            ),
        })
        .collect();
    theirs.sort();
    ours.sort();
    theirs == ours
}

fn not_askable(shape: &Shape, policy: &str) -> String {
    format!(
        "{NOT_ASKABLE}: shape {} may be asked for by sessions under {}, and this session \
         runs under {policy}. Nothing was launched.",
        shape.name,
        shape.askers.join(", ")
    )
}

fn above_session(index: usize, seat: &str, session: &str) -> String {
    format!(
        "{ABOVE_SESSION}: seat {index} asks for {seat}, which grants more than {session}, \
         this session's policy. A seat never gets more than the session that asked for it \
         (map ticket 82). Nothing was launched."
    )
}

/// What a seat is told: its role, then the task.
fn seat_task(role: &str, task: &str) -> String {
    format!("Your seat on this panel: {role}.\n\n{task}")
}

/// How much of the launcher one session's connections may hold.
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    /// Connections answered at once. The next is refused and closed, so a
    /// flood costs a refusal each rather than a thread each.
    pub connections: usize,
    /// How long a connection has to send its whole request line, from accept.
    pub deadline: Duration,
    /// How long a reply may take to write before the launcher gives up on a
    /// reader that stopped reading.
    pub write_timeout: Duration,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            connections: 4,
            deadline: Duration::from_secs(10),
            write_timeout: Duration::from_secs(10),
        }
    }
}

/// The launcher listening on its socket, until [`Served::stop`] or a drop.
pub struct Served {
    launcher: Arc<Launcher>,
    socket: PathBuf,
    stopping: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}

/// Binds `socket` and answers it on a thread of its own, under the default
/// [`Limits`].
///
/// **On its own thread because the launcher blocks** waiting on `claude`. Each
/// connection gets a thread too, up to [`Limits::connections`], so a
/// connection that holds its line back cannot stall the next one, and a panel
/// that takes ten minutes does not hold the accept loop.
pub fn serve(launcher: Arc<Launcher>, socket: &Path) -> Result<Served> {
    serve_with(launcher, socket, Limits::default())
}

/// [`serve`] under `limits`, which the tests shorten.
pub fn serve_with(launcher: Arc<Launcher>, socket: &Path, limits: Limits) -> Result<Served> {
    use std::os::unix::fs::PermissionsExt;
    let listener = UnixListener::bind(socket)
        .with_context(|| format!("binding the seat channel at {}", socket.display()))?;
    std::fs::set_permissions(socket, std::fs::Permissions::from_mode(0o600))
        .with_context(|| format!("restricting {}", socket.display()))?;
    let stopping = Arc::new(AtomicBool::new(false));
    let thread = {
        let launcher = launcher.clone();
        let stopping = stopping.clone();
        std::thread::Builder::new()
            .name("hydra-seats".into())
            .spawn(move || accept(listener, launcher, stopping, limits))
            .context("starting the seat channel's thread")?
    };
    Ok(Served {
        launcher,
        socket: socket.to_path_buf(),
        stopping,
        thread: Some(thread),
    })
}

impl Served {
    /// Closes the session's record to new lines, stops accepting, removes the
    /// socket, and waits up to [`HANDOFF_BUDGET`] for every seat still inside
    /// `run::run` to hand off to its detached watcher. Returns the run ids of
    /// the seats that had not, which die with this process.
    pub fn stop(self) -> Vec<String> {
        self.stop_within(HANDOFF_BUDGET)
    }

    /// [`Served::stop`] with its own budget.
    pub fn stop_within(mut self, budget: Duration) -> Vec<String> {
        self.shut();
        self.launcher.flight.settle(budget)
    }

    /// Everything [`Served::stop`] does but the wait.
    fn shut(&mut self) {
        let Some(thread) = self.thread.take() else {
            return;
        };
        self.launcher.close();
        self.stopping.store(true, Ordering::SeqCst);
        // Wakes the accept loop so it sees the flag. If this connect fails
        // the loop is left blocked, and the process exiting takes it.
        if UnixStream::connect(&self.socket).is_ok() {
            let _ = thread.join();
        }
        let _ = std::fs::remove_file(&self.socket);
    }
}

impl Drop for Served {
    fn drop(&mut self) {
        self.shut();
    }
}

/// Gives a connection's slot back when its thread ends, or when the thread
/// never started.
struct Slot(Arc<AtomicUsize>);

impl Drop for Slot {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

fn accept(
    listener: UnixListener,
    launcher: Arc<Launcher>,
    stopping: Arc<AtomicBool>,
    limits: Limits,
) {
    let in_flight = Arc::new(AtomicUsize::new(0));
    for stream in listener.incoming() {
        if stopping.load(Ordering::SeqCst) {
            return;
        }
        let Ok(stream) = stream else {
            std::thread::sleep(ACCEPT_BACKOFF);
            continue;
        };
        let _ = stream.set_write_timeout(Some(limits.write_timeout));
        if in_flight.fetch_add(1, Ordering::SeqCst) >= limits.connections {
            in_flight.fetch_sub(1, Ordering::SeqCst);
            let busy = Reply {
                text: format!(
                    "{BUSY}: the launcher is already answering {} requests from this \
                     session. Nothing was launched or written; ask again when one returns.",
                    limits.connections
                ),
                is_error: true,
            };
            // A short timeout here, because this runs on the accept thread.
            let _ = stream.set_write_timeout(Some(Duration::from_millis(100)));
            let _ = send(&stream, &busy);
            continue;
        }
        let slot = Slot(in_flight.clone());
        let launcher = launcher.clone();
        // Nothing reports a failed conversation: the launcher shares the
        // human's terminal with the session, and a line on stderr lands in
        // the middle of the provider's screen. The face reads a closed
        // connection as a failure and says so to the model.
        let _ = std::thread::Builder::new()
            .name("hydra-seat-request".into())
            .spawn(move || {
                let _slot = slot;
                let _ = converse(stream, &launcher, limits.deadline);
            });
    }
}

/// One line in, one line out.
fn converse(stream: UnixStream, launcher: &Launcher, deadline: Duration) -> Result<()> {
    let reply = match read_request(&stream, deadline) {
        Ok(line) => match serde_json::from_str::<Ask>(&line) {
            Ok(ask) => launcher.answer(&ask),
            Err(err) => Reply {
                text: format!("{BAD_REQUEST}: the request did not parse: {err}"),
                is_error: true,
            },
        },
        Err(err) => Reply {
            text: format!("{BAD_REQUEST}: {err:#}"),
            is_error: true,
        },
    };
    send(&stream, &reply)
}

/// Reads one request line, all of it within `deadline` of the call. A per-read
/// timeout alone lets a client that sends a byte at a time hold its thread
/// for as long as it likes.
fn read_request(stream: &UnixStream, deadline: Duration) -> Result<String> {
    let until = Instant::now() + deadline;
    let mut line = Vec::new();
    let mut chunk = [0u8; 8192];
    loop {
        let Some(left) = until
            .checked_duration_since(Instant::now())
            .filter(|left| !left.is_zero())
        else {
            bail!("no whole request line within {deadline:?}");
        };
        stream.set_read_timeout(Some(left))?;
        let read = match (&*stream).read(&mut chunk) {
            Ok(read) => read,
            Err(err)
                if matches!(
                    err.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) =>
            {
                bail!("no whole request line within {deadline:?}");
            }
            Err(err) => return Err(err.into()),
        };
        if read == 0 {
            break;
        }
        line.extend_from_slice(&chunk[..read]);
        if let Some(end) = line.iter().position(|&byte| byte == b'\n') {
            line.truncate(end);
            break;
        }
        if line.len() > MAX_REQUEST {
            bail!("the request is longer than {MAX_REQUEST} bytes");
        }
    }
    if line.len() > MAX_REQUEST {
        bail!("the request is longer than {MAX_REQUEST} bytes");
    }
    String::from_utf8(line).context("the request is not UTF-8")
}

fn send(mut stream: &UnixStream, reply: &Reply) -> Result<()> {
    writeln!(stream, "{}", serde_json::to_string(reply)?)?;
    stream.flush()?;
    Ok(())
}

/// Whether `parent` has a launcher socket, which is whether it is a gated
/// interactive session.
pub fn launcher_present(layout: &Layout, parent: &str) -> bool {
    is_run_id(parent) && std::fs::symlink_metadata(socket_path(layout, parent)).is_ok()
}

/// Asks `parent`'s launcher, from the face.
///
/// `Ok(None)` when there is no launcher to ask: no socket, which is every run
/// but a gated interactive session, or a socket nobody listens on, which is a
/// session whose launcher has gone. The caller then does what it did before
/// this channel existed.
pub fn ask(layout: &Layout, parent: &str, ask: &Ask) -> Result<Option<Reply>> {
    if !is_run_id(parent) {
        return Ok(None);
    }
    let socket = socket_path(layout, parent);
    match std::fs::symlink_metadata(&socket) {
        Ok(_) => {}
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => {
            return Err(err).with_context(|| format!("looking for {}", socket.display()));
        }
    }
    let mut stream = match UnixStream::connect(&socket) {
        Ok(stream) => stream,
        Err(err) if err.kind() == std::io::ErrorKind::ConnectionRefused => return Ok(None),
        Err(err) => {
            return Err(err).with_context(|| format!("connecting to {}", socket.display()));
        }
    };
    writeln!(stream, "{}", serde_json::to_string(ask)?)?;
    stream.flush()?;
    stream
        .shutdown(std::net::Shutdown::Write)
        .context("closing the request")?;
    let mut line = String::new();
    BufReader::new(stream)
        .read_line(&mut line)
        .context("reading the launcher's answer")?;
    if line.trim().is_empty() {
        bail!("the session's launcher closed the channel without answering");
    }
    let reply = serde_json::from_str(&line).context("the launcher's answer did not parse")?;
    Ok(Some(reply))
}

/// A run id's alphabet, so a `--parent` cannot walk out of the spawns
/// directory. The kernel refuses another run's socket anyway; this keeps the
/// face from asking.
fn is_run_id(text: &str) -> bool {
    !text.is_empty() && text.chars().all(|c| c.is_ascii_hexdigit() || c == '-')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_parent_that_is_not_a_run_id_names_no_socket() {
        assert!(is_run_id("01a0e7ae-aba4-7513-9241-af73e8d11a73"));
        assert!(!is_run_id("../01a0e7ae"));
        assert!(!is_run_id(""));
    }

    #[test]
    fn a_seat_is_told_its_role_before_the_task() {
        let told = seat_task("judge", "read the diff");
        assert!(told.starts_with("Your seat on this panel: judge."));
        assert!(told.ends_with("read the diff"));
    }
}
