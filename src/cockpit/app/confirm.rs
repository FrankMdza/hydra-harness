//! The confirmations a key opens before the cockpit writes or stops anything.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Clear, Paragraph, Widget, Wrap};

use crate::approve::Pending;
use crate::cockpit::snapshot::Step;

/// What the cockpit says where `hydra approve` would refuse by name.
pub(super) fn inside_seatbelt() -> String {
    format!(
        "{}: the cockpit is inside a seatbelt, which is where a model runs, and no model \
         approves a seat request (map ticket 82). Nothing was written.",
        crate::approve::INSIDE_SEATBELT
    )
}

/// The same for a Master of Puppets step, build ticket 182.
pub(super) fn step_inside_seatbelt() -> String {
    format!(
        "{}: the cockpit is inside a seatbelt, which is where a model runs, and no model \
         approves a Master of Puppets step. Nothing was run.",
        crate::approve::INSIDE_SEATBELT
    )
}

/// An argv on one line, each argument with a space or a quote in it quoted.
pub(super) fn shown_argv(argv: &[String]) -> String {
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
pub(super) fn draw_step_confirmation(step: &Step, area: Rect, buf: &mut Buffer) {
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

pub(super) fn draw_confirmation(request: &Pending, deny: bool, area: Rect, buf: &mut Buffer) {
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
pub(super) fn draw_review_confirmation(task: &str, run: &str, area: Rect, buf: &mut Buffer) {
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
pub(super) fn draw_stop_confirmation(id: &str, drafts: bool, area: Rect, buf: &mut Buffer) {
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
