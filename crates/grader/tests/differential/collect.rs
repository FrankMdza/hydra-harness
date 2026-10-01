//! `labels.rs`, `collect.rs` and `hydra-grader once`, `watch`, `rebuild` and `backfill`
//! against `labels.py` and `grader.py` (build ticket 214).
//!
//! `record_collect.py` wrote two fixtures under `collect/`.
//!
//! `labels.json` is `labels.py` function by function, on every goal log of
//! `crates/puppets/tests/fixtures/read/` with what `puppets.py show` printed for it, and on
//! answers written by hand. `hydra-puppets` is a stub the fixture carries: a shell script
//! that writes down its arguments and environment and prints the planted answer. The labels
//! are compared as the text `json.dumps` gives every pair in order, and a `LabelsError` by
//! its whole sentence.
//!
//! `scenarios.json` is `grader.py` command by command. A scenario is a scratch directory and
//! a list of steps; a step plants or removes files and runs one command from that directory
//! with nothing in the environment but what the step names. `hydra` is a stub too. This test
//! builds the same directory, runs the binary there, and compares the exit code, stdout,
//! stderr and every file that was not planted: `dataset.jsonl`, `cursor.json`,
//! `show-cache.json`, `scores.jsonl`, a `.tmp` file a refused write left, and what each stub
//! wrote down of its arguments and environment.
//!
//! Normalized, and only these:
//! - `collected_at` and `scored_at` are wall clock. The recorder froze the Python's clock at
//!   a made-up second of 2001, a different one in every step. Here each step's new stamps
//!   must lie between the moments the command started and ended, and are then rewritten in
//!   the scratch files to the recorder's second, so the next step reads the bytes the Python
//!   read and a row still says which step wrote it.
//! - The scratch directory's path is `<root>` wherever it is printed or passed.
//! - Where the Python ended in a traceback, the Rust's last line of stderr must start with
//!   the same exception class. What stderr held before the traceback is compared whole, and
//!   so is the sentence of a `LabelsError`.
//! - On a wrong command line, exit code 2, only the last line of stderr is compared, and
//!   the Python's `grader.py` at its start is read as `hydra-grader`.
//! - Where the Python printed its help, the Rust must exit 0 and print a usage line.
//! - One scenario calls `collect()` with a runs directory that is not named `runs`, which no
//!   command can do. It is called in this process, and the scratch directory `hydra` was
//!   handed as `HYDRA_ROOT` is `<tmp>`.
//! - The stubs leave `PWD`, `SHLVL` and `_` out of what they write down, since the shell
//!   that runs a stub adds them.
//!
//! Every command was recorded with one worker, or with one stale record. With more, the
//! Python's cache fills in the order the answers arrive.

use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::os::unix::fs::PermissionsExt;
use std::process::Stdio;

use hydra_grader::collect::{self, Pass};
use hydra_grader::labels::{self, Env, Keyed, LabelsError};

use super::*;

const ROOT: &str = "<root>";

fn has(value: &Value, key: &str) -> bool {
    matches!(value, Value::Dict(dict) if dict.get(key).is_some())
}

fn texts(value: &Value) -> BTreeMap<String, String> {
    let Value::Dict(dict) = value else {
        panic!("an object of text");
    };
    dict.iter()
        .map(|(key, value)| (key.lossy(), string(value)))
        .collect()
}

/// The model files build ticket 213 recorded, by the name of the case that trained each.
fn models() -> BTreeMap<String, String> {
    let mut models = BTreeMap::new();
    for case in items(&fixture("baseline.json")) {
        for file in items(field(case, "written")) {
            if string(field(file, "path")).ends_with(".json") {
                models.insert(string(field(case, "name")), string(field(file, "text")));
            }
        }
    }
    models
}

/// What a fixture plants: the two stubs and the models.
struct Planted {
    stubs: BTreeMap<String, String>,
    models: BTreeMap<String, String>,
}

