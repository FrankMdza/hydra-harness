//! `hydra cockpit`, map ticket 86's cut one and build ticket 150.
//!
//! A full-screen view a human runs in a terminal of their own, beside the chat of
//! an orchestrating session: the task table of one goal from its Master of
//! Puppets log, a kanban of runs by state, and the detail of one run. It reads
//! files and draws. Without `--session` it launches no provider itself; it
//! edits no policy and no config, and needs nothing running to exist (D10).
//!
//! **`v` then `y` starts `puppets.py review`**, build ticket 183, which
//! launches three judge runs. The confirmation says they count against quota.
//!
//! **Its one write is the approval of a waiting seat request**, typed by the
//! human in the cockpit, through [`crate::approve::approve`], the function
//! `hydra approve <id>` calls. Like that command it refuses inside a seatbelt
//! and says so on screen.
//!
//! **With `--session` the chat is inside**, map ticket 86's cut two and build
//! ticket 151: [`chat`] launches `hydra run --interactive` on a pty the cockpit
//! owns and paints it in the left panel, and [`host`] decides which side has
//! the keys. The child is the launch `hy` makes, so its record, gate, seatbelt
//! and `seats.sock` are that command's. The one record the cockpit may append
//! to is that run's, harvested when a hangup killed `hydra run` first.
//!
//! [`app`] holds the state, the event handling and the drawing, with no
//! terminal, so the suite drives them over a `Buffer`. [`snapshot`] is what one
//! read of the disk returns, done on a thread so a slow read never holds a key
//! press.
//!
//! **`p` shows the policy set and `e` opens one file in the human's editor**,
//! build ticket 171. The cockpit leaves the screen, runs `$EDITOR` and waits,
//! then reloads the set and shows its new sha or the loader's refusal. The
//! editor writes the file; the cockpit does not. [`goal`] folds a goal log into the table. [`mascot`] is the dragon in the header, drawn
//! from frames the Python mascot wrote.

pub mod app;
pub mod chat;
pub mod goal;
pub mod host;
pub mod mascot;
pub mod snapshot;
pub mod window;

use std::io::Stdout;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use crossterm::event::{
    DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
};
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::Rect;

use crate::paths::Layout;
use app::{App, Effect};
use chat::{Chat, Launch};
use host::Host;
use snapshot::Reader;

/// How long the input loop waits for an event before it looks for a new
/// snapshot. Short enough that a refresh shows without a key press.
const POLL: Duration = Duration::from_millis(100);

/// The same wait with a chat in the left panel, short enough that what the
/// child prints shows as it arrives rather than a tenth of a second later.
const CHAT_POLL: Duration = Duration::from_millis(15);

/// The chat `--session` asks for: `hydra run --interactive` with these flags.
pub struct Session {
    pub provider: String,
    pub model: Option<String>,
    pub policy: Option<String>,
}

/// Where the goal logs live: `HYDRA_PUPPETS_HOME` when set, as `puppets.py`
/// reads it, and `puppets/` under the root otherwise.
pub fn puppets_home(layout: &Layout) -> PathBuf {
    match std::env::var_os("HYDRA_PUPPETS_HOME") {
        Some(home) if !home.is_empty() => PathBuf::from(home),
        _ => layout.puppets(),
    }
}

/// A goal id a human typed, checked before it is joined onto a path.
pub fn check_goal_id(id: &str) -> Result<()> {
    if !plain_name(id) {
        bail!("{id:?} is not a goal id");
    }
    Ok(())
}

/// Whether `id` is safe to join onto a directory as one file name.
pub(crate) fn plain_name(id: &str) -> bool {
    !id.is_empty()
        && !id.starts_with('.')
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
}

