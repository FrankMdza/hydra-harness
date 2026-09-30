//! `hydra cockpit`, build tickets 150 and 151.
//!
//! No terminal anywhere. The chat of build 151 runs on a real pty, but its
//! child is a `/bin/sh` stub and its screen is drawn into a `Buffer`. The fold reads a goal log written here, the lanes read
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
use hydra::cockpit::snapshot::{Card, Lane, Reader, Snapshot, Spend, lane_of, spend_text};
use hydra::cockpit::window::Budgets;
use hydra::live::State;
use hydra::normalize::Counts;
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
{"seq": 3, "ts": "2026-09-29T10:00:02.000Z", "event": "task", "id": "t1", "text": "fold", "class": "rust_off_launch_path", "grade": "medium", "done": "the fold test passes", "deps": [], "paths": []}
{"seq": 4, "ts": "2026-09-29T10:00:03.000Z", "event": "task", "id": "t2", "text": "lanes", "class": "rust_launch_path", "grade": "high", "deps": ["t1"], "paths": []}
{"seq": 5, "ts": "2026-09-29T10:00:04.000Z", "event": "task", "id": "t3", "text": "docs", "class": "docs", "grade": "low", "done": "a first definition", "deps": [], "paths": []}
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
        done: None,
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
            TaskRow {
                done: Some("the fold test passes".to_string()),
                ..row(
                    "t1",
                    "rust_off_launch_path",
                    "medium",
                    Some(("claude", Some("sonnet"))),
                    "applied",
                    Some("01a0aaaa-0000-7000-8000-000000000001"),
                )
            },
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
            // A repeated `task` resets the row in place, assignment included,
            // and one with no `done` leaves none, as `event.get("done")` does.
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
    write_live_run_on(layout, run_id, None);
}

/// The same, launched under the policy set whose sha is `policy_sha`.
fn write_live_run_on(layout: &Layout, run_id: &str, policy_sha: Option<&str>) {
    write_live_seat(layout, run_id, None, policy_sha);
}