impl Planted {
    /// Write one file of a case under `root` and return its path and bytes.
    fn plant(&self, root: &Path, spec: &Value) -> (String, Vec<u8>) {
        let path = string(field(spec, "path"));
        let data = if has(spec, "stub") {
            self.stubs[&string(field(spec, "stub"))]
                .clone()
                .into_bytes()
        } else if has(spec, "model") {
            self.models[&string(field(spec, "model"))]
                .clone()
                .into_bytes()
        } else if has(spec, "aliases") {
            std::fs::read(fixtures().join("data/aliases.json")).unwrap()
        } else if has(spec, "text") {
            string(field(spec, "text")).into_bytes()
        } else {
            unhex(&string(field(spec, "hex")))
        };
        let target = root.join(&path);
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        std::fs::write(&target, &data).unwrap();
        let mode = if has(spec, "exec") { 0o755 } else { 0o644 };
        std::fs::set_permissions(&target, std::fs::Permissions::from_mode(mode)).unwrap();
        (path, data)
    }
}

fn root_text(root: &Path) -> String {
    root.display().to_string()
}

/// The environment of a case, with the scratch directory where the recorder wrote `<root>`.
fn environment(case: &Value, root: &Path) -> Vec<(String, String)> {
    texts(field(case, "env"))
        .into_iter()
        .map(|(key, value)| (key, value.replace(ROOT, &root_text(root))))
        .collect()
}

fn plain(text: &str, root: &Path) -> String {
    text.replace(&root_text(root), ROOT)
}

fn read_plain(path: &Path, root: &Path) -> Option<String> {
    std::fs::read(path)
        .ok()
        .map(|data| plain(&String::from_utf8_lossy(&data), root))
}

/// What a call of `labels.py` gave, as the recorder wrote it down: the pairs as text, or
/// the class raised, with the sentence of a `LabelsError`.
type Gave = (Option<String>, Option<String>, Option<String>);

fn gave(got: anyhow::Result<Keyed>, root: &Path) -> Gave {
    match got {
        Ok(found) => {
            let pairs = found
                .iter()
                .map(|(run_id, row)| Value::List(vec![run_id.clone(), row.clone()]))
                .collect();
            (Some(json::dumps_default(&Value::List(pairs))), None, None)
        }
        Err(err) => match err.downcast_ref::<LabelsError>() {
            Some(labels) => (
                None,
                Some("LabelsError".to_string()),
                Some(plain(&labels.0, root)),
            ),
            None => (None, Some(class(&err)), None),
        },
    }
}

fn recorded(result: &Value) -> Gave {
    let of = |key: &str| has(result, key).then(|| string(field(result, key)));
    (of("pairs"), of("raises"), of("message"))
}

#[test]
fn the_labels_are_what_the_python_gave() {
    // One recorded goal log holds an event nested 3,000 deep, and reading it recurses once
    // per level, so this runs on the stack the binary gives its command.
    std::thread::Builder::new()
        .stack_size(collect::STACK_BYTES)
        .spawn(compare_the_labels)
        .unwrap()
        .join()
        .unwrap();
}

