//! `hydra-apprentice`: the port of `integrations/apprentice/apprentice.py`. Build tickets 231
//! to 233 ported `bank build`, `attempt`, `score` and `pairs`, and 234 deleted the Python.

use std::process::ExitCode;

/// A record line may nest as deep as `json::MAX_NESTING`, and reading or writing one
/// recurses once per level, so the command runs on a thread with room for it, as
/// `hydra-grader` does. The stack is address space, not memory, until a line that deep is
/// read.
const STACK_BYTES: usize = 1 << 30;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args_os()
        .skip(1)
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect();
    std::thread::Builder::new()
        .stack_size(STACK_BYTES)
        .spawn(move || {
            let (mut stdout, mut stderr) = (std::io::stdout().lock(), std::io::stderr().lock());
            hydra_apprentice::cli::main(
                &args,
                &|name| std::env::var_os(name),
                &mut stdout,
                &mut stderr,
            )
        })
        .and_then(|thread| {
            thread
                .join()
                .map_err(|_| std::io::Error::other("the command panicked"))
        })
        .map_or_else(
            |err| {
                eprintln!("hydra-apprentice: {err}");
                ExitCode::from(1)
            },
            ExitCode::from,
        )
}
