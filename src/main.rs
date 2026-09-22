use anyhow::Result;
use clap::Parser;

use hydra::cli::{Cli, Command, UsageCommand};
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
            parent,
            allow_floor_escape,
            timeout,
            interactive,
        } => {
            if interactive {
                let provider = provider.parse::<Provider>()?;
                anyhow::ensure!(
                    provider == Provider::Claude,
                    "hydra run --interactive supports claude only; cursor-agent and agy are \
                     not launched interactively yet"
                );
                let result = hydra::interactive::run(hydra::interactive::Request {
                    task,
                    model,
                    policy,
                })?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else {
                let task = task.ok_or_else(|| {
                    anyhow::anyhow!("a recorded run needs --task; only --interactive may omit it")
                })?;
                let call = run(Request {
                    provider: provider.parse::<Provider>()?,
                    task,
                    model,
                    policy: policy.unwrap_or_else(|| "standard".to_string()),
                    floor_lifted: allow_floor_escape,
                    timeout,
                    parent,
                    run_id: None,
                    command: "run".to_string(),
                    resumed_from: None,
                    resume_session_id: None,
                    working_dir: None,
                })?;
                // D7: hydra run returns JSON on stdout. Two shapes, because
                // harness spec 9 degrades a blocking call to a handle rather
                // than to a thinner result.
                match call {
                    Call::Ended(result) => println!("{}", serde_json::to_string_pretty(&result)?),
                    Call::Handle(handle) => println!("{}", serde_json::to_string_pretty(&handle)?),
                }
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
        Command::Show { run_id, events } => {
            let mut out = std::io::stdout().lock();
            hydra::show::show(&Layout::discover()?, &run_id, events, &mut out)?;
        }
        Command::Resume {
            run_id,
            task,
            timeout,
        } => {
            let call = hydra::resume::resume(hydra::resume::Request {
                run_id,
                task,
                timeout,
            })?;
            match call {
                Call::Ended(result) => println!("{}", serde_json::to_string_pretty(&result)?),
                Call::Handle(handle) => println!("{}", serde_json::to_string_pretty(&handle)?),
            }
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
            // Harness spec 5's second row, on its own lifetime. A snapshot is
            // shared between the runs that pinned it, so it is swept by sha
            // whether or not a run id was named: reaping one run does not release
            // a snapshot another still holds, and reaping none can release
            // several.
            let swept = hydra::reap::snapshots(&layout, hydra::run::LAUNCH_BUDGET)?;
            print!("{}", hydra::reap::snapshot_report(&swept));
            // A sweep that left a live run's checkout alone succeeded. One that
            // could not finish what it started did not, and a script driving the
            // cleanup has to be able to tell those apart from the exit code.
            let failed = hydra::reap::failures(&reaped);
            let failed_snapshots = hydra::reap::snapshot_failures(&swept);
            if failed + failed_snapshots > 0 {
                anyhow::bail!(
                    "{failed} of {} run(s) and {failed_snapshots} of {} snapshot(s) \
                     could not be reaped",
                    reaped.len(),
                    hydra::reap::snapshots_acted_on(&swept)
                );
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
            let done = hydra::rungs::curate::curate(hydra::rungs::curate::Pass {
                model,
                timeout,
                dry_run,
            })?;
            print!("{}", hydra::rungs::curate::report(&done));
        }
        Command::Evolve {
            model,
            task,
            review,
            evidence,
            timeout,
            dry_run,
        } => {
            let done = hydra::rungs::evolve::evolve(hydra::rungs::evolve::Pass {
                model,
                task,
                review,
                evidence,
                timeout,
                dry_run,
            })?;
            print!("{}", hydra::rungs::evolve::report(&done));
        }
        Command::Usage { command } => match command {
            UsageCommand::Recount { run_id } => {
                let mut out = std::io::stdout().lock();
                hydra::usage::recount(&Layout::discover()?, run_id.as_deref(), &mut out)?;
            }
        },
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
        Command::Doctor => {
            let layout = Layout::discover()?;
            let report = hydra::doctor::run(&layout)?;
            print!("{}", report.text);
            if report.missing {
                std::process::exit(1);
            }
        }
        Command::Export { out, dry_run } => {
            let layout = Layout::discover()?;
            let allowlist = hydra::export::allowlist_path();
            hydra::export::export(&layout, &allowlist, out.as_deref(), dry_run)?;
        }
        Command::Watch { spec } => hydra::watch::watch(&spec)?,
        // Stdout belongs to the protocol from here on, so nothing below prints.
        Command::DelegateFace { parent } => {
            let stdin = std::io::stdin().lock();
            let stdout = std::io::stdout().lock();
            hydra::face::serve(stdin, stdout, parent)?;
        }
    }
    Ok(())
}