fn compare_the_labels() {
    let fixture = fixture("collect/labels.json");
    let planted = Planted {
        stubs: texts(field(&fixture, "stubs")),
        models: BTreeMap::new(),
    };
    let scratch = Scratch::new("labels");
    let mut tally = Tally::new();
    let (mut labelled, mut refused) = (0, 0);

    // `show_goal`, `committed_events` and `labels_for_goal`, a goal at a time.
    for (n, case) in items(field(&fixture, "shows")).iter().enumerate() {
        let name = string(field(case, "name"));
        let root = scratch.0.join(format!("s{n}"));
        std::fs::create_dir_all(&root).unwrap();
        for spec in items(field(case, "files")) {
            planted.plant(&root, spec);
        }
        let env = Env::from_pairs(environment(case, &root));
        let home = root.join("puppets");
        let goal = string(field(case, "goal"));
        let got = labels::show_goal(OsStr::new(&goal), &home, &env).and_then(|shown| {
            let events = labels::committed_events(&home.join(format!("{goal}.jsonl")))?;
            labels::labels_for_goal(&shown, &events)
        });
        let got = gave(got, &root);
        let result = field(case, "result");
        match (&got.0, got.1.as_deref()) {
            (Some(pairs), _) => labelled += pairs.matches("\"task_id\"").count(),
            (None, Some("LabelsError")) => refused += 1,
            _ => {}
        }
        tally.check(&format!("{name}: the labels"), got, recorded(result));
        tally.check(
            &format!("{name}: what the helper was called with"),
            read_plain(&root.join("bin/puppets-calls"), &root),
            optional(field(result, "calls")),
        );
    }

    // `collect`: every goal log of a home.
    for (n, case) in items(field(&fixture, "homes")).iter().enumerate() {
        let name = string(field(case, "name"));
        let root = scratch.0.join(format!("h{n}"));
        std::fs::create_dir_all(&root).unwrap();
        for spec in items(field(case, "files")) {
            planted.plant(&root, spec);
        }
        let env = Env::from_pairs(environment(case, &root));
        let mut stderr = Vec::new();
        let got =
            labels::puppets_home(&env).and_then(|home| labels::collect(&home, &env, &mut stderr));
        let result = field(case, "result");
        tally.check(
            &format!("{name}: the labels"),
            gave(got, &root),
            recorded(result),
        );
        tally.check(
            &format!("{name}: stderr"),
            plain(&String::from_utf8(stderr).unwrap(), &root),
            string(field(result, "stderr")),
        );
        tally.check(
            &format!("{name}: what the helper was called with"),
            read_plain(&root.join("bin/puppets-calls"), &root),
            optional(field(result, "calls")),
        );
    }

    // `collect_llm`: the class files a model wrote.
    for (n, case) in items(field(&fixture, "llm")).iter().enumerate() {
        let name = string(field(case, "name"));
        let root = scratch.0.join(format!("l{n}"));
        std::fs::create_dir_all(&root).unwrap();
        for spec in items(field(case, "files")) {
            planted.plant(&root, spec);
        }
        let got = labels::collect_llm(&root.join("home"));
        tally.check(
            &format!("{name}: the classes"),
            gave(got, &root),
            recorded(field(case, "result")),
        );
    }

    // Where the helper and the puppets home are, by environment.
    for case in items(field(&fixture, "places")) {
        let label = py::repr(field(case, "env"));
        let env = Env::from_pairs(texts(field(case, "env")));
        let by_components = |path: PathBuf| path.components().collect::<PathBuf>();
        tally.check(
            &format!("puppets_home under {label}"),
            by_components(labels::puppets_home(&env).unwrap()),
            PathBuf::from(string(field(case, "puppets_home"))),
        );
        let helper = field(case, "helper");
        let got = labels::helper_bin(&env);
        if has(helper, "checkout") {
            // With nothing configured it is the release build of the checkout, and the
            // checkout is wherever the recorder ran, or this test.
            tally.check(
                &format!("helper_bin under {label}"),
                got.is_ok_and(|path| {
                    path.is_absolute() && path.ends_with("target/release/hydra-puppets")
                }),
                true,
            );
        } else if has(helper, "path") {
            tally.check(
                &format!("helper_bin under {label}"),
                got.ok(),
                Some(PathBuf::from(string(field(helper, "path")))),
            );
        } else {
            let message = got
                .err()
                .and_then(|err| err.downcast_ref::<LabelsError>().map(|err| err.0.clone()));
            tally.check(
                &format!("helper_bin under {label}"),
                message,
                Some(string(field(helper, "message"))),
            );
        }
    }

    println!("{labelled} labels read, {refused} goals refused");
    assert!(labelled > 250, "{labelled} labels");
    assert!(refused > 40, "{refused} refusals");
    tally.finish();
}

const TRACEBACK: &str = "Traceback (most recent call last):\n";

/// The line of a traceback that names the exception. Since 3.11 notes may follow it, and
/// `json.dumps` adds one for each object it was inside of.
fn exception_line(rest: &str) -> &str {
    rest.trim_end_matches('\n')
        .rsplit('\n')
        .find(|line| !line.starts_with("when serializing "))
        .unwrap_or("")
}

/// What of a stderr is compared: all of it, or with a traceback what came before it, the
/// exception class its last line names without the module in front, and for a `LabelsError`
/// its sentence.
fn told(stderr: &str, traceback: bool) -> String {
    if !traceback {
        return stderr.to_string();
    }
    let (before, last) = match stderr.split_once(TRACEBACK) {
        Some((before, rest)) => (before.to_string(), exception_line(rest)),
        // The Rust's: every line but the last came before.
        None => {
            let body = stderr.trim_end_matches('\n');
            match body.rsplit_once('\n') {
                Some((before, last)) => (format!("{before}\n"), last),
                None => (String::new(), body),
            }
        }
    };
    let (class, sentence) = last.split_once(':').unwrap_or((last, ""));
    let class = class.rsplit('.').next().unwrap_or("");
    if class == "LabelsError" {
        format!("{before}{class}:{sentence}")
    } else {
        format!("{before}{class}")
    }
}

