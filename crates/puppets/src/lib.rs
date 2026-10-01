//! Master of Puppets in Rust: the goal log, the fold, and the read commands.
//!
//! This is the first slice of the port of `integrations/puppets/puppets.py` (build ticket
//! 193). The log's format is the Python's, and `tests/differential.rs` runs both on the same
//! logs and requires the same output. Nothing here launches a run or writes an approval yet.

pub mod fold;
pub mod json;
pub mod log;
mod py;
pub mod read;
