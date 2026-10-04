//! `hydra-otel`: the former `integrations/otel/record_to_otlp.py`, build tickets 235 and 236.
//!
//! Build ticket 236 deleted the Python. `tests/otel.rs` compares this binary to a recording
//! of it, and starts no `python3`.

pub mod cli;
pub mod convert;
