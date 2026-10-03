//! The cockpit with or without the chat: which side has the keys, where each
//! is drawn, and when the cockpit may quit.
//!
//! Without `--session` a [`Host`] is build 150's cockpit and hands every event
//! to [`App`]. With one, the chat takes the left panel, the cockpit's three
//! panels stack on the right, and `F2` moves the keys between them. `Ctrl-q`
//! quits from either side once the child has exited; while it is alive the
//! cockpit asks once, and `y` hangs the child up and quits when its record has
//! been read.

use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEventKind,
};
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Layout as Split, Position, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::Block;

use super::app::{App, Effect};
use super::chat::Chat;

/// Which half has the keyboard.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Chat,
    Cockpit,
}

pub struct Host {
    pub app: App,
    chat: Option<(String, Chat)>,
    pub side: Side,
    /// The question `Ctrl-q` asks while the child is alive, while it is up.
    pub asking: bool,
    /// `y` was typed: the child was hung up and the cockpit quits once its
    /// record is read.
    pub hanging_up: bool,
    message: Option<String>,
    chat_area: Rect,
}

impl Host {
    /// The cockpit alone, as build 150 made it.
    pub fn alone(app: App) -> Self {
        Self {
            app,
            chat: None,
            side: Side::Cockpit,
            asking: false,
            hanging_up: false,
            message: None,
            chat_area: Rect::default(),
        }
    }

    /// The cockpit with `chat` in the left panel, which starts with the keys.
    /// `title` is what the panel's border says.
    pub fn with_chat(mut app: App, title: String, chat: Chat) -> Self {
        app.stacked = true;
        app.chat_inside = true;
        Self {
            chat: Some((title, chat)),
            side: Side::Chat,
            ..Self::alone(app)
        }
    }

    pub fn chat(&self) -> Option<&Chat> {
        self.chat.as_ref().map(|(_, chat)| chat)
    }

    /// Where the chat, the cockpit and the status line go in `area`.
    pub fn areas(area: Rect) -> (Rect, Rect, Rect) {
        let [main, status] =
            Split::vertical([Constraint::Min(3), Constraint::Length(1)]).areas(area);
        let [chat, cockpit] =
            Split::horizontal([Constraint::Percentage(60), Constraint::Percentage(40)]).areas(main);
        (chat, cockpit, status)
    }

    /// The pty size the chat gets in `area`: the inside of its border, rows
    /// then columns.
    pub fn chat_size(area: Rect) -> (u16, u16) {
        let (chat, _, _) = Self::areas(area);
        let inner = Block::bordered().inner(chat);
        (inner.height, inner.width)
    }

    /// Reads what the child wrote, and once it names its run, points the
    /// dragon at that session. Returns whether the screen changed.
    pub fn pump(&mut self) -> bool {
        self.settle_stops();
        let Some((_, chat)) = &mut self.chat else {
            return false;
        };
        let changed = chat.pump();
        if let Some(run_id) = chat.run_id() {
            self.app.watch(run_id);
        }
        changed
    }

    /// Whether the loop should stop: after `y`, once the child's record is read.
    pub fn done(&self) -> bool {
        self.hanging_up && self.chat().is_none_or(Chat::finished)
    }

    pub fn handle(&mut self, event: &Event) -> Effect {
        if self.chat.is_none() {
            return self.app.handle(event);
        }
        if let Event::Key(key) = event {
            if key.kind == KeyEventKind::Release {
                return Effect::None;
            }
            if self.asking {
                return self.answer(key);
            }
            if key.code == KeyCode::Char('q') && key.modifiers.contains(KeyModifiers::CONTROL) {
                self.app.close_reveal();
                return self.quit();
            }
            if key.code == KeyCode::F(2) {
                self.app.close_reveal();
                self.side = match self.side {
                    Side::Chat => Side::Cockpit,
                    Side::Cockpit => Side::Chat,
                };
                return Effect::None;
            }
        }
        match self.side {
            Side::Chat => self.forward(event),
            Side::Cockpit => {
                if let Event::Mouse(mouse) = event
                    && matches!(mouse.kind, MouseEventKind::Down(_))
                    && self
                        .chat_area
                        .contains(Position::new(mouse.column, mouse.row))
                {
                    self.app.close_reveal();
                    self.side = Side::Chat;
                    return Effect::None;
                }
                match self.app.handle(event) {
                    Effect::Quit => self.quit(),
                    effect => effect,
                }
            }
        }
    }

