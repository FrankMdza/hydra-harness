//! `hydra-apprentice`: the commands of `integrations/apprentice/apprentice.py`, ticket by
//! ticket.
//!
//! Build ticket 231 ported `bank build` (`bank.rs`), with the `json.dumps` the prompts are
//! written with (`dump.rs`) and the command line (`cli.rs`). Build ticket 232 ported
//! `attempt` (`attempt.rs`) and `score` (`score.rs`). `pairs` is 233, and until then the
//! Python runs it. The record reader, the redactor and the client for the model server are
//! `hydra-records`', as they are the grader's.

pub mod attempt;
pub mod bank;
pub mod cli;
pub mod dump;
pub mod score;
