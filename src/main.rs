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
                run_id: None,
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
        Command::Reap {
            run_id,
            delete_branch,
        } => {
            let layout = Layout::discover()?;
            // The launch grace period, the same one `hydra status` reports
            // against. A checkout exists before its record does, so a sweep that
            // allowed none would eventually remove the checkout of a run that was
            // about to start.
            let reaped = hydra::reap::reap(
                &layout,
                run_id.as_deref(),
                delete_branch,
                hydra::run::LAUNCH_BUDGET,
            )?;
            print!("{}", hydra::reap::report(&reaped));
            // A sweep that left a live run's checkout alone succeeded. One that
            // could not finish what it started did not, and a script driving the
            // cleanup has to be able to tell those apart from the exit code.
            let failed = hydra::reap::failures(&reaped);
            if failed > 0 {
                anyhow::bail!("{failed} of {} could not be reaped", reaped.len());
            }
        }
        Command::Curate {
            model,
            timeout,
            dry_run,
        } => {
            // Text rather than contract 17's JSON. That projection belongs to
            // `hydra run` and the delegate face, where a caller has to switch on
            // it; `hydra curate` is typed by a human like reap and prune are, and
            // its answer is a branch.
            let done = hydra::ouroboros::curate::curate(hydra::ouroboros::curate::Pass {
                model,
                timeout,
                dry_run,
            })?;
            print!("{}", hydra::ouroboros::curate::report(&done));
        }
        Command::Prune {
            run_id,
            delete,
            dry_run,
        } => {
            let layout = Layout::discover()?;
            // The same launch grace reap gets, and for the same reason: the run
            // file exists before line 0 does, so an empty record is a run that is
            // starting rather than one that is over.
            let pruned =
                hydra::prune::prune(&layout, &run_id, delete, dry_run, hydra::run::LAUNCH_BUDGET)?;
            print!("{}", hydra::prune::report(&pruned));
            let failed = hydra::prune::failures(&pruned);
            if failed > 0 {
                anyhow::bail!("{failed} of {} could not be pruned", pruned.len());
            }
        }
        Command::Watch { spec } => hydra::watch::watch(&spec)?,
    }
    Ok(())
}
