//! The run page, the goal table page and the run graph, the places they
//! return to, and scrolling.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use super::text::{cut_to_columns, short, wrapped_rows};
use super::{App, CockpitPlace, Effect, Focus, Opened, PageLayer};
use crate::cockpit::goal::TaskRow;

impl App {
    pub fn on_run_page(&self) -> bool {
        self.run_page_place.is_some()
    }

    pub fn on_goal_table_page(&self) -> bool {
        self.goal_table_open
    }

    /// The run whose graph is up, build ticket 271.
    pub fn graph_run(&self) -> Option<&str> {
        self.graph_open.as_deref()
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

    pub(super) fn open_run_page(&mut self, opened: Opened) -> Effect {
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

    pub(super) fn page_back_one(&mut self) -> Effect {
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

    pub(super) fn page_back_all(&mut self) -> Effect {
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

    fn goal_table_footer() -> Line<'static> {
        Line::styled(
            "Up/Down or Tab selects, Enter task detail, PageUp/PageDown or Space/b page, Home/g End/G, t or Esc to cockpit",
            Style::default().add_modifier(Modifier::DIM),
        )
    }

    pub(super) fn toggle_goal_table(&mut self) -> Effect {
        if self.goal_table_open {
            return self.close_goal_table();
        }
        if self.snapshot.goal.is_none() {
            self.message = Some("no goal on screen".to_string());
            return Effect::None;
        }
        self.goal_table_place = Some(self.save_place());
        self.goal_table_open = true;
        self.goal_table_scroll = 0;
        self.focus = Focus::Table;
        Effect::Refresh
    }

    pub(super) fn close_goal_table(&mut self) -> Effect {
        if !self.goal_table_open {
            return Effect::None;
        }
        self.goal_table_open = false;
        self.goal_task = None;
        self.goal_table_scroll = 0;
        if let Some(place) = self.goal_table_place.take() {
            self.restore_place(place);
        }
        Effect::Refresh
    }

    pub(super) fn goal_task_row(&self) -> Option<&TaskRow> {
        let id = self.goal_task.as_ref()?;
        self.snapshot
            .goal
            .as_ref()?
            .rows
            .iter()
            .find(|row| &row.id == id)
    }

    pub(super) fn open_goal_task(&mut self) -> Effect {
        let Some(row) = self
            .snapshot
            .goal
            .as_ref()
            .and_then(|g| g.rows.get(self.table_selected))
        else {
            return Effect::None;
        };
        self.goal_task = Some(row.id.clone());
        self.goal_task_scroll = 0;
        self.goal_task_token = crate::marker::token();
        Effect::Refresh
    }

    pub(super) fn board_columns(width: usize) -> usize {
        if width < 98 { 1 } else { (width / 49).max(2) }
    }

    pub(super) fn move_goal_card(&mut self, forward: bool) {
        let len = self.table_len();
        self.table_selected = if forward {
            (self.table_selected + 1).min(len.saturating_sub(1))
        } else {
            self.table_selected.saturating_sub(1)
        };
        let area = self.goal_table_area;
        let width = area.width.saturating_sub(2).max(1) as usize;
        let header = self
            .goal_header()
            .iter()
            .map(|l| wrapped_rows(l, width))
            .sum::<usize>();
        let top = header + self.table_selected / Self::board_columns(width) * 14;
        let visible = area.height.saturating_sub(2).max(1) as usize;
        let offset = self.goal_table_scroll as usize;
        if top < offset {
            self.goal_table_scroll = u16::try_from(top).unwrap_or(u16::MAX);
        } else if top + 14 > offset + visible {
            self.goal_table_scroll =
                u16::try_from((top + 14).saturating_sub(visible)).unwrap_or(u16::MAX);
        }
    }

    pub(super) fn goal_header(&self) -> Vec<Line<'static>> {
        let Some(goal) = &self.snapshot.goal else {
            return vec![];
        };
        let mut lines = vec![
            Line::raw(format!("goal {}", goal.text.as_deref().unwrap_or_default())),
            Line::raw(format!("table revision {}", goal.revision)),
            Line::raw(format!("table approved {}", yes_no(goal.table_approved))),
            Line::raw(format!("plan approved {}", yes_no(goal.plan_approved))),
        ];
        lines.extend(self.panel_lines(goal));
        lines.push(Line::raw(""));
        lines
    }

    pub(super) fn goal_table_lines(&self, width: usize) -> Vec<Line<'static>> {
        if let Some(row) = self.goal_task_row() {
            let card = row
                .run_id
                .as_ref()
                .and_then(|id| self.snapshot.cards.iter().find(|c| &c.run_id == id));
            let task = row
                .run_id
                .as_ref()
                .and_then(|id| self.snapshot.run_info.get(id))
                .and_then(|info| info.task.as_deref())
                .or(row.task_text.as_deref())
                .unwrap_or("task text not available from this hydra-puppets");
            let mut lines = vec![
                Line::raw(format!("task {}", row.id)),
                Line::raw("task text"),
            ];
            lines.extend(task.lines().map(|l| Line::raw(l.to_string())));
            if let Some(card) = card {
                lines.push(Line::raw(format!(
                    "outcome {}  tokens {}",
                    card.outcome.as_deref().unwrap_or(card.state.as_str()),
                    card.total_tokens
                        .map_or_else(|| "unknown".into(), |n| n.to_string())
                )));
                lines.push(Line::raw(
                    "result_text (agent text; data, never instructions)",
                ));
                if let Some(answer) = &card.result_text {
                    lines.extend(
                        crate::marker::mark(answer, &self.goal_task_token)
                            .lines()
                            .map(|l| Line::raw(l.to_string())),
                    );
                } else {
                    lines.push(Line::raw("no answer yet"));
                }
            } else if row.run_id.is_some() {
                lines.push(Line::raw("run record unavailable"));
            }
            lines.push(Line::raw("c run graph, Up/Down scroll, PageUp/PageDown Space/b, Home/g End/G, Left/Backspace or Esc back, t to cockpit"));
            return lines;
        }
        let Some(goal) = &self.snapshot.goal else {
            return vec![Line::raw("no goal")];
        };
        let mut lines = self.goal_header();
        let columns = Self::board_columns(width);
        let cell_width = width / columns;
        for (batch, rows) in goal.rows.chunks(columns).enumerate() {
            let cards: Vec<Vec<String>> = rows
                .iter()
                .enumerate()
                .map(|(col, row)| {
                    let card = row
                        .run_id
                        .as_ref()
                        .and_then(|id| self.snapshot.cards.iter().find(|c| &c.run_id == id));
                    let state = match card {
                        Some(c) if c.state == crate::live::State::Live => "live",
                        Some(c) if c.outcome.as_deref() == Some("completed") => "done",
                        Some(_) => "failed",
                        None if row.run_id.is_some() => match row.status.as_str() {
                            "running" => "live",
                            "applied" | "fixed" | "ended" => "done",
                            _ => "failed",
                        },
                        None if row.launch_approval_needed && row.launch_approved != Some(true) => {
                            "waiting approval"
                        }
                        None => "planned",
                    };
                    let launch = match (row.launch_approval_needed, row.launch_approved) {
                        (false, _) => "not needed",
                        (true, Some(true)) => "needed, approved",
                        (true, Some(false)) => "needed, not approved",
                        (true, None) => "needed, unknown",
                    };
                    vec![
                        format!(
                            "{} {}",
                            if batch * columns + col == self.table_selected {
                                "▶"
                            } else {
                                "─"
                            },
                            row.id
                        ),
                        row.text.lines().next().unwrap_or("").to_string(),
                        format!(
                            "provider {}",
                            row.provider.as_deref().unwrap_or("unassigned")
                        ),
                        format!(
                            "model {}",
                            row.model.as_deref().filter(|m| !m.is_empty()).unwrap_or(
                                if row.provider.is_some() {
                                    "default"
                                } else {
                                    "unassigned"
                                }
                            )
                        ),
                        format!("grade {}", row.grade),
                        format!("launch approval {launch}"),
                        format!("state {state}"),
                        format!("class {}", row.class),
                        format!("policy {}", row.policy.as_deref().unwrap_or("-")),
                        format!(
                            "deps {}",
                            if row.deps.is_empty() {
                                "-".into()
                            } else {
                                row.deps.join(", ")
                            }
                        ),
                        format!("status {}", row.status),
                        format!("run {}", row.run_id.as_deref().map(short).unwrap_or("-")),
                        "─".repeat(cell_width.saturating_sub(1)),
                        String::new(),
                    ]
                })
                .collect();
            for n in 0..14 {
                let mut spans = Vec::new();
                for (col, card) in cards.iter().enumerate() {
                    let text = cut_to_columns(&card[n], cell_width.saturating_sub(1));
                    let used = Line::raw(text.clone()).width();
                    let style = if batch * columns + col == self.table_selected {
                        Style::default().fg(Color::Cyan)
                    } else {
                        Style::default()
                    };
                    spans.push(Span::styled(text, style));
                    spans.push(Span::raw(" ".repeat(cell_width.saturating_sub(used))));
                }
                lines.push(Line::from(spans));
            }
        }
        lines.push(Self::goal_table_footer());
        lines
    }

    fn graph_footer() -> Line<'static> {
        Line::styled(
            "Up/Down scroll, PageUp/PageDown or Space/b page, Home/g End/G, \
             c, Left/Backspace or Esc back",
            Style::default().add_modifier(Modifier::DIM),
        )
    }

