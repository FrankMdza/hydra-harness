//! `hydra cockpit`, build ticket 150.
//!
//! No terminal anywhere. The fold reads a goal log written here, the lanes read
//! records copied from the fixture corpus into a scratch root, and the hit test
//! and the approval drive [`App`] over a `Buffer` with synthetic crossterm
//! events. Nothing here touches the real `~/hydra`.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use hydra::approve::Pending;
use hydra::cockpit::app::{App, Effect, Focus, Opened};
use hydra::cockpit::goal::{self, TaskRow};
use hydra::cockpit::snapshot::{Card, Lane, Reader, Snapshot, lane_of};
use hydra::live::State;
use hydra::paths::Layout;
use hydra::policy::PolicySet;
use hydra::seats::AskedSeat;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use serde_json::json;

fn scratch(name: &str) -> Layout {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let root = std::env::temp_dir().join(format!(
        "hydra-cockpit-{}-{name}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&root);
    let layout = Layout::at(&root);
    layout.ensure().unwrap();
    layout
}

fn policies() -> PolicySet {
    PolicySet::load(Path::new(env!("CARGO_MANIFEST_DIR")).join("policies")).unwrap()
}

fn fixture(label: &str, n: u32) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/records")
        .join(label)
        .join(format!("00000000-0000-7000-8000-{n:012}.jsonl"))
}

fn key(code: KeyCode) -> Event {
    Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
}

fn mouse(kind: MouseEventKind, column: u16, row: u16) -> Event {
    Event::Mouse(MouseEvent {
        kind,
        column,
        row,
        modifiers: KeyModifiers::NONE,
    })
}

fn click(column: u16, row: u16) -> Event {
    mouse(MouseEventKind::Down(MouseButton::Left), column, row)
}

/// The text of one row of a buffer.
fn row_of(buf: &Buffer, y: u16) -> String {
    (0..buf.area.width)
        .map(|x| buf[(x, y)].symbol().to_string())
        .collect()
}

fn screen_text(buf: &Buffer) -> String {
    (0..buf.area.height)
        .map(|y| row_of(buf, y))
        .collect::<Vec<_>>()
        .join("\n")
}

// --- the fold ------------------------------------------------------------

/// A goal log with every event that changes a row, one `fold` would raise on,
/// and a last line still being written.
const GOAL_LOG: &str = r#"{"seq": 1, "ts": "2026-09-29T10:00:00.000Z", "event": "goal", "text": "ship the cockpit", "repo": "/r", "base_sha": "abc"}
{"seq": 2, "ts": "2026-09-29T10:00:01.000Z", "event": "plan_proposed", "provider": "claude", "model": "opus", "run_id": "p1"}
{"seq": 3, "ts": "2026-09-29T10:00:02.000Z", "event": "task", "id": "t1", "text": "fold", "class": "rust_off_launch_path", "grade": "medium", "deps": [], "paths": []}
{"seq": 4, "ts": "2026-09-29T10:00:03.000Z", "event": "task", "id": "t2", "text": "lanes", "class": "rust_launch_path", "grade": "high", "deps": ["t1"], "paths": []}
{"seq": 5, "ts": "2026-09-29T10:00:04.000Z", "event": "task", "id": "t3", "text": "docs", "class": "docs", "grade": "low", "deps": [], "paths": []}
{"seq": 6, "ts": "2026-09-29T10:00:05.000Z", "event": "assignment", "task": "t1", "provider": "claude", "model": "sonnet", "policy": "standard", "source": "default"}
{"seq": 7, "ts": "2026-09-29T10:00:06.000Z", "event": "assignment", "task": "t2", "provider": "codex", "model": "", "policy": "standard", "source": "ranked"}
{"seq": 8, "ts": "2026-09-29T10:00:07.000Z", "event": "assignment", "task": "t3", "provider": "cursor-agent", "model": "composer-2.5", "policy": "standard", "source": "default"}
{"seq": 9, "ts": "2026-09-29T10:00:08.000Z", "event": "assignment", "task": "t9", "provider": "agy", "model": "gemini-3.1-pro-high", "policy": "standard", "source": "human"}
{"seq": 10, "ts": "2026-09-29T10:00:09.000Z", "event": "run", "task": "t1", "attempt": 1, "run_id": "01a0aaaa-0000-7000-8000-000000000001"}
{"seq": 11, "ts": "2026-09-29T10:00:10.000Z", "event": "run_end", "task": "t1", "run_id": "01a0aaaa-0000-7000-8000-000000000001", "outcome": "completed", "total_tokens": 10}
{"seq": 12, "ts": "2026-09-29T10:00:11.000Z", "event": "human_verdict", "task": "t1", "run_id": "01a0aaaa-0000-7000-8000-000000000001", "result": "applied", "note": null}
{"seq": 13, "ts": "2026-09-29T10:00:12.000Z", "event": "run", "task": "t2", "attempt": 1, "run_id": "01a0bbbb-0000-7000-8000-000000000002"}
{"seq": 14, "ts": "2026-09-29T10:00:13.000Z", "event": "run_end", "task": "t2", "run_id": "01a0ffff-0000-7000-8000-00000000dead", "outcome": "failed", "total_tokens": 1}
{"seq": 15, "event": "assignment", "task": "t3"}
{"seq": 16, "ts": "2026-09-29T10:00:15.000Z", "event": "task", "id": "t3", "text": "docs again", "class": "docs", "grade": "medium", "deps": [], "paths": []}
{"seq": 17, "ts": "2026-09-29T10:00:16.000Z", "event": "run", "task": "t3", "att"#;

fn row(
    id: &str,
    class: &str,
    grade: &str,
    agent: Option<(&str, Option<&str>)>,
    status: &str,
    run_id: Option<&str>,
) -> TaskRow {
    TaskRow {
        id: id.to_string(),
        class: class.to_string(),
        grade: grade.to_string(),
        provider: agent.map(|(p, _)| p.to_string()),
        model: agent.and_then(|(_, m)| m.map(str::to_string)),
        status: status.to_string(),
        run_id: run_id.map(str::to_string),
    }
}

#[test]
fn a_goal_log_folds_into_the_rows_board_prints() {
    let home = scratch("fold").root().join("puppets");
    std::fs::create_dir_all(&home).unwrap();
    let path = home.join("g-2026-09-29.jsonl");
    std::fs::write(&path, GOAL_LOG).unwrap();

    let goal = goal::read(&path).unwrap();
    assert_eq!(goal.id, "g-2026-09-29");
    assert_eq!(goal.text.as_deref(), Some("ship the cockpit"));
    // Line 15 has no provider, which `fold` raises on. The partial line 17 is
    // a write in progress, which `read_log` leaves for later, not a skip.
    assert_eq!(goal.skipped, 1);
    assert_eq!(
        goal.rows,
        vec![
            // A verdict replaces the status.
            row(
                "t1",
                "rust_off_launch_path",
                "medium",
                Some(("claude", Some("sonnet"))),
                "applied",
                Some("01a0aaaa-0000-7000-8000-000000000001"),
            ),
            // A `run_end` for another run id leaves the row running, and an
            // empty model is no model.
            row(
                "t2",
                "rust_launch_path",
                "high",
                Some(("codex", None)),
                "running",
                Some("01a0bbbb-0000-7000-8000-000000000002"),
            ),
            // A repeated `task` resets the row in place, assignment included.
            row("t3", "docs", "medium", None, "pending", None),
            // An assignment for a task never defined makes one with `fold`'s
            // defaults.
            row(
                "t9",
                "batch_scripting",
                "low",
                Some(("agy", Some("gemini-3.1-pro-high"))),
                "assigned",
                None,
            ),
        ]
    );

    // Without `--goal`, the log that changed last.
    std::thread::sleep(Duration::from_millis(20));
    let newer = home.join("g-newer.jsonl");
    std::fs::write(
        &newer,
        "{\"event\": \"goal\", \"text\": \"n\", \"repo\": \"/r\", \"base_sha\": \"a\"}\n",
    )
    .unwrap();
    assert_eq!(goal::latest(&home).unwrap(), Some(newer));
    assert_eq!(goal::latest(&home.join("absent")).unwrap(), None);
}

// --- the lanes -----------------------------------------------------------

/// A record whose line 0 names this test's own pid, which is alive.
fn write_live_run(layout: &Layout, run_id: &str) {
    let pid = std::process::id();
    let start = hydra::procinfo::start_time(pid).unwrap().unwrap();
    let line = json!({
        "run_id": run_id,
        "parent_run_id": null,
        "seq": 0,
        "ts": "2026-09-29T10:00:00.000Z",
        "provider": "codex",
        "model": "gpt-5.5",
        "secret_flag": null,
        "hydra": {
            "type": "run_start",
            "schema_version": 1,
            "hydra_version": "0.2.0",
            "command": "run",
            "launch_mode": "recorded",
            "provider_version": "stub",
            "provider_session_id": null,
            "task": "probe",
            "policy": "standard",
            "policy_sha": null,
            "cwd": "/tmp",
            "depth": 0,
            "pid": pid,
            "process_start_time": start.to_rfc3339_opts(chrono::SecondsFormat::Micros, true),
            "subscription": "none",
        },
    });
    std::fs::write(layout.run_file(run_id), format!("{line}\n")).unwrap();
}

const SESSION: &str = "01a0cccc-0000-7000-8000-00000000000c";
const REQUEST: &str = "01a0dddd-0000-7000-8000-00000000000d";

fn request() -> Pending {
    Pending {
        request_id: REQUEST.to_string(),
        session: SESSION.to_string(),
        session_policy: "standard".to_string(),
        task: "read the diff\nand say what breaks".to_string(),
        seats: vec![AskedSeat {
            provider: "claude".to_string(),
            model: Some("opus".to_string()),
            policy: "read-only".to_string(),
        }],
        asked_at: "2026-09-29T10:00:00.000Z".to_string(),
    }
}

/// A request waiting in the store, and a launcher socket for its session, the
/// two things `hydra approve` needs before it will rename.
fn plant_request(layout: &Layout) {
    let dir = layout.approvals();
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join(format!("{REQUEST}.pending.json")),
        serde_json::to_string_pretty(&request()).unwrap(),
    )
    .unwrap();
    let socket = hydra::seats::socket_path(layout, SESSION);
    std::fs::create_dir_all(socket.parent().unwrap()).unwrap();
    std::fs::write(socket, "").unwrap();
}

#[test]
fn runs_go_into_lanes_by_liveness_and_outcome() {
    let layout = scratch("lanes");
    let copy = |label: &str, n: u32| {
        let from = fixture(label, n);
        std::fs::copy(&from, layout.runs().join(from.file_name().unwrap())).unwrap();
    };
    copy("claude@2.1.261", 1);
    copy("claude@2.1.261", 4);
    copy("claude@2.1.261", 5);
    copy("claude@2.1.261", 6);
    copy("codex@0.154.0", 9);
    let live = "01a0eeee-0000-7000-8000-00000000000e";
    write_live_run(&layout, live);
    plant_request(&layout);

    let mut reader = Reader::new(
        Layout::at(layout.root()),
        Ok(policies()),
        layout.root().join("puppets"),
        None,
    );
    let snapshot = reader.read(Some(live));
    assert!(snapshot.errors.is_empty(), "{:?}", snapshot.errors);
    let card = |n: u32| -> &Card {
        let id = format!("00000000-0000-7000-8000-{n:012}");
        snapshot
            .cards
            .iter()
            .find(|c| c.run_id == id)
            .unwrap_or_else(|| panic!("no card for {id}"))
    };

    // Each outcome is read the way `hydra show` reads it: normalized, then
    // derived.
    for n in [1, 4, 5, 9] {
        let c = card(n);
        let label = if n == 9 {
            "codex@0.154.0"
        } else {
            "claude@2.1.261"
        };
        let text = std::fs::read_to_string(fixture(label, n)).unwrap();
        let run = hydra::normalize::read_with(&text, &policies(), &layout).unwrap();
        let derived = hydra::outcome::derive(&run).unwrap();
        assert_eq!(c.state, State::Ended, "{n}");
        assert_eq!(c.outcome.as_deref(), Some(derived.outcome.as_str()), "{n}");
        let expected = if derived.outcome.as_str() == "completed" {
            Lane::EndedOk
        } else {
            Lane::EndedNotOk
        };
        assert_eq!(c.lane(), expected, "{n}: {:?}", c.outcome);
    }
    assert_eq!(card(9).lane(), Lane::EndedOk);
    assert_eq!(card(1).lane(), Lane::EndedNotOk);
    assert_eq!(
        card(9).result_text.as_deref().map(str::is_empty),
        Some(false)
    );
    assert!(card(9).total_tokens.is_some());
    // A refusal is an outcome that is not completed.
    assert_eq!(card(5).lane(), Lane::EndedNotOk);
    // No closing line and a pid from another day: abandoned, and not ok.
    assert_eq!(card(6).state, State::Abandoned);
    assert_eq!(card(6).lane(), Lane::EndedNotOk);
    assert!(card(6).outcome.is_none());

    let running = snapshot.cards.iter().find(|c| c.run_id == live).unwrap();
    assert_eq!(running.state, State::Live);
    assert_eq!(running.lane(), Lane::Live);
    assert_eq!(running.model.as_deref(), Some("gpt-5.5"));

    // The waiting lane is what `hydra approve` with no id lists.
    assert_eq!(snapshot.requests.len(), 1);
    assert_eq!(snapshot.requests[0].request_id, REQUEST);
    // The open run's tail, and a note where the table would be.
    let (tail_id, tail) = snapshot.tail.as_ref().unwrap();
    assert_eq!(tail_id, live);
    assert_eq!(tail.len(), 1);
    assert!(snapshot.goal.is_none());
    assert!(
        snapshot
            .goal_note
            .as_deref()
            .unwrap()
            .contains("no goal log")
    );

    // Every outcome of the nine but `completed` is not ok, whatever the pid.
    for outcome in [
        "failed",
        "timed_out",
        "crashed",
        "gated",
        "auth_failed",
        "skill_collision",
        "mcp_unavailable",
        "rate_limited",
    ] {
        assert_eq!(lane_of(State::Ended, Some(outcome)), Lane::EndedNotOk);
    }
    assert_eq!(lane_of(State::Ended, Some("completed")), Lane::EndedOk);
    assert_eq!(lane_of(State::Ended, None), Lane::EndedNotOk);
    assert_eq!(lane_of(State::Live, None), Lane::Live);
    assert_eq!(lane_of(State::Abandoned, None), Lane::EndedNotOk);

    // Reading again changes nothing on disk: no harvest, no record appended.
    let before: Vec<(PathBuf, u64)> = listing(layout.root());
    let _ = reader.read(None);
    assert_eq!(listing(layout.root()), before);
}

/// Every file under `root` with its length.
fn listing(root: &Path) -> Vec<(PathBuf, u64)> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).unwrap().flatten() {
            let path = entry.path();
            let meta = std::fs::symlink_metadata(&path).unwrap();
            if meta.is_dir() {
                stack.push(path);
            } else {
                out.push((path, meta.len()));
            }
        }
    }
    out.sort();
    out
}

