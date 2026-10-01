//! `puppets.py` and `hydra-puppets` on the same goal logs must print the same thing.
//!
//! The inputs are every goal log the Python tests leave on disk (`harvest.py` copies each
//! test's puppets home, scores file and vendors file just before its tearDown), plus the edge
//! logs below. For each, `show`, `board`, `pending --json`, `queue`, `repair` and
//! `repair --yes` run under both, and stdout, stderr and the exit code must be equal. After
//! `repair --yes` each implementation's repaired copy is read back with `show` and
//! `pending --json`, and those must be equal too.
//!
//! These are normalized, and only these:
//! - `pending --json` prints an `argv` that starts with the command that records the
//!   approval. For the Python that is two elements, its `sys.executable` and its own path;
//!   since build 196 the Rust prints one, its own path. Only that pair is replaced, by the
//!   Rust's one.
//! - `repair --yes` names the torn file with the time it ran, in stdout and on disk, and
//!   stamps the `repair` event it appends. Only a name whose 24 characters are a time of
//!   `utc_now()`'s shape is masked, and only the `ts` of the one line the command appended.
//! - Where the Python dies with a traceback, the Rust must exit 1 with the same stdout, and
//!   its one-line error must start `hydra-puppets: <Class>:` with the Python's exception
//!   class; the rest of the traceback is not compared.
//! - A damaged log prints the same named error on stderr in both, with no traceback.
//!
//! The commands that write (`new`, `set`, `approve`, `verdict`, `choose`, `tasks`, `assign`,
//! `reconcile`) each run on a copy of the same starting home per implementation, and the
//! goal logs left behind must be equal line by line. Two more things are normalized there:
//! - the `ts` of each line the command appended, and of no line the input already held;
//! - the goal id `new` generates, in its stdout and in the name of the log it creates.
//!
//! The commands that launch (`plan`, `plans`, `launch`, `wait`, `review`, `choose --judge`)
//! run with a stub in place of `hydra`, `hunk` and `herdr`. The stub writes down each call,
//! and the calls must be equal too: argv, environment, working directory and stdin. One more
//! thing is normalized there: each implementation's own scratch home, wherever it appears.

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
    run_in(program, args, dir, None)
}

