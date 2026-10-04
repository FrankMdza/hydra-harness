//! The command line of `apprentice.py`, as `argparse` reads it, for the commands ported.
//!
//! `bank build [--n N]`, `attempt [--limit N] [--minutes M]`, `score [--model NAME]` and
//! `pairs GRADED` run here. The rules where a caller can tell are `argparse`'s: an
//! option may be cut to any prefix that names one option and takes its value after a space
//! or an `=`; `-h` followed by more letters is `-h` and those letters as further short
//! options, so it asks for help; an option a parser does not know waits and is reported,
//! with any surplus argument, as unrecognized once the rest parsed; the first `--` ends the
//! options, and `pairs` drops it where it sits next to the one argument it takes. A wrong
//! command line exits 2 with the usage of the parser that refused it, then
//! `PROG: error: MESSAGE`. The help texts are what CPython 3.14.8 printed at 80 columns.

use std::ffi::OsString;
use std::io::Write;

use hydra_pyjson::json::Value;
use hydra_pyjson::py;

use crate::{attempt, bank, pairs, score};

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
const ATTEMPT_USAGE: &str =
    "usage: hydra-apprentice attempt [-h] [--limit LIMIT] [--minutes MINUTES]";
const ATTEMPT_HELP: &str =
    "usage: hydra-apprentice attempt [-h] [--limit LIMIT] [--minutes MINUTES]

options:
  -h, --help         show this help message and exit
  --limit LIMIT
  --minutes MINUTES
";
const SCORE_USAGE: &str = "usage: hydra-apprentice score [-h] [--model MODEL]";
const SCORE_HELP: &str = "usage: hydra-apprentice score [-h] [--model MODEL]

options:
  -h, --help     show this help message and exit
  --model MODEL  print only this model's block
";

const PAIRS_USAGE: &str = "usage: hydra-apprentice pairs [-h] graded";
const PAIRS_HELP: &str = "usage: hydra-apprentice pairs [-h] graded

positional arguments:
  graded

options:
  -h, --help  show this help message and exit
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
    if let Some(mut tail) = arg.strip_prefix("-h")
        && !arg.starts_with("--")
    {
        // Each letter after `-h` is read as another short option, and help runs once they
        // are all read. Only `-h` itself is one, so the first other letter ends the reading,
        // and an `=` or a `-` where a letter should be is refused.
        loop {
            let ignored = tail
                .strip_prefix('=')
                .or(tail.starts_with('-').then_some(tail));
            if let Some(explicit) = ignored {
                return Kind::Wrong(format!(
                    "argument -h/--help: ignored explicit argument {}",
                    repr(explicit)
                ));
            }
            match tail.strip_prefix('h') {
                Some(rest) => tail = rest,
                None => return Kind::Help,
            }
        }
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

/// How a subcommand's option turns its text into a value, and the name `argparse` gives
/// the type in its error.
#[derive(Clone, Copy)]
enum Type {
    Int,
    Float,
    Text,
}

impl Type {
    fn convert(self, text: &str) -> Option<Value> {
        match self {
            Type::Int => py::int_value(text),
            Type::Float => py::float_literal(text).map(Value::Float),
            Type::Text => Some(Value::text(text)),
        }
    }

    fn name(self) -> &'static str {
        match self {
            Type::Int => "int",
            Type::Float => "float",
            Type::Text => "str",
        }
    }
}

/// A subcommand's own options, `--help` first: each is given or not, the last one given
/// winning. Positionals and unknown options go to `extras`. `Err(Ok(()))` is a request for
/// help and `Err(Err(message))` a refusal.
fn options(
    args: &[String],
    longs: &[(&str, Type)],
    extras: &mut Vec<String>,
) -> Result<Vec<Option<Value>>, Result<(), String>> {
    let names: Vec<&str> = std::iter::once("--help")
        .chain(longs.iter().map(|(name, _)| *name))
        .collect();
    let mut given = vec![None; longs.len()];
    let mut only_positionals = false;
    let mut i = 0;
    while i < args.len() {
        let arg = args[i].as_str();
        i += 1;
        if !only_positionals && arg == "--" {
            // The subcommand has no positional, so `argparse` keeps the separator among
            // what it does not recognize.
            only_positionals = true;
            extras.push(arg.to_string());
            continue;
        }
        let found = if only_positionals {
            Kind::Positional
        } else {
            kind(arg, &names)
        };
        match found {
            Kind::Positional | Kind::Unknown => extras.push(arg.to_string()),
            Kind::Help => return Err(Ok(())),
            Kind::Wrong(message) => return Err(Err(message)),
            Kind::Valued(explicit) => {
                let head = arg.split_once('=').map_or(arg, |(head, _)| head);
                let at = longs
                    .iter()
                    .position(|(name, _)| *name == head)
                    .or_else(|| longs.iter().position(|(name, _)| name.starts_with(head)))
                    .expect("kind matched one option");
                let (name, of) = longs[at];
                let value = match explicit {
                    Some(value) => value,
                    None => match args.get(i) {
                        Some(next)
                            if next != "--" && matches!(kind(next, &names), Kind::Positional) =>
                        {
                            i += 1;
                            next.clone()
                        }
                        _ => return Err(Err(format!("argument {name}: expected one argument"))),
                    },
                };
                given[at] = Some(of.convert(&value).ok_or_else(|| {
                    Err(format!(
                        "argument {name}: invalid {} value: {}",
                        of.name(),
                        repr(&value)
                    ))
                })?);
            }
        }
    }
    Ok(given)
}

