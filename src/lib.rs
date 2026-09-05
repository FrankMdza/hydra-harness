//! Hydra: a router over the agent CLIs.
//!
//! It launches one provider, `claude`, on a pty it owns, under a gate it compiled
//! from the run's policy, and writes a run record per `docs/run-record-contract.md`.
//! [`normalize`] reads one back into the ten event types of contract 7. The skill
//! store, MCP injection, delegation and outcome derivation are not built yet, and
//! `IMPLEMENTATION.md` says what that does to the records this binary writes.

pub mod cli;
pub mod env;
pub mod normalize;
pub mod paths;
pub mod policy;
pub mod preamble;
pub mod procinfo;
pub mod provider;
pub mod pty;
pub mod record;
pub mod run;
pub mod worktree;
