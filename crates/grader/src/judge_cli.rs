//! `hydra-grader judge label` and `agree`: the arguments of `judge.py` and its two commands
//! (build ticket 218).
//!
//! The arguments follow `argparse` by the rules `argparse.rs` spells out. Neither
//! command takes a positional, so `argparse` keeps a `--` and whatever follows it among the
//! arguments it does not recognize, as it does for `triage`. `--only-unlabelled` is the
//! crate's first option without a value, so `label` reads its own arguments here.

use std::io::Write;

use hydra_pyjson::py;

use crate::argparse::{Kind, kind, quoted, repr, wrong};
use crate::judge;
use crate::labels::Env;

const PROG: &str = "hydra-grader judge";
const COMMANDS: [&str; 2] = ["label", "agree"];
const USAGE: &str = "usage: hydra-grader judge [-h] {label,agree} ...";
const LABEL_USAGE: &str =
    "usage: hydra-grader judge label [-h] [--limit LIMIT] [--only-unlabelled]";
const AGREE_USAGE: &str = "usage: hydra-grader judge agree [-h]";

/// `judge.py`'s `main` on the arguments after `judge`. Returns the exit code.
pub fn main(args: &[String], env: &Env, stdout: &mut dyn Write, stderr: &mut dyn Write) -> u8 {
    let mut rest = args;
    let mut extras = Vec::new();
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
            Kind::Positional => break first.as_str(),
            Kind::Option(..) | Kind::Unknown => extras.push(first.clone()),
        }
    };
    if !COMMANDS.contains(&command) {
        let message = format!(
            "argument command: invalid choice: {} (choose from {})",
            repr(command),
            quoted(&COMMANDS)
        );
        return wrong(stderr, USAGE, PROG, &message);
    }
    let labelling = command == "label";
    let prog = format!("{PROG} {command}");
    let usage = if labelling { LABEL_USAGE } else { AGREE_USAGE };
    let longs: &[&'static str] = if labelling {
        &["--help", "--limit", "--only-unlabelled"]
    } else {
        &["--help"]
    };

    let marker = rest
        .iter()
        .position(|arg| arg == "--")
        .unwrap_or(rest.len());
    let (mut limit, mut only_unlabelled) = (None, false);
    let mut at = 0;
    while at < marker {
        let arg = rest[at].as_str();
        at += 1;
        match kind(arg, longs) {
            Kind::Positional | Kind::Unknown => extras.push(arg.to_string()),
            Kind::Help => {
                let _ = writeln!(stdout, "{usage}");
                return 0;
            }
            Kind::Wrong(message) => return wrong(stderr, usage, &prog, &message),
            Kind::Option("--limit", explicit) => {
                let value = match explicit {
                    Some(value) => value,
                    None => match rest[..marker].get(at) {
                        Some(next) if matches!(kind(next, longs), Kind::Positional) => {
                            at += 1;
                            next.clone()
                        }
                        _ => {
                            let message = "argument --limit: expected one argument";
                            return wrong(stderr, usage, &prog, message);
                        }
                    },
                };
                let Some(number) = py::int_value(&value) else {
                    let message = format!("argument --limit: invalid int value: {}", repr(&value));
                    return wrong(stderr, usage, &prog, &message);
                };
                limit = Some(number);
            }
            Kind::Option(long, Some(value)) => {
                let message = format!(
                    "argument {long}: ignored explicit argument {}",
                    repr(&value)
                );
                return wrong(stderr, usage, &prog, &message);
            }
            Kind::Option(..) => only_unlabelled = true,
        }
    }
    extras.extend_from_slice(&rest[marker..]);
    if !extras.is_empty() {
        let message = format!("unrecognized arguments: {}", extras.join(" "));
        return wrong(stderr, USAGE, PROG, &message);
    }

    let done = if labelling {
        judge::label(limit.as_ref(), only_unlabelled, env, stdout, stderr)
    } else {
        judge::agree(env).map(|printed| {
            let _ = write!(stdout, "{printed}");
            0
        })
    };
    done.unwrap_or_else(|err| {
        // Where the Python ends in a traceback, whose last line starts with the same class.
        let _ = writeln!(stderr, "{err}");
        1
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(args: &[&str]) -> (u8, String, String) {
        let args: Vec<String> = args.iter().map(|arg| arg.to_string()).collect();
        let env = Env::from_pairs([("HYDRA_GRADER_HOME", "/nonexistent/hydra-grader-judge")]);
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let code = main(&args, &env, &mut out, &mut err);
        (
            code,
            String::from_utf8(out).unwrap(),
            String::from_utf8(err).unwrap(),
        )
    }

    #[test]
    fn a_flag_takes_no_value_and_a_limit_takes_an_int() {
        let (code, _, err) = run(&["label", "--only-unlabelled=1"]);
        assert_eq!(code, 2);
        assert!(err.ends_with("ignored explicit argument '1'\n"), "{err}");
        let (code, _, err) = run(&["label", "--limit", "two"]);
        assert_eq!(code, 2);
        assert!(err.ends_with("invalid int value: 'two'\n"), "{err}");
        let (code, _, err) = run(&["agree", "--limit", "2"]);
        assert_eq!(code, 2);
        assert!(
            err.ends_with("unrecognized arguments: --limit 2\n"),
            "{err}"
        );
    }

    #[test]
    fn no_dataset_and_no_labels_agree_on_nothing() {
        let (code, out, _) = run(&["agree"]);
        assert_eq!(code, 0);
        assert!(
            out.starts_with("runs with both: 0\nagree: 0\nagreement: none"),
            "{out}"
        );
    }
}
