//! The cockpit's state, its event handling and its drawing, with no terminal.
//!
//! [`App::draw`] paints into a [`Buffer`] and [`App::handle`] takes a crossterm
//! [`Event`], so the suite drives both without a screen. Every rectangle a
//! click can land on is kept from the last draw in [`Hits`], and a click is a
//! `Rect::contains` over those.

use std::collections::HashMap;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, ExitStatus, Stdio};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::time::{Duration, Instant};

use anyhow::{Context, bail};

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, MouseButton, MouseEventKind};
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Layout as Split, Position, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, Paragraph, Widget, Wrap};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use super::call_head;
use super::goal::{Goal, TaskRow};
use super::keys::MainKey;
use super::mascot::Mascot;
use super::snapshot::{CallLine, Card, Lane, REVEAL_SECONDS, Snapshot, Step, spend_text};
use crate::approve::Pending;
use crate::live::State;
use crate::paths::Layout;

/// Which panel has the keyboard.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Focus {
    Table,
    /// A lane by state. With the lanes by agent only `Lane::Waiting` is drawn.
    Lane(Lane),
    /// With the lanes by agent, the lane of the agent at this place in
    /// [`App::agents`]. Build ticket 191.
    Agent(usize),
    Detail,
}

/// The agents whose lanes come first, in this order; any other follows by name.
const AGENT_ORDER: [&str; 4] = ["claude", "cursor-agent", "agy", "codex"];

/// The agent a card goes under: its provider, or `?` for a record without one.
fn agent_of(card: &Card) -> &str {
    card.provider.as_deref().unwrap_or("?")
}

/// What the detail panel has open.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Opened {
    Run(String),
    Request(String),
    /// A Master of Puppets step, by [`Step::label`]. Build ticket 182.
    Step(String),
}

/// Where the keyboard was before the first full-page run opened.
#[derive(Clone, Debug)]
struct CockpitPlace {
    focus: Focus,
    table_selected: usize,
    lane_selected: [usize; 4],
    agent_selected: HashMap<String, usize>,
    table_offset: usize,
    lane_offset: [usize; 4],
    agent_offset: HashMap<String, usize>,
}

/// A full-page run under the one on screen.
#[derive(Clone, Debug)]
struct PageLayer {
    opened: Opened,
    detail_scroll: u16,
    detail_call: usize,
    raw_tail: bool,
}

/// What `a` has in hand in the waiting lane.
enum InHand {
    Request(Pending),
    Step(Step),
}

/// How long `y` on a step waits for `hydra-puppets approve`. Build ticket 182.
pub const APPROVE_BUDGET: Duration = Duration::from_secs(5);

/// What the loop around [`App::handle`] has to do next.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Effect {
    None,
    /// Ask the reader for a fresh snapshot now, with the open run's tail.
    Refresh,
    /// Give the terminal to the human's editor on this policy file, then hand
    /// what happened to [`App::edited`]. The cockpit never writes the file.
    Edit(PathBuf),
    Quit,
}

/// The rectangles of the last draw.
#[derive(Clone, Debug, Default)]
pub struct Hits {
    pub table: Rect,
    /// A drawn table row and the index of the task it shows.
    pub table_rows: Vec<(Rect, usize)>,
    pub lanes: [Rect; 4],
    /// With the lanes by agent, each agent's lane in [`App::agents`] order.
    pub agent_lanes: Vec<Rect>,
    /// A drawn card, the lane it is in, and its index in that lane.
    pub cards: Vec<(Rect, Focus, usize)>,
    pub detail: Rect,
    /// The full-page run view, when one is open.
    pub run_page: Rect,
    /// The policy pane, while it is up, how many runs it had rows for, and
    /// the offset of its last page.
    pub policy_pane: Rect,
    pub policy_rows: usize,
    pub policy_last: usize,
}

/// A review this cockpit started, build ticket 183: `hydra-puppets review <goal>
/// <task>` in its own process group, both streams in `log`.
struct ReviewRun {
    goal: String,
    task: String,
    log: PathBuf,
    /// How many reviews of the task the goal log held when it started, so
    /// an older verdict is not read as this one's.
    before: usize,
    /// The helper while it lives. [`App::poll_reviews`] reaps it.
    child: Option<Child>,
    /// How it exited, or `None` while it lives or when the wait failed.
    status: Option<ExitStatus>,
    /// The `hunk show ...` line the helper printed to its log, build ticket
    /// 187: `--no-open` prints the command rather than running it.
    open: Option<String>,
}

/// A stop `y` started, build ticket 186: [`crate::stop::stop`] on a thread
/// of its own, whose answer [`App::poll_stops`] takes.
struct StopRun {
    card: Card,
    answer: Receiver<anyhow::Result<Vec<crate::stop::Stopped>>>,
}

/// Width the header keeps for the title beside the dragon, and the rows the panels keep below it.
const TITLE_WIDTH: u16 = 30;
const MIN_BODY: u16 = 12;

pub struct App {
    layout: Layout,
    /// Whether this process is inside a seatbelt. The binary passes
    /// [`crate::sandbox::inside_seatbelt`], the check `hydra approve` makes;
    /// the suite, which may itself run inside one, names it.
    seatbelted: bool,
    interval: Duration,
    pub snapshot: Snapshot,
    pub focus: Focus,
    pub table_selected: usize,
    pub lane_selected: [usize; 4],
    table_offset: usize,
    lane_offset: [usize; 4],
    /// Whether the middle column has a lane per agent rather than the four by
    /// state. `k` flips it, build ticket 191.
    pub by_agent: bool,
    /// The selection and first drawn card of each agent's lane, by agent, so
    /// an agent whose lane moves keeps them.
    agent_selected: HashMap<String, usize>,
    agent_offset: HashMap<String, usize>,
    detail_scroll: u16,
    /// Which tool call or raw tail line the detail has selected, build 206.
    detail_call: usize,
    /// While set, the selected call or line shows its whole input, build 206.
    reveal_until: Option<Instant>,
    revealed_call: Option<CallLine>,
    revealed_line: Option<String>,
    /// Whether the detail shows the raw record lines rather than the tool
    /// calls. `r` flips it, build ticket 178.
    raw_tail: bool,
    pub opened: Option<Opened>,
    /// The cockpit to return to after the last `Left`, build ticket 245.
    run_page_place: Option<CockpitPlace>,
    /// Older full-page runs, newest at the end.
    page_stack: Vec<PageLayer>,
    /// The last rectangle [`draw_run_page`] used, for keys between draws.
    run_page_area: Rect,
    /// The request the confirmation shows, while it is up.
    pub confirming: Option<Pending>,
    /// Whether that confirmation denies rather than approves, build ticket 170.
    pub denying: bool,
    /// The Master of Puppets step the confirmation shows, while it is up,
    /// build ticket 182.
    pub confirming_step: Option<Step>,
    /// Steps approved here, by label and revision, with when. An approval
    /// does not move the goal's revision, so the reader's list, up to ten
    /// seconds old, would offer the same step again; it stays out until a
    /// `pending` read newer than the approval.
    approved_steps: Vec<(String, u64, chrono::DateTime<chrono::Utc>)>,
    pub message: Option<String>,
    pub hits: Hits,
    /// The dragon in the header, or `None` for `--no-mascot` and for a frame directory that
    /// would not load.
    mascot: Option<Mascot>,
    /// Whether the cockpit's panels have the keyboard. False while the chat
    /// of build ticket 151 has it, and then no panel border is lit.
    pub has_keys: bool,
    /// The table above the lanes and the detail, for the narrower column the
    /// cockpit gets beside the chat.
    pub stacked: bool,
    /// `policies/`, which the pane lists and the editor opens files in.
    policies_dir: Option<PathBuf>,
    /// The set's sha as last loaded, or the loader's error.
    policy_set: Option<std::result::Result<String, String>>,
    /// Whether the policy pane of build ticket 171 is up.
    pub policy_pane: bool,
    /// The key help overlay, build ticket 240.
    pub help_pane: bool,
    policy_selected: usize,
    /// The first live run the pane shows, build ticket 174.
    policy_offset: usize,
    /// `hydra-puppets` and the goal log home it is handed, build ticket 183.
    reviewer: Option<(PathBuf, PathBuf)>,
    /// Why there is no helper to run, when there is none. Build ticket 221.
    no_helper: Option<String>,
    /// The goal, task and run the review confirmation asks about, while it is up.
    pub confirming_review: Option<(String, String, String)>,
    reviews: Vec<ReviewRun>,
    /// The seat the stop confirmation asks about, while it is up, build
    /// ticket 184.
    pub confirming_stop: Option<Card>,
    /// The stops that have not reported yet, build ticket 186.
    stops: Vec<StopRun>,
    /// Each seat a stop reported on and the outcome it returned, kept while
    /// the seat's card is still live.
    stop_outcomes: Vec<(String, String)>,
    /// The notes the stops that reported left for the chat, in the order
    /// they reported, until the host takes them. Build ticket 187.
    drafted: Vec<String>,
    /// Whether a chat is inside, so `m` drafts a note to it after a stop.
    pub(crate) chat_inside: bool,
    /// The run the chat named, once it has: the session whose seats `m` stops.
    session: Option<String>,
    /// The preamble every `run_start.task` begins with, which the note leaves
    /// out, or `None` when it would not load.
    preamble: Option<String>,
}

impl App {
    pub fn new(layout: Layout, seatbelted: bool, interval: Duration) -> Self {
        Self {
            layout,
            seatbelted,
            interval,
            snapshot: Snapshot::default(),
            focus: Focus::Table,
            table_selected: 0,
            lane_selected: [0; 4],
            table_offset: 0,
            lane_offset: [0; 4],
            by_agent: false,
            agent_selected: HashMap::new(),
            agent_offset: HashMap::new(),
            detail_scroll: 0,
            detail_call: 0,
            reveal_until: None,
            revealed_call: None,
            revealed_line: None,
            raw_tail: false,
            opened: None,
            run_page_place: None,
            page_stack: Vec::new(),
            run_page_area: Rect::default(),
            confirming: None,
            denying: false,
            confirming_step: None,
            approved_steps: Vec::new(),
            message: None,
            hits: Hits::default(),
            mascot: None,
            has_keys: true,
            stacked: false,
            policies_dir: None,
            policy_set: None,
            policy_pane: false,
            help_pane: false,
            policy_selected: 0,
            policy_offset: 0,
            reviewer: None,
            no_helper: None,
            confirming_review: None,
            reviews: Vec::new(),
            confirming_stop: None,
            stops: Vec::new(),
            stop_outcomes: Vec::new(),
            drafted: Vec::new(),
            chat_inside: false,
            session: None,
            preamble: None,
        }
    }

    /// Names the policy directory the pane shows and loads it once.
    pub fn set_policies(&mut self, dir: PathBuf) {
        self.policies_dir = Some(dir);
        self.reload_policies();
    }

    /// Names the helper `v` starts and the goal log home it is told about,
    /// the one the reader folds.
    pub fn set_reviewer(&mut self, script: PathBuf, puppets_home: PathBuf) {
        self.reviewer = Some((script, puppets_home));
    }

    /// [`super::puppets_helper`]'s answer: the helper, or the line `v` and `a`
    /// answer with in place of running anything. Build ticket 221.
    pub fn set_helper(&mut self, helper: Result<PathBuf, String>, puppets_home: PathBuf) {
        match helper {
            Ok(script) => self.set_reviewer(script, puppets_home),
            Err(note) => {
                self.reviewer = None;
                self.no_helper = Some(note);
            }
        }
    }

    fn reload_policies(&mut self) {
        self.policy_set = self.policies_dir.as_ref().map(|dir| {
            crate::policy::PolicySet::load(dir)
                .map(|set| set.sha().to_string())
                .map_err(|err| format!("{err:#}"))
        });
    }

    /// What the loop reports once the editor on `file` has exited: reloads
    /// the set, so the pane shows the sha the next launch would record, or
    /// the loader's refusal.
    pub fn edited(&mut self, file: &Path, ran: anyhow::Result<()>) {
        self.reload_policies();
        let name = file.file_name().map_or_else(
            || file.display().to_string(),
            |n| n.to_string_lossy().to_string(),
        );
        self.message = Some(match (ran, &self.policy_set) {
            (Err(err), _) => format!("the editor on {name}: {err:#}"),
            (Ok(()), Some(Ok(sha))) => format!("edited {name}: the set is now {}", short_sha(sha)),
            (Ok(()), _) => format!("edited {name}: the set does not load"),
        });
    }

    /// Puts the dragon in the header, or takes it out and gives the row back to the table.
    pub fn set_mascot(&mut self, mascot: Option<Mascot>) {
        self.mascot = mascot;
    }

    /// Names the preamble the note `m` drafts cuts off a seat's task.
    pub fn set_preamble(&mut self, preamble: Option<String>) {
        self.preamble = preamble;
    }

    /// Takes `run_id` as the session, once the chat has named it: the dragon
    /// watches it and `m` stops only its seats.
    pub fn watch(&mut self, run_id: &str) {
        if self.session.as_deref() != Some(run_id) {
            let focused = self.agent_at(self.focus);
            self.session = Some(run_id.to_string());
            self.refocus_agent(focused);
        }
        if let Some(mascot) = &mut self.mascot {
            mascot.watch(run_id);
        }
    }

    pub fn mascot(&self) -> Option<&Mascot> {
        self.mascot.as_ref()
    }

