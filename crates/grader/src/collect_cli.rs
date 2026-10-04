//! `hydra-grader once`, `watch`, `rebuild` and `backfill`: the arguments of `grader.py` and
//! its four commands (build ticket 214).
//!
//! The arguments follow `argparse` by the reading in `argparse.rs`. No command here has a
//! positional, so a `--` and everything after it is left over, the `--` included. The usage
//! line and the help are this binary's own, since its name is not `grader.py`.
//!
//! A pass that fails ends `once`, `rebuild` and `backfill` with exit code 1 and one line on
//! stderr that starts with the class of the exception the Python ends on. `watch` reports
//! it as `grader: pass failed: ...` and tries again after the interval.
//!
//! A pass that finds another one running in the same grader home says so on stderr and
//! exits 0 without reading or writing anything (build ticket 220).

use std::io::Write;

use anyhow::{Result, bail};
use hydra_pyjson::json::Value;
use hydra_pyjson::models::Pair;
use hydra_pyjson::py;

use crate::argparse::{self, Parsed, parse, repr, wrong};
use crate::collect::{self, Busy, Pass};
use crate::data;
use crate::labels::{Env, LabelsError};
use hydra_records::records;

const PROG: &str = "hydra-grader";
/// The commands `grader.py` has, which is what its error for an unknown one lists.
pub const COMMANDS: [&str; 4] = ["once", "watch", "rebuild", "backfill"];
const USAGE: &str = "usage: hydra-grader [-h] {once,watch,rebuild,backfill,baseline} ...";

/// `max(1, jobs)` for a whole number of any width.
fn workers(jobs: &Value) -> usize {
    match jobs {
        Value::Int(n) => usize::try_from((*n).max(1)).unwrap_or(usize::MAX),
        Value::BigInt(digits) if !digits.starts_with('-') => usize::MAX,
        _ => 1,
    }
}

/// `time.sleep(interval)`: the wait, or the error Python raises for a length it cannot
/// sleep.
fn sleep_length(interval: &Value) -> Result<std::time::Duration> {
    // Past this many seconds the length does not fit the nanoseconds Python counts in.
    const LONGEST: i128 = i64::MAX as i128 / 1_000_000_000;
    // The length is counted before its sign is looked at, so one too long is an overflow in
    // either direction.
    match interval {
        Value::Int(n) if (0..=LONGEST).contains(n) => Ok(std::time::Duration::from_secs(*n as u64)),
        Value::Int(n) if (-LONGEST..0).contains(n) => {
            bail!("ValueError: sleep length must be non-negative")
        }
        _ => bail!("OverflowError: timestamp out of range for C PyTime_t"),
    }
}

/// What a failure says of itself: `str(err)` where the Python's sentence is known, which is
/// the text after the class.
fn said(err: &anyhow::Error) -> String {
    if let Some(labels) = err.downcast_ref::<LabelsError>() {
        return labels.0.clone();
    }
    let text = err.to_string();
    match text.split_once(": ") {
        Some((_, message)) => message.to_string(),
        None => text,
    }
}

/// The last line of a pass that failed: the class of the exception, then what it says.
fn raised(err: &anyhow::Error) -> String {
    match err.downcast_ref::<LabelsError>() {
        Some(labels) => format!("LabelsError: {}", labels.0),
        None => err.to_string(),
    }
}

