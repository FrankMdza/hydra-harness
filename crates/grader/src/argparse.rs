//! The reading of `argparse` every command of this binary shares (build ticket 220 made it
//! one copy; `baseline_cli.rs`, `monitor_cli.rs` and `collect_cli.rs` each had their own).
//!
//! The rules, where a caller can tell: an option may be cut short to any prefix that names
//! one option, takes its value after a space or an `=`, and may sit before, between or
//! after the positionals; the first `--` ends the options; a surplus argument is
//! "unrecognized" and reported after everything else parsed. A wrong command line exits 2
//! with a usage line and then `PROG: error: MESSAGE`, and the message is the Python's.

use std::io::Write;

use hydra_pyjson::json::Value;
use hydra_pyjson::py;

/// `repr(text)`, which is how `argparse` quotes a value it refuses.
pub(crate) fn repr(text: &str) -> String {
    py::repr(&Value::text(text))
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
            // The first `--` ends the options. `argparse` drops it while it is still
            // filling positionals, and keeps it among what it does not recognize once a
            // surplus argument came before it or the command has no positional at all. A
            // second `--` is an argument like any other.
            only_positionals = true;
            if places == 0 || !extras.is_empty() {
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

/// The top parser of a program whose first positional is one of `commands`: `-h`, then the
/// command. An option it does not know waits until the command has parsed and is then
/// unrecognized, so it is handed back with the command and what follows it. `Err` is the
/// exit code of a command line that ended here: the help, or a wrong one.
pub(crate) fn command<'a>(
    args: &'a [String],
    usage: &str,
    prog: &str,
    commands: &[&str],
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> Result<(&'a str, &'a [String], Vec<String>), u8> {
    let mut top_extras = Vec::new();
    let mut rest = args;
    let command = loop {
        let Some((first, after)) = rest.split_first() else {
            return Err(wrong(
                stderr,
                usage,
                prog,
                "the following arguments are required: command",
            ));
        };
        rest = after;
        if first == "--" {
            continue;
        }
        match kind(first, &["--help"]) {
            Kind::Positional => {}
            Kind::Help => {
                let _ = writeln!(stdout, "{usage}");
                return Err(0);
            }
            Kind::Wrong(message) => return Err(wrong(stderr, usage, prog, &message)),
            Kind::Option(..) | Kind::Unknown => {
                top_extras.push(first.clone());
                continue;
            }
        }
        break first.as_str();
    };
    if !commands.contains(&command) {
        let message = format!(
            "argument command: invalid choice: {} (choose from {})",
            repr(command),
            quoted(commands)
        );
        return Err(wrong(stderr, usage, prog, &message));
    }
    Ok((command, rest, top_extras))
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

    fn left_over(args: &[&str], places: usize) -> (Vec<String>, Vec<String>) {
        let args: Vec<String> = args.iter().map(|arg| arg.to_string()).collect();
        let mut take = |_: &'static str, _: &str| -> Result<(), String> { Ok(()) };
        match parse(&args, &["--jobs"], places, &mut take) {
            Parsed::Args {
                positionals,
                extras,
            } => (positionals, extras),
            _ => panic!("{args:?} did not parse"),
        }
    }

    /// What `baseline.py score`, `baseline.py train` and `grader.py once` said of each line
    /// under CPython 3.14.7 on 2026-10-01, before the Python was deleted.
    #[test]
    fn the_first_separator_is_kept_only_where_argparse_keeps_it() {
        let cases: [(&[&str], usize, &str); 14] = [
            (&["m", "r", "--", "x"], 2, "x"),
            (&["m", "--", "r", "x"], 2, "x"),
            (&["--", "m", "r"], 2, ""),
            (&["m", "r", "--"], 2, ""),
            (&["m", "r", "x", "--", "y"], 2, "x -- y"),
            (&["x", "y", "z", "--", "w"], 2, "z -- w"),
            (&["m", "--", "r", "--", "x"], 2, "-- x"),
            (&["--", "--", "m", "r"], 2, "r"),
            (&["m", "--", "--", "r"], 2, "r"),
            (&["d", "m", "--", "--label"], 2, "--label"),
            (&["--", "x"], 0, "-- x"),
            (&["--"], 0, "--"),
            (&["x"], 0, "x"),
            (&["--jobs", "2", "--", "--jobs"], 0, "-- --jobs"),
        ];
        for (args, places, extras) in cases {
            assert_eq!(left_over(args, places).1.join(" "), extras, "{args:?}");
        }
        assert_eq!(left_over(&["--", "--", "m", "r"], 2).0, ["--", "m"]);
    }
}
