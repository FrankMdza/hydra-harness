//! The cockpit's state, its event handling and its drawing, with no terminal.
//!
//! [`App::draw`] paints into a [`Buffer`] and [`App::handle`] takes a crossterm
//! [`Event`], so the suite drives both without a screen. Every rectangle a
//! click can land on is kept from the last draw in [`Hits`], and a click is a
//! `Rect::contains` over those.

use std::path::{Path, PathBuf};
use std::time::Duration;

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, MouseButton, MouseEventKind};
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Layout as Split, Position, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, Paragraph, Widget, Wrap};

use super::goal::TaskRow;
use super::mascot::Mascot;
use super::snapshot::{CallLine, Card, Lane, Snapshot, spend_text};
use crate::approve::Pending;
use crate::paths::Layout;

/// Which panel has the keyboard.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Focus {
    Table,
    Lane(Lane),
    Detail,
}

impl Focus {
    const ORDER: [Focus; 6] = [
        Focus::Table,
        Focus::Lane(Lane::Live),
        Focus::Lane(Lane::Waiting),
        Focus::Lane(Lane::EndedOk),
        Focus::Lane(Lane::EndedNotOk),
        Focus::Detail,
    ];

    fn step(self, forward: bool) -> Focus {
        let at = Self::ORDER.iter().position(|f| *f == self).unwrap_or(0);
        let len = Self::ORDER.len();
        Self::ORDER[if forward {
            (at + 1) % len
        } else {
            (at + len - 1) % len
        }]
    }
}

