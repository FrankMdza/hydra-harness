//! `hydra-puppets` must print what `puppets.py` printed on the same goal logs.
//!
//! Build ticket 196 deleted `puppets.py`. Before it went, `record.py` ran it once on every
//! input here and wrote what it gave under `tests/fixtures/`: per case, each command's exit
//! code, stdout and stderr, and the files it left. These tests run `hydra-puppets` on the
//! same inputs and compare with those files. No test starts `python3`.
//!
//! The inputs of the read commands are every goal log the Python tests left on disk (kept as
//! the `seed` of `fixtures/read/h-*.json`), plus the edge logs below. For each, `show`,
//! `board`, `pending --json`, `queue`, `repair` and `repair --yes` run, and stdout, stderr
//! and the exit code must be what the Python gave. After `repair --yes` the repaired copy is
//! read back with `show` and `pending --json`, and those must be equal too.
//!
//! These are normalized, and only these:
//! - `pending --json` prints an `argv` that starts with the command that records the
//!   approval. For the Python that is two elements, its `sys.executable` and its own path;
//!   since build 196 the Rust prints one, its own path. Only that pair is replaced, by the
//!   Rust's one.
//! - `repair --yes` names the torn file with the time it ran, in stdout and on disk, and
//!   stamps the `repair` event it appends. Only a name whose 24 characters are a time of
//!   `utc_now()`'s shape is masked, and only the `ts` of the one line the command appended.
//! - Where the Python died with a traceback, the Rust must exit 1 with the same stdout, and
//!   its one-line error must start `hydra-puppets: <Class>:` with the Python's exception
//!   class; the rest of the traceback is not compared.
//! - A damaged log prints the same named error on stderr in both, with no traceback.
//!
//! The commands that write (`new`, `set`, `approve`, `verdict`, `choose`, `tasks`, `assign`,
//! `reconcile`) each run on the starting home the Python ran on, and the goal logs left
//! behind must be equal line by line. Two more things are normalized there:
//! - the `ts` of each line the command appended, and of no line the input already held;
//! - the goal id `new` generates, in its stdout and in the name of the log it creates.
//!
//! The commands that launch (`plan`, `plans`, `launch`, `wait`, `review`, `choose --judge`)
//! run with a stub in place of `hydra`, `hunk` and `herdr`. The stub writes down each call,
//! and the calls must be equal too: argv, environment, working directory and stdin. One more
//! thing is normalized there: the home the command ran in, wherever it appears.
//!
//! Recording added one normalization, and three things that make a run repeat on another
//! machine:
//! - The directory a group of cases lives under (its plans, its stubs, its repository) is
//!   the recorder's in the fixture and a scratch one here. Each is replaced by `<root>`.
//! - Every command runs with an environment built from nothing, the same variables under
//!   both, so neither the recorder's locale nor its `PATH` is in a fixture.
//! - The data files (`vendors.json`, `aliases.json`, `priors.json`, `prompts/`) are the
//!   frozen copies in `fixtures/data/`, so a seat going unavailable does not change what a
//!   case prints.
//! - The repository `review` and `new` read is committed at a fixed time, so its commit ids
//!   are the recorded ones.
//!
//! To record again, from a checkout that still has the Python, see `record.py`.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;

use hydra_puppets::json::{self, Value};

const RUST: &str = env!("CARGO_BIN_EXE_hydra-puppets");

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