    /// Takes a fresh read and keeps every selection inside it. With the
    /// policy pane up it reloads the set too, so a file changed outside the
    /// cockpit shows at the next refresh rather than at the next `e`.
    /// Lets the suite expire an open reveal without waiting, build ticket 206.
    pub fn expire_reveal(&mut self) {
        self.reveal_until = Some(Instant::now() - Duration::from_secs(1));
    }

    pub fn set_snapshot(&mut self, snapshot: Snapshot) {
        if self.snapshot.goal.as_ref().map(|goal| &goal.id)
            != snapshot.goal.as_ref().map(|goal| &goal.id)
            && (self.confirming_step.is_some() || self.confirming_review.is_some())
        {
            self.confirming_step = None;
            self.confirming_review = None;
            self.message = Some(
                snapshot
                    .goal
                    .as_ref()
                    .and_then(|goal| {
                        goal.damaged_at.map(|line| {
                            format!(
                                "damaged at line {line}: run hydra-puppets repair {}",
                                goal.id
                            )
                        })
                    })
                    .unwrap_or_else(|| {
                        "the shown goal changed; confirmation cancelled".to_string()
                    }),
            );
        }
        if self.policy_pane {
            self.reload_policies();
        }
        if let Some(mascot) = &mut self.mascot {
            mascot.observe(&snapshot);
        }
        let focused = self.agent_at(self.focus);
        self.snapshot = snapshot;
        self.refocus_agent(focused);
        let live: Vec<&str> = self
            .cards_in(Lane::Live)
            .map(|card| card.run_id.as_str())
            .collect();
        let kept: Vec<(String, String)> = self
            .stop_outcomes
            .iter()
            .filter(|(run_id, _)| live.contains(&run_id.as_str()))
            .cloned()
            .collect();
        self.stop_outcomes = kept;
        self.hide_approved_steps();
        let call_len = self.detail_call_len();
        if call_len > 0 {
            self.detail_call = self.detail_call.min(call_len - 1);
        } else {
            self.detail_call = 0;
        }
        self.table_selected = self.table_selected.min(self.table_len().saturating_sub(1));
        for lane in Lane::ALL {
            let i = lane.index();
            self.lane_selected[i] =
                self.lane_selected[i].min(self.lane_len(lane).saturating_sub(1));
        }
        let agents = self.agents();
        self.agent_selected
            .retain(|agent, _| agents.contains(agent));
        for agent in &agents {
            let len = self.cards_of(agent).count();
            if let Some(selected) = self.agent_selected.get_mut(agent) {
                *selected = (*selected).min(len.saturating_sub(1));
            }
        }
    }

    /// Drops the steps approved here since the reader last asked `pending`,
    /// and forgets the approvals a newer read has answered for.
    fn hide_approved_steps(&mut self) {
        let read_at = self.snapshot.steps_read_at;
        self.approved_steps
            .retain(|(_, _, at)| read_at.is_none_or(|read| read <= *at));
        let approved = &self.approved_steps;
        self.snapshot.steps.retain(|step| {
            !approved
                .iter()
                .any(|(label, revision, _)| *label == step.label() && *revision == step.revision)
        });
    }

    /// The run whose tail the reader should read.
    pub fn open_run_id(&self) -> Option<String> {
        match &self.opened {
            Some(Opened::Run(id)) => Some(id.clone()),
            _ => None,
        }
    }

    fn table_len(&self) -> usize {
        self.snapshot
            .goal
            .as_ref()
            .map_or(0, |goal| goal.rows.len())
    }

    fn cards_in(&self, lane: Lane) -> impl Iterator<Item = &Card> {
        self.snapshot
            .cards
            .iter()
            .filter(move |card| card.lane() == lane)
    }

    pub fn lane_len(&self, lane: Lane) -> usize {
        match lane {
            Lane::Waiting => self.snapshot.requests.len() + self.snapshot.steps.len(),
            _ => self.cards_in(lane).count(),
        }
    }

    /// The run ids in a lane, in drawn order. Empty for `Waiting`.
    pub fn lane_runs(&self, lane: Lane) -> Vec<String> {
        self.cards_in(lane)
            .map(|card| card.run_id.clone())
            .collect()
    }

    /// The runs the lanes by agent show. With a session named, only it and
    /// the runs it launched, the ones the dragon counts; otherwise every run.
    fn agent_cards(&self) -> impl Iterator<Item = &Card> {
        let session = self.session.as_deref();
        self.snapshot.cards.iter().filter(move |card| {
            session.is_none_or(|session| {
                card.run_id == session || card.parent_run_id.as_deref() == Some(session)
            })
        })
    }

    /// The agents with a lane by agent, in the order drawn: [`AGENT_ORDER`],
    /// then any other by name.
    pub fn agents(&self) -> Vec<String> {
        let mut agents: Vec<&str> = self.agent_cards().map(agent_of).collect();
        agents.sort_by_key(|agent| {
            let at = AGENT_ORDER.iter().position(|known| known == agent);
            (at.unwrap_or(AGENT_ORDER.len()), *agent)
        });
        agents.dedup();
        agents.into_iter().map(str::to_string).collect()
    }

    fn cards_of<'a>(&'a self, agent: &'a str) -> impl Iterator<Item = &'a Card> {
        self.agent_cards()
            .filter(move |card| agent_of(card) == agent)
    }

    /// The agent whose lane `focus` is, while the snapshot still has one there.
    fn agent_at(&self, focus: Focus) -> Option<String> {
        match focus {
            Focus::Agent(i) => self.agents().into_iter().nth(i),
            _ => None,
        }
    }

    /// Puts the keyboard back on `agent`'s lane after the lanes moved, or on
    /// the waiting lane when `agent` has none left.
    fn refocus_agent(&mut self, agent: Option<String>) {
        let Some(agent) = agent else {
            return;
        };
        self.focus = match self.agents().iter().position(|a| *a == agent) {
            Some(i) => Focus::Agent(i),
            None => Focus::Lane(Lane::Waiting),
        };
    }

    /// The panels `Tab` visits, in order.
    fn order(&self) -> Vec<Focus> {
        let mut order = vec![Focus::Table];
        if self.by_agent {
            order.extend((0..self.agents().len()).map(Focus::Agent));
            order.push(Focus::Lane(Lane::Waiting));
        } else {
            order.extend(Lane::ALL.map(Focus::Lane));
        }
        order.push(Focus::Detail);
        order
    }

    fn step_focus(&mut self, forward: bool) {
        let order = self.order();
        let at = order.iter().position(|f| *f == self.focus).unwrap_or(0);
        let len = order.len();
        self.focus = order[if forward {
            (at + 1) % len
        } else {
            (at + len - 1) % len
        }];
    }

    /// How many items the lane `focus` holds.
    fn len_of(&self, focus: Focus) -> usize {
        match focus {
            Focus::Lane(lane) => self.lane_len(lane),
            Focus::Agent(_) => self
                .agent_at(focus)
                .map_or(0, |agent| self.cards_of(&agent).count()),
            _ => 0,
        }
    }

    /// Where the lane `focus` was last drawn.
    fn rect_of(&self, focus: Focus) -> Rect {
        match focus {
            Focus::Lane(lane) => self.hits.lanes[lane.index()],
            Focus::Agent(i) => self.hits.agent_lanes.get(i).copied().unwrap_or_default(),
            _ => Rect::default(),
        }
    }

    fn selected(&self, focus: Focus) -> usize {
        match focus {
            Focus::Lane(lane) => self.lane_selected[lane.index()],
            Focus::Agent(_) => self
                .agent_at(focus)
                .and_then(|agent| self.agent_selected.get(&agent).copied())
                .unwrap_or(0),
            _ => 0,
        }
    }

    fn set_selected(&mut self, focus: Focus, index: usize) {
        match focus {
            Focus::Lane(lane) => self.lane_selected[lane.index()] = index,
            Focus::Agent(_) => {
                if let Some(agent) = self.agent_at(focus) {
                    self.agent_selected.insert(agent, index);
                }
            }
            _ => {}
        }
    }

    fn offset(&self, focus: Focus) -> usize {
        match focus {
            Focus::Lane(lane) => self.lane_offset[lane.index()],
            Focus::Agent(_) => self
                .agent_at(focus)
                .and_then(|agent| self.agent_offset.get(&agent).copied())
                .unwrap_or(0),
            _ => 0,
        }
    }

    fn set_offset(&mut self, focus: Focus, offset: usize) {
        match focus {
            Focus::Lane(lane) => self.lane_offset[lane.index()] = offset,
            Focus::Agent(_) => {
                if let Some(agent) = self.agent_at(focus) {
                    self.agent_offset.insert(agent, offset);
                }
            }
            _ => {}
        }
    }

    /// What `Enter` would open in the lane `focus` at `index`.
    fn item(&self, focus: Focus, index: usize) -> Option<Opened> {
        let run = |card: &Card| Opened::Run(card.run_id.clone());
        match focus {
            Focus::Lane(Lane::Waiting) => match self.snapshot.requests.get(index) {
                Some(request) => Some(Opened::Request(request.request_id.clone())),
                None => self
                    .snapshot
                    .steps
                    .get(index - self.snapshot.requests.len())
                    .map(|step| Opened::Step(step.label())),
            },
            Focus::Lane(lane) => self.cards_in(lane).nth(index).map(run),
            Focus::Agent(_) => {
                let agent = self.agent_at(focus)?;
                self.cards_of(&agent).nth(index).map(run)
            }
            Focus::Table | Focus::Detail => None,
        }
    }

    /// The run selected in the lane that has the keyboard, in either view.
    fn selected_card(&self) -> Option<Card> {
        match self.item(self.focus, self.selected(self.focus))? {
            Opened::Run(id) => self
                .snapshot
                .cards
                .iter()
                .find(|card| card.run_id == id)
                .cloned(),
            _ => None,
        }
    }

    /// `k`: flips the middle column between the lanes by state and a lane
    /// per agent, build ticket 191. A selected run stays selected, in the
    /// lane the other view puts it in; the waiting lane is in both.
    fn toggle_lanes(&mut self) -> Effect {
        let card = self.selected_card();
        let in_lanes = matches!(self.focus, Focus::Lane(_) | Focus::Agent(_));
        self.by_agent = !self.by_agent;
        if !in_lanes || self.focus == Focus::Lane(Lane::Waiting) {
            return Effect::None;
        }
        self.focus = match card.and_then(|card| self.focus_on(&card)) {
            Some(focus) => focus,
            None => self.order()[1],
        };
        Effect::None
    }

    /// Selects `card` in the lane the current view puts it in, and returns
    /// that lane, or `None` when the view does not show it.
    fn focus_on(&mut self, card: &Card) -> Option<Focus> {
        let focus = if self.by_agent {
            let agent = agent_of(card);
            Focus::Agent(self.agents().iter().position(|a| a == agent)?)
        } else {
            Focus::Lane(card.lane())
        };
        let index = match focus {
            Focus::Agent(_) => self
                .cards_of(agent_of(card))
                .position(|c| c.run_id == card.run_id)?,
            _ => self
                .cards_in(card.lane())
                .position(|c| c.run_id == card.run_id)?,
        };
        self.set_selected(focus, index);
        Some(focus)
    }

    // --- events ----------------------------------------------------------

    pub fn handle(&mut self, event: &Event) -> Effect {
        match event {
            Event::Key(key) if key.kind != KeyEventKind::Release => self.key(key),
            Event::Mouse(mouse) => {
                let at = Position::new(mouse.column, mouse.row);
                match mouse.kind {
                    MouseEventKind::Down(MouseButton::Left) => self.click(at),
                    MouseEventKind::ScrollUp => self.wheel(at, false),
                    MouseEventKind::ScrollDown => self.wheel(at, true),
                    _ => Effect::None,
                }
            }
            _ => Effect::None,
        }
    }

