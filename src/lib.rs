//! Hydra: a router over the agent CLIs.
//!
//! It launches one provider, `claude`, on a pty it owns, under a gate it compiled
//! from the run's policy, and writes a run record per `docs/run-record-contract.md`.
//! [`normalize`] reads one back into the ten event types of contract 7 and
//! [`outcome`] turns one into contract 8's outcome and contract 17's JSON. The
//! skill store, MCP injection and delegation are not built, and
//! `IMPLEMENTATION.md` says what that does to the records this binary writes.
//!
//! Every child is detached: [`watch`] is the half that owns the pty and the record,
//! [`run`] is the half a human talks to, and [`live`] is contract 5.2's liveness
//! test, which is the only thing that tells the two apart afterwards.

pub mod cli;
pub mod env;
pub mod live;
pub mod logs;
pub mod normalize;
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
pub mod status;
pub mod watch;
pub mod worktree;
