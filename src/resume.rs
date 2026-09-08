//! `hydra resume`: continue a finished run's provider session, harness spec 17.
//!
//! Reads the source run's record, refuses when it cannot hand the provider its
//! own resume flag, and launches through the same pre-flight and spawn path as
//! [`crate::run::run`]. The source record is never rewritten.

use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result, bail};
use serde_json::Value;

use crate::live;
use crate::paths::Layout;
use crate::provider::Provider;
use crate::run::{Call, Request as RunRequest, run};

/// The default task when `--task` is omitted: one line, no preamble.
pub const DEFAULT_TASK: &str = "Continue from where the previous run left off.";

pub struct Request {
    pub run_id: String,
    pub task: Option<String>,
    pub timeout: Duration,
}

/// Reads the source record and launches the continuation.
pub fn resume(request: Request) -> Result<Call> {
    resume_in(&Layout::discover()?, request)
}

/// The same launch with an explicit layout, for tests and for callers that
/// already discovered one.
pub fn resume_in(layout: &Layout, request: Request) -> Result<Call> {
    let run_file = layout.run_file(&request.run_id);
    let view = live::read_view(&run_file)
        .with_context(|| format!("reading {}", run_file.display()))?;

    if view.is_live() {
        bail!(
            "run {} is still live, so its provider session has not finished and cannot be \
             resumed yet",
            request.run_id
        );
    }

    let start = live::opening_line(&run_file)?
        .with_context(|| format!("run {} has no run_start on its record", request.run_id))?;

    if start["launch_mode"].as_str() == Some("interactive") {
        bail!(
            "run {} was interactive, and interactive runs carry no provider session a recorded \
             resume can continue",
            request.run_id
        );
    }

    let provider = view
        .provider
        .as_deref()
        .or_else(|| start.get("provider").and_then(Value::as_str))
        .context(format!(
            "run {} names no provider on its record, so there is nothing to route a resume to",
            request.run_id
        ))?
        .parse::<Provider>()?;

    if !provider.supports_resume() {
        bail!(
            "{provider} has no resume concept, so hydra resume cannot continue run {}",
            request.run_id
        );
    }

    let session_id = provider_session_id(&run_file, provider, &start).with_context(|| {
        format!(
            "run {} has no provider_session_id on its record, so there is no session to hand \
             {provider}'s resume flag",
            request.run_id
        )
    })?;

    let policy = start["policy"]
        .as_str()
        .filter(|name| !name.is_empty())
        .with_context(|| {
            format!(
                "run {} has no policy on its record, and a resumed run needs the policy the \
                 original was launched under",
                request.run_id
            )
        })?;

    let cwd = start["cwd"]
        .as_str()
        .filter(|path| !path.is_empty())
        .map(PathBuf::from)
        .with_context(|| format!("run {} has no cwd on its record", request.run_id))?;

    let model = first_envelope_model(&run_file);

    let floor_lifted = start["floor_lifted"].as_bool().unwrap_or(false);

    run(RunRequest {
        provider,
        task: request.task.unwrap_or_else(|| DEFAULT_TASK.to_string()),
        model,
        policy: policy.to_string(),
        floor_lifted,
        timeout: request.timeout,
        parent: None,
        run_id: None,
        command: "resume".to_string(),
        resumed_from: Some(request.run_id.clone()),
        resume_session_id: Some(session_id),
        working_dir: Some(cwd),
    })
}

/// The provider's own session id, from line 0 or from the stream.
pub fn provider_session_id(path: &Path, provider: Provider, start: &Value) -> Option<String> {
    if let Some(id) = start
        .get("provider_session_id")
        .and_then(Value::as_str)
        .filter(|id| !id.is_empty())
    {
        return Some(id.to_string());
    }

    let text = std::fs::read_to_string(path).ok()?;
    for line in text.lines().skip(1) {
        if line.trim().is_empty() {
            continue;
        }
        let Ok(envelope) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        let observed = match envelope.get("raw") {
            Some(Value::String(text)) => provider.observe(text),
            Some(value) => provider.observe(
                &serde_json::to_string(value).unwrap_or_default(),
            ),
            None => continue,
        };
        if let Some(id) = observed.session_id {
            return Some(id);
        }
    }
    None
}

fn first_envelope_model(path: &Path) -> Option<String> {
    let text = std::fs::read_to_string(path).ok()?;
    let first = text.lines().next()?;
    let envelope = serde_json::from_str::<Value>(first).ok()?;
    envelope
        .get("model")
        .and_then(Value::as_str)
        .filter(|name| !name.is_empty())
        .map(str::to_string)
}