// --- the hit test --------------------------------------------------------

fn card(run_id: &str, provider: &str, state: State, outcome: Option<&str>) -> Card {
    Card {
        run_id: run_id.to_string(),
        provider: Some(provider.to_string()),
        model: None,
        policy: Some("standard".to_string()),
        state,
        outcome: outcome.map(str::to_string),
        total_tokens: Some(1234),
        result_text: Some("done".to_string()),
        note: None,
    }
}

fn fixed_snapshot() -> Snapshot {
    let goal = goal::fold("g1", GOAL_LOG.as_bytes());
    Snapshot {
        goal: Some(goal),
        goal_named: true,
        cards: vec![
            card(
                "01a0e001-0000-7000-8000-000000000001",
                "claude",
                State::Live,
                None,
            ),
            card(
                "01a0e002-0000-7000-8000-000000000002",
                "codex",
                State::Ended,
                Some("completed"),
            ),
            card(
                "01a0e003-0000-7000-8000-000000000003",
                "agy",
                State::Ended,
                Some("completed"),
            ),
            card(
                "01a0e004-0000-7000-8000-000000000004",
                "claude",
                State::Ended,
                Some("gated"),
            ),
        ],
        requests: vec![request()],
        read: true,
        ..Snapshot::default()
    }
}

/// 160 by 40. The table is the left 44 percent, 70 columns, with a border,
/// the goal text and the header above its first row, so task `i` is on
/// screen row `3 + i`. The middle column starts at 70 and is four lanes of
/// ten rows, a border line above each lane's first card.
const AREA: Rect = Rect {
    x: 0,
    y: 0,
    width: 160,
    height: 40,
};

