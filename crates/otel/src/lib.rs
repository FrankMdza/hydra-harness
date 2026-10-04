//! `hydra-otel`: `integrations/otel/record_to_otlp.py`, build ticket 235.
//!
//! The Python stays until the cutover. `tests/otel.rs` compares this binary to a recording
//! of the Python, and starts no `python3`.

pub mod cli;
pub mod convert;