    fn key(&mut self, key: &KeyEvent) -> Effect {
        if self.reveal_until.is_some() {
            self.close_reveal();
            return Effect::None;
        }
        if self.confirming_step.is_some() {
            return match key.code {
                KeyCode::Char('y') => self.approve_step(),
                KeyCode::Char('n') | KeyCode::Esc => {
                    self.confirming_step = None;
                    self.message = Some("nothing approved".to_string());
                    Effect::None
                }
                _ => Effect::None,
            };
        }
        if self.confirming_stop.is_some() {
            return match key.code {
                KeyCode::Char('y') => self.stop_seat(),
                KeyCode::Char('n') | KeyCode::Esc => {
                    self.confirming_stop = None;
                    self.message = Some("nothing stopped".to_string());
                    Effect::None
                }
                _ => Effect::None,
            };
        }
        if self.confirming_review.is_some() {
            return match key.code {
                KeyCode::Char('y') => self.start_review(),
                KeyCode::Char('n') | KeyCode::Esc => {
                    self.confirming_review = None;
                    self.message = Some("no review started".to_string());
                    Effect::None
                }
                _ => Effect::None,
            };
        }
        if self.confirming.is_some() {
            return match key.code {
                KeyCode::Char('y') => self.answer(),
                KeyCode::Char('n') | KeyCode::Esc => {
                    self.confirming = None;
                    let verb = if self.denying { "denied" } else { "approved" };
                    self.message = Some(format!("nothing {verb}"));
                    Effect::None
                }
                _ => Effect::None,
            };
        }
        if key.code == KeyCode::Char('?') {
            self.help_pane = !self.help_pane;
            return Effect::None;
        }
        if self.help_pane {
            return match key.code {
                KeyCode::Esc => {
                    self.help_pane = false;
                    Effect::None
                }
                KeyCode::Char('a') if self.chat_inside => {
                    self.drafted
                        .push(super::keys::COCKPIT_ASK_TEMPLATE.to_string());
                    Effect::None
                }
                _ => Effect::None,
            };
        }
        if self.policy_pane {
            return self.policy_key(key);
        }
        if self.on_run_page() {
            if matches!(key.code, KeyCode::Left | KeyCode::Backspace) {
                return self.page_back_one();
            }
            if self.focus == Focus::Detail && self.page_detail_key(key) {
                return Effect::None;
            }
        } else if self.focus == Focus::Detail
            && self.opened.is_some()
            && self.detail_scroll_key(key, self.hits.detail, false)
        {
            return Effect::None;
        }
        match super::keys::main_key(key.code) {
            Some(MainKey::Quit) => Effect::Quit,
            Some(MainKey::Policy) => {
                self.reload_policies();
                self.policy_pane = true;
                self.policy_offset = 0;
                Effect::None
            }
            Some(MainKey::FocusForward) => {
                self.step_focus(true);
                Effect::None
            }
            Some(MainKey::FocusBackward) => {
                self.step_focus(false);
                Effect::None
            }
            Some(MainKey::MoveUp) => self.move_selection(false),
            Some(MainKey::MoveDown) => self.move_selection(true),
            Some(MainKey::Open) => self.open_selected(),
            Some(MainKey::CloseOpened) => {
                if self.on_run_page() {
                    return self.page_back_all();
                }
                if self.opened.take().is_some() {
                    self.detail_scroll = 0;
                    return Effect::Refresh;
                }
                Effect::None
            }
            Some(MainKey::ApproveAsk) => self.ask_to_answer(false),
            Some(MainKey::DenyAsk) => self.ask_to_answer(true),
            Some(MainKey::ReviewAsk) => self.ask_to_review(),
            Some(MainKey::StopAsk) => self.ask_to_stop(),
            Some(MainKey::ToggleLanes) => self.toggle_lanes(),
            Some(MainKey::ToggleRawTail) => {
                self.raw_tail = !self.raw_tail;
                self.detail_call = 0;
                Effect::None
            }
            Some(MainKey::ToggleReveal) => self.toggle_reveal(),
            Some(MainKey::ToggleHelp) => Effect::None,
            None => Effect::None,
        }
    }

    fn toggle_reveal(&mut self) -> Effect {
        if self.focus != Focus::Detail || !matches!(&self.opened, Some(Opened::Run(_))) {
            return Effect::None;
        }
        if self.call_or_tail_hidden() {
            self.message = Some("input hidden: secret_flag".to_string());
            return Effect::None;
        }
        let Some(Opened::Run(id)) = &self.opened else {
            return Effect::None;
        };
        if self.raw_tail {
            self.revealed_line = self
                .snapshot
                .tail
                .as_ref()
                .filter(|(tail_id, _)| tail_id == id)
                .and_then(|(_, lines)| lines.get(self.detail_call))
                .cloned();
        } else {
            self.revealed_call = self
                .snapshot
                .calls
                .as_ref()
                .filter(|(calls_id, _)| calls_id == id)
                .and_then(|(_, calls)| calls.get(self.detail_call))
                .cloned();
        }
        if self.revealed_line.is_some() || self.revealed_call.is_some() {
            self.reveal_until = Some(Instant::now() + Duration::from_secs(REVEAL_SECONDS));
        }
        Effect::None
    }

    /// Close a reveal even when the host consumes the key or click itself.
    pub fn close_reveal(&mut self) {
        self.reveal_until = None;
        self.revealed_call = None;
        self.revealed_line = None;
    }

    fn call_or_tail_hidden(&self) -> bool {
        let id = match &self.opened {
            Some(Opened::Run(id)) => id,
            _ => return false,
        };
        if self.raw_tail {
            return self.snapshot.tail.as_ref().is_some_and(|(tail_id, lines)| {
                tail_id == id
                    && lines
                        .get(self.detail_call)
                        .is_some_and(|line| line.starts_with("[line hidden:"))
            });
        }
        self.snapshot
            .calls
            .as_ref()
            .is_some_and(|(calls_id, calls)| {
                calls_id == id
                    && calls
                        .get(self.detail_call)
                        .is_some_and(|call| call.input.starts_with("input hidden:"))
            })
    }

    fn poll_reveal(&mut self) {
        if self
            .reveal_until
            .is_some_and(|until| Instant::now() >= until)
        {
            self.close_reveal();
        }
    }

    fn detail_call_len(&self) -> usize {
        let id = match &self.opened {
            Some(Opened::Run(id)) => id,
            _ => return 0,
        };
        if self.raw_tail {
            return self
                .snapshot
                .tail
                .as_ref()
                .filter(|(tail_id, _)| tail_id == id)
                .map(|(_, lines)| lines.len())
                .unwrap_or(0);
        }
        self.snapshot
            .calls
            .as_ref()
            .filter(|(calls_id, _)| calls_id == id)
            .map(|(_, calls)| calls.len())
            .unwrap_or(0)
    }

    /// Keys while the policy pane is up. It reads; `e` is the one key that
    /// leads anywhere, and it leads to the human's own editor.
    fn policy_key(&mut self, key: &KeyEvent) -> Effect {
        let last = crate::policy::NAMES.len() - 1;
        match key.code {
            KeyCode::Char('q') => Effect::Quit,
            KeyCode::Char('p') | KeyCode::Esc => {
                self.policy_pane = false;
                Effect::None
            }
            KeyCode::Up => {
                self.policy_selected = self.policy_selected.saturating_sub(1);
                Effect::None
            }
            KeyCode::Down => {
                self.policy_selected = (self.policy_selected + 1).min(last);
                Effect::None
            }
            KeyCode::PageUp => {
                let page = self.hits.policy_rows.max(1);
                self.policy_offset = self.policy_offset.saturating_sub(page);
                Effect::None
            }
            KeyCode::PageDown => {
                let page = self.hits.policy_rows.max(1);
                self.scroll_policy_runs(page);
                Effect::None
            }
            KeyCode::Char('e') => {
                let Some(dir) = &self.policies_dir else {
                    self.message = Some("no policy directory to edit".to_string());
                    return Effect::None;
                };
                if self.seatbelted {
                    self.message = Some(
                        "the cockpit is inside a seatbelt, which is where a model runs, and \
                         only the human edits policies/ (protected config #1)"
                            .to_string(),
                    );
                    return Effect::None;
                }
                let name = crate::policy::NAMES[self.policy_selected.min(last)];
                Effect::Edit(dir.join(format!("{name}.json")))
            }
            _ => Effect::None,
        }
    }

    /// Moves the pane's run list down by `by`, no further than its last page.
    fn scroll_policy_runs(&mut self, by: usize) {
        self.policy_offset = (self.policy_offset + by).min(self.hits.policy_last);
    }

    fn move_selection(&mut self, down: bool) -> Effect {
        let step = |value: usize, len: usize| {
            if down {
                (value + 1).min(len.saturating_sub(1))
            } else {
                value.saturating_sub(1)
            }
        };
        match self.focus {
            Focus::Table => {
                self.table_selected = step(self.table_selected, self.table_len());
                let rows = self.hits.table.height.saturating_sub(4) as usize;
                self.table_offset = follow(self.table_selected, self.table_offset, rows);
            }
            focus @ (Focus::Lane(_) | Focus::Agent(_)) => {
                let selected = step(self.selected(focus), self.len_of(focus));
                self.set_selected(focus, selected);
                let rows = self.rect_of(focus).height.saturating_sub(2) as usize;
                let offset = follow(selected, self.offset(focus), rows);
                self.set_offset(focus, offset);
            }
            Focus::Detail => {
                let len = self.detail_call_len();
                if len > 0 {
                    self.detail_call = step(self.detail_call, len);
                } else {
                    self.detail_scroll = if down {
                        self.detail_scroll.saturating_add(1)
                    } else {
                        self.detail_scroll.saturating_sub(1)
                    };
                }
            }
        }
        Effect::None
    }

    fn open_selected(&mut self) -> Effect {
        let opened = match self.focus {
            Focus::Table => self
                .snapshot
                .goal
                .as_ref()
                .and_then(|goal| goal.rows.get(self.table_selected))
                .and_then(|row| row.run_id.clone())
                .map(Opened::Run),
            focus @ (Focus::Lane(_) | Focus::Agent(_)) => self.item(focus, self.selected(focus)),
            Focus::Detail => return Effect::None,
        };
        match opened {
            Some(opened) => self.open(opened),
            None => {
                self.message = Some("nothing to open there".to_string());
                Effect::None
            }
        }
    }

    fn open(&mut self, opened: Opened) -> Effect {
        if matches!(&opened, Opened::Run(_)) {
            return self.open_run_page(opened);
        }
        self.opened = Some(opened);
        self.detail_scroll = 0;
        self.detail_call = 0;
        self.close_reveal();
        Effect::Refresh
    }

    pub fn on_run_page(&self) -> bool {
        self.run_page_place.is_some()
    }

    fn save_place(&self) -> CockpitPlace {
        CockpitPlace {
            focus: self.focus,
            table_selected: self.table_selected,
            lane_selected: self.lane_selected,
            agent_selected: self.agent_selected.clone(),
            table_offset: self.table_offset,
            lane_offset: self.lane_offset,
            agent_offset: self.agent_offset.clone(),
        }
    }

    fn restore_place(&mut self, place: CockpitPlace) {
        self.focus = place.focus;
        self.table_selected = place.table_selected;
        self.lane_selected = place.lane_selected;
        self.agent_selected = place.agent_selected;
        self.table_offset = place.table_offset;
        self.lane_offset = place.lane_offset;
        self.agent_offset = place.agent_offset;
    }

    fn capture_page_layer(&self) -> PageLayer {
        PageLayer {
            opened: self.opened.clone().expect("page without opened"),
            detail_scroll: self.detail_scroll,
            detail_call: self.detail_call,
            raw_tail: self.raw_tail,
        }
    }

    fn restore_page_layer(&mut self, layer: PageLayer) {
        self.opened = Some(layer.opened);
        self.detail_scroll = layer.detail_scroll;
        self.detail_call = layer.detail_call;
        self.raw_tail = layer.raw_tail;
        self.close_reveal();
    }

    fn open_run_page(&mut self, opened: Opened) -> Effect {
        if self.on_run_page() {
            self.page_stack.push(self.capture_page_layer());
        } else {
            self.run_page_place = Some(self.save_place());
        }
        self.opened = Some(opened);
        self.detail_scroll = 0;
        self.detail_call = 0;
        self.focus = Focus::Detail;
        self.close_reveal();
        Effect::Refresh
    }

    fn page_back_one(&mut self) -> Effect {
        if let Some(layer) = self.page_stack.pop() {
            self.restore_page_layer(layer);
            return Effect::Refresh;
        }
        if let Some(place) = self.run_page_place.take() {
            self.opened = None;
            self.detail_scroll = 0;
            self.detail_call = 0;
            self.restore_place(place);
            return Effect::Refresh;
        }
        Effect::None
    }

    fn page_back_all(&mut self) -> Effect {
        if !self.on_run_page() {
            return Effect::None;
        }
        self.page_stack.clear();
        self.opened = None;
        self.detail_scroll = 0;
        self.detail_call = 0;
        if let Some(place) = self.run_page_place.take() {
            self.restore_place(place);
        }
        Effect::Refresh
    }

