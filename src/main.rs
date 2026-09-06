use anyhow::Result;
use clap::Parser;

use hydra::cli::{Cli, Command};
use hydra::paths::Layout;
use hydra::provider::Provider;
use hydra::run::{Call, Request, run};

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Run {
            provider,
            task,
            model,
            policy,
            allow_floor_escape,
            timeout,
        } => {
            let call = run(Request {
                provider: provider.parse::<Provider>()?,
                task,
                model,
                policy,
                floor_lifted: allow_floor_escape,
                timeout,
            })?;
            // D7: hydra run returns JSON on stdout. Two shapes, because harness
            // spec 9 degrades a blocking call to a handle rather than to a thinner
            // result.
            match call {
                Call::Ended(result) => println!("{}", serde_json::to_string_pretty(&result)?),
                Call::Handle(handle) => println!("{}", serde_json::to_string_pretty(&handle)?),
            }
        }
        Command::Status { run_id } => {
            let layout = Layout::discover()?;
            let text = match run_id {
                Some(id) => hydra::status::one(&layout, &id)?,
                None => hydra::status::all(&layout)?,
            };
            print!("{text}");
        }
        Command::Logs { run_id, follow } => {
            let mut out = std::io::stdout().lock();
            hydra::logs::logs(&Layout::discover()?, &run_id, follow, &mut out)?;
        }
        Command::Watch { spec } => hydra::watch::watch(&spec)?,
    }
    Ok(())
}
