//! The command line of `apprentice.py`, as `argparse` reads it, for the one command ported.
//!
//! `bank build [--n N]` runs here. `attempt`, `score` and `pairs` are commands the parser
//! knows and the Rust does not run yet: each exits 2 and names the Python that still does
//! (build tickets 232 and 233). The rules where a caller can tell are `argparse`'s: an
//! option may be cut to any prefix that names one option and takes its value after a space
//! or an `=`; an option a parser does not know waits and is reported, with any surplus
//! argument, as unrecognized once the rest parsed; the first `--` ends the options. A wrong
//! command line exits 2 with the usage of the parser that refused it, then
//! `PROG: error: MESSAGE`. The help texts are what CPython 3.14.8 printed at 80 columns.

use std::ffi::OsString;
use std::io::Write;

use hydra_pyjson::json::Value;
use hydra_pyjson::py;

use crate::bank;

const PROG: &str = "hydra-apprentice";
const COMMANDS: [&str; 4] = ["bank", "attempt", "score", "pairs"];

const TOP_USAGE: &str = "usage: hydra-apprentice [-h] {bank,attempt,score,pairs} ...";
const TOP_HELP: &str = "usage: hydra-apprentice [-h] {bank,attempt,score,pairs} ...

The apprentice: a bank of tasks about run records, the local model's attempts,
and the pairs (stdlib only). The only network call goes to
`HYDRA_LOCAL_LLM_URL`, an Ollama on this machine. Nothing trains on the pairs
yet (build 160).

positional arguments:
  {bank,attempt,score,pairs}
    bank                the task bank
    attempt             ask the local model each task it has not attempted at
                        this prompt
    score               the verifiable score per model and template
    pairs               turn a graded file into pairs.jsonl

options:
  -h, --help            show this help message and exit
";
const BANK_USAGE: &str = "usage: hydra-apprentice bank [-h] {build} ...";
const BANK_HELP: &str = "usage: hydra-apprentice bank [-h] {build} ...

positional arguments:
  {build}
    build     write bank.jsonl from the run records

options:
  -h, --help  show this help message and exit
";
const BUILD_USAGE: &str = "usage: hydra-apprentice bank build [-h] [--n N]";
const BUILD_HELP: &str = "usage: hydra-apprentice bank build [-h] [--n N]

options:
  -h, --help  show this help message and exit
  --n N
";

/// `repr(text)`, which is how `argparse` quotes a value it refuses.
fn repr(text: &str) -> String {
    py::repr(&Value::text(text))
}

/// What `argparse` takes for a negative number, which is an argument and not an option.
fn negative_number(arg: &str) -> bool {
    let rest = arg.strip_prefix('-').unwrap_or("x");
    let rest = rest.strip_prefix('.').unwrap_or(rest);
    rest.starts_with(|c: char| c.is_ascii_digit())
}

/// What one argument is to a parser whose long options are `longs`, `--help` among them.
enum Kind {
    Positional,
    Help,
    /// `--n`, and the value written after its `=`, if any.
    Valued(Option<String>),
    /// The message after `error: `.
    Wrong(String),
    /// It looks like an option and the parser has none by that name.
    Unknown,
}

