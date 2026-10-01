//! `puppets.py` and `hydra-puppets` on the same goal logs must print the same thing.
//!
//! The inputs are every goal log the Python tests leave on disk (`harvest.py` copies each
//! test's puppets home, scores file and vendors file just before its tearDown), plus the edge
//! logs below. For each, `show`, `board`, `pending --json`, `queue`, `repair` and
//! `repair --yes` run under both, and stdout, stderr and the exit code must be equal.
//!
//! Three things are normalized, and only those:
//! - `pending --json` prints an `argv` whose first element is the Python's `sys.executable`;
//!   the Rust prints `python3`. The script path after it is the same canonical path in both.
//! - `repair --yes` names the torn file with the time it ran, in stdout and on disk.
//! - Where the Python dies with a traceback, the Rust must exit 1 with the same stdout, and
//!   its one-line error must name the Python's exception class; the traceback is not compared.
//! - A damaged log prints the same named error on stderr in both, with no traceback.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

const RUST: &str = env!("CARGO_BIN_EXE_hydra-puppets");

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

fn script() -> PathBuf {
    repo().join("integrations/puppets/puppets.py")
}

/// A scratch directory under the system temp dir, removed on drop.
struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Scratch {
        let path =
            std::env::temp_dir().join(format!("hydra-puppets-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        Scratch(path.canonicalize().unwrap())
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// One goal log to compare: a case directory holding `puppets/`, maybe `grader/scores.jsonl`
/// and `vendors.json`, and the goal id to ask about.
#[derive(Clone)]
struct Input {
    label: String,
    dir: PathBuf,
    goal: String,
}

#[derive(Debug, PartialEq)]
struct Outcome {
    code: i32,
    stdout: String,
    stderr: String,
}

fn run(program: &[&str], args: &[&str], dir: &Path) -> Outcome {
    let mut cmd = Command::new(program[0]);
    cmd.args(&program[1..]).args(args);
    for var in [
        "HYDRA_PUPPETS_HOME",
        "HYDRA_ROOT",
        "HYDRA_GRADER_HOME",
        "HYDRA_PUPPETS_VENDORS",
        "HYDRA_BIN",
    ] {
        cmd.env_remove(var);
    }
    cmd.env("HYDRA_PUPPETS_HOME", dir.join("puppets"))
        .env("HYDRA_ROOT", dir.join("root"))
        .env("HYDRA_GRADER_HOME", dir.join("grader"))
        .env("PYTHONDONTWRITEBYTECODE", "1");
    if dir.join("vendors.json").is_file() {
        cmd.env("HYDRA_PUPPETS_VENDORS", dir.join("vendors.json"));
    }
    let output = cmd.output().unwrap_or_else(|e| panic!("{program:?}: {e}"));
    Outcome {
        code: output.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    }
}

fn python_executable() -> String {
    let out = Command::new("python3")
        .args(["-c", "import sys; print(sys.executable)"])
        .output()
        .unwrap();
    String::from_utf8(out.stdout).unwrap().trim().to_string()
}

/// JSON-escape a path the way `json.dumps` would print it inside `argv`.
fn json_string(text: &str) -> String {
    let mut out = String::from("\"");
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            ' '..='~' => out.push(c),
            c => {
                let mut units = [0u16; 2];
                for unit in c.encode_utf16(&mut units) {
                    out.push_str(&format!("\\u{unit:04x}"));
                }
            }
        }
    }
    out.push('"');
    out
}

/// Replace the `ts` on a `repair` event line so two runs at different times compare equal.
fn mask_repair_ts(content: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    for line in content.split_inclusive(|b| *b == b'\n') {
        const REPAIR: &[u8] = b"\"event\":\"repair\"";
        if line.windows(REPAIR.len()).any(|w| w == REPAIR) {
            let text = String::from_utf8_lossy(line);
            if let Some(start) = text.find("\"ts\":\"") {
                let rest = &text[start + 6..];
                if let Some(end) = rest.find('"') {
                    let mut masked = text[..start].to_string();
                    masked.push_str("\"ts\":\"<ts>\"");
                    masked.push_str(&rest[end + 1..]);
                    out.extend_from_slice(masked.as_bytes());
                    continue;
                }
            }
        }
        out.extend_from_slice(line);
    }
    out
}

/// Replace the time in `<goal>.torn-<time>.txt` with `<ts>`.
fn mask_torn_stamp(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(at) = rest.find(".torn-") {
        let (head, tail) = rest.split_at(at + ".torn-".len());
        out.push_str(head);
        if tail.len() >= 24 + 4 && tail.is_char_boundary(24) && tail[24..].starts_with(".txt") {
            out.push_str("<ts>");
            rest = &tail[24..];
        } else {
            rest = tail;
        }
    }
    out.push_str(rest);
    out
}

/// The exception class on a traceback's last line, without its module.
fn exception_class(stderr: &str) -> Option<String> {
    if !stderr.starts_with("Traceback (most recent call last):") {
        return None;
    }
    let last = stderr.trim_end().lines().last()?;
    let class = last.split(':').next()?;
    Some(class.rsplit('.').next()?.to_string())
}

/// The files a case directory holds after a command, names masked, for `repair --yes`.
fn tree(dir: &Path) -> Vec<(String, Vec<u8>)> {
    let mut files: Vec<(String, Vec<u8>)> = std::fs::read_dir(dir.join("puppets"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.is_file())
        .map(|p| {
            let bytes = std::fs::read(&p).unwrap();
            let bytes = if p.extension().is_some_and(|e| e == "jsonl") {
                mask_repair_ts(&bytes)
            } else {
                bytes
            };
            (
                mask_torn_stamp(&p.file_name().unwrap().to_string_lossy()),
                bytes,
            )
        })
        .collect();
    files.sort();
    files
}

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let path = entry.unwrap().path();
        let dest = to.join(path.file_name().unwrap());
        if path.is_dir() {
            copy_dir(&path, &dest);
        } else {
            std::fs::copy(&path, &dest).unwrap();
        }
    }
}

