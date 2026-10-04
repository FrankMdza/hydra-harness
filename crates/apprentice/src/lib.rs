//! `hydra-apprentice`: the commands of `integrations/apprentice/apprentice.py`, ticket by
//! ticket.
//!
//! Build ticket 231 ported `bank build` (`bank.rs`), with the `json.dumps` the prompts are
//! written with (`dump.rs`) and the command line (`cli.rs`). `attempt` and `score` are build
//! ticket 232, `pairs` is 233, and until then the Python runs them. The record reader and the
//! redactor are `hydra-records`', as they are the grader's.

pub mod bank;
pub mod cli;
pub mod dump;