/// What the detail panel has open.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Opened {
    Run(String),
    Request(String),
}

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
    /// A drawn card, its lane, and its index in that lane.
    pub cards: Vec<(Rect, Lane, usize)>,
    pub detail: Rect,
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
    detail_scroll: u16,
    /// Whether the detail shows the raw record lines rather than the tool
    /// calls. `r` flips it, build ticket 178.
    raw_tail: bool,
    pub opened: Option<Opened>,
    /// The request the confirmation shows, while it is up.
    pub confirming: Option<Pending>,
    /// Whether that confirmation denies rather than approves, build ticket 170.
    pub denying: bool,
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
    policy_selected: usize,
    /// The first live run the pane shows, build ticket 174.
    policy_offset: usize,
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
            detail_scroll: 0,
            raw_tail: false,
            opened: None,
            confirming: None,
            denying: false,
            message: None,
            hits: Hits::default(),
            mascot: None,
            has_keys: true,
            stacked: false,
            policies_dir: None,
            policy_set: None,
            policy_pane: false,
            policy_selected: 0,
            policy_offset: 0,
        }
    }

    /// Names the policy directory the pane shows and loads it once.
    pub fn set_policies(&mut self, dir: PathBuf) {
        self.policies_dir = Some(dir);
        self.reload_policies();
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

    /// Points the dragon at the session `run_id`, once the chat has named it.
    pub fn watch(&mut self, run_id: &str) {
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
    pub fn set_snapshot(&mut self, snapshot: Snapshot) {
        if self.policy_pane {
            self.reload_policies();
        }
        if let Some(mascot) = &mut self.mascot {
            mascot.observe(&snapshot);
        }
        self.snapshot = snapshot;
        self.table_selected = self.table_selected.min(self.table_len().saturating_sub(1));
        for lane in Lane::ALL {
            let i = lane.index();
            self.lane_selected[i] =
                self.lane_selected[i].min(self.lane_len(lane).saturating_sub(1));
        }
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
            Lane::Waiting => self.snapshot.requests.len(),
            _ => self.cards_in(lane).count(),
        }
    }

    /// The run ids in a lane, in drawn order. Empty for `Waiting`.
    pub fn lane_runs(&self, lane: Lane) -> Vec<String> {
        self.cards_in(lane)
            .map(|card| card.run_id.clone())
            .collect()
    }

    /// What `Enter` would open in `lane` at `index`.
    fn item(&self, lane: Lane, index: usize) -> Option<Opened> {
        match lane {
            Lane::Waiting => self
                .snapshot
                .requests
                .get(index)
                .map(|request| Opened::Request(request.request_id.clone())),
            _ => self
                .cards_in(lane)
                .nth(index)
                .map(|card| Opened::Run(card.run_id.clone())),
        }
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
        if self.policy_pane {
            return self.policy_key(key);
        }
        match key.code {
            KeyCode::Char('q') => Effect::Quit,
            KeyCode::Char('p') => {
                self.reload_policies();
                self.policy_pane = true;
                self.policy_offset = 0;
                Effect::None
            }
            KeyCode::Tab | KeyCode::Right => {
                self.focus = self.focus.step(true);
                Effect::None
            }
            KeyCode::BackTab | KeyCode::Left => {
                self.focus = self.focus.step(false);
                Effect::None
            }
            KeyCode::Up => self.move_selection(false),
            KeyCode::Down => self.move_selection(true),
            KeyCode::Enter => self.open_selected(),
            KeyCode::Esc => {
                if self.opened.take().is_some() {
                    self.detail_scroll = 0;
                    return Effect::Refresh;
                }
                Effect::None
            }
            KeyCode::Char('a') => self.ask_to_answer(false),
            KeyCode::Char('d') => self.ask_to_answer(true),
            KeyCode::Char('r') => {
                self.raw_tail = !self.raw_tail;
                Effect::None
            }
            _ => Effect::None,
        }
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
            Focus::Lane(lane) => {
                let i = lane.index();
                self.lane_selected[i] = step(self.lane_selected[i], self.lane_len(lane));
                let rows = self.hits.lanes[i].height.saturating_sub(2) as usize;
                self.lane_offset[i] = follow(self.lane_selected[i], self.lane_offset[i], rows);
            }
            Focus::Detail => {
                self.detail_scroll = if down {
                    self.detail_scroll.saturating_add(1)
                } else {
                    self.detail_scroll.saturating_sub(1)
                };
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
            Focus::Lane(lane) => self.item(lane, self.lane_selected[lane.index()]),
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
        self.opened = Some(opened);
        self.detail_scroll = 0;
        Effect::Refresh
    }

    fn click(&mut self, at: Position) -> Effect {
        if self.confirming.is_some() {
            return Effect::None;
        }
        if let Some((_, index)) = self.hits.table_rows.iter().find(|(r, _)| r.contains(at)) {
            self.focus = Focus::Table;
            self.table_selected = *index;
            return Effect::None;
        }
        if let Some((_, lane, index)) = self.hits.cards.iter().find(|(r, _, _)| r.contains(at)) {
            let (lane, index) = (*lane, *index);
            self.focus = Focus::Lane(lane);
            self.lane_selected[lane.index()] = index;
            return match self.item(lane, index) {
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
            Some(Focus::Lane(lane)) => {
                let i = lane.index();
                let rows = self.hits.lanes[i].height.saturating_sub(2) as usize;
                let max = self.lane_len(lane).saturating_sub(rows);
                self.lane_offset[i] = nudge(self.lane_offset[i], max);
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
        self.hits.detail.contains(at).then_some(Focus::Detail)
    }

    // --- the one write -----------------------------------------------------

    /// The request `a` would ask about: the one open in the detail, or the
    /// one selected in the waiting lane.
    fn request_in_hand(&self) -> Option<Pending> {
        let id = match (&self.opened, self.focus) {
            (Some(Opened::Request(id)), _) => id.clone(),
            (_, Focus::Lane(Lane::Waiting)) => self
                .snapshot
                .requests
                .get(self.lane_selected[Lane::Waiting.index()])?
                .request_id
                .clone(),
            _ => return None,
        };
        self.snapshot
            .requests
            .iter()
            .find(|request| request.request_id == id)
            .cloned()
    }

    /// `a` or `d`: opens the confirmation that approves, or denies, the
    /// request in hand.
    fn ask_to_answer(&mut self, deny: bool) -> Effect {
        let Some(request) = self.request_in_hand() else {
            self.message = Some("select a request in the waiting approval lane first".to_string());
            return Effect::None;
        };
        if self.seatbelted {
            self.message = Some(inside_seatbelt());
            return Effect::None;
        }
        self.confirming = Some(request);
        self.denying = deny;
        Effect::None
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

    // --- drawing ------------------------------------------------------------

    /// Paints the whole screen into `buf` and keeps what was drawn where.
    pub fn draw(&mut self, area: Rect, buf: &mut Buffer) {
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
        let lanes: [Rect; 4] = Split::vertical([Constraint::Ratio(1, 4); 4]).areas(middle);
        for lane in Lane::ALL {
            self.draw_lane(lane, lanes[lane.index()], buf);
        }
        self.draw_detail(right, buf);
        self.draw_status(status, buf);
        if self.policy_pane {
            self.draw_policy_pane(main, buf);
        }
        if let Some(request) = self.confirming.clone() {
            draw_confirmation(&request, self.denying, main, buf);
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
        let footer = [
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
        let room = (rect.height.saturating_sub(2) as usize)
            .saturating_sub(rows_of(&lines) + rows_of(&footer));
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
        self.hits.policy_rows = shown;
        self.hits.policy_last = last;
        lines.extend(
            runs[self.policy_offset..self.policy_offset + shown]
                .iter()
                .cloned(),
        );
        if shown < live.len() {
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
        let dim = Style::default().add_modifier(Modifier::DIM);
        let mut heading = goal.text.clone().unwrap_or_default();
        if goal.skipped > 0 {
            heading = format!("{heading} ({} line(s) skipped)", goal.skipped);
        }
        buf.set_line(inner.x, inner.y, &Line::styled(heading, dim), inner.width);
        let columns = columns(inner.width);
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
            let run = row.run_id.as_deref().map(short).unwrap_or("");
            let text = row_text(&columns, |column| match column {
                Column::Id => &row.id,
                Column::Status => self.status_of(row),
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
    fn status_of<'a>(&'a self, row: &'a TaskRow) -> &'a str {
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
        let i = lane.index();
        self.hits.lanes[i] = area;
        let len = self.lane_len(lane);
        let block = self.block(Focus::Lane(lane), format!(" {} {len} ", lane.title()));
        let inner = block.inner(area);
        block.render(area, buf);
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
                .collect(),
            _ => self
                .cards_in(lane)
                .map(|card| {
                    let said = card
                        .outcome
                        .clone()
                        .unwrap_or_else(|| card.state.as_str().to_string());
                    format!(
                        "{} {} {said}",
                        short(&card.run_id),
                        card.provider.as_deref().unwrap_or("?")
                    )
                })
                .collect(),
        };
        let visible = inner.height as usize;
        self.lane_offset[i] = self.lane_offset[i].min(lines.len().saturating_sub(visible));
        for (slot, index) in (self.lane_offset[i]..lines.len()).take(visible).enumerate() {
            let rect = Rect {
                y: inner.y + slot as u16,
                height: 1,
                ..inner
            };
            let style = self.selected_style(Focus::Lane(lane), index == self.lane_selected[i]);
            buf.set_line(
                rect.x,
                rect.y,
                &Line::styled(lines[index].clone(), style),
                rect.width,
            );
            self.hits.cards.push((rect, lane, index));
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
                if self.raw_tail {
                    lines.push(Line::styled(
                        "last record lines (r shows tool calls)",
                        Style::default().add_modifier(Modifier::BOLD),
                    ));
                    match &self.snapshot.tail {
                        Some((tail_id, tail)) if tail_id == id => {
                            lines.extend(tail.iter().map(|l| Line::raw(l.clone())));
                        }
                        _ => lines.push(Line::raw("reading")),
                    }
                    return lines;
                }
                lines.push(Line::styled(
                    "tool calls (r shows the raw record lines)",
                    Style::default().add_modifier(Modifier::BOLD),
                ));
                match &self.snapshot.calls {
                    Some((calls_id, calls)) if calls_id == id => {
                        if calls.is_empty() {
                            lines.push(Line::raw("none yet"));
                        }
                        lines.extend(calls.iter().map(|call| call_line(call, width)));
                    }
                    _ => lines.push(Line::raw("reading")),
                }
                lines
            }
        }
    }

    fn draw_status(&self, area: Rect, buf: &mut Buffer) {
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
        let keys = "arrows/Tab move, Enter opens, Esc closes, r raw lines, a approves, d denies, \
                    p policies, q quits";
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

/// One tool call on one row: name, what the result said, then the input, which
/// is what gets cut when the row is wider than the pane.
fn call_line(call: &CallLine, width: usize) -> Line<'static> {
    let decision = call.decision.as_deref().unwrap_or("-");
    let style = if decision.starts_with("refused") {
        Style::default().fg(Color::Red)
    } else {
        Style::default().add_modifier(Modifier::DIM)
    };
    let head = format!("{} {decision} ", call.name);
    let room = width.saturating_sub(head.chars().count());
    let mut input: String = call.input.chars().take(room).collect();
    if input.chars().count() < call.input.chars().count() && room > 0 {
        input.pop();
        input.push('~');
    }
    Line::from(vec![
        Span::raw(format!("{} ", call.name)),
        Span::styled(format!("{decision} "), style),
        Span::raw(input),
    ])
}

/// What the cockpit says where `hydra approve` would refuse by name.
fn inside_seatbelt() -> String {
    format!(
        "{}: the cockpit is inside a seatbelt, which is where a model runs, and no model \
         approves a seat request (map ticket 82). Nothing was written.",
        crate::approve::INSIDE_SEATBELT
    )
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
fn wrapped_rows(line: &Line, width: usize) -> usize {
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

/// The first eight characters of an id, which is what `puppets.py board` shows.
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
fn columns(width: u16) -> Vec<(Column, usize)> {
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
