//! The command surface.
//!
//! Harness spec 17 lists eleven commands. This slice ships the first one, and adds
//! none of its own. Section 9 gives `hydra run` eight optional flags and three are
//! here. The five that are not each belong to a ticket that is not built.

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

        /// One of the five policies. `standard` is reads, writes, shell and
        /// network reads inside the worktree.
        #[arg(long, default_value = "standard")]
        policy: String,

        /// Lift the pattern floor for this run, harness spec 16. A human types
        /// this on one run; no policy can set it and no delegation inherits it.
        #[arg(long)]
        allow_floor_escape: bool,
    },
}