fn kind(arg: &str, longs: &[&str]) -> Kind {
    if !arg.starts_with('-') || arg.len() == 1 {
        return Kind::Positional;
    }
    if arg == "-h" {
        return Kind::Help;
    }
    if let Some(explicit) = arg.strip_prefix("-h")
        && !arg.starts_with("--")
    {
        return Kind::Wrong(format!(
            "argument -h/--help: ignored explicit argument {}",
            repr(explicit)
        ));
    }
    if arg.starts_with("--") {
        let (head, explicit) = match arg.split_once('=') {
            Some((head, value)) => (head, Some(value.to_string())),
            None => (arg, None),
        };
        let matches: Vec<&str> = match longs.iter().find(|long| **long == head) {
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
            [_] => return Kind::Valued(explicit),
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

/// A wrong command line: the usage, the error, and exit code 2.
fn wrong(stderr: &mut dyn Write, usage: &str, prog: &str, message: &str) -> u8 {
    let _ = writeln!(stderr, "{usage}\n{prog}: error: {message}");
    2
}

fn quoted(names: &[&str]) -> String {
    let names: Vec<String> = names.iter().map(|name| format!("'{name}'")).collect();
    names.join(", ")
}

/// How a parser with subcommands ended: its help or an error, as an exit code, or the
/// command chosen and the arguments after it.
type Chosen<'a> = Result<(&'a str, &'a [String]), u8>;

/// A parser whose one positional is a subcommand named `dest`: `-h`, then the command.
/// An option it does not know goes to `extras`, to be reported once the rest parsed.
fn subcommand<'a>(
    args: &'a [String],
    (usage, help, prog): (&str, &str, &str),
    dest: &str,
    choices: &[&str],
    extras: &mut Vec<String>,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> Chosen<'a> {
    let mut rest = args;
    loop {
        let Some((first, after)) = rest.split_first() else {
            return Err(wrong(
                stderr,
                usage,
                prog,
                &format!("the following arguments are required: {dest}"),
            ));
        };
        rest = after;
        if first == "--" {
            continue;
        }
        match kind(first, &["--help"]) {
            Kind::Positional => {
                if !choices.contains(&first.as_str()) {
                    let message = format!(
                        "argument {dest}: invalid choice: {} (choose from {})",
                        repr(first),
                        quoted(choices)
                    );
                    return Err(wrong(stderr, usage, prog, &message));
                }
                return Ok((first.as_str(), rest));
            }
            Kind::Help => {
                let _ = write!(stdout, "{help}");
                return Err(0);
            }
            Kind::Wrong(message) => return Err(wrong(stderr, usage, prog, &message)),
            Kind::Valued(_) | Kind::Unknown => extras.push(first.clone()),
        }
    }
}

/// `bank build`'s own options: `--n N`, the last one given winning. Positionals and
/// unknown options go to `extras`.
fn build_options(args: &[String], extras: &mut Vec<String>) -> Result<Value, Result<(), String>> {
    let prog_longs = ["--help", "--n"];
    let mut n = Value::Int(20);
    let mut only_positionals = false;
    let mut i = 0;
    while i < args.len() {
        let arg = args[i].as_str();
        i += 1;
        if !only_positionals && arg == "--" {
            // `bank build` has no positional, so `argparse` keeps the separator among what
            // it does not recognize.
            only_positionals = true;
            extras.push(arg.to_string());
            continue;
        }
        let found = if only_positionals {
            Kind::Positional
        } else {
            kind(arg, &prog_longs)
        };
        match found {
            Kind::Positional | Kind::Unknown => extras.push(arg.to_string()),
            Kind::Help => return Err(Ok(())),
            Kind::Wrong(message) => return Err(Err(message)),
            Kind::Valued(explicit) => {
                let value = match explicit {
                    Some(value) => value,
                    None => match args.get(i) {
                        Some(next)
                            if next != "--"
                                && matches!(kind(next, &prog_longs), Kind::Positional) =>
                        {
                            i += 1;
                            next.clone()
                        }
                        _ => return Err(Err("argument --n: expected one argument".into())),
                    },
                };
                n = py::int_value(&value).ok_or_else(|| {
                    Err(format!("argument --n: invalid int value: {}", repr(&value)))
                })?;
            }
        }
    }
    Ok(n)
}

/// `main`: the exit code of `apprentice.py` given `args`, the arguments after the program.
pub fn main(
    args: &[String],
    env: &dyn Fn(&str) -> Option<OsString>,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> u8 {
    let mut extras = Vec::new();
    let (command, rest) = match subcommand(
        args,
        (TOP_USAGE, TOP_HELP, PROG),
        "command",
        &COMMANDS,
        &mut extras,
        stdout,
        stderr,
    ) {
        Ok(chosen) => chosen,
        Err(code) => return code,
    };
    if command != "bank" {
        let ticket = if command == "pairs" { 233 } else { 232 };
        let _ = writeln!(
            stderr,
            "{PROG}: {command} is not ported yet (build ticket {ticket}); \
             run integrations/apprentice/apprentice.py {command}"
        );
        return 2;
    }
    let bank_prog = format!("{PROG} bank");
    let (_, rest) = match subcommand(
        rest,
        (BANK_USAGE, BANK_HELP, &bank_prog),
        "action",
        &["build"],
        &mut extras,
        stdout,
        stderr,
    ) {
        Ok(chosen) => chosen,
        Err(code) => return code,
    };
    let n = match build_options(rest, &mut extras) {
        Ok(n) => n,
        Err(Ok(())) => {
            let _ = write!(stdout, "{BUILD_HELP}");
            return 0;
        }
        Err(Err(message)) => {
            return wrong(stderr, BUILD_USAGE, &format!("{bank_prog} build"), &message);
        }
    };
    if !extras.is_empty() {
        return wrong(
            stderr,
            TOP_USAGE,
            PROG,
            &format!("unrecognized arguments: {}", extras.join(" ")),
        );
    }
    match bank::bank_build(&n, env, stdout, stderr) {
        Ok(()) => 0,
        Err(err) => {
            let _ = writeln!(stderr, "{err}");
            1
        }
    }
}