/// A scratch directory under the system temp dir, removed on drop. It holds the frozen data
/// files under `data/`, where every command here is told to find them.
struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Scratch {
        let path =
            std::env::temp_dir().join(format!("hydra-puppets-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        let path = path.canonicalize().unwrap();
        prepare_root(&path);
        Scratch(path)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn prepare_root(root: &Path) {
    copy_dir(&fixtures().join("data"), &root.join("data"));
}

/// One goal log to compare: a case directory holding `puppets/`, maybe `grader/scores.jsonl`
/// and `vendors.json`, and the goal id to ask about.
#[derive(Clone)]
struct Input {
    name: String,
    label: String,
    dir: PathBuf,
    goal: String,
}

#[derive(Debug, PartialEq, Clone)]
struct Outcome {
    code: i32,
    stdout: String,
    stderr: String,
}

/// What one command gave, before anything is masked.
#[derive(Clone)]
struct Step {
    code: i32,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

impl Step {
    /// The outcome with the home and then the root replaced.
    fn masked(&self, home: &Path, root: &Path) -> Outcome {
        let text = |bytes: &[u8]| {
            String::from_utf8_lossy(&mask_root(&mask_home(bytes, home), root)).into_owned()
        };
        Outcome {
            code: self.code,
            stdout: text(&self.stdout),
            stderr: text(&self.stderr),
        }
    }
}

/// One case: the commands run in one home, in order, and the environment they run in.
/// `record.py` ran the same under `puppets.py`, from the `job.json` this is written to.
struct Job {
    /// The fixture's file name, without `.json`.
    name: String,
    label: String,
    goal: Option<String>,
    steps: Vec<Vec<String>>,
    /// Variables beyond the ones every job has, the later one winning.
    env: Vec<(String, String)>,
    cwd: Option<PathBuf>,
    /// What to keep of the home afterwards, by path under it.
    keep: Vec<&'static str>,
    /// Whether the fixture keeps the starting home too, and the labels the grader read
    /// from the Python fold of it.
    seed: bool,
}

impl Job {
    fn new(name: String, label: &str, steps: Vec<Vec<String>>) -> Job {
        Job {
            name,
            label: label.to_string(),
            goal: None,
            steps,
            env: Vec::new(),
            cwd: None,
            keep: vec!["puppets"],
            seed: false,
        }
    }

    /// Everything in the environment of a command of this job run in `home` under `root`.
    /// Nothing is inherited.
    fn env(&self, home: &Path, root: &Path) -> Vec<(String, String)> {
        let path = |p: PathBuf| p.to_str().unwrap().to_string();
        let mut env: Vec<(String, String)> = [
            ("PATH", format!("{}:/usr/bin:/bin", path(root.join("bin")))),
            ("HOME", path(home.to_path_buf())),
            ("LC_ALL", "en_US.UTF-8".to_string()),
            ("GIT_CONFIG_GLOBAL", "/dev/null".to_string()),
            ("GIT_CONFIG_NOSYSTEM", "1".to_string()),
            ("HYDRA_PUPPETS_HOME", path(home.join("puppets"))),
            ("HYDRA_ROOT", path(home.join("root"))),
            ("HYDRA_GRADER_HOME", path(home.join("grader"))),
            ("HYDRA_PUPPETS_DATA", path(root.join("data"))),
            ("PYTHONDONTWRITEBYTECODE", "1".to_string()),
        ]
        .into_iter()
        .map(|(key, value)| (key.to_string(), value))
        .collect();
        if home.join("vendors.json").is_file() {
            env.push((
                "HYDRA_PUPPETS_VENDORS".into(),
                path(home.join("vendors.json")),
            ));
        }
        for (key, value) in &self.env {
            env.retain(|(k, _)| k != key);
            env.push((key.clone(), value.clone()));
        }
        env
    }

    /// Run every step under `hydra-puppets`.
    fn run(&self, home: &Path, root: &Path) -> Vec<Step> {
        self.steps
            .iter()
            .map(|args| {
                let mut cmd = Command::new(RUST);
                cmd.args(args)
                    .stdin(std::process::Stdio::null())
                    .env_clear()
                    .envs(self.env(home, root));
                if let Some(cwd) = &self.cwd {
                    cmd.current_dir(cwd);
                }
                let output = cmd.output().unwrap_or_else(|e| panic!("{RUST}: {e}"));
                Step {
                    code: output.status.code().unwrap_or(-1),
                    stdout: output.stdout,
                    stderr: output.stderr,
                }
            })
            .collect()
    }

    /// Write `job.json` beside `home`, for `record.py`.
    fn write(&self, home: &Path, root: &Path) {
        let list = |items: &[String]| {
            let cells: Vec<String> = items.iter().map(|item| json_string(item)).collect();
            format!("[{}]", cells.join(","))
        };
        let steps: Vec<String> = self.steps.iter().map(|args| list(args)).collect();
        let env: Vec<String> = self
            .env(home, root)
            .iter()
            .map(|(key, value)| format!("{}:{}", json_string(key), json_string(value)))
            .collect();
        let keep: Vec<String> = self.keep.iter().map(|k| k.to_string()).collect();
        let text = |p: &Path| json_string(p.to_str().unwrap());
        let body = format!(
            "{{\"label\":{},\"goal\":{},\"root\":{},\"home\":{},\"cwd\":{},\"env\":{{{}}},\"steps\":[{}],\"keep\":{},\"seed\":{}}}\n",
            json_string(&self.label),
            self.goal.as_deref().map_or("null".into(), json_string),
            text(root),
            text(home),
            self.cwd.as_deref().map_or("null".into(), text),
            env.join(","),
            steps.join(","),
            list(&keep),
            self.seed
        );
        std::fs::write(home.parent().unwrap().join("job.json"), body).unwrap();
    }
}

/// What `record.py` wrote for one job: what `puppets.py` gave.
struct Recorded {
    root: PathBuf,
    home: PathBuf,
    /// The interpreter and the script the Python names in an `argv`.
    python: String,
    script: String,
    steps: Vec<Step>,
    files: Vec<(String, Vec<u8>)>,
    seed: Vec<(String, Vec<u8>)>,
}

fn member<'a>(value: &'a Value, key: &str) -> &'a Value {
    match value {
        Value::Dict(dict) => dict.get(key).unwrap_or(&Value::Null),
        _ => panic!("the fixture holds no object where `{key}` is read"),
    }
}

fn text_of(value: &Value) -> String {
    match value {
        Value::Str(text) => text
            .to_utf8()
            .expect("a fixture string with a lone surrogate"),
        _ => panic!("the fixture holds no string where text is read"),
    }
}

fn items(value: &Value) -> &[Value] {
    match value {
        Value::List(items) => items,
        Value::Null => &[],
        _ => panic!("the fixture holds no list where one is read"),
    }
}

/// Bytes as `record.py` wrote them: `{"text": ...}` when they were UTF-8, else `{"hex": ...}`.
fn blob(value: &Value) -> Vec<u8> {
    if let Value::Str(text) = member(value, "text") {
        return text.to_utf8().unwrap().into_bytes();
    }
    let hex = text_of(member(value, "hex"));
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect()
}

fn named_blobs(value: &Value) -> Vec<(String, Vec<u8>)> {
    items(value)
        .iter()
        .map(|file| (text_of(member(file, "path")), blob(member(file, "bytes"))))
        .collect()
}

fn load_fixture(path: &Path) -> (Value, Recorded) {
    let raw = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("no fixture {}: {e}", path.display()));
    let value = json::loads_str(&raw).unwrap_or_else(|_| panic!("{} is not JSON", path.display()));
    let recorded = Recorded {
        root: PathBuf::from(text_of(member(&value, "root"))),
        home: PathBuf::from(text_of(member(&value, "home"))),
        python: text_of(member(&value, "python")),
        script: text_of(member(&value, "script")),
        steps: items(member(&value, "steps"))
            .iter()
            .map(|step| Step {
                code: match member(step, "code") {
                    Value::Int(code) => i32::try_from(*code).unwrap(),
                    _ => panic!("{}: a step has no exit code", path.display()),
                },
                stdout: blob(member(step, "stdout")),
                stderr: blob(member(step, "stderr")),
            })
            .collect(),
        files: named_blobs(member(&value, "files")),
        seed: named_blobs(member(&value, "seed")),
    };
    (value, recorded)
}

/// The fixture of `job` in `group`. It must be the recording of this job: the same label and
/// the same commands, so a case edited here is never compared with another's output. A
/// label or an argument that names a file under the root names it under `<root>` in both.
fn recorded(group: &str, job: &Job, root: &Path) -> Recorded {
    let path = fixtures().join(group).join(format!("{}.json", job.name));
    let (value, recorded) = load_fixture(&path);
    let rooted =
        |text: &str, root: &Path| String::from_utf8(mask_root(text.as_bytes(), root)).unwrap();
    assert_eq!(
        rooted(&text_of(member(&value, "label")), &recorded.root),
        rooted(&job.label, root),
        "{}",
        path.display()
    );
    let theirs: Vec<Vec<String>> = items(member(&value, "steps"))
        .iter()
        .map(|step| {
            let args = items(member(step, "args")).iter();
            args.map(|arg| rooted(&text_of(arg), &recorded.root))
                .collect()
        })
        .collect();
    let ours: Vec<Vec<String>> = job
        .steps
        .iter()
        .map(|args| args.iter().map(|arg| rooted(arg, root)).collect())
        .collect();
    assert_eq!(theirs, ours, "{}", path.display());
    recorded
}

/// The fixtures `group` holds, by path, in name order.
fn fixture_paths(group: &str) -> Vec<PathBuf> {
    let mut paths: Vec<PathBuf> = std::fs::read_dir(fixtures().join(group))
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "json"))
        .collect();
    paths.sort();
    paths
}