/// The arguments of `pairs`, which takes one positional, `graded`, and no option but
/// `--help`. Arguments it does not take go to `extras`. `Err(Ok(()))` is a request for help
/// and `Err(Err(message))` a refusal.
///
/// This is `argparse`'s loop for that one parser. Each argument is an option (`O`), an
/// argument (`A`), or the first `--` (`-`), after which every argument is an `A`. Before
/// each option, and once after the last, `graded` takes `-*A-*` if the arguments there match
/// it, and a `--` it took is dropped.
fn graded(args: &[String], extras: &mut Vec<String>) -> Result<String, Result<(), String>> {
    let mut ended = false;
    let pattern: Vec<char> = args
        .iter()
        .map(|arg| match arg.as_str() {
            _ if ended => 'A',
            "--" => {
                ended = true;
                '-'
            }
            arg => match kind(arg, &["--help"]) {
                Kind::Positional => 'A',
                _ => 'O',
            },
        })
        .collect();
    let mut graded: Option<String> = None;
    let consume = |start: usize, graded: &mut Option<String>| {
        if graded.is_some() {
            return start;
        }
        let at = |i: usize| pattern.get(i).copied();
        let mut end = start + usize::from(at(start) == Some('-'));
        if at(end) != Some('A') {
            return start;
        }
        end += 1;
        while at(end) == Some('-') {
            end += 1;
        }
        let mut taken: Vec<&String> = args[start..end].iter().collect();
        if pattern[start..end].contains(&'-') {
            let dashes = taken.iter().position(|arg| *arg == "--");
            taken.remove(dashes.expect("a `-` is a `--`"));
        }
        *graded = Some(taken[0].clone());
        end
    };
    let options: Vec<usize> = (0..args.len()).filter(|i| pattern[*i] == 'O').collect();
    let mut start = 0;
    if let Some(&last) = options.last() {
        while start <= last {
            let next = *options
                .iter()
                .find(|i| **i >= start)
                .expect("start <= last");
            if start != next {
                let end = consume(start, &mut graded);
                if end > start {
                    start = end;
                    continue;
                }
            }
            if pattern[start] != 'O' {
                extras.extend(args[start..next].iter().cloned());
                start = next;
            }
            match kind(&args[start], &["--help"]) {
                Kind::Help => return Err(Ok(())),
                Kind::Wrong(message) => return Err(Err(message)),
                _ => extras.push(args[start].clone()),
            }
            start += 1;
        }
    }
    let stop = consume(start, &mut graded);
    extras.extend(args[stop..].iter().cloned());
    graded.ok_or_else(|| Err("the following arguments are required: graded".into()))
}

/// The surplus arguments, if any, refused as `argparse` refuses them once the rest parsed.
fn unrecognized(extras: &[String], stderr: &mut dyn Write) -> Option<u8> {
    (!extras.is_empty()).then(|| {
        wrong(
            stderr,
            TOP_USAGE,
            PROG,
            &format!("unrecognized arguments: {}", extras.join(" ")),
        )
    })
}

/// The exit code of a command that ran, or 1 with its error on stderr.
fn finished(ran: anyhow::Result<u8>, stderr: &mut dyn Write) -> u8 {
    ran.unwrap_or_else(|err| {
        let _ = writeln!(stderr, "{err}");
        1
    })
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
    if command == "pairs" {
        let graded = match graded(rest, &mut extras) {
            Ok(graded) => graded,
            Err(Ok(())) => {
                let _ = write!(stdout, "{PAIRS_HELP}");
                return 0;
            }
            Err(Err(message)) => {
                return wrong(stderr, PAIRS_USAGE, &format!("{PROG} pairs"), &message);
            }
        };
        if let Some(code) = unrecognized(&extras, stderr) {
            return code;
        }
        let path = std::path::PathBuf::from(bank::pure_path(&graded));
        return finished(pairs::pairs(&path, env, stdout), stderr);
    }
    let bank_prog = format!("{PROG} bank");
    let (rest, longs, usage, help): (&[String], &[(&str, Type)], &str, &str) = match command {
        "bank" => {
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
            (rest, &[("--n", Type::Int)], BUILD_USAGE, BUILD_HELP)
        }
        "attempt" => (
            rest,
            &[("--limit", Type::Int), ("--minutes", Type::Float)],
            ATTEMPT_USAGE,
            ATTEMPT_HELP,
        ),
        _ => (rest, &[("--model", Type::Text)], SCORE_USAGE, SCORE_HELP),
    };
    let given = match options(rest, longs, &mut extras) {
        Ok(given) => given,
        Err(Ok(())) => {
            let _ = write!(stdout, "{help}");
            return 0;
        }
        Err(Err(message)) => {
            // The parser's own name is its usage line up to the first option.
            let prog = usage.trim_start_matches("usage: ");
            let prog = prog.split(" [").next().unwrap_or(prog);
            return wrong(stderr, usage, prog, &message);
        }
    };
    if let Some(code) = unrecognized(&extras, stderr) {
        return code;
    }
    let ran = match command {
        "bank" => {
            let n = given[0].clone().unwrap_or(Value::Int(20));
            bank::bank_build(&n, env, stdout, stderr).map(|()| 0)
        }
        "attempt" => {
            let minutes = match given[1] {
                Some(Value::Float(minutes)) => Some(minutes),
                _ => None,
            };
            attempt::attempt(given[0].as_ref(), minutes, env, stdout, stderr)
        }
        _ => {
            let only = match &given[0] {
                Some(Value::Str(model)) => Some(model.lossy()),
                _ => None,
            };
            score::score(only.as_deref(), env, stdout)
        }
    };
    finished(ran, stderr)
}
