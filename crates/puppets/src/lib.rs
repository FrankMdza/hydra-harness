//! Master of Puppets in Rust: the goal log, the fold, the read commands, the commands that
//! write without launching, and the commands that launch.
//!
//! This is the port of `integrations/puppets/puppets.py` as far as build tickets 193 to
//! 195 take it, which is every command. The log's format is the Python's, and
//! `tests/differential.rs` runs both on the same logs and requires the same output and the
//! same log afterwards. The launches in that test go to a stub named by `HYDRA_BIN`.

pub mod fold;
pub mod json;
pub mod launch;
pub mod log;
pub mod models;
pub mod panel;
pub mod printable;
mod py;
pub use py::{int_literal, int_value};
pub mod pysort;
pub mod read;
pub mod sheet;
pub mod write;
