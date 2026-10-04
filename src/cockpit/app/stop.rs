//! Stepping in on a seat, build ticket 184.

use std::sync::mpsc::{self, Receiver, TryRecvError};

use anyhow::Context;

use super::text::{draft, short};
use super::{App, Effect};
use crate::cockpit::snapshot::Card;
use crate::live::State;
use crate::paths::Layout;

/// A stop `y` started, build ticket 186: [`crate::stop::stop`] on a thread
/// of its own, whose answer [`App::poll_stops`] takes.
pub(super) struct StopRun {
    pub(super) card: Card,
    answer: Receiver<anyhow::Result<Vec<crate::stop::Stopped>>>,
}

impl App {
    /// `m`: opens the confirmation that stops the seat selected in the live
    /// lane, or the live seat selected in an agent's. With a chat inside, a
    /// seat is a run the chat's session launched; without one, any live run
    /// with a parent.
    pub(super) fn ask_to_stop(&mut self) -> Effect {
        let card = self
            .selected_card()
            .filter(|card| card.state == State::Live);
        let Some(card) = card else {
            self.message = Some(if self.by_agent {
                "select a live seat first".to_string()
            } else {
                "select a seat in the live lane first".to_string()
            });
            return Effect::None;
        };
        let id = short(&card.run_id);
        self.poll_stops();
        if self.stops.iter().any(|run| run.card.run_id == card.run_id) {
            self.message = Some(format!(
                "{id} is stopping already; its outcome shows here when the stop reports"
            ));
            return Effect::None;
        }
        let refused = match (&card.parent_run_id, self.chat_inside) {
            (None, _) => Some(format!("{id} is not a seat: no run launched it")),
            (Some(parent), true) if self.session.as_deref() != Some(parent.as_str()) => {
                Some(format!("{id} is not a seat of this session"))
            }
            _ => None,
        };
        if let Some(refused) = refused {
            self.message = Some(refused);
            return Effect::None;
        }
        if self.seatbelted {
            self.message = Some(format!(
                "{}: the cockpit is inside a seatbelt, which is where a model runs, and no \
                 model stops a run. Nothing was signalled.",
                crate::stop::INSIDE_SEATBELT
            ));
            return Effect::None;
        }
        self.confirming_stop = Some(card);
        Effect::None
    }

    /// `y` on that confirmation: starts [`crate::stop::stop`] on the seat on a
    /// thread of its own and returns at once, build ticket 186.
    ///
    /// The stop waits for the record to close, up to [`crate::stop::GRACE`]
    /// and [`crate::stop::CLOSE_BUDGET`], and the cockpit keeps drawing and
    /// reading keys meanwhile. A thread rather than a child `hydra stop`: the
    /// thread hands back [`crate::stop::Stopped`] as it is, where a child
    /// would find the policies with `discover` rather than the set this
    /// cockpit reads, refuse inside a seatbelt again, and answer in text to
    /// parse. The marker is written and the signal sent before the thread
    /// waits, so a cockpit that quits meanwhile leaves the seat stopping.
    pub(super) fn stop_seat(&mut self) -> Effect {
        let Some(card) = self.confirming_stop.take() else {
            return Effect::None;
        };
        let id = short(&card.run_id).to_string();
        let policies = match self
            .policies_dir
            .as_ref()
            .context("no policy directory to read the outcome with")
            .and_then(crate::policy::PolicySet::load)
        {
            Ok(policies) => policies,
            Err(err) => {
                self.message = Some(format!("{id} was not stopped: {err:#}"));
                return Effect::Refresh;
            }
        };
        let layout = Layout::at(self.layout.root());
        let run_id = card.run_id.clone();
        let (tx, answer) = mpsc::channel();
        let started = std::thread::Builder::new()
            .name(format!("stop-{id}"))
            .spawn(move || {
                let stopped = crate::stop::stop(
                    &layout,
                    &policies,
                    crate::stop::Target::Run(&run_id),
                    crate::stop::GRACE,
                );
                // The cockpit may have quit, and then nobody is left to tell.
                let _ = tx.send(stopped);
            });
        if let Err(err) = started {
            self.message = Some(format!("{id} was not stopped: starting the stop: {err}"));
            return Effect::None;
        }
        self.message = Some(format!("stopping {id}"));
        self.stops.push(StopRun { card, answer });
        Effect::None
    }

    /// Takes the answer of each stop that reported, [`mpsc::Receiver::try_recv`]
    /// and no wait. [`App::draw`] calls it, and so does
    /// [`super::super::host::Host`] before it looks for a note to paste.
    pub fn poll_stops(&mut self) {
        let mut i = 0;
        while i < self.stops.len() {
            let answer = match self.stops[i].answer.try_recv() {
                Err(TryRecvError::Empty) => {
                    i += 1;
                    continue;
                }
                Ok(answer) => answer,
                Err(TryRecvError::Disconnected) => {
                    Err(anyhow::anyhow!("the stop's thread ended without an answer"))
                }
            };
            let run = self.stops.remove(i);
            self.stop_reported(run.card, answer);
        }
    }

    /// What one stop's answer leaves: the outcome on its card and the status
    /// line, and with a chat inside, the note for [`App::take_drafts`]. A
    /// failed stop leaves its error and no note.
    fn stop_reported(&mut self, card: Card, answer: anyhow::Result<Vec<crate::stop::Stopped>>) {
        let id = short(&card.run_id).to_string();
        let stopped = match answer {
            Ok(stopped) => stopped,
            Err(err) => {
                self.message = Some(format!("{id} was not stopped: {err:#}"));
                return;
            }
        };
        let said = crate::stop::report(&stopped).trim_end().to_string();
        let outcome = stopped
            .iter()
            .find(|run| run.run_id == card.run_id)
            .and_then(|run| run.outcome)
            .map_or("stop sent, record open", |outcome| outcome.as_str());
        self.stop_outcomes
            .push((card.run_id.clone(), outcome.to_string()));
        if !self.chat_inside {
            self.message = Some(format!("{said}; tell the session yourself"));
            return;
        }
        self.message = Some(format!(
            "{said}; the note to the session is in the chat, unsent"
        ));
        let task = super::super::snapshot::line_zero(&self.layout.run_file(&card.run_id))
            .and_then(|line| serde_json::from_str::<serde_json::Value>(&line).ok())
            .and_then(|envelope| envelope["hydra"]["task"].as_str().map(str::to_string));
        self.drafted
            .push(draft(&card, task.as_deref(), self.preamble.as_deref()));
    }

    /// Whether a stop `y` started has not reported yet.
    pub fn stopping(&self) -> bool {
        !self.stops.is_empty()
    }

    /// The notes the stops that reported left for the chat, oldest first, once.
    pub fn take_drafts(&mut self) -> Vec<String> {
        std::mem::take(&mut self.drafted)
    }
}