/// Runs the cockpit until `q`, or with a session until `Ctrl-q`.
pub fn run(
    layout: &Layout,
    goal: Option<String>,
    interval: Duration,
    with_mascot: bool,
    session: Option<Session>,
) -> Result<()> {
    if let Some(id) = &goal {
        check_goal_id(id)?;
    }
    if interval.is_zero() {
        bail!("--interval must be at least one second");
    }

    let reader = Reader::new(
        Layout::at(layout.root()),
        crate::policy::PolicySet::discover(),
        puppets_home(layout),
        goal,
    )
    .with_vendors(crate::paths::hydra_home().join("integrations/puppets"));
    let (asks, asked) = mpsc::channel::<Option<String>>();
    let (snapshots, snapshot) = mpsc::channel();
    std::thread::Builder::new()
        .name("cockpit-reader".to_string())
        .spawn(move || reader.serve(interval, asked, snapshots))
        .context("starting the reader")?;

    let mut app = App::new(
        Layout::at(layout.root()),
        crate::sandbox::inside_seatbelt(),
        interval,
    );

    app.set_policies(crate::paths::hydra_home().join("policies"));
    app.set_reviewer(
        crate::paths::hydra_home().join("integrations/puppets/puppets.py"),
        puppets_home(layout),
    );

    if with_mascot {
        match mascot::Frames::load(&mascot::default_dir()) {
            Ok(frames) => app.set_mascot(Some(mascot::Mascot::new(frames))),
            Err(err) => app.message = Some(format!("no mascot: {err:#}")),
        }
    }

    let mut screen = Screen::enter()?;
    let (mut host, poll) = match session {
        None => (Host::alone(app), POLL),
        Some(session) => {
            let area = screen
                .terminal
                .size()
                .context("reading the terminal size")?;
            let (rows, cols) = Host::chat_size(Rect::new(0, 0, area.width, area.height));
            let launch = Launch::hydra_run(
                layout,
                &session.provider,
                session.model.as_deref(),
                session.policy.as_deref(),
            )?;
            let chat = Chat::spawn(launch, rows, cols)?;
            let title = match &session.policy {
                Some(policy) => format!("{} under {policy}", session.provider),
                None => session.provider.clone(),
            };
            (Host::with_chat(app, title, chat), CHAT_POLL)
        }
    };
    loop {
        while let Ok(fresh) = snapshot.try_recv() {
            host.app.set_snapshot(fresh);
        }
        host.pump();
        if host.done() {
            break;
        }
        screen
            .terminal
            .draw(|frame| {
                let area = frame.area();
                host.draw(area, frame.buffer_mut());
            })
            .context("drawing")?;
        if !crossterm::event::poll(poll).context("waiting for input")? {
            continue;
        }
        let event = crossterm::event::read().context("reading input")?;
        match host.handle(&event) {
            Effect::Quit => break,
            Effect::Edit(file) => {
                // The chat's reader thread keeps draining its pty while the
                // editor has the terminal, so a `--session` child runs on.
                screen.suspend();
                let ran = run_editor(&editor(), &file);
                screen.resume()?;
                host.app.edited(&file, ran);
            }
            Effect::Refresh => {
                // The reader is gone only if it panicked, and then the screen
                // simply stops refreshing; quitting still works.
                let _ = asks.send(host.app.open_run_id());
            }
            Effect::None => {}
        }
    }
    Ok(())
}

/// `$EDITOR`, or `vi` when it is unset or empty.
pub fn editor() -> String {
    std::env::var("EDITOR")
        .ok()
        .filter(|e| !e.trim().is_empty())
        .unwrap_or_else(|| "vi".to_string())
}

/// Runs `editor` on `file` and waits for it. `editor` goes through `sh` the
/// way git runs it, so `EDITOR="code -w"` works; the file is an argument and
/// never part of the script.
pub fn run_editor(editor: &str, file: &Path) -> Result<()> {
    let status = std::process::Command::new("/bin/sh")
        .arg("-c")
        .arg(format!("{editor} \"$1\""))
        .arg("sh")
        .arg(file)
        .status()
        .with_context(|| format!("starting {editor}"))?;
    if !status.success() {
        bail!("{editor} exited with {status}");
    }
    Ok(())
}

/// The terminal in raw mode, the alternate screen and mouse capture, undone
/// on drop and on panic.
struct Screen {
    terminal: Terminal<CrosstermBackend<Stdout>>,
}

impl Screen {
    fn enter() -> Result<Self> {
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            restore();
            previous(info);
        }));
        enable_raw_mode().context("entering raw mode")?;
        let mut stdout = std::io::stdout();
        if let Err(err) = crossterm::execute!(
            stdout,
            EnterAlternateScreen,
            EnableMouseCapture,
            EnableBracketedPaste
        ) {
            restore();
            return Err(err).context("opening the screen");
        }
        match Terminal::new(CrosstermBackend::new(stdout)) {
            Ok(terminal) => Ok(Self { terminal }),
            Err(err) => {
                restore();
                Err(err).context("opening the screen")
            }
        }
    }
}

impl Screen {
    /// Gives the terminal to another program, as the panic hook does.
    fn suspend(&mut self) {
        restore();
    }

    /// Takes it back and forces a full repaint of what the editor drew over.
    fn resume(&mut self) -> Result<()> {
        enable_raw_mode().context("entering raw mode")?;
        crossterm::execute!(
            std::io::stdout(),
            EnterAlternateScreen,
            EnableMouseCapture,
            EnableBracketedPaste
        )
        .context("opening the screen")?;
        self.terminal.clear().context("clearing the screen")
    }
}

impl Drop for Screen {
    fn drop(&mut self) {
        restore();
        let _ = self.terminal.show_cursor();
    }
}

/// Gives the terminal back. Safe to call twice, which a panic inside the loop
/// does: once from the hook and once from the drop.
fn restore() {
    let _ = crossterm::execute!(
        std::io::stdout(),
        DisableBracketedPaste,
        DisableMouseCapture,
        LeaveAlternateScreen
    );
    let _ = disable_raw_mode();
    let _ = crossterm::execute!(std::io::stdout(), crossterm::cursor::Show);
}
