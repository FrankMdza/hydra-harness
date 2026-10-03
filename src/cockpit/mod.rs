//! `hydra cockpit`, map ticket 86's cut one and build ticket 150.
//!
//! A full-screen view a human runs in a terminal of their own, beside the chat of
//! an orchestrating session: the task table of one goal from its Master of
//! Puppets log, a kanban of runs by state, and the detail of one run. It reads
//! files and draws. Without `--session` it launches no provider itself; it
//! edits no policy and no config, and needs nothing running to exist (D10).
//!
//! **`v` then `y` starts `hydra-puppets review`**, build ticket 183, which
//! launches three judge runs. The confirmation says they count against quota.
//!
//! **Its one write is the approval of a waiting seat request**, typed by the
//! human in the cockpit, through [`crate::approve::approve`], the function
//! `hydra approve <id>` calls. Like that command it refuses inside a seatbelt
//! and says so on screen.
//!
//! **It never writes a goal log.** Since build ticket 182 the waiting lane also
//! lists what `hydra-puppets pending <goal> --json` says the shown goal waits on,
//! and `a` then `y` runs the argv that listed, `--expect-revision` included, as
//! a child. `hydra-puppets` stays the only writer of its log. Inside a seatbelt
//! `a` refuses these too.
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
pub mod call_head;
pub mod chat;
pub mod goal;
pub mod host;
pub mod keys;
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
}

/// The Master of Puppets helper's file name.
pub const PUPPETS_HELPER: &str = "hydra-puppets";

/// The helper the cockpit runs for `pending`, `approve` and `review`, build ticket 196,
/// or the line the cockpit shows when there is none.
pub fn puppets_helper() -> Result<PathBuf, String> {
    helper_beside(std::env::current_exe().ok().as_deref())
}

/// `hydra-puppets` in the directory that holds the real `exe`, which is where one `cargo
/// build --workspace` puts both binaries. `exe` is canonicalized first, because
/// `current_exe` answers with a symlink's own path on macOS and the helper is beside the
/// binary the link names. With no helper there the answer is the line to show and nothing
/// is run: a bare name would be looked up on `PATH`, and `PATH` is in the environment a
/// gated run is given. Build ticket 221.
pub fn helper_beside(exe: Option<&Path>) -> Result<PathBuf, String> {
    let real = exe.and_then(|exe| exe.canonicalize().ok());
    let helper = real
        .as_deref()
        .and_then(Path::parent)
        .map(|dir| dir.join(PUPPETS_HELPER))
        .filter(|helper| helper.is_absolute() && helper.is_file());
    helper.ok_or_else(|| {
        let beside = match (&real, exe) {
            (Some(real), _) => real.display().to_string(),
            (None, Some(exe)) => exe.display().to_string(),
            (None, None) => "a hydra whose path is unknown".to_string(),
        };
        format!("no {PUPPETS_HELPER} beside {beside}; run cargo build --release --workspace")
    })
}

/// A command that runs `program`, refused unless `program` is an absolute path. Every
/// spawn of the helper goes through here, the argv a step carries included, so none of
/// them is a search of `PATH`. Build ticket 221.
pub(crate) fn helper_command(program: &Path) -> Result<std::process::Command> {
    if !program.is_absolute() {
        bail!(
            "{} is not an absolute path, and the helper is never looked up on PATH",
            program.display()
        );
    }
    Ok(std::process::Command::new(program))
}

/// Where the goal logs live: `HYDRA_PUPPETS_HOME` when set, as `hydra-puppets`
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
    .with_vendors(crate::paths::hydra_home().join("integrations/puppets"))
    .with_helper(puppets_helper());
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
    app.set_preamble(crate::preamble::load().ok());
    app.set_helper(puppets_helper(), puppets_home(layout));

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
            let launch = Launch::hydra_run(layout, &session.provider, session.model.as_deref())?;
            let chat = Chat::spawn(launch, rows, cols)?;
            let title = session.provider;
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

/// How a child [`bounded`] waited on ended.
pub(crate) enum Bounded {
    /// It exited inside the budget: its status and the last non-empty line
    /// it printed, stderr's before stdout's when it failed.
    Exited(std::process::ExitStatus, String),
    /// It was still running when the budget ran out.
    Late(std::process::Child),
}

/// Starts `command` with stdin closed, both streams piped and a process group
/// of its own, and waits at most `budget` for it. Each stream is drained on a
/// thread so a child that prints a lot cannot stall on a full pipe.
pub(crate) fn bounded(
    mut command: std::process::Command,
    budget: Duration,
) -> Result<(Bounded, Vec<u8>)> {
    use std::io::Read;
    use std::os::unix::process::CommandExt;
    use std::process::Stdio;

    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .process_group(0)
        .spawn()
        .context("starting the helper")?;
    let drain = |stream: Option<Box<dyn Read + Send>>| {
        let (tx, rx) = mpsc::channel();
        if let Some(mut stream) = stream {
            std::thread::spawn(move || {
                let mut bytes = Vec::new();
                let _ = stream.read_to_end(&mut bytes);
                let _ = tx.send(bytes);
            });
        }
        rx
    };
    let out = drain(
        child
            .stdout
            .take()
            .map(|s| Box::new(s) as Box<dyn Read + Send>),
    );
    let err = drain(
        child
            .stderr
            .take()
            .map(|s| Box::new(s) as Box<dyn Read + Send>),
    );
    let start = std::time::Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().context("waiting for the helper")? {
            break status;
        }
        if start.elapsed() >= budget {
            return Ok((Bounded::Late(child), Vec::new()));
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    // A grandchild holding a pipe open does not hold the cockpit past the budget.
    let left = budget.saturating_sub(start.elapsed()) + Duration::from_millis(200);
    let out = out.recv_timeout(left).unwrap_or_default();
    let err = err.recv_timeout(left).unwrap_or_default();
    let last = |bytes: &[u8]| {
        String::from_utf8_lossy(bytes)
            .lines()
            .rev()
            .find(|line| !line.trim().is_empty())
            .map(|line| line.trim().to_string())
    };
    let line = if status.success() {
        last(&out).or_else(|| last(&err))
    } else {
        last(&err).or_else(|| last(&out))
    };
    Ok((Bounded::Exited(status, line.unwrap_or_default()), out))
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
