//! `hydra-otel`: `integrations/otel/record_to_otlp.py`.

use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args_os()
        .skip(1)
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect();
    let (mut stdout, mut stderr) = (std::io::stdout().lock(), std::io::stderr().lock());
    ExitCode::from(hydra_otel::cli::main(&args, &mut stdout, &mut stderr))
}
