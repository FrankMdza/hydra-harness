//! Hydra: a router over the agent CLIs.
//!
//! This is the first slice. It launches one provider, `claude`, on a pty it owns
//! and writes a run record per `docs/run-record-contract.md`. The gate, the skill
//! store, MCP injection, delegation and the normalizer are not built yet, and
//! `IMPLEMENTATION.md` says what that does to the records this slice writes.

pub mod cli;
pub mod env;
pub mod paths;
pub mod policy;
pub mod procinfo;
pub mod provider;
pub mod pty;
pub mod record;
pub mod run;
pub mod worktree;
