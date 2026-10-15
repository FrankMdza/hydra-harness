//! `hydra stop`, build ticket 169.
//!
//! Every run here is a real detached watcher over a stub provider on `/bin/sh`,
//! the way `tests/detached.rs` drives one, so the stop goes through the same
//! marker, signal and closing line a real run would and spends no subscription.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use hydra::env::ChildEnv;
use hydra::outcome::Outcome;
use hydra::paths::Layout;
use hydra::policy::PolicySet;
use hydra::provider::Provider;
use hydra::record::{RunStart, SCHEMA_VERSION};
use hydra::stop::{self, Target};
use hydra::watch::{self, Launch, Waited};

const SESSION: &str = "01996d3c-8f2a-7c31-9b44-000000000169";
const SEAT: &str = "01996d3c-8f2a-7c31-9b44-000000000170";

/// Prints one frame and then waits far longer than any test does.
const LONG_STUB: &str =
    r#"printf '{"type":"system","subtype":"init","model":"stub-1"}\n'; sleep 120"#;

fn sandbox(name: &str) -> Layout {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let root = std::env::temp_dir().join(format!(
        "hydra-stop-{}-{name}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&root);
    let layout = Layout::at(&root);
    layout.ensure().unwrap();
    layout
}

fn line_zero(depth: u32) -> serde_json::Value {
    serde_json::to_value(RunStart {
        kind: "run_start",
        schema_version: SCHEMA_VERSION,
        hydra_version: "0.2.0".into(),
        command: "run".into(),
        launch_mode: Some("recorded".into()),
        provider_version: Some("stub".into()),
        provider_session_id: None,
        task: Some("stub".into()),
        policy: Some("standard".into()),
        policy_sha: None,
        floor_lifted: Some(false),
        gate_sha: None,
        sandbox: None,
        sandbox_kind: None,
        host: None,
        scopes: None,
        cwd: "/tmp".into(),
        worktree: None,
        depth,
        parent_policy: None,
        pid: None,
        process_start_time: None,
        skills: None,
        mcp: None,
        auth: None,
        env_allowlist: Some(vec!["PATH".into()]),
        subscription: Some("none".into()),
        resumed_from: None,
        may_edit: None,
    })
    .unwrap()
}

/// Starts a detached watcher for `run_id` over `script` and waits for line 0.
fn start(layout: &Layout, run_id: &str, parent: Option<&str>, script: &str) -> std::process::Child {
    let spec = layout.launch_spec(run_id);
    watch::write_spec(
        &spec,
        &Launch {
            run_id: run_id.to_string(),
            parent_run_id: parent.map(str::to_string),
            provider: "claude".to_string(),
            binary: "/bin/sh".to_string(),
            argv: vec!["-c".to_string(), script.to_string()],
            cwd: PathBuf::from("/tmp"),
            run_file: layout.run_file(run_id),
            model: None,
            hydra_env: Default::default(),
            run_start: line_zero(u32::from(parent.is_some())),
            sandbox_profile: None,
            skill_snapshot: None,
        },
    )
    .unwrap();
    let mut watcher = watch::detach(
        Path::new(env!("CARGO_BIN_EXE_hydra")),
        &spec,
        &layout.watcher_log(run_id),
        &ChildEnv::build(Provider::Claude).unwrap(),
    )
    .unwrap();
    watch::wait_for_start(
        &layout.run_file(run_id),
        &mut watcher,
        Duration::from_secs(10),
    )
    .unwrap();
    watcher
}

fn policies() -> PolicySet {
    PolicySet::discover().unwrap()
}

fn closing(layout: &Layout, run_id: &str) -> serde_json::Value {
    let text = std::fs::read_to_string(layout.run_file(run_id)).unwrap();
    let last = text.lines().last().unwrap();
    serde_json::from_str(last).unwrap()
}

#[test]
fn a_stopped_run_closes_its_record_and_derives_stopped() {
    let layout = sandbox("one");
    let mut watcher = start(&layout, SESSION, None, LONG_STUB);

    let stopped = stop::stop(
        &layout,
        &policies(),
        Target::Run(&SESSION[..20]),
        Duration::from_secs(10),
    )
    .unwrap();
    watcher.wait().unwrap();

    let end = closing(&layout, SESSION);
    assert_eq!(end["hydra"]["type"], "run_end", "{end}");
    let error = end["hydra"]["error"].as_str().unwrap_or_default();
    assert!(error.starts_with(watch::STOP_NOTE), "{error}");

    assert_eq!(stopped.len(), 1);
    assert_eq!(stopped[0].run_id, SESSION);
    assert!(!stopped[0].killed, "sh exits on SIGTERM");
    // Before map ticket 89 this was `crashed`, off the watcher's signal note.
    assert_eq!(stopped[0].outcome, Some(Outcome::Stopped));
    assert!(!Outcome::Stopped.rung_one_learns());
    assert!(stop::report(&stopped).contains(&format!("stopped {SESSION}: stopped")));
}

#[test]
fn a_session_stop_ends_the_seat_before_the_session() {
    let layout = sandbox("session");
    let mut session = start(&layout, SESSION, None, LONG_STUB);
    let mut seat = start(&layout, SEAT, Some(SESSION), LONG_STUB);

    let stopped = stop::stop(
        &layout,
        &policies(),
        Target::Session(SESSION),
        Duration::from_secs(10),
    )
    .unwrap();
    session.wait().unwrap();
    seat.wait().unwrap();

    let order: Vec<&str> = stopped.iter().map(|run| run.run_id.as_str()).collect();
    assert_eq!(order, [SEAT, SESSION]);
    assert!(
        stopped
            .iter()
            .all(|run| run.outcome == Some(Outcome::Stopped)),
        "{stopped:?}"
    );

    // The records agree with the order: the seat's closing line is no later
    // than the session's, and `stop` waits for each record to close in turn.
    let seat_end = closing(&layout, SEAT)["ts"].as_str().unwrap().to_string();
    let session_end = closing(&layout, SESSION)["ts"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(seat_end <= session_end, "{seat_end} {session_end}");
}

#[test]
fn a_run_that_is_not_live_is_refused() {
    let layout = sandbox("ended");
    let mut watcher = start(&layout, SESSION, None, "true");
    assert_eq!(
        watch::wait_for_end(
            &layout.run_file(SESSION),
            &mut watcher,
            Duration::from_secs(20)
        )
        .unwrap(),
        Waited::Ended
    );
    watcher.wait().unwrap();

    let err = stop::stop(
        &layout,
        &policies(),
        Target::Run(SESSION),
        Duration::from_secs(10),
    )
    .unwrap_err()
    .to_string();
    assert!(err.contains("not live"), "{err}");
    // Nothing was marked, so the ended record keeps its own reading.
    assert!(!layout.stop_marker(SESSION).exists());
}

#[test]
fn a_prefix_that_names_two_runs_is_refused() {
    let layout = sandbox("ambiguous");
    std::fs::create_dir_all(layout.runs()).unwrap();
    for run_id in [SESSION, SEAT] {
        std::fs::write(layout.run_file(run_id), "").unwrap();
    }

    let err = stop::stop(
        &layout,
        &policies(),
        Target::Run("01996d3c"),
        Duration::from_secs(10),
    )
    .unwrap_err()
    .to_string();
    assert!(err.contains("matches 2 runs"), "{err}");
    assert!(!layout.stop_marker(SESSION).exists());
    assert!(!layout.stop_marker(SEAT).exists());
}

/// Whether `pid` is still a process, zombie or not.
fn alive(pid: libc::pid_t) -> bool {
    // SAFETY: kill with signal 0 takes plain integers and sends nothing.
    unsafe { libc::kill(pid, 0) == 0 }
}

#[test]
fn a_descendant_that_ignores_sigterm_is_killed_after_the_shell_exits() {
    // Build ticket 173: the recorded pid is the shell, which exits on SIGTERM
    // at once. Its background child ignores SIGTERM and SIGHUP and stays in
    // the shell's process group, which the pty made the shell lead.
    let layout = sandbox("descendant");
    let pid_file = layout.root().join("descendant.pid");
    let script = format!(
        r#"printf '{{"type":"system","subtype":"init","model":"stub-1"}}\n'; (trap '' TERM HUP; exec sleep 120) & echo $! > '{}'; wait"#,
        pid_file.display()
    );
    let mut watcher = start(&layout, SESSION, None, &script);
    let descendant: libc::pid_t = {
        let deadline = std::time::Instant::now() + Duration::from_secs(10);
        loop {
            if let Some(pid) = std::fs::read_to_string(&pid_file)
                .ok()
                .and_then(|text| text.trim().parse().ok())
            {
                break pid;
            }
            assert!(std::time::Instant::now() < deadline, "no descendant pid");
            std::thread::sleep(Duration::from_millis(20));
        }
    };

    let stopped = stop::stop(
        &layout,
        &policies(),
        Target::Run(SESSION),
        Duration::from_secs(1),
    )
    .unwrap();
    watcher.wait().unwrap();

    // Reparented once the shell went, so whoever adopted it reaps it.
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while alive(descendant) && std::time::Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(20));
    }
    let survived = alive(descendant);
    if survived {
        // SAFETY: plain integers; the pid is the stub's own `sleep`.
        unsafe { libc::kill(descendant, libc::SIGKILL) };
    }
    assert!(!survived, "descendant {descendant} outlived the stop");
    assert_eq!(stopped.len(), 1);
    assert!(stopped[0].killed, "the group got SIGKILL: {stopped:?}");
    assert_eq!(stopped[0].outcome, Some(Outcome::Stopped));
}

#[test]
fn a_seat_whose_record_appears_after_the_stops_scan_stops_itself() {
    // Build ticket 173: the seat's launch passed the launcher's check before
    // the stop marked the session, and its record appears only after the
    // stop's scan, so no stop list names it. Driven in that order: the stop
    // marks, scans and ends the session, then the seat's watcher writes line 0.
    let layout = sandbox("late-seat");
    let mut session = start(&layout, SESSION, None, LONG_STUB);
    let stopped = stop::stop(
        &layout,
        &policies(),
        Target::Session(SESSION),
        Duration::from_secs(10),
    )
    .unwrap();
    session.wait().unwrap();
    let order: Vec<&str> = stopped.iter().map(|run| run.run_id.as_str()).collect();
    assert_eq!(order, [SESSION], "the seat did not exist at the scan");

    let mut seat = start(&layout, SEAT, Some(SESSION), LONG_STUB);
    let waited =
        watch::wait_for_end(&layout.run_file(SEAT), &mut seat, Duration::from_secs(20)).unwrap();
    if waited != Waited::Ended {
        let _ = seat.kill();
    }
    assert_eq!(
        waited,
        Waited::Ended,
        "the seat outlived its stopped session"
    );
    seat.wait().unwrap();

    let end = closing(&layout, SEAT);
    let error = end["hydra"]["error"].as_str().unwrap_or_default();
    assert!(error.starts_with(watch::STOP_NOTE), "{error}");
    assert!(layout.stop_marker(SEAT).exists());
}

// --- stepping in on a seat from the cockpit, build ticket 184 -----------------

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use hydra::cockpit::app::{App, Focus};
use hydra::cockpit::chat::{Chat, Launch as ChatLaunch};
use hydra::cockpit::host::{Host, Side};
use hydra::cockpit::snapshot::{Card, Lane, Snapshot};
use hydra::live::State;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

const AREA: Rect = Rect::new(0, 0, 200, 40);

/// What `y` pastes for [`SEAT`], whose line 0 says its task was `stub`.
const DRAFT: &str = "Seat 01996d3c (claude stub-1) was stopped by the human. Its task was: stub. \
                     Instead: ";

fn key(code: KeyCode) -> Event {
    Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
}

/// A cockpit on `layout` with the live lane focused, which knows where the
/// policies are so a stop can read the outcome back.
fn cockpit_on_the_seat(layout: &Layout) -> App {
    let mut app = App::new(Layout::at(layout.root()), false, Duration::from_secs(2));
    app.set_policies(Path::new(env!("CARGO_MANIFEST_DIR")).join("policies"));
    app.focus = Focus::Lane(Lane::Live);
    app
}

fn seat_card() -> Card {
    Card {
        run_id: SEAT.to_string(),
        provider: Some("claude".to_string()),
        model: Some("stub-1".to_string()),
        policy: Some("standard".to_string()),
        policy_sha: None,
        parent_run_id: Some(SESSION.to_string()),
        state: State::Live,
        outcome: None,
        total_tokens: None,
        result_text: None,
        note: None,
    }
}

fn drawn(host: &mut Host) -> String {
    let mut buf = Buffer::empty(AREA);
    host.draw(AREA, &mut buf);
    (0..AREA.height)
        .map(|y| {
            (0..AREA.width)
                .map(|x| buf[(x, y)].symbol().to_string())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// A chat whose child names [`SESSION`] as its run, puts the pty in raw
/// mode, turns bracketed paste on when `bracketed`, and writes every byte it
/// is sent to `out`, so the test reads what reached the pty and nothing the
/// line discipline did to it.
fn recording_chat(layout: &Layout, out: &Path, bracketed: bool) -> Host {
    let paste_mode = if bracketed {
        r"printf '\033[?2004h'; "
    } else {
        ""
    };
    let script = format!(
        r#"stty raw -echo; {paste_mode}printf 'hydra: run %s\r\nready\r\n' "$RUN"; exec cat > "$OUT""#
    );
    let launch = ChatLaunch {
        program: "/bin/sh".to_string(),
        args: vec!["-c".to_string(), script],
        cwd: layout.root().to_path_buf(),
        env: vec![
            ("PATH".to_string(), "/usr/bin:/bin".to_string()),
            ("TERM".to_string(), "xterm-256color".to_string()),
            ("RUN".to_string(), SESSION.to_string()),
            ("OUT".to_string(), out.display().to_string()),
        ],
        layout: Layout::at(layout.root()),
        policies: Ok(policies()),
        harvest_home: None,
    };
    let (rows, cols) = Host::chat_size(AREA);
    let chat = Chat::spawn(launch, rows, cols).unwrap();
    let mut host = Host::with_chat(cockpit_on_the_seat(layout), "stub".to_string(), chat);
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while !(host.chat().is_some_and(|chat| chat.run_id().is_some())
        && drawn(&mut host).contains("ready"))
    {
        assert!(
            std::time::Instant::now() < deadline,
            "the stub never named its run"
        );
        host.pump();
        std::thread::sleep(Duration::from_millis(20));
    }
    host
}

/// The bytes the stub has written to `out` once they end in `until`, or
/// whatever it holds after five seconds.
fn recorded(out: &Path, until: &[u8]) -> Vec<u8> {
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    loop {
        let bytes = std::fs::read(out).unwrap_or_default();
        if bytes.ends_with(until) || std::time::Instant::now() >= deadline {
            return bytes;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// Types `abc` into the chat, stops [`SEAT`] from the cockpit with `m` and
/// `y`, and returns what reached the pty.
fn step_in(layout: &Layout, bracketed: bool) -> (Host, Vec<u8>) {
    let out = layout.root().join("pty-bytes");
    let mut host = recording_chat(layout, &out, bracketed);
    let mut watcher = start(layout, SEAT, Some(SESSION), LONG_STUB);

    for c in ['a', 'b', 'c'] {
        host.handle(&key(KeyCode::Char(c)));
    }
    assert_eq!(recorded(&out, b"abc"), b"abc", "the typing reached the pty");
    host.handle(&key(KeyCode::F(2)));
    host.app.set_snapshot(Snapshot {
        cards: vec![seat_card()],
        ..Snapshot::default()
    });

    host.handle(&key(KeyCode::Char('m')));
    let asked = drawn(&mut host);
    host.handle(&key(KeyCode::Char('y')));
    reported(&mut host);
    ended(layout, SEAT, &mut watcher);
    assert!(
        asked.contains("stop 01996d3c and draft a note to the session? y"),
        "{asked}"
    );

    let end = closing(layout, SEAT);
    let error = end["hydra"]["error"].as_str().unwrap_or_default();
    assert!(error.starts_with(watch::STOP_NOTE), "{error}");
    let tail: &[u8] = if bracketed {
        b"\x1b[201~"
    } else {
        b"Instead: "
    };
    let bytes = recorded(&out, tail);
    (host, bytes)
}

/// The stop has reported, so the seat's record is closed by now. A watcher
/// that is still going is killed rather than waited on for two minutes.
fn ended(layout: &Layout, run_id: &str, watcher: &mut std::process::Child) {
    let waited =
        watch::wait_for_end(&layout.run_file(run_id), watcher, Duration::from_secs(1)).unwrap();
    if waited != Waited::Ended {
        let _ = watcher.kill();
    }
    watcher.wait().unwrap();
    assert_eq!(waited, Waited::Ended, "the seat was not stopped");
}

/// Pumps and draws until the stop has reported and set its message, then
/// captures a fresh screen of that settled state.
fn reported(host: &mut Host) -> String {
    let deadline = Instant::now() + stop::GRACE + stop::CLOSE_BUDGET + Duration::from_secs(5);
    loop {
        host.pump();
        // Host::draw polls stops again after painting the card. Checking
        // after that draw can accept a screen painted before the answer.
        if !host.app.stopping() && host.app.message.is_some() {
            return drawn(host);
        }
        let text = drawn(host);
        assert!(
            Instant::now() < deadline,
            "the stop never reported:\n{text}"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn hang_up(host: &mut Host) {
    host.handle(&Event::Key(KeyEvent::new(
        KeyCode::Char('q'),
        KeyModifiers::CONTROL,
    )));
    host.handle(&key(KeyCode::Char('y')));
}

#[test]
fn m_stops_the_seat_and_pastes_a_note_the_human_has_to_send() {
    let layout = sandbox("draft-bracketed");
    let (mut host, bytes) = step_in(&layout, true);

    let expected = [b"abc\x1b[200~".as_slice(), DRAFT.as_bytes(), b"\x1b[201~"].concat();
    assert_eq!(
        String::from_utf8_lossy(&bytes),
        String::from_utf8_lossy(&expected)
    );
    // Nothing after the closing bracket: no `\r`, no `\n`, so the note sits
    // in the input until the human sends it, after what they had typed.
    assert!(bytes.ends_with(b"\x1b[201~"), "{bytes:?}");
    assert_eq!(host.side, Side::Chat, "the keys went back to the chat");
    hang_up(&mut host);
}

#[test]
fn without_bracketed_paste_the_note_still_ends_without_a_newline() {
    let layout = sandbox("draft-plain");
    let (mut host, bytes) = step_in(&layout, false);

    let expected = [b"abc".as_slice(), DRAFT.as_bytes()].concat();
    assert_eq!(
        String::from_utf8_lossy(&bytes),
        String::from_utf8_lossy(&expected)
    );
    assert!(
        !bytes.ends_with(b"\r") && !bytes.ends_with(b"\n"),
        "{bytes:?}"
    );
    hang_up(&mut host);
}

#[test]
fn m_without_a_session_stops_the_seat_and_says_to_tell_the_session() {
    let layout = sandbox("draft-alone");
    let mut watcher = start(&layout, SEAT, Some(SESSION), LONG_STUB);
    let mut host = Host::alone(cockpit_on_the_seat(&layout));
    host.app.set_snapshot(Snapshot {
        cards: vec![seat_card()],
        ..Snapshot::default()
    });

    host.handle(&key(KeyCode::Char('m')));
    let asked = drawn(&mut host);
    host.handle(&key(KeyCode::Char('y')));
    reported(&mut host);
    ended(&layout, SEAT, &mut watcher);
    assert!(asked.contains("stop 01996d3c? y"), "{asked}");

    let end = closing(&layout, SEAT);
    let error = end["hydra"]["error"].as_str().unwrap_or_default();
    assert!(error.starts_with(watch::STOP_NOTE), "{error}");
    let said = host.app.message.clone().unwrap_or_default();
    assert!(said.contains(&format!("stopped {SEAT}: stopped")), "{said}");
    assert!(said.contains("tell the session yourself"), "{said}");
}

// --- the cockpit draws while a stop waits, build ticket 186 -------------------

/// Traps SIGTERM and exits two seconds after it, so the stop waits that long
/// for the group to empty and [`stop::GRACE`] never runs out.
const HOLDS_ON: &str = r#"trap 'sleep 2; exit 0' TERM; printf '{"type":"system","subtype":"init","model":"stub-1"}\n'; while :; do sleep 0.1; done"#;

#[test]
fn y_returns_at_once_and_the_cockpit_keeps_going_while_the_seat_holds_on() {
    let layout = sandbox("holds-on");
    let mut watcher = start(&layout, SEAT, Some(SESSION), HOLDS_ON);
    let mut host = Host::alone(cockpit_on_the_seat(&layout));
    host.app.set_snapshot(Snapshot {
        cards: vec![seat_card()],
        ..Snapshot::default()
    });

    host.handle(&key(KeyCode::Char('m')));
    let asked_at = Instant::now();
    host.handle(&key(KeyCode::Char('y')));
    let held = asked_at.elapsed();
    let text = drawn(&mut host);
    assert!(
        held < Duration::from_secs(1),
        "y held the cockpit for {held:?}"
    );
    assert!(text.contains("01996d3c claude stub-1 stopping"), "{text}");

    // A key handled while the seat holds on: a second `m` says so and asks nothing.
    host.handle(&key(KeyCode::Char('m')));
    let text = drawn(&mut host);
    assert!(
        host.app.stopping(),
        "the seat held on, so the stop is still waiting"
    );
    let said = host.app.message.clone().unwrap_or_default();
    assert!(said.contains("01996d3c is stopping already"), "{said}");
    assert!(!text.contains("stop 01996d3c? y"), "{text}");

    let text = reported(&mut host);
    ended(&layout, SEAT, &mut watcher);
    assert!(
        asked_at.elapsed() >= Duration::from_secs(2),
        "the seat did not hold on, so this test proved nothing"
    );
    assert!(text.contains("01996d3c claude stub-1 stopped"), "{text}");
    let said = host.app.message.clone().unwrap_or_default();
    assert!(said.contains(&format!("stopped {SEAT}: stopped")), "{said}");
}

#[test]
fn the_note_waits_until_the_stop_reports() {
    let layout = sandbox("note-waits");
    let out = layout.root().join("pty-bytes");
    let mut host = recording_chat(&layout, &out, false);
    let mut watcher = start(&layout, SEAT, Some(SESSION), HOLDS_ON);
    host.handle(&key(KeyCode::F(2)));
    host.app.set_snapshot(Snapshot {
        cards: vec![seat_card()],
        ..Snapshot::default()
    });

    host.handle(&key(KeyCode::Char('m')));
    host.handle(&key(KeyCode::Char('y')));
    drawn(&mut host);
    assert_eq!(
        host.side,
        Side::Cockpit,
        "the keys moved before the stop reported"
    );
    assert!(
        host.app.stopping(),
        "the seat held on, so the stop is still waiting"
    );
    std::thread::sleep(Duration::from_millis(300));
    host.pump();
    assert_eq!(
        std::fs::read(&out).unwrap_or_default(),
        b"",
        "something reached the pty before the stop reported"
    );

    reported(&mut host);
    ended(&layout, SEAT, &mut watcher);
    let bytes = recorded(&out, b"Instead: ");
    assert_eq!(String::from_utf8_lossy(&bytes), DRAFT);
    assert_eq!(
        host.side,
        Side::Chat,
        "the keys went to the chat with the note"
    );
    hang_up(&mut host);
}

#[test]
fn a_stop_that_fails_pastes_nothing() {
    let layout = sandbox("stop-fails");
    let out = layout.root().join("pty-bytes");
    let mut host = recording_chat(&layout, &out, false);
    // No watcher, so no record, and the stop refuses.
    host.handle(&key(KeyCode::F(2)));
    host.app.set_snapshot(Snapshot {
        cards: vec![seat_card()],
        ..Snapshot::default()
    });

    host.handle(&key(KeyCode::Char('m')));
    host.handle(&key(KeyCode::Char('y')));
    let text = reported(&mut host);
    let said = host.app.message.clone().unwrap_or_default();
    assert!(said.contains("01996d3c was not stopped"), "{said}");
    assert!(text.contains("01996d3c claude stub-1 live"), "{text}");
    assert_eq!(host.side, Side::Cockpit);
    std::thread::sleep(Duration::from_millis(300));
    host.pump();
    assert_eq!(std::fs::read(&out).unwrap_or_default(), b"");
    hang_up(&mut host);
}

// --- every note reaches the chat, build ticket 187 ----------------------------

const SECOND_SEAT: &str = "01996d3c-8f2a-7c31-9b44-000000000171";

#[test]
fn two_stops_that_report_in_one_poll_paste_both_notes_in_order() {
    let layout = sandbox("two-notes");
    let out = layout.root().join("pty-bytes");
    let mut host = recording_chat(&layout, &out, false);
    let mut first = start(&layout, SEAT, Some(SESSION), HOLDS_ON);
    let mut second = start(&layout, SECOND_SEAT, Some(SESSION), HOLDS_ON);
    host.handle(&key(KeyCode::F(2)));
    // The second card names another model, so its note differs from the first's.
    let other = Card {
        run_id: SECOND_SEAT.to_string(),
        model: Some("stub-2".to_string()),
        ..seat_card()
    };
    host.app.set_snapshot(Snapshot {
        cards: vec![seat_card(), other],
        ..Snapshot::default()
    });

    host.handle(&key(KeyCode::Char('m')));
    host.handle(&key(KeyCode::Char('y')));
    host.handle(&key(KeyCode::Down));
    host.handle(&key(KeyCode::Char('m')));
    let said = host.app.message.clone().unwrap_or_default();
    assert!(host.app.confirming_stop.is_some(), "{said}");
    host.handle(&key(KeyCode::Char('y')));
    assert!(host.app.stopping());

    // Both seats hold on two seconds after the signal. Nothing draws or
    // pumps meanwhile, so both answers wait for the same poll.
    for (run_id, watcher) in [(SEAT, &mut first), (SECOND_SEAT, &mut second)] {
        let waited =
            watch::wait_for_end(&layout.run_file(run_id), watcher, Duration::from_secs(20))
                .unwrap();
        if waited != Waited::Ended {
            let _ = watcher.kill();
        }
        watcher.wait().unwrap();
        assert_eq!(waited, Waited::Ended, "{run_id} was not stopped");
    }
    // Each thread sends once it has read its record closed.
    std::thread::sleep(Duration::from_secs(1));
    assert_eq!(std::fs::read(&out).unwrap_or_default(), b"");
    host.pump();
    assert!(!host.app.stopping(), "a stop had not reported by the poll");

    let second_draft = DRAFT.replace("stub-1", "stub-2");
    let expected = format!("{DRAFT}{second_draft}");
    let bytes = recorded(&out, expected.as_bytes());
    assert_eq!(String::from_utf8_lossy(&bytes), expected);
    assert!(
        !bytes.ends_with(b"\r") && !bytes.ends_with(b"\n"),
        "{bytes:?}"
    );
    assert_eq!(host.side, Side::Chat);
    hang_up(&mut host);
}

// --- a refusal received during painting, build ticket 311 --------------------

#[test]
fn a_refusal_during_draw_paints_live_and_its_message_in_that_frame() {
    use std::io::Write;
    use std::os::unix::ffi::OsStrExt;

    let layout = sandbox("refusal-during-draw");
    let out = layout.root().join("pty-bytes");
    let mut host = recording_chat(&layout, &out, false);
    host.handle(&key(KeyCode::F(2)));
    // A busy snapshot leaves a window for the controlled watcher to answer.
    host.app.set_snapshot(Snapshot {
        cards: vec![seat_card(); 10_000],
        ..Snapshot::default()
    });
    let path = layout.run_file(SEAT);
    let name = std::ffi::CString::new(path.as_os_str().as_bytes()).unwrap();
    // SAFETY: name is a terminated path; mkfifo retains no pointer.
    assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
    host.handle(&key(KeyCode::Char('m')));
    host.handle(&key(KeyCode::Char('y')));
    // Opening the writer rendezvous with the stop's record reader. Keeping
    // it open holds that reader until this fixture releases it for the draw.
    let mut writer = std::fs::OpenOptions::new().write(true).open(&path).unwrap();
    assert!(host.app.stopping());
    let (tx, rx) = std::sync::mpsc::channel();
    let watcher = std::thread::spawn(move || {
        rx.recv_timeout(Duration::from_secs(5)).unwrap();
        std::thread::sleep(Duration::from_millis(10));
        std::fs::remove_file(&path).unwrap();
        std::fs::write(&path, b"refused\n").unwrap();
        writer.write_all(b"refused\n").unwrap();
        // The held reader gets its line; later opens see the refused record.
    });
    tx.send(()).unwrap();
    let text = drawn(&mut host);
    watcher.join().unwrap();
    let settled = !host.app.stopping();
    let said = host.app.message.clone().unwrap_or_default();
    let side = host.side;
    hang_up(&mut host);
    assert!(settled, "the fixture did not answer during the draw");
    assert!(!text.contains("01996d3c claude stub-1 stopping"), "{text}");
    assert!(said.contains("01996d3c was not stopped"), "{said}");
    assert!(text.contains("01996d3c claude stub-1 live"), "{text}");
    assert!(text.contains("01996d3c was not stopped"), "{text}");
    assert_eq!(side, Side::Cockpit);
    assert_eq!(std::fs::read(&out).unwrap_or_default(), b"");
}
