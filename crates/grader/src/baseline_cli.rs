//! `hydra-grader baseline train` and `hydra-grader baseline score`: the arguments of
//! `baseline.py` and its two commands that build ticket 213 ported.
//!
//! The arguments follow `argparse` where a caller can tell: an option may be cut short to
//! any prefix that names one option, takes its value after a space or an `=`, and may sit
//! before, between or after the positionals; `--` ends the options; a surplus argument is
//! "unrecognized" and reported after everything else parsed. A wrong command line exits 2
//! with a usage line and then `PROG: error: MESSAGE`, and the message is the Python's. The
//! usage line and the help are this binary's own, since its name is not `baseline.py`.
//!
//! Build 217 adds `label-queue` and `label-import` using the same argument parser.

use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::Result;
use hydra_pyjson::json::{self, PyStr, Value};
use hydra_pyjson::models::{self, Pair};
use hydra_pyjson::py;

use crate::baseline::{self, Label, Refusal};
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

/// Where `aliases.json` is read from, found when the command runs: `HYDRA_PUPPETS_DATA`
/// when set, else `integrations/puppets` under `HYDRA_HOME`, else the first directory above
/// this binary that holds one. It is the rule `hydra-puppets` finds its data by, and the
/// same file `records.load_aliases()` reads in the Python.
fn aliases_path() -> PathBuf {
    let named = |name: &str| std::env::var_os(name).filter(|value| !value.is_empty());
    const FILE: &str = "integrations/puppets/aliases.json";
    if let Some(dir) = named("HYDRA_PUPPETS_DATA") {
        return PathBuf::from(dir).join("aliases.json");
    }
    if let Some(home) = named("HYDRA_HOME") {
        return PathBuf::from(home).join(FILE);
    }
    std::env::current_exe()
        .ok()
        .and_then(|exe| {
            exe.ancestors()
                .map(|dir| dir.join(FILE))
                .find(|file| file.is_file())
        })
        .unwrap_or_else(|| PathBuf::from(FILE))
}

pub(crate) fn load_aliases() -> Result<Vec<(Pair, Pair)>> {
    models::load_aliases(&aliases_path())
}

/// `repr(text)`, which is how `argparse` quotes a value it refuses.
pub(crate) fn repr(text: &str) -> String {
    py::repr(&Value::text(text))
}

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

/// What `argparse` takes for a negative number, which is an argument and not an option: a
/// `-`, then a digit or a `.` and a digit, then anything at all.
fn negative_number(arg: &str) -> bool {
    let rest = arg.strip_prefix('-').unwrap_or("x");
    let rest = rest.strip_prefix('.').unwrap_or(rest);
    rest.starts_with(|c: char| c.is_ascii_digit())
}

