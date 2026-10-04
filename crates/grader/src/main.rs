//! `hydra-grader`: the commands the Python of `integrations/grader/` had, which build ticket
//! 220 deleted.
//!
//! Build ticket 213 ported `baseline.py`'s `train` and `score`, which run as
//! `hydra-grader baseline train` and `hydra-grader baseline score`. Build ticket 215 ported
//! `monitor.py`, which runs as `hydra-grader monitor features`, `train` and `score`. Build
//! ticket 214 ported the commands of `grader.py` itself: `once`, `watch`, `rebuild` and
//! `backfill`. Anything else exits 2 with the error `argparse` gives a command it does not
//! know.

use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args_os()
        .skip(1)
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect();
    if args.first().is_some_and(|first| first == "sheet") {
        return ExitCode::from(hydra_grader::sheet_cli::main(
            &args[1..],
            &mut std::io::stdout().lock(),
            &mut std::io::stderr().lock(),
        ));
    }
    if args.first().is_some_and(|first| first == "originals") {
        return ExitCode::from(hydra_grader::originals_cli::main(
            &args[1..],
            &mut std::io::stdout().lock(),
            &mut std::io::stderr().lock(),
        ));
    }
    if args.first().is_some_and(|first| first == "system-one") {
        return ExitCode::from(hydra_grader::system_one_cli::main(
            &args[1..],
            &mut std::io::stdout().lock(),
            &mut std::io::stderr().lock(),
        ));
    }
    // Build ticket 213: `baseline.py`'s `train` and `score`.
    if args.first().is_some_and(|first| first == "baseline") {
        let (mut stdout, mut stderr) = (std::io::stdout().lock(), std::io::stderr().lock());
        return ExitCode::from(hydra_grader::baseline_cli::main(
            &args[1..],
            &mut stdout,
            &mut stderr,
        ));
    }
    if args.first().is_some_and(|first| first == "jev") {
        return ExitCode::from(hydra_grader::jev_cli::main(
            &args[1..],
            &mut std::io::stdout().lock(),
            &mut std::io::stderr().lock(),
        ));
    }
    // Build ticket 215: `monitor.py`'s `features`, `train` and `score`.
    if args.first().is_some_and(|first| first == "monitor") {
        let (mut stdout, mut stderr) = (std::io::stdout().lock(), std::io::stderr().lock());
        return ExitCode::from(hydra_grader::monitor_cli::main(
            &args[1..],
            &mut stdout,
            &mut stderr,
        ));
    }
    if args.first().is_some_and(|first| first == "triage") {
        return ExitCode::from(hydra_grader::triage_cli::main(
            &args[1..],
            &mut std::io::stdout().lock(),
            &mut std::io::stderr().lock(),
        ));
    }
    // Build ticket 218: `judge.py`'s `label` and `agree`, on the same deep stack as the pass
    // below, since both read lines of the dataset and what a model wrote.
    let judging = args.first().is_some_and(|first| first == "judge");
    // Build ticket 214: the four commands of `grader.py` itself, and its errors for the rest.
    // A record line or a goal log line may nest as deep as `json::MAX_NESTING`, and reading
    // one recurses once per level, so the command runs on a thread with room for it, as
    // `hydra-puppets` does. The stack is address space, not memory, until a line that deep
    // is read.
    std::thread::Builder::new()
        .stack_size(hydra_grader::collect::STACK_BYTES)
        .spawn(move || {
            let (mut stdout, mut stderr) = (std::io::stdout().lock(), std::io::stderr().lock());
            let env = hydra_grader::labels::Env::process();
            if judging {
                return hydra_grader::judge_cli::main(&args[1..], &env, &mut stdout, &mut stderr);
            }
            hydra_grader::collect_cli::main(&args, &env, &mut stdout, &mut stderr)
        })
        .and_then(|thread| {
            thread
                .join()
                .map_err(|_| std::io::Error::other("the command panicked"))
        })
        .map_or_else(
            |err| {
                eprintln!("hydra-grader: {err}");
                ExitCode::from(1)
            },
            ExitCode::from,
        )
}
