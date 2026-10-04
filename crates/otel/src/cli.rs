//! The command line of `record_to_otlp.py`, as `argparse` reads it.
//!
//! One optional positional `record`, and `--out`, `--all` and `--out-dir`. An option may be
//! cut to any prefix that names one option and takes its value after a space or an `=`. A
//! wrong command line exits 2 with the usage, then `hydra-otel: error: MESSAGE`. The help
//! text is what CPython 3.14.8 printed at 80 columns with this program name.

use std::io::Write;
use std::path::PathBuf;

use anyhow::Result;
use hydra_pyjson::json::Value;
use hydra_pyjson::py;

use crate::convert;

const PROG: &str = "hydra-otel";
const USAGE: &str = "usage: hydra-otel [-h] [--out OUT] [--all RUNS] [--out-dir OUT_DIR] [record]";
const HELP: &str = "\
usage: hydra-otel [-h] [--out OUT] [--all RUNS] [--out-dir OUT_DIR] [record]

Translate Hydra records to local OTLP JSON files, using only the stdlib.

positional arguments:
  record

options:
  -h, --help         show this help message and exit
  --out OUT
  --all RUNS
  --out-dir OUT_DIR
";

fn repr(text: &str) -> String {
    py::repr(&Value::text(text))
}

fn negative_number(arg: &str) -> bool {
    let rest = arg.strip_prefix('-').unwrap_or("x");
    let rest = rest.strip_prefix('.').unwrap_or(rest);
    rest.starts_with(|c: char| c.is_ascii_digit())
}

enum Kind {
    Positional,
    Help,
    Option(&'static str, Option<String>),
    Wrong(String),
    Unknown,
}

fn kind(arg: &str, longs: &[&'static str]) -> Kind {
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

fn wrong(stderr: &mut dyn Write, message: &str) -> u8 {
    let _ = writeln!(stderr, "{USAGE}\n{PROG}: error: {message}");
    2
}

fn as_path(text: &str) -> PathBuf {
    if text.is_empty() {
        PathBuf::from(".")
    } else {
        PathBuf::from(text)
    }
}

struct Args {
    record: Option<PathBuf>,
    out: Option<PathBuf>,
    runs: Option<PathBuf>,
    out_dir: Option<PathBuf>,
}

fn parse(args: &[String], stdout: &mut dyn Write, stderr: &mut dyn Write) -> Result<Args, u8> {
    let longs: [&'static str; 4] = ["--help", "--out", "--all", "--out-dir"];
    let mut record = None;
    let mut out = None;
    let mut runs = None;
    let mut out_dir = None;
    let mut extras = Vec::new();
    let mut only_positionals = false;
    let mut i = 0;
    while i < args.len() {
        let arg = args[i].as_str();
        i += 1;
        if !only_positionals && arg == "--" {
            only_positionals = true;
            if !extras.is_empty() {
                extras.push(arg.to_string());
            }
            continue;
        }
        let found = if only_positionals {
            Kind::Positional
        } else {
            kind(arg, &longs)
        };
        match found {
            Kind::Help => {
                let _ = write!(stdout, "{HELP}");
                return Err(0);
            }
            Kind::Wrong(message) => return Err(wrong(stderr, &message)),
            Kind::Unknown => extras.push(arg.to_string()),
            Kind::Positional if record.is_some() => extras.push(arg.to_string()),
            Kind::Positional => record = Some(as_path(arg)),
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
                            return Err(wrong(
                                stderr,
                                &format!("argument {long}: expected one argument"),
                            ));
                        }
                    },
                };
                let path = as_path(&value);
                match long {
                    "--out" => out = Some(path),
                    "--all" => runs = Some(path),
                    _ => out_dir = Some(path),
                }
            }
        }
    }
    if !extras.is_empty() {
        return Err(wrong(
            stderr,
            &format!("unrecognized arguments: {}", extras.join(" ")),
        ));
    }
    Ok(Args {
        record,
        out,
        runs,
        out_dir,
    })
}

/// `main`: the exit code of `record_to_otlp.py` given `args`, the arguments after the program.
pub fn main(args: &[String], stdout: &mut dyn Write, stderr: &mut dyn Write) -> u8 {
    let parsed = match parse(args, stdout, stderr) {
        Ok(parsed) => parsed,
        Err(code) => return code,
    };
    let ran = if let Some(runs) = parsed.runs {
        if parsed.out_dir.is_none() || parsed.record.is_some() || parsed.out.is_some() {
            return wrong(
                stderr,
                "--all requires --out-dir and cannot be combined with record or --out",
            );
        }
        convert::batch(&runs, &parsed.out_dir.expect("checked"))
    } else {
        if parsed.record.is_none() || parsed.out_dir.is_some() {
            return wrong(stderr, "provide a record, or --all with --out-dir");
        }
        let record = parsed.record.expect("checked");
        match convert::convert(&record) {
            Ok(result) => match parsed.out {
                Some(out) => convert::write_json(&out, &result),
                None => convert::dump_stdout(stdout, &result),
            },
            Err(err) => Err(err),
        }
    };
    match ran {
        Ok(()) => 0,
        Err(err) => {
            let _ = writeln!(stderr, "{err}");
            1
        }
    }
}
