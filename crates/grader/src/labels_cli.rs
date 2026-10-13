//! `hydra-grader labels count` (build ticket 303): the label rows every goal log under the
//! puppets home gives, counted per class. It has no Python original.

use std::io::Write;

use crate::argparse::{self, Parsed, parse, wrong};
use crate::labels::{self, Env};

const PROG: &str = "hydra-grader labels";
const COMMANDS: [&str; 1] = ["count"];
const USAGE: &str = "usage: hydra-grader labels [-h] {count} ...";
const COUNT_USAGE: &str = "usage: hydra-grader labels count [-h]";

/// The arguments after `labels`. Returns the exit code: 1 when a goal log cannot be shown.
pub fn main(args: &[String], env: &Env, stdout: &mut dyn Write, stderr: &mut dyn Write) -> u8 {
    let (_, rest, mut extras) =
        match argparse::command(args, USAGE, PROG, &COMMANDS, stdout, stderr) {
            Ok(read) => read,
            Err(code) => return code,
        };
    match parse(rest, &[], 0, &mut |_, _| Ok(())) {
        Parsed::Help => {
            let _ = writeln!(stdout, "{COUNT_USAGE}");
            return 0;
        }
        Parsed::Wrong(message) => {
            return wrong(stderr, COUNT_USAGE, &format!("{PROG} count"), &message);
        }
        Parsed::Args {
            positionals,
            extras: more,
        } => {
            extras.extend(positionals);
            extras.extend(more);
        }
    }
    if !extras.is_empty() {
        let message = format!("unrecognized arguments: {}", extras.join(" "));
        return wrong(stderr, USAGE, PROG, &message);
    }
    let counted = labels::puppets_home(env)
        .and_then(|home| labels::collect(&home, env, stderr))
        .and_then(|found| labels::count(&found, stdout));
    match counted {
        Ok(()) => 0,
        Err(err) => {
            let _ = writeln!(stderr, "{PROG}: {err}");
            1
        }
    }
}