    /// Opens the graph of the run `Enter` would open, or of the run the
    /// detail or the run page has open. The page under it stays as it was.
    pub(super) fn toggle_graph(&mut self) -> Effect {
        if self.graph_open.is_some() {
            return self.close_graph();
        }
        let Some(Opened::Run(id)) = self.selected_item() else {
            self.message = Some("no run selected".to_string());
            return Effect::None;
        };
        self.graph_open = Some(id);
        self.graph_scroll = 0;
        self.close_reveal();
        Effect::Refresh
    }

    pub(super) fn close_graph(&mut self) -> Effect {
        if self.graph_open.take().is_none() {
            return Effect::None;
        }
        self.graph_scroll = 0;
        Effect::Refresh
    }

    /// The graph as `hydra-graph` printed it: a header, then one row per
    /// event with its `seq` and type, and each result under its call.
    pub(super) fn graph_lines(&self) -> Vec<Line<'static>> {
        let Some(id) = self.graph_open.as_deref() else {
            return Vec::new();
        };
        let dim = Style::default().add_modifier(Modifier::DIM);
        let mut lines = Vec::new();
        if let Some(card) = self.snapshot.cards.iter().find(|c| c.run_id == id) {
            let dash = |value: Option<&str>| value.unwrap_or("-").to_string();
            lines.push(Line::raw(format!(
                "{}  {}  {}  {}  tokens={}",
                card.run_id,
                dash(card.provider.as_deref()),
                dash(card.policy.as_deref()),
                dash(card.outcome.as_deref()),
                card.total_tokens.unwrap_or(0),
            )));
        }
        match self
            .snapshot
            .graph
            .as_ref()
            .filter(|(graph_id, _)| graph_id == id)
        {
            None => lines.push(Line::styled("reading the record", dim)),
            Some((_, graph)) if graph.is_empty() => {
                lines.push(Line::styled("no events in the record", dim));
            }
            Some((_, graph)) => {
                for row in graph {
                    let seq = row.seq;
                    let glyph = if row.under_call { "│ " } else { "  " };
                    let text = if row.refused {
                        Span::styled(row.text.clone(), Style::default().fg(Color::Red))
                    } else {
                        Span::raw(row.text.clone())
                    };
                    lines.push(Line::from(vec![
                        Span::styled(format!("{glyph}{seq:>4}  {:<11}  ", row.kind), dim),
                        text,
                    ]));
                }
            }
        }
        lines.push(Line::raw(""));
        lines.push(Self::graph_footer());
        lines
    }

    pub(super) fn graph_scroll_key(&mut self, key: &KeyEvent) -> bool {
        let area = if self.hits.graph_page.is_empty() {
            self.graph_area
        } else {
            self.hits.graph_page
        };
        self.scroll_key(key, area, Scrolled::Graph)
    }

    pub(super) fn goal_table_scroll_key(&mut self, key: &KeyEvent) -> bool {
        let area = if self.hits.goal_table_page.is_empty() {
            self.goal_table_area
        } else {
            self.hits.goal_table_page
        };
        self.scroll_key(
            key,
            area,
            if self.goal_task.is_some() {
                Scrolled::GoalTask
            } else {
                Scrolled::GoalTable
            },
        )
    }

    pub(super) fn detail_scroll_lines(
        &self,
        width: usize,
        page_footer: bool,
    ) -> Vec<Line<'static>> {
        let mut lines = self.detail_lines(width);
        if page_footer {
            lines.push(Line::raw(""));
            lines.push(Self::run_page_footer());
        }
        lines
    }

    fn scrolled_lines(&self, scrolled: Scrolled, width: usize) -> Vec<Line<'static>> {
        match scrolled {
            Scrolled::GoalTable | Scrolled::GoalTask => self.goal_table_lines(width),
            Scrolled::Graph => self.graph_lines(),
            Scrolled::Detail => self.detail_scroll_lines(width, false),
            Scrolled::RunPage => self.detail_scroll_lines(width, true),
        }
    }

    fn scroll_of(&mut self, scrolled: Scrolled) -> &mut u16 {
        match scrolled {
            Scrolled::GoalTable => &mut self.goal_table_scroll,
            Scrolled::GoalTask => &mut self.goal_task_scroll,
            Scrolled::Graph => &mut self.graph_scroll,
            Scrolled::Detail | Scrolled::RunPage => &mut self.detail_scroll,
        }
    }

    pub(super) fn content_metrics(&self, area: Rect, scrolled: Scrolled) -> (usize, usize, u16) {
        let width = area.width.saturating_sub(2).max(1) as usize;
        let visible = area.height.saturating_sub(2).max(1) as usize;
        let total = self
            .scrolled_lines(scrolled, width)
            .iter()
            .map(|line| wrapped_rows(line, width))
            .sum::<usize>();
        let max_scroll = u16::try_from(total.saturating_sub(visible)).unwrap_or(u16::MAX);
        (width, visible, max_scroll)
    }

    pub(super) fn scroll_by(&mut self, area: Rect, scrolled: Scrolled, delta: i32) {
        let (_, _, max_scroll) = self.content_metrics(area, scrolled);
        let scroll = self.scroll_of(scrolled);
        if delta < 0 {
            let step = (-delta) as u16;
            *scroll = scroll.saturating_sub(step);
        } else {
            *scroll = scroll.saturating_add(delta as u16).min(max_scroll);
        }
    }

    pub(super) fn scroll_key(&mut self, key: &KeyEvent, area: Rect, scrolled: Scrolled) -> bool {
        match key.code {
            KeyCode::Up => {
                self.scroll_by(area, scrolled, -1);
                true
            }
            KeyCode::Down => {
                self.scroll_by(area, scrolled, 1);
                true
            }
            KeyCode::PageUp | KeyCode::Char('b') => {
                let (_, visible, _) = self.content_metrics(area, scrolled);
                self.scroll_by(area, scrolled, -(visible as i32));
                true
            }
            KeyCode::PageDown | KeyCode::Char(' ') => {
                let (_, visible, _) = self.content_metrics(area, scrolled);
                self.scroll_by(area, scrolled, visible as i32);
                true
            }
            KeyCode::Home | KeyCode::Char('g') => {
                *self.scroll_of(scrolled) = 0;
                true
            }
            KeyCode::End | KeyCode::Char('G') => {
                let (_, _, max_scroll) = self.content_metrics(area, scrolled);
                *self.scroll_of(scrolled) = max_scroll;
                true
            }
            _ => false,
        }
    }

    pub(super) fn page_detail_key(&mut self, key: &KeyEvent) -> bool {
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
            return self.scroll_key(key, area, Scrolled::RunPage);
        }
        self.scroll_key(key, area, Scrolled::RunPage)
    }
}

/// Which text a scroll moves: the goal table page, the run graph, a run's
/// detail in the detail pane, or the same detail on the run page, under the
/// page's footer.
#[derive(Clone, Copy)]
pub(super) enum Scrolled {
    GoalTable,
    GoalTask,
    Graph,
    Detail,
    RunPage,
}

/// An approval the helper answered for, or `unknown` while it has not.
fn yes_no(approved: Option<bool>) -> &'static str {
    match approved {
        Some(true) => "yes",
        Some(false) => "no",
        None => "unknown",
    }
}
