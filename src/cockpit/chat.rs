//! The chat inside the cockpit, map ticket 86's cut two and build ticket 151.
//!
//! `hydra cockpit --session <provider>` launches `hydra run --interactive` on a
//! pty this module owns and paints that pty's screen in the left panel. The
//! vendor keeps its rectangle (D3): the child's bytes go through `vt100`, the
//! parser `tui-term` draws, and nothing Hydra writes lands inside it. The
//! child is the same launch a human gets from `hy`, so the record, the gate,
//! the seatbelt and `seats.sock` are that command's and nothing here touches
//! them.
//!
//! **What `tui-term` does not do is ours.** It draws a `vt100` screen and
//! nothing else, so turning a crossterm key or mouse event back into the bytes
//! a terminal would have sent is [`key_bytes`] and [`mouse_bytes`] below, and
//! the two queries a child may block on, the cursor position and the device
//! attributes, are answered by [`Replies`]. Every other query goes unanswered,
//! as `vt100` leaves it.

use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::time::Instant;

use anyhow::{Context, Result};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use portable_pty::{CommandBuilder, ExitStatus, MasterPty, PtySize, native_pty_system};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Widget};
use tui_term::vt100::{self, MouseProtocolEncoding, MouseProtocolMode};
use tui_term::widget::{Cursor, PseudoTerminal};

use crate::live::State;
use crate::paths::Layout;
use crate::policy::PolicySet;

/// How much of what the child printed is kept while looking for its run id.
/// `hydra run --interactive` names the run on its first line of stderr.
const HEARD_CAP: usize = 4096;

/// What `hydra run --interactive` prints before its run id.
const RUN_LINE: &str = "hydra: run ";

/// What to launch and where its record will be.
pub struct Launch {
    pub program: String,
    pub args: Vec<String>,
    pub cwd: PathBuf,
    /// The child's whole environment. The cockpit hands `hydra run` its own,
    /// as a shell would, and `hydra run` builds the provider's allowlist from
    /// it the way it does under `hy`.
    pub env: Vec<(String, String)>,
    pub layout: Layout,
    pub policies: std::result::Result<PolicySet, String>,
    /// The home whose `claude` transcripts a dead run is harvested from, or
    /// `None` to leave the harvest to `hydra status`. The suite passes `None`.
    pub harvest_home: Option<PathBuf>,
}

impl Launch {
    /// `hydra run --interactive` for `provider`, from this binary, in this
    /// directory and with this environment, `TERM` set to what `vt100` draws.
    pub fn hydra_run(layout: &Layout, provider: &str, model: Option<&str>) -> Result<Self> {
        let program = std::env::current_exe().context("finding this binary")?;
        let mut args = vec![
            "run".to_string(),
            "--interactive".to_string(),
            "--provider".to_string(),
            provider.to_string(),
        ];
        if let Some(model) = model {
            args.extend(["--model".to_string(), model.to_string()]);
        }
        let mut env: Vec<(String, String)> = std::env::vars()
            .filter(|(name, _)| name != "TERM")
            .collect();
        env.push(("TERM".to_string(), "xterm-256color".to_string()));
        Ok(Self {
            program: program.to_string_lossy().into_owned(),
            args,
            cwd: std::env::current_dir().context("reading the working directory")?,
            env,
            layout: Layout::at(layout.root()),
            policies: PolicySet::discover().map_err(|err| format!("{err:#}")),
            harvest_home: crate::env::real_home().ok(),
        })
    }
}

/// Answers to the queries a child can block on. `vt100` hands every CSI it
/// does not implement to [`vt100::Callbacks::unhandled_csi`], and these are
/// written back to the child after each read.
#[derive(Default)]
pub struct Replies(Vec<u8>);

impl vt100::Callbacks for Replies {
    fn unhandled_csi(
        &mut self,
        screen: &mut vt100::Screen,
        i1: Option<u8>,
        _i2: Option<u8>,
        params: &[&[u16]],
        c: char,
    ) {
        let first = params.first().and_then(|p| p.first()).copied();
        match (i1, c, first) {
            // Cursor position report, 1-based.
            (None, 'n', Some(6)) => {
                let (row, col) = screen.cursor_position();
                self.0
                    .extend(format!("\x1b[{};{}R", row + 1, col + 1).as_bytes());
            }
            // Device status: fine.
            (None, 'n', Some(5)) => self.0.extend(b"\x1b[0n"),
            // Primary device attributes: a VT100 with advanced video.
            (None, 'c', None | Some(0)) => self.0.extend(b"\x1b[?1;2c"),
            _ => {}
        }
    }
}

