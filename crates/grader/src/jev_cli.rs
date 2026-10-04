//! Argument parsing for the local commands of `jev_probe.py`.
use crate::argparse::{Parsed, parse, required, wrong};
use std::io::Write;
use std::path::Path;

pub fn main(args: &[String], stdout: &mut dyn Write, stderr: &mut dyn Write) -> u8 {
    const PROG: &str = "hydra-grader jev";
    const USAGE: &str = "usage: hydra-grader jev [-h] {build,patches,ask,score} ...";
    let mut extras = Vec::new();
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
        match parse(std::slice::from_ref(first), &[], 1, &mut |_, _| Ok(())) {
            Parsed::Help => {
                let _ = writeln!(stdout, "{USAGE}");
                return 0;
            }
            Parsed::Wrong(message) => return wrong(stderr, USAGE, PROG, &message),
            Parsed::Args {
                positionals,
                extras: unknown,
            } => {
                extras.extend(unknown);
                if !positionals.is_empty() {
                    break first.as_str();
                }
            }
        }
    };
    let names: &[&str] = match command {
        "build" => &["out"],
        "patches" => &["labels", "out"],
        "score" => &["answers"],
        "ask" => &["inp", "out"],
        _ => {
            return wrong(
                stderr,
                USAGE,
                PROG,
                &format!(
                    "argument command: invalid choice: {} (choose from 'build', 'patches', 'ask', 'score')",
                    hydra_pyjson::py::repr(&hydra_pyjson::json::Value::text(command))
                ),
            );
        }
    };
    let prog = format!("{PROG} {command}");
    let usage = format!("usage: {prog} [-h] {}", names.join(" "));
    let positionals = match parse(rest, &[], names.len(), &mut |_, _| Ok(())) {
        Parsed::Help => {
            let _ = writeln!(stdout, "{usage}");
            return 0;
        }
        Parsed::Wrong(message) => return wrong(stderr, &usage, &prog, &message),
        Parsed::Args {
            positionals,
            extras: unknown,
        } => {
            extras.extend(unknown);
            positionals
        }
    };
    if let Some(message) = required(names, positionals.len()) {
        return wrong(stderr, &usage, &prog, &message);
    }
    if !extras.is_empty() {
        return wrong(
            stderr,
            USAGE,
            PROG,
            &format!("unrecognized arguments: {}", extras.join(" ")),
        );
    }
    let done = match command {
        "build" => crate::jev::cmd_build(Path::new(&positionals[0])).map(|(text, _)| text),
        "patches" => {
            crate::jev::cmd_patches(Path::new(&positionals[0]), Path::new(&positionals[1]))
        }
        "score" => crate::jev::cmd_score_with_output(Path::new(&positionals[0]), stdout),
        _ => {
            let _ = writeln!(
                stderr,
                "{PROG}: ask was dropped with jev_probe.py (map ticket 94): it posted to a remote API and its results stay recorded"
            );
            return 2;
        }
    };
    match done {
        Ok(text) => {
            let _ = write!(stdout, "{text}");
            0
        }
        Err(err) => {
            let _ = writeln!(stderr, "{err}");
            1
        }
    }
}
