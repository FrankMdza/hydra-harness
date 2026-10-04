//! The pure text helpers the cockpit draws with: wrapping, cutting to a
//! width, the stop note and the goal table's columns.

use std::path::Path;

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::cockpit::snapshot::{CallLine, Card};

/// One tool call on one row: name, what the result said, then the input, which
/// is what gets cut when the row is wider than the pane.
pub(super) fn call_line(
    call: &CallLine,
    input: &str,
    width: usize,
    row_style: Style,
) -> Line<'static> {
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

pub(super) fn wrap_call_reveal(call: &CallLine, width: usize) -> Vec<Line<'static>> {
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

pub(super) fn wrap_plain(text: &str, width: usize) -> Vec<String> {
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

/// How much of a seat's task the note quotes, in characters.
pub(super) const DRAFT_TASK: usize = 200;

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
pub(super) fn hunk_line(log: &Path) -> Option<String> {
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
pub(super) fn follow(selected: usize, offset: usize, rows: usize) -> usize {
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
pub(super) fn short_sha(sha: &str) -> &str {
    sha.get(..12).unwrap_or(sha)
}

/// A seat's answer can begin with a fenced object and have trailing prose,
/// the same framing accepted by Puppets' plan-answer reader.
pub(super) fn panel_vote(text: &str) -> Option<String> {
    let text = text.trim();
    let text = if let Some(fenced) = text.strip_prefix("```") {
        fenced.split_once('\n')?.1
    } else {
        text
    };
    let value = serde_json::Deserializer::from_str(text)
        .into_iter::<serde_json::Value>()
        .next()?
        .ok()?;
    value["chosen"]
        .as_str()
        .filter(|vote| !vote.is_empty())
        .map(str::to_string)
}

/// The first eight characters of an id, which is what `hydra-puppets board` shows.
pub(super) fn short(id: &str) -> &str {
    id.get(..8).unwrap_or(id)
}

/// A column of the task table, in the order it is drawn.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Column {
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
    pub(super) fn name(self) -> &'static str {
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
pub(super) fn columns_with_run(width: u16, run_width: usize) -> Vec<(Column, usize)> {
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
pub(super) fn row_text<'a>(
    columns: &[(Column, usize)],
    cell: impl Fn(Column) -> &'a str,
) -> String {
    columns
        .iter()
        .map(|&(column, width)| {
            let text: String = cell(column).chars().take(width).collect();
            format!("{text:<width$}")
        })
        .collect::<Vec<_>>()
        .join(" ")
}
