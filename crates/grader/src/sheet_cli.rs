//! `hydra-grader sheet build|budget|show`, with argparse's option rules.
use std::io::Write;
use std::path::PathBuf;

use anyhow::Result;
use hydra_pyjson::file::read_text;
use hydra_pyjson::json::{self, Value};
use hydra_pyjson::py;

use crate::argparse::{Kind, Parsed, kind, parse, quoted, repr, wrong};
use crate::{labels::Env, records, sheet};
const PROG: &str = "hydra-grader sheet";
const COMMANDS: [&str; 3] = ["build", "budget", "show"];
const USAGE: &str = "usage: hydra-grader sheet [-h] {build,budget,show} ...";

pub fn main(args: &[String], stdout: &mut dyn Write, stderr: &mut dyn Write) -> u8 {
    let mut rest = args;
    let mut top_extras = Vec::new();
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
            Kind::Help => {
                let _ = writeln!(stdout, "{USAGE}");
                return 0;
            }
            Kind::Wrong(message) => return wrong(stderr, USAGE, PROG, &message),
            Kind::Unknown | Kind::Option(..) => {
                top_extras.push(first.clone());
                continue;
            }
            Kind::Positional => break first.as_str(),
        }
    };
    if !COMMANDS.contains(&command) {
        return wrong(
            stderr,
            USAGE,
            PROG,
            &format!(
                "argument command: invalid choice: {} (choose from {})",
                repr(command),
                quoted(&COMMANDS)
            ),
        );
    }
    let root = std::env::var_os("HYDRA_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::home_dir().unwrap_or_default().join("hydra"));
    let mut home = std::env::var_os("HYDRA_PUPPETS_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("puppets"));
    let mut runs = root.join("runs");
    let mut vendors: Option<PathBuf> = None;
    let mut weight = 1.0;
    let mut since = None;
    let options: &[&'static str] = match command {
        "build" => &["--home", "--runs", "--cache-read-weight"],
        "budget" => &[
            "--home",
            "--runs",
            "--vendors",
            "--since",
            "--cache-read-weight",
        ],
        _ => &["--home"],
    };
    let mut take = |option: &'static str, value: &str| -> Result<(), String> {
        match option {
            "--home" => home = value.into(),
            "--runs" => runs = value.into(),
            "--vendors" => vendors = Some(value.into()),
            "--since" => since = Some(value.to_owned()),
            _ => {
                weight = py::float_literal(value).ok_or_else(|| {
                    format!(
                        "argument --cache-read-weight: invalid float value: {}",
                        repr(value)
                    )
                })?
            }
        };
        Ok(())
    };
    let count = match command {
        "show" => 2,
        "budget" => 1,
        _ => 0,
    };
    let prog = format!("{PROG} {command}");
    let (positionals, extras) = match parse(rest, options, count, &mut take) {
        Parsed::Help => {
            let _ = writeln!(
                stdout,
                "usage: {prog} [-h] {}",
                match command {
                    "show" => "[--home HOME] provider [model]",
                    "budget" =>
                        "[--home HOME] [--runs RUNS] [--vendors VENDORS] [--since SINCE] [--cache-read-weight CACHE_READ_WEIGHT] run_id",
                    _ => "[--home HOME] [--runs RUNS] [--cache-read-weight CACHE_READ_WEIGHT]",
                }
            );
            return 0;
        }
        Parsed::Wrong(message) => return wrong(stderr, USAGE, &prog, &message),
        Parsed::Args {
            positionals,
            extras,
        } => (positionals, extras),
    };
    if command != "build" && positionals.is_empty() {
        return wrong(
            stderr,
            USAGE,
            &prog,
            &format!(
                "the following arguments are required: {}",
                if command == "show" {
                    "provider"
                } else {
                    "run_id"
                }
            ),
        );
    }
    top_extras.extend(extras);
    if !top_extras.is_empty() {
        return wrong(
            stderr,
            USAGE,
            PROG,
            &format!("unrecognized arguments: {}", top_extras.join(" ")),
        );
    }
    if !weight.is_finite() || weight < 0.0 {
        return wrong(
            stderr,
            USAGE,
            PROG,
            "cache-read-weight must be finite and nonnegative",
        );
    }
    if command == "show" {
        let path = home.join("agents").join(sheet::filename(
            &positionals[0],
            positionals.get(1).map(String::as_str),
        ));
        if !path.exists() {
            return wrong(
                stderr,
                USAGE,
                PROG,
                "no sheet for that provider/model; build first",
            );
        }
        return finish(read_text(&path).map(Some), stdout, stderr);
    }
    let hydra = std::env::var("HYDRA_BIN").unwrap_or_else(|_| "hydra".into());
    let env = Env::process();
    let done = (|| -> Result<Option<String>> {
        // The data directory `hydra-puppets` reads, by the one rule of `data.rs`.
        let data = crate::data::dir(&env)?;
        let aliases = crate::data::aliases_in(&data)?;
        let vendors = vendors.unwrap_or_else(|| data.join("vendors.json"));
        if command == "build" {
            let vendors = records::read_json(&vendors)?.unwrap_or(Value::Null);
            let sheets = sheet::build(&runs, &home, &vendors, &hydra, weight, &aliases, &env)?;
            Ok(Some(
                json::dumps_default(&sheet::object([(
                    "agents",
                    Value::Int(sheets.len() as i128),
                )])) + "\n",
            ))
        } else {
            sheet::revise_budget(
                &sheet::Budget {
                    run_id: &positionals[0],
                    runs: &runs,
                    home: &home,
                    vendors: &vendors,
                    hydra: &hydra,
                    weight,
                    since: since.as_deref(),
                },
                &aliases,
                &env,
            )
            .map(|line| line.map(|line| line + "\n"))
        }
    })();
    finish(done, stdout, stderr)
}
fn finish(done: Result<Option<String>>, stdout: &mut dyn Write, stderr: &mut dyn Write) -> u8 {
    match done {
        Ok(Some(text)) => {
            let _ = write!(stdout, "{text}");
            0
        }
        Ok(None) => 0,
        Err(error) => {
            let _ = writeln!(stderr, "{error}");
            1
        }
    }
}