/// `run`, from a working directory: `new` reads the repo from it and `choose` resolves a
/// relative plan path against it.
fn run_in(program: &[&str], args: &[&str], dir: &Path, cwd: Option<&Path>) -> Outcome {
    let mut cmd = Command::new(program[0]);
    cmd.args(&program[1..]).args(args);
    if let Some(cwd) = cwd {
        cmd.current_dir(cwd);
    }
    cmd.stdin(std::process::Stdio::null());
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

/// Mask the `ts` of the `repair` event `repair --yes` appended, so two runs at different
/// times compare equal. Only that line: the log's last, when the input did not hold it and
/// it has exactly the shape the command writes. A `repair` event that was already in the
/// input keeps its `ts`, and so does one nested in another event.
fn mask_repair_ts(before: &[u8], after: &[u8]) -> Vec<u8> {
    let Some(last) = after.split_inclusive(|b| *b == b'\n').next_back() else {
        return after.to_vec();
    };
    if before
        .split_inclusive(|b| *b == b'\n')
        .any(|line| line == last)
    {
        return after.to_vec();
    }
    let Some((head, tail)) = appended_repair_line(last) else {
        return after.to_vec();
    };
    let mut out = after[..after.len() - last.len()].to_vec();
    out.extend_from_slice(head.as_bytes());
    out.extend_from_slice(b"<ts>");
    out.extend_from_slice(tail.as_bytes());
    out
}

/// Split `{"seq":N,"ts":"<stamp>","event":"repair","removed":N}` and its newline around the
/// stamp, when the line is exactly that and the stamp has `utc_now()`'s shape.
fn appended_repair_line(line: &[u8]) -> Option<(&str, &str)> {
    let line = std::str::from_utf8(line).ok()?;
    let digits = |text: &str| !text.is_empty() && text.bytes().all(|b| b.is_ascii_digit());
    let (seq, rest) = line.strip_prefix("{\"seq\":")?.split_once(",\"ts\":\"")?;
    let stamp = rest.get(..24)?;
    let removed = rest
        .get(24..)?
        .strip_prefix("\",\"event\":\"repair\",\"removed\":")?
        .strip_suffix("}\n")?;
    if !digits(seq) || !digits(removed) || !is_utc_stamp(stamp) {
        return None;
    }
    let stamp_at = line.len() - rest.len();
    Some((&line[..stamp_at], &line[stamp_at + 24..]))
}

/// `pending --json` under the Python starts each `argv` with `sys.executable` and the
/// script; make that pair the one element the Rust prints, `helper`. Only that pair: the
/// two lines after an item's `"argv": [`. A task named like the executable keeps its name
/// everywhere else. A JSON string cannot hold a raw newline, so a line that
/// is exactly an item's `argv` key at that indent is one.
fn mask_argv0(stdout: &str, python_exe: &str, script: &str, helper: &str) -> String {
    let argv0 = format!("      {},", json_string(python_exe));
    let argv1 = format!("      {},", json_string(script));
    let mut out = String::new();
    let mut lines = stdout.split_inclusive('\n').peekable();
    let mut after_argv = false;
    while let Some(line) = lines.next() {
        let body = line.strip_suffix('\n').unwrap_or(line);
        let next = lines
            .peek()
            .map(|next| next.strip_suffix('\n').unwrap_or(next));
        if after_argv && body == argv0 && next == Some(argv1.as_str()) {
            lines.next();
            out.push_str(&format!("      {},", json_string(helper)));
            out.push_str(&line[body.len()..]);
        } else {
            out.push_str(line);
        }
        after_argv = body == "    \"argv\": [";
    }
    out
}

/// The Rust helper as its own `pending --json` names it: its canonical path.
fn helper() -> String {
    let path = Path::new(RUST).canonicalize().unwrap();
    path.to_str().unwrap().to_string()
}

/// Whether the Rust's stderr starts by naming the exception class the Python raised:
/// `hydra-puppets: <Class>: ...`. A class named further into the message does not count.
fn names_class(rust_stderr: &str, class: &str) -> bool {
    rust_stderr
        .strip_prefix("hydra-puppets: ")
        .and_then(|rest| rest.split_once(':'))
        .is_some_and(|(named, _)| named == class)
}

/// Whether `text` has the shape `utc_now()` writes: `2026-10-01T09:08:07.123Z`.
fn is_utc_stamp(text: &str) -> bool {
    const SHAPE: &[u8; 24] = b"dddd-dd-ddTdd:dd:dd.dddZ";
    text.len() == SHAPE.len()
        && text.bytes().zip(SHAPE).all(|(got, want)| match want {
            b'd' => got.is_ascii_digit(),
            other => got == *other,
        })
}

/// Replace the time in `<goal>.torn-<time>.txt` with `<ts>`, where it is a time.
fn mask_torn_stamp(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(at) = rest.find(".torn-") {
        let (head, tail) = rest.split_at(at + ".torn-".len());
        out.push_str(head);
        if tail.len() >= 24 + 4
            && tail.is_char_boundary(24)
            && is_utc_stamp(&tail[..24])
            && tail[24..].starts_with(".txt")
        {
            out.push_str("<ts>");
            rest = &tail[24..];
        } else {
            rest = tail;
        }
    }
    out.push_str(rest);
    out
}

/// The exception class a traceback ends on, without its module. The line that names it is
/// the last one that is not indented and follows a frame's indented line, so a message that
/// runs over several lines is not read as a class.
fn exception_class(stderr: &str) -> Option<String> {
    if !stderr.starts_with("Traceback (most recent call last):") {
        return None;
    }
    let lines: Vec<&str> = stderr.trim_end().lines().collect();
    let raised = lines
        .windows(2)
        .rfind(|pair| pair[0].starts_with(' ') && !pair[1].starts_with(' '))?;
    let class = raised[1].split(':').next()?;
    Some(class.rsplit('.').next()?.to_string())
}

/// The files a case directory holds after a command, names masked, for `repair --yes`.
/// `original` is the directory the command's copy was made from.
fn tree(dir: &Path, original: &Path) -> Vec<(String, Vec<u8>)> {
    let mut files: Vec<(String, Vec<u8>)> = std::fs::read_dir(dir.join("puppets"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.is_file())
        .map(|p| {
            let bytes = std::fs::read(&p).unwrap();
            let bytes = if p.extension().is_some_and(|e| e == "jsonl") {
                let before = original.join("puppets").join(p.file_name().unwrap());
                mask_repair_ts(&std::fs::read(before).unwrap_or_default(), &bytes)
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

/// Whether `outcome` is argparse refusing the command line: exit 2, the usage block, and
/// the `error:` line last.
fn is_usage_error(outcome: &Outcome) -> bool {
    let last = outcome.stderr.trim_end().lines().last().unwrap_or("");
    outcome.code == 2 && outcome.stderr.starts_with("usage: ") && last.contains(": error: ")
}

/// What differs between the two outcomes of one command, and the exception class when the
/// Python died with a traceback. A traceback is compared by exit code, stdout and class.
/// A command line argparse refuses is compared by exit code and stdout, and clap must have
/// refused it too: the two word the refusal differently.
fn judge(py: &Outcome, rs: &Outcome) -> (Option<String>, Option<String>) {
    if is_usage_error(py) {
        let same = rs.code == 2 && py.stdout == rs.stdout && rs.stderr.starts_with("error: ");
        return (
            None,
            (!same).then(|| format!("python {py:#?}\nrust {rs:#?}")),
        );
    }
    match exception_class(&py.stderr) {
        Some(class) => {
            let same =
                py.code == rs.code && py.stdout == rs.stdout && names_class(&rs.stderr, &class);
            let problem = (!same).then(|| {
                format!(
                    "python raised {class}; rust gave {rs:?}; python stdout {:?}",
                    py.stdout
                )
            });
            (Some(class), problem)
        }
        None => (
            None,
            (py != rs).then(|| format!("python {py:#?}\nrust {rs:#?}")),
        ),
    }
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
    let run_both = |args: &[&str]| {
        let mut py = run(&["python3", script], args, &py_dir);
        let rs = run(&[RUST], args, &rs_dir);
        if args[0] == "pending" {
            py.stdout = mask_argv0(&py.stdout, python_exe, script, &helper());
        }
        (py, rs)
    };
    let (mut py, mut rs) = run_both(args);
    let what = format!("{} `{}`", input.label, args.join(" "));
    let mut problems = Vec::new();
    let mut crashes = Vec::new();
    let mut compared = 1;
    if writes {
        py.stdout = mask_torn_stamp(&py.stdout);
        rs.stdout = mask_torn_stamp(&rs.stdout);
        if tree(&py_dir, &input.dir) != tree(&rs_dir, &input.dir) {
            problems.push("the files left behind differ".to_string());
        }
    }
    let (class, problem) = judge(&py, &rs);
    crashes.extend(class.map(|class| format!("{what}: {class}")));
    problems.extend(problem);
    if writes {
        // A repaired log folds again, which a damaged one never did: read each copy back
        // with the implementation that repaired it.
        for after in [&["show", args[1]][..], &["pending", args[1], "--json"]] {
            let (py, rs) = run_both(after);
            let (class, problem) = judge(&py, &rs);
            let then = format!("then `{}`", after.join(" "));
            crashes.extend(class.map(|class| format!("{what}, {then}: {class}")));
            problems.extend(problem.map(|problem| format!("{then}: {problem}")));
            compared += 1;
        }
    }
    let mut report = report.lock().unwrap();
    report.compared += compared;
    report.crashes.extend(crashes);
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

/// The inputs of build ticket 199: each is a log the two implementations read differently
/// before that ticket.
fn review_inputs(goal: &str) -> Vec<EdgeCase<'static>> {
    let line = |s: &str| format!("{s}\n");
    let task = |id: &str| {
        line(&format!(
            r#"{{"event":"task","id":{id},"text":"t","class":"c","grade":"low"}}"#
        ))
    };
    // A run that has ended and waits for a review, which is what `queue` sorts.
    let ended = |id: &str, run: &str| {
        [
            line(&format!(
                r#"{{"event":"run","task":{id},"attempt":1,"run_id":"{run}"}}"#
            )),
            line(&format!(
                r#"{{"event":"run_end","task":{id},"run_id":"{run}"}}"#
            )),
        ]
        .concat()
    };
    let score = |run: &str, p: &str| {
        line(&format!(
            r#"{{"label":"verdict","model_version":"v","run_id":"{run}","p":{p}}}"#
        ))
    };
    let utf16be = |s: &str| -> Vec<u8> { s.encode_utf16().flat_map(u16::to_be_bytes).collect() };
    let utf16le = |s: &str| -> Vec<u8> { s.encode_utf16().flat_map(u16::to_le_bytes).collect() };
    let utf32be =
        |s: &str| -> Vec<u8> { s.chars().flat_map(|c| (c as u32).to_be_bytes()).collect() };
    let wide = "1393796574908163946345982392040522594123776"; // 2**140
    let two_131 = "2722258935367507707706996859454145691648";
    let two_130 = "1361129467683753853853498429727072845824";
    let min_i128 = "-170141183460469231731687303715884105728"; // -2**127
    let past_i128 = "680564733841876926926749214863536422912"; // 2**129
    // Enough tasks to leave the insertion sort and reach the merges, a third of them NaN.
    let mut many = line(goal);
    let mut many_scores = String::new();
    let mut seed: u64 = 199;
    for n in 0..150 {
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let id = format!("\"t{n:03}\"");
        let run = format!("r{n:03}");
        many.push_str(&task(&id));
        many.push_str(&ended(&id, &run));
        match (seed >> 33) % 6 {
            0 | 1 => many_scores.push_str(&score(&run, "NaN")),
            2 => {}
            _ => many_scores.push_str(&score(&run, &format!("0.{}", (seed >> 40) % 50))),
        }
    }
    vec![
        (
            "item 1: a lone surrogate in the goal text and in a task",
            [
                line(&goal.replace("\"test\"", r#""\ud800""#)),
                line(r#"{"event":"task","id":"t1","text":"\udc00 x \ud83d\ude00","class":"c","grade":"low"}"#),
            ]
            .concat()
            .into_bytes(),
            None,
            None,
        ),
        (
            "item 1: a lone surrogate as a task id and a dict key",
            [
                line(goal),
                task(r#""\udfff""#),
                line(r#"{"event":"note","\ud800":{"\ud800\ud800":1}}"#),
            ]
            .concat()
            .into_bytes(),
            None,
            None,
        ),
        (
            "item 2: one task id as a wide int and as its float",
            [line(goal), task(wide), task("1.393796574908164e42")]
                .concat()
                .into_bytes(),
            None,
            None,
        ),
        (
            "item 2: two wide ids with tied scores",
            [
                line(goal),
                task(two_131),
                task(two_130),
                ended(two_131, "r2"),
                ended(two_130, "r1"),
            ]
            .concat()
            .into_bytes(),
            None,
            None,
        ),
        (
            "item 2: -2**127 and its exact float",
            [
                line(goal),
                task(min_i128),
                task("-1.7014118346046923e38"),
                task(past_i128),
                task("6.80564733841877e38"),
                task("1e400"),
                task("3"),
                task("3.5"),
            ]
            .concat()
            .into_bytes(),
            None,
            None,
        ),
        (
            "item 2: wide and narrow ids, ints and floats, tied",
            [
                line(goal),
                task(two_131),
                task("1.5"),
                task(&format!("-{two_130}")),
                task("7"),
                task("-1e300"),
                ended(two_131, "a"),
                ended("1.5", "b"),
                ended(&format!("-{two_130}"), "c"),
                ended("7", "d"),
                ended("-1e300", "e"),
            ]
            .concat()
            .into_bytes(),
            None,
            None,
        ),
        (
            "item 3: a NaN score ahead of a real one",
            [
                line(goal),
                task("\"z\""),
                task("\"a\""),
                ended("\"z\"", "rz"),
                ended("\"a\"", "ra"),
            ]
            .concat()
            .into_bytes(),
            Some([score("rz", "NaN"), score("ra", "0.5")].concat()),
            None,
        ),
        ("item 3: 150 tasks, a third with NaN scores", many.into_bytes(), Some(many_scores), None),
        (
            "item 5: a UTF-16BE line",
            [
                line(goal).into_bytes(),
                utf16be(&task("\"u\\ud800 é😀\"")),
                utf16be(&line(r#"{"event":"note"}"#)),
            ]
            .concat(),
            None,
            None,
        ),
        (
            "item 5: UTF-16 and UTF-32 lines with a BOM, and UTF-32BE without",
            [
                [b"\xfe\xff".to_vec(), utf16be(&line(goal))].concat(),
                [b"\x00\x00\xfe\xff".to_vec(), utf32be(&task("\"a\""))].concat(),
                utf32be(&task("\"b😀\"")),
            ]
            .concat(),
            None,
            None,
        ),
        (
            "control for item 5: a UTF-16LE line is damage in both",
            [line(goal).into_bytes(), utf16le(&task("\"u\""))].concat(),
            None,
            None,
        ),
        (
            "item 5: a surrogate as raw UTF-8 bytes",
            [
                line(goal).into_bytes(),
                b"{\"event\":\"task\",\"id\":\"\xed\xa0\x80\",\"text\":\"t\",\"class\":\"c\",\"grade\":\"low\"}\n".to_vec(),
            ]
            .concat(),
            None,
            None,
        ),
        (
            "item 5: an odd UTF-16 line, a short one and one past U+10FFFF",
            [
                line(goal).into_bytes(),
                b"\x00{\x00}\x00\n".to_vec(),
                b"\x001\n".to_vec(),
                b"\x00\x11\x00\x00\x00\x00\x00\n".to_vec(),
            ]
            .concat(),
            None,
            None,
        ),
        (
            "item 6: run_end for an unknown task with no run_id",
            [line(goal), line(r#"{"event":"run_end","task":"missing"}"#)]
                .concat()
                .into_bytes(),
            None,
            None,
        ),
        (
            "control for item 6: run_end with no run_id while a run is pending",
            [
                line(goal),
                line(r#"{"event":"run","task":"t9","attempt":1,"run_id":"r"}"#),
                line(r#"{"event":"run_end","task":"missing"}"#),
            ]
            .concat()
            .into_bytes(),
            None,
            None,
        ),
        (
            "item 7: repr of strings Python does not print raw",
            line(&goal.replace(
                "\"test\"",
                r#"["\u2028","\u200b","\u00ad","\ud800","\ue000","\u0378","a'b","\u00a0","\u3000","\ud83d\ude00","\udb40\udc01","é","\u007f","a'b\"c"]"#,
            ))
            .into_bytes(),
            None,
            None,
        ),
        (
            "item 8: an empty model version",
            line(goal).into_bytes(),
            Some(line(
                r#"{"label":"verdict","model_version":"","run_id":"r","p":0.5}"#,
            )),
            None,
        ),
        (
            "item 9: an integer of 4,301 digits",
            [
                line(goal),
                line(&format!(r#"{{"event":"note","n":{}}}"#, "1".repeat(4301))),
            ]
            .concat()
            .into_bytes(),
            None,
            None,
        ),
        (
            "control for item 9: integers of 4,300 digits and a float of 5,000",
            [
                line(goal),
                task(&"7".repeat(4300)),
                task(&format!("-{}", "7".repeat(4300))),
                task(&format!("{}.5", "7".repeat(5000))),
            ]
            .concat()
            .into_bytes(),
            None,
            None,
        ),
        (
            "item 12: two repairs that claim i64::MAX bad lines, around two approvals",
            [
                line(goal),
                task("\"t1\""),
                line(r#"{"event":"repair","removed":9223372036854775807}"#),
                line(r#"{"event":"approval","kind":"table","revision":2,"yes":true}"#),
                line(r#"{"event":"approval","kind":"table","revision":9223372036854775807,"yes":true}"#),
                line(r#"{"event":"repair","removed":9223372036854775807}"#),
            ]
            .concat()
            .into_bytes(),
            None,
            None,
        ),
        (
            "item 12: removed past i128, at a billion and one past it",
            [
                line(goal),
                task("\"t1\""),
                line(&format!(r#"{{"event":"repair","removed":{past_i128}}}"#)),
                line(r#"{"event":"repair","removed":1000000000}"#),
                line(r#"{"event":"repair","removed":1000000001}"#),
                line(r#"{"event":"approval","kind":"table","revision":1000000004,"yes":true}"#),
            ]
            .concat()
            .into_bytes(),
            None,
            None,
        ),
        (
            "item 13: a lone carriage return inside the file",
            [
                line(goal).as_str(),
                "BAD\r",
                task("\"t1\"").as_str(),
                task("\"t2\"").as_str(),
            ]
            .concat()
            .into_bytes(),
            None,
            None,
        ),
        (
            "item 13: a good line that ends in a lone carriage return, and a CR tail",
            [
                line(goal).as_str(),
                "{\"event\":\"note\"}\r",
                task("\"t1\"").as_str(),
                task("\"t2\"").as_str(),
                "{\"event\":\"note\"}\r",
            ]
            .concat()
            .into_bytes(),
            None,
            None,
        ),
        // Two more the ticket did not list, found while probing its items.
        (
            "found: NaN task ids and attempts are one dict key",
            [
                line(goal),
                task("NaN"),
                line(r#"{"event":"assignment","task":NaN,"provider":"codex","model":"","policy":"p","source":"s"}"#),
                line(r#"{"event":"launch_intent","task":NaN,"attempt":NaN}"#),
                line(r#"{"event":"run","task":NaN,"attempt":NaN,"run_id":"r"}"#),
                line(r#"{"event":"run_end","task":NaN,"run_id":"r"}"#),
                line(r#"{"event":"approval","kind":"diff","ref":NaN,"run_id":"r","revision":2,"yes":true}"#),
                line(r#"{"event":"approval","kind":"diff","ref":[NaN],"run_id":"r","revision":2,"yes":true}"#),
            ]
            .concat()
            .into_bytes(),
            None,
            None,
        ),
        (
            "found: an event nested 3,000 deep",
            [
                line(goal),
                line(&format!(
                    r#"{{"event":"note","x":{}{}}}"#,
                    "[".repeat(3000),
                    "]".repeat(3000)
                )),
                line(&format!(
                    r#"{{"event":"note","x":{}1{}}}"#,
                    r#"{"a":"#.repeat(3000),
                    "}".repeat(3000)
                )),
            ]
            .concat()
            .into_bytes(),
            None,
            None,
        ),
        (
            "item 9: 4,301 digits in the scores file",
            line(goal).into_bytes(),
            Some(line(&format!(
                r#"{{"label":"outcome","n":{}}}"#,
                "1".repeat(4301)
            ))),
            None,
        ),
    ]
}

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
    cases.extend(review_inputs(GOAL));
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

#[test]
fn the_comparator_masks_only_the_repair_line_the_command_appended() {
    let goal = "{\"seq\":1,\"event\":\"goal\"}\n";
    let nested = "{\"seq\":2,\"ts\":\"2026-01-01T00:00:00.000Z\",\"event\":\"note\",\"x\":{\"event\":\"repair\",\"ts\":\"then\"}}\n";
    let earlier =
        "{\"seq\":3,\"ts\":\"2026-09-30T12:00:00.000Z\",\"event\":\"repair\",\"removed\":1}\n";
    let appended =
        "{\"seq\":4,\"ts\":\"2026-10-01T09:08:07.123Z\",\"event\":\"repair\",\"removed\":1}\n";
    let masked = "{\"seq\":4,\"ts\":\"<ts>\",\"event\":\"repair\",\"removed\":1}\n";
    let before = [goal, nested, earlier, "not json\n"].concat();
    let after = [goal, nested, earlier, appended].concat();
    assert_eq!(
        String::from_utf8(mask_repair_ts(before.as_bytes(), after.as_bytes())).unwrap(),
        [goal, nested, earlier, masked].concat()
    );
    // A torn tail and no bad line: nothing was appended, so the last line is the input's.
    let before = [goal, nested, earlier, "{\"event\":"].concat();
    let after = [goal, nested, earlier].concat();
    assert_eq!(
        String::from_utf8(mask_repair_ts(before.as_bytes(), after.as_bytes())).unwrap(),
        after
    );
    // A last line of another shape is not the command's.
    let odd = "{\"seq\":4,\"ts\":\"2026-10-01T09:08:07.123Z\",\"event\":\"repair\",\"removed\":1,\"x\":1}\n";
    let after = [goal, odd].concat();
    assert_eq!(
        String::from_utf8(mask_repair_ts(goal.as_bytes(), after.as_bytes())).unwrap(),
        after
    );
}

#[test]
fn the_comparator_masks_only_argv0() {
    let exe = "/opt/py/bin/python3";
    let python = [
        "[",
        "  {",
        "    \"kind\": \"launch\",",
        "    \"ref\": \"/opt/py/bin/python3\",",
        "    \"revision\": 2,",
        "    \"argv\": [",
        "      \"/opt/py/bin/python3\",",
        "      \"/repo/puppets.py\",",
        "      \"approve\",",
        "      \"g1\",",
        "      \"--launch\",",
        "      \"/opt/py/bin/python3\",",
        "      \"--expect-revision\",",
        "      \"2\"",
        "    ]",
        "  }",
        "]",
        "",
    ]
    .join("\n");
    let masked = mask_argv0(&python, exe, "/repo/puppets.py", "/bin/hydra-puppets");
    assert_eq!(
        masked.matches("\"/bin/hydra-puppets\"").count(),
        1,
        "{masked}"
    );
    assert_eq!(masked.matches("\"/opt/py/bin/python3\"").count(), 2);
    assert_eq!(masked.matches("puppets.py").count(), 0);
    assert!(
        masked.contains("    \"argv\": [\n      \"/bin/hydra-puppets\",\n      \"approve\",\n")
    );
    // Another script after the same interpreter is not the helper, and is left alone.
    assert_eq!(
        mask_argv0(&python, exe, "/other/puppets.py", "/bin/hydra-puppets"),
        python
    );
}

#[test]
fn the_comparator_masks_a_torn_stamp_only_when_it_is_a_timestamp() {
    assert_eq!(
        mask_torn_stamp("moved the tail into g1.torn-2026-10-01T09:08:07.123Z.txt\n"),
        "moved the tail into g1.torn-<ts>.txt\n"
    );
    // Twenty-four characters that are not a time are a different file name.
    for other in [
        "g1.torn-aaaaaaaaaaaaaaaaaaaaaaaa.txt",
        "g1.torn-2026-10-01T09:08:07.123+.txt",
        "g1.torn-2026-10-01 09:08:07.123Z.txt",
        "g1.torn-2026-10-01T09:08:07,123Z.txt",
        "g1.torn-<UTC timestamp>.txt beside the log",
    ] {
        assert_eq!(mask_torn_stamp(other), other);
    }
    assert_eq!(
        mask_torn_stamp("a.torn-x.torn-2026-10-01T09:08:07.123Z.txt"),
        "a.torn-x.torn-<ts>.txt"
    );
}

#[test]
fn the_comparator_reads_the_class_the_rust_names_first() {
    assert!(names_class(
        "hydra-puppets: KeyError: 'run_id'\n",
        "KeyError"
    ));
    // The class named later in the message is not the one raised.
    assert!(!names_class(
        "hydra-puppets: TypeError: expected a KeyError here\n",
        "KeyError"
    ));
    assert!(!names_class(
        "hydra-puppets: UnicodeDecodeError: x\n",
        "UnicodeError"
    ));
    assert!(!names_class(
        "hydra-puppets: goal log line is not JSON\n",
        "JSON"
    ));
    assert!(!names_class("KeyError: 'run_id'\n", "KeyError"));
    assert_eq!(
        exception_class(
            "Traceback (most recent call last):\n  File \"x\"\njson.decoder.JSONDecodeError: Expecting value: line 1\n"
        ),
        Some("JSONDecodeError".to_string())
    );
    // A message that goes on for more lines does not hide the class, and a line of it
    // that looks like one is not it.
    assert_eq!(
        exception_class(
            "Traceback (most recent call last):\n  File \"x\"\n    raise RuntimeError(m)\nRuntimeError: stderr was:\nhydra: refused\nKeyError: 1\n"
        ),
        Some("RuntimeError".to_string())
    );
}

/// Lists of floats for the sort: random with ties, runs up and down, sorted blocks end to end
/// so the merges gallop, and NaN through all of them. `None` is NaN.
fn sort_cases() -> Vec<Vec<Option<f64>>> {
    let mut seed: u64 = 0x199;
    let mut next = move |below: u64| {
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (seed >> 33) % below
    };
    let sizes = [
        0, 1, 2, 3, 4, 5, 7, 8, 13, 31, 32, 33, 63, 64, 65, 66, 100, 127, 128, 129, 150, 200, 257,
        500, 1000, 2500,
    ];
    let mut cases = Vec::new();
    for size in sizes {
        for mode in 0..6 {
            for nan_one_in in [2, 5, 40, 0] {
                let mut xs: Vec<Option<f64>> = Vec::with_capacity(size);
                let mut block_left = 0;
                let mut value = 0.0;
                for i in 0..size {
                    let x = match mode {
                        0 => next(10) as f64,
                        1 => i as f64,
                        2 => (size - i) as f64 / 3.0,
                        3 => ((size - i) / 4) as f64,
                        // Sorted blocks of uneven length, each starting below the last.
                        4 => {
                            if block_left == 0 {
                                block_left = 1 + next(120);
                                value = next(50) as f64;
                            }
                            block_left -= 1;
                            value += next(3) as f64;
                            value
                        }
                        _ => (next(1000) as f64).sqrt().floor(),
                    };
                    if nan_one_in > 0 && next(nan_one_in) == 0 {
                        xs.push(None);
                    } else {
                        xs.push(Some(x));
                    }
                }
                cases.push(xs);
            }
        }
    }
    cases
}

/// Build ticket 199, item 3: `queue` must leave NaN scores where `sorted` leaves them, which
/// only the same algorithm does. This runs the port and `python3` on the same lists.
#[test]
fn the_sort_matches_python_sorted_on_keys_that_hold_nan() {
    let scratch = Scratch::new("sort");
    let cases = sort_cases();
    let mut input = String::new();
    for xs in &cases {
        let cells: Vec<String> = xs
            .iter()
            .map(|x| x.map_or("NaN".to_string(), |x| format!("{x:?}")))
            .collect();
        input.push_str(&format!("[{}]\n", cells.join(",")));
    }
    let path = scratch.0.join("cases.jsonl");
    std::fs::write(&path, input).unwrap();
    let program = "import json, sys\n\
        for line in open(sys.argv[1]):\n\
        \x20   xs = json.loads(line)\n\
        \x20   print(json.dumps(sorted(range(len(xs)), key=xs.__getitem__)))\n";
    let out = Command::new("python3")
        .args(["-c", program])
        .arg(&path)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), cases.len());
    let mut wrong = Vec::new();
    for (n, (xs, line)) in cases.iter().zip(lines).enumerate() {
        let keys: Vec<f64> = xs.iter().map(|x| x.unwrap_or(f64::NAN)).collect();
        let got =
            hydra_puppets::pysort::sorted(keys.len(), |i, j| Ok::<bool, ()>(keys[i] < keys[j]))
                .unwrap();
        let got = format!("{got:?}");
        if got != line {
            wrong.push(format!("case {n}, {} keys", keys.len()));
        }
    }
    assert!(
        wrong.is_empty(),
        "{} of {} lists sort differently: {wrong:?}",
        wrong.len(),
        cases.len()
    );
}

/// Build ticket 199, item 7: the table `repr` reads is `str.isprintable` as the `python3` on
/// this machine has it.
#[test]
fn the_printable_table_is_python_isprintable() {
    let program = "ranges, start = [], None\n\
        for c in range(0x110000):\n\
        \x20   hidden = not chr(c).isprintable()\n\
        \x20   if hidden and start is None:\n\
        \x20       start = c\n\
        \x20   if not hidden and start is not None:\n\
        \x20       ranges.append((start, c - 1))\n\
        \x20       start = None\n\
        ranges.append((start, 0x10FFFF))\n\
        for low, high in ranges:\n\
        \x20   print(low, high)\n";
    let out = Command::new("python3")
        .args(["-c", program])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let python: Vec<(u32, u32)> = String::from_utf8(out.stdout)
        .unwrap()
        .lines()
        .map(|line| {
            let (low, high) = line.split_once(' ').unwrap();
            (low.parse().unwrap(), high.parse().unwrap())
        })
        .collect();
    assert_eq!(python, hydra_puppets::printable::NOT_PRINTABLE.to_vec());
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

/// Mask the `ts` of each line a write command appended. `after` must begin with `before`,
/// byte for byte, or nothing is masked: a writer only appends. A line is masked only when
/// it starts `{"seq":N,"ts":"<stamp>","event":"` with a stamp of `utc_now()`'s shape, so a
/// `ts` the input held, or one nested further into an event, stays as it is.
fn mask_appended_ts(before: &[u8], after: &[u8]) -> Vec<u8> {
    let Some(appended) = after.strip_prefix(before) else {
        return after.to_vec();
    };
    let mut out = before.to_vec();
    for line in appended.split_inclusive(|b| *b == b'\n') {
        match appended_line(line) {
            Some((head, tail)) => {
                out.extend_from_slice(head);
                out.extend_from_slice(b"<ts>");
                out.extend_from_slice(tail);
            }
            None => out.extend_from_slice(line),
        }
    }
    out
}

/// Split an appended line around its stamp, when it has the shape `_locked_update` writes.
fn appended_line(line: &[u8]) -> Option<(&[u8], &[u8])> {
    let text = std::str::from_utf8(line).ok()?;
    let (seq, rest) = text.strip_prefix("{\"seq\":")?.split_once(",\"ts\":\"")?;
    let stamp = rest.get(..24)?;
    let tail = rest.get(24..)?;
    let digits = !seq.is_empty() && seq.bytes().all(|b| b.is_ascii_digit());
    if !digits
        || !is_utc_stamp(stamp)
        || !tail.starts_with("\",\"event\":\"")
        || !tail.ends_with("}\n")
    {
        return None;
    }
    let at = text.len() - rest.len();
    Some((&line[..at], &line[at + 24..]))
}

/// Whether `text` has the shape `str(uuid.uuid4())` prints: lower-case hex, version 4, and
/// the variant of RFC 4122, whose character is `8`, `9`, `a` or `b`.
fn is_goal_id(text: &str) -> bool {
    text.len() == 36
        && text.bytes().enumerate().all(|(i, b)| match i {
            8 | 13 | 18 | 23 => b == b'-',
            14 => b == b'4',
            19 => matches!(b, b'8' | b'9' | b'a' | b'b'),
            _ => b.is_ascii_digit() || (b'a'..=b'f').contains(&b),
        })
}

/// The goal id `new` printed, when it printed one and nothing else and the log it names is
/// in `dir`. A writer that prints one id and creates the log under another gets `None`.
fn new_goal_id<'a>(stdout: &'a str, dir: &Path) -> Option<&'a str> {
    let id = stdout.strip_suffix('\n').filter(|id| is_goal_id(id))?;
    dir.join("puppets")
        .join(format!("{id}.jsonl"))
        .is_file()
        .then_some(id)
}

fn new_stdout_names_log(stdout: &str, dir: &Path) -> bool {
    new_goal_id(stdout, dir).is_some()
}

/// `new` prints the goal id it generated and nothing else: mask that one line, when it
/// names the log `new` created in `dir`.
fn mask_new_stdout(stdout: &str, dir: &Path) -> String {
    match new_goal_id(stdout, dir) {
        Some(_) => "<goal>\n".to_string(),
        None => stdout.to_string(),
    }
}

/// The goal logs a write command left in `dir`, each with the stamps of its appended lines
/// masked. `seed` is the home both copies were made from. Under `new` the log named by
/// the goal id on stdout is listed as `<goal>.jsonl`, and no other log is renamed.
fn written(dir: &Path, seed: &Path, new_goal: Option<&str>) -> Vec<(String, Vec<u8>)> {
    let mut files: Vec<(String, Vec<u8>)> = std::fs::read_dir(dir.join("puppets"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.is_file())
        .map(|p| {
            let name = p.file_name().unwrap().to_string_lossy().into_owned();
            let before = std::fs::read(seed.join("puppets").join(&name)).unwrap_or_default();
            let bytes = mask_appended_ts(&before, &std::fs::read(&p).unwrap());
            let name = match name.strip_suffix(".jsonl") {
                Some(stem) if new_goal == Some(stem) => "<goal>.jsonl".to_string(),
                _ => name,
            };
            (name, bytes)
        })
        .collect();
    files.sort();
    files
}

/// One write command on one starting home.
struct WriteCase {
    label: String,
    /// `puppets/g1.jsonl`, or no log at all.
    log: Option<Vec<u8>>,
    vendors: Option<&'static str>,
    /// Files under `puppets/agents/`, which `assign` ranks from.
    sheets: Vec<(&'static str, String)>,
    args: Vec<String>,
    cwd: PathBuf,
}

impl WriteCase {
    fn new(label: &str, log: Option<&[u8]>, args: &[&str], cwd: &Path) -> WriteCase {
        WriteCase {
            label: label.to_string(),
            log: log.map(<[u8]>::to_vec),
            vendors: None,
            sheets: Vec::new(),
            args: args.iter().map(|a| a.to_string()).collect(),
            cwd: cwd.to_path_buf(),
        }
    }
}

/// Run one case under both implementations, each on its own copy of the seed home. Returns
/// the exception class when the Python raised, and what differed.
fn compare_write(case: &WriteCase, root: &Path, n: usize) -> (Option<String>, Vec<String>) {
    let base = root.join(format!("w{n:03}"));
    let (seed, py_dir, rs_dir) = (base.join("seed"), base.join("py"), base.join("rs"));
    std::fs::create_dir_all(seed.join("puppets")).unwrap();
    if let Some(log) = &case.log {
        std::fs::write(seed.join("puppets/g1.jsonl"), log).unwrap();
    }
    if let Some(vendors) = case.vendors {
        std::fs::write(seed.join("vendors.json"), vendors).unwrap();
    }
    if !case.sheets.is_empty() {
        std::fs::create_dir_all(seed.join("puppets/agents")).unwrap();
    }
    for (name, body) in &case.sheets {
        std::fs::write(seed.join("puppets/agents").join(name), body).unwrap();
    }
    copy_dir(&seed, &py_dir);
    copy_dir(&seed, &rs_dir);
    let script = script();
    let args: Vec<&str> = case.args.iter().map(String::as_str).collect();
    let mut py = run_in(
        &["python3", script.to_str().unwrap()],
        &args,
        &py_dir,
        Some(&case.cwd),
    );
    let mut rs = run_in(&[RUST], &args, &rs_dir, Some(&case.cwd));
    let mut problems = Vec::new();
    // The id each printed is tied to the log it created before either is masked.
    let (mut py_goal, mut rs_goal) = (None, None);
    if args[0] == "new" {
        for (who, outcome, dir, goal) in [
            ("python", &mut py, &py_dir, &mut py_goal),
            ("rust", &mut rs, &rs_dir, &mut rs_goal),
        ] {
            *goal = new_goal_id(&outcome.stdout, dir).map(str::to_string);
            if outcome.code == 0 && goal.is_none() {
                problems.push(format!(
                    "{who} exited 0 and its stdout names no log it created: {:?}",
                    outcome.stdout
                ));
            }
            outcome.stdout = mask_new_stdout(&outcome.stdout, dir);
        }
    }
    let (py_files, rs_files) = (
        written(&py_dir, &seed, py_goal.as_deref()),
        written(&rs_dir, &seed, rs_goal.as_deref()),
    );
    if py_files != rs_files {
        let show = |files: &[(String, Vec<u8>)]| {
            files
                .iter()
                .map(|(name, bytes)| format!("{name}:\n{}", String::from_utf8_lossy(bytes)))
                .collect::<Vec<_>>()
                .join("\n")
        };
        problems.push(format!(
            "the goal logs differ\npython left\n{}\nrust left\n{}",
            show(&py_files),
            show(&rs_files)
        ));
    }
    let (class, problem) = judge(&py, &rs);
    problems.extend(problem);
    (class, problems)
}

/// The starting homes and the commands run on each.
fn write_cases(shared: &Path) -> Vec<WriteCase> {
    const GOAL: &str = r#"{"seq":1,"ts":"2026-09-30T12:00:00.000Z","event":"goal","text":"test","repo":"/repo","base_sha":"abc"}"#;
    let line = |s: &str| format!("{s}\n");
    let task = |id: &str, class: &str, grade: &str| {
        line(&format!(
            r#"{{"event":"task","id":"{id}","text":"do {id}","class":"{class}","grade":"{grade}","done":null,"deps":[],"paths":[]}}"#
        ))
    };
    let approval = |kind: &str, rest: &str, revision: u32| {
        line(&format!(
            r#"{{"event":"approval","kind":"{kind}",{rest}"revision":{revision},"yes":true}}"#
        ))
    };
    let plan = |name: &str| shared.join(name).to_str().unwrap().to_string();

    // Five tasks, revision 5: a default for each of t1, t2 and t5, an `agi` grade, and a
    // class the default table does not hold.
    let tasks = [
        line(GOAL),
        task("t1", "docs", "low"),
        task("t2", "rust_launch_path", "high"),
        task("t3", "research", "agi"),
        task("t4", "weird", "low"),
        task("t5", "security_review", "medium"),
    ]
    .concat();
    let assigned = [
        tasks.clone(),
        line(r#"{"event":"assignment","task":"t1","provider":"codex","policy":"standard","source":"human"}"#),
    ]
    .concat();
    let running = [
        assigned.clone(),
        approval("table", r#""ref":null,"#, 6),
        approval("launch", r#""ref":"t1","#, 6),
        line(r#"{"event":"launch_intent","task":"t1","attempt":1,"provider":"codex"}"#),
        line(r#"{"event":"run","task":"t1","attempt":1,"run_id":"r1"}"#),
    ]
    .concat();
    let ended = [
        running.clone(),
        line(r#"{"event":"run_end","task":"t1","run_id":"r1","outcome":"completed","total_tokens":7}"#),
    ]
    .concat();
    let diff_approved = [
        ended.clone(),
        approval("diff", r#""ref":"t1","run_id":"r1","#, 6),
    ]
    .concat();
    let dangling = [
        assigned.clone(),
        line(r#"{"event":"launch_intent","task":"t1","attempt":1,"provider":"codex"}"#),
        line(r#"{"event":"launch_intent","task":"t2","attempt":2,"provider":"claude"}"#),
    ]
    .concat();
    let degraded = [
        tasks.clone(),
        line(r#"{"event":"approval","kind":"degraded","missing":"codex","substitute":"claude","revision":5,"yes":true}"#),
    ]
    .concat();
    let proposals = [
        line(GOAL),
        line(r#"{"event":"plan_proposed","provider":"codex","model":null,"run_id":"p1"}"#),
        line(r#"{"event":"plan_proposed","provider":"claude","model":"opus","run_id":"p2"}"#),
        line(&format!(
            r#"{{"event":"plan_ready","provider":"codex","run_id":"p1","path":"{}"}}"#,
            plan("good.md")
        )),
        line(&format!(
            r#"{{"event":"plan_ready","provider":"claude","run_id":"p2","path":"{}"}}"#,
            plan("crlf.md")
        )),
    ]
    .concat();
    let no_substitute = [
        proposals.clone(),
        line(r#"{"event":"approval","kind":"degraded","missing":"codex","substitute":"","revision":0,"yes":true}"#),
    ]
    .concat();
    let chosen = |path_json: &str| {
        [
            line(GOAL),
            line(&format!(
                r#"{{"event":"panel_verdict","judge_run_ids":[],"chosen":"codex","path":{path_json}}}"#
            )),
        ]
        .concat()
    };
    let chosen_file = |name: &str| chosen(&format!("\"{}\"", plan(name)));
    let plan_approved = |log: String| [log, approval("plan", r#""ref":null,"#, 0)].concat();
    // What `assign` ranks from: a verdict on r1, which both sonnet sheets hold.
    let ranked = [
        line(GOAL),
        task("t1", "docs", "low"),
        line(r#"{"event":"assignment","task":"t1","provider":"claude","model":"sonnet","policy":"standard","source":"default"}"#),
        line(r#"{"event":"run","task":"t1","attempt":1,"run_id":"r1"}"#),
        line(r#"{"event":"run_end","task":"t1","run_id":"r1","outcome":"completed"}"#),
        line(r#"{"event":"human_verdict","task":"t1","run_id":"r1","result":"applied","note":""}"#),
        task("t6", "docs", "low"),
        task("t7", "docs", "high"),
        task("t8", "probe", "low"),
    ]
    .concat();
    let sheet = |provider: &str, model: &str, available: bool, runs: &str, p50: &str| {
        format!(
            r#"{{"provider":"{provider}","model":{model},"available":{available},"policies":["judge","standard"],"runs":{runs},"cost":{{"p50":{p50},"n":1}}}}"#
        )
    };
    let r1 = r#"[{"run_id":"r1","task_id":"r1","parent_run_id":null}]"#;
    let sheets = vec![
        (
            "claude--sonnet.json",
            sheet("claude", "\"sonnet\"", true, r1, "0.02"),
        ),
        (
            "claude--Sonnet%205.json",
            sheet("claude", "\"Sonnet 5\"", true, r1, "0.01"),
        ),
        (
            "cursor-agent--composer-2.5.json",
            sheet("cursor-agent", "\"composer-2.5\"", true, "[]", "null"),
        ),
        (
            "codex--~null.json",
            sheet("codex", "null", false, r1, "0.001"),
        ),
    ];
    let untaught = vec![(
        "claude--sonnet.json",
        sheet("claude", "\"sonnet\"", true, "[]", "0.02"),
    )];
    let codex_down = r#"[{"provider":"codex","model":null,"available":false}]"#;

    let mut cases = Vec::new();

    // Build ticket 207: `--expect-revision` and `attempt` are `int()` of the argument. The
    // log is at revision 5, so the fifth row records the approval under both.
    let wide = "99999999999999999999999999999999999999999999999999";
    let too_long = "9".repeat(4301);
    for value in [
        "\u{1c}7",
        "٧",
        " +٧_２\u{a0}",
        wide,
        "\u{3000}٥\u{2028}",
        "0_5",
        "-0",
        "5_",
        "²",
        &too_long,
    ] {
        let label: String = value.chars().take(60).collect();
        cases.push(WriteCase::new(
            &format!("Python int revision {label:?}"),
            Some(tasks.as_bytes()),
            &["approve", "g1", "--plan", "--expect-revision", value],
            shared,
        ));
    }
    // Build ticket 207: argparse takes a prefix of an option that only one option starts
    // with, and refuses one that two do.
    let abbreviated: [&[&str]; 9] = [
        &["approve", "g1", "--pl"],
        &["approve", "g1", "--t", "--expect-rev", "5"],
        &["approve", "g1", "--expect", "1"],
        &["approve", "g1", "--expect-r=5", "--plan"],
        &["approve", "g1", "--d", "t1"],
        &["approve", "g1", "--di", "t1"],
        &[
            "set",
            "g1",
            "t1",
            "--pr",
            "codex",
            "--po=standard",
            "--m",
            "sol",
        ],
        &["set", "g1", "t1", "--p", "codex"],
        &["verdict", "g1", "t1", "fixed", "--no", "ok"],
    ];
    for args in abbreviated {
        cases.push(WriteCase::new(
            &format!("abbreviation {args:?}"),
            Some(tasks.as_bytes()),
            args,
            shared,
        ));
    }

    // Every writer that names a goal, on the logs a writer must refuse and on no log at all.
    let damaged = [line(GOAL), line("not json"), task("t1", "docs", "low")].concat();
    let unterminated = [tasks.clone(), r#"{"event":"task","id":"t9""#.to_string()].concat();
    let bad_tail = [tasks.clone().into_bytes(), b"{\"event\":\xff\xfe".to_vec()].concat();
    let bad_line = [
        line(GOAL).into_bytes(),
        b"{\"event\":\"note\",\"text\":\"\xff\"}\n".to_vec(),
        task("t1", "docs", "low").into_bytes(),
    ]
    .concat();
    let hostile: [(&str, Option<&[u8]>); 5] = [
        ("a damaged log", Some(damaged.as_bytes())),
        ("an unterminated log", Some(unterminated.as_bytes())),
        ("an invalid UTF-8 tail", Some(&bad_tail)),
        ("invalid UTF-8 inside a terminated line", Some(&bad_line)),
        ("no log", None),
    ];
    let writers: [&[&str]; 9] = [
        &[
            "set",
            "g1",
            "t1",
            "--provider",
            "codex",
            "--policy",
            "standard",
        ],
        &["approve", "g1", "--table"],
        &["approve", "g1", "--plan", "--expect-revision", "9"],
        &["verdict", "g1", "t1", "rejected"],
        &["choose", "g1", "codex"],
        &["choose", "g1", "--judge"],
        &["tasks", "g1"],
        &["assign", "g1"],
        &["reconcile", "g1", "t1", "1", "--abandon"],
    ];
    for (name, log) in hostile {
        for args in writers {
            cases.push(WriteCase::new(
                &format!("{name}: {}", args.join(" ")),
                log,
                args,
                shared,
            ));
        }
    }

    let mut each = |name: &str, log: &str, commands: &[&[&str]]| {
        for args in commands {
            cases.push(WriteCase::new(
                &format!("{name}: {}", args.join(" ")),
                Some(log.as_bytes()),
                args,
                shared,
            ));
        }
    };
    each(
        "five tasks",
        &tasks,
        &[
            &[
                "set",
                "g1",
                "t1",
                "--provider",
                "codex",
                "--policy",
                "standard",
            ],
            &[
                "set",
                "g1",
                "t1",
                "--provider",
                "claude",
                "--model",
                "opus",
                "--policy",
                "judge",
            ],
            &[
                "set",
                "g1",
                "t1",
                "--provider",
                "claude",
                "--model",
                "",
                "--policy",
                "judge",
            ],
            &[
                "set",
                "g1",
                "nobody",
                "--provider",
                "ñandú ☕",
                "--policy",
                "standard",
            ],
            &["approve", "g1"],
            &["approve", "g1", "--plan"],
            &["approve", "g1", "--table"],
            &["approve", "g1", "--launch", "t1"],
            &["approve", "g1", "--launch", "nobody"],
            &["approve", "g1", "--launch", ""],
            &["approve", "g1", "--diff", "t1"],
            &["approve", "g1", "--diff", "nobody"],
            &["approve", "g1", "--plan", "--expect-revision", "5"],
            &["approve", "g1", "--plan", "--expect-revision", "4"],
            &["approve", "g1", "--table", "--expect-revision", "-1"],
            &["approve", "g1", "--table", "--expect-run", "r1"],
            &["approve", "g1", "--table", "--expect-plan", "/p"],
            &["approve", "g1", "--plan", "--expect-plan", "/p"],
            &["approve", "g1", "--diff", "nobody", "--expect-run", "r1"],
            &["approve", "g1", "--degraded", "codex=claude"],
            &["approve", "g1", "--degraded", "codex=claude:opus"],
            &["approve", "g1", "--degraded", " codex = claude : opus "],
            &["approve", "g1", "--degraded", "codex=claude:"],
            &[
                "approve",
                "g1",
                "--degraded",
                "codex=<substitute-provider[:model]>",
            ],
            &["approve", "g1", "--degraded", "codex"],
            &["approve", "g1", "--degraded", "=claude"],
            &["approve", "g1", "--degraded", "codex="],
            &["approve", "g1", "--degraded", "codex=claude", "--plan"],
            &[
                "approve",
                "g1",
                "--degraded",
                "codex=claude",
                "--expect-revision",
                "3",
            ],
            &["verdict", "g1", "t1", "applied"],
            &["verdict", "g1", "nobody", "rejected"],
            &["choose", "g1"],
            &["tasks", "g1"],
            &["assign", "g1"],
            &["reconcile", "g1", "t1", "1", "--abandon"],
        ],
    );
    each(
        "a degraded approval",
        &degraded,
        &[
            &["approve", "g1", "--degraded", "codex=agy"],
            &["approve", "g1", "--degraded", "claude=agy"],
        ],
    );
    each(
        "a running task",
        &running,
        &[
            &["approve", "g1", "--diff", "t1"],
            &["approve", "g1", "--diff", "t1", "--expect-run", "r1"],
            &["approve", "g1", "--diff", "t1", "--expect-run", "r2"],
            &[
                "approve",
                "g1",
                "--diff",
                "t1",
                "--expect-run",
                "r1",
                "--expect-revision",
                "6",
            ],
            &["approve", "g1", "--table"],
            &["approve", "g1", "--launch", "t1"],
            &["verdict", "g1", "t1", "rejected"],
            &["assign", "g1"],
        ],
    );
    each(
        "an ended run",
        &ended,
        &[
            &["verdict", "g1", "t1", "applied"],
            &["verdict", "g1", "t1", "fixed"],
            &["verdict", "g1", "t1", "rejected", "--note", "no good"],
            &["verdict", "g1", "t2", "rejected"],
        ],
    );
    each(
        "an approved diff",
        &diff_approved,
        &[
            &["verdict", "g1", "t1", "applied"],
            &["verdict", "g1", "t1", "fixed", "--note", "café ☕ 😀"],
            &["verdict", "g1", "t1", "rejected"],
            &["approve", "g1", "--diff", "t1"],
            &["approve", "g1", "--diff", "t1", "--expect-run", "r1"],
        ],
    );
    each(
        "two dangling intents",
        &dangling,
        &[
            &["reconcile", "g1", "t1", "1", "--run-id", "r9"],
            &["reconcile", "g1", "t1", "1", "--abandon"],
            &["reconcile", "g1", "t1", "1", "--run-id", ""],
            &["reconcile", "g1", "t1", "2", "--abandon"],
            &["reconcile", "g1", "t2", "2", "--run-id", "r9"],
            &["reconcile", "g1", "nobody", "1", "--abandon"],
            &["reconcile", "g1", "t1", " ١\u{a0}", "--ab"],
            &["reconcile", "g1", "t1", "\u{1c}1", "--abandon"],
            &["reconcile", "g1", "t1", wide, "--run", "r9"],
            &["reconcile", "g1", "t1", "1", "--r", "r9"],
        ],
    );
    let good = plan("good.md");
    each(
        "a goal",
        &line(GOAL),
        &[
            &["choose", "g1", "codex"],
            &["choose", "g1", &good],
            &["choose", "g1", "good.md"],
            &["choose", "g1", "./sub/../a.b.md"],
            &["choose", "g1", ".hidden"],
            &["choose", "g1", "sub"],
            &["choose", "g1", "sub/trailing."],
            &["choose", "g1", "absent.md"],
            &["choose", "g1", "--judge"],
            &["choose", "g1", "codex", "--judge"],
            &["tasks", "g1"],
            &["assign", "g1"],
        ],
    );
    each(
        "a chosen plan, not approved",
        &chosen_file("good.md"),
        &[
            &["tasks", "g1"],
            &["approve", "g1", "--plan", "--expect-plan", &good],
            &["approve", "g1", "--plan", "--expect-plan", "/elsewhere.md"],
        ],
    );
    for name in [
        "good.md",
        "crlf.md",
        "bare-fence.md",
        "two-fences.md",
        "missing-key.md",
        "no-id.md",
        "bad-class.md",
        "bad-grade.md",
        "list-class.md",
        "no-fence.md",
        "bad-json.md",
        "number-task.md",
        "string-task.md",
        "list-task.md",
        "empty.md",
        "absent.md",
    ] {
        each(
            &format!("an approved plan {name}"),
            &plan_approved(chosen_file(name)),
            &[&["tasks", "g1"]],
        );
    }
    each(
        "an approved plan whose path is a number",
        &plan_approved(chosen("5")),
        &[&["tasks", "g1"]],
    );
    each(
        "an approved plan with no path",
        &plan_approved(chosen("null")),
        &[&["tasks", "g1"]],
    );

    let mut with = |label: &str,
                    log: &str,
                    vendors: Option<&'static str>,
                    sheets: &[(&'static str, String)],
                    args: &[&str]| {
        let mut case = WriteCase::new(label, Some(log.as_bytes()), args, shared);
        case.vendors = vendors;
        case.sheets = sheets.to_vec();
        cases.push(case);
    };
    with(
        "two proposals, codex down: choose --judge",
        &proposals,
        Some(codex_down),
        &[],
        &["choose", "g1", "--judge"],
    );
    with(
        "two proposals, codex down, a degraded approval with no substitute: choose --judge",
        &no_substitute,
        Some(codex_down),
        &[],
        &["choose", "g1", "--judge"],
    );
    with(
        "a ranking: assign",
        &ranked,
        None,
        &sheets,
        &["assign", "g1"],
    );
    with(
        "sheets with no verdict behind them: assign",
        &ranked,
        None,
        &untaught,
        &["assign", "g1"],
    );
    with(
        "a grade the ranking does not know: assign",
        &[line(GOAL), task("t1", "docs", "weird")].concat(),
        None,
        &sheets,
        &["assign", "g1"],
    );
    with(
        "a sheet that is not JSON: assign",
        &tasks,
        None,
        &[("broken.json", "{".to_string())],
        &["assign", "g1"],
    );
    with(
        "a sheet that is a list: assign",
        &tasks,
        None,
        &[("list.json", "[1]".to_string())],
        &["assign", "g1"],
    );
    with(
        "an agents directory with no sheet: assign",
        &tasks,
        None,
        &[("notes.txt", "x".to_string())],
        &["assign", "g1"],
    );

    // `new` reads the repo and the commit from its working directory.
    for (label, cwd, text) in [
        ("new outside a repo", shared.to_path_buf(), "a goal"),
        ("new inside this repo", repo(), "a goal"),
        (
            "new with text that is not ASCII",
            shared.to_path_buf(),
            "café ☕ 😀",
        ),
        ("new with empty text", shared.to_path_buf(), ""),
    ] {
        cases.push(WriteCase::new(label, None, &["new", text], &cwd));
    }
    cases
}

/// The plan files the `tasks` and `choose` cases name, in one directory both read.
fn write_plans(shared: &Path) {
    let fenced = |body: &str| format!("# Plan\n\nSome prose.\n\n```json\n{body}\n```\n\nMore.\n");
    let good = r#"[
  {"id": "a", "text": "café ☕ 😀", "class": "docs", "grade": "low"},
  {"id": "b", "text": "two", "class": "rust_launch_path", "grade": "agi",
   "done": "cargo test", "deps": ["a"], "paths": ["src/x.rs", "src/y.rs"], "extra": 1},
  {"id": 3, "text": ["not", "text"], "class": "probe", "grade": "high",
   "done": {"k": [1.5, 1e16, NaN, -0.0]}, "deps": "a", "paths": null}
]"#;
    let files: [(&str, String); 15] = [
        ("good.md", fenced(good)),
        ("crlf.md", fenced(good).replace('\n', "\r\n")),
        (
            "bare-fence.md",
            "```\n[{\"id\":\"a\",\"text\":\"t\",\"class\":\"docs\",\"grade\":\"low\"}]```".into(),
        ),
        (
            "two-fences.md",
            format!(
                "```rust\nlet x = [1];\n```\n{}{}",
                fenced(r#"[{"id":"first","text":"t","class":"docs","grade":"low"}]"#),
                fenced(r#"[{"id":"second","text":"t","class":"docs","grade":"low"}]"#)
            ),
        ),
        (
            "missing-key.md",
            fenced(r#"[{"id":"a","text":"t","class":"docs","grade":"low"},{"id":"b","text":"t"}]"#),
        ),
        ("no-id.md", fenced(r#"[{"text":"t","class":"docs"}]"#)),
        (
            "bad-class.md",
            fenced(r#"[{"id":"a","text":"t","class":"poetry","grade":"low"}]"#),
        ),
        (
            "bad-grade.md",
            fenced(r#"[{"id":"a","text":"t","class":"docs","grade":1}]"#),
        ),
        (
            "list-class.md",
            fenced(r#"[{"id":"a","text":"t","class":["docs"],"grade":"low"}]"#),
        ),
        (
            "no-fence.md",
            "No tasks here, [1, 2] is not fenced.\n".into(),
        ),
        ("bad-json.md", fenced("[{\"id\": }]")),
        ("number-task.md", fenced("[1]")),
        ("string-task.md", fenced(r#"["id text class grade"]"#)),
        ("list-task.md", fenced(r#"[["id", "text"]]"#)),
        ("empty.md", fenced("[]")),
    ];
    for (name, body) in files {
        std::fs::write(shared.join(name), body).unwrap();
    }
    std::fs::create_dir_all(shared.join("sub")).unwrap();
    for name in ["a.b.md", ".hidden", "sub/trailing."] {
        std::fs::write(shared.join(name), "x").unwrap();
    }
}

#[test]
fn the_write_commands_match_puppets_py_and_leave_the_same_goal_log() {
    let scratch = Scratch::new("write");
    let shared = scratch.0.join("shared");
    std::fs::create_dir_all(&shared).unwrap();
    write_plans(&shared);
    let cases = write_cases(&shared);
    let mut crashes = Vec::new();
    let mut mismatches = Vec::new();
    for (n, case) in cases.iter().enumerate() {
        let (class, problems) = compare_write(case, &scratch.0, n);
        crashes.extend(class.map(|class| format!("{}: {class}", case.label)));
        if !problems.is_empty() {
            mismatches.push(format!("{}\n{}", case.label, problems.join("\n")));
        }
    }
    eprintln!("compared {} write command runs:", cases.len());
    for case in &cases {
        eprintln!("  {}", case.label);
    }
    eprintln!(
        "python raised, stderr compared by exception class ({}):",
        crashes.len()
    );
    for crash in &crashes {
        eprintln!("  {crash}");
    }
    assert!(
        mismatches.is_empty(),
        "{} of {} differ:\n\n{}",
        mismatches.len(),
        cases.len(),
        mismatches.join("\n\n")
    );
}

#[test]
fn the_comparator_masks_only_the_stamps_of_appended_lines() {
    let before = "{\"seq\":1,\"ts\":\"2026-10-01T09:08:07.123Z\",\"event\":\"goal\"}\n";
    let appended =
        "{\"seq\":2,\"ts\":\"2026-10-01T09:08:07.456Z\",\"event\":\"task\",\"id\":\"t\"}\n";
    let masked = "{\"seq\":2,\"ts\":\"<ts>\",\"event\":\"task\",\"id\":\"t\"}\n";
    let mask = |before: &str, after: &str| {
        String::from_utf8(mask_appended_ts(before.as_bytes(), after.as_bytes())).unwrap()
    };
    // The appended line loses its stamp and the line the input held keeps its own.
    assert_eq!(
        mask(before, &[before, appended].concat()),
        [before, masked].concat()
    );
    assert_eq!(
        mask(before, &[before, appended, appended].concat()),
        [before, masked, masked].concat()
    );
    // A log that does not begin with the input was rewritten, not appended to: left alone.
    let rewritten = [appended, appended].concat();
    assert_eq!(mask(before, &rewritten), rewritten);
    // A `ts` that is not a time, a `ts` further into the event, and a line with no newline.
    for odd in [
        "{\"seq\":2,\"ts\":\"yesterday\",\"event\":\"task\"}\n".to_string(),
        "{\"seq\":2,\"event\":\"task\",\"ts\":\"2026-10-01T09:08:07.456Z\"}\n".to_string(),
        "{\"seq\":2,\"ts\":\"2026-10-01T09:08:07.456Z\",\"other\":1}\n".to_string(),
        "{\"seq\":x,\"ts\":\"2026-10-01T09:08:07.456Z\",\"event\":\"task\"}\n".to_string(),
        appended.trim_end().to_string(),
    ] {
        let after = [before, odd.as_str()].concat();
        assert_eq!(mask(before, &after), after);
    }
}

#[test]
fn the_comparator_masks_a_goal_id_only_when_it_is_a_uuid4() {
    let scratch = Scratch::new("mask-goal-id");
    let dir = &scratch.0;
    std::fs::create_dir_all(dir.join("puppets")).unwrap();
    let id = "0b9f2c3e-6a1d-4c7b-9e2f-5d8a7b6c4e31";
    let seven = "0b9f2c3e-6a1d-7c7b-9e2f-5d8a7b6c4e31";
    for name in [id, seven, "g1"] {
        std::fs::write(dir.join("puppets").join(format!("{name}.jsonl")), b"{}\n").unwrap();
    }
    assert!(is_goal_id(id));
    assert_eq!(mask_new_stdout(&format!("{id}\n"), dir), "<goal>\n");
    for odd in [
        "g1\n".to_string(),
        id.to_string(),
        format!("{id}\n{id}\n"),
        format!("goal {id}\n"),
        format!("{}\n", id.to_uppercase()),
        // Version 7, which `uuid.uuid4()` never prints.
        format!("{seven}\n"),
    ] {
        assert_eq!(mask_new_stdout(&odd, dir), odd);
    }
}

#[test]
fn stdout_surrogate_matches_python_locales() {
    let scratch = Scratch::new("stdout-surrogate");
    let home = scratch.0.join("puppets");
    std::fs::create_dir(&home).unwrap();
    std::fs::write(
        home.join("g1.jsonl"),
        b"{\"event\":\"goal\",\"text\":\"\\udc80\",\"repo\":\"/r\",\"base_sha\":\"a\"}\n",
    )
    .unwrap();
    for (locale, utf8, encoding) in [
        ("C", None, None),
        ("en_US.UTF-8", None, None),
        ("C", Some("0"), None),
        ("en_US.UTF-8", Some("1"), None),
        ("C", None, Some("utf-8")),
        ("C", None, Some("utf-8:strict")),
        ("en_US.UTF-8", None, Some(":surrogateescape")),
        ("C", None, Some("ascii:backslashreplace")),
        ("en_US.UTF-8", None, Some("latin-1:replace")),
    ] {
        let invoke = |python: bool| {
            let mut cmd = Command::new(if python { "python3" } else { RUST });
            if python {
                cmd.arg(script());
            }
            cmd.args(["board", "g1"])
                .env("HYDRA_PUPPETS_HOME", &home)
                .env("HYDRA_GRADER_HOME", scratch.0.join("grader"))
                .env("LC_ALL", locale)
                .env_remove("PYTHONUTF8")
                .env_remove("PYTHONIOENCODING")
                .stdin(std::process::Stdio::null());
            if let Some(value) = utf8 {
                cmd.env("PYTHONUTF8", value);
            }
            if let Some(value) = encoding {
                cmd.env("PYTHONIOENCODING", value);
            }
            cmd.output().unwrap()
        };
        let python = invoke(true);
        let rust = invoke(false);
        assert_eq!(rust.stdout, python.stdout, "{locale}");
        assert_eq!(rust.status.code(), python.status.code(), "{locale}");
    }
}

#[test]
fn repair_temporary_symlink_matches_python() {
    let scratch = Scratch::new("repair-symlink");
    let raw = b"{\"event\":\"goal\",\"text\":\"test\",\"repo\":\"/r\",\"base_sha\":\"a\"}\ntorn";
    let mut outcomes = Vec::new();
    for python in [true, false] {
        let dir = scratch.0.join(if python { "python" } else { "rust" });
        let home = dir.join("puppets");
        std::fs::create_dir_all(&home).unwrap();
        let path = home.join("g1.jsonl");
        std::fs::write(&path, raw).unwrap();
        let tmp = home.join("g1.jsonl.repair-tmp");
        std::os::unix::fs::symlink(&path, &tmp).unwrap();
        let result = if python {
            run(
                &["python3", script().to_str().unwrap()],
                &["repair", "g1", "--yes"],
                &dir,
            )
        } else {
            run(&[RUST], &["repair", "g1", "--yes"], &dir)
        };
        assert_eq!(result.code, 1);
        assert_eq!(std::fs::read(&path).unwrap(), raw);
        assert!(tmp.symlink_metadata().unwrap().file_type().is_symlink());
        assert_eq!(std::fs::read_dir(&home).unwrap().count(), 2);
        outcomes.push(result);
    }
    assert_eq!(outcomes[0], outcomes[1]);
}

// The commands that launch: `plan`, `plans`, `launch`, `wait`, `review` and `choose --judge`.
//
// Neither implementation starts a provider here. `HYDRA_BIN` names a stub, and stubs named
// `hunk` and `herdr` come first on `PATH`. Each stub call writes its argv, its environment,
// its working directory and, for `hunk session`, its stdin under `stub/calls/`. Both
// implementations run from an environment built from nothing, so what a stub records is
// what the command handed it.

/// One script under three names. The name and the first argument pick what it does. A
/// `hydra run` is numbered per case, so the run ids are the same under both
/// implementations, and `stub/answers/<n>` is what `hydra show` prints for the nth run.
const STUB: &str = r#"#!/bin/bash
state="$STUB_STATE"
prog=$(basename "$0")
case "$prog" in hydra*) prog=hydra ;; esac
key="$prog-${1:-none}"
mkdir -p "$state/calls" "$state/show"
n=$(( $(cat "$state/count-$key" 2>/dev/null || echo 0) + 1 ))
echo "$n" > "$state/count-$key"
call="$state/calls/$key-$n"
printf '%s\0' "$@" > "$call.argv"
env | sort > "$call.env"
pwd > "$call.cwd"
case "$key" in
  hydra-run)
    if [ -f "$state/run-stderr" ]; then
      cat "$state/run-stderr" >&2
      exit 3
    fi
    id="run-$n"
    mkdir -p "$HYDRA_ROOT/runs"
    printf '%s\n' \
      '{"hydra":{"type":"run_start"},"run_id":"'"$id"'","seq":0}' \
      '{"hydra":{"type":"run_end"},"run_id":"'"$id"'","seq":1}' \
      > "$HYDRA_ROOT/runs/$id.jsonl"
    if [ -f "$state/answers/$n" ]; then cp "$state/answers/$n" "$state/show/$id"; fi
    echo "hydra: run $id" >&2
    ;;
  hydra-show)
    if [ -f "$state/show/$2" ]; then
      cat "$state/show/$2"
      exit 0
    fi
    echo "hydra: no run record for $2" >&2
    exit 1
    ;;
  hunk-session)
    cat > "$call.stdin"
    ;;
  hunk-show)
    echo "hunk shows $2"
    ;;
  herdr-pane)
    if [ "$2" = split ]; then
      if [ -f "$state/herdr-split" ]; then
        cat "$state/herdr-split"
      else
        echo '{"result":{"pane":{"pane_id":"pane-9"}}}'
      fi
    fi
    ;;
esac
"#;

/// One launch command on one starting home.
struct LaunchCase {
    label: String,
    args: Vec<String>,
    /// Files under the case home by relative path: `puppets/g1.jsonl`, `vendors.json`,
    /// `root/runs/<id>.jsonl`, `root/worktrees/...`, `stub/answers/<n>`, `stub/show/<id>`.
    files: Vec<(String, Vec<u8>)>,
    /// A link and what it points at, both relative to the case home; `true` for a hard link.
    links: Vec<(String, String, bool)>,
    env: Vec<(&'static str, &'static str)>,
}

impl LaunchCase {
    fn new(label: &str, log: Option<&str>, args: &[&str]) -> LaunchCase {
        let mut case = LaunchCase {
            label: label.to_string(),
            args: args.iter().map(|a| a.to_string()).collect(),
            files: Vec::new(),
            links: Vec::new(),
            env: Vec::new(),
        };
        if let Some(log) = log {
            case = case.file("puppets/g1.jsonl", log);
        }
        case
    }

    fn file(mut self, path: &str, body: &str) -> LaunchCase {
        self.files
            .push((path.to_string(), body.as_bytes().to_vec()));
        self
    }

    /// What `hydra show` prints for the nth run the command launches.
    fn answers(mut self, answers: &[&str]) -> LaunchCase {
        for (n, answer) in answers.iter().enumerate() {
            self = self.file(&format!("stub/answers/{}", n + 1), &shown(answer, None));
        }
        self
    }

    fn herdr(mut self) -> LaunchCase {
        self.env.push(("HERDR_ENV", "1"));
        self.env.push(("HERDR_PANE_ID", "pane-1"));
        self
    }

    fn seed(&self, dir: &Path) {
        for sub in ["puppets", "root/runs", "stub", "grader"] {
            std::fs::create_dir_all(dir.join(sub)).unwrap();
        }
        for (path, body) in &self.files {
            let path = dir.join(path);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, body).unwrap();
        }
        for (link, target, hard) in &self.links {
            let (link, target) = (dir.join(link), dir.join(target));
            std::fs::create_dir_all(link.parent().unwrap()).unwrap();
            if *hard {
                std::fs::hard_link(target, link).unwrap();
            } else {
                std::os::unix::fs::symlink(target, link).unwrap();
            }
        }
    }
}

/// What the real `hydra show` prints for an ended run, `src/outcome.rs`'s `RunResult`, with
/// this `result_text` and, for a run that left one, this branch.
fn shown(result_text: &str, branch: Option<&str>) -> String {
    let branch = branch.map_or(String::new(), |b| format!("\"branch\":{},", json_string(b)));
    format!(
        r#"{{"run_id":"shown","outcome":"completed","outcome_source":"provider","exit_code":0,"error":null,"result_text":{},{branch}"usage":{{"usage_source":"provider","counts":{{"input_tokens":5,"output_tokens":7,"raw_input":5,"total_tokens":12}}}},"run_file":"/f.jsonl"}}"#,
        json_string(result_text)
    )
}

/// Replace every occurrence of one implementation's home with `<home>`.
fn mask_home(bytes: &[u8], dir: &Path) -> Vec<u8> {
    let needle = dir.to_str().unwrap().as_bytes();
    let mut out = Vec::new();
    let mut rest = bytes;
    while let Some(at) = rest.windows(needle.len()).position(|w| w == needle) {
        out.extend_from_slice(&rest[..at]);
        out.extend_from_slice(b"<home>");
        rest = &rest[at + needle.len()..];
    }
    out.extend_from_slice(rest);
    out
}

fn files_under(dir: &Path, prefix: &str, out: &mut Vec<(String, Vec<u8>)>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries {
        let path = entry.unwrap().path();
        let name = format!("{prefix}{}", path.file_name().unwrap().to_string_lossy());
        if path.is_dir() {
            files_under(&path, &format!("{name}/"), out);
        } else {
            out.push((name, std::fs::read(&path).unwrap()));
        }
    }
}

/// What one implementation left: every file under `puppets/` and every stub call, with the
/// stamps of appended log lines and the implementation's own home masked.
fn launched(case: &LaunchCase, dir: &Path) -> Vec<(String, Vec<u8>)> {
    let mut files = Vec::new();
    files_under(&dir.join("puppets"), "puppets/", &mut files);
    files_under(&dir.join("stub/calls"), "calls/", &mut files);
    for (name, bytes) in &mut files {
        if name.ends_with(".jsonl") {
            let before = case
                .files
                .iter()
                .find(|(path, _)| path == name)
                .map_or(&[][..], |(_, body)| body);
            *bytes = mask_appended_ts(before, bytes);
        }
        *bytes = mask_home(bytes, dir);
    }
    files.sort();
    files
}

fn run_launch(program: &[&str], case: &LaunchCase, dir: &Path, bin: &Path, cwd: &Path) -> Outcome {
    let mut cmd = Command::new(program[0]);
    let path = format!(
        "{}:{}",
        bin.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    cmd.args(&program[1..])
        .args(&case.args)
        .current_dir(cwd)
        .stdin(std::process::Stdio::null())
        .env_clear()
        .env("PATH", path)
        .env("HOME", dir)
        .env("LC_ALL", "en_US.UTF-8")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("HYDRA_BIN", bin.join("hydra-stub"))
        .env("STUB_STATE", dir.join("stub"))
        .env("HYDRA_PUPPETS_HOME", dir.join("puppets"))
        .env("HYDRA_ROOT", dir.join("root"))
        .env("HYDRA_GRADER_HOME", dir.join("grader"))
        // `review` ends in `execvp`, which does not flush Python's buffer when stdout is a
        // pipe. Unbuffered, it prints what it prints on a terminal.
        .env("PYTHONUNBUFFERED", "1")
        .env("PYTHONDONTWRITEBYTECODE", "1");
    if dir.join("vendors.json").is_file() {
        cmd.env("HYDRA_PUPPETS_VENDORS", dir.join("vendors.json"));
    }
    for (key, value) in &case.env {
        cmd.env(key, value);
    }
    let output = cmd.output().unwrap_or_else(|e| panic!("{program:?}: {e}"));
    let text = |bytes: &[u8]| String::from_utf8_lossy(&mask_home(bytes, dir)).into_owned();
    Outcome {
        code: output.status.code().unwrap_or(-1),
        stdout: text(&output.stdout),
        stderr: text(&output.stderr),
    }
}

fn compare_launch(
    case: &LaunchCase,
    root: &Path,
    n: usize,
    bin: &Path,
    cwd: &Path,
) -> (Option<String>, Vec<String>) {
    let base = root.join(format!("l{n:03}"));
    let (py_dir, rs_dir) = (base.join("py"), base.join("rs"));
    case.seed(&py_dir);
    case.seed(&rs_dir);
    let script = script();
    let py = run_launch(
        &["python3", script.to_str().unwrap()],
        case,
        &py_dir,
        bin,
        cwd,
    );
    let rs = run_launch(&[RUST], case, &rs_dir, bin, cwd);
    let mut problems = Vec::new();
    let (py_files, rs_files) = (launched(case, &py_dir), launched(case, &rs_dir));
    let names = |files: &[(String, Vec<u8>)]| -> Vec<String> {
        files.iter().map(|(name, _)| name.clone()).collect()
    };
    if names(&py_files) != names(&rs_files) {
        problems.push(format!(
            "the files differ\npython left {:?}\nrust left {:?}",
            names(&py_files),
            names(&rs_files)
        ));
    } else {
        for ((name, py_bytes), (_, rs_bytes)) in py_files.iter().zip(&rs_files) {
            if py_bytes != rs_bytes {
                problems.push(format!(
                    "{name} differs\npython:\n{}\nrust:\n{}",
                    String::from_utf8_lossy(py_bytes),
                    String::from_utf8_lossy(rs_bytes)
                ));
            }
        }
    }
    let (class, problem) = judge(&py, &rs);
    problems.extend(problem);
    (class, problems)
}

/// A repository with a base commit on `main` and a branch `hydra/r1` one commit ahead that
/// touches a source file and a test. Returns the base sha.
fn review_repo(repo: &Path) -> String {
    let git = |args: &[&str]| -> String {
        let out = Command::new("git")
            .current_dir(repo)
            .env_clear()
            .env("PATH", std::env::var("PATH").unwrap_or_default())
            .env("HOME", repo)
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .args(["-c", "user.name=t", "-c", "user.email=t@example.com"])
            .args(args)
            .output()
            .unwrap();
        assert!(out.status.success(), "git {args:?}: {out:?}");
        String::from_utf8(out.stdout).unwrap().trim().to_string()
    };
    std::fs::create_dir_all(repo.join("src")).unwrap();
    std::fs::create_dir_all(repo.join("tests")).unwrap();
    git(&["init", "-q", "-b", "main"]);
    std::fs::write(repo.join("src/a.rs"), "fn a() {}\n").unwrap();
    git(&["add", "."]);
    git(&["commit", "-q", "-m", "base"]);
    let base = git(&["rev-parse", "HEAD"]);
    git(&["checkout", "-q", "-b", "hydra/r1"]);
    std::fs::write(repo.join("src/a.rs"), "fn a() {}\nfn b() {}\n").unwrap();
    std::fs::write(repo.join("tests/a_test.rs"), "#[test]\nfn t() {}\n").unwrap();
    git(&["add", "."]);
    git(&["commit", "-q", "-m", "work"]);
    git(&["checkout", "-q", "main"]);
    base
}

/// Answers a judge might give that are not the JSON asked for, one per message
/// `json.JSONDecodeError` has, and the shapes `parse_judge_answer` refuses by name.
const BAD_ANSWERS: [&str; 36] = [
    "I think the diff is fine.",
    "",
    "{",
    "{\"verdict\"",
    "{\"verdict\":}",
    "{\"verdict\":\"pass\",}",
    "[1,]",
    "[1 2]",
    "{\"a\":\"b\" \"c\"}",
    "\"abc",
    "\"a\\qb\"",
    "\"a\\u12\"",
    "\"\\ud800\\u12",
    "\"\\ud800\\udc0",
    "\"\\u1234",
    "{\"verdict\":\"pass\",\"findings\":[]} trailing",
    "\"a\u{1}b\"",
    "-",
    "[",
    "{\"a\":1,\n \"b\"}",
    "nul",
    "{1:2}",
    "[1e]",
    "\n\n  {\"verdict\": \"pass\",\n  \"findings\": [}",
    "[]",
    "{\"verdict\":\"maybe\",\"findings\":[]}",
    "{\"verdict\":\"pass\"}",
    "{\"verdict\":\"pass\",\"findings\":[1]}",
    "{\"verdict\":\"pass\",\"findings\":[{\"item\":1,\"severity\":\"minor\",\"note\":\"n\"}]}",
    "{\"verdict\":\"pass\",\"findings\":[{\"item\":\"it's \\\"new\\\"\",\"severity\":\"minor\",\"note\":\"n\"}]}",
    "{\"verdict\":\"pass\",\"findings\":[{\"item\":\"dead_code\",\"severity\":\"fatal\",\"note\":\"n\"}]}",
    "{\"verdict\":[\"pass\"],\"findings\":[]}",
    "```json\n{\"verdict\":\"pass\",\"findings\":[]}\n```",
    "```\n{\"verdict\":\"block\",\"findings\":[]}\n```\n",
    "```rust\n{\"verdict\":\"pass\",\"findings\":[]}\n```",
    "```json\n{\"verdict\":\"pass\",\"findings\":[]}\n``` and ``` more ```",
];

fn launch_cases(shared: &Path, base_sha: &str) -> Vec<LaunchCase> {
    let line = |s: &str| format!("{s}\n");
    let shared_path = |name: &str| shared.join(name).to_str().unwrap().to_string();
    let repo = shared_path("repo");
    const GOAL: &str = r#"{"seq":1,"ts":"2026-09-30T12:00:00.000Z","event":"goal","text":"ship café ☕","repo":"/repo","base_sha":"abc"}"#;
    let goal = line(GOAL);
    let damaged = [goal.clone(), line("not json")].concat();
    let torn = [goal.clone(), "{\"event\":\"ta".to_string()].concat();
    let mut cases = Vec::new();

    // plan
    for (label, log, args) in [
        (
            "plan: two seats",
            Some(goal.as_str()),
            &["plan", "g1", "--seat", "codex", "--seat", "claude:opus"][..],
        ),
        (
            "plan: seats with an empty model and two colons",
            Some(goal.as_str()),
            &["plan", "g1", "--seat", "claude:", "--seat", "a:b:c"][..],
        ),
        ("plan: no log", None, &["plan", "g1", "--seat", "codex"][..]),
        (
            "plan: a damaged log",
            Some(damaged.as_str()),
            &["plan", "g1", "--seat", "codex"][..],
        ),
        (
            "plan: an unterminated log",
            Some(torn.as_str()),
            &["plan", "g1", "--seat", "codex", "--seat", "claude"][..],
        ),
    ] {
        cases.push(LaunchCase::new(label, log, args));
    }
    let plan = ["plan", "g1", "--seat", "codex:gpt"];
    for (label, stderr) in [
        ("plan: hydra prints no run id", "hydra: could not start\n"),
        ("plan: hydra prints nothing", ""),
        (
            "plan: the run id line is not the first and ends in CRLF",
            "warming up\r\nhydra: run abc-1 and more\r\nhydra: run second\n",
        ),
        ("plan: the run id line is indented", " hydra: run abc\n"),
    ] {
        cases.push(LaunchCase::new(label, Some(&goal), &plan).file("stub/run-stderr", stderr));
    }
    cases.push(LaunchCase::new("plan: inside Herdr", Some(&goal), &plan).herdr());
    for (label, split) in [
        ("plan: Herdr answers with no JSON", "no"),
        ("plan: Herdr answers without a pane", "{\"result\":{}}"),
    ] {
        cases.push(
            LaunchCase::new(label, Some(&goal), &plan)
                .herdr()
                .file("stub/herdr-split", split),
        );
    }

    // plans
    let proposed = [
        goal.clone(),
        line(r#"{"event":"plan_proposed","provider":"codex","run_id":"p1"}"#),
        line(r#"{"event":"plan_proposed","provider":"claude","model":"opus","run_id":"p2"}"#),
    ]
    .concat();
    let ended =
        "{\"hydra\":{\"type\":\"run_start\"}}\n\nnot json\n{\"hydra\":{\"type\":\"run_end\"}}\n";
    let plans = ["plans", "g1"];
    cases.push(
        LaunchCase::new(
            "plans: one from the worktree, one from the run's text",
            Some(&proposed),
            &plans,
        )
        .file("root/worktrees/p1/docs/proposals/b.md", "second\n")
        .file("root/worktrees/p1/docs/proposals/a.md", "first café\r\n")
        .file("root/runs/p2.jsonl", ended)
        .file("stub/show/p2", &shown("the plan\n```json\n[]\n```\n", None)),
    );
    cases.push(
        LaunchCase::new("plans: neither run has ended", Some(&proposed), &plans).file(
            "root/runs/p1.jsonl",
            "{\"hydra\":{\"type\":\"run_start\"}}\n",
        ),
    );
    cases.push(
        LaunchCase::new(
            "plans: a bare run_end line, blank text, and a show that fails",
            Some(&proposed),
            &plans,
        )
        .file("root/runs/p1.jsonl", "{\"type\":\"run_end\"}\n")
        .file("root/runs/p2.jsonl", ended)
        .file("stub/show/p1", &shown(" \n", None)),
    );
    cases.push(
        LaunchCase::new("plans: show prints null", Some(&proposed), &plans)
            .file("root/runs/p1.jsonl", ended)
            .file("stub/show/p1", "null\n"),
    );
    cases.push(
        LaunchCase::new("plans: show prints no JSON", Some(&proposed), &plans)
            .file("root/runs/p1.jsonl", ended)
            .file("stub/show/p1", "oops"),
    );
    let mut linked = LaunchCase::new(
        "plans: a symlink is passed over and a hard link skipped by name",
        Some(&proposed),
        &plans,
    )
    .file("root/outside.md", "outside\n")
    .file("root/runs/p1.jsonl", ended)
    .file("stub/show/p1", &shown("from the run", None));
    linked.links = vec![
        (
            "root/worktrees/p1/docs/proposals/a.md".into(),
            "root/outside.md".into(),
            false,
        ),
        (
            "root/worktrees/p1/docs/proposals/b.md".into(),
            "root/outside.md".into(),
            true,
        ),
    ];
    cases.push(linked);
    cases.push(
        LaunchCase::new(
            "plans: a directory among the proposals",
            Some(&proposed),
            &plans,
        )
        .file("root/worktrees/p1/docs/proposals/a.md/inner", "x")
        .file("root/worktrees/p1/docs/proposals/z.md", "z\n"),
    );
    let ready = [
        proposed.clone(),
        line(
            r#"{"event":"plan_ready","provider":"codex","run_id":"p1","path":"/somewhere/codex.md"}"#,
        ),
        line(
            r#"{"event":"plan_ready","provider":"claude","run_id":"p2","path":"/somewhere/claude.md"}"#,
        ),
    ]
    .concat();
    cases.push(LaunchCase::new(
        "plans: every plan already has a path",
        Some(&ready),
        &plans,
    ));
    cases.push(LaunchCase::new("plans: no log", None, &plans));
    cases.push(LaunchCase::new(
        "plans: a damaged log",
        Some(&damaged),
        &plans,
    ));

    // launch
    let task = |id: &str, grade: &str, done: &str| {
        line(&format!(
            r#"{{"event":"task","id":"{id}","text":"do {id} ☕","class":"docs","grade":"{grade}","done":{done},"deps":[],"paths":[]}}"#
        ))
    };
    let assignment = |id: &str, provider: &str, model: &str| {
        line(&format!(
            r#"{{"event":"assignment","task":"{id}","provider":"{provider}",{model}"policy":"standard","source":"human"}}"#
        ))
    };
    let approval = |kind: &str, reference: &str, revision: u32| {
        line(&format!(
            r#"{{"event":"approval","kind":"{kind}","ref":{reference},"revision":{revision},"yes":true}}"#
        ))
    };
    let assigned = [
        goal.clone(),
        task("t1", "low", "null"),
        task("t2", "high", "\"cargo test passes\""),
        task("t3", "low", "null"),
        assignment("t1", "cursor-agent", "\"model\":\"composer-2.5\","),
        assignment("t2", "codex", ""),
    ]
    .concat();
    let approved = [
        assigned.clone(),
        approval("table", "null", 5),
        approval("launch", "\"t2\"", 5),
    ]
    .concat();
    let ran = [
        approved.clone(),
        line(r#"{"event":"launch_intent","task":"t1","attempt":1,"provider":"cursor-agent","model":"composer-2.5","policy":"standard","argv_sha256":"x"}"#),
        line(r#"{"event":"run","task":"t1","attempt":1,"run_id":"r1"}"#),
    ]
    .concat();
    let run_ended = [
        ran.clone(),
        line(r#"{"event":"run_end","task":"t1","run_id":"r1","outcome":"completed","total_tokens":12}"#),
    ]
    .concat();
    let dangling = [
        approved.clone(),
        line(r#"{"event":"launch_intent","task":"t1","attempt":4,"provider":"cursor-agent","model":"composer-2.5","policy":"standard","argv_sha256":"x"}"#),
    ]
    .concat();
    for (label, log, args) in [
        (
            "launch: no table approval",
            &assigned,
            &["launch", "g1", "t1"][..],
        ),
        (
            "launch: a task with a model",
            &approved,
            &["launch", "g1", "t1"][..],
        ),
        (
            "launch: codex, no model, a done line, a launch approval",
            &approved,
            &["launch", "g1", "t2"][..],
        ),
        (
            "launch: no assignment",
            &approved,
            &["launch", "g1", "t3"][..],
        ),
        (
            "launch: an unknown task",
            &approved,
            &["launch", "g1", "nope"][..],
        ),
        (
            "launch: a second attempt",
            &run_ended,
            &["launch", "g1", "t1"][..],
        ),
        (
            "launch: a dangling intent",
            &dangling,
            &["launch", "g1", "t1"][..],
        ),
        (
            "launch: a damaged log",
            &damaged,
            &["launch", "g1", "t1"][..],
        ),
    ] {
        cases.push(LaunchCase::new(label, Some(log), args));
    }
    cases.push(LaunchCase::new(
        "launch: no log",
        None,
        &["launch", "g1", "t1"],
    ));
    cases.push(
        LaunchCase::new(
            "launch: hydra prints no run id",
            Some(&approved),
            &["launch", "g1", "t1"],
        )
        .file("stub/run-stderr", "hydra: refused\n"),
    );
    cases.push(
        LaunchCase::new(
            "launch: inside Herdr",
            Some(&approved),
            &["launch", "g1", "t1"],
        )
        .herdr(),
    );

    // wait
    let wait = ["wait", "g1"];
    let two_pending = [
        ran.clone(),
        line(r#"{"event":"launch_intent","task":"t2","attempt":1,"provider":"codex","model":"","policy":"standard","argv_sha256":"x"}"#),
        line(r#"{"event":"run","task":"t2","attempt":1,"run_id":"r2"}"#),
    ]
    .concat();
    cases.push(LaunchCase::new(
        "wait: nothing pending",
        Some(&approved),
        &wait,
    ));
    cases.push(LaunchCase::new("wait: no log", None, &wait));
    cases.push(LaunchCase::new(
        "wait: a damaged log",
        Some(&damaged),
        &wait,
    ));
    cases.push(LaunchCase::new(
        "wait: a pending run with no record",
        Some(&ran),
        &wait,
    ));
    cases.push(
        LaunchCase::new(
            "wait: one ended, one still running",
            Some(&two_pending),
            &wait,
        )
        .file(
            "root/runs/r1.jsonl",
            "{\"hydra\":{\"type\":\"run_start\"}}\n",
        )
        .file("root/runs/r2.jsonl", ended)
        .file("stub/show/r2", &shown("done", None)),
    );
    cases.push(
        LaunchCase::new(
            "wait: both ended, the second has no outcome or counts",
            Some(&two_pending),
            &wait,
        )
        .file("root/runs/r1.jsonl", "{\"type\":\"run_end\"}\r\n")
        .file("root/runs/r2.jsonl", ended)
        .file("stub/show/r1", &shown("done", None))
        .file(
            "stub/show/r2",
            "{\"outcome\":null,\"usage\":{\"usage_source\":\"unavailable\"}}",
        ),
    );
    for (label, show) in [
        (
            "wait: total_tokens is a float",
            "{\"outcome\":\"failed\",\"usage\":{\"counts\":{\"total_tokens\":7.9}}}",
        ),
        (
            "wait: total_tokens is a string",
            "{\"outcome\":\"failed\",\"usage\":{\"counts\":{\"total_tokens\":\" 1_0 \"}}}",
        ),
        (
            "wait: total_tokens is not a number",
            "{\"outcome\":\"failed\",\"usage\":{\"counts\":{\"total_tokens\":\"many\"}}}",
        ),
        (
            "wait: total_tokens is a list",
            "{\"outcome\":\"failed\",\"usage\":{\"counts\":{\"total_tokens\":[1]}}}",
        ),
        ("wait: show prints null", "null"),
        ("wait: show prints no JSON", "oops"),
    ] {
        cases.push(
            LaunchCase::new(label, Some(&ran), &wait)
                .file("root/runs/r1.jsonl", ended)
                .file("stub/show/r1", show),
        );
    }
    cases.push(
        LaunchCase::new("wait: the run ended and show fails", Some(&ran), &wait)
            .file("root/runs/r1.jsonl", ended),
    );
    cases.push(
        LaunchCase::new("wait: a run record that is a list", Some(&ran), &wait)
            .file("root/runs/r1.jsonl", "[1]\n"),
    );

    // choose --judge
    let codex_down = r#"[{"provider":"codex","model":null,"available":false}]"#;
    let two_down = r#"[{"provider":"codex","model":null,"available":false},{"provider":"cursor-agent","available":false}]"#;
    let proposals = [
        goal.clone(),
        line(r#"{"event":"plan_proposed","provider":"codex","model":null,"run_id":"p1"}"#),
        line(r#"{"event":"plan_proposed","provider":"claude","model":"opus","run_id":"p2"}"#),
        line(&format!(
            r#"{{"event":"plan_ready","provider":"codex","run_id":"p1","path":"{}"}}"#,
            shared_path("codex.md")
        )),
        line(&format!(
            r#"{{"event":"plan_ready","provider":"claude","run_id":"p2","path":"{}"}}"#,
            shared_path("claude.md")
        )),
    ]
    .concat();
    let degraded = |approvals: &[&str]| {
        let mut log = proposals.clone();
        for approval in approvals {
            log.push_str(&line(approval));
        }
        log
    };
    let judge = ["choose", "g1", "--judge"];
    let pick = |name: &str| format!("{{\"chosen\": \"{name}\", \"findings\": []}}");
    let (codex, claude, agy) = (pick("codex"), pick("claude"), pick("agy"));
    let composed = "{\"chosen\": \"composed\"}\n\n```json\n[{\"id\": \"a\", \"text\": \"café\", \"n\": [1.5, null]}]\n```\n";
    let only_composed = "{\"chosen\":\"composed\"}";
    let fenced = format!("```json\n{codex}\n```");
    let with_finding = "{\"chosen\":\"codex\",\"findings\":[{\"item\":\"anything\",\"severity\":\"major\",\"note\":\"n\"}]} and prose after";
    for (label, answers) in [
        (
            "choose --judge: three seats agree",
            [codex.as_str(), &codex, &codex],
        ),
        (
            "choose --judge: a fenced answer and trailing prose",
            [fenced.as_str(), with_finding, &codex],
        ),
        (
            "choose --judge: the seats disagree",
            [codex.as_str(), &claude, &codex],
        ),
        (
            "choose --judge: a provider with no plan",
            [agy.as_str(), &agy, &agy],
        ),
        (
            "choose --judge: composed with a task block",
            [composed, only_composed, composed],
        ),
        (
            "choose --judge: composed with no task block",
            [only_composed, only_composed, only_composed],
        ),
        (
            "choose --judge: answers that are not JSON",
            ["prose", "", " {\"chosen\": "],
        ),
        (
            "choose --judge: answers of the wrong shape",
            [
                "[1]",
                "{\"chosen\":\" \"}",
                "{\"chosen\":\"codex\",\"findings\":{}}",
            ],
        ),
        (
            "choose --judge: findings of the wrong shape",
            [
                "{\"chosen\":\"codex\",\"findings\":[1]}",
                "{\"chosen\":\"codex\",\"findings\":[{\"item\":\"i\",\"note\":2}]}",
                "{\"chosen\":\"codex\",\"findings\":[{\"item\":\"i\",\"note\":\"n\",\"severity\":\"bad\"}]}",
            ],
        ),
    ] {
        cases.push(LaunchCase::new(label, Some(&proposals), &judge).answers(&answers));
    }
    cases.push(
        LaunchCase::new(
            "choose --judge: codex down, nothing approved",
            Some(&proposals),
            &judge,
        )
        .file("vendors.json", codex_down),
    );
    cases.push(
        LaunchCase::new(
            "choose --judge: codex down, claude with a model sits in",
            Some(&degraded(&[
                r#"{"event":"approval","kind":"degraded","missing":"codex","substitute":"claude","model":"sonnet","revision":0,"yes":true}"#,
            ])),
            &judge,
        )
        .file("vendors.json", codex_down)
        .answers(&[&codex, &codex, &codex]),
    );
    cases.push(
        LaunchCase::new(
            "choose --judge: two seats down, a seat provider and an outsider sit in",
            Some(&degraded(&[
                r#"{"event":"approval","kind":"degraded","missing":"codex","substitute":"agy","revision":0,"yes":true}"#,
                r#"{"event":"approval","kind":"degraded","missing":"cursor-agent","substitute":"codex","revision":0,"yes":true}"#,
            ])),
            &judge,
        )
        .file("vendors.json", two_down)
        .answers(&[&claude, &claude, &claude]),
    );
    cases.push(
        LaunchCase::new(
            "choose --judge: hydra prints no run id",
            Some(&proposals),
            &judge,
        )
        .file("stub/run-stderr", "nope\n"),
    );

    // review
    let reviewed = [
        line(&format!(
            r#"{{"seq":1,"ts":"2026-09-30T12:00:00.000Z","event":"goal","text":"ship","repo":"{repo}","base_sha":"{base_sha}"}}"#
        )),
        task("t1", "low", "null"),
        task("t2", "low", "null"),
        assignment("t1", "cursor-agent", "\"model\":\"composer-2.5\","),
        approval("table", "null", 3),
        line(r#"{"event":"launch_intent","task":"t1","attempt":1,"provider":"cursor-agent","model":"composer-2.5","policy":"standard","argv_sha256":"x"}"#),
        line(r#"{"event":"run","task":"t1","attempt":1,"run_id":"r1"}"#),
        line(r#"{"event":"run_end","task":"t1","run_id":"r1","outcome":"completed","total_tokens":12}"#),
    ]
    .concat();
    let no_open = ["review", "g1", "t1", "--no-open"];
    let branch = Some("hydra/r1");
    let pass = "{\"verdict\":\"pass\",\"findings\":[]}";
    let findings = r#"{"verdict":"pass","findings":[
        {"item":"dead_code","severity":"critical","note":"clamped to minor"},
        {"item":"scope_match","severity":"major","note":"café ☕ \"quoted\""},
        {"item":"security_issues","severity":"critical","note":"a key"}]}"#;
    let blocks = "{\"verdict\":\"block\",\"findings\":[{\"item\":\"tests_present\",\"severity\":\"minor\",\"note\":\"none\"}]}";
    cases.push(LaunchCase::new(
        "review: a task with no run",
        Some(&reviewed),
        &["review", "g1", "t2", "--no-open"],
    ));
    cases.push(LaunchCase::new(
        "review: an unknown task",
        Some(&reviewed),
        &["review", "g1", "nope"],
    ));
    cases.push(LaunchCase::new("review: no log", None, &no_open));
    cases.push(LaunchCase::new(
        "review: show fails",
        Some(&reviewed),
        &no_open,
    ));
    cases.push(
        LaunchCase::new("review: the run has no branch", Some(&reviewed), &no_open)
            .file("stub/show/r1", &shown("text", None)),
    );
    cases.push(
        LaunchCase::new("review: show prints a list", Some(&reviewed), &no_open)
            .file("stub/show/r1", "[1]"),
    );
    cases.push(
        LaunchCase::new(
            "review --no-open: three passes, the run's text is the projection",
            Some(&reviewed),
            &no_open,
        )
        .file("stub/show/r1", &shown("what the run said\n", branch))
        .answers(&[pass, pass, pass]),
    );
    cases.push(
        LaunchCase::new(
            "review --no-open: no result text, the record is the projection",
            Some(&reviewed),
            &no_open,
        )
        .file("stub/show/r1", &shown("", branch))
        .answers(&[pass, pass, pass]),
    );
    cases.push(
        LaunchCase::new(
            "review --no-open: findings go to Hunk and a critical one blocks",
            Some(&reviewed),
            &no_open,
        )
        .file("stub/show/r1", &shown("text", branch))
        .answers(&[findings, blocks, pass]),
    );
    cases.push(
        LaunchCase::new(
            "review --no-open: a branch that does not exist",
            Some(&reviewed),
            &no_open,
        )
        .file("stub/show/r1", &shown("text", Some("hydra/absent")))
        .answers(&[findings, pass, pass]),
    );
    cases.push(
        LaunchCase::new(
            "review --no-open: codex down, nothing approved",
            Some(&reviewed),
            &no_open,
        )
        .file("stub/show/r1", &shown("text", branch))
        .file("vendors.json", codex_down),
    );
    cases.push(
        LaunchCase::new(
            "review --no-open: codex down, claude sits in",
            Some(
                &[
                    reviewed.clone(),
                    line(r#"{"event":"approval","kind":"degraded","missing":"codex","substitute":"claude","revision":3,"yes":true}"#),
                ]
                .concat(),
            ),
            &no_open,
        )
        .file("stub/show/r1", &shown("text", branch))
        .file("vendors.json", codex_down)
        .answers(&[pass, blocks, pass]),
    );
    cases.push(
        LaunchCase::new(
            "review: Hunk is opened in place of the command",
            Some(&reviewed),
            &["review", "g1", "t1"],
        )
        .file("stub/show/r1", &shown("text", branch))
        .answers(&[pass, findings, pass]),
    );
    cases.push(
        LaunchCase::new(
            "review: inside Herdr the diff goes to a new pane",
            Some(&reviewed),
            &["review", "g1", "t1"],
        )
        .file("stub/show/r1", &shown("text", branch))
        .answers(&[findings, pass, pass])
        .herdr(),
    );
    cases.push(
        LaunchCase::new(
            "review --no-open: inside Herdr no pane is opened",
            Some(&reviewed),
            &no_open,
        )
        .file("stub/show/r1", &shown("text", branch))
        .answers(&[pass, pass, pass])
        .herdr(),
    );
    for (n, answers) in BAD_ANSWERS.chunks(3).enumerate() {
        cases.push(
            LaunchCase::new(
                &format!("review --no-open: answers that do not parse, set {n}"),
                Some(&reviewed),
                &no_open,
            )
            .file("stub/show/r1", &shown("text", branch))
            .answers(answers),
        );
    }
    cases
}

#[test]
fn the_launch_commands_match_puppets_py_and_hand_the_stubs_the_same_calls() {
    use std::os::unix::fs::PermissionsExt;
    let scratch = Scratch::new("launch");
    let (shared, bin) = (scratch.0.join("shared"), scratch.0.join("bin"));
    std::fs::create_dir_all(&shared).unwrap();
    std::fs::create_dir_all(&bin).unwrap();
    for name in ["hydra-stub", "hunk", "herdr"] {
        std::fs::write(bin.join(name), STUB).unwrap();
        std::fs::set_permissions(bin.join(name), std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    std::fs::write(
        shared.join("codex.md"),
        "  Codex plan.\r\n\r\n```json\n[]\n```\n\n",
    )
    .unwrap();
    std::fs::write(shared.join("claude.md"), "Claude plan, café.\n").unwrap();
    let base_sha = review_repo(&shared.join("repo"));
    let cases = launch_cases(&shared, &base_sha);
    let mut crashes = Vec::new();
    let mut mismatches = Vec::new();
    for (n, case) in cases.iter().enumerate() {
        let (class, problems) = compare_launch(case, &scratch.0, n, &bin, &shared);
        crashes.extend(class.map(|class| format!("{}: {class}", case.label)));
        if !problems.is_empty() {
            mismatches.push(format!("{}\n{}", case.label, problems.join("\n")));
        }
    }
    eprintln!("compared {} launch command runs:", cases.len());
    for case in &cases {
        eprintln!("  {}", case.label);
    }
    eprintln!(
        "python raised, stderr compared by exception class ({}):",
        crashes.len()
    );
    for crash in &crashes {
        eprintln!("  {crash}");
    }
    assert!(
        mismatches.is_empty(),
        "{} of {} differ:\n\n{}",
        mismatches.len(),
        cases.len(),
        mismatches.join("\n\n")
    );
}

#[test]
fn the_comparator_masks_an_implementation_home_and_nothing_else() {
    let dir = Path::new("/tmp/case/py");
    assert_eq!(
        mask_home(b"a /tmp/case/py/puppets b /tmp/case/rs /tmp/case/py", dir),
        b"a <home>/puppets b /tmp/case/rs <home>"
    );
    assert_eq!(mask_home(b"/tmp/case/p", dir), b"/tmp/case/p");
    assert_eq!(mask_home(b"", dir), b"");
}

/// Build ticket 207: `c` at the variant character is the Microsoft variant and `0` the NCS
/// one, and `uuid.uuid4()` prints neither.
#[test]
fn goal_id_requires_the_uuid4_variant() {
    assert!(!is_goal_id("0b9f2c3e-6a1d-4c7b-0e2f-5d8a7b6c4e31"));
    for (variant, valid) in [
        ('7', false),
        ('8', true),
        ('9', true),
        ('a', true),
        ('b', true),
        ('c', false),
        ('f', false),
    ] {
        let id = format!("0b9f2c3e-6a1d-4c7b-{variant}e2f-5d8a7b6c4e31");
        assert_eq!(is_goal_id(&id), valid, "{id}");
    }
}

/// Build ticket 207: a writer that prints one valid id and creates the log under another
/// used to pass, because stdout and the file name were masked apart.
#[test]
fn new_stdout_must_name_the_created_log() {
    let scratch = Scratch::new("new-id-link");
    let (dir, seed) = (scratch.0.join("run"), scratch.0.join("seed"));
    std::fs::create_dir_all(dir.join("puppets")).unwrap();
    std::fs::create_dir_all(seed.join("puppets")).unwrap();
    let printed = "0b9f2c3e-6a1d-4c7b-9e2f-5d8a7b6c4e31";
    let created = "1b9f2c3e-6a1d-4c7b-9e2f-5d8a7b6c4e31";
    std::fs::write(
        dir.join("puppets").join(format!("{created}.jsonl")),
        b"{}\n",
    )
    .unwrap();
    assert!(!new_stdout_names_log(&format!("{printed}\n"), &dir));
    assert!(new_stdout_names_log(&format!("{created}\n"), &dir));
    // The id that names no log stays on stdout, and the log it did not name keeps its name.
    let stdout = format!("{printed}\n");
    assert_eq!(mask_new_stdout(&stdout, &dir), stdout);
    let goal = new_goal_id(&stdout, &dir);
    let names = |goal: Option<&str>| -> Vec<String> {
        let files = written(&dir, &seed, goal);
        files.into_iter().map(|(name, _)| name).collect()
    };
    assert_eq!(names(goal), [format!("{created}.jsonl")]);
    assert_eq!(names(Some(created)), ["<goal>.jsonl"]);
}

fn python(code: &str) -> String {
    let out = Command::new("python3")
        .args(["-c", code])
        .env("PYTHONDONTWRITEBYTECODE", "1")
        .env("PYTHONIOENCODING", "utf-8")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap()
}

/// Build ticket 207. `int()` under the `python3` on this machine, for every code point
/// before a `7`, after one and alone: which it strips, which are digits and of what value.
/// It is the check on the digit table and the whitespace rule in `src/py.rs`.
#[test]
fn the_int_parser_matches_python_on_every_code_point() {
    let expected = python(
        r#"
def read(text):
    try:
        return str(int(text))
    except ValueError:
        return '-'
for c in range(0x110000):
    if 0xD800 <= c < 0xE000:
        continue
    ch = chr(c)
    row = [read(ch + '7'), read('7' + ch), read(ch), read(ch + '_7'), read('7_' + ch)]
    if any(cell != '-' for cell in row):
        print('%x' % c, ' '.join(row))
"#,
    );
    let read = |text: String| hydra_puppets::int_literal(&text).unwrap_or_else(|| "-".to_string());
    let mut got = String::new();
    for c in (0..0x110000).filter_map(char::from_u32) {
        let row = [
            read(format!("{c}7")),
            read(format!("7{c}")),
            read(format!("{c}")),
            read(format!("{c}_7")),
            read(format!("7_{c}")),
        ];
        if row.iter().any(|cell| cell != "-") {
            got.push_str(&format!("{:x} {}\n", u32::from(c), row.join(" ")));
        }
    }
    assert!(
        expected.lines().count() > 700,
        "python printed {expected:?}"
    );
    for (ours, theirs) in got.lines().zip(expected.lines()) {
        assert_eq!(ours, theirs);
    }
    assert_eq!(got.lines().count(), expected.lines().count());
}

/// Build ticket 207. `math.lgamma` and `sheet.beta_lower` under the `python3` on this
/// machine, bit for bit: `rank` orders agents by a quotient of the second, and a last bit
/// swaps a near tie.
#[test]
fn the_ranking_arithmetic_matches_python() {
    let bits = |f: f64| format!("{:016x}", f.to_bits());
    let expected = python(
        r#"
import math, struct
for k in range(1, 6001):
    for x in (float(k), k / 2, k / 7):
        print(struct.pack('>d', math.lgamma(x)).hex())
"#,
    );
    let mut got = String::new();
    for k in 1..=6000 {
        let k = f64::from(k);
        for x in [k, k / 2.0, k / 7.0] {
            got.push_str(&bits(hydra_puppets::sheet::lgamma(x)));
            got.push('\n');
        }
    }
    let wrong = got
        .lines()
        .zip(expected.lines())
        .filter(|(ours, theirs)| ours != theirs)
        .count();
    assert_eq!(wrong, 0, "lgamma differs on {wrong} of 18000 arguments");
    assert_eq!(got.len(), expected.len());

    let pairs: Vec<(i128, i128)> = (0..=24)
        .flat_map(|a| (0..=24).map(move |b| (a, b)))
        .chain([(120, 80), (300, 10), (2, 400)])
        .collect();
    let list: Vec<String> = pairs.iter().map(|(a, b)| format!("({a},{b})")).collect();
    let code = format!(
        "import struct, sys\nsys.path.insert(0, {:?})\nimport sheet\nfor a, b in [{}]:\n    print(struct.pack('>d', sheet.beta_lower(a, b)).hex())\n",
        script().parent().unwrap().to_str().unwrap(),
        list.join(",")
    );
    let expected = python(&code);
    let expected: Vec<&str> = expected.lines().collect();
    assert_eq!(expected.len(), pairs.len());
    for ((a, b), theirs) in pairs.iter().zip(expected) {
        let ours = hydra_puppets::sheet::beta_lower(*a, *b).unwrap();
        assert_eq!(bits(ours), theirs, "beta_lower({a}, {b})");
    }
}
