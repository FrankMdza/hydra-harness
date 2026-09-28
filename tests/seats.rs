//! The seat channel of map ticket 82, build ticket 140: a gated interactive
//! session's face asks its own launcher for seats over `seats.sock`.
//!
//! **No provider is launched.** The launcher takes the function that starts one
//! seat as an argument, and every test here hands it a stub that keeps the
//! `run::Request` it was given and answers a canned result. What the stub
//! stands in for is `run::run`, which `tests/delegation.rs` already drives on
//! the `--parent` it writes to `run_start`; what is tested here is everything
//! the launcher decides before that call and everything it does with the
//! answer.
//!
//! **Roots live under `/private/tmp` with short names.** `sun_path` is 104
//! bytes, and a root under `$TMPDIR` plus `spawn/<uuid>/seats.sock` overflows
//! it. Measured 2026-09-28 by the session that wrote the ticket.
//!
//! The tests that need a live `sandbox-exec` skip inside a seatbelt, which
//! cannot nest, and say so on stderr.

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use hydra::outcome::{RunResult, Usage};
use hydra::paths::Layout;
use hydra::policy::PolicySet;
use hydra::provider::Provider;
use hydra::record::{RunStart, SCHEMA_VERSION, Writer};
use hydra::run::{Call, Request};
use hydra::seats::{self, Ask, AskedSeat, Launcher, Limits, Session};
use serde_json::{Value, json};

mod corpus;
use corpus::seatbelt_exec_or_skip;

const SESSION: &str = "01a0e7ae-0000-7000-8000-000000000140";
const OTHER: &str = "01a0e7ae-0000-7000-8000-000000000141";

