//! `hydra-local-llm`: a stdio MCP server backed by local Ollama.

use std::io::{self, BufReader};
use std::process::ExitCode;

fn main() -> ExitCode {
    let env = hydra_local_llm::Env::from_os();
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut stdin = BufReader::new(stdin.lock());
    let mut stdout = stdout.lock();
    match hydra_local_llm::serve(&mut stdin, &mut stdout, &env) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) if err.kind() == io::ErrorKind::BrokenPipe => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("hydra-local-llm: {err}");
            ExitCode::from(1)
        }
    }
}
