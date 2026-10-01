//! `hydra-puppets`: the read commands of `puppets.py`, with its arguments and its output.

use std::io::Write;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use hydra_puppets::log::{self, Home};
use hydra_puppets::read;

#[derive(Parser)]
#[command(
    name = "hydra-puppets",
    about = "Master of Puppets goal log",
    infer_long_args = true
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// the goal at a glance
    Board {
        /// goal id
        goal: String,
    },
    /// fold and print goal state
    Show {
        /// goal id
        goal: String,
    },
    /// approvals the goal is waiting on, as JSON
    Pending {
        /// goal id
        goal: String,
        /// print a JSON array of the approvals still waiting
        #[arg(long, required = true)]
        json: bool,
    },
    /// tasks waiting on a review, highest verdict p first
    Queue {
        /// goal id
        goal: String,
    },
    /// save an unterminated tail and truncate the log to its last newline
    Repair {
        /// goal id
        goal: String,
        /// move the tail aside and truncate; without this flag, print what it would do and
        /// write nothing
        #[arg(long)]
        yes: bool,
    },
}

fn run(cli: Cli, out: &mut dyn Write, err: &mut dyn Write) -> anyhow::Result<i32> {
    let home = Home::from_env()?;
    match cli.command {
        Command::Board { goal } => read::board(&home, &goal, out, err),
        Command::Show { goal } => read::show(&home, &goal, out),
        Command::Pending { goal, json: _ } => {
            let script = log::python_dir().join("puppets.py");
            read::pending(&home, &goal, "python3", &script.to_string_lossy(), out, err)
        }
        Command::Queue { goal } => read::queue(&home, &goal, out, err),
        Command::Repair { goal, yes } => log::repair(&home, &goal, yes, out),
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let mut out = std::io::stdout().lock();
    let mut err = std::io::stderr().lock();
    let code = match run(cli, &mut out, &mut err) {
        Ok(code) => code,
        Err(e) => {
            let _ = out.flush();
            let _ = writeln!(err, "hydra-puppets: {e:#}");
            1
        }
    };
    let _ = out.flush();
    ExitCode::from(u8::try_from(code).unwrap_or(1))
}