#[test]
fn a_click_selects_the_row_or_opens_the_card_under_it() {
    let mut app = App::new(
        Layout::at("/nonexistent/hydra"),
        false,
        Duration::from_secs(2),
    );
    app.set_snapshot(fixed_snapshot());
    let mut buf = Buffer::empty(AREA);
    app.draw(AREA, &mut buf);

    // What is drawn where the hit test thinks it is.
    assert!(row_of(&buf, 3).contains("t1"), "{}", row_of(&buf, 3));
    assert!(row_of(&buf, 5).contains("t3"), "{}", row_of(&buf, 5));
    assert!(screen_text(&buf).contains("goal g1 |"));

    // A click on the third task's row selects it, wherever on the row.
    assert_eq!(app.handle(&click(40, 5)), Effect::None);
    assert_eq!(app.focus, Focus::Table);
    assert_eq!(app.table_selected, 2);
    app.handle(&click(1, 6));
    assert_eq!(app.table_selected, 3);
    // Below the last row is the panel, not a row.
    app.handle(&click(10, 20));
    assert_eq!(app.table_selected, 3);

    // The ended-ok lane is the third: rows 20 to 29, cards from row 21.
    let lanes = app.hits.lanes;
    assert_eq!(lanes[Lane::EndedOk.index()].y, 20);
    let (x, y) = (80, 22);
    assert!(row_of(&buf, y).contains("01a0e003"), "{}", row_of(&buf, y));
    assert_eq!(app.handle(&click(x, y)), Effect::Refresh);
    assert_eq!(app.focus, Focus::Lane(Lane::EndedOk));
    assert_eq!(
        app.opened,
        Some(Opened::Run(
            "01a0e003-0000-7000-8000-000000000003".to_string()
        ))
    );
    app.draw(AREA, &mut buf);
    let screen = screen_text(&buf);
    assert!(screen.contains("tokens 1234"), "{screen}");
    assert!(screen.contains("ended ok 2"), "{screen}");

    // Keys: Esc closes, Tab moves on, Enter on a table row opens its run.
    assert_eq!(app.handle(&key(KeyCode::Esc)), Effect::Refresh);
    assert_eq!(app.opened, None);
    app.handle(&click(1, 3));
    app.handle(&key(KeyCode::Enter));
    assert_eq!(
        app.opened,
        Some(Opened::Run(
            "01a0aaaa-0000-7000-8000-000000000001".to_string()
        ))
    );
    app.handle(&key(KeyCode::Tab));
    assert_eq!(app.focus, Focus::Lane(Lane::Live));
    app.handle(&key(KeyCode::BackTab));
    assert_eq!(app.focus, Focus::Table);
    app.handle(&key(KeyCode::Down));
    assert_eq!(app.table_selected, 1);

    // The wheel over the detail scrolls it and moves no selection.
    app.draw(AREA, &mut buf);
    app.handle(&mouse(MouseEventKind::ScrollDown, 150, 10));
    assert_eq!(app.table_selected, 1);
    assert_eq!(app.focus, Focus::Table);

    assert_eq!(app.handle(&key(KeyCode::Char('q'))), Effect::Quit);
}