fn last_line(stderr: &str) -> String {
    let line = stderr
        .trim_end_matches('\n')
        .rsplit('\n')
        .next()
        .unwrap_or("");
    match line.strip_prefix("grader.py") {
        Some(rest) => format!("hydra-grader{rest}"),
        None => line.to_string(),
    }
}

fn walk(dir: &Path, found: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            walk(&path, found);
        } else {
            found.push(path);
        }
    }
}

/// Rewrite the stamps this step wrote to the second the recorder froze its clock at, and
/// return how many there were. A stamp is this step's when it lies between `before` and
/// `after`; one from an earlier step is already a second of 2001.
fn restamp(root: &Path, now: &str, before: &str, after: &str) -> usize {
    let mut files = Vec::new();
    walk(root, &mut files);
    let mut count = 0;
    for path in files {
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        if !name.starts_with("dataset.jsonl") && !name.starts_with("scores.jsonl") {
            continue;
        }
        let Ok(mut text) = String::from_utf8(std::fs::read(&path).unwrap()) else {
            continue;
        };
        let mut changed = false;
        for key in ["\"collected_at\": \"", "\"scored_at\": \""] {
            let mut from = 0;
            while let Some(at) = text[from..].find(key) {
                let start = from + at + key.len();
                from = start;
                let Some(stamp) = text.get(start..start + 20) else {
                    break;
                };
                if stamp >= before && stamp <= after && stamp.ends_with('Z') {
                    text.replace_range(start..start + 20, now);
                    changed = true;
                    count += 1;
                }
            }
        }
        if changed {
            std::fs::write(&path, text).unwrap();
        }
    }
    count
}

/// Every file under `root` that is not one of `given` unchanged, by path.
fn left_behind(
    root: &Path,
    given: &BTreeMap<String, Vec<u8>>,
    lib: bool,
) -> BTreeMap<String, String> {
    let mut files = Vec::new();
    walk(root, &mut files);
    let mut found = BTreeMap::new();
    for path in files {
        let relative = path.strip_prefix(root).unwrap().display().to_string();
        let data = std::fs::read(&path).unwrap();
        if given.get(&relative) == Some(&data) {
            continue;
        }
        let mut text = plain(&String::from_utf8_lossy(&data), root);
        if lib {
            // The scratch directory `hydra` was handed, which has a new name every time.
            text = text
                .split_inclusive('\n')
                .map(|line| {
                    if line.starts_with("env: HYDRA_ROOT=") && !line.contains(ROOT) {
                        "env: HYDRA_ROOT=<tmp>\n".to_string()
                    } else {
                        line.to_string()
                    }
                })
                .collect();
        }
        found.insert(relative, text);
    }
    found
}

/// One step, run: the exit code, stdout and stderr.
type Ran = (Option<i128>, String, String);

/// `collect()` called in this process, as the recorder's child called the Python's.
fn call_collect(lib: &Value, env: &[(String, String)], root: &Path) -> Ran {
    let at = |key: &str| string(field(lib, key)).replace(ROOT, &root_text(root));
    let Value::Int(jobs) = field(lib, "jobs") else {
        panic!("a number of jobs");
    };
    let pass = Pass {
        runs: Some(PathBuf::from(at("runs"))),
        home: Some(PathBuf::from(at("home"))),
        hydra: Some(at("hydra")),
        jobs: *jobs as usize,
        rebuild: matches!(field(lib, "rebuild"), Value::Bool(true)),
    };
    let env = Env::from_pairs(env.iter().cloned());
    let mut stderr = Vec::new();
    let done = collect::collect(&pass, &env, &|| Ok(aliases()), &mut stderr);
    let mut stderr = String::from_utf8(stderr).unwrap();
    match done {
        Ok(count) => (Some(0), format!("{count}\n"), stderr),
        Err(err) => {
            stderr.push_str(&format!("{err}\n"));
            (Some(1), String::new(), stderr)
        }
    }
}

