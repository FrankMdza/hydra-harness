//! The command surface.
//!
//! Harness spec 17 lists eleven commands. This slice ships the first one, and adds
//! none of its own.

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "hydra", version, about = "A router over the agent CLIs")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Launch a provider on a recorded run and write its run record.
    Run {
        /// Which binary to route to. `claude` is the only one this slice builds.
        #[arg(long)]
        provider: String,

        /// The task string, passed to the provider verbatim.
        #[arg(long)]
        task: String,

        /// A model inside the provider. Never chosen on your behalf.
        #[arg(long)]
        model: Option<String>,
    },
}
