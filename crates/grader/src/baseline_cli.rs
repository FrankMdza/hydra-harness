//! `hydra-grader baseline train` and `hydra-grader baseline score`: the arguments of
//! `baseline.py` and its two commands that build ticket 213 ported.
//!
//! The arguments follow `argparse` by the reading in `argparse.rs`. The usage line and the
//! help are this binary's own, since its name is not `baseline.py`.
//!
//! Build 217 adds `label-queue` and `label-import` using the same argument parser.

use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::Result;
use hydra_pyjson::json::{self, PyStr, Value};

use crate::argparse::{self, Parsed, parse, quoted, repr, required, wrong};
use crate::baseline::{self, Label, Refusal};
use crate::data::process_aliases;
use crate::isotime::{self, DateTime};
use crate::records;

const PROG: &str = "hydra-grader baseline";
const COMMANDS: [&str; 4] = ["train", "score", "label-queue", "label-import"];
const USAGE: &str = "usage: hydra-grader baseline [-h] {train,score,label-queue,label-import} ...";
const TRAIN_USAGE: &str = "usage: hydra-grader baseline train [-h] [--label {outcome,verdict,task}] \
                           [--test-from TEST_FROM] dataset [model]";
const SCORE_USAGE: &str = "usage: hydra-grader baseline score [-h] model run";
/// `--test-from` when none is given.
pub(crate) const TEST_FROM: &str = "2026-09-23";

/// `str(Path(text))`: no `.` component, no doubled or trailing slash, and `.` for nothing.
pub(crate) fn shown(path: &Path) -> String {
    let clean: PathBuf = path
        .components()
        .filter(|part| !matches!(part, std::path::Component::CurDir))
        .collect();
    if clean.as_os_str().is_empty() {
        ".".to_string()
    } else {
        clean.display().to_string()
    }
}

/// `day`: `datetime.fromisoformat(text).replace(tzinfo=timezone.utc)`.
pub(crate) fn day(text: &str) -> Option<DateTime> {
    isotime::fromisoformat(&PyStr::from(text)).map(DateTime::replace_utc)
}

/// The command a run of `baseline.py` ends with: what it printed and its exit code.
pub(crate) fn finish(done: Result<String>, stdout: &mut dyn Write, stderr: &mut dyn Write) -> u8 {
    match done {
        Ok(printed) => {
            let _ = writeln!(stdout, "{printed}");
            0
        }
        Err(err) => {
            // A refusal is its sentence. Anything else is where the Python ends in a
            // traceback, and its last line starts with the same exception class.
            let _ = match err.downcast_ref::<Refusal>() {
                Some(refusal) => writeln!(stderr, "{refusal}"),
                None => writeln!(stderr, "{err}"),
            };
            1
        }
    }
}

fn train(
    dataset: &Path,
    model: Option<&Path>,
    label: Label,
    test_from: &DateTime,
) -> Result<String> {
    let rows = records::read_jsonl(dataset)?;
    let fitted = baseline::train(&rows, label, test_from)?;
    let path = match model {
        Some(path) => path.to_path_buf(),
        None => records::grader_home()?
            .join("models")
            .join(format!("{}.json", label.name())),
    };
    records::write_json(&path, &fitted)?;
    Ok(json::dumps(
        &baseline::report(&fitted, &rows, label)?,
        Some(2),
    ))
}

/// `record_path`: a record path, or a full run id under `$HYDRA_ROOT/runs/`.
pub(crate) fn record_path(run: &str) -> Result<PathBuf> {
    if Path::new(run).is_file() {
        return Ok(PathBuf::from(run));
    }
    Ok(records::hydra_root()?
        .join("runs")
        .join(format!("{run}.jsonl")))
}

fn score(model: &Path, run: &str, stderr: &mut dyn Write) -> Result<String> {
    let model = records::read_json(model)?.unwrap_or(Value::Null);
    let path = record_path(run)?;
    let row = baseline::row_from_record(&path, &shown(&path), &process_aliases, stderr)?;
    baseline::score_line(&model, &row)
}

