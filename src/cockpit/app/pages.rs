//! The run page and the goal table page, the places they return to, and
//! scrolling.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

use super::text::{short, wrapped_rows};
use super::{App, CockpitPlace, Effect, Focus, Opened, PageLayer};

impl App {
    pub fn on_run_page(&self) -> bool {
        self.run_page_place.is_some()
    }

    pub fn on_goal_table_page(&self) -> bool {
        self.goal_table_open
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
            "Up/Down scroll, PageUp/PageDown or Space/b page, Home/g End/G, Left/Backspace back, \
             t or Esc to cockpit",
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
        Effect::Refresh
    }

    pub(super) fn close_goal_table(&mut self) -> Effect {
        if !self.goal_table_open {
            return Effect::None;
        }
        self.goal_table_open = false;
        self.goal_table_scroll = 0;
        if let Some(place) = self.goal_table_place.take() {
            self.restore_place(place);
        }
        Effect::Refresh
    }

    pub(super) fn goal_table_lines(&self) -> Vec<Line<'static>> {
        let dim = Style::default().add_modifier(Modifier::DIM);
        let label = |name: &str, value: String| {
            Line::from(vec![
                Span::styled(format!("{name} "), dim),
                Span::raw(value),
            ])
        };
        let Some(goal) = self.snapshot.goal.as_ref() else {
            return vec![Line::raw("no goal")];
        };
        let mut lines = vec![
            label("goal", goal.text.clone().unwrap_or_default()),
            label("table revision", goal.revision.to_string()),
            label(
                "table approved",
                if goal.table_approved { "yes" } else { "no" }.to_string(),
            ),
            label(
                "plan approved",
                if goal.plan_approved { "yes" } else { "no" }.to_string(),
            ),
            Line::raw(""),
        ];
        lines.extend(self.panel_lines(goal));
        lines.push(Line::raw(""));
        for row in &goal.rows {
            lines.push(Line::styled(
                row.id.clone(),
                Style::default().add_modifier(Modifier::BOLD),
            ));
            lines.push(label("text", row.text.clone()));
            lines.push(label("class", row.class.clone()));
            lines.push(label("grade", row.grade.clone()));
            lines.push(label(
                "provider",
                row.provider.clone().unwrap_or_else(|| "-".into()),
            ));
            lines.push(label(
                "model",
                row.model.clone().unwrap_or_else(|| "-".into()),
            ));
            lines.push(label(
                "policy",
                row.policy.clone().unwrap_or_else(|| "-".into()),
            ));
            let deps = if row.deps.is_empty() {
                "-".to_string()
            } else {
                row.deps.join(", ")
            };
            lines.push(label("deps", deps));
            lines.push(label("status", row.status.clone()));
            let run = row
                .run_id
                .as_deref()
                .map(short)
                .map(str::to_string)
                .unwrap_or_else(|| "-".to_string());
            lines.push(label("run", run));
            let launch = if row.launch_approval_needed {
                if row.launch_approved {
                    "needed, approved"
                } else {
                    "needed, not approved"
                }
            } else {
                "not needed"
            };
            lines.push(label("launch approval", launch.to_string()));
            lines.push(Line::raw(""));
        }
        lines.push(Line::raw(""));
        lines.push(Self::goal_table_footer());
        lines
    }

    pub(super) fn goal_table_scroll_key(&mut self, key: &KeyEvent) -> bool {
        let area = if self.hits.goal_table_page.is_empty() {
            self.goal_table_area
        } else {
            self.hits.goal_table_page
        };
        self.scroll_key(key, area, Scrolled::GoalTable)
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
            Scrolled::GoalTable => self.goal_table_lines(),
            Scrolled::Detail => self.detail_scroll_lines(width, false),
            Scrolled::RunPage => self.detail_scroll_lines(width, true),
        }
    }

    fn scroll_of(&mut self, scrolled: Scrolled) -> &mut u16 {
        match scrolled {
            Scrolled::GoalTable => &mut self.goal_table_scroll,
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

/// Which text a scroll moves: the goal table page, a run's detail in the
/// detail pane, or the same detail on the run page, under the page's footer.
#[derive(Clone, Copy)]
pub(super) enum Scrolled {
    GoalTable,
    Detail,
    RunPage,
}