/// A scratch root short enough for a socket path, one per test.
fn root() -> PathBuf {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let root = PathBuf::from(format!(
        "/private/tmp/hs{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&root);
    Layout::at(&root).ensure().unwrap();
    root
}

fn policies_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("policies")
}

/// The session's own record as the launcher leaves it while `claude` runs:
/// `run_start` and nothing after it.
fn write_session(layout: &Layout, run_id: &str, policy: &str) -> PathBuf {
    let run_file = layout.run_file(run_id);
    let mut writer = Writer::create(&run_file, run_id, None, Some("claude"), None).unwrap();
    writer
        .hydra_line(&RunStart {
            kind: "run_start",
            schema_version: SCHEMA_VERSION,
            hydra_version: "0.2.0".into(),
            command: "run".into(),
            launch_mode: Some("interactive".into()),
            provider_version: Some("stub".into()),
            provider_session_id: Some("01a0e7ae-0000-7000-8000-0000000001aa".into()),
            task: None,
            policy: Some(policy.into()),
            policy_sha: None,
            floor_lifted: Some(false),
            gate_sha: None,
            sandbox: None,
            scopes: None,
            cwd: "/tmp".into(),
            worktree: None,
            depth: 0,
            parent_policy: None,
            pid: None,
            process_start_time: None,
            skills: None,
            mcp: None,
            auth: None,
            env_allowlist: Some(vec!["PATH".into()]),
            subscription: Some("anthropic".into()),
            resumed_from: None,
            may_edit: None,
        })
        .unwrap();
    run_file
}

/// A two-seat shape standard sessions may ask for once an hour, and a
/// one-seat shape only `permissive` may ask for.
fn write_panels(root: &Path) -> PathBuf {
    let path = root.join("panels.json");
    let document = json!({
        "shapes": [
            {
                "name": "review",
                "askers": ["standard"],
                "cap_per_hour": 1,
                "seats": [
                    {"provider": "claude", "model": "opus", "policy": "read-only", "role": "reviewer"},
                    {"provider": "codex", "policy": "judge", "role": "judge"}
                ]
            },
            {
                "name": "wide",
                "askers": ["permissive"],
                "cap_per_hour": 5,
                "seats": [
                    {"provider": "claude", "policy": "standard", "role": "worker"}
                ]
            }
        ]
    });
    std::fs::write(&path, serde_json::to_string_pretty(&document).unwrap()).unwrap();
    path
}

/// What the stub launch was handed, one entry per seat.
#[derive(Clone, Debug)]
struct Launched {
    provider: Provider,
    model: Option<String>,
    policy: String,
    parent: Option<String>,
    run_id: Option<String>,
    task: String,
    quiet: bool,
}

fn stub(launched: Arc<Mutex<Vec<Launched>>>) -> Box<seats::LaunchSeat> {
    Box::new(move |request: Request| {
        let run_id = request.run_id.clone().unwrap_or_default();
        let said = format!(
            "seat {} under {} read the diff",
            request.provider, request.policy
        );
        launched.lock().unwrap().push(Launched {
            provider: request.provider,
            model: request.model.clone(),
            policy: request.policy.clone(),
            parent: request.parent.clone(),
            run_id: request.run_id.clone(),
            task: request.task.clone(),
            quiet: request.quiet,
        });
        Ok(Call::Ended(Box::new(RunResult {
            run_id: run_id.clone(),
            outcome: Some("completed"),
            outcome_source: Some("stream"),
            exit_code: Some(0),
            error: None,
            result_text: Some(said),
            branch: None,
            diff_stat: None,
            usage: Usage::unavailable(),
            run_file: format!("/tmp/{run_id}.jsonl"),
        })))
    })
}

struct Fixture {
    root: PathBuf,
    layout: Layout,
    run_file: PathBuf,
    launcher: Arc<Launcher>,
    launched: Arc<Mutex<Vec<Launched>>>,
}

impl Fixture {
    fn new(session_policy: &str) -> Self {
        Self::launching(session_policy, stub)
    }

    /// A fixture whose launcher starts seats with `launch`, handed the list
    /// [`Fixture::launched`] reads.
    fn launching(
        session_policy: &str,
        launch: impl FnOnce(Arc<Mutex<Vec<Launched>>>) -> Box<seats::LaunchSeat>,
    ) -> Self {
        let root = root();
        let layout = Layout::at(&root);
        let run_file = write_session(&layout, SESSION, session_policy);
        // A real launch has written the gate here by the time it binds.
        std::fs::create_dir_all(layout.spawn(SESSION)).unwrap();
        let panels = write_panels(&root);
        let launched = Arc::new(Mutex::new(Vec::new()));
        let launcher = Arc::new(Launcher::new(
            Session {
                run_id: SESSION.to_string(),
                policy: session_policy.to_string(),
                model: None,
                run_file: run_file.clone(),
                working_dir: root.clone(),
            },
            policies_dir(),
            panels,
            launch(launched.clone()),
        ));
        Self {
            root,
            layout,
            run_file,
            launcher,
            launched,
        }
    }

    fn launched(&self) -> Vec<Launched> {
        self.launched.lock().unwrap().clone()
    }

    /// What the harvest does once `claude` exits, at a home with no
    /// transcript, which writes `run_end` all the same.
    fn harvest(&self) {
        let home = self.root.join("home");
        std::fs::create_dir_all(&home).unwrap();
        hydra::interactive::harvest_in(&self.layout, SESSION, Some(0), &home).unwrap();
        assert!(hydra::live::has_run_end(&self.run_file).unwrap());
        hydra::record::validate::check_file(&self.run_file)
            .unwrap()
            .assert_ok();
    }

    /// Every `seat_request` line on the session's record, in order.
    fn requests(&self) -> Vec<Value> {
        std::fs::read_to_string(&self.run_file)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str::<Value>(line).unwrap())
            .filter(|line| line["hydra"]["type"] == "seat_request")
            .map(|line| line["hydra"].clone())
            .collect()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

fn one_seat(provider: &str, policy: &str) -> Ask {
    Ask {
        task: "read the diff and say what breaks".into(),
        shape: None,
        seats: Some(vec![AskedSeat {
            provider: provider.into(),
            model: None,
            policy: policy.into(),
        }]),
    }
}

fn by_shape(shape: &str) -> Ask {
    Ask {
        task: "read the diff and say what breaks".into(),
        shape: Some(shape.into()),
        seats: None,
    }
}

/// The built binary's face, spoken to over a pipe as `claude` would.
struct Face {
    child: Child,
    input: ChildStdin,
    output: BufReader<ChildStdout>,
}

impl Face {
    fn start(root: &Path, parent: &str, profile: Option<&Path>) -> Self {
        let mut command = match profile {
            Some(profile) => {
                let mut wrapped = Command::new("/usr/bin/sandbox-exec");
                wrapped
                    .arg("-f")
                    .arg(profile)
                    .arg(env!("CARGO_BIN_EXE_hydra"));
                wrapped
            }
            None => Command::new(env!("CARGO_BIN_EXE_hydra")),
        };
        let mut child = command
            .arg("delegate-face")
            .arg("--parent")
            .arg(parent)
            .env("HYDRA_ROOT", root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let input = child.stdin.take().unwrap();
        let output = BufReader::new(child.stdout.take().unwrap());
        Self {
            child,
            input,
            output,
        }
    }

    fn call(&mut self, tool: &str, arguments: Value) -> (String, bool) {
        let request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {"name": tool, "arguments": arguments},
        });
        writeln!(self.input, "{request}").unwrap();
        self.input.flush().unwrap();
        let mut line = String::new();
        self.output.read_line(&mut line).unwrap();
        let answer: Value = serde_json::from_str(&line).unwrap();
        let result = &answer["result"];
        (
            result["content"][0]["text"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
            result["isError"].as_bool().unwrap_or(false),
        )
    }
}

impl Drop for Face {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

// ---------------------------------------------------------------------------
// Do, item 1
// ---------------------------------------------------------------------------

#[test]
fn a_request_matching_a_shape_launches_each_seat_as_a_child_of_the_session() {
    let fixture = Fixture::new("standard");
    let socket = seats::socket_path(&fixture.layout, SESSION);
    let served = seats::serve(fixture.launcher.clone(), &socket).unwrap();

    let mut face = Face::start(&fixture.root, SESSION, None);
    let (text, is_error) = face.call(
        "panel",
        json!({"shape": "review", "task": "read the diff and say what breaks"}),
    );
    assert!(!is_error, "{text}");

    let launched = fixture.launched();
    assert_eq!(launched.len(), 2, "one launch per seat: {launched:?}");
    let reviewer = launched
        .iter()
        .find(|seat| seat.provider == Provider::Claude)
        .expect("the claude seat");
    assert_eq!(reviewer.policy, "read-only");
    assert_eq!(reviewer.model.as_deref(), Some("opus"));
    assert!(reviewer.task.contains("reviewer"), "{}", reviewer.task);
    assert!(reviewer.task.contains("read the diff and say what breaks"));
    let judge = launched
        .iter()
        .find(|seat| seat.provider == Provider::Codex)
        .expect("the codex seat");
    assert_eq!(judge.policy, "judge");
    for seat in &launched {
        // The parent is what `run::run` writes as `parent_run_id` on the
        // seat's own `run_start`, and the run id is minted by the launcher
        // so the session's record can name it before the seat exists.
        assert_eq!(seat.parent.as_deref(), Some(SESSION), "{seat:?}");
        assert!(seat.run_id.is_some(), "{seat:?}");
    }

    // Each child's text is inside one boundary drawn after both stopped.
    let body: Value = serde_json::from_str(&text).unwrap();
    let answers = body["seats"].as_array().expect("one entry per seat");
    assert_eq!(answers.len(), 2);
    for answer in answers {
        let said = answer["result"]["result_text"].as_str().unwrap();
        assert!(said.starts_with("<<<hydra-result "), "{said}");
        assert!(said.contains("read the diff"), "{said}");
        assert!(said.contains("<<<end hydra-result "), "{said}");
    }

    // The request is on the session's record with the runs it started.
    let requests = fixture.requests();
    assert_eq!(requests.len(), 1, "{requests:?}");
    assert_eq!(requests[0]["decision"], "launched");
    assert_eq!(requests[0]["shape"], "review");
    let runs: Vec<&str> = requests[0]["runs"]
        .as_array()
        .unwrap()
        .iter()
        .map(|id| id.as_str().unwrap())
        .collect();
    for seat in &launched {
        assert!(runs.contains(&seat.run_id.as_deref().unwrap()), "{runs:?}");
    }

    // The shape's cap is one an hour, so the second ask launches nothing.
    let (text, is_error) = face.call("panel", json!({"shape": "review", "task": "again"}));
    assert!(is_error, "{text}");
    assert!(text.contains("hydra/panel-cap"), "{text}");
    assert_eq!(
        fixture.launched().len(),
        2,
        "the refused ask launched a seat"
    );
    assert_eq!(fixture.requests().len(), 2);
    assert_eq!(fixture.requests()[1]["decision"], "refused");

    drop(face);
    served.stop();
    hydra::record::validate::check_file(&fixture.run_file)
        .unwrap()
        .assert_ok();
}

#[test]
fn a_one_seat_delegate_that_matches_a_one_seat_shape_launches_under_it() {
    // `delegate` is a request for one seat, and `wide` is exactly that seat,
    // so a `permissive` session gets it with no prompt.
    let fixture = Fixture::new("permissive");
    let reply = fixture.launcher.answer(&one_seat("claude", "standard"));
    assert!(!reply.is_error, "{}", reply.text);
    let launched = fixture.launched();
    assert_eq!(launched.len(), 1);
    assert!(launched[0].task.contains("worker"), "{}", launched[0].task);
    assert_eq!(fixture.requests()[0]["shape"], "wide");
}

// ---------------------------------------------------------------------------
// Do, item 2
// ---------------------------------------------------------------------------

#[test]
fn a_shape_the_sessions_policy_may_not_ask_for_is_refused_by_name() {
    let fixture = Fixture::new("standard");
    let reply = fixture.launcher.answer(&by_shape("wide"));
    assert!(reply.is_error, "{}", reply.text);
    assert!(
        reply.text.contains("hydra/shape-not-askable"),
        "{}",
        reply.text
    );
    assert!(reply.text.contains("wide"), "{}", reply.text);
    assert!(
        fixture.launched().is_empty(),
        "a refused shape launched a seat"
    );
    assert_eq!(fixture.requests()[0]["decision"], "refused");
    assert_eq!(fixture.requests()[0]["reason"], "hydra/shape-not-askable");
}

#[test]
fn a_seat_above_the_sessions_policy_is_refused_by_name() {
    let fixture = Fixture::new("read-only");
    let reply = fixture.launcher.answer(&one_seat("claude", "standard"));
    assert!(reply.is_error, "{}", reply.text);
    assert!(
        reply.text.contains("hydra/seat-above-session"),
        "{}",
        reply.text
    );
    assert!(reply.text.contains("standard"), "{}", reply.text);
    assert!(
        fixture.launched().is_empty(),
        "a seat above the session launched"
    );
    assert_eq!(fixture.requests()[0]["reason"], "hydra/seat-above-session");
}

#[test]
fn a_shape_nobody_approved_is_refused_by_name() {
    let fixture = Fixture::new("standard");
    let reply = fixture.launcher.answer(&by_shape("council"));
    assert!(reply.text.contains("hydra/unknown-shape"), "{}", reply.text);
    assert!(fixture.launched().is_empty());
}

// ---------------------------------------------------------------------------
// Do, item 3
// ---------------------------------------------------------------------------

#[test]
fn a_request_matching_no_shape_waits_for_approval_and_is_on_the_record() {
    let fixture = Fixture::new("standard");
    let reply = fixture.launcher.answer(&one_seat("claude", "read-only"));
    assert!(
        reply.text.contains("hydra/approval-required"),
        "{}",
        reply.text
    );
    assert!(
        fixture.launched().is_empty(),
        "an unapproved request launched"
    );

    let requests = fixture.requests();
    assert_eq!(requests.len(), 1, "{requests:?}");
    let id = requests[0]["request_id"].as_str().unwrap();
    assert!(
        reply.text.contains(id),
        "the reply names {id}: {}",
        reply.text
    );
    assert_eq!(requests[0]["decision"], "approval_required");
    assert_eq!(requests[0]["seats"][0]["provider"], "claude");
    assert_eq!(requests[0]["seats"][0]["policy"], "read-only");
    assert_eq!(requests[0]["task"], "read the diff and say what breaks");
    hydra::record::validate::check_file(&fixture.run_file)
        .unwrap()
        .assert_ok();
}

#[test]
fn once_the_session_has_ended_a_request_writes_nothing_and_launches_nothing() {
    let fixture = Fixture::new("standard");
    fixture.launcher.close();
    let reply = fixture.launcher.answer(&by_shape("review"));
    assert!(reply.is_error, "{}", reply.text);
    assert!(fixture.launched().is_empty());
    assert!(fixture.requests().is_empty());
}

#[test]
fn a_face_with_no_launcher_outside_a_seatbelt_does_not_ask_a_socket() {
    // No `seats.sock` in the parent's spawn directory is a run with no
    // launcher to ask, which is every recorded run.
    let root = root();
    let layout = Layout::at(&root);
    std::fs::create_dir_all(layout.spawn(SESSION)).unwrap();
    let asked = seats::ask(&layout, SESSION, &by_shape("review")).unwrap();
    assert!(asked.is_none());
    let _ = std::fs::remove_dir_all(&root);
}

// ---------------------------------------------------------------------------
// Do, item 4, and the profile every run gets
// ---------------------------------------------------------------------------

fn spawn_profile(layout: &Layout, run_id: &str, provider: Provider, policy: &str) -> PathBuf {
    let set = PolicySet::load(policies_dir()).unwrap();
    let worktree = layout.root().join(format!("wt-{run_id}"));
    std::fs::create_dir_all(&worktree).unwrap();
    hydra::run::write_spawn_config(
        layout,
        run_id,
        &set,
        provider,
        policy,
        Some(&worktree),
        None,
        None,
        false,
    )
    .unwrap()
    .sandbox_profile
    .expect("a macOS spawn writes a profile")
}

#[test]
fn every_profile_scopes_unix_sockets_to_the_runs_own_seats_socket() {
    if !hydra::sandbox::wraps_launches() {
        return;
    }
    let root = root();
    let layout = Layout::at(&root);
    let spawns = std::fs::canonicalize(layout.spawns()).unwrap();
    for provider in [
        Provider::Claude,
        Provider::CursorAgent,
        Provider::Agy,
        Provider::Codex,
    ] {
        for policy in hydra::policy::NAMES {
            let run_id = format!("{}-{policy}", provider.binary());
            let text =
                std::fs::read_to_string(spawn_profile(&layout, &run_id, provider, policy)).unwrap();
            let deny = format!(
                r#"(deny network-outbound (remote unix-socket (subpath "{}")))"#,
                spawns.display()
            );
            let allow = format!(
                r#"(allow network-outbound (remote unix-socket (path-literal "{}/{run_id}/seats.sock")))"#,
                spawns.display()
            );
            let at_deny = text.find(&deny);
            let at_allow = text.find(&allow);
            assert!(at_deny.is_some(), "{run_id}: no socket deny\n{text}");
            assert!(at_allow.is_some(), "{run_id}: no own socket allow\n{text}");
            // SBPL takes the last matching rule, so the allow comes after.
            assert!(at_deny < at_allow, "{run_id}: allow before deny\n{text}");
        }
    }
    let _ = std::fs::remove_dir_all(&root);
}

/// Connects to `socket` under `profile` and prints what `connect` answered.
fn connect_under(profile: &Path, socket: &Path) -> String {
    let script = format!(
        "import socket\ns = socket.socket(socket.AF_UNIX)\n\
         try:\n    s.connect({:?})\n    print('connected')\n\
         except OSError as e:\n    print('refused', e.errno)\n",
        socket.display().to_string()
    );
    let out = Command::new("/usr/bin/sandbox-exec")
        .arg("-f")
        .arg(profile)
        .arg("/usr/bin/python3")
        .arg("-c")
        .arg(script)
        .output()
        .unwrap();
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

#[test]
fn from_inside_the_seatbelt_another_runs_socket_is_refused_by_the_kernel() {
    if !seatbelt_exec_or_skip(
        "from_inside_the_seatbelt_another_runs_socket_is_refused_by_the_kernel",
    ) {
        return;
    }
    let root = root();
    let layout = Layout::at(&root);
    let profile = spawn_profile(&layout, SESSION, Provider::Claude, "standard");
    std::fs::create_dir_all(layout.spawn(OTHER)).unwrap();
    let own = std::os::unix::net::UnixListener::bind(seats::socket_path(&layout, SESSION)).unwrap();
    let other = std::os::unix::net::UnixListener::bind(seats::socket_path(&layout, OTHER)).unwrap();

    let out = connect_under(&profile, &seats::socket_path(&layout, SESSION));
    assert!(out.contains("connected"), "the run's own socket: {out}");
    let out = connect_under(&profile, &seats::socket_path(&layout, OTHER));
    assert!(
        out.contains("refused 1"),
        "another run's socket, EPERM: {out}"
    );

    drop((own, other));
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_non_interactive_runs_face_still_refuses_inside_the_seatbelt() {
    if !seatbelt_exec_or_skip("a_non_interactive_runs_face_still_refuses_inside_the_seatbelt") {
        return;
    }
    // A recorded run has a spawn directory and no launcher waiting on it.
    let root = root();
    let layout = Layout::at(&root);
    write_session(&layout, SESSION, "standard");
    let profile = spawn_profile(&layout, SESSION, Provider::Claude, "standard");
    let mut face = Face::start(&root, SESSION, Some(&profile));
    let (text, is_error) = face.call(
        "delegate",
        json!({"provider": "claude", "task": "go", "policy": "read-only"}),
    );
    assert!(is_error, "{text}");
    assert!(text.contains("hydra/seatbelt-delegation"), "{text}");
    drop(face);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn an_interactive_sessions_face_reaches_its_launcher_from_inside_the_seatbelt() {
    if !seatbelt_exec_or_skip(
        "an_interactive_sessions_face_reaches_its_launcher_from_inside_the_seatbelt",
    ) {
        return;
    }
    let fixture = Fixture::new("standard");
    let profile = spawn_profile(&fixture.layout, SESSION, Provider::Claude, "standard");
    let served = seats::serve(
        fixture.launcher.clone(),
        &seats::socket_path(&fixture.layout, SESSION),
    )
    .unwrap();
    let mut face = Face::start(&fixture.root, SESSION, Some(&profile));
    let (text, is_error) = face.call(
        "panel",
        json!({"shape": "review", "task": "read the diff and say what breaks"}),
    );
    assert!(!is_error, "{text}");
    assert_eq!(fixture.launched().len(), 2);
    drop(face);
    served.stop();
}

// ---------------------------------------------------------------------------
// Build ticket 142: the launcher under a hostile session
// ---------------------------------------------------------------------------

/// One raw line to `socket` and the reply, as the session's own shell could
/// send it without the face.
fn send_raw(socket: &Path, line: &str) -> Value {
    let mut stream = UnixStream::connect(socket).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    // A line past the limit is answered and closed before it is all sent, and
    // the rest of the write then fails with a broken pipe.
    let _ = stream
        .write_all(line.as_bytes())
        .and_then(|()| stream.write_all(b"\n"));
    let mut reply = String::new();
    BufReader::new(stream).read_line(&mut reply).unwrap();
    serde_json::from_str(&reply).unwrap()
}

/// Reads the one line the launcher answers on `stream`, or `None` when
/// nothing came within `wait`.
fn reply_within(stream: &UnixStream, wait: Duration) -> Option<Value> {
    stream.set_read_timeout(Some(wait)).unwrap();
    let mut reply = String::new();
    match BufReader::new(stream).read_line(&mut reply) {
        Ok(read) if read > 0 => Some(serde_json::from_str(&reply).unwrap()),
        _ => None,
    }
}

#[test]
fn a_session_past_its_request_cap_is_refused_and_the_record_says_so_once() {
    // Every request is a line on the record, refused ones included, so a
    // loop asking for a shape nobody approved grew the record without bound.
    let fixture = Fixture::new("standard");
    let asks = seats::REQUESTS_PER_HOUR + 10;
    for index in 0..asks {
        let reply = fixture.launcher.answer(&by_shape("council"));
        assert!(reply.is_error, "{}", reply.text);
        let capped = reply.text.contains("hydra/request-cap");
        assert_eq!(
            capped,
            index >= seats::REQUESTS_PER_HOUR,
            "request {index}: {}",
            reply.text
        );
    }
    let requests = fixture.requests();
    assert_eq!(
        requests.len(),
        seats::REQUESTS_PER_HOUR + 1,
        "one line per request under the cap and one saying it was hit"
    );
    let last = requests.last().unwrap();
    assert_eq!(last["decision"], "refused");
    assert_eq!(last["reason"], "hydra/request-cap");
    assert_eq!(
        requests
            .iter()
            .filter(|line| line["reason"] == "hydra/request-cap")
            .count(),
        1
    );
    // The cap counts every request, so an approved shape is refused too.
    let reply = fixture.launcher.answer(&by_shape("review"));
    assert!(reply.text.contains("hydra/request-cap"), "{}", reply.text);
    assert!(fixture.launched().is_empty());
    hydra::record::validate::check_file(&fixture.run_file)
        .unwrap()
        .assert_ok();
}

#[test]
fn a_request_line_over_64_kb_is_refused_before_the_record() {
    let fixture = Fixture::new("standard");
    let socket = seats::socket_path(&fixture.layout, SESSION);
    let served = seats::serve(fixture.launcher.clone(), &socket).unwrap();

    let ask = json!({"shape": "review", "task": "x".repeat(100 << 10)});
    let reply = send_raw(&socket, &ask.to_string());
    assert_eq!(reply["is_error"], true, "{reply}");
    let text = reply["text"].as_str().unwrap();
    assert!(text.contains("hydra/seat-request"), "{text}");
    assert!(text.contains("65536"), "{text}");
    assert!(fixture.launched().is_empty(), "the long line launched");
    assert!(fixture.requests().is_empty(), "the long line was recorded");

    // A line just under the limit still reaches the launcher.
    let ask = json!({"shape": "review", "task": "x".repeat(60 << 10)});
    let reply = send_raw(&socket, &ask.to_string());
    assert_eq!(reply["is_error"], false, "{reply}");
    drop(served);
}

#[test]
fn a_flood_and_a_slow_client_do_not_keep_a_request_from_its_answer() {
    let fixture = Fixture::new("standard");
    let socket = seats::socket_path(&fixture.layout, SESSION);
    let limits = Limits {
        connections: 4,
        deadline: Duration::from_millis(500),
        write_timeout: Duration::from_secs(1),
    };
    let served = seats::serve_with(fixture.launcher.clone(), &socket, limits).unwrap();

    // A slow client sends a byte every 100 ms. A timeout per read never
    // fires on it; a deadline for the line does.
    let slow = UnixStream::connect(&socket).unwrap();
    let dribble = {
        let mut slow = slow.try_clone().unwrap();
        std::thread::spawn(move || {
            let until = Instant::now() + Duration::from_secs(3);
            while Instant::now() < until && slow.write_all(b" ").is_ok() {
                std::thread::sleep(Duration::from_millis(100));
            }
        })
    };
    // Three more that send nothing fill the four slots.
    let silent: Vec<UnixStream> = (0..3)
        .map(|_| UnixStream::connect(&socket).unwrap())
        .collect();

    // The flood: every connection past four is refused at once.
    for index in 0..10 {
        let flood = UnixStream::connect(&socket).unwrap();
        let reply = reply_within(&flood, Duration::from_secs(2))
            .unwrap_or_else(|| panic!("flood connection {index} held rather than refused"));
        let text = reply["text"].as_str().unwrap();
        assert!(text.contains("hydra/seat-channel-busy"), "{text}");
    }
    assert!(fixture.requests().is_empty(), "a refused connection wrote");

    let started = Instant::now();
    let reply = reply_within(&slow, Duration::from_secs(3))
        .expect("the slow client was answered within its deadline");
    assert!(
        started.elapsed() < Duration::from_secs(2),
        "{:?}",
        started.elapsed()
    );
    assert!(
        reply["text"]
            .as_str()
            .unwrap()
            .contains("no whole request line"),
        "{reply}"
    );
    dribble.join().unwrap();
    drop(silent);

    // With the slots given back, a well-formed request is answered.
    let deadline = Instant::now() + Duration::from_secs(5);
    let answered = loop {
        let reply = seats::ask(&fixture.layout, SESSION, &by_shape("review"))
            .unwrap()
            .expect("the launcher is listening");
        if !reply.text.contains("hydra/seat-channel-busy") {
            break reply;
        }
        assert!(Instant::now() < deadline, "still busy: {}", reply.text);
        std::thread::sleep(Duration::from_millis(50));
    };
    assert!(!answered.is_error, "{}", answered.text);
    assert_eq!(fixture.launched().len(), 2);

    // A flood still running when the session ends does not keep the
    // harvest from writing `run_end`.
    let flooding = Arc::new(std::sync::atomic::AtomicBool::new(true));
    let flood = {
        let flooding = flooding.clone();
        let socket = socket.clone();
        std::thread::spawn(move || {
            let mut held = Vec::new();
            while flooding.load(Ordering::SeqCst) {
                if let Ok(stream) = UnixStream::connect(&socket) {
                    held.push(stream);
                }
                std::thread::sleep(Duration::from_millis(5));
            }
        })
    };
    std::thread::sleep(Duration::from_millis(100));
    let stopping = Instant::now();
    assert!(served.stop().is_empty());
    assert!(
        stopping.elapsed() < Duration::from_secs(3),
        "{:?}",
        stopping.elapsed()
    );
    fixture.harvest();
    flooding.store(false, Ordering::SeqCst);
    flood.join().unwrap();
}

/// A launch that blocks inside `run::run` until the test lets it go, and
/// hands off only when `hands_off` says so.
struct Held {
    entered: mpsc::Receiver<String>,
    release: mpsc::Sender<()>,
    finish: mpsc::Sender<()>,
}

fn held_launch(hands_off: bool) -> (Held, Box<seats::LaunchSeat>) {
    let (entered_tx, entered) = mpsc::channel();
    let (release, release_rx) = mpsc::channel::<()>();
    let (finish, finish_rx) = mpsc::channel::<()>();
    let state = Mutex::new((entered_tx, release_rx, finish_rx));
    let launch = Box::new(move |mut request: Request| {
        let state = state.lock().unwrap();
        let run_id = request.run_id.clone().unwrap_or_default();
        state.0.send(run_id.clone()).unwrap();
        // Still in `run::run`: the worktree, the skills, the fork.
        let _ = state.1.recv();
        if hands_off && let Some(handed_off) = request.handed_off.take() {
            handed_off();
        }
        // Past the handoff, the view waits on the run's end.
        let _ = state.2.recv();
        Err(anyhow::anyhow!("seat {run_id} stubbed"))
    });
    (
        Held {
            entered,
            release,
            finish,
        },
        launch,
    )
}

#[test]
fn the_launcher_waits_for_a_seat_still_launching_before_the_harvest() {
    let (held, launch) = held_launch(true);
    let fixture = Fixture::launching("permissive", |_| launch);
    let socket = seats::socket_path(&fixture.layout, SESSION);
    let served = seats::serve(fixture.launcher.clone(), &socket).unwrap();

    let asker = {
        let layout = Layout::at(&fixture.root);
        std::thread::spawn(move || seats::ask(&layout, SESSION, &one_seat("claude", "standard")))
    };
    let seat = held
        .entered
        .recv_timeout(Duration::from_secs(5))
        .expect("the seat's launch started");

    let (stopped_tx, stopped) = mpsc::channel();
    let stopper = std::thread::spawn(move || {
        stopped_tx
            .send(served.stop_within(Duration::from_secs(10)))
            .unwrap();
    });
    assert!(
        stopped.recv_timeout(Duration::from_millis(300)).is_err(),
        "stop returned while seat {seat} was still inside run::run"
    );

    held.release.send(()).unwrap();
    let missed = stopped
        .recv_timeout(Duration::from_secs(5))
        .expect("stop returned once the seat handed off");
    assert!(missed.is_empty(), "{missed:?}");
    stopper.join().unwrap();
    // Stop waits for the handoff, not for the seat's end.
    fixture.harvest();

    held.finish.send(()).unwrap();
    let _ = asker.join().unwrap();
}

#[test]
fn a_seat_that_never_hands_off_is_named_when_the_wait_runs_out() {
    let (held, launch) = held_launch(false);
    let fixture = Fixture::launching("permissive", |_| launch);
    let socket = seats::socket_path(&fixture.layout, SESSION);
    let served = seats::serve(fixture.launcher.clone(), &socket).unwrap();

    let asker = {
        let layout = Layout::at(&fixture.root);
        std::thread::spawn(move || seats::ask(&layout, SESSION, &one_seat("claude", "standard")))
    };
    let seat = held
        .entered
        .recv_timeout(Duration::from_secs(5))
        .expect("the seat's launch started");

    let started = Instant::now();
    let missed = served.stop_within(Duration::from_millis(300));
    assert!(started.elapsed() >= Duration::from_millis(300));
    assert_eq!(missed, vec![seat.clone()]);
    assert_eq!(fixture.requests()[0]["runs"][0], seat.as_str());
    fixture.harvest();

    held.release.send(()).unwrap();
    held.finish.send(()).unwrap();
    let _ = asker.join().unwrap();
}

#[test]
fn a_seat_launch_is_told_to_keep_off_the_humans_terminal() {
    // `run::run` prints its `hydra: run ...` lines on stderr, and a seat's
    // stderr is the launcher's, which is the terminal `claude` draws on.
    let fixture = Fixture::new("permissive");
    let reply = fixture.launcher.answer(&one_seat("claude", "standard"));
    assert!(!reply.is_error, "{}", reply.text);
    let launched = fixture.launched();
    assert_eq!(launched.len(), 1);
    assert!(launched[0].quiet, "{:?}", launched[0]);
}