    fn run_page_footer() -> Line<'static> {
        Line::styled(
            "Up/Down scroll, PageUp/PageDown or Space/b page, Home/g End/G, Left/Backspace back, \
             Esc to cockpit, r raw, i reveal",
            Style::default().add_modifier(Modifier::DIM),
        )
    }

    fn detail_scroll_lines(&self, width: usize, page_footer: bool) -> Vec<Line<'static>> {
        let mut lines = self.detail_lines(width);
        if page_footer {
            lines.push(Line::raw(""));
            lines.push(Self::run_page_footer());
        }
        lines
    }

    fn detail_content_metrics(&self, area: Rect, page_footer: bool) -> (usize, usize, u16) {
        let width = area.width.saturating_sub(2).max(1) as usize;
        let visible = area.height.saturating_sub(2).max(1) as usize;
        let total = self
            .detail_scroll_lines(width, page_footer)
            .iter()
            .map(|line| wrapped_rows(line, width))
            .sum::<usize>();
        let max_scroll = total.saturating_sub(visible) as u16;
        (width, visible, max_scroll)
    }

    fn scroll_detail_lines(&mut self, area: Rect, page_footer: bool, delta: i32) {
        let (_, _, max_scroll) = self.detail_content_metrics(area, page_footer);
        if delta < 0 {
            let step = (-delta) as u16;
            self.detail_scroll = self.detail_scroll.saturating_sub(step);
        } else {
            self.detail_scroll = (self.detail_scroll + delta as u16).min(max_scroll);
        }
    }

    fn detail_scroll_key(&mut self, key: &KeyEvent, area: Rect, page_footer: bool) -> bool {
        match key.code {
            KeyCode::Up => {
                self.scroll_detail_lines(area, page_footer, -1);
                true
            }
            KeyCode::Down => {
                self.scroll_detail_lines(area, page_footer, 1);
                true
            }
            KeyCode::PageUp | KeyCode::Char('b') => {
                let (_, visible, _) = self.detail_content_metrics(area, page_footer);
                self.scroll_detail_lines(area, page_footer, -(visible as i32));
                true
            }
            KeyCode::PageDown | KeyCode::Char(' ') => {
                let (_, visible, _) = self.detail_content_metrics(area, page_footer);
                self.scroll_detail_lines(area, page_footer, visible as i32);
                true
            }
            KeyCode::Home | KeyCode::Char('g') => {
                self.detail_scroll = 0;
                true
            }
            KeyCode::End | KeyCode::Char('G') => {
                let (_, _, max_scroll) = self.detail_content_metrics(area, page_footer);
                self.detail_scroll = max_scroll;
                true
            }
            _ => false,
        }
    }

    fn page_detail_key(&mut self, key: &KeyEvent) -> bool {
        let area = if self.hits.run_page.is_empty() {
            self.run_page_area
        } else {
            self.hits.run_page
        };
        if self.raw_tail {
            let len = self.detail_call_len();
            if len > 0 {
                match key.code {
                    KeyCode::Up => {
                        self.detail_call = self.detail_call.saturating_sub(1);
                        return true;
                    }
                    KeyCode::Down => {
                        self.detail_call = (self.detail_call + 1).min(len - 1);
                        return true;
                    }
                    _ => {}
                }
            }
            return self.detail_scroll_key(key, area, true);
        }
        self.detail_scroll_key(key, area, true)
    }

    fn click(&mut self, at: Position) -> Effect {
        if self.confirming.is_some()
            || self.confirming_step.is_some()
            || self.confirming_review.is_some()
            || self.confirming_stop.is_some()
        {
            return Effect::None;
        }
        if let Some((_, index)) = self.hits.table_rows.iter().find(|(r, _)| r.contains(at)) {
            self.focus = Focus::Table;
            self.table_selected = *index;
            return Effect::None;
        }
        if let Some((_, focus, index)) = self.hits.cards.iter().find(|(r, _, _)| r.contains(at)) {
            let (focus, index) = (*focus, *index);
            self.focus = focus;
            self.set_selected(focus, index);
            return match self.item(focus, index) {
                Some(opened) => self.open(opened),
                None => Effect::None,
            };
        }
        if let Some(focus) = self.panel_at(at) {
            self.focus = focus;
        }
        Effect::None
    }

    fn wheel(&mut self, at: Position, down: bool) -> Effect {
        if self.on_run_page() && self.hits.run_page.contains(at) {
            let step = if down { 3 } else { -3 };
            self.scroll_detail_lines(self.hits.run_page, true, step);
            return Effect::None;
        }
        if self.policy_pane {
            if self.hits.policy_pane.contains(at) {
                if down {
                    self.scroll_policy_runs(1);
                } else {
                    self.policy_offset = self.policy_offset.saturating_sub(1);
                }
            }
            return Effect::None;
        }
        let nudge = |value: usize, max: usize| {
            if down {
                (value + 1).min(max)
            } else {
                value.saturating_sub(1)
            }
        };
        match self.panel_at(at) {
            Some(Focus::Table) => {
                let rows = self.hits.table.height.saturating_sub(4) as usize;
                let max = self.table_len().saturating_sub(rows);
                self.table_offset = nudge(self.table_offset, max);
            }
            Some(focus @ (Focus::Lane(_) | Focus::Agent(_))) => {
                let rows = self.rect_of(focus).height.saturating_sub(2) as usize;
                let max = self.len_of(focus).saturating_sub(rows);
                let offset = nudge(self.offset(focus), max);
                self.set_offset(focus, offset);
            }
            Some(Focus::Detail) => {
                self.detail_scroll = if down {
                    self.detail_scroll.saturating_add(3)
                } else {
                    self.detail_scroll.saturating_sub(3)
                };
            }
            None => {}
        }
        Effect::None
    }

    fn panel_at(&self, at: Position) -> Option<Focus> {
        if self.hits.table.contains(at) {
            return Some(Focus::Table);
        }
        for lane in Lane::ALL {
            if self.hits.lanes[lane.index()].contains(at) {
                return Some(Focus::Lane(lane));
            }
        }
        if let Some(i) = self.hits.agent_lanes.iter().position(|r| r.contains(at)) {
            return Some(Focus::Agent(i));
        }
        self.hits.detail.contains(at).then_some(Focus::Detail)
    }

    // --- the one write -----------------------------------------------------

    fn goal_damaged_message(&self) -> Option<String> {
        let goal = self.snapshot.goal.as_ref()?;
        let line = goal.damaged_at?;
        Some(format!(
            "damaged at line {line}: run hydra-puppets repair {}",
            goal.id
        ))
    }

    /// What `a` would ask about: the request or step open in the detail, or
    /// the one selected in the waiting lane.
    fn in_hand(&self) -> Option<InHand> {
        let opened = match (&self.opened, self.focus) {
            (Some(opened @ (Opened::Request(_) | Opened::Step(_))), _) => opened.clone(),
            (_, focus @ Focus::Lane(Lane::Waiting)) => self.item(focus, self.selected(focus))?,
            _ => return None,
        };
        match opened {
            Opened::Request(id) => self
                .snapshot
                .requests
                .iter()
                .find(|request| request.request_id == id)
                .cloned()
                .map(InHand::Request),
            Opened::Step(label) => self
                .snapshot
                .steps
                .iter()
                .find(|step| step.label() == label)
                .cloned()
                .map(InHand::Step),
            Opened::Run(_) => None,
        }
    }

    /// `a` or `d`: opens the confirmation that approves, or denies, the
    /// request or step in hand. A step has no deny: leaving it is saying no.
    fn ask_to_answer(&mut self, deny: bool) -> Effect {
        let in_hand = self.in_hand();
        // A seat request goes to `hydra approve` and has nothing to do with the goal log.
        if !matches!(in_hand, Some(InHand::Request(_)))
            && let Some(message) = self.goal_damaged_message()
        {
            self.message = Some(message);
            return Effect::None;
        }
        let Some(in_hand) = in_hand else {
            self.message = Some("select a request in the waiting approval lane first".to_string());
            return Effect::None;
        };
        if self.seatbelted {
            self.message = Some(match in_hand {
                InHand::Request(_) => inside_seatbelt(),
                InHand::Step(_) => step_inside_seatbelt(),
            });
            return Effect::None;
        }
        match in_hand {
            InHand::Request(request) => {
                self.confirming = Some(request);
                self.denying = deny;
            }
            InHand::Step(step) if deny => {
                self.message = Some(format!(
                    "{} has no deny here; leave it and it stays waiting",
                    step.label()
                ));
            }
            InHand::Step(step) => self.confirming_step = Some(step),
        }
        Effect::None
    }

    /// `y` on a step's confirmation: runs the argv `pending` listed, as a
    /// child, and waits at most [`APPROVE_BUDGET`]. `hydra-puppets` writes the
    /// goal log; the cockpit never does.
    fn approve_step(&mut self) -> Effect {
        let Some(step) = self.confirming_step.take() else {
            return Effect::None;
        };
        if self.seatbelted {
            self.message = Some(step_inside_seatbelt());
            return Effect::None;
        }
        if let Some(message) = self.goal_damaged_message() {
            self.message = Some(message);
            return Effect::None;
        }
        let label = step.label();
        let Some((program, args)) = step.argv.split_first() else {
            self.message = Some(format!("{label} has no command to run"));
            return Effect::None;
        };
        if let Some(note) = &self.no_helper {
            self.message = Some(format!("{label} was not approved: {note}"));
            return Effect::None;
        }
        let mut command = match super::helper_command(Path::new(program)) {
            Ok(command) => command,
            Err(err) => {
                self.message = Some(format!("{label} was not approved: {err:#}"));
                return Effect::None;
            }
        };
        command.args(args).env("HYDRA_ROOT", self.layout.root());
        if let Some((_, home)) = &self.reviewer {
            command.env("HYDRA_PUPPETS_HOME", home);
        }
        self.message = Some(match super::bounded(command, APPROVE_BUDGET) {
            Ok((super::Bounded::Exited(status, line), _)) => match status.code() {
                Some(0) => {
                    self.approved_steps
                        .push((label.clone(), step.revision, chrono::Utc::now()));
                    self.hide_approved_steps();
                    format!("{label}: {line}")
                }
                Some(2) => format!("{label}: the goal moved on; nothing approved ({line})"),
                // The log holds this approval already, so the step is as
                // good as approved here and leaves the lane the same way.
                Some(3) => {
                    self.approved_steps
                        .push((label.clone(), step.revision, chrono::Utc::now()));
                    self.hide_approved_steps();
                    format!("{label}: already approved, nothing written ({line})")
                }
                _ => format!("{label} not approved, the helper exited with {status}: {line}"),
            },
            Ok((super::Bounded::Late(mut child), _)) => {
                // An approve killed halfway could leave half a line in the
                // log, so it is left to finish and reaped off this thread.
                std::thread::spawn(move || child.wait());
                format!(
                    "{label}: the helper did not answer in {}s and is still running; \
                     the next read shows what it did",
                    APPROVE_BUDGET.as_secs()
                )
            }
            Err(err) => format!("{label} was not approved: {err:#}"),
        });
        Effect::Refresh
    }

    /// `y` on the confirmation: [`crate::approve::approve`] or
    /// [`crate::approve::deny`], the functions `hydra approve <id>` and
    /// `hydra deny <id>` call once they have refused a seatbelt.
    fn answer(&mut self) -> Effect {
        let Some(request) = self.confirming.take() else {
            return Effect::None;
        };
        if self.seatbelted {
            self.message = Some(inside_seatbelt());
            return Effect::None;
        }
        let id = &request.request_id;
        let said = if self.denying {
            crate::approve::deny(&self.layout, id, &crate::approve::login()).map(|denied| {
                format!(
                    "denied {}: session {} is told no",
                    denied.request.request_id, denied.request.session
                )
            })
        } else {
            crate::approve::approve(&self.layout, id).map(|approved| {
                format!(
                    "approved {}: session {} launches it now",
                    approved.request_id, approved.session
                )
            })
        };
        self.message = Some(said.unwrap_or_else(|err| format!("{err:#}")));
        Effect::Refresh
    }

    // --- stepping in on a seat, build ticket 184 -------------------------------

    /// `m`: opens the confirmation that stops the seat selected in the live
    /// lane, or the live seat selected in an agent's. With a chat inside, a
    /// seat is a run the chat's session launched; without one, any live run
    /// with a parent.
    fn ask_to_stop(&mut self) -> Effect {
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
    fn stop_seat(&mut self) -> Effect {
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
    /// [`super::host::Host`] before it looks for a note to paste.
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
        let task = super::snapshot::line_zero(&self.layout.run_file(&card.run_id))
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

    /// What a live card says in its lane: `stopping` while a stop of it
    /// waits, then the outcome that stop returned until a read moves it.
    fn live_label(&self, card: &Card) -> Option<&str> {
        if self.stops.iter().any(|run| run.card.run_id == card.run_id) {
            return Some("stopping");
        }
        self.stop_outcomes
            .iter()
            .find(|(run_id, _)| *run_id == card.run_id)
            .map(|(_, outcome)| outcome.as_str())
    }

    // --- a review, build ticket 183 -----------------------------------------

    /// `v`: opens the confirmation for the selected task's review, which
    /// names what it costs.
    fn ask_to_review(&mut self) -> Effect {
        if let Some(message) = self.goal_damaged_message() {
            self.message = Some(message);
            return Effect::None;
        }
        let row = match (&self.snapshot.goal, self.focus) {
            (Some(goal), Focus::Table) => goal.rows.get(self.table_selected).map(|row| (goal, row)),
            _ => None,
        };
        let Some((goal, row)) = row else {
            self.message = Some("select a task row first".to_string());
            return Effect::None;
        };
        if row.run_id.is_none() {
            self.message = Some(format!("task {} has no run to review", row.id));
            return Effect::None;
        }
        let asked = (goal.id.clone(), row.id.clone(), row.run_id.clone().unwrap());
        self.poll_reviews();
        if self.review_running(&asked.0, &asked.1) {
            self.message = Some(format!("a review of {} is already running", asked.1));
            return Effect::None;
        }
        self.confirming_review = Some(asked);
        Effect::None
    }

    /// `y` on that confirmation: starts the helper, or says why it did not.
    fn start_review(&mut self) -> Effect {
        let Some((goal, task, run_id)) = self.confirming_review.take() else {
            return Effect::None;
        };
        if let Some(message) = self.goal_damaged_message() {
            self.message = Some(message);
            return Effect::None;
        }
        let current_run = self
            .snapshot
            .goal
            .as_ref()
            .filter(|shown| shown.id == goal)
            .and_then(|shown| shown.rows.iter().find(|row| row.id == task))
            .and_then(|row| row.run_id.as_deref());
        if current_run != Some(run_id.as_str()) {
            self.message = Some(format!(
                "task {task} run changed; review confirmation cancelled"
            ));
            return Effect::None;
        }
        self.poll_reviews();
        if self.review_running(&goal, &task) {
            self.message = Some(format!("a review of {task} is already running"));
            return Effect::None;
        }
        match self.spawn_review(&goal, &task) {
            Ok(run) => {
                self.message = Some(format!(
                    "a review of {task} started, its output in {}",
                    run.log.display()
                ));
                self.reviews.retain(|r| !(r.goal == goal && r.task == task));
                self.reviews.push(run);
            }
            Err(err) => {
                self.message = Some(format!("the review of {task} did not start: {err:#}"));
            }
        }
        Effect::None
    }

    /// `hydra-puppets review <goal> <task> --no-open` in a process group of its
    /// own, so a `Ctrl-C` in the cockpit's terminal does not reach it, with
    /// stdin closed and both streams in `$HYDRA_ROOT/cockpit/review-<task>.log`.
    ///
    /// `--no-open`, build ticket 187: the helper has no terminal to open Hunk
    /// in, so it prints the `hunk show ...` command and exits, and the run's
    /// detail shows that line for the human to run. Inside Herdr it starts
    /// the same way and opens no pane.
    fn spawn_review(&self, goal: &str, task: &str) -> anyhow::Result<ReviewRun> {
        if let Some(note) = &self.no_helper {
            bail!("{note}");
        }
        let Some((script, home)) = &self.reviewer else {
            bail!("no review helper is set");
        };
        super::check_goal_id(goal)?;
        if !super::plain_name(task) {
            bail!("{task:?} cannot name a log file");
        }
        let dir = self.layout.root().join("cockpit");
        std::fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
        let log = dir.join(format!("review-{task}.log"));
        let out =
            std::fs::File::create(&log).with_context(|| format!("creating {}", log.display()))?;
        let err = out.try_clone().context("opening the log twice")?;
        let child = super::helper_command(script)?
            .arg("review")
            .arg(goal)
            .arg(task)
            .arg("--no-open")
            .env("HYDRA_ROOT", self.layout.root())
            .env("HYDRA_PUPPETS_HOME", home)
            .stdin(Stdio::null())
            .stdout(out)
            .stderr(err)
            .process_group(0)
            .spawn()
            .with_context(|| format!("starting {}", script.display()))?;
        let before = self
            .snapshot
            .goal
            .as_ref()
            .map_or(0, |g| g.reviews_of(task).0);
        Ok(ReviewRun {
            goal: goal.to_string(),
            task: task.to_string(),
            log,
            before,
            child: Some(child),
            status: None,
            open: None,
        })
    }

    /// Reaps each helper that has exited, `waitpid` with `WNOHANG` through
    /// [`Child::try_wait`], and says so once. [`App::draw`] calls it, and the
    /// loop draws at least every tenth of a second, so `review running` is
    /// gone well inside the four seconds the panel asked for.
    pub fn poll_reviews(&mut self) {
        let mut said = None;
        for run in &mut self.reviews {
            let Some(child) = &mut run.child else {
                continue;
            };
            match child.try_wait() {
                Ok(None) => continue,
                Ok(Some(status)) => run.status = Some(status),
                Err(_) => run.status = None,
            }
            run.child = None;
            if run.status.is_some_and(|status| status.success()) {
                run.open = hunk_line(&run.log);
            }
            said = Some(match run.status {
                Some(status) if status.success() => format!(
                    "the review of {} ended; its verdict shows at the next read",
                    run.task
                ),
                _ => format!("review failed, see {}", run.log.display()),
            });
        }
        if said.is_some() {
            self.message = said;
        }
    }

    /// The `hunk show ...` command the review of `task` this cockpit started
    /// printed, once it ended.
    fn review_open(&self, goal: &str, task: &str) -> Option<&str> {
        self.reviews
            .iter()
            .find(|r| r.goal == goal && r.task == task)
            .and_then(|r| r.open.as_deref())
    }

    fn review_running(&self, goal: &str, task: &str) -> bool {
        self.reviews
            .iter()
            .any(|r| r.goal == goal && r.task == task && r.child.is_some())
    }

    /// What the table cell and the detail say about `task`'s review: the one
    /// this cockpit started while it runs or when it failed, and otherwise the
    /// last verdict in the goal log. `None` when there is neither.
    fn review_of(&self, goal: &Goal, task: &str) -> Option<(&'static str, String)> {
        let (count, last) = goal.reviews_of(task);
        let ours = self
            .reviews
            .iter()
            .find(|r| r.goal == goal.id && r.task == task);
        if let Some(run) = ours {
            let log = run.log.display();
            if run.child.is_some() {
                return Some(("review running", format!("running, output in {log}")));
            }
            // A helper that failed after it recorded a verdict still gave one.
            if count <= run.before {
                return Some(match run.status {
                    Some(status) if status.success() => (
                        "review ended",
                        format!("ended, no verdict read yet, see {log}"),
                    ),
                    _ => ("review failed", format!("review failed, see {log}")),
                });
            }
        }
        let last = last?;
        let verdict = last.verdict.as_deref().unwrap_or("without a verdict");
        let cell = match verdict {
            "pass" => "review pass",
            "block" => "review block",
            _ => "reviewed",
        };
        let degraded = if last.degraded {
            " by a degraded panel"
        } else {
            ""
        };
        Some((cell, format!("{verdict}{degraded}")))
    }

    // --- drawing ------------------------------------------------------------

    /// Paints the whole screen into `buf` and keeps what was drawn where.
    pub fn draw(&mut self, area: Rect, buf: &mut Buffer) {
        self.poll_reviews();
        self.poll_stops();
        self.poll_reveal();
        self.hits = Hits::default();
        let area = self.draw_header(area, buf);
        let [main, status] =
            Split::vertical([Constraint::Min(3), Constraint::Length(1)]).areas(area);
        let [left, middle, right] = if self.stacked {
            let [top, bottom] =
                Split::vertical([Constraint::Percentage(40), Constraint::Percentage(60)])
                    .areas(main);
            let [middle, right] =
                Split::horizontal([Constraint::Percentage(45), Constraint::Percentage(55)])
                    .areas(bottom);
            [top, middle, right]
        } else {
            Split::horizontal([
                Constraint::Percentage(44),
                Constraint::Percentage(26),
                Constraint::Percentage(30),
            ])
            .areas(main)
        };
        self.draw_table(left, buf);
        if self.by_agent {
            self.draw_agent_lanes(middle, buf);
        } else {
            let lanes: [Rect; 4] = Split::vertical([Constraint::Ratio(1, 4); 4]).areas(middle);
            for lane in Lane::ALL {
                self.draw_lane(lane, lanes[lane.index()], buf);
            }
        }
        draw_mode(self.by_agent, middle, buf);
        self.draw_detail(right, buf);
        self.draw_status(status, buf);
        if self.policy_pane {
            self.draw_policy_pane(main, buf);
        }
        if self.help_pane {
            self.draw_help_pane(main, buf);
        }
        if let Some(request) = self.confirming.clone() {
            draw_confirmation(&request, self.denying, main, buf);
        }
        if let Some(step) = &self.confirming_step {
            draw_step_confirmation(step, main, buf);
        }
        if let Some((_, task, run)) = &self.confirming_review {
            draw_review_confirmation(task, run, main, buf);
        }
        if let Some(card) = &self.confirming_stop {
            draw_stop_confirmation(short(&card.run_id), self.chat_inside, main, buf);
        }
        if self.on_run_page() {
            self.draw_run_page(main, buf);
        }
    }

    /// The policy pane: the five files, the set's sha now, and each live
    /// run's sha beside it. A run on another sha kept the gate it launched
    /// with, and the mark says so. When the runs outnumber the rows, a line
    /// counts the ones cut and PageUp, PageDown or the wheel moves the list.
    fn draw_policy_pane(&mut self, area: Rect, buf: &mut Buffer) {
        let width = area.width.saturating_sub(8).min(110);
        let height = area.height.saturating_sub(2);
        let rect = Rect {
            x: area.x + (area.width - width) / 2,
            y: area.y + (area.height - height) / 2,
            width,
            height,
        };
        Clear.render(rect, buf);
        self.hits.policy_pane = rect;
        let bold = Style::default().add_modifier(Modifier::BOLD);
        let red = Style::default().fg(Color::Red);
        let mut lines = Vec::new();
        let current = match &self.policy_set {
            Some(Ok(sha)) => {
                lines.push(Line::styled(format!("policy set {sha}"), bold));
                Some(sha.as_str())
            }
            Some(Err(err)) => {
                lines.push(Line::styled(
                    "the policy set does not load, and a launch now would be refused",
                    red.add_modifier(Modifier::BOLD),
                ));
                lines.extend(err.lines().map(|l| Line::styled(l.to_string(), red)));
                None
            }
            None => {
                lines.push(Line::styled("no policy directory", red));
                None
            }
        };
        lines.push(Line::raw(""));
        let dir = self
            .policies_dir
            .as_ref()
            .map_or_else(String::new, |d| d.display().to_string());
        lines.push(Line::styled(format!("policies in {dir}"), bold));
        for (i, name) in crate::policy::NAMES.iter().enumerate() {
            let style = if i == self.policy_selected {
                Style::default().add_modifier(Modifier::REVERSED)
            } else {
                Style::default()
            };
            lines.push(Line::styled(format!("  {name}.json"), style));
        }
        lines.push(Line::raw(""));
        lines.push(Line::styled(
            "live runs and the set each launched under",
            bold,
        ));
        let mut footer = vec![
            Line::raw(""),
            Line::styled(
                "Up/Down select, e opens $EDITOR, PageUp/PageDown scroll runs, p or Esc closes. \
                 The cockpit never writes it.",
                Style::default().add_modifier(Modifier::DIM),
            ),
        ];
        let inner = rect.width.saturating_sub(2).max(1) as usize;
        let rows_of =
            |lines: &[Line]| -> usize { lines.iter().map(|line| wrapped_rows(line, inner)).sum() };
        let available = rect.height.saturating_sub(2) as usize;
        // The field rather than `cards_in`, which would borrow all of `self`.
        let live: Vec<&Card> = self
            .snapshot
            .cards
            .iter()
            .filter(|card| card.lane() == Lane::Live)
            .collect();
        if live.is_empty() {
            lines.push(Line::raw("  no live run"));
        }
        let runs: Vec<Line> = live
            .iter()
            .map(|card| {
                let sha = card.policy_sha.as_deref();
                let (mark, style) = match (sha, current) {
                    (None, _) => ("no sha recorded", Style::default()),
                    (Some(_), None) => ("cannot compare", Style::default()),
                    (Some(sha), Some(now)) if sha == now => ("current", Style::default()),
                    (Some(_), Some(_)) => (
                        "differs: it keeps the gate it launched with",
                        Style::default().fg(Color::Yellow),
                    ),
                };
                Line::styled(
                    format!(
                        "  {} {:<10} {:<12} {mark}",
                        short(&card.run_id),
                        card.policy.as_deref().unwrap_or("-"),
                        sha.map_or("-", short_sha),
                    ),
                    style,
                )
            })
            .collect();
        // A run's line wraps in a narrow pane, so the page is counted in
        // rows, and a cut list keeps one row for the line that says so.
        let heights: Vec<usize> = runs.iter().map(|line| wrapped_rows(line, inner)).collect();
        // Make room for a full run and the cut-list indicator. Drop help first,
        // then the file list and directory, then excess headings.
        let needed = heights.iter().copied().max().unwrap_or(0) + usize::from(runs.len() > 1);
        if rows_of(&lines) + rows_of(&footer) + needed > available {
            footer.clear();
        }
        if rows_of(&lines) + needed > available {
            lines.truncate(lines.len().saturating_sub(crate::policy::NAMES.len() + 4));
            lines.push(Line::styled("live runs", bold));
        }
        while rows_of(&lines) + needed > available && !lines.is_empty() {
            lines.pop();
        }
        let room = available.saturating_sub(rows_of(&lines) + rows_of(&footer));
        let budget = if heights.iter().sum::<usize>() <= room {
            room
        } else {
            room.saturating_sub(1)
        };
        let fits = |from: usize| {
            heights[from..]
                .iter()
                .scan(0, |used, rows| {
                    *used += rows;
                    Some(*used)
                })
                .take_while(|used| *used <= budget)
                .count()
        };
        let last = (0..runs.len())
            .find(|&from| from + fits(from) == runs.len())
            .unwrap_or(runs.len().saturating_sub(1));
        self.policy_offset = self.policy_offset.min(last);
        let shown = fits(self.policy_offset);
        let clipped = shown == 0 && room > 0 && !runs.is_empty();
        let shown = if clipped { 1 } else { shown };
        self.hits.policy_rows = shown;
        self.hits.policy_last = last;
        lines.extend(
            runs[self.policy_offset..self.policy_offset + shown]
                .iter()
                .cloned(),
        );
        if shown < live.len() && !clipped {
            lines.push(Line::styled(
                format!(
                    "  {} more, {} above and {} below",
                    live.len() - shown,
                    self.policy_offset,
                    live.len() - shown - self.policy_offset,
                ),
                Style::default().add_modifier(Modifier::DIM),
            ));
        }
        lines.extend(footer);
        Paragraph::new(lines)
            .block(Block::bordered().title(" policies "))
            .wrap(Wrap { trim: false })
            .render(rect, buf);
    }

    /// The help overlay: every key from [`super::keys::HELP`], grouped like the
    /// README table.
    fn draw_help_pane(&self, area: Rect, buf: &mut Buffer) {
        let width = area.width.saturating_sub(8).min(100);
        let height = area.height.saturating_sub(2);
        let rect = Rect {
            x: area.x + (area.width - width) / 2,
            y: area.y + (area.height - height) / 2,
            width,
            height,
        };
        Clear.render(rect, buf);
        let bold = Style::default().add_modifier(Modifier::BOLD);
        let dim = Style::default().add_modifier(Modifier::DIM);
        let mut lines = vec![Line::styled("cockpit keys", bold), Line::raw("")];
        let session = self.chat_inside;
        let mut group = None;
        for (keys, description) in super::keys::help_rows(session) {
            let row_group = super::keys::HELP
                .iter()
                .find(|e| e.keys == keys)
                .map(|e| e.group);
            if row_group != group {
                group = row_group;
                if let Some(g) = group {
                    let title = match g {
                        super::keys::HelpGroup::Moving => "moving",
                        super::keys::HelpGroup::Opening => "opening",
                        super::keys::HelpGroup::Confirm => "confirm",
                        super::keys::HelpGroup::Views => "views",
                        super::keys::HelpGroup::Quit => "quit",
                        super::keys::HelpGroup::Help => "help",
                        super::keys::HelpGroup::PolicyPane => "policy pane",
                        super::keys::HelpGroup::Session => "with --session",
                    };
                    lines.push(Line::styled(title, bold));
                }
            }
            lines.push(Line::from(vec![
                Span::styled(format!("  {keys:<22}"), bold),
                Span::raw(description),
            ]));
        }
        lines.push(Line::raw(""));
        lines.push(Line::styled("Esc or ? closes", dim));
        Paragraph::new(lines)
            .block(Block::bordered().title(" help "))
            .wrap(Wrap { trim: false })
            .render(rect, buf);
    }

    /// The dragon left of the title, at the top, when the screen has room for it and the table
    /// still has its rows. Returns what is left below it.
    ///
    /// Each provider with a live run gets its tokens per minute and window share
    /// there, build ticket 172. Without the dragon they take one row of their
    /// own, and only while some run is live.
    fn draw_header(&self, area: Rect, buf: &mut Buffer) -> Rect {
        let spend: Vec<String> = self.snapshot.spend.iter().map(spend_text).collect();
        let Some(mascot) = self.mascot.as_ref().filter(|m| {
            let (cols, rows) = m.size();
            area.width >= cols + TITLE_WIDTH && area.height >= rows + MIN_BODY
        }) else {
            if spend.is_empty() || area.height <= MIN_BODY {
                return area;
            }
            let [row, rest] =
                Split::vertical([Constraint::Length(1), Constraint::Min(0)]).areas(area);
            buf.set_line(row.x, row.y, &Line::raw(spend.join(" | ")), row.width);
            return rest;
        };
        let (cols, rows) = mascot.size();
        let [header, rest] =
            Split::vertical([Constraint::Length(rows), Constraint::Min(0)]).areas(area);
        mascot.draw(header, buf);
        let text = Rect {
            x: header.x + cols + 1,
            width: header.width - cols - 1,
            ..header
        };
        let dim = Style::default().add_modifier(Modifier::DIM);
        let mut lines = vec![Line::styled(
            "hydra cockpit",
            Style::default().add_modifier(Modifier::BOLD),
        )];
        lines.push(Line::styled(
            match mascot.extra() {
                0 => String::new(),
                more => format!("+{more} more live past the three heads"),
            },
            dim,
        ));
        lines.push(Line::styled(
            format!("{} live seat(s), {}", mascot.live(), mascot.pose().name()),
            dim,
        ));
        if let Some((run_id, outcome)) = mascot.ended() {
            lines.push(Line::raw(format!("{} {outcome}", short(run_id))));
        }
        lines.extend(spend.into_iter().map(Line::raw));
        Paragraph::new(lines).render(text, buf);
        rest
    }

    fn block(&self, focus: Focus, title: String) -> Block<'static> {
        let style = if self.has_keys && self.focus == focus {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        };
        Block::bordered().border_style(style).title(title)
    }

    fn run_label<'a>(&'a self, id: &'a str) -> &'a str {
        crate::resolver::unique_prefix(
            id,
            self.snapshot
                .cards
                .iter()
                .filter(|card| {
                    !self.by_agent
                        || self.session.as_deref().is_none_or(|session| {
                            card.run_id == session || card.parent_run_id.as_deref() == Some(session)
                        })
                })
                .map(|card| card.run_id.as_str())
                .chain(
                    self.snapshot
                        .goal
                        .iter()
                        .flat_map(|goal| goal.rows.iter().filter_map(|row| row.run_id.as_deref())),
                ),
        )
    }

    fn draw_table(&mut self, area: Rect, buf: &mut Buffer) {
        self.hits.table = area;
        let title = match &self.snapshot.goal {
            Some(goal) => format!(" tasks of {} ", goal.id),
            None => " tasks ".to_string(),
        };
        let block = self.block(Focus::Table, title);
        let inner = block.inner(area);
        block.render(area, buf);
        let Some(goal) = self.snapshot.goal.clone() else {
            let note = if self.snapshot.read {
                self.snapshot
                    .goal_note
                    .clone()
                    .unwrap_or_else(|| "no goal".to_string())
            } else {
                "reading".to_string()
            };
            Paragraph::new(note)
                .wrap(Wrap { trim: false })
                .render(inner, buf);
            return;
        };
        if inner.height < 2 {
            return;
        }
        if let Some(line) = goal.damaged_at {
            let heading = format!(
                "damaged at line {line}: run hydra-puppets repair {}",
                goal.id
            );
            buf.set_line(
                inner.x,
                inner.y,
                &Line::styled(heading, Style::default().fg(Color::Yellow)),
                inner.width,
            );
            return;
        }
        let dim = Style::default().add_modifier(Modifier::DIM);
        let mut heading = goal.text.clone().unwrap_or_default();
        if goal.skipped > 0 {
            heading = format!("{heading} ({} line(s) skipped)", goal.skipped);
        }
        buf.set_line(inner.x, inner.y, &Line::styled(heading, dim), inner.width);
        let run_width = self
            .snapshot
            .goal
            .iter()
            .flat_map(|goal| &goal.rows)
            .filter_map(|row| row.run_id.as_deref())
            .map(|id| self.run_label(id).len())
            .max()
            .unwrap_or(8);
        let columns = columns_with_run(inner.width, run_width);
        let header = row_text(&columns, Column::name);
        buf.set_line(
            inner.x,
            inner.y + 1,
            &Line::styled(header, Style::default().add_modifier(Modifier::BOLD)),
            inner.width,
        );
        let rows_area = Rect {
            y: inner.y + 2,
            height: inner.height - 2,
            ..inner
        };
        let visible = rows_area.height as usize;
        self.table_offset = self
            .table_offset
            .min(goal.rows.len().saturating_sub(visible));
        for (slot, index) in (self.table_offset..goal.rows.len())
            .take(visible)
            .enumerate()
        {
            let row = &goal.rows[index];
            let run = row
                .run_id
                .as_deref()
                .map(|id| self.run_label(id))
                .unwrap_or("");
            let status = self.status_of(&goal, row);
            let text = row_text(&columns, |column| match column {
                Column::Id => &row.id,
                Column::Status => status,
                Column::Provider => row.provider.as_deref().unwrap_or("-"),
                Column::Model => row.model.as_deref().unwrap_or("-"),
                Column::Run => run,
                Column::Grade => &row.grade,
                Column::Class => &row.class,
                Column::Done => row.done.as_deref().unwrap_or("-"),
            });
            let rect = Rect {
                y: rows_area.y + slot as u16,
                height: 1,
                ..rows_area
            };
            let style = self.selected_style(Focus::Table, index == self.table_selected);
            buf.set_line(rect.x, rect.y, &Line::styled(text, style), rect.width);
            self.hits.table_rows.push((rect, index));
        }
    }

    /// `fold`'s status, except that an `ended` row whose run the snapshot
    /// has normalized reads that run's outcome. A verdict is never `ended`,
    /// so the human's word still wins.
    ///
    /// A review this cockpit runs, or saw fail, shows over any status. A
    /// verdict from the log shows over an `ended` one, build ticket 183.
    fn status_of<'a>(&'a self, goal: &Goal, row: &'a TaskRow) -> &'a str {
        let ours = self
            .reviews
            .iter()
            .any(|r| r.goal == goal.id && r.task == row.id);
        if let Some((cell, _)) = self.review_of(goal, &row.id)
            && (ours || row.status == "ended")
        {
            return cell;
        }
        if row.status != "ended" {
            return &row.status;
        }
        row.run_id
            .as_deref()
            .and_then(|id| self.snapshot.cards.iter().find(|c| c.run_id == id))
            .and_then(|card| card.outcome.as_deref())
            .unwrap_or(&row.status)
    }

    fn selected_style(&self, panel: Focus, selected: bool) -> Style {
        match (selected, self.focus == panel) {
            (true, true) => Style::default().add_modifier(Modifier::REVERSED),
            (true, false) => Style::default().add_modifier(Modifier::BOLD),
            _ => Style::default(),
        }
    }

    fn draw_lane(&mut self, lane: Lane, area: Rect, buf: &mut Buffer) {
        self.hits.lanes[lane.index()] = area;
        let lines: Vec<String> = match lane {
            Lane::Waiting => self
                .snapshot
                .requests
                .iter()
                .map(|request| {
                    format!(
                        "{} session {} {} seat(s)",
                        short(&request.request_id),
                        short(&request.session),
                        request.seats.len()
                    )
                })
                .chain(self.snapshot.steps.iter().map(Step::label))
                .collect(),
            _ => self
                .cards_in(lane)
                .map(|card| {
                    format!(
                        "{} {} {}",
                        self.run_label(&card.run_id),
                        agent_of(card),
                        self.said(card)
                    )
                })
                .collect(),
        };
        let title = format!(" {} {} ", lane.title(), self.lane_len(lane));
        let listed = lines.len();
        self.draw_cards(Focus::Lane(lane), title, lines, area, buf);
        // The line that says why no step is listed goes under the requests, where
        // the steps would be. It is not a card: nothing selects or opens it.
        if lane == Lane::Waiting
            && let Some(note) = &self.snapshot.steps_note
        {
            let row = u16::try_from(listed)
                .ok()
                .and_then(|listed| area.y.checked_add(1)?.checked_add(listed));
            if let Some(row) = row
                && row < area.bottom().saturating_sub(1)
            {
                buf.set_line(
                    area.x + 1,
                    row,
                    &Line::raw(note.clone()),
                    area.width.saturating_sub(2),
                );
            }
        }
    }

    /// The lanes by agent, build ticket 191: one per agent in [`App::agents`],
    /// then the waiting lane, so a request or a step is reachable from here too.
    fn draw_agent_lanes(&mut self, area: Rect, buf: &mut Buffer) {
        let agents = self.agents();
        let count = agents.len() + 1;
        let rects = Split::vertical(vec![Constraint::Ratio(1, count as u32); count]).split(area);
        self.hits.agent_lanes = rects[..agents.len()].to_vec();
        for (i, agent) in agents.iter().enumerate() {
            let lines: Vec<String> = self
                .cards_of(agent)
                .map(|card| self.agent_line(card))
                .collect();
            let title = format!(" {agent} {} ", lines.len());
            self.draw_cards(Focus::Agent(i), title, lines, rects[i], buf);
        }
        self.draw_lane(Lane::Waiting, rects[agents.len()], buf);
    }

    /// A card in an agent's lane: short id, model, what [`App::said`] says,
    /// and the task when the goal table names the run.
    fn agent_line(&self, card: &Card) -> String {
        let model = card.model.as_deref().unwrap_or("-");
        let mut line = format!(
            "{} {model} {}",
            self.run_label(&card.run_id),
            self.said(card)
        );
        let task = self.snapshot.goal.as_ref().and_then(|goal| {
            goal.rows
                .iter()
                .find(|row| row.run_id.as_ref() == Some(&card.run_id))
        });
        if let Some(task) = task {
            line.push(' ');
            line.push_str(&task.id);
        }
        line
    }

    /// A run's state or outcome on its card: a live one's stop label while
    /// it has one, then the outcome, then the state.
    fn said(&self, card: &Card) -> String {
        match self.live_label(card) {
            Some(label) if card.lane() == Lane::Live => label.to_string(),
            _ => card
                .outcome
                .clone()
                .unwrap_or_else(|| card.state.as_str().to_string()),
        }
    }

    /// One lane of the middle column, `lines` one card a row from its offset.
    fn draw_cards(
        &mut self,
        focus: Focus,
        title: String,
        lines: Vec<String>,
        area: Rect,
        buf: &mut Buffer,
    ) {
        let block = self.block(focus, title);
        let inner = block.inner(area);
        block.render(area, buf);
        let visible = inner.height as usize;
        let offset = self.offset(focus).min(lines.len().saturating_sub(visible));
        self.set_offset(focus, offset);
        let selected = self.selected(focus);
        for (slot, index) in (offset..lines.len()).take(visible).enumerate() {
            let rect = Rect {
                y: inner.y + slot as u16,
                height: 1,
                ..inner
            };
            let style = self.selected_style(focus, index == selected);
            buf.set_line(
                rect.x,
                rect.y,
                &Line::styled(lines[index].clone(), style),
                rect.width,
            );
            self.hits.cards.push((rect, focus, index));
        }
    }

    fn draw_detail(&mut self, area: Rect, buf: &mut Buffer) {
        self.hits.detail = area;
        let block = self.block(Focus::Detail, " detail ".to_string());
        let lines = self.detail_lines(area.width.saturating_sub(2) as usize);
        Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false })
            .scroll((self.detail_scroll, 0))
            .render(area, buf);
    }

    /// A run opened with `Enter`, over the cockpit, build ticket 245.
    fn draw_run_page(&mut self, area: Rect, buf: &mut Buffer) {
        self.hits.run_page = area;
        self.run_page_area = area;
        Clear.render(area, buf);
        let Some(Opened::Run(id)) = &self.opened else {
            return;
        };
        let Some(card) = self.snapshot.cards.iter().find(|c| &c.run_id == id) else {
            return;
        };
        let dash = |value: Option<&str>| value.unwrap_or("-").to_string();
        let title = format!(
            "{}  {}  {}  {}  {}",
            short(&card.run_id),
            dash(card.provider.as_deref()),
            dash(card.model.as_deref()),
            dash(card.policy.as_deref()),
            card.state.as_str(),
        );
        let inner_w = area.width.saturating_sub(2).max(1) as usize;
        let body = self.detail_scroll_lines(inner_w, true);
        let block = Block::bordered()
            .title(title)
            .title_style(Style::default().add_modifier(Modifier::BOLD));
        Paragraph::new(body)
            .block(block)
            .wrap(Wrap { trim: false })
            .scroll((self.detail_scroll, 0))
            .render(area, buf);
    }

    /// `width` is the pane's inside, which a tool call's line is cut to.
    fn detail_lines(&self, width: usize) -> Vec<Line<'static>> {
        let label = |name: &str, value: String| {
            Line::from(vec![
                Span::styled(
                    format!("{name} "),
                    Style::default().add_modifier(Modifier::DIM),
                ),
                Span::raw(value),
            ])
        };
        match &self.opened {
            None => vec![Line::raw(
                "Enter opens the selected run or request. A click on a card opens it.",
            )],
            Some(Opened::Request(id)) => {
                match self.snapshot.requests.iter().find(|r| &r.request_id == id) {
                    Some(request) => {
                        let mut lines: Vec<Line> =
                            crate::approve::listing(std::slice::from_ref(request))
                                .lines()
                                .map(|line| Line::raw(line.to_string()))
                                .collect();
                        lines.push(Line::raw("a asks to approve it here"));
                        lines
                    }
                    None => vec![Line::raw(format!("request {id} is no longer waiting"))],
                }
            }
            Some(Opened::Step(id)) => {
                match self.snapshot.steps.iter().find(|step| &step.label() == id) {
                    Some(step) => vec![
                        label("goal", step.goal.clone()),
                        label("waits on", step.kind.clone()),
                        label("ref", step.reference.clone().unwrap_or_else(|| "-".into())),
                        label("revision", step.revision.to_string()),
                        label("runs", shown_argv(&step.argv)),
                        Line::raw("a asks to approve it here"),
                    ],
                    None => vec![Line::raw(format!("{id} is no longer waiting"))],
                }
            }
            Some(Opened::Run(id)) => {
                let Some(card) = self.snapshot.cards.iter().find(|c| &c.run_id == id) else {
                    return vec![Line::raw(format!("no record for run {id}"))];
                };
                let or_dash = |value: Option<&str>| value.unwrap_or("-").to_string();
                let mut lines = vec![label("run", card.run_id.clone())];
                let task = self
                    .snapshot
                    .goal
                    .as_ref()
                    .and_then(|goal| goal.rows.iter().find(|r| r.run_id.as_ref() == Some(id)));
                if let Some(task) = task {
                    lines.push(label("task", task.id.clone()));
                    lines.push(label("done", or_dash(task.done.as_deref())));
                    let goal = self.snapshot.goal.as_ref();
                    if let Some((_, said)) = goal.and_then(|g| self.review_of(g, &task.id)) {
                        lines.push(label("review", said));
                    }
                    if let Some(open) = goal.and_then(|g| self.review_open(&g.id, &task.id)) {
                        lines.push(label("hunk", open.to_string()));
                    }
                }
                lines.extend([
                    label("provider", or_dash(card.provider.as_deref())),
                    label("model", or_dash(card.model.as_deref())),
                    label("policy", or_dash(card.policy.as_deref())),
                    label("state", card.state.as_str().to_string()),
                    label("outcome", or_dash(card.outcome.as_deref())),
                    label(
                        "tokens",
                        card.total_tokens
                            .map_or_else(|| "-".to_string(), |t| t.to_string()),
                    ),
                ]);
                if let Some(note) = &card.note {
                    lines.push(label("note", note.clone()));
                }
                lines.push(Line::raw(""));
                lines.push(Line::styled(
                    "result_text",
                    Style::default().add_modifier(Modifier::BOLD),
                ));
                match &card.result_text {
                    Some(text) => lines.extend(text.lines().map(|l| Line::raw(l.to_string()))),
                    None => lines.push(Line::raw("-")),
                }
                lines.push(Line::raw(""));
                if self.reveal_until.is_some() {
                    if let Some(call) = &self.revealed_call {
                        lines.extend(wrap_call_reveal(call, width));
                    }
                    if let Some(line) = &self.revealed_line {
                        lines.extend(wrap_plain(line, width.max(1)).into_iter().map(Line::raw));
                    }
                    return lines;
                }
                if self.raw_tail {
                    lines.push(Line::styled(
                        "last record lines (r shows tool calls, i reveals)",
                        Style::default().add_modifier(Modifier::BOLD),
                    ));
                    match &self.snapshot.tail {
                        Some((tail_id, tail)) if tail_id == id => {
                            for (index, line) in tail.iter().enumerate() {
                                let text = call_head::tail_line_shown(line, 400);
                                let style = self.detail_call_style(index);
                                lines.push(Line::styled(text, style));
                            }
                        }
                        _ => lines.push(Line::raw("reading")),
                    }
                    return lines;
                }
                lines.push(Line::styled(
                    "tool calls (r shows the raw record lines, i reveals)",
                    Style::default().add_modifier(Modifier::BOLD),
                ));
                match &self.snapshot.calls {
                    Some((calls_id, calls)) if calls_id == id => {
                        if calls.is_empty() {
                            lines.push(Line::raw("none yet"));
                        }
                        for (index, call) in calls.iter().enumerate() {
                            lines.extend(self.call_rows(call, width, index));
                        }
                    }
                    _ => lines.push(Line::raw("reading")),
                }
                lines
            }
        }
    }

    fn detail_call_style(&self, index: usize) -> Style {
        if self.focus == Focus::Detail && index == self.detail_call {
            Style::default().add_modifier(Modifier::REVERSED)
        } else {
            Style::default()
        }
    }

    fn call_rows(&self, call: &CallLine, width: usize, index: usize) -> Vec<Line<'static>> {
        vec![call_line(
            call,
            &call.input,
            width,
            self.detail_call_style(index),
        )]
    }

    fn draw_status(&self, area: Rect, buf: &mut Buffer) {
        if area.is_empty() {
            return;
        }
        let goal = match &self.snapshot.goal {
            Some(goal) if self.snapshot.goal_named => format!("goal {}", goal.id),
            Some(goal) => format!("goal {} (the log that changed last)", goal.id),
            None => "no goal".to_string(),
        };
        let counts = Lane::ALL
            .iter()
            .map(|lane| format!("{} {}", lane.title(), self.lane_len(*lane)))
            .collect::<Vec<_>>()
            .join(", ");
        let lanes = if self.by_agent {
            "k lanes by state"
        } else {
            "k lanes by agent"
        };
        let keys = format!(
            "arrows/Tab move, Enter opens, {lanes}, Esc closes, r raw lines, i reveals input, \
             a approves, d denies, v reviews a task, m stops a seat, p policies, ? help, q quits"
        );
        let mut text = format!(
            "{goal} | every {}s | {counts} | {keys}",
            self.interval.as_secs_f32()
        );
        if let Some(message) = &self.message {
            text = format!("{message} | {text}");
        } else if let Some(error) = self.snapshot.errors.first() {
            text = format!("{error} | {text}");
        }
        buf.set_line(area.x, area.y, &Line::raw(text), area.width);
    }
}

