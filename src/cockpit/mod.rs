//! `hydra cockpit`, map ticket 86's cut one and build ticket 150.
//!
//! A full-screen view a human runs in a terminal of their own, beside the chat of
//! an orchestrating session: the task table of one goal from its Master of
//! Puppets log, a kanban of runs by state, and the detail of one run. It reads
//! files and draws. It launches no provider, edits no policy and no config, and
//! needs nothing running to exist (D10).
//!
//! **Its one write is the approval of a waiting seat request**, typed by the
//! human in the cockpit, through [`crate::approve::approve`], the function
//! `hydra approve <id>` calls. Like that command it refuses inside a seatbelt
//! and says so on screen.
//!
//! [`app`] holds the state, the event handling and the drawing, with no
//! terminal, so the suite drives them over a `Buffer`. [`snapshot`] is what one
//! read of the disk returns, done on a thread so a slow read never holds a key
//! press. [`goal`] folds a goal log into the table. [`mascot`] is the dragon in the header, drawn
//! from frames the Python mascot wrote.

pub mod app;
pub mod goal;
pub mod mascot;
pub mod snapshot;

use std::io::Stdout;
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

use crate::paths::Layout;
use app::{App, Effect};
use snapshot::Reader;

/// How long the input loop waits for an event before it looks for a new
/// snapshot. Short enough that a refresh shows without a key press.
const POLL: Duration = Duration::from_millis(100);

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
    let fine = !id.is_empty()
        && !id.starts_with('.')
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'));
    if !fine {
        bail!("{id:?} is not a goal id");
    }
    Ok(())
}

/// Runs the cockpit until `q`.
pub fn run(
    layout: &Layout,
    goal: Option<String>,
    interval: Duration,
    with_mascot: bool,
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
    );
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

    if with_mascot {
        match mascot::Frames::load(&mascot::default_dir()) {
            Ok(frames) => app.set_mascot(Some(mascot::Mascot::new(frames))),
            Err(err) => app.message = Some(format!("no mascot: {err:#}")),
        }
    }

    let mut screen = Screen::enter()?;
    loop {
        while let Ok(fresh) = snapshot.try_recv() {
            app.set_snapshot(fresh);
        }
        screen
            .terminal
            .draw(|frame| {
                let area = frame.area();
                app.draw(area, frame.buffer_mut());
            })
            .context("drawing")?;
        if !crossterm::event::poll(POLL).context("waiting for input")? {
            continue;
        }
        let event = crossterm::event::read().context("reading input")?;
        match app.handle(&event) {
            Effect::Quit => break,
            Effect::Refresh => {
                // The reader is gone only if it panicked, and then the screen
                // simply stops refreshing; quitting still works.
                let _ = asks.send(app.open_run_id());
            }
            Effect::None => {}
        }
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
        if let Err(err) = crossterm::execute!(stdout, EnterAlternateScreen, EnableMouseCapture) {
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

impl Drop for Screen {
    fn drop(&mut self) {
        restore();
        let _ = self.terminal.show_cursor();
    }
}

/// Gives the terminal back. Safe to call twice, which a panic inside the loop
/// does: once from the hook and once from the drop.
fn restore() {
    let _ = crossterm::execute!(std::io::stdout(), DisableMouseCapture, LeaveAlternateScreen);
    let _ = disable_raw_mode();
    let _ = crossterm::execute!(std::io::stdout(), crossterm::cursor::Show);
}