/// `grader.py`'s `main` on `args`, in the environment `env`. Returns the exit code.
pub fn main(args: &[String], env: &Env, stdout: &mut dyn Write, stderr: &mut dyn Write) -> u8 {
    let (command, rest, mut top_extras) =
        match argparse::command(args, USAGE, PROG, &COMMANDS, stdout, stderr) {
            Ok(read) => read,
            Err(code) => return code,
        };

    let prog = format!("{PROG} {command}");
    let (valued, usage): (&[&'static str], String) = match command {
        "watch" => (
            &["--interval", "--jobs"],
            format!("usage: {prog} [-h] --interval INTERVAL [--jobs JOBS]"),
        ),
        "backfill" => (&[], format!("usage: {prog} [-h]")),
        _ => (&["--jobs"], format!("usage: {prog} [-h] [--jobs JOBS]")),
    };
    let mut jobs = Value::Int(4);
    let mut interval = None;
    let mut take = |option: &'static str, value: &str| -> Result<(), String> {
        let number = py::int_value(value)
            .ok_or_else(|| format!("argument {option}: invalid int value: {}", repr(value)))?;
        if option == "--jobs" {
            jobs = number;
        } else {
            interval = Some(number);
        }
        Ok(())
    };
    let extras = match parse(rest, valued, 0, &mut take) {
        Parsed::Help => {
            let _ = writeln!(stdout, "{usage}");
            return 0;
        }
        Parsed::Wrong(message) => return wrong(stderr, &usage, &prog, &message),
        Parsed::Args { extras, .. } => extras,
    };
    if command == "watch" && interval.is_none() {
        return wrong(
            stderr,
            &usage,
            &prog,
            "the following arguments are required: --interval",
        );
    }
    top_extras.extend(extras);
    if !top_extras.is_empty() {
        let message = format!("unrecognized arguments: {}", top_extras.join(" "));
        return wrong(stderr, USAGE, PROG, &message);
    }

    let aliases = || -> Result<Vec<(Pair, Pair)>> { data::aliases(env) };
    let pass = Pass {
        jobs: workers(&jobs),
        rebuild: command == "rebuild",
        ..Pass::default()
    };
    let finish = |done: Result<usize>, stdout: &mut dyn Write, stderr: &mut dyn Write| -> u8 {
        match done {
            Ok(count) => {
                let _ = writeln!(stdout, "{count}");
                0
            }
            // Another pass is at work in this home and will write what this one would.
            Err(err) if err.is::<Busy>() => {
                let _ = writeln!(stderr, "{err}");
                0
            }
            Err(err) => {
                let _ = writeln!(stderr, "{}", raised(&err));
                1
            }
        }
    };

    match command {
        "backfill" => {
            let done = (|| {
                let home = records::grader_home_in(&|name| env.get(name))?;
                let _lock = collect::lock_home(&home)?;
                let rows = records::read_jsonl(&home.join("dataset.jsonl"))?;
                collect::score_rows(&home, &rows, &collect::utc_now())
            })();
            finish(done, stdout, stderr)
        }
        "once" | "rebuild" => {
            let done = collect::collect(&pass, env, &aliases, stderr);
            finish(done, stdout, stderr)
        }
        _ => {
            let interval = interval.expect("checked above");
            // A pass that fails (a `hydra show` that errors, a record mid-write) must not
            // end the watch: report it on stderr and try again after the interval.
            loop {
                match collect::collect(&pass, env, &aliases, stderr) {
                    Ok(count) => {
                        let _ = writeln!(stdout, "{count}");
                        let _ = stdout.flush();
                    }
                    Err(err) if err.is::<Busy>() => {
                        let _ = writeln!(stderr, "{err}");
                        let _ = stderr.flush();
                    }
                    Err(err) => {
                        let _ = writeln!(stderr, "grader: pass failed: {}", said(&err));
                        let _ = stderr.flush();
                    }
                }
                match sleep_length(&interval) {
                    Ok(length) => std::thread::sleep(length),
                    Err(err) => {
                        let _ = writeln!(stderr, "{err}");
                        return 1;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jobs_are_at_least_one_at_any_width() {
        assert_eq!(workers(&Value::Int(-3)), 1);
        assert_eq!(workers(&Value::Int(0)), 1);
        assert_eq!(workers(&Value::Int(6)), 6);
        assert_eq!(workers(&Value::BigInt("-9".repeat(50))), 1);
        assert_eq!(workers(&Value::BigInt("9".repeat(50))), usize::MAX);
    }

    #[test]
    fn a_sleep_python_refuses_is_refused() {
        let class = |interval: Value| {
            let err = sleep_length(&interval).unwrap_err().to_string();
            err.split(':').next().unwrap().to_string()
        };
        assert_eq!(class(Value::Int(-1)), "ValueError");
        assert_eq!(class(Value::Int(-9_223_372_037)), "OverflowError");
        assert_eq!(
            class(Value::BigInt(format!("-{}", "9".repeat(50)))),
            "OverflowError"
        );
        assert_eq!(class(Value::Int(9_223_372_037)), "OverflowError");
        assert_eq!(class(Value::BigInt("9".repeat(50))), "OverflowError");
        assert_eq!(
            sleep_length(&Value::Int(9_223_372_036)).unwrap().as_secs(),
            9_223_372_036
        );
        assert_eq!(sleep_length(&Value::Int(0)).unwrap().as_secs(), 0);
    }

    #[test]
    fn a_failure_says_what_the_python_exception_says() {
        let labels: anyhow::Error = LabelsError("cannot run /x: gone".into()).into();
        assert_eq!(said(&labels), "cannot run /x: gone");
        assert_eq!(raised(&labels), "LabelsError: cannot run /x: gone");
        let other = anyhow::anyhow!("KeyError: 'run_id'");
        assert_eq!(said(&other), "'run_id'");
        assert_eq!(raised(&other), "KeyError: 'run_id'");
    }
}