/// Which view the middle column shows, on its top border at the right, over
/// the first lane's.
fn draw_mode(by_agent: bool, area: Rect, buf: &mut Buffer) {
    let mode = if by_agent { " by agent " } else { " by state " };
    let width = mode.width() as u16;
    if area.width > width + 2 && area.height > 0 {
        buf.set_string(
            area.right() - width - 1,
            area.y,
            mode,
            Style::default().add_modifier(Modifier::DIM),
        );
    }
}

/// One tool call on one row: name, what the result said, then the input, which
/// is what gets cut when the row is wider than the pane.
fn call_line(call: &CallLine, input: &str, width: usize, row_style: Style) -> Line<'static> {
    let decision = call.decision.as_deref().unwrap_or("-");
    let decision_style = if decision.starts_with("refused") {
        Style::default().fg(Color::Red)
    } else {
        Style::default().add_modifier(Modifier::DIM)
    };
    let head = format!("{} {decision} ", call.name);
    let room = width.saturating_sub(head.width());
    let shown = cut_to_columns(input, room);
    Line::from(vec![
        Span::styled(format!("{} ", call.name), row_style),
        Span::styled(format!("{decision} "), decision_style),
        Span::styled(shown, row_style),
    ])
}

fn wrap_call_reveal(call: &CallLine, width: usize) -> Vec<Line<'static>> {
    let decision = call.decision.as_deref().unwrap_or("-");
    let decision_style = if decision.starts_with("refused") {
        Style::default().fg(Color::Red)
    } else {
        Style::default().add_modifier(Modifier::DIM)
    };
    let prefix = format!("{} {decision} ", call.name);
    let room = width.saturating_sub(prefix.width()).max(1);
    let mut lines = Vec::new();
    let wrapped = wrap_plain(&call.full_input, room);
    for (i, chunk) in wrapped.into_iter().enumerate() {
        if i == 0 {
            lines.push(Line::from(vec![
                Span::raw(format!("{} ", call.name)),
                Span::styled(format!("{decision} "), decision_style),
                Span::raw(chunk),
            ]));
        } else {
            lines.push(Line::raw(chunk));
        }
    }
    if lines.is_empty() {
        lines.push(Line::from(vec![
            Span::raw(format!("{} ", call.name)),
            Span::styled(format!("{decision} "), decision_style),
        ]));
    }
    lines
}