/// Each test checks it compared as many jobs as its group has fixtures, so a case dropped
/// here leaves no recording that nothing reads.
fn assert_every_fixture_was_read(group: &str, compared: usize) {
    assert_eq!(
        fixture_paths(group).len(),
        compared,
        "fixtures/{group} holds a recording no case reads, or lacks one"
    );
}

fn strings(args: &[&str]) -> Vec<String> {
    args.iter().map(|a| a.to_string()).collect()
}

/// What a group does with each job once its home is seeded: compare it with its fixture, or
/// write it down for `record.py`.
type Visit<'a> = &'a mut dyn FnMut(&Job, &Path);

/// `rs` for the home a test runs in, `py` for the one `record.py` will.
fn side(recording: bool) -> &'static str {
    if recording { "py" } else { "rs" }
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

/// The files a home holds under `keep`, by path under the home, in path order. This is
/// what `record.py` kept of the home the Python ran in.
fn kept(home: &Path, keep: &[&str]) -> Vec<(String, Vec<u8>)> {
    let mut files = Vec::new();
    for top in keep {
        let path = home.join(top);
        if path.is_file() {
            files.push((top.to_string(), std::fs::read(&path).unwrap()));
        } else {
            files_under(&path, &format!("{top}/"), &mut files);
        }
    }
    files.sort();
    files
}

/// What a home held before a command ran: its goal logs, its scores and its vendors file.
const SEED: [&str; 3] = ["puppets", "grader", "vendors.json"];

fn bytes_of<'a>(files: &'a [(String, Vec<u8>)], name: &str) -> &'a [u8] {
    files
        .iter()
        .find(|(path, _)| path == name)
        .map_or(&[][..], |(_, bytes)| bytes)
}

/// The files a home holds after `repair --yes`, names masked, and in each goal log the
/// stamp of the line the command appended. `before` is the home it started from.
fn tree(files: &[(String, Vec<u8>)], before: &[(String, Vec<u8>)]) -> Vec<(String, Vec<u8>)> {
    let mut files: Vec<(String, Vec<u8>)> = files
        .iter()
        .map(|(name, bytes)| {
            let bytes = if name.ends_with(".jsonl") {
                mask_repair_ts(bytes_of(before, name), bytes)
            } else {
                bytes.clone()
            };
            (mask_torn_stamp(name), bytes)
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

/// What each read input is asked, the goal id going in after the command's name. The last
/// two read the repaired copy back: a repaired log folds again, which a damaged one never
/// did.
const READ_STEPS: [&[&str]; 8] = [
    &["show"],
    &["board"],
    &["pending", "--json"],
    &["queue"],
    &["repair"],
    &["repair", "--yes"],
    &["show"],
    &["pending", "--json"],
];

/// The step of `READ_STEPS` that writes.
const REPAIR_YES: usize = 5;

fn read_job(input: &Input) -> Job {
    let steps = READ_STEPS
        .iter()
        .map(|cmd| {
            let mut args = strings(cmd);
            args.insert(1, input.goal.clone());
            args
        })
        .collect();
    let mut job = Job::new(input.name.clone(), &input.label, steps);
    job.goal = Some(input.goal.clone());
    job.seed = true;
    job
}

/// Seed one home per input under `root` and hand each to `visit`.
fn visit_read(root: &Path, inputs: &[Input], recording: bool, visit: Visit) {
    for input in inputs {
        let home = root.join(&input.name).join(side(recording));
        copy_dir(&input.dir, &home);
        visit(&read_job(input), &home);
    }
}

/// Run one read job and compare every step with what the Python gave.
fn compare_read(job: &Job, home: &Path, root: &Path, report: &mut Report) {
    let rec = recorded("read", job, root);
    let before = kept(home, &SEED);
    assert_eq!(
        rec.seed, before,
        "{}: the fixture was recorded from another input",
        job.label
    );
    let steps = job.run(home, root);
    let script = String::from_utf8(mask_root(rec.script.as_bytes(), &rec.root)).unwrap();
    let mut problems = Vec::new();
    for (n, (args, (py, rs))) in job
        .steps
        .iter()
        .zip(rec.steps.iter().zip(&steps))
        .enumerate()
    {
        let mut py = py.masked(&rec.home, &rec.root);
        let mut rs = rs.masked(home, root);
        if args[0] == "pending" {
            py.stdout = mask_argv0(&py.stdout, &rec.python, &script, &helper());
        }
        if n == REPAIR_YES {
            py.stdout = mask_torn_stamp(&py.stdout);
            rs.stdout = mask_torn_stamp(&rs.stdout);
        }
        let then = if n > REPAIR_YES { "then " } else { "" };
        let what = format!("{} {then}`{}`", job.label, args.join(" "));
        let (class, problem) = judge(&py, &rs);
        report
            .crashes
            .extend(class.map(|class| format!("{what}: {class}")));
        problems.extend(problem.map(|problem| format!("{what}\n{problem}")));
        report.compared += 1;
    }
    if tree(&rec.files, &before) != tree(&kept(home, &["puppets"]), &before) {
        problems.push(format!(
            "{}: the files left behind by `repair --yes` differ",
            job.label
        ));
    }
    report.mismatches.extend(problems);
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
            "labels: verdicts, reviews, plans, and the ids 3 and \"3\"",
            [
                GOAL,
                r#"{"event":"plan_proposed","provider":"codex","model":null,"run_id":"p-codex"}"#,
                r#"{"event":"plan_proposed","provider":"claude","model":"opus","run_id":"p-claude"}"#,
                r#"{"event":"panel_verdict","judge_run_ids":["pj1","pj2","pj3"],"chosen":"codex","path":"/p/codex.md"}"#,
                r#"{"event":"task","id":"applied","text":"café ☕","class":"docs","grade":"low"}"#,
                r#"{"event":"task","id":"rejected","text":"two","class":"rust_launch_path","grade":"high"}"#,
                r#"{"event":"task","id":"fixed","text":"three","class":"security_review","grade":"medium"}"#,
                r#"{"event":"task","id":"unjudged","text":"four","class":"probe","grade":"low"}"#,
                r#"{"event":"task","id":3,"text":"the number","class":"docs","grade":"low"}"#,
                r#"{"event":"task","id":"3","text":"the string","class":"docs","grade":"high"}"#,
                r#"{"event":"run","task":"applied","attempt":1,"run_id":"r-applied"}"#,
                r#"{"event":"run_end","task":"applied","run_id":"r-applied","outcome":"completed"}"#,
                r#"{"event":"review","task":"applied","judge_run_ids":["j1","j2","j3"],"verdict":"pass"}"#,
                r#"{"event":"human_verdict","task":"applied","run_id":"r-applied","result":"applied","note":""}"#,
                r#"{"event":"run","task":"rejected","attempt":1,"run_id":"r-rejected-1"}"#,
                r#"{"event":"run_end","task":"rejected","run_id":"r-rejected-1","outcome":"failed"}"#,
                r#"{"event":"human_verdict","task":"rejected","run_id":"r-rejected-1","result":"rejected","note":"no"}"#,
                r#"{"event":"run","task":"rejected","attempt":2,"run_id":"r-rejected-2"}"#,
                r#"{"event":"run","task":"fixed","attempt":1,"run_id":"r-fixed"}"#,
                r#"{"event":"run_end","task":"fixed","run_id":"r-fixed","outcome":"completed"}"#,
                r#"{"event":"review","task":"fixed","judge_run_ids":["j4"],"verdict":"block"}"#,
                r#"{"event":"human_verdict","task":"fixed","run_id":"r-fixed","result":"fixed","note":"by hand"}"#,
                r#"{"event":"run","task":3,"attempt":1,"run_id":"r-number"}"#,
                r#"{"event":"run","task":"3","attempt":1,"run_id":"r-string"}"#,
                r#"{"event":"review","task":3,"judge_run_ids":["j5"],"verdict":"pass"}"#,
                r#"{"event":"review","task":"nobody","judge_run_ids":["j6"],"verdict":"pass"}"#,
                r#"{"event":"review","task":"unjudged","verdict":"pass"}"#,
            ]
            .map(line)
            .concat()
            .into_bytes(),
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
        let name = format!("edge-{n:02}");
        let dir = root.join("inputs").join(&name);
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
            name,
            label: format!("edge: {label}"),
            dir,
            goal: "g1".into(),
        });
    }
    let dir = root.join("inputs/edge-absent");
    std::fs::create_dir_all(dir.join("puppets")).unwrap();
    inputs.push(Input {
        name: "edge-absent".into(),
        label: "edge: no log for the goal".into(),
        dir,
        goal: "absent".into(),
    });
    inputs
}

