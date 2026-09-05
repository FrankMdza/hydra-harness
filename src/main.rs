use anyhow::Result;
use clap::Parser;

use hydra::cli::{Cli, Command};
use hydra::provider::Provider;
use hydra::run::{Request, run};

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Run {
            provider,
            task,
            model,
            policy,
            allow_floor_escape,
        } => {
            let outcome = run(Request {
                provider: provider.parse::<Provider>()?,
                task,
                model,
                policy,
                floor_lifted: allow_floor_escape,
            })?;
            // D7: hydra run returns JSON on stdout.
            println!("{}", serde_json::to_string_pretty(&outcome)?);
        }
    }
    Ok(())
}
