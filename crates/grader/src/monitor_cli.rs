//! `hydra-grader monitor features`, `train` and `score`: the arguments of `monitor.py` and
//! its three commands (build ticket 215).
//!
//! The arguments follow `argparse` by the rules `baseline_cli.rs` spells out, and through
//! its reader. A wrong command line exits 2 with a usage line and then
//! `PROG: error: MESSAGE`, and the message is the Python's. The usage line and the help are
//! this binary's own, since its name is not `monitor.py`.

use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::Result;
use hydra_pyjson::json::{self, Value};

use crate::baseline::{Label, Refusal};
use crate::baseline_cli::{
    Kind, Parsed, TEST_FROM, day, finish, kind, load_aliases, parse, quoted, record_path, repr,
    shown, wrong,
};
use crate::isotime::DateTime;
use crate::monitor;
use crate::records;

const PROG: &str = "hydra-grader monitor";
const COMMANDS: [&str; 3] = ["features", "train", "score"];
const USAGE: &str = "usage: hydra-grader monitor [-h] {features,train,score} ...";
const FEATURES_USAGE: &str = "usage: hydra-grader monitor features [-h] --at AT record";
const TRAIN_USAGE: &str = "usage: hydra-grader monitor train [-h] [--label {outcome,verdict,task}] \
                           --at AT [--test-from TEST_FROM] [--records RECORDS] dataset [model]";
const SCORE_USAGE: &str = "usage: hydra-grader monitor score [-h] model run";

/// What `argparse` says when `--at` is not one of the three fractions.
fn fraction(value: &str) -> Result<f64, String> {
    monitor::parse_fraction(value).ok_or_else(|| {
        let tokens: Vec<String> = monitor::FRACTIONS
            .into_iter()
            .map(monitor::fraction_token)
            .collect();
        format!("argument --at: at must be one of {}", tokens.join(", "))
    })
}

/// The required arguments that were not given, in the order the parser declares them.
fn missing(positional: &str, given: usize, at: Option<f64>) -> Option<String> {
    let mut names = Vec::new();
    if given == 0 {
        names.push(positional);
    }
    if at.is_none() {
        names.push("--at");
    }
    (!names.is_empty())
        .then(|| format!("the following arguments are required: {}", names.join(", ")))
}

fn features(record: &str, at: f64, stderr: &mut dyn Write) -> Result<String> {
    let path = record_path(record)?;
    monitor::feature_view(&path, &shown(&path), at, &load_aliases, stderr)
}

struct Train<'a> {
    dataset: &'a Path,
    model: Option<&'a Path>,
    label: Label,
    at: f64,
    test_from: DateTime,
    records: Option<PathBuf>,
}

fn train(args: &Train, stderr: &mut dyn Write) -> Result<String> {
    let records_dir = match &args.records {
        Some(dir) => dir.clone(),
        None => records::hydra_root()?.join("runs"),
    };
    let rows = monitor::join(
        records::read_jsonl(args.dataset)?,
        &records_dir,
        args.at,
        stderr,
    )?;
    let fitted = monitor::train(&rows, args.label, &args.test_from, args.at)?;
    let path = match args.model {
        Some(path) => path.to_path_buf(),
        // A folder of its own: `grader.py` scores every `models/*.json` as a pre-launch
        // model.
        None => records::grader_home()?
            .join("models")
            .join("monitor")
            .join(format!(
                "monitor-{}-{}.json",
                args.label.name(),
                monitor::fraction_token(args.at)
            )),
    };
    records::write_json(&path, &fitted)?;
    Ok(json::dumps(
        &monitor::report(&fitted, &rows, args.label)?,
        Some(2),
    ))
}

fn score(model_path: &Path, run: &str, stderr: &mut dyn Write) -> Result<String> {
    let model = records::read_json(model_path)?.unwrap_or(Value::Null);
    let Some(at) = monitor::model_fraction(&model)? else {
        return Err(Refusal(format!(
            "monitor: {} has no prefix fraction",
            shown(model_path)
        ))
        .into());
    };
    let path = record_path(run)?;
    monitor::score_line(&model, at, &path, &shown(&path), &load_aliases, stderr)
}

/// `monitor.py`'s `main` on the arguments after `monitor`. Returns the exit code.
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
    let prog = format!("{PROG} {command}");

    match command {
        "features" => {
            let mut at = None;
            let mut take = |_: &'static str, value: &str| -> Result<(), String> {
                at = Some(fraction(value)?);
                Ok(())
            };
            let (positionals, extras) = match parse(rest, &["--at"], 1, &mut take) {
                Parsed::Help => {
                    let _ = writeln!(stdout, "{FEATURES_USAGE}");
                    return 0;
                }
                Parsed::Wrong(message) => return wrong(stderr, FEATURES_USAGE, &prog, &message),
                Parsed::Args {
                    positionals,
                    extras,
                } => (positionals, extras),
            };
            let at = match (missing("record", positionals.len(), at), at) {
                (None, Some(at)) => at,
                (message, _) => {
                    return wrong(stderr, FEATURES_USAGE, &prog, &message.unwrap_or_default());
                }
            };
            if let Some(code) = unrecognized(stderr, extras) {
                return code;
            }
            let done = features(&positionals[0], at, stderr);
            finish(done, stdout, stderr)
        }
        "train" => {
            let mut label = Label::Outcome;
            let mut at = None;
            let mut test_from = day(TEST_FROM).expect("the default day reads");
            let mut records = None;
            let mut take = |option: &'static str, value: &str| -> Result<(), String> {
                match option {
                    "--label" => {
                        label = Label::named(value).ok_or_else(|| {
                            let names: Vec<&str> = Label::ALL.iter().map(|l| l.name()).collect();
                            format!(
                                "argument --label: invalid choice: {} (choose from {})",
                                repr(value),
                                quoted(&names)
                            )
                        })?;
                    }
                    "--at" => at = Some(fraction(value)?),
                    "--test-from" => {
                        test_from = day(value).ok_or_else(|| {
                            format!("argument --test-from: invalid day value: {}", repr(value))
                        })?;
                    }
                    _ => records = Some(PathBuf::from(value)),
                }
                Ok(())
            };
            let options = ["--label", "--at", "--test-from", "--records"];
            let (positionals, extras) = match parse(rest, &options, 2, &mut take) {
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
            let at = match (missing("dataset", positionals.len(), at), at) {
                (None, Some(at)) => at,
                (message, _) => {
                    return wrong(stderr, TRAIN_USAGE, &prog, &message.unwrap_or_default());
                }
            };
            if let Some(code) = unrecognized(stderr, extras) {
                return code;
            }
            let args = Train {
                dataset: Path::new(&positionals[0]),
                model: positionals.get(1).map(Path::new),
                label,
                at,
                test_from,
                records,
            };
            let done = train(&args, stderr);
            finish(done, stdout, stderr)
        }
        _ => {
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
            if positionals.len() < 2 {
                let names = ["model", "run"];
                let message = format!(
                    "the following arguments are required: {}",
                    names[positionals.len()..].join(", ")
                );
                return wrong(stderr, SCORE_USAGE, &prog, &message);
            }
            if let Some(code) = unrecognized(stderr, extras) {
                return code;
            }
            let done = score(Path::new(&positionals[0]), &positionals[1], stderr);
            finish(done, stdout, stderr)
        }
    }
}
