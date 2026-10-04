//! The reference's argparse rules, through the existing shared parser.
use crate::{
    argparse::{Kind, Parsed, kind, parse, quoted, repr, wrong},
    records, triage,
};
use anyhow::Result;
use hydra_pyjson::{json::Value, py};
use std::io::Write;
const PROG: &str = "hydra-grader triage";
const COMMANDS: [&str; 3] = ["run", "table", "sample"];
const USAGE: &str = "usage: hydra-grader triage [-h] {run,table,sample} ...";

/// Dispatch the arguments after `triage`, returning the reference's exit status.
pub fn main(args: &[String], stdout: &mut dyn Write, stderr: &mut dyn Write) -> u8 {
    let mut rest = args;
    let mut extras = Vec::new();
    let command = loop {
        let Some((first, after)) = rest.split_first() else {
            return wrong(
                stderr,
                USAGE,
                PROG,
                "the following arguments are required: cmd",
            );
        };
        rest = after;
        if first == "--" {
            continue;
        }
        match kind(first, &["--help"]) {
            Kind::Help => {
                let _ = writeln!(stdout, "{USAGE}");
                return 0;
            }
            Kind::Wrong(m) => return wrong(stderr, USAGE, PROG, &m),
            Kind::Positional => break first.as_str(),
            _ => extras.push(first.clone()),
        }
    };
    if !COMMANDS.contains(&command) {
        return wrong(
            stderr,
            USAGE,
            PROG,
            &format!(
                "argument cmd: invalid choice: {} (choose from {})",
                repr(command),
                quoted(&COMMANDS)
            ),
        );
    }
    let prog = format!("{PROG} {command}");
    let usage = if command == "sample" {
        format!("usage: {prog} [-h] [--n N] [--seed SEED]")
    } else {
        format!("usage: {prog} [-h]")
    };
    let (mut n, mut seed) = (Value::Int(20), String::from("s"));
    let mut take = |option: &'static str, value: &str| -> Result<(), String> {
        if option == "--n" {
            n = py::int_value(value)
                .ok_or_else(|| format!("argument --n: invalid int value: {}", repr(value)))?;
        } else {
            seed = value.to_string();
        }
        Ok(())
    };
    let options: &[&'static str] = if command == "sample" {
        &["--n", "--seed"]
    } else {
        &[]
    };
    // With no positional arguments argparse preserves the separator itself among
    // unrecognized arguments. The shared parser otherwise consumes it.
    let marker = rest
        .iter()
        .position(|arg| arg == "--")
        .unwrap_or(rest.len());
    match parse(&rest[..marker], options, 0, &mut take) {
        Parsed::Help => {
            let _ = writeln!(stdout, "{usage}");
            return 0;
        }
        Parsed::Wrong(m) => return wrong(stderr, &usage, &prog, &m),
        Parsed::Args { extras: more, .. } => extras.extend(more),
    }
    extras.extend_from_slice(&rest[marker..]);
    if !extras.is_empty() {
        return wrong(
            stderr,
            USAGE,
            PROG,
            &format!("unrecognized arguments: {}", extras.join(" ")),
        );
    }
    let done = (|| -> Result<String> {
        let home = records::grader_home()?;
        match command {
            "run" => {
                triage::run(&home, &records::hydra_root()?.join("runs"))?;
                Ok(String::new())
            }
            "table" => triage::table(&home),
            _ => triage::sample(&home, &n, &seed),
        }
    })();
    match done {
        Ok(text) => {
            let _ = write!(stdout, "{text}");
            0
        }
        Err(e) => {
            let _ = writeln!(stderr, "{e}");
            1
        }
    }
}