    /// Everything goes to the child unchanged, except a click outside the
    /// panel, which hands the keys to the cockpit.
    fn forward(&mut self, event: &Event) -> Effect {
        let inner = Block::bordered().inner(self.chat_area);
        let Some((_, chat)) = &mut self.chat else {
            return Effect::None;
        };
        match event {
            Event::Key(key) => chat.send_key(key),
            Event::Paste(text) => chat.paste(text),
            Event::Mouse(mouse) => {
                let at = Position::new(mouse.column, mouse.row);
                if inner.contains(at) {
                    chat.send_mouse(mouse, at.x - inner.x, at.y - inner.y);
                } else if mouse.kind == MouseEventKind::Down(MouseButton::Left)
                    && !self.chat_area.contains(at)
                {
                    self.side = Side::Cockpit;
                    return self.app.handle(event);
                }
            }
            _ => {}
        }
        Effect::None
    }

    /// Takes the stops that reported since the last look and pastes the note
    /// each left, in order, build tickets 186 and 187. A failed stop leaves none.
    fn settle_stops(&mut self) {
        self.app.poll_stops();
        for note in self.app.take_drafts() {
            self.draft(&note);
        }
    }

    /// Pastes `note` into the chat's input and gives the chat the keys, build
    /// ticket 184. [`Chat::paste`] adds no `\r`, so the note waits there
    /// behind whatever the human had typed until they send it.
    fn draft(&mut self, note: &str) {
        let Some((_, chat)) = &mut self.chat else {
            return;
        };
        if chat.exited() {
            self.message = Some("the chat has exited, so the note was not drafted".to_string());
            return;
        }
        chat.paste(note);
        self.side = Side::Chat;
    }

    fn quit(&mut self) -> Effect {
        match self.chat() {
            None => Effect::Quit,
            Some(chat) if chat.finished() => Effect::Quit,
            // A second Ctrl-q while the hangup is pending leaves without waiting.
            Some(_) if self.hanging_up => Effect::Quit,
            Some(chat) if chat.exited() => {
                self.hanging_up = true;
                self.message = Some("the chat has exited; reading its record".to_string());
                Effect::None
            }
            Some(_) => {
                self.asking = true;
                Effect::None
            }
        }
    }

    fn answer(&mut self, key: &KeyEvent) -> Effect {
        self.asking = false;
        if key.code != KeyCode::Char('y') {
            self.message = Some("the chat keeps running".to_string());
            return Effect::None;
        }
        if let Some((_, chat)) = &mut self.chat {
            chat.hang_up();
        }
        self.hanging_up = true;
        self.message = Some(
            "hung up the chat; the cockpit quits once its record is read, Ctrl-q leaves now"
                .to_string(),
        );
        Effect::None
    }

    pub fn draw(&mut self, area: Rect, buf: &mut Buffer) {
        let Some((title, chat)) = &mut self.chat else {
            self.app.has_keys = true;
            self.app.draw(area, buf);
            return;
        };
        let (chat_area, cockpit_area, status) = Self::areas(area);
        self.chat_area = chat_area;
        let (rows, cols) = Self::chat_size(area);
        chat.resize(rows, cols);
        chat.draw(title, chat_area, buf, self.side == Side::Chat);
        self.app.has_keys = self.side == Side::Cockpit;
        self.app.draw(cockpit_area, buf);
        // The cockpit's draw may have taken a stop's answer; its note goes in
        // now rather than a loop later.
        self.settle_stops();
        self.draw_status(status, buf);
    }

    fn draw_status(&self, area: Rect, buf: &mut Buffer) {
        if area.is_empty() {
            return;
        }
        if self.asking {
            let ask = "the chat is still running: y hangs it up and quits once its record is \
                       read, any other key keeps it";
            let style = Style::default().fg(Color::Red).add_modifier(Modifier::BOLD);
            buf.set_line(area.x, area.y, &Line::styled(ask, style), area.width);
            return;
        }
        let side = match self.side {
            Side::Chat => "keys go to: chat",
            Side::Cockpit => "keys go to: cockpit",
        };
        let help = match self.side {
            Side::Chat => "F2 then ? for help",
            Side::Cockpit => "? help",
        };
        let mut text = format!("{side} | {help} | F2 switches | Ctrl-q quits");
        if let Some(message) = &self.message {
            text = format!("{message} | {text}");
        }
        buf.set_line(
            area.x,
            area.y,
            &Line::styled(text, Style::default().add_modifier(Modifier::BOLD)),
            area.width,
        );
    }
}
