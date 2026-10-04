//! `hydra-grader system-one`: fit the v0 logistic head and print its report.

use std::io::Write;
use std::path::PathBuf;

use anyhow::Result;

use crate::argparse::{Parsed, parse, wrong};
use crate::baseline::Refusal;
use crate::baseline_cli::{TEST_FROM, day};
use crate::labels::Env;
use crate::system_one;
use hydra_pyjson::json;
use hydra_records::records;

const PROG: &str = "hydra-grader system-one";
const USAGE: &str = "usage: hydra-grader system-one [-h] [--home HOME] [--dataset DATASET] \
                     [--test-from TEST_FROM]";

/// The command after `system-one`. Returns the exit code.
pub fn main(args: &[String], stdout: &mut dyn Write, stderr: &mut dyn Write) -> u8 {
    let mut home = None;
    let mut dataset = None;
    let mut test_from = day(TEST_FROM).expect("the default day reads");
    let mut take = |option: &'static str, value: &str| -> Result<(), String> {
        match option {
            "--home" => home = Some(PathBuf::from(value)),
            "--dataset" => dataset = Some(PathBuf::from(value)),
            "--test-from" => {
                test_from = day(value).ok_or_else(|| {
                    format!(
                        "argument --test-from: invalid day value: {}",
                        crate::argparse::repr(value)
                    )
                })?;
            }
            _ => {}
        }
        Ok(())
    };
    let (positionals, extras) =
        match parse(args, &["--home", "--dataset", "--test-from"], 0, &mut take) {
            Parsed::Help => {
                let _ = writeln!(stdout, "{USAGE}");
                return 0;
            }
            Parsed::Wrong(message) => return wrong(stderr, USAGE, PROG, &message),
            Parsed::Args {
                positionals,
                extras,
            } => (positionals, extras),
        };
    if !positionals.is_empty() || !extras.is_empty() {
        let mut all = extras;
        all.extend(positionals);
        return wrong(
            stderr,
            USAGE,
            PROG,
            &format!("unrecognized arguments: {}", all.join(" ")),
        );
    }
    let done = (|| -> Result<String> {
        let home = match home {
            Some(path) => path,
            None => records::grader_home()?,
        };
        let dataset = dataset.unwrap_or_else(|| home.join("dataset.jsonl"));
        let rows = records::read_jsonl(&dataset)?;
        let env = Env::process();
        let report = system_one::report(&rows, &test_from, &env)?;
        Ok(json::dumps(&report, Some(2)))
    })();
    match done {
        Ok(printed) => {
            let _ = writeln!(stdout, "{printed}");
            0
        }
        Err(err) => {
            let _ = match err.downcast_ref::<Refusal>() {
                Some(refusal) => writeln!(stderr, "{refusal}"),
                None => writeln!(stderr, "{err}"),
            };
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(args: &[&str]) -> (u8, String, String) {
        let args: Vec<String> = args.iter().map(|arg| arg.to_string()).collect();
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let code = main(&args, &mut out, &mut err);
        (
            code,
            String::from_utf8(out).unwrap(),
            String::from_utf8(err).unwrap(),
        )
    }

    #[test]
    fn help_prints_the_usage_line() {
        let (code, out, err) = run(&["-h"]);
        assert_eq!(code, 0, "{err}");
        assert!(out.contains("hydra-grader system-one"), "{out}");
        assert!(err.is_empty(), "{err}");
    }

    #[test]
    fn a_surplus_argument_is_unrecognized() {
        let (code, _, err) = run(&["surprise"]);
        assert_eq!(code, 2);
        assert!(err.contains("unrecognized arguments: surprise"), "{err}");
    }
}