// --- the one write -------------------------------------------------------

#[test]
fn approving_in_the_cockpit_writes_what_hydra_approve_writes() {
    // Two stores alike. One is approved from the cockpit, the other by the
    // function `hydra approve <id>` calls, and the two are compared after.
    let cockpit = scratch("approve-cockpit");
    let command = scratch("approve-command");
    plant_request(&cockpit);
    plant_request(&command);

    let mut app = App::new(Layout::at(cockpit.root()), false, Duration::from_secs(2));
    let mut reader = Reader::new(
        Layout::at(cockpit.root()),
        Ok(policies()),
        cockpit.root().join("puppets"),
        None,
    );
    app.set_snapshot(reader.read(None));
    let mut buf = Buffer::empty(AREA);
    app.draw(AREA, &mut buf);

    // The waiting lane is the second: rows 10 to 19, its first card on 11.
    app.handle(&click(80, 11));
    assert_eq!(app.focus, Focus::Lane(Lane::Waiting));
    assert_eq!(app.opened, Some(Opened::Request(REQUEST.to_string())));
    app.handle(&key(KeyCode::Char('a')));
    let shown = app.confirming.clone().expect("no confirmation");
    assert_eq!(shown.request_id, REQUEST);
    app.draw(AREA, &mut buf);
    let screen = screen_text(&buf);
    assert!(screen.contains("approve this request?"), "{screen}");
    assert!(
        screen.contains(&format!("hydra approve {REQUEST}")),
        "{screen}"
    );
    // The confirmation holds the keyboard: `q` does not quit under it.
    assert_eq!(app.handle(&key(KeyCode::Char('q'))), Effect::None);

    let pending = cockpit.approvals().join(format!("{REQUEST}.pending.json"));
    assert!(pending.exists(), "approved before `y`");
    assert_eq!(app.handle(&key(KeyCode::Char('y'))), Effect::Refresh);
    assert!(app.confirming.is_none());
    let said = app.message.clone().unwrap();
    assert_eq!(
        said,
        format!("approved {REQUEST}: session {SESSION} launches it now")
    );

    let approved = hydra::approve::approve(&command, REQUEST).unwrap();
    assert_eq!(approved.request_id, REQUEST);

    let relative = |layout: &Layout| -> Vec<(PathBuf, Vec<u8>)> {
        listing(layout.root())
            .into_iter()
            .map(|(path, _)| {
                let bytes = std::fs::read(&path).unwrap();
                (
                    path.strip_prefix(layout.root()).unwrap().to_path_buf(),
                    bytes,
                )
            })
            .collect()
    };
    assert_eq!(relative(&cockpit), relative(&command));
    assert!(!pending.exists());
    assert!(
        cockpit
            .approvals()
            .join(format!("{REQUEST}.approved.json"))
            .exists()
    );

    // A second `a` finds nothing waiting once the reader has looked again.
    app.set_snapshot(reader.read(None));
    assert!(app.snapshot.requests.is_empty());
}

