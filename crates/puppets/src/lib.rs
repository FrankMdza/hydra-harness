//! Master of Puppets in Rust: the goal log, the fold, the read commands, and the commands
//! that write without launching.
//!
//! This is the port of `integrations/puppets/puppets.py` as far as build tickets 193 and
//! 194 take it. The log's format is the Python's, and `tests/differential.rs` runs both on
//! the same logs and requires the same output and the same log afterwards. Nothing here
//! launches a run: `plan`, `plans`, `launch`, `review`, `wait` and the panel behind
//! `choose --judge` are still the Python's.

pub mod fold;
pub mod json;
pub mod log;
pub mod models;
pub mod printable;
mod py;
pub mod pysort;
pub mod read;
pub mod sheet;
pub mod write;