/// `baseline.py`'s `main` on the arguments after `baseline`. Returns the exit code.
pub fn main(args: &[String], stdout: &mut dyn Write, stderr: &mut dyn Write) -> u8 {
    let (command, rest, top_extras) =
        match argparse::command(args, USAGE, PROG, &COMMANDS, stdout, stderr) {
            Ok(read) => read,
            Err(code) => return code,
        };
    let unrecognized = |stderr: &mut dyn Write, extras: Vec<String>| -> Option<u8> {
        let mut all = top_extras.clone();
        all.extend(extras);
        (!all.is_empty()).then(|| {
            let message = format!("unrecognized arguments: {}", all.join(" "));
            wrong(stderr, USAGE, PROG, &message)
        })
    };

    match command {
        "train" => {
            let prog = format!("{PROG} train");
            let mut label = Label::Outcome;
            let mut test_from = day(TEST_FROM).expect("the default day reads");
            let mut take = |option: &'static str, value: &str| -> Result<(), String> {
                if option == "--label" {
                    label = Label::named(value).ok_or_else(|| {
                        let names: Vec<&str> = Label::ALL.iter().map(|l| l.name()).collect();
                        format!(
                            "argument --label: invalid choice: {} (choose from {})",
                            repr(value),
                            quoted(&names)
                        )
                    })?;
                } else {
                    test_from = day(value).ok_or_else(|| {
                        format!("argument --test-from: invalid day value: {}", repr(value))
                    })?;
                }
                Ok(())
            };
            let (positionals, extras) = match parse(rest, &["--label", "--test-from"], 2, &mut take)
            {
                Parsed::Help => {
                    let _ = writeln!(stdout, "{TRAIN_USAGE}");
                    return 0;
                }
                Parsed::Wrong(message) => return wrong(stderr, TRAIN_USAGE, &prog, &message),
                Parsed::Args {
                    positionals,
                    extras,
                } => (positionals, extras),
            };
            if let Some(message) = required(&["dataset"], positionals.len()) {
                return wrong(stderr, TRAIN_USAGE, &prog, &message);
            }
            if let Some(code) = unrecognized(stderr, extras) {
                return code;
            }
            let model = positionals.get(1).map(Path::new);
            let done = train(Path::new(&positionals[0]), model, label, &test_from);
            finish(done, stdout, stderr)
        }
        "score" => {
            let prog = format!("{PROG} score");
            let mut take = |_: &'static str, _: &str| -> Result<(), String> { Ok(()) };
            let (positionals, extras) = match parse(rest, &[], 2, &mut take) {
                Parsed::Help => {
                    let _ = writeln!(stdout, "{SCORE_USAGE}");
                    return 0;
                }
                Parsed::Wrong(message) => return wrong(stderr, SCORE_USAGE, &prog, &message),
                Parsed::Args {
                    positionals,
                    extras,
                } => (positionals, extras),
            };
            if let Some(message) = required(&["model", "run"], positionals.len()) {
                return wrong(stderr, SCORE_USAGE, &prog, &message);
            }
            if let Some(code) = unrecognized(stderr, extras) {
                return code;
            }
            let done = score(Path::new(&positionals[0]), &positionals[1], stderr);
            finish(done, stdout, stderr)
        }
        command => {
            let prog = format!("{PROG} {command}");
            let usage = format!(
                "usage: {prog} [-h] [--dataset DATASET] {}",
                if command == "label-queue" {
                    "[--tickets TICKETS] out"
                } else {
                    "[--home HOME] csv"
                }
            );
            let mut dataset = None;
            let mut home = None;
            let mut tickets = crate::queue::default_tickets_dir();
            let options = if command == "label-queue" {
                vec!["--tickets", "--dataset"]
            } else {
                vec!["--home", "--dataset"]
            };
            let mut take = |option: &'static str, value: &str| -> Result<(), String> {
                match option {
                    "--dataset" => dataset = Some(PathBuf::from(value)),
                    "--home" => home = Some(PathBuf::from(value)),
                    _ => tickets = PathBuf::from(value),
                }
                Ok(())
            };
            let (positionals, extras) = match parse(rest, &options, 1, &mut take) {
                Parsed::Help => {
                    let _ = writeln!(stdout, "{usage}");
                    return 0;
                }
                Parsed::Wrong(message) => return wrong(stderr, &usage, &prog, &message),
                Parsed::Args {
                    positionals,
                    extras,
                } => (positionals, extras),
            };
            if let Some(message) = required(
                &[if command == "label-queue" {
                    "out"
                } else {
                    "csv"
                }],
                positionals.len(),
            ) {
                return wrong(stderr, &usage, &prog, &message);
            }
            if let Some(code) = unrecognized(stderr, extras) {
                return code;
            }
            let done = (|| -> Result<String> {
                if command == "label-queue" {
                    let dataset = match dataset {
                        Some(p) => p,
                        None => records::grader_home()?.join("dataset.jsonl"),
                    };
                    let rows = records::read_jsonl(&dataset)?;
                    let (queue, skips) = crate::queue::label_queue(&tickets, &rows)?;
                    crate::queue::write_label_queue_csv(Path::new(&positionals[0]), &queue)?;
                    let mut printed = format!("rows: {}", queue.len());
                    for skip in skips {
                        printed.push_str(&format!("\nskip: {skip}"));
                    }
                    Ok(printed)
                } else {
                    let home = match home {
                        Some(p) => p,
                        None => records::grader_home()?,
                    };
                    let dataset = dataset.unwrap_or_else(|| home.join("dataset.jsonl"));
                    let (labelled, unknown) =
                        crate::queue::label_import(Path::new(&positionals[0]), &dataset, &home)?;
                    Ok(format!("labelled: {labelled}\nunknown: {unknown}"))
                }
            })();
            finish(done, stdout, stderr)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_path_prints_as_pathlib_prints_it() {
        assert_eq!(shown(Path::new("./a//b/./c/")), "a/b/c");
        assert_eq!(shown(Path::new("")), ".");
        assert_eq!(shown(Path::new("/a/../b")), "/a/../b");
    }
}
