//! `hydra-puppets`: the commands of `puppets.py` that read a goal log or write one without
//! launching, with its arguments, its output and its exit codes.

use std::ffi::OsString;
use std::io::Write;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use hydra_puppets::log::{self, Home};
use hydra_puppets::read;
use hydra_puppets::write::{self, Approve, arg};

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
    /// start a goal
    #[command(args_override_self = true)]
    New {
        /// goal description
        goal_text: OsString,
    },
    /// human assignment override
    #[command(args_override_self = true)]
    Set {
        /// goal id
        goal: String,
        /// task id
        task: OsString,
        #[arg(long, allow_negative_numbers = true)]
        provider: OsString,
        #[arg(long, default_value = "", allow_negative_numbers = true)]
        model: OsString,
        #[arg(long, allow_negative_numbers = true)]
        policy: OsString,
    },
    /// record human approval
    #[command(args_override_self = true)]
    Approve {
        /// goal id
        goal: String,
        #[arg(long)]
        plan: bool,
        #[arg(long)]
        table: bool,
        #[arg(long, value_name = "TASK", allow_negative_numbers = true)]
        launch: Option<OsString>,
        #[arg(long, value_name = "TASK", allow_negative_numbers = true)]
        diff: Option<OsString>,
        /// approve a degraded panel when a seat's provider is unavailable
        #[arg(long, value_name = "MISSING=SUBSTITUTE", allow_negative_numbers = true)]
        degraded: Option<OsString>,
        /// refuse unless the goal is still at this revision
        #[arg(long, value_parser = py_int, allow_negative_numbers = true)]
        expect_revision: Option<i128>,
        /// with --diff, refuse unless the task's current run is this id
        #[arg(long, allow_negative_numbers = true)]
        expect_run: Option<OsString>,
        /// with --plan, refuse unless the chosen plan's path is this path
        #[arg(long, allow_negative_numbers = true)]
        expect_plan: Option<OsString>,
    },
    /// record human verdict on a diff
    #[command(args_override_self = true)]
    Verdict {
        /// goal id
        goal: String,
        /// task id
        task: OsString,
        #[arg(value_parser = ["applied", "fixed", "rejected"])]
        result: String,
        #[arg(long, default_value = "", allow_negative_numbers = true)]
        note: OsString,
    },
    /// record panel verdict
    #[command(args_override_self = true)]
    Choose {
        /// goal id
        goal: String,
        /// provider name or plan file path (omit with --judge)
        choice: Option<OsString>,
        /// launch the panel to pick among finished proposals
        #[arg(long)]
        judge: bool,
    },
    /// import tasks from chosen plan
    Tasks {
        /// goal id
        goal: String,
    },
    /// assign providers from sheet
    Assign {
        /// goal id
        goal: String,
    },
    /// settle a launch intent that has no run
    #[command(args_override_self = true)]
    Reconcile {
        /// goal id
        goal: String,
        /// task id
        task: OsString,
        /// attempt number from the intent
        #[arg(value_parser = py_int, allow_negative_numbers = true)]
        attempt: i128,
        /// the run hydra status shows for this attempt
        #[arg(
            long,
            conflicts_with = "abandon",
            required_unless_present = "abandon",
            allow_negative_numbers = true
        )]
        run_id: Option<OsString>,
        /// no run was ever started
        #[arg(long)]
        abandon: bool,
    },
}

/// `int(text)` as argparse's `type=int` applies it: whitespace around the number, a sign,
/// and single underscores between digits. An int past `i128` is refused here, where Python
/// has no limit; no revision or attempt is that wide.
fn py_int(text: &str) -> Result<i128, String> {
    let invalid = || format!("invalid int value: '{text}'");
    let body = text.trim_matches(|c: char| c.is_whitespace() || ('\u{1c}'..='\u{1f}').contains(&c));
    let (negative, number) = match body.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, body.strip_prefix('+').unwrap_or(body)),
    };
    let well_formed = number
        .split('_')
        .all(|run| !run.is_empty() && run.bytes().all(|b| b.is_ascii_digit()));
    if !well_formed {
        return Err(invalid());
    }
    let sign = if negative { "-" } else { "" };
    format!("{sign}{}", number.replace('_', ""))
        .parse()
        .map_err(|_| invalid())
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
        Command::New { goal_text } => write::new(&home, &arg(&goal_text), out),
        Command::Set {
            goal,
            task,
            provider,
            model,
            policy,
        } => write::set(
            &home,
            &goal,
            &arg(&task),
            &arg(&provider),
            &arg(&model),
            &arg(&policy),
        ),
        Command::Approve {
            goal,
            plan,
            table,
            launch,
            diff,
            degraded,
            expect_revision,
            expect_run,
            expect_plan,
        } => {
            let text = |option: Option<OsString>| option.map(|value| arg(&value));
            let args = Approve {
                plan,
                table,
                launch: text(launch),
                diff: text(diff),
                degraded: text(degraded),
                expect_revision,
                expect_run: text(expect_run),
                expect_plan: text(expect_plan),
            };
            write::approve(&home, &goal, &args, err)
        }
        Command::Verdict {
            goal,
            task,
            result,
            note,
        } => write::verdict(&home, &goal, &arg(&task), &result, &arg(&note), err),
        Command::Choose {
            goal,
            choice,
            judge,
        } => write::choose(&home, &goal, choice.as_deref(), judge, err),
        Command::Tasks { goal } => write::tasks(&home, &goal, err),
        Command::Assign { goal } => write::assign(&home, &goal, out, err),
        Command::Reconcile {
            goal,
            task,
            attempt,
            run_id,
            abandon: _,
        } => {
            let run_id = run_id.map(|run_id| arg(&run_id));
            write::reconcile(&home, &goal, &arg(&task), attempt, run_id.as_ref(), err)
        }
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