/// What one argument is to a parser whose long options are `longs`.
pub(crate) enum Kind {
    Positional,
    Help,
    /// An option and the value written after its `=`, if any.
    Option(&'static str, Option<String>),
    /// The message after `error: `.
    Wrong(String),
    /// It looks like an option and the parser has none by that name.
    Unknown,
}

/// `_parse_optional`: an option may be cut short to any prefix that names one option, and
/// what names none is a positional when it looks like a negative number or holds a space.
pub(crate) fn kind(arg: &str, longs: &[&'static str]) -> Kind {
    if !arg.starts_with('-') || arg.len() == 1 {
        return Kind::Positional;
    }
    if !arg.starts_with("--") {
        if arg.starts_with("-h") {
            return Kind::Help;
        }
    } else {
        let (head, explicit) = match arg.split_once('=') {
            Some((head, value)) => (head, Some(value.to_string())),
            None => (arg, None),
        };
        let matches: Vec<&'static str> = match longs.iter().find(|long| **long == head) {
            Some(exact) => vec![exact],
            None => longs
                .iter()
                .copied()
                .filter(|long| long.starts_with(head))
                .collect(),
        };
        match matches.as_slice() {
            [] => {}
            ["--help"] => {
                return match explicit {
                    Some(value) => Kind::Wrong(format!(
                        "argument -h/--help: ignored explicit argument {}",
                        repr(&value)
                    )),
                    None => Kind::Help,
                };
            }
            [long] => return Kind::Option(long, explicit),
            several => {
                return Kind::Wrong(format!(
                    "ambiguous option: {arg} could match {}",
                    several.join(", ")
                ));
            }
        }
    }
    if negative_number(arg) || arg.contains(' ') {
        Kind::Positional
    } else {
        Kind::Unknown
    }
}

/// One command line, read.
pub(crate) enum Parsed {
    Help,
    /// The message after `error: `.
    Wrong(String),
    Args {
        positionals: Vec<String>,
        /// What no option or positional took, in the order it was given.
        extras: Vec<String>,
    },
}

/// Read `args` for a command with `valued` options, each taking one value, and at most
/// `places` positionals. `take` is handed each option's value as it is met and answers with
/// the error `argparse` would raise for it.
pub(crate) fn parse(
    args: &[String],
    valued: &[&'static str],
    places: usize,
    take: &mut dyn FnMut(&'static str, &str) -> Result<(), String>,
) -> Parsed {
    let mut longs: Vec<&'static str> = vec!["--help"];
    longs.extend(valued);
    let (mut positionals, mut extras) = (Vec::new(), Vec::new());
    let mut only_positionals = false;
    let mut i = 0;
    while i < args.len() {
        let arg = args[i].as_str();
        i += 1;
        if !only_positionals && arg == "--" {
            only_positionals = true;
            continue;
        }
        let found = if only_positionals {
            Kind::Positional
        } else {
            kind(arg, &longs)
        };
        match found {
            Kind::Positional if positionals.len() < places => positionals.push(arg.to_string()),
            Kind::Positional | Kind::Unknown => extras.push(arg.to_string()),
            Kind::Help => return Parsed::Help,
            Kind::Wrong(message) => return Parsed::Wrong(message),
            Kind::Option(long, explicit) => {
                let value = match explicit {
                    Some(value) => value,
                    None => match args.get(i) {
                        Some(next)
                            if next != "--" && matches!(kind(next, &longs), Kind::Positional) =>
                        {
                            i += 1;
                            next.clone()
                        }
                        _ => {
                            return Parsed::Wrong(format!(
                                "argument {long}: expected one argument"
                            ));
                        }
                    },
                };
                if let Err(message) = take(long, &value) {
                    return Parsed::Wrong(message);
                }
            }
        }
    }
    Parsed::Args {
        positionals,
        extras,
    }
}

/// A wrong command line: the usage, the error, and exit code 2.
pub(crate) fn wrong(stderr: &mut dyn Write, usage: &str, prog: &str, message: &str) -> u8 {
    let _ = writeln!(stderr, "{usage}\n{prog}: error: {message}");
    2
}

pub(crate) fn required(names: &[&str], given: usize) -> Option<String> {
    (given < names.len()).then(|| {
        format!(
            "the following arguments are required: {}",
            names[given..].join(", ")
        )
    })
}

pub(crate) fn quoted(names: &[&str]) -> String {
    let names: Vec<String> = names.iter().map(|name| format!("'{name}'")).collect();
    names.join(", ")
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
    let row = baseline::row_from_record(&path, &shown(&path), &load_aliases, stderr)?;
    baseline::score_line(&model, &row)
}

/// `baseline.py`'s `main` on the arguments after `baseline`. Returns the exit code.
pub fn main(args: &[String], stdout: &mut dyn Write, stderr: &mut dyn Write) -> u8 {
    // The top parser: `-h`, then the command. An option it does not know waits until the
    // command has parsed and is then unrecognized.
    let mut top_extras = Vec::new();
    let mut rest = args;
    let command = loop {
        let Some((first, after)) = rest.split_first() else {
            return wrong(
                stderr,
                USAGE,
                PROG,
                "the following arguments are required: command",
            );
        };
        rest = after;
        if first == "--" {
            continue;
        }
        match kind(first, &["--help"]) {
            Kind::Positional => {}
            Kind::Help => {
                let _ = writeln!(stdout, "{USAGE}");
                return 0;
            }
            Kind::Wrong(message) => return wrong(stderr, USAGE, PROG, &message),
            Kind::Option(..) | Kind::Unknown => {
                top_extras.push(first.clone());
                continue;
            }
        }
        break first.as_str();
    };
    if !COMMANDS.contains(&command) {
        let message = format!(
            "argument command: invalid choice: {} (choose from {})",
            repr(command),
            quoted(&COMMANDS)
        );
        return wrong(stderr, USAGE, PROG, &message);
    }
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
    fn a_negative_number_is_an_argument_and_not_an_option() {
        for number in ["-1", "-1.5", "-.5", "-1_000", "-1.", "-1x"] {
            assert!(negative_number(number), "{number}");
        }
        for other in ["-", "--1", "-x", "-.", "-_1", "1", ".5"] {
            assert!(!negative_number(other), "{other}");
        }
    }

    #[test]
    fn a_path_prints_as_pathlib_prints_it() {
        assert_eq!(shown(Path::new("./a//b/./c/")), "a/b/c");
        assert_eq!(shown(Path::new("")), ".");
        assert_eq!(shown(Path::new("/a/../b")), "/a/../b");
    }
}