#[test]
fn collect_writes_what_the_python_wrote() {
    let fixture = fixture("collect/scenarios.json");
    let planted = Planted {
        stubs: texts(field(&fixture, "stubs")),
        models: models(),
    };
    let scratch = Scratch::new("collect");
    let mut tally = Tally::new();
    let (mut steps, mut stamps, mut passes) = (0, 0, 0);
    for (n, scenario) in items(field(&fixture, "scenarios")).iter().enumerate() {
        let scenario_name = string(field(scenario, "name"));
        let root = scratch.0.join(format!("c{n}"));
        std::fs::create_dir_all(&root).unwrap();
        let mut given = BTreeMap::new();
        for (k, step) in items(field(scenario, "steps")).iter().enumerate() {
            steps += 1;
            for spec in items(field(step, "puts")) {
                let (path, data) = planted.plant(&root, spec);
                given.insert(path, data);
            }
            for path in items(field(step, "removes")) {
                let path = string(path);
                std::fs::remove_file(root.join(&path)).unwrap();
                given.remove(&path);
            }
            let argv: Vec<String> = items(field(step, "argv")).iter().map(string).collect();
            let name = format!("{scenario_name}, step {k} ({})", argv.join(" "));
            let env = environment(step, &root);
            let lib = field(step, "lib");
            let is_lib = !matches!(lib, Value::Null);

            let before = collect::utc_now();
            let (code, stdout, stderr) = if is_lib {
                call_collect(lib, &env, &root)
            } else {
                let ran = Command::new(RUST)
                    .args(&argv)
                    .current_dir(&root)
                    .env_clear()
                    .envs(env.iter().cloned())
                    .stdin(Stdio::null())
                    .output()
                    .unwrap();
                (
                    ran.status.code().map(i128::from),
                    String::from_utf8(ran.stdout).unwrap(),
                    String::from_utf8(ran.stderr).unwrap(),
                )
            };
            let after = collect::utc_now();
            stamps += restamp(&root, &string(field(step, "now")), &before, &after);
            let (stdout, stderr) = (plain(&stdout, &root), plain(&stderr, &root));

            let Value::Int(exit) = field(step, "exit") else {
                panic!("an exit code");
            };
            let python_out = string(field(step, "stdout"));
            let python_err = string(field(step, "stderr"));
            tally.check(&format!("{name}: exit code"), code, Some(*exit));
            if *exit == 0 && python_out.starts_with("usage: ") {
                tally.check(
                    &format!("{name}: prints a usage line"),
                    stdout.starts_with("usage: hydra-grader"),
                    true,
                );
            } else {
                if *exit == 0 && python_out != "0\n" {
                    passes += 1;
                }
                tally.check(&format!("{name}: stdout"), stdout, python_out);
            }
            if *exit == 2 {
                tally.check(
                    &format!("{name}: the error line"),
                    last_line(&stderr),
                    last_line(&python_err),
                );
            } else {
                let traceback = python_err.contains(TRACEBACK);
                tally.check(
                    &format!("{name}: stderr"),
                    told(&stderr, traceback),
                    told(&python_err, traceback),
                );
            }
            let python: BTreeMap<String, String> = items(field(step, "files"))
                .iter()
                .map(|file| (string(field(file, "path")), string(field(file, "text"))))
                .collect();
            let rust = left_behind(&root, &given, is_lib);
            let paths = |files: &BTreeMap<String, String>| -> Vec<String> {
                files.keys().cloned().collect()
            };
            tally.check(
                &format!("{name}: which files are left"),
                paths(&rust),
                paths(&python),
            );
            for (path, text) in &python {
                if let Some(ours) = rust.get(path) {
                    tally.check(&format!("{name}: {path}"), ours.clone(), text.clone());
                }
            }
            // What the stubs wrote down is one step's.
            for calls in ["bin/calls", "bin/puppets-calls"] {
                let _ = std::fs::remove_file(root.join(calls));
            }
        }
    }
    println!("{steps} commands, {passes} of them wrote rows or scores, {stamps} stamps");
    assert!(steps > 300, "{steps} steps");
    assert!(passes > 80, "{passes} passes");
    assert!(stamps > 300, "{stamps} stamps");
    tally.finish();
}