struct Report {
    compared: usize,
    crashes: Vec<String>,
    mismatches: Vec<String>,
}

/// Compare one command on one input; `scratch` is where `repair --yes` gets its copies.
fn compare(
    input: &Input,
    args: &[&str],
    python_exe: &str,
    scratch: &Path,
    n: usize,
    report: &Mutex<Report>,
) {
    let script = script();
    let script = script.to_str().unwrap();
    let writes = args.contains(&"--yes");
    let (py_dir, rs_dir) = if writes {
        let py_dir = scratch.join(format!("{n}-py"));
        let rs_dir = scratch.join(format!("{n}-rs"));
        copy_dir(&input.dir, &py_dir);
        copy_dir(&input.dir, &rs_dir);
        (py_dir, rs_dir)
    } else {
        (input.dir.clone(), input.dir.clone())
    };
    let mut py = run(&["python3", script], args, &py_dir);
    let mut rs = run(&[RUST], args, &rs_dir);
    let what = format!("{} `{}`", input.label, args.join(" "));
    let mut problems = Vec::new();
    if args[0] == "pending" {
        py.stdout = py.stdout.replace(&json_string(python_exe), "\"python3\"");
    }
    if writes {
        py.stdout = mask_torn_stamp(&py.stdout);
        rs.stdout = mask_torn_stamp(&rs.stdout);
        if tree(&py_dir) != tree(&rs_dir) {
            problems.push("the files left behind differ".to_string());
        }
    }
    let mut report = report.lock().unwrap();
    report.compared += 1;
    match exception_class(&py.stderr) {
        Some(class) => {
            report.crashes.push(format!("{what}: {class}"));
            if py.code != rs.code || py.stdout != rs.stdout || !rs.stderr.contains(&class) {
                problems.push(format!(
                    "python raised {class}; rust gave {rs:?}; python stdout {:?}",
                    py.stdout
                ));
            }
        }
        None => {
            if py != rs {
                problems.push(format!("python {py:#?}\nrust {rs:#?}"));
            }
        }
    }
    if !problems.is_empty() {
        report
            .mismatches
            .push(format!("{what}\n{}", problems.join("\n")));
    }
}

const COMMANDS: [&[&str]; 6] = [
    &["show"],
    &["board"],
    &["pending", "--json"],
    &["queue"],
    &["repair"],
    &["repair", "--yes"],
];

