//! The cockpit's state, its event handling and its drawing, with no terminal.
//!
//! [`App::draw`] paints into a [`Buffer`] and [`App::handle`] takes a crossterm
//! [`Event`], so the suite drives both without a screen. Every rectangle a
//! click can land on is kept from the last draw in [`Hits`], and a click is a
//! `Rect::contains` over those.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, MouseButton, MouseEventKind};
use ratatui::layout::{Position, Rect};

use super::keys::MainKey;
use super::mascot::Mascot;
use super::snapshot::{CallLine, Card, Lane, REVEAL_SECONDS, Snapshot, Step};
use crate::approve::Pending;
use crate::paths::Layout;

mod confirm;
mod draw;
mod pages;
mod review;
mod stop;
mod text;

use confirm::{inside_seatbelt, step_inside_seatbelt};
use pages::Scrolled;
use review::ReviewRun;
use stop::StopRun;
pub use text::{draft, wrapped_rows};
use text::{follow, short_sha};

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
    /// The full-page goal table, when `t` opened it.
    pub goal_table_page: Rect,
    /// The run graph, when `c` opened it.
    pub graph_page: Rect,
    /// The policy pane, while it is up, how many runs it had rows for, and
    /// the offset of its last page.
    pub policy_pane: Rect,
    pub policy_rows: usize,
    pub policy_last: usize,
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
    /// The goal's table over the cockpit, build ticket 263.
    goal_table_open: bool,
    goal_table_scroll: u16,
    goal_table_place: Option<CockpitPlace>,
    goal_table_area: Rect,
    /// The run whose graph is over the cockpit, build ticket 271.
    graph_open: Option<String>,
    graph_scroll: u16,
    graph_area: Rect,
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
            goal_table_open: false,
            goal_table_scroll: 0,
            goal_table_place: None,
            goal_table_area: Rect::default(),
            graph_open: None,
            graph_scroll: 0,
            graph_area: Rect::default(),
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

    /// The run whose tail the reader should read: the graph's while it is
    /// up, then the one the detail has open.
    pub fn open_run_id(&self) -> Option<String> {
        if let Some(id) = &self.graph_open {
            return Some(id.clone());
        }
        match &self.opened {
            Some(Opened::Run(id)) => Some(id.clone()),
            _ => None,
        }
    }

    fn table_len(&self) -> usize {
        self.snapshot
            .goal
            .as_ref()
            .map_or(self.snapshot.session_runs.len(), |goal| goal.rows.len())
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

    /// The runs the lanes by agent show. With a session named, only it, the
    /// runs it launched, and runs that started after it, the ones the dragon
    /// counts; otherwise every run.
    fn agent_cards(&self) -> impl Iterator<Item = &Card> {
        let session = self.session.as_deref();
        let run_info = &self.snapshot.run_info;
        self.snapshot.cards.iter().filter(move |card| {
            session.is_none_or(|session| Snapshot::is_session_seat(session, card, run_info))
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
        if self.graph_open.is_some() {
            if matches!(key.code, KeyCode::Left | KeyCode::Backspace) {
                return self.close_graph();
            }
            if self.graph_scroll_key(key) {
                return Effect::None;
            }
        }
        if self.goal_table_open {
            if matches!(key.code, KeyCode::Left | KeyCode::Backspace) {
                return self.close_goal_table();
            }
            if self.goal_table_scroll_key(key) {
                return Effect::None;
            }
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
            && self.scroll_key(key, self.hits.detail, Scrolled::Detail)
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
                if self.graph_open.is_some() {
                    return self.close_graph();
                }
                if self.goal_table_open {
                    return self.close_goal_table();
                }
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
            Some(MainKey::ToggleGoalTable) => self.toggle_goal_table(),
            Some(MainKey::ToggleGraph) => self.toggle_graph(),
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
                        self.detail_scroll
                            .saturating_add(1)
                            .min(self.content_metrics(self.hits.detail, Scrolled::Detail).2)
                    } else {
                        self.detail_scroll.saturating_sub(1)
                    };
                }
            }
        }
        Effect::None
    }

    fn open_selected(&mut self) -> Effect {
        if self.focus == Focus::Detail {
            return Effect::None;
        }
        match self.selected_item() {
            Some(opened) => self.open(opened),
            None => {
                self.message = Some("nothing to open there".to_string());
                Effect::None
            }
        }
    }

    /// What `Enter` would open from the focused panel; on the detail, what
    /// it has open.
    fn selected_item(&self) -> Option<Opened> {
        match self.focus {
            Focus::Table => self
                .snapshot
                .goal
                .as_ref()
                .and_then(|goal| goal.rows.get(self.table_selected))
                .and_then(|row| row.run_id.clone())
                .or_else(|| {
                    self.snapshot
                        .goal
                        .is_none()
                        .then(|| self.snapshot.session_runs.get(self.table_selected).cloned())
                        .flatten()
                })
                .map(Opened::Run),
            focus @ (Focus::Lane(_) | Focus::Agent(_)) => self.item(focus, self.selected(focus)),
            Focus::Detail => self.opened.clone(),
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
            self.scroll_by(self.hits.run_page, Scrolled::RunPage, step);
            return Effect::None;
        }
        if self.graph_open.is_some() && self.hits.graph_page.contains(at) {
            let step = if down { 3 } else { -3 };
            self.scroll_by(self.hits.graph_page, Scrolled::Graph, step);
            return Effect::None;
        }
        if self.goal_table_open && self.hits.goal_table_page.contains(at) {
            let step = if down { 3 } else { -3 };
            self.scroll_by(self.hits.goal_table_page, Scrolled::GoalTable, step);
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
                    self.detail_scroll
                        .saturating_add(3)
                        .min(self.content_metrics(self.hits.detail, Scrolled::Detail).2)
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
}

#[cfg(test)]
mod tests {
    use ratatui::buffer::Buffer;
    use ratatui::text::Line;
    use ratatui::widgets::{Paragraph, Widget, Wrap};

    use super::text::DRAFT_TASK;
    use super::*;
    use crate::cockpit::goal::{Goal, TaskRow};

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
        let rows = ids
            .iter()
            .enumerate()
            .map(|(i, id)| TaskRow {
                id: format!("task{i}"),
                text: String::new(),
                class: String::new(),
                grade: String::new(),
                deps: Vec::new(),
                provider: None,
                model: None,
                policy: None,
                status: "running".to_string(),
                run_id: Some(id.to_string()),
                done: None,
                launch_approval_needed: false,
                launch_approved: None,
            })
            .collect();
        app.snapshot.goal = Some(Goal {
            rows,
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
