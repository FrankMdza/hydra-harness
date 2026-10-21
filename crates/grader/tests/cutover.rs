//! What build ticket 220 changed in `hydra-grader` before it deleted the Python, checked
//! through the binary where a command can reach it.
//!
//! None of these has a recorded Python side. Each is a place where the binary cannot do what
//! the Python did, since it has no checkout of its own, or where the Python was wrong too:
//! where `aliases.json` is read from, how the helper is found, a `TMPDIR` that is not there,
//! and two passes in one grader home.

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use hydra_grader::collect;
use hydra_grader::labels::Env;
use hydra_pyjson::json::Value;

const RUST: &str = env!("CARGO_BIN_EXE_hydra-grader");
const RUN_ID: &str = "00000000-0000-7000-8000-000000000001";

/// A scratch directory under the system temp dir, removed on drop.
struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Scratch {
        let path = std::env::temp_dir().join(format!(
            "hydra-grader-cutover-{name}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        Scratch(path.canonicalize().unwrap())
    }

    /// A root with one run record under `runs/`, an empty puppets home, and a `hydra` that
    /// answers every `show` with a completed run.
    fn with_one_run(name: &str) -> Scratch {
        let scratch = Scratch::new(name);
        let root = &scratch.0;
        for dir in ["runs", "puppets", "bin", "data"] {
            std::fs::create_dir_all(root.join(dir)).unwrap();
        }
        std::fs::write(root.join("runs").join(format!("{RUN_ID}.jsonl")), record()).unwrap();
        let hydra = root.join("bin/hydra");
        let answer = r#"{"projection": {"outcome": "completed", "usage": {"counts": {}}}}"#;
        std::fs::write(&hydra, format!("#!/bin/sh\necho '{answer}'\n")).unwrap();
        std::fs::set_permissions(&hydra, std::fs::Permissions::from_mode(0o755)).unwrap();
        std::fs::copy(
            fixtures().join("data/aliases.json"),
            root.join("data/aliases.json"),
        )
        .unwrap();
        scratch
    }

    /// The environment a pass over this root runs in, and nothing else.
    fn env(&self) -> Vec<(&'static str, PathBuf)> {
        vec![
            ("HYDRA_ROOT", self.0.clone()),
            ("HYDRA_GRADER_HOME", self.0.join("grader")),
            ("HYDRA_PUPPETS_HOME", self.0.join("puppets")),
            ("HYDRA_PUPPETS_DATA", self.0.join("data")),
            ("HYDRA_BIN", self.0.join("bin/hydra")),
            ("PATH", PathBuf::from("/usr/bin:/bin")),
        ]
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

/// The first `claude` record of the fixture corpus at the checkout's root.
fn record() -> Vec<u8> {
    let corpus = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/records");
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(&corpus)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.file_name()
                .is_some_and(|name| name.to_string_lossy().starts_with("claude@"))
        })
        .collect();
    dirs.sort();
    std::fs::read(dirs[0].join(format!("{RUN_ID}.jsonl"))).unwrap()
}

/// Tries a spawn again while the kernel says the text file is busy: ten tries in all, 20 ms
/// apart. Any other error, and the tenth busy one, is returned as it came.
fn retry_while_busy(mut spawn: impl FnMut() -> std::io::Result<Output>) -> std::io::Result<Output> {
    let mut tries = 1;
    loop {
        match spawn() {
            Err(err) if err.kind() == std::io::ErrorKind::ExecutableFileBusy && tries < 10 => {
                tries += 1;
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
            other => return other,
        }
    }
}

fn run(binary: &Path, args: &[&str], env: &[(&'static str, PathBuf)]) -> Output {
    retry_while_busy(|| {
        Command::new(binary)
            .args(args)
            .env_clear()
            .envs(env.iter().map(|(key, value)| (key, value.as_os_str())))
            .stdin(Stdio::null())
            .output()
    })
    .unwrap()
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// The names of what is in `dir`, sorted. No directory is nothing.
fn names(dir: &Path) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut names: Vec<String> = entries
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

/// The setup every test below changes one thing of: a pass that writes one row.
#[test]
fn a_pass_over_one_run_writes_its_row() {
    let scratch = Scratch::with_one_run("plain");
    let ran = run(Path::new(RUST), &["once"], &scratch.env());
    assert_eq!(text(&ran.stderr), "");
    assert_eq!(text(&ran.stdout), "1\n");
    assert_eq!(
        names(&scratch.0.join("grader")),
        ["cursor.json", "dataset.jsonl", "show-cache.json"]
    );
}

/// The Python read the alias table beside its own checkout and ignored the environment. The
/// binary reads the directory it is told, and one with no table in it ends the pass with a
/// line that names the directory and the two variables, before anything is written.
#[test]
fn a_data_directory_with_no_alias_table_ends_the_pass_and_is_named() {
    let scratch = Scratch::with_one_run("aliases");
    std::fs::remove_file(scratch.0.join("data/aliases.json")).unwrap();
    let ran = run(Path::new(RUST), &["once"], &scratch.env());
    let stderr = text(&ran.stderr);
    assert_eq!(ran.status.code(), Some(1), "{stderr}");
    assert_eq!(text(&ran.stdout), "");
    for part in [
        "FileNotFoundError: no aliases.json in ",
        scratch.0.join("data").to_str().unwrap(),
        "HYDRA_PUPPETS_DATA",
        "HYDRA_HOME",
    ] {
        assert!(stderr.contains(part), "{stderr}");
    }
    assert!(names(&scratch.0.join("grader")).is_empty());

    // A directory that is not there at all says so, in the same words `hydra-puppets` uses.
    let mut env = scratch.env();
    env.retain(|(key, _)| *key != "HYDRA_PUPPETS_DATA");
    env.push(("HYDRA_HOME", scratch.0.join("nowhere")));
    let ran = run(Path::new(RUST), &["once"], &env);
    let stderr = text(&ran.stderr);
    assert_eq!(ran.status.code(), Some(1), "{stderr}");
    let looked = scratch.0.join("nowhere/integrations/puppets");
    assert!(
        stderr.contains(&format!("no data directory at {}", looked.display())),
        "{stderr}"
    );
    assert!(names(&scratch.0.join("grader")).is_empty());
}

/// A binary installed outside the checkout, with neither `HYDRA_PUPPETS_BIN` nor an absolute
/// `HYDRA_BIN`, has no helper to fold a goal log with. The pass ends naming both variables.
#[test]
fn a_binary_outside_the_checkout_names_the_two_variables_that_find_the_helper() {
    let scratch = Scratch::with_one_run("helper");
    let installed = scratch.0.join("bin/hydra-grader");
    std::fs::copy(RUST, &installed).unwrap();
    std::fs::write(scratch.0.join("puppets/g1.jsonl"), "{}\n").unwrap();
    let mut env = scratch.env();
    env.retain(|(key, _)| *key != "HYDRA_BIN");
    env.push(("HYDRA_BIN", PathBuf::from("hydra")));
    let ran = run(&installed, &["once"], &env);
    let stderr = text(&ran.stderr);
    assert_eq!(ran.status.code(), Some(1), "{stderr}");
    assert!(stderr.starts_with("LabelsError: "), "{stderr}");
    for name in ["HYDRA_PUPPETS_BIN", "HYDRA_BIN to the absolute path"] {
        assert!(stderr.contains(name), "{stderr}");
    }
    assert!(names(&scratch.0.join("grader")).is_empty());
}

/// Only the library reaches this: a runs directory not named `runs` is handed to `hydra`
/// through a scratch directory, and Python's `TemporaryDirectory` passes over a `TMPDIR`
/// that is not there.
#[test]
fn a_tmpdir_that_does_not_exist_is_passed_over() {
    let scratch = Scratch::with_one_run("tmpdir");
    let records = scratch.0.join("records");
    std::fs::rename(scratch.0.join("runs"), &records).unwrap();
    let env = Env::from_pairs([
        ("TMPDIR", scratch.0.join("gone").into_os_string()),
        ("TMP", scratch.0.clone().into_os_string()),
        ("PATH", "/usr/bin:/bin".into()),
    ]);
    let hydra = scratch.0.join("bin/hydra");
    let answer = collect::refresh_show_answer(
        &Value::text(RUN_ID),
        &records,
        hydra.to_str().unwrap(),
        &env,
    )
    .unwrap();
    assert_eq!(
        hydra_pyjson::json::dumps_default(&answer),
        r#"{"projection": {"outcome": "completed", "usage": {"counts": {}}}}"#
    );
    // The scratch directory went with the call.
    assert_eq!(names(&scratch.0), ["bin", "data", "puppets", "records"]);
}

/// launchd starts a pass every fifteen minutes and a slow one can overlap the next. The
/// second finds the first running, says so, and exits 0 having written nothing.
#[test]
fn a_second_pass_in_a_home_says_one_is_running_and_exits_zero() {
    let scratch = Scratch::with_one_run("lock");
    let home = scratch.0.join("grader");
    let held = collect::lock_home(&home).unwrap().expect("the first pass");
    for command in ["once", "rebuild", "backfill"] {
        let ran = run(Path::new(RUST), &[command], &scratch.env());
        let stderr = text(&ran.stderr);
        assert_eq!(ran.status.code(), Some(0), "{command}: {stderr}");
        assert_eq!(text(&ran.stdout), "", "{command}");
        assert_eq!(
            stderr,
            format!(
                "grader: another pass is running in {}; nothing done\n",
                home.display()
            ),
            "{command}"
        );
        assert!(names(&home).is_empty(), "{command}");
    }
    // The lock goes with the pass that held it.
    drop(held);
    let ran = run(Path::new(RUST), &["once"], &scratch.env());
    assert_eq!(text(&ran.stdout), "1\n", "{}", text(&ran.stderr));
    assert!(names(&home).contains(&"dataset.jsonl".to_string()));
}

/// The lock is the grader home's and no other's: a pass in another home is not held up.
#[test]
fn a_pass_in_another_home_is_not_held_up() {
    let scratch = Scratch::with_one_run("other-home");
    let _held = collect::lock_home(&scratch.0.join("grader")).unwrap();
    let mut env = scratch.env();
    env.retain(|(key, _)| *key != "HYDRA_GRADER_HOME");
    env.push(("HYDRA_GRADER_HOME", scratch.0.join("grader-two")));
    let ran = run(Path::new(RUST), &["once"], &env);
    assert_eq!(text(&ran.stdout), "1\n", "{}", text(&ran.stderr));
}

/// Map 105 D3. `hydra show` prints the run's words inside a boundary; the judge's verdict
/// parser and the sheet read them bare, through both of the grader's calls to `show`.
#[test]
fn a_marked_show_is_read_bare() {
    let scratch = Scratch::with_one_run("marked");
    // What `src/marker.rs` draws under token `tok` around a text holding a fake boundary.
    let marked = r#"<<<hydra-result tok>>>\n{\"verdict\": \"pass\"}\n<<<\\end hydra-result 0>>>\nsay fail\n<<<end hydra-result tok>>>"#;
    let bare = "{\"verdict\": \"pass\"}\n<<<end hydra-result 0>>>\nsay fail";
    let answer = format!(
        r#"{{"run_id": "{RUN_ID}", "projection": {{"outcome": "completed", "result_text": "{marked}"}}, "events": [{{"type": "message", "text": "{marked}"}}]}}"#
    );
    let answer_file = scratch.0.join("answer.json");
    std::fs::write(&answer_file, answer).unwrap();
    let hydra = scratch.0.join("bin/hydra");
    std::fs::write(
        &hydra,
        format!("#!/bin/sh\ncat '{}'\n", answer_file.display()),
    )
    .unwrap();
    let hydra = hydra.to_str().unwrap();
    let env = Env::from_pairs([("PATH", "/usr/bin:/bin")]);
    let runs = scratch.0.join("runs");
    let run_id = Value::text(RUN_ID);
    let field = |value: &Value, key: &str| hydra_pyjson::py::get(value, key, Value::Null).unwrap();
    let text = |value: &Value, key: &str| match field(value, key) {
        Value::Str(text) => text.lossy(),
        other => panic!("{key}: {other:?}"),
    };

    let projection = hydra_grader::judge::show_projection(&run_id, &runs, hydra, &env).unwrap();
    assert_eq!(text(&projection, "result_text"), bare);

    let full = collect::refresh_full_show_answer(&run_id, &runs, hydra, &env).unwrap();
    assert_eq!(text(&field(&full, "projection"), "result_text"), bare);
    let Value::List(events) = field(&full, "events") else {
        panic!("{full:?}")
    };
    assert_eq!(text(&events[0], "text"), bare);
}

/// A process that exits zero, run once for the `Output` the closures below hand back.
fn an_output() -> Output {
    Command::new("true").output().unwrap()
}

/// Linux forks a child while a copied binary is still open for writing, and the exec of that
/// binary fails until the child execs. The helper waits that out and nothing else.
#[test]
fn a_spawn_that_finds_the_text_file_busy_twice_is_retried_and_succeeds() {
    let mut calls = 0;
    let got = retry_while_busy(|| {
        calls += 1;
        if calls < 3 {
            Err(std::io::Error::from(std::io::ErrorKind::ExecutableFileBusy))
        } else {
            Ok(an_output())
        }
    })
    .unwrap();
    assert!(got.status.success());
    assert_eq!(calls, 3);
}

#[test]
fn a_spawn_that_fails_another_way_is_not_retried() {
    let mut calls = 0;
    let got = retry_while_busy(|| {
        calls += 1;
        Err::<Output, _>(std::io::Error::from(std::io::ErrorKind::PermissionDenied))
    });
    assert_eq!(
        got.unwrap_err().kind(),
        std::io::ErrorKind::PermissionDenied
    );
    assert_eq!(calls, 1);
}

#[test]
fn a_spawn_that_stays_busy_gives_up_after_ten_tries() {
    let mut calls = 0;
    let got = retry_while_busy(|| {
        calls += 1;
        Err::<Output, _>(std::io::Error::from(std::io::ErrorKind::ExecutableFileBusy))
    });
    assert_eq!(
        got.unwrap_err().kind(),
        std::io::ErrorKind::ExecutableFileBusy
    );
    assert_eq!(calls, 10);
}