fn compare_all(inputs: &[Input], commands: &[&[&str]], scratch: &Path) -> Report {
    let python_exe = python_executable();
    let report = Mutex::new(Report {
        compared: 0,
        crashes: Vec::new(),
        mismatches: Vec::new(),
    });
    let jobs: Vec<(usize, &Input, Vec<&str>)> = inputs
        .iter()
        .flat_map(|input| commands.iter().map(move |cmd| (input, cmd)))
        .enumerate()
        .map(|(n, (input, cmd))| {
            let mut args = cmd.to_vec();
            args.insert(1, input.goal.as_str());
            (n, input, args)
        })
        .collect();
    let next = Mutex::new(0usize);
    std::thread::scope(|s| {
        for _ in 0..4 {
            s.spawn(|| {
                loop {
                    let n = {
                        let mut next = next.lock().unwrap();
                        let n = *next;
                        *next += 1;
                        n
                    };
                    let Some((id, input, args)) = jobs.get(n) else {
                        break;
                    };
                    compare(input, args, &python_exe, scratch, *id, &report);
                }
            });
        }
    });
    report.into_inner().unwrap()
}

/// A label, the log's bytes, `scores.jsonl`, and `vendors.json`.
type EdgeCase<'a> = (&'a str, Vec<u8>, Option<String>, Option<&'a str>);

