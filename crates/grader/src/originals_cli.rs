//! `hydra-grader originals`: arguments for joining verdicts with the original diffs.

use std::io::Write;
use std::path::PathBuf;

use anyhow::Result;

use crate::argparse::{Parsed, parse, required, wrong};
use crate::originals::{self, Paths};
use crate::queue;
use hydra_records::records;

const PROG: &str = "hydra-grader originals";
const USAGE: &str = "usage: hydra-grader originals [-h] [--home HOME] [--dataset DATASET] \
                     [--tickets TICKETS] [--repo REPO] [--runs RUNS] out";

/// The command after `originals`. Returns the exit code.
pub fn main(args: &[String], stdout: &mut dyn Write, stderr: &mut dyn Write) -> u8 {
    let mut home = None;
    let mut dataset = None;
    let mut tickets = queue::default_tickets_dir();
    let mut repo = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let mut runs = None;
    let mut take = |option: &'static str, value: &str| -> Result<(), String> {
        match option {
            "--home" => home = Some(PathBuf::from(value)),
            "--dataset" => dataset = Some(PathBuf::from(value)),
            "--tickets" => tickets = PathBuf::from(value),
            "--repo" => repo = PathBuf::from(value),
            "--runs" => runs = Some(PathBuf::from(value)),
            _ => {}
        }
        Ok(())
    };
    let (positionals, extras) = match parse(
        args,
        &["--home", "--dataset", "--tickets", "--repo", "--runs"],
        1,
        &mut take,
    ) {
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
    if let Some(message) = required(&["out"], positionals.len()) {
        return wrong(stderr, USAGE, PROG, &message);
    }
    if !extras.is_empty() {
        return wrong(
            stderr,
            USAGE,
            PROG,
            &format!("unrecognized arguments: {}", extras.join(" ")),
        );
    }
    let done = (|| -> Result<usize> {
        let home = match home {
            Some(path) => path,
            None => records::grader_home()?,
        };
        let dataset = dataset.unwrap_or_else(|| home.join("dataset.jsonl"));
        let runs = match runs {
            Some(path) => path,
            None => records::hydra_root()?.join("runs"),
        };
        originals::write_originals(&Paths {
            home: &home,
            dataset: &dataset,
            tickets: &tickets,
            repo: &repo,
            runs: &runs,
            out: std::path::Path::new(&positionals[0]),
        })
    })();
    match done {
        Ok(count) => {
            let _ = writeln!(stdout, "lines: {count}");
            0
        }
        Err(err) => {
            let _ = writeln!(stderr, "{err}");
            1
        }
    }
}
