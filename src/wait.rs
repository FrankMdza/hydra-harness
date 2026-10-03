//! Blocking, read-only observation of selected runs.
use anyhow::{Context, Result, ensure};
use hydra::{live, paths::Layout, resolver, status};
use serde_json::Value;
use std::{
    io::Write,
    time::{Duration, Instant},
};

pub fn wait(
    layout: &Layout,
    runs: &[String],
    any: bool,
    timeout: Option<Duration>,
    out: &mut dyn Write,
) -> Result<i32> {
    ensure!(!runs.is_empty(), "at least one run is required");
    let ids = runs
        .iter()
        .map(|id| resolver::resolve(layout, id))
        .collect::<Result<Vec<_>>>()?;
    for id in &ids {
        ensure!(layout.run_file(id).is_file(), "no run record for {id}");
    }
    let started = Instant::now();
    loop {
        let views = ids
            .iter()
            .map(|id| live::read_view(&layout.run_file(id)))
            .collect::<Result<Vec<_>>>()?;
        let finished = views.iter().filter(|view| !view.is_live()).count();
        let ready = if any {
            finished > 0
        } else {
            finished == views.len()
        };
        let expired = timeout.is_some_and(|limit| started.elapsed() >= limit);
        if ready || expired {
            for view in &views {
                summary(layout, view, out)?;
            }
            if !ready {
                for view in views.iter().filter(|view| view.is_live()) {
                    writeln!(out, "still live: {}", view.run_id)?;
                }
                return Ok(2);
            }
            return Ok(0);
        }
        let sleep = timeout
            .map(|limit| {
                limit
                    .saturating_sub(started.elapsed())
                    .min(Duration::from_secs(2))
            })
            .unwrap_or(Duration::from_secs(2));
        std::thread::sleep(sleep);
    }
}

fn summary(layout: &Layout, view: &live::RunView, out: &mut dyn Write) -> Result<()> {
    // The model off the record's first envelope; everything else is the
    // projection `hydra show` prints, so the two never disagree about a run.
    let text = std::fs::read_to_string(&view.run_file).context("reading wait summary")?;
    let first = text
        .lines()
        .next()
        .and_then(|line| serde_json::from_str::<Value>(line).ok());
    let model = first
        .as_ref()
        .and_then(|v| v["model"].as_str())
        .unwrap_or("unknown")
        .to_string();
    let provider = view.provider.as_deref().unwrap_or("unknown");
    let (state, tokens) = if view.is_live() {
        ("live".to_string(), "unavailable".to_string())
    } else if view.state == live::State::Abandoned {
        ("abandoned".to_string(), "unavailable".to_string())
    } else {
        let projection = status::projection(layout, &view.run_id)?;
        let state = projection["outcome"].as_str().unwrap_or("unavailable");
        let tokens = projection["usage"]["counts"]["total_tokens"]
            .as_u64()
            .map_or_else(|| "unavailable".to_string(), |n| n.to_string());
        (state.to_string(), tokens)
    };
    writeln!(
        out,
        "{}\t{provider}\t{model}\t{state}\t{tokens}",
        view.run_id
    )?;
    Ok(())
}
