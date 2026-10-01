//! `hydra-grader`: the commands of `integrations/grader/`, as they are ported.
//!
//! Build ticket 213 ported `baseline.py`'s `train` and `score`, which run as
//! `hydra-grader baseline train` and `hydra-grader baseline score`. Build ticket 215 ported
//! `monitor.py`, which runs as `hydra-grader monitor features`, `train` and `score`. The
//! commands of `grader.py` itself are not ported yet: anything else says so and exits 2, which is what
//! `argparse` exits with for a command it does not know.

use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args_os()
        .skip(1)
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect();
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
    eprintln!(
        "hydra-grader: no command is ported yet; run integrations/grader/grader.py (build ticket 212 ported the record scan and the redactor as a library)"
    );
    ExitCode::from(2)
}
