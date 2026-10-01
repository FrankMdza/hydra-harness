//! What a port of Hydra's Python needs to print what the Python printed: `json.loads` and
//! `json.dumps` (`json.rs`), the operators, `str`, `repr` and `int` over those values
//! (`py.rs`), `sorted` (`pysort.rs`), `str.isprintable` (`printable.rs`), `models.py`
//! (`models.rs`) and `Path.read_text` (`file.rs`).
//!
//! Build ticket 210 moved these out of `crates/puppets` so the grader's port can use them.
//! There is no `serde_json` here for the reason `crates/puppets` had none: `preserve_order`
//! would reach `hydra` in a workspace build. `crates/puppets/tests/differential.rs` is still
//! the check that this crate matches the Python, against fixtures recorded from it.

pub mod file;
pub mod json;
pub mod math;
pub mod math_probe;
pub mod models;
pub mod printable;
pub mod py;
pub mod pysort;
