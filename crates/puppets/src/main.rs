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

/// The reader recurses once per level of nesting in a log value, to parse it, copy it, print
/// it and drop it, and a goal log line may nest as deep as `json::MAX_NESTING`. The main
/// thread's 8 MiB does not hold that, so the command runs on a thread with room for it. The
/// stack is address space, not memory, until a line that deep is read.
const STACK_BYTES: usize = 1 << 30;

fn main() -> ExitCode {
    let cli = Cli::parse();
    std::thread::Builder::new()
        .stack_size(STACK_BYTES)
        .spawn(move || command(cli))
        .and_then(|thread| {
            thread
                .join()
                .map_err(|_| std::io::Error::other("the command panicked"))
        })
        .unwrap_or_else(|err| {
            eprintln!("hydra-puppets: {err}");
            ExitCode::from(1)
        })
}

fn command(cli: Cli) -> ExitCode {
    let mut out = std::io::stdout().lock();
    let mut err = std::io::stderr().lock();
    let code = match run(cli, &mut out, &mut err) {
        Ok(code) => code,
        Err(e) => {
            let _ = out.flush();
            if let Some(damaged) = e.downcast_ref::<log::DamagedLog>() {
                let _ = writeln!(err, "{damaged}");
            } else if let Some(unterminated) = e.downcast_ref::<log::UnterminatedLog>() {
                let _ = writeln!(err, "{unterminated}");
            } else {
                let _ = writeln!(err, "hydra-puppets: {e:#}");
            }
            1
        }
    };
    let _ = out.flush();
    ExitCode::from(u8::try_from(code).unwrap_or(1))
}