/// The same, launched by `parent`.
fn write_live_seat(layout: &Layout, run_id: &str, parent: Option<&str>, policy_sha: Option<&str>) {
    let pid = std::process::id();
    let start = hydra::procinfo::start_time(pid).unwrap().unwrap();
    let line = json!({
        "run_id": run_id,
        "parent_run_id": parent,
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
            "policy_sha": policy_sha,
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
        policy_sha: None,
        parent_run_id: None,
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

/// Four tasks whose runs ended: one the snapshot knows failed, one the human
/// judged, one whose record the snapshot does not hold, and one never run
/// with a `done` longer than any screen.
const ENDED_LOG: &str = r#"{"event": "goal", "text": "the cockpit redesign", "repo": "/r", "base_sha": "abc"}
{"event": "task", "id": "a", "text": "x", "class": "docs", "grade": "low", "done": "the table shows done", "deps": [], "paths": []}
{"event": "task", "id": "b", "text": "x", "class": "docs", "grade": "low", "deps": [], "paths": []}
{"event": "task", "id": "c", "text": "x", "class": "docs", "grade": "low", "deps": [], "paths": []}
{"event": "task", "id": "d", "text": "x", "class": "docs", "grade": "low", "done": "LONG-DONE", "deps": [], "paths": []}
{"event": "run", "task": "a", "attempt": 1, "run_id": "01a0e004-0000-7000-8000-000000000004"}
{"event": "run_end", "task": "a", "run_id": "01a0e004-0000-7000-8000-000000000004", "outcome": "gated", "total_tokens": 1}
{"event": "run", "task": "b", "attempt": 1, "run_id": "01a0e002-0000-7000-8000-000000000002"}
{"event": "run_end", "task": "b", "run_id": "01a0e002-0000-7000-8000-000000000002", "outcome": "completed", "total_tokens": 1}
{"event": "human_verdict", "task": "b", "run_id": "01a0e002-0000-7000-8000-000000000002", "result": "rejected", "note": null}
{"event": "run", "task": "c", "attempt": 1, "run_id": "01a0cccc-0000-7000-8000-00000000000c"}
{"event": "run_end", "task": "c", "run_id": "01a0cccc-0000-7000-8000-00000000000c", "outcome": "failed", "total_tokens": 1}
"#;

/// A `done` of 300 characters, ending in a word the detail must still show.
fn long_done() -> String {
    format!("{} the-very-end", "word ".repeat(59))
}

fn ended_snapshot() -> Snapshot {
    let log = ENDED_LOG.replace("LONG-DONE", &long_done());
    Snapshot {
        goal: Some(goal::fold("g2", log.as_bytes())),
        ..fixed_snapshot()
    }
}

#[test]
fn the_table_shows_done_and_an_ended_rows_outcome() {
    let mut app = App::new(
        Layout::at("/nonexistent/hydra"),
        false,
        Duration::from_secs(2),
    );
    app.stacked = true;
    app.set_snapshot(ended_snapshot());
    let mut buf = Buffer::empty(AREA);
    app.draw(AREA, &mut buf);
    let table = app.hits.table;
    let header = row_of(&buf, table.y + 2);
    let done_at = header.find("done").expect(&header);
    let line = |i: u16| row_of(&buf, table.y + 3 + i);

    // An ended row the snapshot knows reads its run's outcome.
    assert!(line(0).contains("gated"), "{}", line(0));
    assert!(!line(0).contains("ended"), "{}", line(0));
    // The human's verdict still wins over a completed run.
    assert!(line(1).contains("rejected"), "{}", line(1));
    assert!(!line(1).contains("completed"), "{}", line(1));
    // A run the snapshot has no card for stays `ended`.
    assert!(line(2).contains("ended"), "{}", line(2));
    // `done` sits under its header, and a long one is cut at the border.
    assert_eq!(line(0).find("the table shows done"), Some(done_at));
    assert_eq!(line(3).find("word word"), Some(done_at), "{}", line(3));
    assert!(!screen_text(&buf).contains("the-very-end"));

    // The detail of a run shows its task's `done` whole.
    app.handle(&click(1, table.y + 3));
    app.handle(&key(KeyCode::Enter));
    app.draw(AREA, &mut buf);
    let detail = screen_text(&buf);
    assert!(detail.contains("task a"), "{detail}");
    assert!(detail.contains("done the table shows done"), "{detail}");

    // A long `done` wraps in the detail rather than being cut. Task `d` never
    // ran, so the detail of a run that was its is built by hand here.
    let mut snapshot = ended_snapshot();
    let goal = snapshot.goal.as_mut().unwrap();
    goal.rows[3].run_id = Some("01a0e003-0000-7000-8000-000000000003".to_string());
    app.set_snapshot(snapshot);
    app.handle(&click(1, table.y + 6));
    app.handle(&key(KeyCode::Enter));
    app.draw(AREA, &mut buf);
    assert!(screen_text(&buf).contains("the-very-end"));
}

/// The table panel drawn stacked, so its inner width is `inner`. Returns the
/// header's words, and the header and the first three rows inside the border.
fn table_at(inner: u16) -> (Vec<String>, Vec<String>) {
    let mut app = App::new(
        Layout::at("/nonexistent/hydra"),
        false,
        Duration::from_secs(2),
    );
    app.stacked = true;
    app.set_snapshot(ended_snapshot());
    let area = Rect::new(0, 0, inner + 2, 40);
    let mut buf = Buffer::empty(area);
    app.draw(area, &mut buf);
    let table = app.hits.table;
    let inside = |y: u16| -> String {
        (table.x + 1..table.x + table.width - 1)
            .map(|x| buf[(x, y)].symbol().to_string())
            .collect()
    };
    let header = inside(table.y + 2);
    let words = header.split_whitespace().map(str::to_string).collect();
    let lines = (0..4).map(|i| inside(table.y + 2 + i)).collect();
    (words, lines)
}

/// Task `a`'s status and `done` sit under their headers at `inner` columns.
fn aligned(inner: u16) {
    let (_, lines) = table_at(inner);
    let (header, first) = (&lines[0], &lines[1]);
    let screen = lines.join("\n");
    assert_eq!(first.find("gated"), header.find("status"), "{screen}");
    assert_eq!(first.find("the table"), header.find("done"), "{screen}");
    assert!(first.chars().count() <= inner as usize, "{screen}");
}

#[test]
fn beside_the_chat_the_table_keeps_id_status_and_done() {
    let (words, lines) = table_at(52);
    assert_eq!(
        words,
        ["id", "status", "provider", "done"],
        "{}",
        lines.join("\n")
    );
    aligned(52);
}

#[test]
fn at_eighty_columns_the_table_drops_class_first() {
    let (words, lines) = table_at(80);
    assert_eq!(
        words,
        ["id", "status", "provider", "model", "run", "grade", "done"],
        "{}",
        lines.join("\n")
    );
    aligned(80);
}

#[test]
fn class_shrinks_to_twelve_before_it_goes() {
    let (words, lines) = table_at(95);
    let screen = lines.join("\n");
    assert_eq!(
        words,
        [
            "id", "status", "provider", "model", "run", "grade", "class", "done"
        ],
        "{screen}"
    );
    // Task `a`'s class is `docs`; the header's `class` column is 12 wide.
    let header = &lines[0];
    let class_at = header.find("class").unwrap();
    assert_eq!(header.find("done"), Some(class_at + 13), "{screen}");
    aligned(95);
}

#[test]
fn at_full_width_the_table_shows_all_eight_columns() {
    let (words, lines) = table_at(150);
    let screen = lines.join("\n");
    assert_eq!(
        words,
        [
            "id", "status", "provider", "model", "run", "grade", "class", "done"
        ],
        "{screen}"
    );
    let header = &lines[0];
    let class_at = header.find("class").unwrap();
    assert_eq!(header.find("done"), Some(class_at + 21), "{screen}");
    aligned(150);
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
fn d_then_y_in_the_cockpit_denies_the_selected_request() {
    let layout = scratch("deny-cockpit");
    plant_request(&layout);

    let mut app = App::new(Layout::at(layout.root()), false, Duration::from_secs(2));
    let mut snapshot = fixed_snapshot();
    snapshot.requests = hydra::approve::pending(&layout).unwrap();
    app.set_snapshot(snapshot);
    let mut buf = Buffer::empty(AREA);
    app.draw(AREA, &mut buf);

    app.handle(&click(80, 11));
    app.handle(&key(KeyCode::Char('d')));
    let shown = app.confirming.clone().expect("no confirmation");
    assert_eq!(shown.request_id, REQUEST);
    app.draw(AREA, &mut buf);
    let screen = screen_text(&buf);
    assert!(screen.contains("deny this request?"), "{screen}");
    assert!(screen.contains("y denies"), "{screen}");

    let pending = layout.approvals().join(format!("{REQUEST}.pending.json"));
    let denied = layout.approvals().join(format!("{REQUEST}.denied.json"));
    assert!(pending.exists(), "denied before `y`");
    assert_eq!(app.handle(&key(KeyCode::Char('y'))), Effect::Refresh);
    assert!(app.confirming.is_none());
    assert_eq!(
        app.message.clone().unwrap(),
        format!("denied {REQUEST}: session {SESSION} is told no")
    );
    assert!(!pending.exists());
    let file: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&denied).unwrap()).unwrap();
    assert_eq!(file["request_id"], REQUEST);
    assert!(file["denied_at"].is_string(), "{file}");
    assert!(file["denied_by"].is_string(), "{file}");

    // `n` on a denial's confirmation writes nothing.
    plant_request(&layout);
    app.handle(&key(KeyCode::Char('d')));
    assert!(app.confirming.is_some());
    app.handle(&key(KeyCode::Char('n')));
    assert!(app.confirming.is_none());
    assert!(pending.exists());
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

// --- the mascot ------------------------------------------------------------

use hydra::cockpit::mascot::{self, Frames, Mascot, Pose};

fn committed_frames() -> Frames {
    Frames::load(&mascot::default_dir()).expect("the committed frames load")
}

fn snapshot_of(cards: Vec<Card>, requests: Vec<Pending>) -> Snapshot {
    Snapshot {
        cards,
        requests,
        read: true,
        ..Snapshot::default()
    }
}

fn live_cards(n: usize) -> Vec<Card> {
    (0..n)
        .map(|i| card(&format!("run-live-{i}"), "claude", State::Live, None))
        .collect()
}

#[test]
fn the_committed_frames_load_at_the_pets_size() {
    let frames = committed_frames();
    for pose in Pose::ALL {
        assert_eq!(frames.size(pose), (24, 12), "{}", pose.name());
    }
    // A frame with a dragon in it is not blank.
    let mut buf = Buffer::empty(Rect::new(0, 0, 24, 12));
    frames.draw(Pose::Idle, Rect::new(0, 0, 24, 12), &mut buf);
    assert!(
        (0..12).any(|y| (0..24).any(|x| buf[(x, y)].symbol() != " ")),
        "idle drew nothing"
    );
}

#[test]
fn a_frame_directory_that_lies_about_its_size_is_refused() {
    let dir = scratch("frames-lie").root().join("frames");
    std::fs::create_dir_all(&dir).unwrap();
    let mut poses = serde_json::Map::new();
    for pose in Pose::ALL {
        std::fs::write(dir.join(format!("{}.ans", pose.name())), "\x1b[0m \x1b[K\n").unwrap();
        poses.insert(
            pose.name().to_string(),
            json!({"file": format!("{}.ans", pose.name()), "rows": 12, "cols": 24}),
        );
    }
    std::fs::write(
        dir.join("index.json"),
        json!({ "poses": poses }).to_string(),
    )
    .unwrap();
    let err = format!("{:#}", Frames::load(&dir).expect_err("refused"));
    assert!(err.contains("rows"), "{err}");
}

#[test]
fn heads_awake_follow_the_live_seats() {
    let mut mascot = Mascot::new(committed_frames());
    let cases = [
        (0, Pose::Idle, 0),
        (1, Pose::TwoAsleep, 0),
        (2, Pose::OneAsleep, 0),
        (3, Pose::Working, 0),
        (5, Pose::Working, 2),
    ];
    for (live, pose, extra) in cases {
        mascot.observe(&snapshot_of(live_cards(live), vec![]));
        assert_eq!(mascot.pose(), pose, "{live} live");
        assert_eq!(mascot.extra(), extra, "{live} live");
    }
}

#[test]
fn a_seat_that_ends_flashes_once_and_history_does_not() {
    let mut mascot = Mascot::new(committed_frames());
    // Runs that ended before the cockpit opened raise nothing.
    let old = vec![
        card("old-gated", "codex", State::Ended, Some("gated")),
        card("old-bad", "codex", State::Ended, Some("failed")),
    ];
    mascot.observe(&snapshot_of(old.clone(), vec![]));
    assert_eq!(mascot.pose(), Pose::Idle);

    let mut cards = live_cards(2);
    mascot.observe(&snapshot_of(cards.clone(), vec![]));
    assert_eq!(mascot.pose(), Pose::OneAsleep);

    // One seat ends gated: the refused pose for this refresh only.
    cards[0] = card("run-live-0", "claude", State::Ended, Some("gated"));
    mascot.observe(&snapshot_of(cards.clone(), vec![]));
    assert_eq!(mascot.pose(), Pose::Refused);
    mascot.observe(&snapshot_of(cards.clone(), vec![]));
    assert_eq!(mascot.pose(), Pose::TwoAsleep);

    // The other ends not ok and not gated: the meme head.
    cards[1] = card("run-live-1", "claude", State::Ended, Some("timeout"));
    mascot.observe(&snapshot_of(cards.clone(), vec![]));
    assert_eq!(mascot.pose(), Pose::Meme);
    mascot.observe(&snapshot_of(cards, vec![]));
    assert_eq!(mascot.pose(), Pose::Idle);

    // A seat that ends ok changes the head count and nothing else.
    let mut ok = live_cards(1);
    mascot.observe(&snapshot_of(ok.clone(), vec![]));
    ok[0] = card("run-live-0", "claude", State::Ended, Some("completed"));
    mascot.observe(&snapshot_of(ok, vec![]));
    assert_eq!(mascot.pose(), Pose::Idle);
}

#[test]
fn a_request_that_leaves_the_waiting_lane_breathes_fire_once() {
    let mut mascot = Mascot::new(committed_frames());
    mascot.observe(&snapshot_of(vec![], vec![request()]));
    assert_eq!(mascot.pose(), Pose::Idle);
    mascot.observe(&snapshot_of(live_cards(1), vec![]));
    assert_eq!(mascot.pose(), Pose::Fire);
    mascot.observe(&snapshot_of(live_cards(1), vec![]));
    assert_eq!(mascot.pose(), Pose::TwoAsleep);
}

// --- the session's seats, build ticket 177 ---------------------------------

const WATCHED: &str = "01a0aaaa-0000-7000-8000-00000000000a";
const SEAT: &str = "01a0bbbb-0000-7000-8000-00000000000b";

/// A card launched by `parent`.
fn seat(run_id: &str, parent: &str, state: State, outcome: Option<&str>) -> Card {
    Card {
        parent_run_id: Some(parent.to_string()),
        ..card(run_id, "codex", state, outcome)
    }
}

#[test]
fn a_card_carries_the_run_that_launched_it() {
    let layout = scratch("parent");
    write_live_run(&layout, WATCHED);
    write_live_seat(&layout, SEAT, Some(WATCHED), None);
    let mut reader = Reader::new(
        Layout::at(layout.root()),
        Ok(policies()),
        layout.root().join("puppets"),
        None,
    );
    let snapshot = reader.read(None);
    assert!(snapshot.errors.is_empty(), "{:?}", snapshot.errors);
    let parent_of = |id: &str| {
        snapshot
            .cards
            .iter()
            .find(|c| c.run_id == id)
            .unwrap_or_else(|| panic!("no card for {id}"))
            .parent_run_id
            .clone()
    };
    assert_eq!(parent_of(WATCHED), None);
    assert_eq!(parent_of(SEAT).as_deref(), Some(WATCHED));
}

/// The session, `seats` live runs it launched, and three live runs of nobody's.
fn session_cards(seats: usize) -> Vec<Card> {
    let mut cards = vec![card(WATCHED, "claude", State::Live, None)];
    cards.extend((0..seats).map(|i| seat(&format!("seat-{i}"), WATCHED, State::Live, None)));
    cards.extend(live_cards(3));
    cards
}

#[test]
fn with_a_session_the_heads_count_its_seats_and_itself() {
    let mut mascot = Mascot::new(committed_frames());
    mascot.observe(&snapshot_of(session_cards(1), vec![]));
    assert_eq!(mascot.live(), 5, "without a session every live run counts");

    mascot.watch(WATCHED);
    let cases = [
        (0, 1, Pose::TwoAsleep, 0),
        (1, 2, Pose::OneAsleep, 0),
        (4, 5, Pose::Working, 2),
    ];
    for (seats, live, pose, extra) in cases {
        mascot.observe(&snapshot_of(session_cards(seats), vec![]));
        assert_eq!(mascot.live(), live, "{seats} seat(s)");
        assert_eq!(mascot.pose(), pose, "{seats} seat(s)");
        assert_eq!(mascot.extra(), extra, "{seats} seat(s)");
    }

    // The header's `+N` and seat count say the same.
    let mut app = App::new(scratch("session-heads"), true, Duration::from_secs(2));
    let mut watching = Mascot::new(committed_frames());
    watching.watch(WATCHED);
    app.set_mascot(Some(watching));
    let mut snapshot = fixed_snapshot();
    snapshot.cards = session_cards(4);
    app.set_snapshot(snapshot);
    let mut buf = Buffer::empty(AREA);
    app.draw(AREA, &mut buf);
    let header = (0..12).map(|y| row_of(&buf, y)).collect::<String>();
    assert!(header.contains("+2 more"), "{header}");
    assert!(header.contains("5 live seat(s)"), "{header}");
}

#[test]
fn a_session_seat_that_ends_not_ok_is_named_for_one_refresh() {
    let mut app = App::new(scratch("session-flash"), true, Duration::from_secs(2));
    let mut watching = Mascot::new(committed_frames());
    watching.watch(WATCHED);
    app.set_mascot(Some(watching));
    let header = |app: &mut App, cards: Vec<Card>| {
        let mut snapshot = fixed_snapshot();
        snapshot.cards = cards;
        app.set_snapshot(snapshot);
        let mut buf = Buffer::empty(AREA);
        app.draw(AREA, &mut buf);
        (0..12).map(|y| row_of(&buf, y)).collect::<String>()
    };

    let mut cards = vec![
        card(WATCHED, "claude", State::Live, None),
        seat(SEAT, WATCHED, State::Live, None),
        card("run-other", "codex", State::Live, None),
    ];
    header(&mut app, cards.clone());

    // A run outside the session ends not ok: no flash, no name.
    cards[2] = card("run-other", "codex", State::Ended, Some("failed"));
    let text = header(&mut app, cards.clone());
    assert!(text.contains("one_asleep"), "{text}");
    assert!(!text.contains("failed"), "{text}");

    // The session's seat ends gated: refused, and its short id with the outcome.
    cards[1] = seat(SEAT, WATCHED, State::Ended, Some("gated"));
    let text = header(&mut app, cards.clone());
    assert!(text.contains("refused"), "{text}");
    assert!(text.contains("01a0bbbb gated"), "{text}");

    // One refresh only.
    let text = header(&mut app, cards);
    assert!(text.contains("two_asleep"), "{text}");
    assert!(!text.contains("01a0bbbb"), "{text}");
}

#[test]
fn without_a_session_an_ended_run_is_not_named() {
    let mut app = App::new(scratch("no-session-flash"), true, Duration::from_secs(2));
    app.set_mascot(Some(Mascot::new(committed_frames())));
    let mut snapshot = fixed_snapshot();
    snapshot.cards = vec![card(SEAT, "codex", State::Live, None)];
    app.set_snapshot(snapshot.clone());
    snapshot.cards = vec![card(SEAT, "codex", State::Ended, Some("timeout"))];
    app.set_snapshot(snapshot);
    let mut buf = Buffer::empty(AREA);
    app.draw(AREA, &mut buf);
    let text = (0..12).map(|y| row_of(&buf, y)).collect::<String>();
    assert!(text.contains("meme"), "{text}");
    assert!(!text.contains("01a0bbbb"), "{text}");
}

fn dragon_cells(buf: &Buffer, area: Rect) -> usize {
    (area.y..area.y + area.height)
        .flat_map(|y| (area.x..area.x + area.width).map(move |x| (x, y)))
        .filter(|pos| buf[*pos].symbol() != " ")
        .count()
}

#[test]
fn the_header_draws_the_dragon_left_of_the_title_and_no_mascot_gives_the_row_back() {
    let mut with = App::new(scratch("mascot"), true, Duration::from_secs(2));
    with.set_mascot(Some(Mascot::new(committed_frames())));
    let mut snapshot = fixed_snapshot();
    snapshot.cards = live_cards(5);
    with.set_snapshot(snapshot.clone());
    let mut buf = Buffer::empty(AREA);
    with.draw(AREA, &mut buf);

    // 24 columns by 12 rows at the top left, and the title beside them.
    assert!(dragon_cells(&buf, Rect::new(0, 0, 24, 12)) > 20);
    assert!(row_of(&buf, 1).contains("+2"), "{}", row_of(&buf, 1));
    let title = (0..12).map(|y| row_of(&buf, y)).collect::<String>();
    assert!(title.contains("hydra cockpit"), "{title}");
    let table_with = with.hits.table;
    assert_eq!(table_with.y, 12);

    let mut without = App::new(scratch("no-mascot"), true, Duration::from_secs(2));
    without.set_snapshot(snapshot);
    let mut plain = Buffer::empty(AREA);
    without.draw(AREA, &mut plain);
    assert_eq!(without.hits.table.y, 0);
    assert_eq!(without.hits.table.height, table_with.height + 12);
    assert!(!screen_text(&plain).contains("hydra cockpit"));
}

#[test]
fn a_screen_too_small_for_the_dragon_hides_it() {
    let small = Rect::new(0, 0, 100, 20);
    let mut app = App::new(scratch("mascot-small"), true, Duration::from_secs(2));
    app.set_mascot(Some(Mascot::new(committed_frames())));
    app.set_snapshot(fixed_snapshot());
    let mut buf = Buffer::empty(small);
    app.draw(small, &mut buf);
    assert_eq!(app.hits.table.y, 0);
}

// --- the chat inside, build ticket 151 ---------------------------------------

use hydra::cockpit::chat::{Chat, Launch};
use hydra::cockpit::host::{Host, Side};

/// A stub child on `/bin/sh`, as `pty_tee.rs` drives one: `script` runs with
/// `RUN` naming a run id, and nothing it does reaches a provider.
fn stub(layout: &Layout, script: &str, run_id: &str, area: Rect) -> Host {
    let launch = Launch {
        program: "/bin/sh".to_string(),
        args: vec!["-c".to_string(), script.to_string()],
        cwd: layout.root().to_path_buf(),
        env: vec![
            ("PATH".to_string(), "/usr/bin:/bin".to_string()),
            ("TERM".to_string(), "xterm-256color".to_string()),
            ("RUN".to_string(), run_id.to_string()),
        ],
        layout: Layout::at(layout.root()),
        policies: Ok(policies()),
        harvest_home: None,
    };
    let (rows, cols) = Host::chat_size(area);
    let chat = Chat::spawn(launch, rows, cols).unwrap();
    let app = App::new(Layout::at(layout.root()), false, Duration::from_secs(2));
    Host::with_chat(app, "stub".to_string(), chat)
}

/// Pumps and draws until `done` says so, for five seconds at most.
fn until(host: &mut Host, area: Rect, what: &str, done: impl Fn(&Host, &str) -> bool) -> Buffer {
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    loop {
        host.pump();
        let mut buf = Buffer::empty(area);
        host.draw(area, &mut buf);
        let text = chat_text(&buf, area);
        if done(host, &text) {
            return buf;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "waited five seconds for {what}; the panel shows:\n{text}"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// The text inside the chat panel's border, one line per row.
fn chat_text(buf: &Buffer, area: Rect) -> String {
    let (chat, _, _) = Host::areas(area);
    (chat.y + 1..chat.y + chat.height - 1)
        .map(|y| {
            (chat.x + 1..chat.x + chat.width - 1)
                .map(|x| buf[(x, y)].symbol().to_string())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

const WIDE: Rect = Rect::new(0, 0, 120, 30);

/// A child that says `ready`, reads one line, prints it back in brackets and
/// waits for another.
const ECHO: &str = r#"printf 'ready\n'; read line; printf 'got <%s>\n' "$line"; read again"#;

#[test]
fn the_chat_panel_shows_what_the_child_printed_with_its_colours() {
    let layout = scratch("chat-shows");
    let mut host = stub(
        &layout,
        r#"printf 'plain then \033[31mred words\033[0m\n'; read line"#,
        "",
        WIDE,
    );
    let buf = until(&mut host, WIDE, "the stub's line", |_, text| {
        text.contains("plain then red words")
    });
    let (chat, _, _) = Host::areas(WIDE);
    let (x, y) = (chat.x + 1 + "plain then ".len() as u16, chat.y + 1);
    assert_eq!(buf[(x, y)].symbol(), "r");
    assert_eq!(buf[(x, y)].fg, ratatui::style::Color::Indexed(1));
    assert_eq!(buf[(chat.x + 1, y)].fg, ratatui::style::Color::Reset);
    assert_eq!(host.side, Side::Chat);
    host.handle(&key(KeyCode::Enter));
}

#[test]
fn a_key_with_the_chat_focused_reaches_the_child_unchanged() {
    let layout = scratch("chat-key");
    let mut host = stub(&layout, ECHO, "", WIDE);
    until(&mut host, WIDE, "ready", |_, text| text.contains("ready"));
    // `q` and `a` are build 150's quit and approve; with the chat focused
    // they are two letters the child reads.
    for code in [KeyCode::Char('q'), KeyCode::Char('a'), KeyCode::Enter] {
        assert_eq!(host.handle(&key(code)), Effect::None);
    }
    until(&mut host, WIDE, "got <qa>", |_, text| {
        text.contains("got <qa>")
    });
    assert!(!host.asking);
    host.handle(&key(KeyCode::Enter));
}

#[test]
fn a_key_with_the_cockpit_focused_does_not_reach_the_child() {
    let layout = scratch("chat-no-key");
    let mut host = stub(&layout, ECHO, "", WIDE);
    until(&mut host, WIDE, "ready", |_, text| text.contains("ready"));
    host.handle(&key(KeyCode::F(2)));
    assert_eq!(host.side, Side::Cockpit);
    let buf = until(&mut host, WIDE, "the status line", |_, _| true);
    let (_, _, status) = Host::areas(WIDE);
    assert!(row_of(&buf, status.y).contains("the cockpit has the keys"));
    for code in [KeyCode::Char('x'), KeyCode::Down, KeyCode::Enter] {
        host.handle(&key(code));
    }
    host.handle(&key(KeyCode::F(2)));
    assert_eq!(host.side, Side::Chat);
    for code in [KeyCode::Char('y'), KeyCode::Enter] {
        host.handle(&key(code));
    }
    let buf = until(&mut host, WIDE, "a line back", |_, text| {
        text.contains("got <")
    });
    let text = chat_text(&buf, WIDE);
    assert!(text.contains("got <y>"), "{text}");
    assert!(row_of(&buf, status.y).contains("the chat has the keys"));
    host.handle(&key(KeyCode::Enter));
}

#[test]
fn a_resize_reaches_the_child() {
    let layout = scratch("chat-resize");
    let mut host = stub(
        &layout,
        r#"printf 'first %s\n' "$(stty size)"; read line; printf 'second %s\n' "$(stty size)"; read again"#,
        "",
        WIDE,
    );
    let (rows, cols) = Host::chat_size(WIDE);
    let first = format!("first {rows} {cols}");
    until(&mut host, WIDE, &first, |_, text| text.contains(&first));

    let taller = Rect::new(0, 0, 150, 44);
    let (rows, cols) = Host::chat_size(taller);
    assert_ne!((rows, cols), Host::chat_size(WIDE));
    let mut buf = Buffer::empty(taller);
    host.draw(taller, &mut buf);
    host.handle(&key(KeyCode::Enter));
    let second = format!("second {rows} {cols}");
    until(&mut host, taller, &second, |_, text| text.contains(&second));
    assert_eq!(host.chat().unwrap().size(), (rows, cols));
    host.handle(&key(KeyCode::Enter));
}

#[test]
fn the_childs_exit_puts_its_run_id_and_outcome_on_the_line() {
    let layout = scratch("chat-exit");
    let from = fixture("claude@2.1.261", 1);
    let run_id = from.file_stem().unwrap().to_str().unwrap().to_string();
    std::fs::copy(&from, layout.run_file(&run_id)).unwrap();
    let text = std::fs::read_to_string(&from).unwrap();
    let run = hydra::normalize::read_with(&text, &policies(), &layout).unwrap();
    let outcome = hydra::outcome::derive(&run)
        .unwrap()
        .outcome
        .as_str()
        .to_string();

    let mut host = stub(
        &layout,
        r#"printf 'hydra: run %s\n' "$RUN" >&2; printf 'the last screen\n'; exit 0"#,
        &run_id,
        WIDE,
    );
    let buf = until(&mut host, WIDE, "the child's exit", |host, _| {
        host.chat().unwrap().finished()
    });
    let chat = host.chat().unwrap();
    assert_eq!(chat.run_id(), Some(run_id.as_str()));
    let line = format!("run {run_id}: {outcome}; hydra exited 0");
    assert_eq!(chat.ended_line(), Some(line.as_str()));

    // The last screen stays, and the line is on the panel's bottom border.
    let (area, _, _) = Host::areas(WIDE);
    assert!(chat_text(&buf, WIDE).contains("the last screen"));
    assert!(
        row_of(&buf, area.y + area.height - 1).contains(&format!("run {run_id}: {outcome}")),
        "{}",
        screen_text(&buf)
    );
    // The cockpit stays open, and Ctrl-q now quits without asking.
    assert!(!host.done());
    let quit = Event::Key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::CONTROL));
    assert_eq!(host.handle(&quit), Effect::Quit);
}

#[test]
fn the_dragon_watches_the_run_the_chat_names() {
    let layout = scratch("chat-watch");
    let mut host = stub(
        &layout,
        r#"printf 'hydra: run %s\n' "$RUN" >&2; exit 0"#,
        WATCHED,
        WIDE,
    );
    host.app.set_mascot(Some(Mascot::new(committed_frames())));
    until(&mut host, WIDE, "the run id", |host, _| {
        host.chat().unwrap().run_id().is_some()
    });
    host.app.set_snapshot(snapshot_of(session_cards(1), vec![]));
    assert_eq!(host.app.mascot().unwrap().live(), 2);
}

#[test]
fn ctrl_q_asks_while_the_child_lives_and_y_hangs_it_up() {
    let layout = scratch("chat-hangup");
    let mut host = stub(
        &layout,
        r#"trap 'exit 7' HUP; printf 'ready\n'; while :; do sleep 1; done"#,
        "",
        WIDE,
    );
    until(&mut host, WIDE, "ready", |_, text| text.contains("ready"));
    let quit = Event::Key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::CONTROL));
    assert_eq!(host.handle(&quit), Effect::None);
    assert!(host.asking);
    // Any key but `y` keeps it running.
    assert_eq!(host.handle(&key(KeyCode::Char('n'))), Effect::None);
    assert!(!host.asking && !host.hanging_up);

    assert_eq!(host.handle(&quit), Effect::None);
    assert_eq!(host.handle(&key(KeyCode::Char('y'))), Effect::None);
    assert!(host.hanging_up);
    until(&mut host, WIDE, "the hangup", |host, _| host.done());
    let line = host.chat().unwrap().ended_line().unwrap();
    assert!(line.starts_with("the chat "), "{line}");
}

// --- the policy pane, build ticket 171 -------------------------------------

/// A copy of the shipped policy set under a scratch root, so an editor stub
/// can change it without touching the real one.
fn scratch_policies(layout: &Layout) -> PathBuf {
    let from = Path::new(env!("CARGO_MANIFEST_DIR")).join("policies");
    let to = layout.root().join("policies");
    std::fs::create_dir_all(&to).unwrap();
    for entry in std::fs::read_dir(&from).unwrap().flatten() {
        std::fs::copy(entry.path(), to.join(entry.file_name())).unwrap();
    }
    to
}

/// An `EDITOR` that appends `text` to the file it is handed.
fn editor_stub(layout: &Layout, text: &str) -> String {
    let path = layout.root().join("editor");
    std::fs::write(
        &path,
        format!("#!/bin/sh\nprintf '%s' '{text}' >> \"$1\"\n"),
    )
    .unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    path.display().to_string()
}

const PANE_AREA: Rect = Rect::new(0, 0, 200, 50);

fn draw_wide(app: &mut App) -> Buffer {
    let mut buf = Buffer::empty(PANE_AREA);
    app.draw(PANE_AREA, &mut buf);
    buf
}

/// The screen row that holds `needle`, whole.
fn line_with(buf: &Buffer, needle: &str) -> String {
    (0..buf.area.height)
        .map(|y| row_of(buf, y))
        .find(|row| row.contains(needle))
        .unwrap_or_else(|| panic!("no row holds {needle:?}:\n{}", screen_text(buf)))
}

#[test]
fn the_policy_pane_lists_the_five_and_marks_a_run_on_another_sha() {
    let layout = scratch("policy-pane");
    let dir = scratch_policies(&layout);
    let current = PolicySet::load(&dir).unwrap().sha().to_string();
    let same = "01a0f001-0000-7000-8000-000000000001";
    let other = "01a0f002-0000-7000-8000-000000000002";
    write_live_run_on(&layout, same, Some(&current));
    write_live_run_on(&layout, other, Some(&"f1".repeat(32)));

    let mut reader = Reader::new(
        Layout::at(layout.root()),
        Ok(policies()),
        layout.root().join("puppets"),
        None,
    );
    let snapshot = reader.read(None);
    assert!(snapshot.errors.is_empty(), "{:?}", snapshot.errors);

    let mut app = App::new(Layout::at(layout.root()), false, Duration::from_secs(2));
    app.set_policies(dir);
    app.set_snapshot(snapshot);
    let before = screen_text(&draw_wide(&mut app));
    assert!(!before.contains(&current[..12]), "the pane is shut until p");

    assert_eq!(app.handle(&key(KeyCode::Char('p'))), Effect::None);
    let buf = draw_wide(&mut app);
    let text = screen_text(&buf);
    for name in hydra::policy::NAMES {
        assert!(
            text.contains(&format!("{name}.json")),
            "{name} missing:\n{text}"
        );
    }
    assert!(line_with(&buf, "policy set").contains(&current[..12]));
    let same_line = line_with(&buf, &same[..8]);
    let other_line = line_with(&buf, &other[..8]);
    assert!(same_line.contains(&current[..12]), "{same_line}");
    assert!(!same_line.contains("differs"), "{same_line}");
    assert!(other_line.contains(&"f1".repeat(6)), "{other_line}");
    assert!(other_line.contains("differs"), "{other_line}");

    // Read-only: nothing under policies/ changed by opening the pane.
    assert_eq!(
        PolicySet::load(layout.root().join("policies"))
            .unwrap()
            .sha(),
        current
    );

    assert_eq!(app.handle(&key(KeyCode::Esc)), Effect::None);
    assert!(!screen_text(&draw_wide(&mut app)).contains("policy set"));
}

/// `e` in the pane, the editor run the way the loop runs it, and the reload.
fn edit_standard(app: &mut App, editor: &str) -> PathBuf {
    app.handle(&key(KeyCode::Char('p')));
    // judge, curate, read-only, standard.
    for _ in 0..3 {
        app.handle(&key(KeyCode::Down));
    }
    let Effect::Edit(file) = app.handle(&key(KeyCode::Char('e'))) else {
        panic!("e on a policy asks the loop to open the editor");
    };
    let ran = hydra::cockpit::run_editor(editor, &file);
    app.edited(&file, ran);
    file
}

#[test]
fn e_runs_the_editor_on_the_selected_policy_and_the_pane_shows_the_new_sha() {
    let layout = scratch("policy-edit");
    let dir = scratch_policies(&layout);
    let old = PolicySet::load(&dir).unwrap().sha().to_string();
    let mut app = App::new(Layout::at(layout.root()), false, Duration::from_secs(2));
    app.set_policies(dir.clone());

    // JSON has no comments, so the stub that leaves a loadable file appends
    // whitespace. It still changes the bytes, and so the sha.
    let file = edit_standard(&mut app, &editor_stub(&layout, "\n\n"));
    assert_eq!(file, dir.join("standard.json"));
    assert!(std::fs::read_to_string(&file).unwrap().ends_with("}\n\n\n"));

    let new = PolicySet::load(&dir).unwrap().sha().to_string();
    assert_ne!(new, old);
    let buf = draw_wide(&mut app);
    let head = line_with(&buf, "policy set");
    assert!(head.contains(&new[..12]), "{head}");
    assert!(!screen_text(&buf).contains(&old[..12]));
}

#[test]
fn an_editor_that_leaves_an_unloadable_file_shows_the_loaders_error_in_red() {
    let layout = scratch("policy-broken");
    let dir = scratch_policies(&layout);
    let old = PolicySet::load(&dir).unwrap().sha().to_string();
    let mut app = App::new(Layout::at(layout.root()), false, Duration::from_secs(2));
    app.set_policies(dir.clone());

    edit_standard(&mut app, &editor_stub(&layout, "// a comment"));
    let err = format!("{:#}", PolicySet::load(&dir).err().unwrap());
    assert!(err.contains("parsing"), "{err}");

    let buf = draw_wide(&mut app);
    let text = screen_text(&buf);
    assert!(
        !text.contains(&old[..12]),
        "no stale sha once the set breaks"
    );
    let y = (0..buf.area.height)
        .find(|&y| row_of(&buf, y).contains("does not load"))
        .unwrap_or_else(|| panic!("no refusal:\n{text}"));
    let x = (0..buf.area.width)
        .find(|&x| {
            (x..buf.area.width)
                .map(|x| buf[(x, y)].symbol())
                .collect::<String>()
                .starts_with("does not load")
        })
        .unwrap();
    assert_eq!(buf[(x, y)].fg, ratatui::style::Color::Red);
    assert!(text.contains("parsing"), "the loader's own words:\n{text}");
}

// --- the header's counts, build ticket 172 ---------------------------------

/// The clock the spend tests read at.
fn noon() -> chrono::DateTime<chrono::Utc> {
    chrono::DateTime::parse_from_rfc3339("2026-09-30T12:00:00Z")
        .unwrap()
        .with_timezone(&chrono::Utc)
}

/// A live record of `provider` that started at `started`, with `frames` after
/// line 0 in the envelope every fixture record carries.
fn write_live_frames(
    layout: &Layout,
    run_id: &str,
    provider: &str,
    model: Option<&str>,
    started: chrono::DateTime<chrono::Utc>,
    frames: &[serde_json::Value],
) {
    let pid = std::process::id();
    let start = hydra::procinfo::start_time(pid).unwrap().unwrap();
    let ts = started.to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    let mut text = json!({
        "run_id": run_id,
        "parent_run_id": null,
        "seq": 0,
        "ts": ts,
        "provider": provider,
        "model": model,
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
    })
    .to_string();
    text.push('\n');
    std::fs::write(layout.run_file(run_id), text).unwrap();
    append_frames(layout, run_id, provider, 1, frames);
}

fn append_frames(
    layout: &Layout,
    run_id: &str,
    provider: &str,
    first_seq: u64,
    frames: &[serde_json::Value],
) {
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .open(layout.run_file(run_id))
        .unwrap();
    for (i, raw) in frames.iter().enumerate() {
        let line = json!({
            "model": null,
            "parent_run_id": null,
            "provider": provider,
            "raw": raw,
            "run_id": run_id,
            "secret_flag": null,
            "seq": first_seq + i as u64,
            "ts": "2026-09-30T11:59:00.000Z",
        });
        writeln!(file, "{line}").unwrap();
    }
}

/// An `agy` step that finished and reported `output` tokens.
fn agy_step(index: u64, input: u64, output: u64, cache_read: u64) -> serde_json::Value {
    json!({
        "event": "step_update",
        "step_update": {
            "conversation_id": "c",
            "state": "DONE",
            "step_index": index,
            "step_type": "agent_response",
            "text_delta": "x",
            "usage": {
                "cache_read_tokens": cache_read,
                "input_tokens": input,
                "output_tokens": output,
                "thinking_tokens": 0,
                "total_tokens": input + output,
            },
        },
    })
}

/// A scratch `vendors.json` and `aliases.json` in the shapes
/// `integrations/puppets/` holds.
fn write_vendors(layout: &Layout, agy_budget: Option<f64>) -> PathBuf {
    let dir = layout.root().join("vendors");
    std::fs::create_dir_all(&dir).unwrap();
    let row = |provider: &str, model: Option<&str>, budget: Option<f64>| {
        json!({
            "provider": provider,
            "model": model,
            "as_of": null,
            "source_url": null,
            "context_tokens": null,
            "thinking_levels": null,
            "plan": null,
            "windows": "7d",
            "window_budget_tokens": budget,
        })
    };
    let vendors = json!([
        row("agy", Some("gemini-3.1-pro-high"), agy_budget),
        row("agy", None, Some(1.0)),
        row("claude", Some("opus"), Some(1000.0)),
    ]);
    let aliases = json!([{
        "provider": "agy",
        "recorded_model": "Gemini 3.1 Pro",
        "canonical": {"provider": "agy", "model": "gemini-3.1-pro-high"},
    }]);
    std::fs::write(dir.join("vendors.json"), vendors.to_string()).unwrap();
    std::fs::write(dir.join("aliases.json"), aliases.to_string()).unwrap();
    dir
}

fn spend_reader(layout: &Layout, vendors: PathBuf) -> Reader {
    Reader::new(
        Layout::at(layout.root()),
        Ok(policies()),
        layout.root().join("puppets"),
        None,
    )
    .with_vendors(vendors)
}

fn spend_of<'a>(snapshot: &'a Snapshot, provider: &str) -> &'a Spend {
    snapshot
        .spend
        .iter()
        .find(|spend| spend.provider == provider)
        .unwrap_or_else(|| panic!("no spend for {provider}: {:?}", snapshot.spend))
}

#[test]
fn a_live_agy_run_counts_its_steps_before_it_ends() {
    let layout = scratch("spend-agy");
    let vendors = write_vendors(&layout, Some(4600.0));
    let run = "01a0f001-0000-7000-8000-000000000001";
    write_live_frames(
        &layout,
        run,
        "agy",
        Some("Gemini 3.1 Pro"),
        noon() - chrono::Duration::minutes(2),
        &[agy_step(1, 100, 200, 50), agy_step(2, 10, 100, 0)],
    );
    let snapshot = spend_reader(&layout, vendors).read_at(None, noon());
    assert!(snapshot.errors.is_empty(), "{:?}", snapshot.errors);

    let agy = spend_of(&snapshot, "agy");
    assert_eq!((agy.live, agy.counted), (1, 1));
    assert_eq!(agy.output_tokens, 300);
    // Started two minutes ago at zero: 300 tokens over two minutes.
    assert_eq!(agy.tokens_per_minute, Some(150.0));
    // sheet.py: (raw_input + output_tokens) / budget, with the model read
    // through aliases.json. raw_input is 110 in and 50 read from cache.
    assert_eq!(agy.window_share, Some((160.0 + 300.0) / 4600.0));
    assert_eq!(spend_text(agy), "agy 150 tok/min, window 10.0%");
    // Only a provider with a live run gets an entry.
    assert_eq!(snapshot.spend.len(), 1);
}

#[test]
fn a_provider_that_counts_at_the_end_says_so_rather_than_zero() {
    let layout = scratch("spend-end");
    let vendors = write_vendors(&layout, Some(4600.0));
    let started = noon() - chrono::Duration::minutes(1);
    // `claude` carries `message.usage` on its assistant frames, and the
    // reader counts the `result` frame's, so a live run has none yet.
    write_live_frames(
        &layout,
        "01a0f002-0000-7000-8000-000000000002",
        "claude",
        Some("opus"),
        started,
        &[json!({
            "type": "assistant",
            "message": {
                "id": "msg_01",
                "role": "assistant",
                "content": [{"type": "text", "text": "working"}],
                "usage": {"input_tokens": 5, "output_tokens": 70},
            },
        })],
    );
    write_live_frames(
        &layout,
        "01a0f003-0000-7000-8000-000000000003",
        "codex",
        None,
        started,
        &[
            json!({"type": "thread.started", "thread_id": "t"}),
            json!({"type": "turn.started"}),
            json!({"type": "item.completed", "item": {"id": "m1", "type": "agent_message", "text": "hi"}}),
        ],
    );
    write_live_frames(
        &layout,
        "01a0f004-0000-7000-8000-000000000004",
        "cursor-agent",
        None,
        started,
        &[json!({
            "type": "assistant",
            "session_id": "s",
            "message": {"role": "assistant", "content": [{"type": "text", "text": "hi"}]},
        })],
    );
    let snapshot = spend_reader(&layout, vendors).read_at(None, noon());
    assert!(snapshot.errors.is_empty(), "{:?}", snapshot.errors);
    assert_eq!(snapshot.spend.len(), 3, "{:?}", snapshot.spend);
    for provider in ["claude", "codex", "cursor-agent"] {
        let spend = spend_of(&snapshot, provider);
        assert_eq!((spend.live, spend.counted), (1, 0), "{provider}");
        assert_eq!(spend.tokens_per_minute, None, "{provider}");
        assert_eq!(spend.window_share, None, "{provider}");
        assert_eq!(
            spend_text(spend),
            format!("{provider} no count until a run ends")
        );
    }
}

#[test]
fn tokens_per_minute_look_back_five_minutes_across_reads() {
    let layout = scratch("spend-rate");
    let vendors = write_vendors(&layout, None);
    let run = "01a0f005-0000-7000-8000-000000000005";
    // Started twenty minutes ago, so its start says nothing about the last
    // five and the first read has one sample only.
    write_live_frames(
        &layout,
        run,
        "agy",
        Some("gemini-3.1-pro-high"),
        noon() - chrono::Duration::minutes(20),
        &[agy_step(1, 10, 100, 0)],
    );
    let mut reader = spend_reader(&layout, vendors);
    let first = reader.read_at(None, noon());
    assert_eq!(spend_of(&first, "agy").output_tokens, 100);
    assert_eq!(spend_of(&first, "agy").tokens_per_minute, None);
    assert_eq!(
        spend_text(spend_of(&first, "agy")),
        "agy - tok/min, window -"
    );

    append_frames(&layout, run, "agy", 2, &[agy_step(2, 10, 300, 0)]);
    let second = reader.read_at(None, noon() + chrono::Duration::minutes(1));
    assert_eq!(spend_of(&second, "agy").output_tokens, 400);
    assert_eq!(spend_of(&second, "agy").tokens_per_minute, Some(300.0));

    // Nothing more is written. Once the first two samples are more than five
    // minutes old, the rate is over what is left, which did not move.
    reader.read_at(None, noon() + chrono::Duration::minutes(4));
    let later = reader.read_at(None, noon() + chrono::Duration::minutes(7));
    assert_eq!(spend_of(&later, "agy").tokens_per_minute, Some(0.0));
}

/// The rate a run shows at `at`, read by `reader`.
fn agy_rate(reader: &mut Reader, at: chrono::DateTime<chrono::Utc>) -> (u64, Option<f64>) {
    let snapshot = reader.read_at(None, at);
    assert!(snapshot.errors.is_empty(), "{:?}", snapshot.errors);
    let agy = spend_of(&snapshot, "agy");
    (agy.output_tokens, agy.tokens_per_minute)
}

#[test]
fn a_record_that_shrinks_drops_its_old_samples() {
    let layout = scratch("spend-shrink");
    let vendors = write_vendors(&layout, None);
    let run = "01a0f007-0000-7000-8000-000000000007";
    let started = noon() - chrono::Duration::minutes(20);
    let model = Some("gemini-3.1-pro-high");
    write_live_frames(
        &layout,
        run,
        "agy",
        model,
        started,
        &[agy_step(1, 10, 1000, 0)],
    );
    let mut reader = spend_reader(&layout, vendors);
    assert_eq!(agy_rate(&mut reader, noon()).0, 1000);

    // Same line 0, fewer bytes after it: the 1,000 is gone, and so is its sample.
    write_live_frames(
        &layout,
        run,
        "agy",
        model,
        started,
        &[agy_step(1, 10, 10, 0)],
    );
    let one = noon() + chrono::Duration::minutes(1);
    assert_eq!(agy_rate(&mut reader, one), (10, None));
    append_frames(&layout, run, "agy", 2, &[agy_step(2, 10, 100, 0)]);
    let two = noon() + chrono::Duration::minutes(2);
    assert_eq!(agy_rate(&mut reader, two), (110, Some(100.0)));
}

#[test]
fn a_record_with_a_new_line_zero_rereads_its_start() {
    let layout = scratch("spend-restart");
    let vendors = write_vendors(&layout, None);
    let run = "01a0f008-0000-7000-8000-000000000008";
    let model = Some("gemini-3.1-pro-high");
    write_live_frames(
        &layout,
        run,
        "agy",
        model,
        noon() - chrono::Duration::minutes(20),
        &[agy_step(1, 10, 1000, 0)],
    );
    let mut reader = spend_reader(&layout, vendors);
    assert_eq!(agy_rate(&mut reader, noon()).0, 1000);

    // A longer record that started at noon: 200 tokens over two minutes, not
    // measured against the old record's 1,000.
    write_live_frames(
        &layout,
        run,
        "agy",
        model,
        noon(),
        &[agy_step(1, 10, 100, 0), agy_step(2, 10, 100, 0)],
    );
    let two = noon() + chrono::Duration::minutes(2);
    assert_eq!(agy_rate(&mut reader, two), (200, Some(100.0)));
}

#[test]
fn an_equal_length_replacement_is_read_again() {
    let layout = scratch("spend-same-len");
    let vendors = write_vendors(&layout, None);
    let run = "01a0f009-0000-7000-8000-000000000009";
    let model = Some("gemini-3.1-pro-high");
    let path = layout.run_file(run);
    let started = noon() - chrono::Duration::minutes(20);
    write_live_frames(
        &layout,
        run,
        "agy",
        model,
        started,
        &[agy_step(1, 10, 100, 0)],
    );
    let len = std::fs::metadata(&path).unwrap().len();
    let mut reader = spend_reader(&layout, vendors);
    assert_eq!(agy_rate(&mut reader, noon()).0, 100);

    let started = started - chrono::Duration::minutes(1);
    write_live_frames(
        &layout,
        run,
        "agy",
        model,
        started,
        &[agy_step(1, 10, 900, 0)],
    );
    assert_eq!(std::fs::metadata(&path).unwrap().len(), len);
    let one = noon() + chrono::Duration::minutes(1);
    assert_eq!(agy_rate(&mut reader, one).0, 900);
}

#[test]
fn a_null_budget_and_claude_show_a_dash_for_the_window() {
    let layout = scratch("spend-dash");
    let vendors = write_vendors(&layout, None);
    write_live_frames(
        &layout,
        "01a0f006-0000-7000-8000-000000000006",
        "agy",
        Some("gemini-3.1-pro-high"),
        noon() - chrono::Duration::minutes(1),
        &[agy_step(1, 10, 60, 0)],
    );
    let snapshot = spend_reader(&layout, vendors.clone()).read_at(None, noon());
    let agy = spend_of(&snapshot, "agy");
    assert_eq!(agy.window_share, None);
    assert_eq!(spend_text(agy), "agy 60 tok/min, window -");

    // `claude` has a positive budget in the scratch file and still gets no
    // share: sheet.py measures its window by utilization, not token spend. A
    // live `claude` run carries no counts, so this asks the budgets directly.
    let counts = Counts {
        input_tokens: 5,
        output_tokens: 10,
        ..Counts::default()
    };
    let budgets = Budgets::load(&vendors).unwrap();
    let claude = Spend {
        provider: "claude".to_string(),
        live: 2,
        counted: 1,
        output_tokens: 10,
        tokens_per_minute: Some(10.0),
        window_share: budgets.share("claude", Some("opus"), &counts),
    };
    assert_eq!(
        spend_text(&claude),
        "claude 10 tok/min, window - (1 of 2 runs counted)"
    );
}

#[test]
fn an_alias_onto_claude_gets_no_share() {
    let layout = scratch("spend-alias");
    let vendors = write_vendors(&layout, Some(4600.0));
    // sheet.py excludes `claude` after `canonical`, so a spelling that maps
    // onto it is excluded too, whatever provider recorded it.
    let aliases = json!([{
        "provider": "agy",
        "recorded_model": "Claude Opus",
        "canonical": {"provider": "claude", "model": "opus"},
    }]);
    std::fs::write(vendors.join("aliases.json"), aliases.to_string()).unwrap();
    write_live_frames(
        &layout,
        "01a0f00a-0000-7000-8000-00000000000a",
        "agy",
        Some("Claude Opus"),
        noon() - chrono::Duration::minutes(1),
        &[agy_step(1, 10, 60, 0)],
    );
    let snapshot = spend_reader(&layout, vendors).read_at(None, noon());
    assert!(snapshot.errors.is_empty(), "{:?}", snapshot.errors);
    let agy = spend_of(&snapshot, "agy");
    assert_eq!((agy.counted, agy.output_tokens), (1, 60));
    assert_eq!(agy.window_share, None);
    assert_eq!(spend_text(agy), "agy 60 tok/min, window -");
}

#[test]
fn the_header_shows_the_counts_without_the_mascot() {
    let mut app = App::new(
        Layout::at("/nonexistent/hydra"),
        false,
        Duration::from_secs(2),
    );
    let mut snapshot = fixed_snapshot();
    snapshot.spend = vec![
        Spend {
            provider: "agy".to_string(),
            live: 1,
            counted: 1,
            output_tokens: 300,
            tokens_per_minute: Some(150.0),
            window_share: Some(0.1),
        },
        Spend {
            provider: "claude".to_string(),
            live: 1,
            counted: 0,
            output_tokens: 0,
            tokens_per_minute: None,
            window_share: None,
        },
    ];
    app.set_snapshot(snapshot);
    let mut buf = Buffer::empty(AREA);
    app.draw(AREA, &mut buf);
    let top = row_of(&buf, 0);
    assert!(top.contains("agy 150 tok/min, window 10.0%"), "{top}");
    assert!(top.contains("claude no count until a run ends"), "{top}");
    assert!(!screen_text(&buf).contains('$'), "no dollars");
    // The panels start below it.
    assert_eq!(app.hits.table.y, 1);
}

// --- the policy pane stays true and scrolls, build ticket 174 --------------

#[test]
fn a_policy_edited_outside_the_cockpit_shows_on_the_next_refresh() {
    let layout = scratch("policy-outside");
    let dir = scratch_policies(&layout);
    let old = PolicySet::load(&dir).unwrap().sha().to_string();
    let mut on_old = card(
        "01a0f003-0000-7000-8000-000000000003",
        "codex",
        State::Live,
        None,
    );
    on_old.policy_sha = Some(old.clone());
    let mut app = App::new(Layout::at(layout.root()), false, Duration::from_secs(2));
    app.set_policies(dir.clone());
    app.set_snapshot(snapshot_of(vec![on_old.clone()], Vec::new()));
    app.handle(&key(KeyCode::Char('p')));
    let line = line_with(&draw_wide(&mut app), "01a0f003");
    assert!(line.contains("current"), "{line}");

    // Someone else's editor, not the cockpit's `e`.
    let file = dir.join("standard.json");
    let mut text = std::fs::read_to_string(&file).unwrap();
    text.push('\n');
    std::fs::write(&file, &text).unwrap();
    app.set_snapshot(snapshot_of(vec![on_old.clone()], Vec::new()));
    let buf = draw_wide(&mut app);
    let line = line_with(&buf, "01a0f003");
    assert!(line.contains("differs"), "{line}");
    let new = PolicySet::load(&dir).unwrap().sha().to_string();
    assert!(line_with(&buf, "policy set").contains(&new[..12]));

    // And a broken outside save puts the loader's error up, not the old sha.
    text.push_str("// a comment");
    std::fs::write(&file, &text).unwrap();
    app.set_snapshot(snapshot_of(vec![on_old], Vec::new()));
    let text = screen_text(&draw_wide(&mut app));
    assert!(text.contains("does not load"), "{text}");
    assert!(!text.contains(&new[..12]), "{text}");
}

#[test]
fn ten_live_runs_at_120x24_scroll_to_the_last() {
    let area = Rect::new(0, 0, 120, 24);
    let cards: Vec<Card> = (0..10)
        .map(|i| card(&format!("{i:08}-live"), "claude", State::Live, None))
        .collect();
    let mut snapshot = snapshot_of(cards, Vec::new());
    snapshot.spend = vec![Spend {
        provider: "claude".to_string(),
        live: 10,
        counted: 0,
        output_tokens: 0,
        tokens_per_minute: None,
        window_share: None,
    }];
    let layout = scratch("policy-scroll");
    let mut app = App::new(Layout::at(layout.root()), false, Duration::from_secs(2));
    app.set_policies(scratch_policies(&layout));
    app.set_snapshot(snapshot);
    app.handle(&key(KeyCode::Char('p')));
    let draw = |app: &mut App| {
        let mut buf = Buffer::empty(area);
        app.draw(area, &mut buf);
        screen_text(&buf)
    };
    let first = draw(&mut app);
    assert!(first.contains("00000000"), "{first}");
    assert!(!first.contains("00000009"), "{first}");
    assert!(first.contains(" more"), "the cut says so:\n{first}");

    // Past the end on purpose: the list stops at its last page.
    for _ in 0..5 {
        app.handle(&key(KeyCode::PageDown));
        draw(&mut app);
    }
    let paged = draw(&mut app);
    assert!(
        paged.contains("00000009"),
        "PageDown reaches the last:\n{paged}"
    );
    assert!(!paged.contains("00000000"), "{paged}");
    assert!(paged.contains("below"), "{paged}");

    for _ in 0..5 {
        app.handle(&key(KeyCode::PageUp));
    }
    let back = draw(&mut app);
    assert!(back.contains("00000000"), "{back}");

    // The wheel over the pane, one run at a time. The pane covers the
    // middle of the screen.
    for _ in 0..10 {
        app.handle(&mouse(MouseEventKind::ScrollDown, 60, 12));
    }
    let wheeled = draw(&mut app);
    assert!(
        wheeled.contains("00000009"),
        "the wheel reaches the last:\n{wheeled}"
    );
}