/// Goal logs and home files the Python tests do not build.
fn edge_inputs(root: &Path) -> Vec<Input> {
    const GOAL: &str = r#"{"seq":1,"ts":"2026-09-30T12:00:00.000Z","event":"goal","text":"test","repo":"/repo","base_sha":"abc"}"#;
    const TASK: &str = r#"{"seq":2,"ts":"2026-09-30T12:00:00.000Z","event":"task","id":"t1","text":"do work","class":"batch_scripting","grade":"low","deps":[],"paths":[]}"#;
    let line = |s: &str| format!("{s}\n");
    let busy = [
        GOAL,
        r#"{"event":"plan_proposed","provider":"codex","model":null,"run_id":"11111111-aaaa"}"#,
        r#"{"event":"plan_proposed","provider":"cursor-agent","model":"composer-2.5","run_id":"22222222-bbbb"}"#,
        r#"{"event":"plan_ready","provider":"codex","run_id":"11111111-aaaa","path":"/p/codex.md"}"#,
        r#"{"event":"plan_ready","provider":"cursor-agent","run_id":"22222222-bbbb","path":"/p/cursor.md"}"#,
        r#"{"event":"panel_verdict","judge_run_ids":["a","b"],"chosen":"codex","path":"/p/codex.md","degraded":true,"substitutions":[{"missing":"codex","substitute":"cursor-agent","model":""}]}"#,
        r#"{"event":"task","id":"t1","text":"café ☕ 😀","class":"rust_launch_path","grade":"high","deps":"ab","paths":["src/x.rs"]}"#,
        r#"{"event":"task","id":"t2","text":"two","class":"docs","grade":"low"}"#,
        r#"{"event":"task","id":"t3","text":"three","class":"security_review","grade":"medium","deps":["t1"],"paths":[]}"#,
        r#"{"event":"assignment","task":"t1","provider":"claude","model":"sonnet","policy":"standard","source":"sheet"}"#,
        r#"{"event":"assignment","task":"t2","provider":"cursor-agent","model":"composer-2.5","policy":"standard","source":"default"}"#,
        r#"{"event":"assignment","task":"t3","provider":"codex","model":"","policy":"standard","source":"default"}"#,
        r#"{"event":"assignment","task":"t4","provider":"agy","policy":"judge","source":"human"}"#,
        r#"{"event":"approval","kind":"table","ref":null,"revision":7,"yes":true}"#,
        r#"{"event":"approval","kind":"launch","ref":"t3","revision":7.0,"yes":true}"#,
        r#"{"event":"approval","kind":"plan","ref":null,"revision":1,"yes":true,"extra":1e16}"#,
        r#"{"event":"launch_intent","task":"t2","attempt":1,"provider":"cursor-agent","model":"composer-2.5","policy":"standard","argv_sha256":"x"}"#,
        r#"{"event":"run","task":"t2","attempt":1,"run_id":"33333333-cccc-dddd"}"#,
        r#"{"event":"run_end","task":"t2","run_id":"33333333-cccc-dddd","outcome":"completed","total_tokens":1.5e-7}"#,
        r#"{"event":"launch_intent","task":"t3","attempt":1,"provider":"codex"}"#,
        r#"{"event":"run","task":"t3","attempt":1,"run_id":"44444444-eeee"}"#,
        r#"{"event":"run_end","task":"t3","run_id":"44444444-eeee","outcome":"failed"}"#,
        r#"{"event":"review","task":"t3","judge_run_ids":["j"],"verdict":"reject","degraded":true,"substitutions":[{"missing":"codex","substitute":"claude"}]}"#,
        r#"{"event":"launch_intent","task":"t4","attempt":2}"#,
        r#"{"event":"human_plan","accepted":true}"#,
        r#"{"event":"note","text":"ignored"}"#,
    ]
    .map(line)
    .concat();
    let scores = [
        r#"{"label":"verdict","model_version":"v1","run_id":"33333333-cccc-dddd","p":0.125,"scored_at":"2026-09-01"}"#,
        r#"{"label":"verdict","model_version":"v2","run_id":"44444444-eeee","p":1,"scored_at":"2026-09-02"}"#,
        r#"{"label":"outcome","model_version":"v3","run_id":"33333333-cccc-dddd","p":0.9,"scored_at":"2026-09-09"}"#,
        "not json",
        "",
    ]
    .map(line)
    .concat();
    let unavailable = r#"[{"provider":"codex","model":null,"available":false},{"provider":"claude","model":"opus","available":false},{"provider":"claude","model":null}]"#;
    let torn_long = format!("{}{}", "é".repeat(310), "\u{0}\u{7}");
    let mut cases: Vec<EdgeCase> = vec![
        (
            "unterminated tail",
            [line(GOAL), TASK.into()].concat().into_bytes(),
            None,
            None,
        ),
        (
            "CR tail",
            [
                line(GOAL),
                r#"{"event":"assignment","task":"t1"}"#.into(),
                "\r".into(),
            ]
            .concat()
            .into_bytes(),
            None,
            None,
        ),
        (
            "bad interior line",
            [line(GOAL), line("not json"), line(TASK)]
                .concat()
                .into_bytes(),
            None,
            None,
        ),
        (
            "bad last line",
            [line(GOAL), line(TASK), line("{\"event\":")]
                .concat()
                .into_bytes(),
            None,
            None,
        ),
        (
            "blank interior line",
            [line(GOAL), "\n".into(), line(TASK)].concat().into_bytes(),
            None,
            None,
        ),
        ("empty", Vec::new(), None, None),
        (
            "CRLF lines",
            format!("{GOAL}\r\n{TASK}\r\n").into_bytes(),
            None,
            None,
        ),
        (
            "BOM on the first line",
            [b"\xef\xbb\xbf".to_vec(), line(GOAL).into_bytes()].concat(),
            None,
            None,
        ),
        ("no goal event", line(TASK).into_bytes(), None, None),
        (
            "missing key",
            line(r#"{"event":"goal","text":"x"}"#).into_bytes(),
            None,
            None,
        ),
        (
            "event not an object",
            [line(GOAL), line("[1,2]")].concat().into_bytes(),
            None,
            None,
        ),
        (
            "invalid utf-8 torn tail",
            [line(GOAL).into_bytes(), b"{\"event\":\xff\xfe".to_vec()].concat(),
            None,
            None,
        ),
        (
            "long torn tail",
            [line(GOAL), torn_long].concat().into_bytes(),
            None,
            None,
        ),
        (
            "busy goal",
            busy.clone().into_bytes(),
            Some(scores.clone()),
            None,
        ),
        (
            "busy goal, seats down",
            busy.clone().into_bytes(),
            None,
            Some(unavailable),
        ),
        (
            "busy goal, unterminated",
            [busy.clone(), GOAL.into()].concat().into_bytes(),
            Some(scores),
            Some(unavailable),
        ),
        (
            "two bad lines and torn tail",
            [
                line(GOAL),
                line("bad1"),
                "\n".into(),
                line(TASK),
                "{\"event\":\"x\"".into(),
            ]
            .concat()
            .into_bytes(),
            None,
            None,
        ),
        (
            "a repair event between two table approvals",
            [
                line(GOAL),
                line(TASK),
                line(r#"{"event":"assignment","task":"t1","provider":"codex","model":"","policy":"standard","source":"default"}"#),
                line(r#"{"seq":4,"ts":"t","event":"approval","kind":"table","ref":null,"revision":2,"yes":true}"#),
                line(r#"{"seq":5,"ts":"t","event":"approval","kind":"plan","ref":null,"revision":2,"yes":true}"#),
                line(r#"{"seq":6,"ts":"t","event":"repair","removed":2}"#),
                line(r#"{"seq":7,"ts":"t","event":"approval","kind":"table","ref":null,"revision":5,"yes":true}"#),
                line(r#"{"seq":8,"ts":"t","event":"repair","removed":"x"}"#),
                line(r#"{"seq":9,"ts":"t","event":"repair","removed":true}"#),
                line(r#"{"seq":10,"ts":"t","event":"repair","removed":-3}"#),
                line(r#"{"seq":11,"ts":"t","event":"repair"}"#),
            ]
            .concat()
            .into_bytes(),
            None,
            None,
        ),
        (
            "invalid utf-8 inside a terminated line",
            [
                line(GOAL).into_bytes(),
                b"{\"event\":\"note\",\"text\":\"\xff\"}\n".to_vec(),
                line(TASK).into_bytes(),
            ]
            .concat(),
            None,
            None,
        ),
        (
            "repair voids table approval",
            [
                line(GOAL),
                line(TASK),
                line(
                    r#"{"seq":3,"ts":"t","event":"approval","kind":"table","revision":1,"yes":true}"#,
                ),
                line(r#"{"seq":4,"ts":"t","event":"repair","removed":1}"#),
                line("not json"),
            ]
            .concat()
            .into_bytes(),
            None,
            None,
        ),
    ];
    let mut inputs = Vec::new();
    for (n, (label, log, scores, vendors)) in cases.drain(..).enumerate() {
        let dir = root.join(format!("edge-{n:02}"));
        std::fs::create_dir_all(dir.join("puppets")).unwrap();
        std::fs::write(dir.join("puppets/g1.jsonl"), log).unwrap();
        if let Some(scores) = scores {
            std::fs::create_dir_all(dir.join("grader")).unwrap();
            std::fs::write(dir.join("grader/scores.jsonl"), scores).unwrap();
        }
        if let Some(vendors) = vendors {
            std::fs::write(dir.join("vendors.json"), vendors).unwrap();
        }
        inputs.push(Input {
            label: format!("edge: {label}"),
            dir,
            goal: "g1".into(),
        });
    }
    let dir = root.join("edge-absent");
    std::fs::create_dir_all(dir.join("puppets")).unwrap();
    inputs.push(Input {
        label: "edge: no log for the goal".into(),
        dir,
        goal: "absent".into(),
    });
    inputs
}

/// Every goal log the Python tests leave on disk, one input per log, repeats dropped.
fn harvested_inputs(root: &Path) -> Vec<Input> {
    let out = root.join("harvest");
    std::fs::create_dir_all(&out).unwrap();
    let harvest = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/harvest.py");
    let status = Command::new("python3")
        .arg(&harvest)
        .arg(repo().join("integrations/puppets/tests"))
        .arg(&out)
        .env("PYTHONDONTWRITEBYTECODE", "1")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .unwrap();
    assert!(status.success(), "harvest.py failed");
    let mut seen = HashSet::new();
    let mut inputs = Vec::new();
    let mut cases: Vec<PathBuf> = std::fs::read_dir(&out)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    cases.sort();
    for case in cases {
        let mut logs: Vec<PathBuf> = std::fs::read_dir(case.join("puppets"))
            .unwrap()
            .map(|e| e.unwrap().path())
            .collect();
        logs.sort();
        for log in logs {
            let goal = log.file_stem().unwrap().to_string_lossy().into_owned();
            let key = (
                goal.clone(),
                std::fs::read(&log).unwrap(),
                std::fs::read(case.join("grader/scores.jsonl")).ok(),
                std::fs::read(case.join("vendors.json")).ok(),
            );
            if !seen.insert(key) {
                continue;
            }
            // One log per directory, so `repair --yes` and the file comparison see only it.
            let dir = root.join(format!("h-{}", inputs.len()));
            std::fs::create_dir_all(dir.join("puppets")).unwrap();
            std::fs::copy(&log, dir.join("puppets").join(log.file_name().unwrap())).unwrap();
            if case.join("grader").is_dir() {
                copy_dir(&case.join("grader"), &dir.join("grader"));
            }
            if case.join("vendors.json").is_file() {
                std::fs::copy(case.join("vendors.json"), dir.join("vendors.json")).unwrap();
            }
            let name = case.file_name().unwrap().to_string_lossy().into_owned();
            inputs.push(Input {
                label: format!("harvest {name}/{goal}"),
                dir,
                goal,
            });
        }
    }
    assert!(
        inputs.len() > 50,
        "harvest found only {} goal logs",
        inputs.len()
    );
    inputs
}

fn finish(report: Report, inputs: &[Input]) {
    eprintln!(
        "compared {} command runs over {} inputs",
        report.compared,
        inputs.len()
    );
    for input in inputs.iter().filter(|i| i.label.starts_with("edge")) {
        eprintln!("  {}", input.label);
    }
    eprintln!(
        "python raised, stderr compared by exception class ({}):",
        report.crashes.len()
    );
    for crash in &report.crashes {
        eprintln!("  {crash}");
    }
    assert!(
        report.mismatches.is_empty(),
        "{} of {} differ:\n\n{}",
        report.mismatches.len(),
        report.compared,
        report.mismatches.join("\n\n")
    );
}

#[test]
fn the_read_commands_match_puppets_py_on_every_test_log_and_edge_log() {
    let scratch = Scratch::new("diff");
    let mut inputs = harvested_inputs(&scratch.0);
    inputs.extend(edge_inputs(&scratch.0));
    let copies = scratch.0.join("copies");
    std::fs::create_dir_all(&copies).unwrap();
    let report = compare_all(&inputs, &COMMANDS, &copies);
    finish(report, &inputs);
}

/// The session runs this by hand: `cargo test -p hydra-puppets -- --ignored`. It copies the
/// real goal logs and scores into a scratch home and runs every read command but
/// `repair --yes`, so the real files are only read.
#[test]
#[ignore = "reads the real ~/hydra/puppets; run by the session"]
fn the_read_commands_match_puppets_py_on_the_real_goal_logs() {
    let home = std::env::home_dir().unwrap();
    let source = std::env::var_os("HYDRA_PUPPETS_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join("hydra/puppets"));
    let grader = std::env::var_os("HYDRA_GRADER_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join("hydra/grader"));
    let scratch = Scratch::new("real");
    let mut inputs = Vec::new();
    let mut logs: Vec<PathBuf> = std::fs::read_dir(&source)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.is_file() && p.extension().is_some_and(|e| e == "jsonl"))
        .collect();
    logs.sort();
    for log in logs {
        let goal = log.file_stem().unwrap().to_string_lossy().into_owned();
        let dir = scratch.0.join(&goal);
        std::fs::create_dir_all(dir.join("puppets")).unwrap();
        std::fs::copy(&log, dir.join("puppets").join(log.file_name().unwrap())).unwrap();
        if grader.join("scores.jsonl").is_file() {
            std::fs::create_dir_all(dir.join("grader")).unwrap();
            std::fs::copy(grader.join("scores.jsonl"), dir.join("grader/scores.jsonl")).unwrap();
        }
        inputs.push(Input {
            label: format!("real {goal}"),
            dir,
            goal,
        });
    }
    assert!(
        !inputs.is_empty(),
        "no goal logs under {}",
        source.display()
    );
    let copies = scratch.0.join("copies");
    std::fs::create_dir_all(&copies).unwrap();
    let report = compare_all(&inputs, &COMMANDS[..5], &copies);
    finish(report, &inputs);
}