/// Every goal log the Python tests left on disk, one input per log: the `seed` of each
/// `fixtures/read/h-*.json`, written out under `root`.
fn recorded_inputs(root: &Path) -> Vec<Input> {
    let mut inputs = Vec::new();
    for path in fixture_paths("read") {
        let name = path.file_stem().unwrap().to_string_lossy().into_owned();
        if !name.starts_with("h-") {
            continue;
        }
        let (value, recorded) = load_fixture(&path);
        let dir = root.join("inputs").join(&name);
        std::fs::create_dir_all(dir.join("puppets")).unwrap();
        for (file, bytes) in &recorded.seed {
            let file = dir.join(file);
            std::fs::create_dir_all(file.parent().unwrap()).unwrap();
            std::fs::write(file, bytes).unwrap();
        }
        inputs.push(Input {
            name,
            label: text_of(member(&value, "label")),
            dir,
            goal: text_of(member(&value, "goal")),
        });
    }
    assert!(
        inputs.len() > 50,
        "the fixtures hold only {} harvested goal logs",
        inputs.len()
    );
    inputs
}

/// The same inputs from the directory `record.py harvest` wrote, repeats dropped. Only the
/// recording reads this, from a checkout that still has the Python tests.
fn harvested_inputs(root: &Path, out: &Path) -> Vec<Input> {
    let mut seen = HashSet::new();
    let mut inputs = Vec::new();
    let mut cases: Vec<PathBuf> = std::fs::read_dir(out)
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
            let name = format!("h-{:03}", inputs.len());
            let dir = root.join("inputs").join(&name);
            std::fs::create_dir_all(dir.join("puppets")).unwrap();
            std::fs::copy(&log, dir.join("puppets").join(log.file_name().unwrap())).unwrap();
            if case.join("grader").is_dir() {
                copy_dir(&case.join("grader"), &dir.join("grader"));
            }
            if case.join("vendors.json").is_file() {
                std::fs::copy(case.join("vendors.json"), dir.join("vendors.json")).unwrap();
            }
            let case = case.file_name().unwrap().to_string_lossy().into_owned();
            inputs.push(Input {
                name,
                label: format!("harvest {case}/{goal}"),
                dir,
                goal,
            });
        }
    }
    inputs
}