fn wrap_plain(text: &str, width: usize) -> Vec<String> {
    let width = width.max(1);
    let mut lines = Vec::new();
    let mut current = String::new();
    let mut used = 0usize;
    for word in text.split_whitespace() {
        let w = word.width();
        let gap = usize::from(!current.is_empty());
        if used + gap + w <= width {
            if !current.is_empty() {
                current.push(' ');
            }
            current.push_str(word);
            used += gap + w;
        } else if w <= width {
            if !current.is_empty() {
                lines.push(current.clone());
            }
            current = word.to_string();
            used = w;
        } else {
            if !current.is_empty() {
                lines.push(current.clone());
                current.clear();
                used = 0;
            }
            let mut chunk = String::new();
            let mut chunk_used = 0;
            for c in word.chars() {
                let cw = c.width().unwrap_or(0);
                if chunk_used + cw > width && !chunk.is_empty() {
                    lines.push(chunk.clone());
                    chunk.clear();
                    chunk_used = 0;
                }
                chunk.push(c);
                chunk_used += cw;
            }
            if !chunk.is_empty() {
                current = chunk;
                used = chunk_used;
            }
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

/// `text` in at most `room` terminal columns. Text that does not fit ends in
/// `~`, counted inside `room`, and a wide character that would straddle the
/// edge is left out rather than split.
fn cut_to_columns(text: &str, room: usize) -> String {
    if text.width() <= room {
        return text.to_string();
    }
    let Some(room) = room.checked_sub(1) else {
        return String::new();
    };
    let mut cut = String::new();
    let mut used = 0;
    for c in text.chars() {
        let w = c.width().unwrap_or(0);
        if used + w > room {
            break;
        }
        used += w;
        cut.push(c);
    }
    cut.push('~');
    cut
}

/// What the cockpit says where `hydra approve` would refuse by name.
fn inside_seatbelt() -> String {
    format!(
        "{}: the cockpit is inside a seatbelt, which is where a model runs, and no model \
         approves a seat request (map ticket 82). Nothing was written.",
        crate::approve::INSIDE_SEATBELT
    )
}

/// The same for a Master of Puppets step, build ticket 182.
fn step_inside_seatbelt() -> String {
    format!(
        "{}: the cockpit is inside a seatbelt, which is where a model runs, and no model \
         approves a Master of Puppets step. Nothing was run.",
        crate::approve::INSIDE_SEATBELT
    )
}

/// An argv on one line, each argument with a space or a quote in it quoted.
fn shown_argv(argv: &[String]) -> String {
    argv.iter()
        .map(|arg| {
            if arg.is_empty() || arg.contains(|c: char| c.is_whitespace() || "'\"\\$".contains(c)) {
                format!("'{}'", arg.replace('\'', "'\\''"))
            } else {
                arg.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// The question `a` asks before it runs a step's approval.
fn draw_step_confirmation(step: &Step, area: Rect, buf: &mut Buffer) {
    let width = area.width.saturating_sub(8).min(110);
    let height = area.height.saturating_sub(4).min(10);
    let rect = Rect {
        x: area.x + (area.width - width) / 2,
        y: area.y + (area.height - height) / 2,
        width,
        height,
    };
    Clear.render(rect, buf);
    let lines = vec![
        Line::raw(format!(
            "{} at revision {}. y runs, as a child:",
            step.label(),
            step.revision
        )),
        Line::raw(shown_argv(&step.argv)),
        Line::styled(
            "y approves, n closes",
            Style::default().add_modifier(Modifier::BOLD),
        ),
    ];
    Paragraph::new(lines)
        .block(
            Block::bordered()
                .border_style(Style::default().fg(Color::Red))
                .title(" approve this step? "),
        )
        .wrap(Wrap { trim: false })
        .render(rect, buf);
}

fn draw_confirmation(request: &Pending, deny: bool, area: Rect, buf: &mut Buffer) {
    let width = area.width.saturating_sub(8).min(90);
    let height = area.height.saturating_sub(4).min(20);
    let rect = Rect {
        x: area.x + (area.width - width) / 2,
        y: area.y + (area.height - height) / 2,
        width,
        height,
    };
    Clear.render(rect, buf);
    let mut lines: Vec<Line> = crate::approve::listing(std::slice::from_ref(request))
        .lines()
        .map(|line| Line::raw(line.to_string()))
        .collect();
    let (keys, title) = if deny {
        ("y denies, n closes", " deny this request? ")
    } else {
        ("y approves, n closes", " approve this request? ")
    };
    lines.push(Line::styled(
        keys,
        Style::default().add_modifier(Modifier::BOLD),
    ));
    Paragraph::new(lines)
        .block(
            Block::bordered()
                .border_style(Style::default().fg(Color::Red))
                .title(title),
        )
        .wrap(Wrap { trim: false })
        .render(rect, buf);
}

/// The question `v` asks before three judge runs spend quota.
fn draw_review_confirmation(task: &str, run: &str, area: Rect, buf: &mut Buffer) {
    let width = area.width.saturating_sub(8).min(90);
    let height = 7.min(area.height);
    let rect = Rect {
        x: area.x + (area.width - width) / 2,
        y: area.y + (area.height - height) / 2,
        width,
        height,
    };
    Clear.render(rect, buf);
    let lines = vec![
        Line::raw(format!("run {run}")),
        Line::raw(format!(
            "review {task}: three judge runs, each counts against quota. y starts it"
        )),
        Line::styled("n closes", Style::default().add_modifier(Modifier::BOLD)),
    ];
    Paragraph::new(lines)
        .block(
            Block::bordered()
                .border_style(Style::default().fg(Color::Red))
                .title(" start a review? "),
        )
        .wrap(Wrap { trim: false })
        .render(rect, buf);
}

/// The question `m` asks before it stops a seat.
fn draw_stop_confirmation(id: &str, drafts: bool, area: Rect, buf: &mut Buffer) {
    let width = area.width.saturating_sub(8).min(90);
    let height = 4.min(area.height);
    let rect = Rect {
        x: area.x + (area.width - width) / 2,
        y: area.y + (area.height - height) / 2,
        width,
        height,
    };
    Clear.render(rect, buf);
    let ask = if drafts {
        format!("stop {id} and draft a note to the session? y")
    } else {
        format!("stop {id}? y")
    };
    let lines = vec![
        Line::raw(ask),
        Line::styled("n closes", Style::default().add_modifier(Modifier::BOLD)),
    ];
    Paragraph::new(lines)
        .block(
            Block::bordered()
                .border_style(Style::default().fg(Color::Red))
                .title(" stop a seat? "),
        )
        .wrap(Wrap { trim: false })
        .render(rect, buf);
}

/// How much of a seat's task the note quotes, in characters.
const DRAFT_TASK: usize = 200;

/// The note `m` pastes into the chat once `card` is stopped: which seat, what
/// it was told, and `Instead: ` for the human to finish.
///
/// **It never ends in a newline and carries no control character.** Each
/// field goes through [`one_line`], so neither a `\r` that would send it nor
/// an `ESC[201~` that would close the bracketed paste early can come from the
/// task or the model name. `task` is `run_start.task`, which begins with the
/// preamble; the note quotes what follows it.
pub fn draft(card: &Card, task: Option<&str>, preamble: Option<&str>) -> String {
    let who = [card.provider.as_deref(), card.model.as_deref()]
        .into_iter()
        .flatten()
        .map(one_line)
        .collect::<Vec<_>>()
        .join(" ");
    let who = if who.is_empty() {
        "provider unknown".to_string()
    } else {
        who
    };
    let task = match task {
        Some(task) => {
            let own = preamble
                .and_then(|preamble| task.strip_prefix(preamble))
                .and_then(|rest| rest.strip_prefix("\n\n"))
                .unwrap_or(task);
            one_line(own).chars().take(DRAFT_TASK).collect()
        }
        None => "not on its record".to_string(),
    };
    format!(
        "Seat {} ({who}) was stopped by the human. Its task was: {task}. Instead: ",
        one_line(short(&card.run_id))
    )
}

/// The last `hunk show ...` line in the review log at `log`, on one line,
/// or `None` when it holds none or will not read.
fn hunk_line(log: &Path) -> Option<String> {
    let text = std::fs::read_to_string(log).ok()?;
    text.lines()
        .rev()
        .find(|line| line.starts_with("hunk show "))
        .map(one_line)
}

/// `text` on one line: every run of whitespace and control characters, `\r`,
/// `\n` and `ESC` among them, becomes one space, and the ends are trimmed.
fn one_line(text: &str) -> String {
    text.split(|c: char| c.is_whitespace() || c.is_control())
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Keeps `selected` inside a window of `rows` starting at `offset`.
fn follow(selected: usize, offset: usize, rows: usize) -> usize {
    if rows == 0 || selected < offset {
        selected
    } else if selected >= offset + rows {
        selected + 1 - rows
    } else {
        offset
    }
}

/// The rows `line` takes wrapped at `width`, breaking between words as the
/// pane's `Wrap` does and cutting a word wider than a row.
pub fn wrapped_rows(line: &Line, width: usize) -> usize {
    let text: String = line
        .spans
        .iter()
        .map(|span| span.content.as_ref())
        .collect();
    let width = width.max(1);
    // The space before a word counts, except on a row the wrap just opened,
    // which drops it.
    let (mut rows, mut used, mut opened) = (1, 0, false);
    for (i, word) in text.split(' ').enumerate() {
        let len = Span::raw(word).width();
        let gap = usize::from(i > 0 && !opened);
        if used + gap + len <= width {
            used += gap + len;
            opened = opened && len == 0;
        } else if len <= width {
            rows += 1;
            used = len;
            opened = len == 0;
        } else {
            // A long word starts on its own row and runs over as many as it needs.
            rows += usize::from(used > 0) + (len - 1) / width;
            used = (len - 1) % width + 1;
            opened = false;
        }
    }
    rows
}

/// The first twelve characters of a sha, as the policy pane shows it.
fn short_sha(sha: &str) -> &str {
    sha.get(..12).unwrap_or(sha)
}

/// The first eight characters of an id, which is what `hydra-puppets board` shows.
fn short(id: &str) -> &str {
    id.get(..8).unwrap_or(id)
}

/// A column of the task table, in the order it is drawn.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Column {
    Id,
    Status,
    Provider,
    Model,
    Run,
    Grade,
    Class,
    Done,
}

impl Column {
    fn name(self) -> &'static str {
        match self {
            Column::Id => "id",
            Column::Status => "status",
            Column::Provider => "provider",
            Column::Model => "model",
            Column::Run => "run",
            Column::Grade => "grade",
            Column::Class => "class",
            Column::Done => "done",
        }
    }
}

/// The columns a table `width` wide shows, in drawing order, each with its
/// width. They are taken in priority order, each at the first of its widths
/// that fits, and the first that fits at none ends the table, so a column
/// never shows while one above it is gone. `done` is 12 at least and gets
/// what the others leave. The status column is as wide as the longest of
/// contract 8's outcomes, `skill_collision`.
fn columns_with_run(width: u16, run_width: usize) -> Vec<(Column, usize)> {
    const PRIORITY: [(Column, &[usize]); 8] = [
        (Column::Id, &[8]),
        (Column::Status, &[15]),
        (Column::Done, &[12]),
        (Column::Provider, &[12]),
        (Column::Model, &[12]),
        (Column::Run, &[8]),
        (Column::Grade, &[6]),
        (Column::Class, &[20, 12]),
    ];
    let width = width as usize;
    // Each column shown so far and the space after it.
    let used = |shown: &[(Column, usize)]| shown.iter().map(|(_, w)| w + 1).sum::<usize>();
    let mut shown = Vec::new();
    for (column, widths) in PRIORITY {
        let taken = used(&shown);
        let run_widths = [run_width.max(8)];
        let widths = if column == Column::Run {
            &run_widths[..]
        } else {
            widths
        };
        let Some(&w) = widths.iter().find(|&&w| taken + w <= width) else {
            break;
        };
        shown.push((column, w));
    }
    shown.sort_by_key(|&(column, _)| column);
    if let Some(last) = shown.len().checked_sub(1)
        && shown[last].0 == Column::Done
    {
        shown[last].1 = width - used(&shown[..last]);
    }
    shown
}

/// One table line: each of `columns` cut and padded to its width. The detail
/// of the task's run shows a cut `done` whole.
fn row_text<'a>(columns: &[(Column, usize)], cell: impl Fn(Column) -> &'a str) -> String {
    columns
        .iter()
        .map(|&(column, width)| {
            let text: String = cell(column).chars().take(width).collect();
            format!("{text:<width$}")
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stopped_seat(model: Option<&str>) -> Card {
        Card {
            run_id: "01996d3c-8f2a-7c31-9b44-000000000170".to_string(),
            provider: Some("codex".to_string()),
            model: model.map(str::to_string),
            policy: None,
            policy_sha: None,
            parent_run_id: Some("01996d3c-8f2a-7c31-9b44-000000000169".to_string()),
            state: crate::live::State::Live,
            outcome: None,
            total_tokens: None,
            result_text: None,
            note: None,
        }
    }

    #[test]
    fn colliding_run_labels_are_distinct_in_each_view() {
        let mut app = App::new(
            Layout::at(std::env::temp_dir().join("hydra-label-test")),
            false,
            Duration::from_secs(2),
        );
        let ids = [
            "01a0fe73-1b67-7000-8000-000000000001",
            "01a0fe73-5a9c-7000-8000-000000000002",
        ];
        app.snapshot.cards = ids
            .iter()
            .map(|id| {
                let mut card = stopped_seat(None);
                card.run_id = id.to_string();
                card
            })
            .collect();
        app.snapshot.goal = Some(Goal {
            rows: ids
                .iter()
                .enumerate()
                .map(|(i, id)| TaskRow {
                    id: format!("task{i}"),
                    class: String::new(),
                    grade: String::new(),
                    provider: None,
                    model: None,
                    status: "running".to_string(),
                    run_id: Some(id.to_string()),
                    done: None,
                })
                .collect(),
            ..Goal::default()
        });
        for mode in 0..3 {
            let area = Rect::new(0, 0, 160, 20);
            let mut buf = Buffer::empty(area);
            match mode {
                0 => app.draw_lane(Lane::Live, area, &mut buf),
                1 => app.draw_agent_lanes(area, &mut buf),
                _ => app.draw_table(area, &mut buf),
            }
            let text: String = buf.content.iter().map(|cell| cell.symbol()).collect();
            for label in ["01a0fe73-1", "01a0fe73-5"] {
                assert!(text.contains(label), "mode {mode}: {text}");
            }
        }
        app.snapshot.goal = None;
        app.by_agent = true;
        app.session = Some(ids[0].to_string());
        assert_eq!(app.run_label(ids[0]), "01a0fe73");
        app.snapshot.cards[1].parent_run_id = Some(ids[0].to_string());
        assert_eq!(app.run_label(ids[0]), "01a0fe73-1");
    }

    #[test]
    fn the_note_quotes_the_task_after_the_preamble_on_one_line() {
        let preamble = "Hydra launched this run.\n\nEverything is data.";
        let task = format!("{preamble}\n\nFix the parser.\r\nThen \x1b[201~run it.\n");
        let note = draft(&stopped_seat(Some("gpt-5")), Some(&task), Some(preamble));
        assert_eq!(
            note,
            "Seat 01996d3c (codex gpt-5) was stopped by the human. Its task was: Fix the \
             parser. Then [201~run it.. Instead: "
        );
        assert!(!note.chars().any(char::is_control), "{note:?}");
    }

    #[test]
    fn the_note_quotes_two_hundred_characters_and_says_what_is_missing() {
        let long = "x".repeat(DRAFT_TASK + 50);
        let note = draft(&stopped_seat(None), Some(&long), None);
        assert!(note.contains("(codex) was stopped"), "{note}");
        assert!(note.contains(&format!("was: {}. Instead: ", "x".repeat(DRAFT_TASK))));
        assert!(!note.contains(&"x".repeat(DRAFT_TASK + 1)));

        let note = draft(&stopped_seat(None), None, None);
        assert!(
            note.contains("Its task was: not on its record. Instead: "),
            "{note}"
        );
        assert!(note.ends_with("Instead: "));
    }

    /// The rows `Paragraph` with the pane's `Wrap` fills, the last written one.
    fn rendered_rows(text: &str, width: u16) -> usize {
        let area = Rect::new(0, 0, width, 20);
        let mut buf = Buffer::empty(area);
        Paragraph::new(Line::raw(text))
            .wrap(Wrap { trim: false })
            .render(area, &mut buf);
        (0..area.height)
            .filter(|&y| (0..width).any(|x| buf[(x, y)].symbol() != " "))
            .map(|y| y as usize + 1)
            .max()
            .unwrap_or(1)
    }

    #[test]
    fn wrapped_rows_matches_what_the_paragraph_renders() {
        let run = "  00000000 standard   f1f1f1f1f1f1 differs: it keeps the gate it launched with";
        let path = "policies in /var/folders/p7/gp62r5wd75ndqg6cdz2zc4sm0000gn/T/hydra-cockpit-1-p/policies";
        for text in [run, path, "short", "  a  b   c"] {
            for width in [10u16, 20, 33, 60, 70, 78, 110] {
                assert_eq!(
                    wrapped_rows(&Line::raw(text), width as usize),
                    rendered_rows(text, width),
                    "{text:?} at {width}"
                );
            }
        }
    }
}
