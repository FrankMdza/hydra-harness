//! The grader in Rust, a ticket at a time.
//!
//! This is the port of `integrations/grader/`, which stays where it is as the reference
//! until the last ticket of map 94 deletes it. Build ticket 212 ported the two modules
//! every other one loads: `records.py` without `refresh_show_answer` and `iter_records`
//! (`records.rs`), and `redact.py` whole (`redact.rs`). `isotime.rs` is the part of
//! Python's `datetime` that `duration_seconds` leans on.
//!
//! `tests/differential.rs` calls these on the inputs `tests/record.py` gave the Python and
//! requires what the Python returned, wrote and printed. Neither module has a command line,
//! so the comparison is function by function, and the binary has no command yet.
//!
//! There is no `serde_json` here, for the reason `hydra-pyjson` gives.

pub mod isotime;
pub mod records;
pub mod redact;
