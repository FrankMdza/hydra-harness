//! Hydra: a router over the agent CLIs.
//!
//! It launches two providers, `claude` and `cursor-agent`, on a pty it owns, under
//! a gate it compiled from the run's policy, and writes a run record per
//! `docs/run-record-contract.md`.
//! [`normalize`] reads one back into the ten event types of contract 7 and
//! [`outcome`] turns one into contract 8's outcome and contract 17's JSON.
//! [`skills`] is the store, the per-sha snapshot and what a run gets linked into
//! its checkout, and [`mcp`] is the canonical MCP document and the file one launch
//! generates from it. [`face`] is the MCP delegate face of harness spec 9, and the
//! four arguments it takes are the whole of what an orchestrator may decide.
//!
//! Every child is detached: [`watch`] is the half that owns the pty and the record,
//! [`run`] is the half a human talks to, and [`live`] is contract 5.2's liveness
//! test, which is the only thing that tells the two apart afterwards.
//!
//! [`ouroboros`] is the other half of the design and reads what the router wrote.
//! Rung one is built, as `hydra curate`; rung two is not.

pub mod cli;
pub mod env;
pub mod face;
pub mod interactive;
pub mod live;
pub mod logs;
pub mod mcp;
pub mod normalize;
pub mod ouroboros;
pub mod outcome;
pub mod paths;
pub mod policy;
pub mod preamble;
pub mod procinfo;
pub mod provider;
pub mod prune;
pub mod pty;
pub mod reap;
pub mod record;
pub mod run;
pub mod show;
pub mod skills;
pub mod status;
pub mod usage;
pub mod watch;
pub mod worktree;
