//! `hydra-puppets`: the commands of the Python helper it replaced, with its arguments, its
//! output and its exit codes.

use std::ffi::OsString;
use std::io::Write;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use hydra_puppets::frontier;
use hydra_puppets::json::Value;
use hydra_puppets::launch;
use hydra_puppets::log::{self, Home};
use hydra_puppets::panel;
use hydra_puppets::read;
use hydra_puppets::write::{self, Approve, arg};

#[derive(Debug, Parser)]
#[command(
    name = "hydra-puppets",
    about = "Master of Puppets goal log",
    infer_long_args = true
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
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
        expect_revision: Option<Value>,
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
    /// launch plan proposers
    Plan {
        /// goal id
        goal: String,
        /// provider[:model][@stance], with minimal, risk, contrarian or builder
        #[arg(long, required = true, allow_negative_numbers = true)]
        seat: Vec<OsString>,
    },
    /// copy finished proposals from worktrees
    Plans {
        /// goal id
        goal: String,
    },
    /// launch a task run
    Launch {
        /// goal id
        goal: String,
        /// task id
        task: OsString,
    },
    /// poll pending runs for completion
    Wait {
        /// goal id
        goal: String,
    },
    /// open a task's diff in Hunk
    #[command(args_override_self = true)]
    Review {
        /// goal id
        goal: String,
        /// task id
        task: OsString,
        /// record the review and print the hunk command without opening it
        #[arg(long)]
        no_open: bool,
    },
    /// offer the findings on stdin to the Hunk session on a repository; `review` starts this
    #[command(hide = true)]
    PushHunkComments {
        /// the repository the session is on
        repo: OsString,
    },
    /// which map or build tickets are takeable now
    Frontier {
        /// tracker directory under `.scratch/hydra` (default: `issues`)
        tracker: Option<String>,
        /// read tickets from another directory
        #[arg(long, value_name = "DIR")]
        dir: Option<std::path::PathBuf>,
    },
    /// print a tracker index as Markdown
    Index {
        #[command(subcommand)]
        command: IndexCommand,
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
        attempt: Value,
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

#[derive(Debug, Subcommand)]
enum IndexCommand {
    /// list every numbered ticket in the build directory
    Build {
        /// read tickets from another directory
        #[arg(long, value_name = "DIR")]
        dir: Option<std::path::PathBuf>,
    },
}

/// `int(text)` as argparse's `type=int` applies it, at any width: Python has no widest int,
/// and one past `i128` reaches the command as it does there.
fn py_int(text: &str) -> Result<Value, String> {
    hydra_puppets::int_value(text).ok_or_else(|| format!("invalid int value: '{text}'"))
}

/// This binary, as the `argv` of `pending --json` names it: its canonical path, so the
/// command runs from any directory, or the bare name for `PATH` when the path is unknown.
fn helper_path() -> String {
    std::env::current_exe()
        .and_then(|exe| exe.canonicalize())
        .map_or_else(
            |_| "hydra-puppets".into(),
            |exe| exe.to_string_lossy().into_owned(),
        )
}

fn run(cli: Cli, out: &mut dyn Write, err: &mut dyn Write) -> anyhow::Result<i32> {
    let home = Home::from_env()?;
    match cli.command {
        Command::Board { goal } => read::board(&home, &goal, out, err),
        Command::Show { goal } => read::show(&home, &goal, out),
        Command::Pending { goal, json: _ } => read::pending(&home, &goal, &helper_path(), out, err),
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
        } => write::choose(&home, &goal, choice.as_deref(), judge, out, err),
        Command::Plan { goal, seat } => {
            let seats: Vec<_> = seat.iter().map(|seat| arg(seat)).collect();
            launch::plan(&home, &goal, &seats, out, err)
        }
        Command::Plans { goal } => launch::plans(&home, &goal, out, err),
        Command::Launch { goal, task } => launch::launch(&home, &goal, &arg(&task), out, err),
        Command::Wait { goal } => launch::wait(&home, &goal, out),
        Command::Review {
            goal,
            task,
            no_open,
        } => panel::review(&home, &goal, &arg(&task), no_open, out, err),
        Command::PushHunkComments { repo } => panel::push_comments(&repo),
        Command::Tasks { goal } => write::tasks(&home, &goal, err),
        Command::Assign { goal } => write::assign(&home, &goal, out, err),
        Command::Frontier { tracker, dir } => {
            frontier::run(tracker.as_deref(), dir.as_deref(), out)
        }
        Command::Index {
            command: IndexCommand::Build { dir },
        } => hydra_puppets::index::build(dir.as_deref(), out),
        Command::Reconcile {
            goal,
            task,
            attempt,
            run_id,
            abandon: _,
        } => {
            let run_id = run_id.map(|run_id| arg(&run_id));
            write::reconcile(&home, &goal, &arg(&task), &attempt, run_id.as_ref(), err)
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
            if let Some(unsafe_tmp) = e.downcast_ref::<log::UnsafeRepairTemporary>() {
                let _ = writeln!(err, "{unsafe_tmp}");
            } else if let Some(damaged) = e.downcast_ref::<log::DamagedLog>() {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn revision(text: &str) -> Result<Option<Value>, clap::Error> {
        let argv = [
            "puppets",
            "approve",
            "g1",
            "--plan",
            "--expect-revision",
            text,
        ];
        Cli::try_parse_from(argv).map(|cli| match cli.command {
            Command::Approve {
                expect_revision, ..
            } => expect_revision,
            _ => None,
        })
    }

    /// Each row is what `int()` of Python 3.14 answers.
    #[test]
    fn revision_int_matches_python() {
        let wide = "99999999999999999999999999999999999999999999999999";
        for (input, expected) in [
            ("٧", "7"),
            (" +٧_２\u{a0}", "72"),
            ("\u{3000}-0_7\u{2028}", "-7"),
            (wide, wide),
        ] {
            let value = revision(input).unwrap().unwrap();
            assert_eq!(
                hydra_puppets::json::dumps(&value, None),
                expected,
                "{input:?}"
            );
        }
        for input in [
            "\u{1c}7", "7\u{1f}", "1__2", "1_", "²", "_7", "- 7", "0x7", "",
        ] {
            assert!(revision(input).is_err(), "{input:?}");
        }
    }

    #[test]
    fn revision_rejects_ascii_separator() {
        assert!(py_int("\u{1c}7").is_err());
    }

    #[test]
    fn revision_accepts_wide_integer() {
        let wide = "99999999999999999999999999999999999999999999999999";
        assert!(matches!(py_int(wide), Ok(Value::BigInt(digits)) if digits == wide));
        assert!(matches!(py_int("7"), Ok(Value::Int(7))));
    }

    /// One full command line per command that takes an option, the options last.
    const LINES: [&[&str]; 8] = [
        &["plan", "g1", "--seat=x"],
        &["choose", "g1", "--judge"],
        &["set", "g1", "t1", "--provider=x", "--model=x", "--policy=x"],
        &[
            "approve",
            "g1",
            "--plan",
            "--table",
            "--launch=x",
            "--diff=x",
            "--degraded=x",
            "--expect-revision=7",
            "--expect-run=x",
            "--expect-plan=x",
        ],
        &["reconcile", "g1", "t1", "1", "--run-id=x"],
        &["reconcile", "g1", "t1", "1", "--abandon"],
        &["review", "g1", "t1", "--no-open"],
        &["verdict", "g1", "t1", "fixed", "--note=x"],
    ];

    /// argparse takes a prefix of a long option when exactly one option of the command
    /// starts with it, `--help` among them, and calls any other prefix ambiguous.
    #[test]
    fn a_prefix_of_an_option_resolves_as_argparse_resolves_it() {
        let parse = |line: &[String]| {
            let argv = std::iter::once("puppets").chain(line.iter().map(String::as_str));
            Cli::try_parse_from(argv).map(|cli| format!("{cli:?}"))
        };
        let mut checked = 0;
        for line in LINES {
            let full: Vec<String> = line.iter().map(|word| word.to_string()).collect();
            let expected = parse(&full).unwrap();
            let mut names: Vec<&str> = line
                .iter()
                .filter(|word| word.starts_with("--"))
                .map(|word| word.split('=').next().unwrap())
                .collect();
            if line[0] == "reconcile" {
                names = vec!["--run-id", "--abandon"];
            }
            names.push("--help");
            for (at, word) in line.iter().enumerate() {
                let Some(name) = word.strip_prefix("--") else {
                    continue;
                };
                let (name, value) = match name.split_once('=') {
                    Some((name, value)) => (name, format!("={value}")),
                    None => (name, String::new()),
                };
                for cut in 1..name.len() {
                    let prefix = format!("--{}", &name[..cut]);
                    let matches = names.iter().filter(|n| n.starts_with(&prefix)).count();
                    let mut abbreviated = full.clone();
                    abbreviated[at] = format!("{prefix}{value}");
                    let parsed = parse(&abbreviated);
                    match matches {
                        1 => assert_eq!(parsed.unwrap(), expected, "{abbreviated:?}"),
                        _ => assert!(parsed.is_err(), "{abbreviated:?}"),
                    }
                    checked += 1;
                }
            }
        }
        assert!(checked > 60, "{checked}");
    }

    #[test]
    fn writer_options_accept_unambiguous_prefixes() {
        for args in [
            vec!["puppets", "approve", "g1", "--pl"],
            vec![
                "puppets", "set", "g1", "t1", "--pr", "codex", "--po", "standard",
            ],
            vec!["puppets", "verdict", "g1", "t1", "fixed", "--no", "ok"],
            vec!["puppets", "reconcile", "g1", "t1", "1", "--ab"],
        ] {
            assert!(Cli::try_parse_from(args.clone()).is_ok(), "{args:?}");
        }
        assert!(Cli::try_parse_from(["puppets", "approve", "g1", "--expect", "1"]).is_err());
    }
}