/// The harvested logs and then the edge logs. `harvest` is set only while recording.
fn read_inputs(root: &Path, harvest: Option<&Path>) -> Vec<Input> {
    let mut inputs = match harvest {
        Some(out) => harvested_inputs(root, out),
        None => recorded_inputs(root),
    };
    inputs.extend(edge_inputs(root));
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
    let root = &scratch.0;
    let inputs = read_inputs(root, None);
    let mut report = Report {
        compared: 0,
        crashes: Vec::new(),
        mismatches: Vec::new(),
    };
    visit_read(root, &inputs, false, &mut |job, home| {
        compare_read(job, home, root, &mut report)
    });
    assert_every_fixture_was_read("read", inputs.len());
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

/// One of the files under `fixtures/python/`: what the interpreter itself answered when
/// `record.py` asked it, CPython 3.14 (`version.txt` beside them says which).
fn python_said(name: &str) -> String {
    let path = fixtures().join("python").join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// The lists `sort_cases` builds, one JSON list a line, as `record.py` reads them.
fn sort_cases_text() -> String {
    let mut input = String::new();
    for xs in &sort_cases() {
        let cells: Vec<String> = xs
            .iter()
            .map(|x| x.map_or("NaN".to_string(), |x| format!("{x:?}")))
            .collect();
        input.push_str(&format!("[{}]\n", cells.join(",")));
    }
    input
}

/// Build ticket 199, item 3: `queue` must leave NaN scores where `sorted` leaves them, which
/// only the same algorithm does. `sorted.txt` is `sorted(range(len(xs)), key=xs.__getitem__)`
/// for each list.
#[test]
fn the_sort_matches_python_sorted_on_keys_that_hold_nan() {
    let cases = sort_cases();
    let expected = python_said("sorted.txt");
    let lines: Vec<&str> = expected.lines().collect();
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

/// Build ticket 199, item 7: the table `repr` reads is `str.isprintable` as CPython 3.14
/// has it. `isprintable.txt` is every range of code points it calls not printable.
#[test]
fn the_printable_table_is_python_isprintable() {
    let python: Vec<(u32, u32)> = python_said("isprintable.txt")
        .lines()
        .map(|line| {
            let (low, high) = line.split_once(' ').unwrap();
            (low.parse().unwrap(), high.parse().unwrap())
        })
        .collect();
    assert_eq!(python, hydra_puppets::printable::NOT_PRINTABLE.to_vec());
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
/// among `files`, the files of the home it ran in. A writer that prints one id and creates
/// the log under another gets `None`.
fn new_goal_id<'a>(stdout: &'a str, files: &[(String, Vec<u8>)]) -> Option<&'a str> {
    let id = stdout.strip_suffix('\n').filter(|id| is_goal_id(id))?;
    let log = format!("puppets/{id}.jsonl");
    files.iter().any(|(name, _)| *name == log).then_some(id)
}

fn new_stdout_names_log(stdout: &str, files: &[(String, Vec<u8>)]) -> bool {
    new_goal_id(stdout, files).is_some()
}

/// `new` prints the goal id it generated and nothing else: mask that one line, when it
/// names the log `new` created.
fn mask_new_stdout(stdout: &str, files: &[(String, Vec<u8>)]) -> String {
    match new_goal_id(stdout, files) {
        Some(_) => "<goal>\n".to_string(),
        None => stdout.to_string(),
    }
}

/// The goal logs a write command left, from the files of its home: each file directly
/// under `puppets/`, by name, its root masked and then the stamps of its appended lines.
/// `seed` is the home the command started from, its root already masked. Under `new` the
/// log named by the goal id on stdout is listed as `<goal>.jsonl`, and no other log is
/// renamed.
fn written(
    files: &[(String, Vec<u8>)],
    seed: &[(String, Vec<u8>)],
    new_goal: Option<&str>,
    root: &Path,
) -> Vec<(String, Vec<u8>)> {
    let mut files: Vec<(String, Vec<u8>)> = files
        .iter()
        .filter_map(|(path, bytes)| {
            let name = path.strip_prefix("puppets/").filter(|n| !n.contains('/'))?;
            let bytes = mask_appended_ts(bytes_of(seed, path), &mask_root(bytes, root));
            let name = match name.strip_suffix(".jsonl") {
                Some(stem) if new_goal == Some(stem) => "<goal>.jsonl".to_string(),
                _ => name.to_string(),
            };
            Some((name, bytes))
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

/// Every file of `files` with `root` masked.
fn rooted(files: &[(String, Vec<u8>)], root: &Path) -> Vec<(String, Vec<u8>)> {
    files
        .iter()
        .map(|(name, bytes)| (name.clone(), mask_root(bytes, root)))
        .collect()
}

/// Seed one home per write case under `root` and hand each to `visit`.
fn visit_write(root: &Path, recording: bool, visit: Visit) {
    let shared = root.join("shared");
    std::fs::create_dir_all(&shared).unwrap();
    write_plans(&shared);
    review_repo(&shared.join("repo"));
    for (n, case) in write_cases(&shared).iter().enumerate() {
        let name = format!("w{n:03}");
        let home = root.join(&name).join(side(recording));
        std::fs::create_dir_all(home.join("puppets")).unwrap();
        if let Some(log) = &case.log {
            std::fs::write(home.join("puppets/g1.jsonl"), log).unwrap();
        }
        if let Some(vendors) = case.vendors {
            std::fs::write(home.join("vendors.json"), vendors).unwrap();
        }
        if !case.sheets.is_empty() {
            std::fs::create_dir_all(home.join("puppets/agents")).unwrap();
        }
        for (name, body) in &case.sheets {
            std::fs::write(home.join("puppets/agents").join(name), body).unwrap();
        }
        let mut job = Job::new(name, &case.label, vec![case.args.clone()]);
        job.cwd = Some(case.cwd.clone());
        visit(&job, &home);
    }
}

/// Run one write case and compare it with what the Python gave on the same starting home.
/// Returns the exception class when the Python raised, and what differed.
fn compare_write(job: &Job, home: &Path, root: &Path) -> (Option<String>, Vec<String>) {
    let rec = recorded("write", job, root);
    let seed = rooted(&kept(home, &["puppets"]), root);
    let ran = job.run(home, root).remove(0);
    let (py_left, rs_left) = (&rec.files, kept(home, &["puppets"]));
    let mut py = rec.steps[0].masked(&rec.home, &rec.root);
    let mut rs = ran.masked(home, root);
    let mut problems = Vec::new();
    // The id each printed is tied to the log it created before either is masked.
    let (mut py_goal, mut rs_goal) = (None, None);
    if job.steps[0][0] == "new" {
        for (who, outcome, left, goal) in [
            ("python", &mut py, py_left, &mut py_goal),
            ("rust", &mut rs, &rs_left, &mut rs_goal),
        ] {
            *goal = new_goal_id(&outcome.stdout, left).map(str::to_string);
            if outcome.code == 0 && goal.is_none() {
                problems.push(format!(
                    "{who} exited 0 and its stdout names no log it created: {:?}",
                    outcome.stdout
                ));
            }
            outcome.stdout = mask_new_stdout(&outcome.stdout, left);
        }
    }
    let (py_files, rs_files) = (
        written(py_left, &seed, py_goal.as_deref(), &rec.root),
        written(&rs_left, &seed, rs_goal.as_deref(), root),
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

    // `new` reads the repo and the commit from its working directory. The repository is the
    // one `review_repo` builds, so the commit it records is the same on every machine.
    for (label, cwd, text) in [
        ("new outside a repo", shared.to_path_buf(), "a goal"),
        ("new inside a repo", shared.join("repo/src"), "a goal"),
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

// Ticket 270 changes default cursor assignments and the security panel model.
// Keep the Python fixtures intact; pin the new tables in the Rust unit tests.
const DIVERGED_SINCE_270_WRITE: &[&str] = &[
    "five tasks: assign g1",
    "a ranking: assign",
    "sheets with no verdict behind them: assign",
    "an agents directory with no sheet: assign",
];
const DIVERGED_SINCE_270_LAUNCH: &[&str] = &[
    "review --no-open:",
    "review: Hunk is opened in place of the command",
    "review: inside Herdr the diff goes to a new pane",
];

#[test]
fn the_write_commands_match_puppets_py_and_leave_the_same_goal_log() {
    let scratch = Scratch::new("write");
    let root = &scratch.0;
    let mut labels = Vec::new();
    let mut crashes = Vec::new();
    let mut mismatches = Vec::new();
    visit_write(root, false, &mut |job, home| {
        if DIVERGED_SINCE_270_WRITE.contains(&job.label.as_str()) {
            eprintln!("not compared, changed by ticket 270: {}", job.label);
            labels.push(job.label.clone());
            return;
        }
        let (class, problems) = compare_write(job, home, root);
        crashes.extend(class.map(|class| format!("{}: {class}", job.label)));
        if !problems.is_empty() {
            mismatches.push(format!("{}\n{}", job.label, problems.join("\n")));
        }
        labels.push(job.label.clone());
    });
    assert_every_fixture_was_read("write", labels.len());
    eprintln!("compared {} write command runs:", labels.len());
    for label in &labels {
        eprintln!("  {label}");
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
        labels.len(),
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
    let id = "0b9f2c3e-6a1d-4c7b-9e2f-5d8a7b6c4e31";
    let seven = "0b9f2c3e-6a1d-7c7b-9e2f-5d8a7b6c4e31";
    let files: Vec<(String, Vec<u8>)> = [id, seven, "g1"]
        .iter()
        .map(|name| (format!("puppets/{name}.jsonl"), b"{}\n".to_vec()))
        .collect();
    assert!(is_goal_id(id));
    assert_eq!(mask_new_stdout(&format!("{id}\n"), &files), "<goal>\n");
    for odd in [
        "g1\n".to_string(),
        id.to_string(),
        format!("{id}\n{id}\n"),
        format!("goal {id}\n"),
        format!("{}\n", id.to_uppercase()),
        // Version 7, which `uuid.uuid4()` never prints.
        format!("{seven}\n"),
    ] {
        assert_eq!(mask_new_stdout(&odd, &files), odd);
    }
}

/// `LC_ALL`, `PYTHONUTF8` and `PYTHONIOENCODING` for one `board` of a goal whose text is a
/// lone surrogate.
const LOCALES: [(&str, Option<&str>, Option<&str>); 9] = [
    ("C", None, None),
    ("en_US.UTF-8", None, None),
    ("C", Some("0"), None),
    ("en_US.UTF-8", Some("1"), None),
    ("C", None, Some("utf-8")),
    ("C", None, Some("utf-8:strict")),
    ("en_US.UTF-8", None, Some(":surrogateescape")),
    ("C", None, Some("ascii:backslashreplace")),
    ("en_US.UTF-8", None, Some("latin-1:replace")),
];

fn visit_locale(root: &Path, recording: bool, visit: Visit) {
    for (n, (locale, utf8, encoding)) in LOCALES.iter().enumerate() {
        let name = format!("locale-{n}");
        let home = root.join(&name).join(side(recording));
        std::fs::create_dir_all(home.join("puppets")).unwrap();
        std::fs::write(
            home.join("puppets/g1.jsonl"),
            b"{\"event\":\"goal\",\"text\":\"\\udc80\",\"repo\":\"/r\",\"base_sha\":\"a\"}\n",
        )
        .unwrap();
        let label = format!("LC_ALL={locale} PYTHONUTF8={utf8:?} PYTHONIOENCODING={encoding:?}");
        let mut job = Job::new(name, &label, vec![strings(&["board", "g1"])]);
        job.env.push(("LC_ALL".into(), locale.to_string()));
        for (key, value) in [("PYTHONUTF8", utf8), ("PYTHONIOENCODING", encoding)] {
            if let Some(value) = value {
                job.env.push((key.into(), value.to_string()));
            }
        }
        job.keep = Vec::new();
        visit(&job, &home);
    }
}

#[test]
fn stdout_surrogate_matches_python_locales() {
    let scratch = Scratch::new("stdout-surrogate");
    let root = &scratch.0;
    let mut compared = 0;
    visit_locale(root, false, &mut |job, home| {
        let python = &recorded("locale", job, root).steps[0];
        let rust = &job.run(home, root)[0];
        assert_eq!(rust.stdout, python.stdout, "{}", job.label);
        assert_eq!(rust.code, python.code, "{}", job.label);
        compared += 1;
    });
    assert_every_fixture_was_read("locale", compared);
}

const TORN_LOG: &[u8] =
    b"{\"event\":\"goal\",\"text\":\"test\",\"repo\":\"/r\",\"base_sha\":\"a\"}\ntorn";

/// A home whose `g1.jsonl.repair-tmp` is a symlink to the log `repair --yes` would rewrite.
fn visit_symlink(root: &Path, recording: bool, visit: Visit) {
    let home = root.join("symlink").join(side(recording));
    std::fs::create_dir_all(home.join("puppets")).unwrap();
    let path = home.join("puppets/g1.jsonl");
    std::fs::write(&path, TORN_LOG).unwrap();
    std::os::unix::fs::symlink(&path, home.join("puppets/g1.jsonl.repair-tmp")).unwrap();
    let mut job = Job::new(
        "symlink".into(),
        "repair --yes over a temporary that is a symlink",
        vec![strings(&["repair", "g1", "--yes"])],
    );
    job.keep = Vec::new();
    visit(&job, &home);
}

#[test]
fn repair_temporary_symlink_matches_python() {
    let scratch = Scratch::new("repair-symlink");
    let root = &scratch.0;
    let mut compared = 0;
    visit_symlink(root, false, &mut |job, home| {
        let rec = recorded("symlink", job, root);
        let python = rec.steps[0].masked(&rec.home, &rec.root);
        let rust = job.run(home, root)[0].masked(home, root);
        let puppets = home.join("puppets");
        let tmp = puppets.join("g1.jsonl.repair-tmp");
        assert_eq!(rust.code, 1);
        assert_eq!(std::fs::read(puppets.join("g1.jsonl")).unwrap(), TORN_LOG);
        assert!(tmp.symlink_metadata().unwrap().file_type().is_symlink());
        assert_eq!(std::fs::read_dir(&puppets).unwrap().count(), 2);
        assert_eq!(python, rust);
        compared += 1;
    });
    assert_every_fixture_was_read("symlink", compared);
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

/// Replace every occurrence of the path `dir` with `with`.
fn mask_path(bytes: &[u8], dir: &Path, with: &[u8]) -> Vec<u8> {
    let needle = dir.to_str().unwrap().as_bytes();
    let mut out = Vec::new();
    let mut rest = bytes;
    while let Some(at) = rest.windows(needle.len()).position(|w| w == needle) {
        out.extend_from_slice(&rest[..at]);
        out.extend_from_slice(with);
        rest = &rest[at + needle.len()..];
    }
    out.extend_from_slice(rest);
    out
}

/// Replace every occurrence of the home a command ran in with `<home>`.
fn mask_home(bytes: &[u8], dir: &Path) -> Vec<u8> {
    mask_path(bytes, dir, b"<home>")
}

/// Replace every occurrence of the directory a group of cases lives under with `<root>`:
/// the recorder's in a fixture, a scratch one in a test. A home is under its root, so the
/// home is masked first.
fn mask_root(bytes: &[u8], dir: &Path) -> Vec<u8> {
    mask_path(bytes, dir, b"<root>")
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

/// What a launch command left: every file under `puppets/` and every stub call, with the
/// home and the root masked, and then the stamps of appended log lines. `seed` is the home
/// the command started from, masked the same way.
fn launched(
    files: &[(String, Vec<u8>)],
    seed: &[(String, Vec<u8>)],
    home: &Path,
    root: &Path,
) -> Vec<(String, Vec<u8>)> {
    files
        .iter()
        .map(|(name, bytes)| {
            let bytes = mask_root(&mask_home(bytes, home), root);
            let bytes = if name.ends_with(".jsonl") {
                mask_appended_ts(bytes_of(seed, name), &bytes)
            } else {
                bytes
            };
            (name.clone(), bytes)
        })
        .collect()
}

/// Write the stubs, the plans and the repository every launch case shares under `root`,
/// seed one home per case and hand each to `visit`.
fn visit_launch(root: &Path, recording: bool, visit: Visit) {
    use std::os::unix::fs::PermissionsExt;
    let (shared, bin) = (root.join("shared"), root.join("bin"));
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
    for (n, case) in launch_cases(&shared, &base_sha).iter().enumerate() {
        let name = format!("l{n:03}");
        let home = root.join(&name).join(side(recording));
        case.seed(&home);
        let mut job = Job::new(name, &case.label, vec![case.args.clone()]);
        job.cwd = Some(shared.clone());
        job.keep = vec!["puppets", "stub/calls"];
        let path = |p: PathBuf| p.to_str().unwrap().to_string();
        job.env = vec![
            // By name, found on `PATH`: `launch` hashes the argv it starts, and a path under
            // the root would make that hash the recorder's.
            ("HYDRA_BIN".into(), "hydra-stub".into()),
            ("STUB_STATE".into(), path(home.join("stub"))),
            // `review` ends in `execvp`, which does not flush Python's buffer when stdout is
            // a pipe. Unbuffered, it prints what it prints on a terminal.
            ("PYTHONUNBUFFERED".into(), "1".into()),
        ];
        for (key, value) in &case.env {
            job.env.push((key.to_string(), value.to_string()));
        }
        visit(&job, &home);
    }
}

/// Run one launch case and compare it, and the calls its stubs wrote down, with what the
/// Python gave.
fn compare_launch(job: &Job, home: &Path, root: &Path) -> (Option<String>, Vec<String>) {
    let rec = recorded("launch", job, root);
    let seed: Vec<(String, Vec<u8>)> = kept(home, &["puppets"])
        .into_iter()
        .map(|(name, bytes)| (name, mask_root(&mask_home(&bytes, home), root)))
        .collect();
    let ran = job.run(home, root).remove(0);
    let py = rec.steps[0].masked(&rec.home, &rec.root);
    let rs = ran.masked(home, root);
    let mut problems = Vec::new();
    let py_files = launched(&rec.files, &seed, &rec.home, &rec.root);
    let rs_files = launched(&kept(home, &job.keep), &seed, home, root);
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
/// touches a source file and a test. Returns the base sha, which is the same on every
/// machine: both commits carry a fixed time.
fn review_repo(repo: &Path) -> String {
    let git = |args: &[&str]| -> String {
        let out = Command::new("git")
            .current_dir(repo)
            .env_clear()
            .env("PATH", std::env::var("PATH").unwrap_or_default())
            .env("HOME", repo)
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_AUTHOR_DATE", "2026-10-01T00:00:00Z")
            .env("GIT_COMMITTER_DATE", "2026-10-01T00:00:00Z")
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

/// Build ticket 252 made `choose --judge` blind, in a different order per seat, with a
/// tie-break on a split, so its prompts and calls no longer match what `puppets.py` recorded.
/// Those cases still run through `visit_launch` so the fixture count holds; the new behaviour
/// is tested in `panel.rs`.
const DIVERGED_SINCE_252: &str = "choose --judge";

/// Build 262 appends distinct stance files and records stance in plan events.
const DIVERGED_SINCE_262: &str = "plan:";

#[test]
fn the_launch_commands_match_puppets_py_and_hand_the_stubs_the_same_calls() {
    let scratch = Scratch::new("launch");
    let root = &scratch.0;
    let mut labels = Vec::new();
    let mut crashes = Vec::new();
    let mut mismatches = Vec::new();
    let mut diverged = Vec::new();
    visit_launch(root, false, &mut |job, home| {
        if job.label.starts_with(DIVERGED_SINCE_252)
            || job.label.starts_with(DIVERGED_SINCE_262)
            || DIVERGED_SINCE_270_LAUNCH
                .iter()
                .any(|prefix| job.label.starts_with(prefix))
        {
            diverged.push(job.label.clone());
            labels.push(job.label.clone());
            return;
        }
        let (class, problems) = compare_launch(job, home, root);
        crashes.extend(class.map(|class| format!("{}: {class}", job.label)));
        if !problems.is_empty() {
            mismatches.push(format!("{}\n{}", job.label, problems.join("\n")));
        }
        labels.push(job.label.clone());
    });
    assert_every_fixture_was_read("launch", labels.len());
    eprintln!("compared {} launch command runs:", labels.len());
    for label in &labels {
        eprintln!("  {label}");
    }
    eprintln!(
        "not compared, changed on purpose by builds 252, 262 and 270 ({}):",
        diverged.len()
    );
    for label in &diverged {
        eprintln!("  {label}");
    }
    assert!(!diverged.is_empty(), "no `choose --judge` case was found");
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
        labels.len(),
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
    let printed = "0b9f2c3e-6a1d-4c7b-9e2f-5d8a7b6c4e31";
    let created = "1b9f2c3e-6a1d-4c7b-9e2f-5d8a7b6c4e31";
    let files = vec![(format!("puppets/{created}.jsonl"), b"{}\n".to_vec())];
    assert!(!new_stdout_names_log(&format!("{printed}\n"), &files));
    assert!(new_stdout_names_log(&format!("{created}\n"), &files));
    // The id that names no log stays on stdout, and the log it did not name keeps its name.
    let stdout = format!("{printed}\n");
    assert_eq!(mask_new_stdout(&stdout, &files), stdout);
    let goal = new_goal_id(&stdout, &files);
    let names = |goal: Option<&str>| -> Vec<String> {
        let files = written(&files, &[], goal, Path::new("/nowhere"));
        files.into_iter().map(|(name, _)| name).collect()
    };
    assert_eq!(names(goal), [format!("{created}.jsonl")]);
    assert_eq!(names(Some(created)), ["<goal>.jsonl"]);
}

/// Build ticket 207. `int()` under CPython 3.14, for every code point before a `7`, after
/// one and alone: which it strips, which are digits and of what value. `int.txt` holds a row
/// for each code point where any of the five reads gave a number. It is the check on the
/// digit table and the whitespace rule in `crates/pyjson/src/py.rs`.
#[test]
fn the_int_parser_matches_python_on_every_code_point() {
    let expected = python_said("int.txt");
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

/// Build ticket 207. `math.lgamma` and `sheet.beta_lower` under CPython 3.14, bit for bit:
/// `rank` orders agents by a quotient of the second, and a last bit swaps a near tie.
/// `lgamma.txt` is the first over `k`, `k / 2` and `k / 7` for `k` from 1 to 6000, and each
/// line of `beta_lower.txt` is two arguments and the second's answer.
#[test]
fn the_ranking_arithmetic_matches_python() {
    let bits = |f: f64| format!("{:016x}", f.to_bits());
    let expected = python_said("lgamma.txt");
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

    let expected = python_said("beta_lower.txt");
    assert_eq!(expected.lines().count(), 25 * 25 + 3);
    for line in expected.lines() {
        let cells: Vec<&str> = line.split(' ').collect();
        let (a, b): (i128, i128) = (cells[0].parse().unwrap(), cells[1].parse().unwrap());
        let ours = hydra_puppets::sheet::beta_lower(a, b).unwrap();
        assert_eq!(bits(ours), cells[2], "beta_lower({a}, {b})");
    }
}

/// Not a test of anything: it writes the jobs `record.py run` runs under `puppets.py`, one
/// directory per case under `HYDRA_PUPPETS_JOBS`, each the home a test here seeds plus a
/// `job.json`. `HYDRA_PUPPETS_HARVEST` names what `record.py harvest` wrote; without it the
/// harvested logs come from the fixtures. `record.py` says when this was run and how.
#[test]
#[ignore = "writes the jobs record.py runs, under HYDRA_PUPPETS_JOBS"]
fn write_the_jobs_record_py_runs() {
    let jobs = std::env::var_os("HYDRA_PUPPETS_JOBS").expect("HYDRA_PUPPETS_JOBS is not set");
    std::fs::create_dir_all(&jobs).unwrap();
    let jobs = Path::new(&jobs).canonicalize().unwrap();
    let harvest = std::env::var_os("HYDRA_PUPPETS_HARVEST").map(PathBuf::from);
    let group = |name: &str| {
        let root = jobs.join(name);
        std::fs::create_dir_all(&root).unwrap();
        prepare_root(&root);
        root
    };
    let root = group("read");
    let inputs = read_inputs(&root, harvest.as_deref());
    visit_read(&root, &inputs, true, &mut |job, home| {
        job.write(home, &root)
    });
    let root = group("write");
    visit_write(&root, true, &mut |job, home| job.write(home, &root));
    let root = group("launch");
    visit_launch(&root, true, &mut |job, home| job.write(home, &root));
    let root = group("locale");
    visit_locale(&root, true, &mut |job, home| job.write(home, &root));
    let root = group("symlink");
    visit_symlink(&root, true, &mut |job, home| job.write(home, &root));
    std::fs::create_dir_all(jobs.join("python")).unwrap();
    std::fs::write(jobs.join("python/sort-cases.jsonl"), sort_cases_text()).unwrap();
}