/// The child on its pty, its screen, and what it said about its run.
pub struct Chat {
    parser: vt100::Parser<Replies>,
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    child: Box<dyn portable_pty::Child + Send + Sync>,
    pid: Option<u32>,
    bytes: Receiver<Vec<u8>>,
    reader_done: bool,
    heard: String,
    run_id: Option<String>,
    exited: Option<(Instant, ExitStatus)>,
    /// The line under the panel once the child is gone and its record read.
    ended: Option<String>,
    layout: Layout,
    policies: std::result::Result<PolicySet, String>,
    harvest_home: Option<PathBuf>,
}

impl Chat {
    /// Spawns the child on a pty of `rows` by `cols`, which is the inside of
    /// the panel it will be drawn in.
    pub fn spawn(launch: Launch, rows: u16, cols: u16) -> Result<Self> {
        let (rows, cols) = (rows.max(1), cols.max(1));
        let pair = native_pty_system()
            .openpty(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .context("opening a pty for the chat")?;
        let mut cmd = CommandBuilder::new(&launch.program);
        cmd.args(&launch.args);
        cmd.cwd(&launch.cwd);
        cmd.env_clear();
        for (name, value) in &launch.env {
            cmd.env(name, value);
        }
        let child = pair
            .slave
            .spawn_command(cmd)
            .with_context(|| format!("spawning {}", launch.program))?;
        // The child holds the only slave from here, so the reader sees EOF
        // when it exits, as `pty::spawn` arranges for a recorded run.
        drop(pair.slave);
        let pid = child.process_id();
        let mut reader = pair
            .master
            .try_clone_reader()
            .context("reading the chat's pty")?;
        let writer = pair
            .master
            .take_writer()
            .context("writing to the chat's pty")?;
        let (sender, bytes) = mpsc::channel();
        std::thread::Builder::new()
            .name("cockpit-chat".to_string())
            .spawn(move || {
                let mut chunk = [0u8; 65536];
                loop {
                    match reader.read(&mut chunk) {
                        Ok(0) | Err(_) => break,
                        Ok(n) => {
                            if sender.send(chunk[..n].to_vec()).is_err() {
                                break;
                            }
                        }
                    }
                }
            })
            .context("starting the chat's reader")?;
        Ok(Self {
            parser: vt100::Parser::new_with_callbacks(rows, cols, 0, Replies::default()),
            master: pair.master,
            writer,
            child,
            pid,
            bytes,
            reader_done: false,
            heard: String::new(),
            run_id: None,
            exited: None,
            ended: None,
            layout: launch.layout,
            policies: launch.policies,
            harvest_home: launch.harvest_home,
        })
    }

    /// Feeds what the child wrote since the last call to the screen, answers
    /// its queries, and notices its exit. Returns whether anything changed.
    pub fn pump(&mut self) -> bool {
        let mut changed = false;
        loop {
            match self.bytes.try_recv() {
                Ok(chunk) => {
                    self.parser.process(&chunk);
                    self.listen(&chunk);
                    changed = true;
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    self.reader_done = true;
                    break;
                }
            }
        }
        let replies = std::mem::take(&mut self.parser.callbacks_mut().0);
        if !replies.is_empty() {
            let _ = self.writer.write_all(&replies);
            let _ = self.writer.flush();
        }
        if self.ended.is_some() {
            return changed;
        }
        if self.exited.is_none()
            && let Ok(Some(status)) = self.child.try_wait()
        {
            self.exited = Some((Instant::now(), status));
        }
        if let Some((since, status)) = &self.exited
            && (self.reader_done || since.elapsed() >= crate::pty::PTY_EOF_DEADLINE)
        {
            let status = status.clone();
            self.ended = Some(self.settle(&status));
            changed = true;
        }
        changed
    }

    /// Looks for `hydra: run <uuid>` in what the child printed, until found.
    fn listen(&mut self, chunk: &[u8]) {
        if self.run_id.is_some() {
            return;
        }
        self.heard.push_str(&String::from_utf8_lossy(chunk));
        if let Some(at) = self.heard.find(RUN_LINE) {
            let rest = &self.heard[at + RUN_LINE.len()..];
            if let Some(id) = rest.get(..36)
                && uuid::Uuid::parse_str(id).is_ok()
            {
                self.run_id = Some(id.to_string());
                self.heard.clear();
                return;
            }
            if rest.len() >= 36 {
                // Not a run id after all; look past this one next time.
                self.heard.drain(..at + RUN_LINE.len());
            }
            return;
        }
        if self.heard.len() > HEARD_CAP {
            let cut = self.heard.len() - RUN_LINE.len();
            let cut = (0..=cut)
                .rev()
                .find(|i| self.heard.is_char_boundary(*i))
                .unwrap_or(0);
            self.heard.drain(..cut);
        }
    }

    /// The line the panel keeps once the child is gone: its run id and the
    /// outcome its record reads back to.
    fn settle(&mut self, status: &ExitStatus) -> String {
        let how = match status.signal() {
            Some(signal) => format!("died on {signal}"),
            None => format!("exited {}", status.exit_code()),
        };
        let Some(run_id) = self.run_id.clone() else {
            return format!("the chat {how} before it named a run");
        };
        format!("run {run_id}: {}; hydra {how}", self.outcome(&run_id))
    }

    fn outcome(&self, run_id: &str) -> String {
        let path = self.layout.run_file(run_id);
        if !path.is_file() {
            return format!("no record at {}", path.display());
        }
        let mut view = match crate::live::read_view(&path) {
            Ok(view) => view,
            Err(err) => return format!("record unreadable: {err:#}"),
        };
        // A hangup kills `hydra run` before its own harvest, so the cockpit
        // does what `hydra status` would do on the same dead run.
        if let Some(home) = &self.harvest_home
            && crate::status::is_abandoned_interactive_claude(&view)
            && crate::interactive::harvest_in(&self.layout, run_id, None, home).is_ok()
            && let Ok(again) = crate::live::read_view(&path)
        {
            view = again;
        }
        match view.state {
            State::Live => "still live".to_string(),
            State::Abandoned => {
                format!("no run_end yet, `hydra status {run_id}` harvests it")
            }
            State::Ended => {
                let policies = match &self.policies {
                    Ok(policies) => policies,
                    Err(err) => return format!("ended, no policies to read it with: {err}"),
                };
                let read = std::fs::read_to_string(&path)
                    .map_err(anyhow::Error::from)
                    .and_then(|text| crate::normalize::read_with(&text, policies, &self.layout));
                match read.map(|run| crate::outcome::derive(&run)) {
                    Ok(Some(derived)) => derived.outcome.as_str().to_string(),
                    Ok(None) => "ended, no outcome derived".to_string(),
                    Err(err) => format!("ended, would not read back: {err:#}"),
                }
            }
        }
    }

    /// Whether the child has exited and its record has been read.
    pub fn finished(&self) -> bool {
        self.ended.is_some()
    }

    pub fn exited(&self) -> bool {
        self.exited.is_some()
    }

    pub fn run_id(&self) -> Option<&str> {
        self.run_id.as_deref()
    }

    /// The line under the panel, once there is one.
    pub fn ended_line(&self) -> Option<&str> {
        self.ended.as_deref()
    }

    /// The pty's size, rows then columns.
    pub fn size(&self) -> (u16, u16) {
        self.parser.screen().size()
    }

    /// Gives the pty and the screen a new size. The kernel sends the child
    /// `SIGWINCH`.
    pub fn resize(&mut self, rows: u16, cols: u16) {
        let (rows, cols) = (rows.max(1), cols.max(1));
        if self.size() == (rows, cols) {
            return;
        }
        self.parser.screen_mut().set_size(rows, cols);
        if self.exited.is_none() {
            let _ = self.master.resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            });
        }
    }

    fn send(&mut self, bytes: &[u8]) {
        if bytes.is_empty() || self.exited.is_some() {
            return;
        }
        let _ = self.writer.write_all(bytes);
        let _ = self.writer.flush();
    }

    /// Whether `key` is `Ctrl-Z`, which would suspend the chat's foreground
    /// process with no shell behind the pty to bring it back with `fg`.
    pub fn is_suspend_key(key: &KeyEvent) -> bool {
        key.modifiers.contains(KeyModifiers::CONTROL)
            && matches!(key.code, KeyCode::Char('z') | KeyCode::Char('Z'))
    }

    /// Writes the bytes for `key` to the pty. Returns false when the key is
    /// withheld, as with [`is_suspend_key`].
    pub fn send_key(&mut self, key: &KeyEvent) -> bool {
        if Self::is_suspend_key(key) {
            return false;
        }
        let bytes = key_bytes(key, self.parser.screen().application_cursor());
        self.send(&bytes);
        true
    }

    /// A mouse event at `column`, `row` inside the panel, sent only when the
    /// child asked for mouse reports.
    pub fn send_mouse(&mut self, mouse: &MouseEvent, column: u16, row: u16) {
        let screen = self.parser.screen();
        let bytes = mouse_bytes(
            mouse,
            column,
            row,
            screen.mouse_protocol_mode(),
            screen.mouse_protocol_encoding(),
        );
        self.send(&bytes);
    }

    pub fn paste(&mut self, text: &str) {
        if self.parser.screen().bracketed_paste() {
            let mut bytes = b"\x1b[200~".to_vec();
            bytes.extend(text.as_bytes());
            bytes.extend(b"\x1b[201~");
            self.send(&bytes);
        } else {
            self.send(text.as_bytes());
        }
    }

    /// Sends the child's process group a hangup, which is what closing a
    /// terminal window does to the `hydra run` and the `claude` inside it.
    pub fn hang_up(&mut self) {
        let Some(pid) = self.pid.and_then(|pid| i32::try_from(pid).ok()) else {
            let _ = self.child.kill();
            return;
        };
        // SAFETY: `kill` takes plain integers and touches no memory. The pty
        // made the child a session leader, so `-pid` is its process group.
        let sent = unsafe { libc::kill(-pid, libc::SIGHUP) };
        if sent != 0 {
            // SAFETY: as above, on the one pid.
            unsafe { libc::kill(pid, libc::SIGHUP) };
        }
    }

    /// Draws the child's screen inside a bordered panel, with the ended line
    /// on the bottom border once there is one.
    pub fn draw(&self, title: &str, area: Rect, buf: &mut Buffer, focused: bool) {
        let border = if focused {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        };
        let mut block = Block::bordered()
            .border_style(border)
            .title(format!(" {title} "));
        if let Some(line) = &self.ended {
            block = block.title_bottom(Line::styled(
                format!(" {line} "),
                Style::default().fg(Color::Cyan),
            ));
        }
        PseudoTerminal::new(self.parser.screen())
            .block(block)
            .cursor(Cursor::default().visibility(focused && self.ended.is_none()))
            .render(area, buf);
    }
}

