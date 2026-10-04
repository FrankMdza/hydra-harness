//! Drawing: everything [`App::draw`] paints.

use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Layout as Split, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, Paragraph, Widget, Wrap};
use unicode_width::UnicodeWidthStr;

use super::confirm::{
    draw_confirmation, draw_review_confirmation, draw_step_confirmation, draw_stop_confirmation,
    shown_argv,
};
use super::text::{
    Column, call_line, columns_with_run, panel_vote, row_text, short, short_sha, wrap_call_reveal,
    wrap_plain, wrapped_rows,
};
use super::{App, Focus, Hits, MIN_BODY, Opened, TITLE_WIDTH, agent_of};
use crate::cockpit::call_head;
use crate::cockpit::goal::{Goal, TaskRow};
use crate::cockpit::snapshot::{CallLine, Card, Lane, Snapshot, Step, spend_text};
use crate::live::State;

impl App {
    /// Paints the whole screen into `buf` and keeps what was drawn where.
    pub fn draw(&mut self, area: Rect, buf: &mut Buffer) {
        self.poll_reviews();
        self.poll_stops();
        self.poll_reveal();
        self.hits = Hits::default();
        let area = self.draw_header(area, buf);
        let [main, status] =
            Split::vertical([Constraint::Min(3), Constraint::Length(1)]).areas(area);
        let [left, middle, right] = if area.width <= 80 {
            Split::vertical([
                Constraint::Percentage(30),
                Constraint::Percentage(40),
                Constraint::Percentage(30),
            ])
            .areas(main)
        } else if self.stacked {
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
        if self.goal_table_open {
            self.draw_goal_table_page(main, buf);
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

    /// The help overlay: every key from [`super::super::keys::HELP`], grouped like the
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
        for (keys, description) in super::super::keys::help_rows(session) {
            let row_group = super::super::keys::HELP
                .iter()
                .find(|e| e.keys == keys)
                .map(|e| e.group);
            if row_group != group {
                group = row_group;
                if let Some(g) = group {
                    let title = match g {
                        super::super::keys::HelpGroup::Moving => "moving",
                        super::super::keys::HelpGroup::Opening => "opening",
                        super::super::keys::HelpGroup::Confirm => "confirm",
                        super::super::keys::HelpGroup::Views => "views",
                        super::super::keys::HelpGroup::Quit => "quit",
                        super::super::keys::HelpGroup::Help => "help",
                        super::super::keys::HelpGroup::PolicyPane => "policy pane",
                        super::super::keys::HelpGroup::Session => "with --session",
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

    pub(super) fn run_label<'a>(&'a self, id: &'a str) -> &'a str {
        let session = self.session.as_deref();
        let run_info = &self.snapshot.run_info;
        crate::resolver::unique_prefix(
            id,
            self.snapshot
                .cards
                .iter()
                .filter(|card| {
                    !self.by_agent
                        || session.is_none_or(|session| {
                            Snapshot::is_session_seat(session, card, run_info)
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

    pub(super) fn draw_table(&mut self, area: Rect, buf: &mut Buffer) {
        self.hits.table = area;
        let title = match &self.snapshot.goal {
            Some(goal) => format!(" tasks of {} ", goal.id),
            None => " tasks of this session ".to_string(),
        };
        let block = self.block(Focus::Table, title);
        let inner = block.inner(area);
        block.render(area, buf);
        let Some(goal) = self.snapshot.goal.clone() else {
            self.draw_session_tasks(inner, buf);
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
        let panel_lines = self.panel_lines(&goal);
        let panel_height = if panel_lines.is_empty() {
            0
        } else {
            panel_lines
                .iter()
                .map(|line| wrapped_rows(line, inner.width.max(1) as usize))
                .sum::<usize>()
                .min(inner.height.saturating_sub(3) as usize) as u16
        };
        if panel_height > 0 {
            Paragraph::new(panel_lines)
                .wrap(Wrap { trim: false })
                .render(
                    Rect::new(inner.x, inner.y + 1, inner.width, panel_height),
                    buf,
                );
        }
        let inner = Rect::new(
            inner.x,
            inner.y + panel_height,
            inner.width,
            inner.height - panel_height,
        );
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

    fn draw_session_tasks(&mut self, inner: Rect, buf: &mut Buffer) {
        if inner.height < 2 {
            return;
        }
        buf.set_line(
            inner.x,
            inner.y,
            &Line::raw("run status provider model policy description"),
            inner.width,
        );
        let visible = inner.height.saturating_sub(2) as usize;
        self.table_offset = self
            .table_offset
            .min(self.snapshot.session_runs.len().saturating_sub(visible));
        for (slot, index) in (self.table_offset..self.snapshot.session_runs.len())
            .take(visible)
            .enumerate()
        {
            let id = &self.snapshot.session_runs[index];
            let Some(card) = self.snapshot.cards.iter().find(|card| &card.run_id == id) else {
                continue;
            };
            let task = self
                .snapshot
                .run_info
                .get(id)
                .and_then(|info| info.task.as_deref())
                .unwrap_or("");
            let task = self
                .preamble
                .as_deref()
                .and_then(|p| {
                    task.strip_prefix(p.trim_end())
                        .and_then(|rest| rest.strip_prefix("\n\n"))
                })
                .unwrap_or(task);
            let text = format!(
                "{} {} {} {} {} {}",
                self.run_label(id),
                if card.state == State::Live {
                    "live"
                } else {
                    card.outcome.as_deref().unwrap_or(card.state.as_str())
                },
                card.provider.as_deref().unwrap_or("-"),
                card.model.as_deref().unwrap_or("default"),
                card.policy.as_deref().unwrap_or("-"),
                task.lines().next().unwrap_or("")
            );
            let rect = Rect::new(inner.x, inner.y + 2 + slot as u16, inner.width, 1);
            buf.set_line(
                rect.x,
                rect.y,
                &Line::styled(
                    text,
                    self.selected_style(Focus::Table, index == self.table_selected),
                ),
                rect.width,
            );
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

    pub(super) fn draw_lane(&mut self, lane: Lane, area: Rect, buf: &mut Buffer) {
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
                .map(|card| self.run_line(card, area.width.saturating_sub(2) as usize, false))
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
    pub(super) fn draw_agent_lanes(&mut self, area: Rect, buf: &mut Buffer) {
        let agents = self.agents();
        let count = agents.len() + 1;
        let rects = Split::vertical(vec![Constraint::Ratio(1, count as u32); count]).split(area);
        self.hits.agent_lanes = rects[..agents.len()].to_vec();
        for (i, agent) in agents.iter().enumerate() {
            let lines: Vec<String> = self
                .cards_of(agent)
                .map(|card| self.run_line(card, rects[i].width.saturating_sub(2) as usize, true))
                .collect();
            let title = format!(" {agent} {} ", lines.len());
            self.draw_cards(Focus::Agent(i), title, lines, rects[i], buf);
        }
        self.draw_lane(Lane::Waiting, rects[agents.len()], buf);
    }

    fn run_line(&self, card: &Card, width: usize, by_agent: bool) -> String {
        let info = self.snapshot.run_info.get(&card.run_id);
        let model = card
            .model
            .as_deref()
            .unwrap_or(if info.is_some() { "default" } else { "-" });
        let role = info.and_then(|info| info.role.as_deref());
        let elapsed = info
            .and_then(|info| info.elapsed_seconds)
            .map_or_else(|| "-".to_string(), |s| format!("{s}s"));
        let tokens = card
            .total_tokens
            .map_or_else(|| "-".to_string(), |n| n.to_string());
        let mut fields = vec![self.run_label(&card.run_id).to_string()];
        if !by_agent {
            fields.push(agent_of(card).to_string());
        }
        fields.extend([model.to_string(), self.said(card)]);
        if let Some(task) = self.snapshot.goal.as_ref().and_then(|goal| {
            goal.rows
                .iter()
                .find(|row| row.run_id.as_ref() == Some(&card.run_id))
        }) {
            fields.push(task.id.clone());
        }
        if let Some(role) = role {
            fields.push(role.to_string());
        }
        let identifying = fields.len();
        fields.extend([
            card.policy.as_deref().unwrap_or("-").to_string(),
            format!("{tokens}tok"),
            elapsed,
        ]);
        // Narrow panes drop elapsed, tokens, then policy; identifying values
        // are clipped only after that. The detail retains every full value.
        while fields.len() > identifying && fields.join(" ").chars().count() > width {
            fields.pop();
        }
        fields.join(" ")
    }

    pub(super) fn panel_lines(&self, goal: &Goal) -> Vec<Line<'static>> {
        let mut lines = Vec::new();
        for panel in &goal.panels {
            // Written before build 252: no verdict source and no reading orders. A
            // newer panel that split without a verdict has orders, and draws in full.
            if panel.source.is_none() && panel.seats.iter().all(|seat| seat.order.is_empty()) {
                lines.push(Line::raw(format!(
                    "panel verdict {} (recorded before seat details)",
                    panel.chosen.as_deref().unwrap_or("none")
                )));
                continue;
            }
            lines.push(Line::raw(format!(
                "panel verdict {} from {}",
                panel.chosen.as_deref().unwrap_or("none"),
                panel.source.as_deref().unwrap_or("unknown")
            )));
            let mut votes = Vec::new();
            for seat in &panel.seats {
                let card = self
                    .snapshot
                    .cards
                    .iter()
                    .find(|card| card.run_id == seat.run_id);
                let vote = card
                    .and_then(|card| card.result_text.as_deref())
                    .and_then(panel_vote);
                let vote = vote.and_then(|vote| {
                    if panel.labels.is_empty() || vote == "composed" {
                        Some(vote)
                    } else {
                        panel.labels.get(&vote).cloned()
                    }
                });
                if !seat.tie_break {
                    votes.push(vote.clone());
                }
                lines.push(Line::raw(format!(
                    "{} {} {} {} vote {} order {}",
                    if seat.tie_break { "tie-break" } else { "seat" },
                    card.map(agent_of).unwrap_or("?"),
                    card.and_then(|card| card.model.as_deref())
                        .unwrap_or(if card.is_some() { "default" } else { "?" }),
                    seat.role.as_deref().unwrap_or(""),
                    vote.as_deref().unwrap_or("unknown"),
                    seat.order.join(",")
                )));
            }
            let split = if votes.len() == 3 && votes.iter().all(Option::is_some) {
                if votes.windows(2).any(|pair| pair[0] != pair[1]) {
                    "split"
                } else {
                    "unanimous"
                }
            } else {
                "split unknown"
            };
            lines.push(Line::raw(split));
        }
        lines
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

    /// The shown goal's table, opened with `t`, build ticket 263.
    fn draw_goal_table_page(&mut self, area: Rect, buf: &mut Buffer) {
        self.hits.goal_table_page = area;
        self.goal_table_area = area;
        Clear.render(area, buf);
        let Some(goal) = self.snapshot.goal.as_ref() else {
            return;
        };
        let title = format!(" tasks of {} ", goal.id);
        let body = self.goal_table_lines();
        let block = Block::bordered()
            .title(title)
            .title_style(Style::default().add_modifier(Modifier::BOLD));
        Paragraph::new(body)
            .block(block)
            .wrap(Wrap { trim: false })
            .scroll((self.goal_table_scroll, 0))
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
    pub(super) fn detail_lines(&self, width: usize) -> Vec<Line<'static>> {
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
                let info = self.snapshot.run_info.get(id);
                let mut lines = Vec::new();
                if let Some(task) = info.and_then(|info| info.task.as_ref()) {
                    lines.push(label(
                        if info.is_some_and(|info| info.panel_seat)
                            || self.snapshot.goal.as_ref().is_some_and(|goal| {
                                goal.panels
                                    .iter()
                                    .flat_map(|panel| &panel.seats)
                                    .any(|seat| &seat.run_id == id)
                            })
                        {
                            "question"
                        } else {
                            "task text"
                        },
                        task.clone(),
                    ));
                }
                lines.push(label("run", card.run_id.clone()));
                if let Some(role) = info.and_then(|info| info.role.as_ref()) {
                    lines.push(label("role", role.clone()));
                }
                if let Some(seconds) = info.and_then(|info| info.elapsed_seconds) {
                    lines.push(label("elapsed", format!("{seconds}s")));
                }
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
                    label(
                        "model",
                        card.model.as_deref().unwrap_or("default").to_string(),
                    ),
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