#[test]
fn inside_a_seatbelt_the_cockpit_refuses_by_name_and_writes_nothing() {
    let layout = scratch("approve-seatbelt");
    plant_request(&layout);
    let before = listing(layout.root());

    let mut app = App::new(Layout::at(layout.root()), true, Duration::from_secs(2));
    let mut snapshot = fixed_snapshot();
    snapshot.requests = hydra::approve::pending(&layout).unwrap();
    app.set_snapshot(snapshot);
    let mut buf = Buffer::empty(AREA);
    app.draw(AREA, &mut buf);

    app.handle(&click(80, 11));
    app.handle(&key(KeyCode::Char('a')));
    assert!(
        app.confirming.is_none(),
        "a confirmation opened in a seatbelt"
    );
    let said = app.message.clone().unwrap();
    assert!(said.contains(hydra::approve::INSIDE_SEATBELT), "{said}");
    app.handle(&key(KeyCode::Char('y')));
    app.draw(AREA, &mut buf);
    assert!(screen_text(&buf).contains(hydra::approve::INSIDE_SEATBELT));
    assert_eq!(listing(layout.root()), before);
}

#[test]
fn a_goal_id_cannot_name_a_path() {
    for bad in ["", "../runs/x", "a/b", ".hidden"] {
        assert!(hydra::cockpit::check_goal_id(bad).is_err(), "{bad:?}");
    }
    hydra::cockpit::check_goal_id("g-2026-09-29_1.a").unwrap();
}