/// The xterm modifier parameter: 1, plus 1 for shift, 2 for alt, 4 for control.
fn modifier_param(modifiers: KeyModifiers) -> u8 {
    1 + u8::from(modifiers.contains(KeyModifiers::SHIFT))
        + 2 * u8::from(modifiers.contains(KeyModifiers::ALT))
        + 4 * u8::from(modifiers.contains(KeyModifiers::CONTROL))
}

/// The bytes an xterm sends for `key`. `application_cursor` is DECCKM, which a
/// child sets to get `ESC O A` rather than `ESC [ A` for the arrows.
pub fn key_bytes(key: &KeyEvent, application_cursor: bool) -> Vec<u8> {
    let alt = key.modifiers.contains(KeyModifiers::ALT);
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    let m = modifier_param(key.modifiers);
    let with_alt = |bytes: Vec<u8>| {
        if alt {
            let mut out = vec![0x1b];
            out.extend(bytes);
            out
        } else {
            bytes
        }
    };
    let cursor = |letter: char| -> Vec<u8> {
        if m > 1 {
            format!("\x1b[1;{m}{letter}").into_bytes()
        } else if application_cursor {
            format!("\x1bO{letter}").into_bytes()
        } else {
            format!("\x1b[{letter}").into_bytes()
        }
    };
    let tilde = |n: u8| -> Vec<u8> {
        if m > 1 {
            format!("\x1b[{n};{m}~").into_bytes()
        } else {
            format!("\x1b[{n}~").into_bytes()
        }
    };
    match key.code {
        KeyCode::Char(c) if ctrl => {
            let byte = match c {
                'a'..='z' => Some(c as u8 - b'a' + 1),
                'A'..='Z' => Some(c as u8 - b'A' + 1),
                '@' | ' ' => Some(0),
                '[' | '\\' | ']' | '^' | '_' => Some(c as u8 & 0x1f),
                '?' => Some(0x7f),
                _ => None,
            };
            match byte {
                Some(byte) => with_alt(vec![byte]),
                None => with_alt(c.to_string().into_bytes()),
            }
        }
        KeyCode::Char(c) => with_alt(c.to_string().into_bytes()),
        KeyCode::Enter => with_alt(b"\r".to_vec()),
        KeyCode::Tab => with_alt(b"\t".to_vec()),
        KeyCode::BackTab => b"\x1b[Z".to_vec(),
        KeyCode::Backspace if ctrl => with_alt(vec![0x08]),
        KeyCode::Backspace => with_alt(vec![0x7f]),
        KeyCode::Esc => with_alt(vec![0x1b]),
        KeyCode::Up => cursor('A'),
        KeyCode::Down => cursor('B'),
        KeyCode::Right => cursor('C'),
        KeyCode::Left => cursor('D'),
        KeyCode::Home => cursor('H'),
        KeyCode::End => cursor('F'),
        KeyCode::Insert => tilde(2),
        KeyCode::Delete => tilde(3),
        KeyCode::PageUp => tilde(5),
        KeyCode::PageDown => tilde(6),
        KeyCode::F(n @ 1..=4) => {
            let letter = (b'P' + n - 1) as char;
            if m > 1 {
                format!("\x1b[1;{m}{letter}").into_bytes()
            } else {
                format!("\x1bO{letter}").into_bytes()
            }
        }
        KeyCode::F(n @ 5..=12) => tilde([15, 17, 18, 19, 20, 21, 23, 24][usize::from(n - 5)]),
        KeyCode::Null => vec![0],
        _ => Vec::new(),
    }
}

