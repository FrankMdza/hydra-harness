//! Master of Puppets in Rust: the goal log, the fold, the read commands, the commands that
//! write without launching, and the commands that launch.
//!
//! This is the port of the Python helper, `integrations/puppets/puppets.py`, which build
//! ticket 196 deleted. Build tickets 193 to 195 ported every command. The log's format is
//! the Python's, and `tests/differential.rs` runs this on the logs the Python ran on and
//! requires the output and the log afterwards that `tests/record.py` recorded from it. The
//! launches in that test go to a stub named by `HYDRA_BIN`.
//!
//! Since build ticket 196 the cockpit, the Herdr board, `pending`'s `argv` and the grader
//! all run this binary. `sheet.py` and `models.py` stay in `integrations/puppets/`: the
//! first holds three commands nobody ported, and both wait for the grader's own port.

pub use hydra_pyjson::{json, models, printable, pysort};

pub mod fold;
pub mod frontier;
pub mod index;
pub mod launch;
pub mod log;
pub mod panel;
use hydra_pyjson::py;
pub use py::{int_literal, int_value};
pub mod read;
pub mod report;
pub mod sheet;
pub mod write;
