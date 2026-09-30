//! The cockpit's state, its event handling and its drawing, with no terminal.
//!
//! [`App::draw`] paints into a [`Buffer`] and [`App::handle`] takes a crossterm
//! [`Event`], so the suite drives both without a screen. Every rectangle a
//! click can land on is kept from the last draw in [`Hits`], and a click is a
//! `Rect::contains` over those.

use std::time::Duration;

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, MouseButton, MouseEventKind};
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Layout as Split, Position, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, Paragraph, Widget, Wrap};

use super::snapshot::{Card, Lane, Snapshot};
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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Effect {
    None,
    /// Ask the reader for a fresh snapshot now, with the open run's tail.
    Refresh,
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
}

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
    pub opened: Option<Opened>,
    /// The request the confirmation shows, while it is up.
    pub confirming: Option<Pending>,
    pub message: Option<String>,
    pub hits: Hits,
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
            opened: None,
            confirming: None,
            message: None,
            hits: Hits::default(),
        }
    }

    /// Takes a fresh read and keeps every selection inside it.
    pub fn set_snapshot(&mut self, snapshot: Snapshot) {
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
                KeyCode::Char('y') => self.approve(),
                KeyCode::Char('n') | KeyCode::Esc => {
                    self.confirming = None;
                    self.message = Some("nothing approved".to_string());
                    Effect::None
                }
                _ => Effect::None,
            };
        }
        match key.code {
            KeyCode::Char('q') => Effect::Quit,
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
            KeyCode::Char('a') => self.ask_to_approve(),
            _ => Effect::None,
        }
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

    fn ask_to_approve(&mut self) -> Effect {
        let Some(request) = self.request_in_hand() else {
            self.message = Some("select a request in the waiting approval lane first".to_string());
            return Effect::None;
        };
        if self.seatbelted {
            self.message = Some(inside_seatbelt());
            return Effect::None;
        }
        self.confirming = Some(request);
        Effect::None
    }

    /// `y` on the confirmation: [`crate::approve::approve`], the function
    /// `hydra approve <id>` calls once it has refused a seatbelt.
    fn approve(&mut self) -> Effect {
        let Some(request) = self.confirming.take() else {
            return Effect::None;
        };
        if self.seatbelted {
            self.message = Some(inside_seatbelt());
            return Effect::None;
        }
        self.message = Some(
            match crate::approve::approve(&self.layout, &request.request_id) {
                Ok(approved) => format!(
                    "approved {}: session {} launches it now",
                    approved.request_id, approved.session
                ),
                Err(err) => format!("{err:#}"),
            },
        );
        Effect::Refresh
    }

    // --- drawing ------------------------------------------------------------

    /// Paints the whole screen into `buf` and keeps what was drawn where.
    pub fn draw(&mut self, area: Rect, buf: &mut Buffer) {
        self.hits = Hits::default();
        let [main, status] =
            Split::vertical([Constraint::Min(3), Constraint::Length(1)]).areas(area);
        let [left, middle, right] = Split::horizontal([
            Constraint::Percentage(44),
            Constraint::Percentage(26),
            Constraint::Percentage(30),
        ])
        .areas(main);
        self.draw_table(left, buf);
        let lanes: [Rect; 4] = Split::vertical([Constraint::Ratio(1, 4); 4]).areas(middle);
        for lane in Lane::ALL {
            self.draw_lane(lane, lanes[lane.index()], buf);
        }
        self.draw_detail(right, buf);
        self.draw_status(status, buf);
        if let Some(request) = self.confirming.clone() {
            draw_confirmation(&request, main, buf);
        }
    }

    fn block(&self, focus: Focus, title: String) -> Block<'static> {
        let style = if self.focus == focus {
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
        let header = row_text(["id", "class", "grade", "provider", "model", "status", "run"]);
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
            let text = row_text([
                &row.id,
                &row.class,
                &row.grade,
                row.provider.as_deref().unwrap_or("-"),
                row.model.as_deref().unwrap_or("-"),
                &row.status,
                run,
            ]);
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
        let lines = self.detail_lines();
        Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false })
            .scroll((self.detail_scroll, 0))
            .render(area, buf);
    }

    fn detail_lines(&self) -> Vec<Line<'static>> {
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
                let mut lines = vec![
                    label("run", card.run_id.clone()),
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
                ];
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
                lines.push(Line::styled(
                    "last record lines",
                    Style::default().add_modifier(Modifier::BOLD),
                ));
                match &self.snapshot.tail {
                    Some((tail_id, tail)) if tail_id == id => {
                        lines.extend(tail.iter().map(|l| Line::raw(l.clone())));
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
        let keys = "arrows/Tab move, Enter opens, Esc closes, a approves, q quits";
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

/// What the cockpit says where `hydra approve` would refuse by name.
fn inside_seatbelt() -> String {
    format!(
        "{}: the cockpit is inside a seatbelt, which is where a model runs, and no model \
         approves a seat request (map ticket 82). Nothing was written.",
        crate::approve::INSIDE_SEATBELT
    )
}

fn draw_confirmation(request: &Pending, area: Rect, buf: &mut Buffer) {
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
    lines.push(Line::styled(
        "y approves, n closes",
        Style::default().add_modifier(Modifier::BOLD),
    ));
    Paragraph::new(lines)
        .block(
            Block::bordered()
                .border_style(Style::default().fg(Color::Red))
                .title(" approve this request? "),
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

/// The first eight characters of an id, which is what `puppets.py board` shows.
fn short(id: &str) -> &str {
    id.get(..8).unwrap_or(id)
}

/// One table line with the columns `board` prints, narrower than its widths
/// so the run id fits beside two other panels.
fn row_text(cells: [&str; 7]) -> String {
    let widths = [8, 20, 6, 12, 12, 8, 8];
    cells
        .iter()
        .zip(widths)
        .map(|(cell, width)| format!("{cell:<width$}"))
        .collect::<Vec<_>>()
        .join(" ")
}