/// The bytes an xterm sends for `mouse` at `column`, `row` of the child's
/// screen, 0-based, under the mode and encoding the child asked for. Empty
/// when the child did not ask for this kind of event.
pub fn mouse_bytes(
    mouse: &MouseEvent,
    column: u16,
    row: u16,
    mode: MouseProtocolMode,
    encoding: MouseProtocolEncoding,
) -> Vec<u8> {
    use MouseProtocolMode as Mode;
    let button = |b: MouseButton| match b {
        MouseButton::Left => 0u16,
        MouseButton::Middle => 1,
        MouseButton::Right => 2,
    };
    let (code, release, wanted) = match mouse.kind {
        MouseEventKind::Down(b) => (button(b), false, mode != Mode::None),
        MouseEventKind::Up(b) => (
            button(b),
            true,
            matches!(
                mode,
                Mode::PressRelease | Mode::ButtonMotion | Mode::AnyMotion
            ),
        ),
        MouseEventKind::Drag(b) => (
            button(b) + 32,
            false,
            matches!(mode, Mode::ButtonMotion | Mode::AnyMotion),
        ),
        MouseEventKind::Moved => (3 + 32, false, mode == Mode::AnyMotion),
        MouseEventKind::ScrollUp => (64, false, mode != Mode::None),
        MouseEventKind::ScrollDown => (65, false, mode != Mode::None),
        MouseEventKind::ScrollLeft => (66, false, mode != Mode::None),
        MouseEventKind::ScrollRight => (67, false, mode != Mode::None),
    };
    if !wanted {
        return Vec::new();
    }
    let mut code = code;
    if mouse.modifiers.contains(KeyModifiers::SHIFT) {
        code += 4;
    }
    if mouse.modifiers.contains(KeyModifiers::ALT) {
        code += 8;
    }
    if mouse.modifiers.contains(KeyModifiers::CONTROL) {
        code += 16;
    }
    let (x, y) = (column + 1, row + 1);
    match encoding {
        MouseProtocolEncoding::Sgr => {
            let end = if release { 'm' } else { 'M' };
            format!("\x1b[<{code};{x};{y}{end}").into_bytes()
        }
        MouseProtocolEncoding::Default | MouseProtocolEncoding::Utf8 => {
            // The legacy encodings have no release button of their own.
            let code = if release { 3 + (code & !3) } else { code };
            let mut out = b"\x1b[M".to_vec();
            for value in [code, x, y] {
                let value = u32::from(value) + 32;
                match (encoding, u8::try_from(value)) {
                    (MouseProtocolEncoding::Default, Ok(byte)) => out.push(byte),
                    (MouseProtocolEncoding::Default, Err(_)) => return Vec::new(),
                    _ => match char::from_u32(value) {
                        Some(c) => out.extend(c.to_string().as_bytes()),
                        None => return Vec::new(),
                    },
                }
            }
            out
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent::new(code, modifiers)
    }

    #[test]
    fn keys_encode_as_an_xterm_sends_them() {
        let none = KeyModifiers::NONE;
        assert_eq!(key_bytes(&key(KeyCode::Char('q'), none), false), b"q");
        assert_eq!(
            key_bytes(&key(KeyCode::Char('c'), KeyModifiers::CONTROL), false),
            [3]
        );
        assert_eq!(key_bytes(&key(KeyCode::Enter, none), false), b"\r");
        assert_eq!(key_bytes(&key(KeyCode::Up, none), false), b"\x1b[A");
        assert_eq!(key_bytes(&key(KeyCode::Up, none), true), b"\x1bOA");
        assert_eq!(
            key_bytes(&key(KeyCode::Left, KeyModifiers::SHIFT), true),
            b"\x1b[1;2D"
        );
        assert_eq!(
            key_bytes(&key(KeyCode::Char('b'), KeyModifiers::ALT), false),
            b"\x1bb"
        );
        assert_eq!(key_bytes(&key(KeyCode::F(5), none), false), b"\x1b[15~");
        assert_eq!(key_bytes(&key(KeyCode::Backspace, none), false), [0x7f]);
        assert_eq!(
            key_bytes(&key(KeyCode::Char('z'), KeyModifiers::CONTROL), false),
            [0x1a]
        );
    }

    #[test]
    fn suspend_key_is_ctrl_z_alone() {
        assert!(Chat::is_suspend_key(&key(
            KeyCode::Char('z'),
            KeyModifiers::CONTROL
        )));
        assert!(Chat::is_suspend_key(&key(
            KeyCode::Char('Z'),
            KeyModifiers::CONTROL
        )));
        assert!(!Chat::is_suspend_key(&key(
            KeyCode::Char('z'),
            KeyModifiers::NONE
        )));
        assert!(!Chat::is_suspend_key(&key(
            KeyCode::Char('c'),
            KeyModifiers::CONTROL
        )));
    }

    #[test]
    fn a_click_encodes_only_when_the_child_asked_for_it() {
        let click = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 0,
            row: 0,
            modifiers: KeyModifiers::NONE,
        };
        assert!(
            mouse_bytes(
                &click,
                4,
                2,
                MouseProtocolMode::None,
                MouseProtocolEncoding::Sgr
            )
            .is_empty()
        );
        assert_eq!(
            mouse_bytes(
                &click,
                4,
                2,
                MouseProtocolMode::PressRelease,
                MouseProtocolEncoding::Sgr
            ),
            b"\x1b[<0;5;3M"
        );
        assert_eq!(
            mouse_bytes(
                &click,
                4,
                2,
                MouseProtocolMode::Press,
                MouseProtocolEncoding::Default
            ),
            [0x1b, b'[', b'M', 32, 37, 35]
        );
    }

    #[test]
    fn a_cursor_query_is_answered() {
        let mut parser = vt100::Parser::new_with_callbacks(5, 20, 0, Replies::default());
        parser.process(b"ab\x1b[6n\x1b[c");
        assert_eq!(parser.callbacks().0, b"\x1b[1;3R\x1b[?1;2c");
    }
}
