//! What more than the grader reads: a run record and the grader's own files (`records.rs`),
//! the redactor (`redact.rs`), the part of Python's `datetime` that `duration_seconds` leans
//! on (`isotime.rs`), and a client that POSTs to this machine and nowhere else
//! (`loopback_http.rs`).
//!
//! Build ticket 224 moved these out of `crates/grader`, as build 210 moved the JSON code to
//! `crates/pyjson`, so the apprentice, otel and `local-llm` can use them without the grader.
//! `crates/grader/tests/differential.rs` is still the check that they match the Python.

pub mod isotime;
pub mod loopback_http;
pub mod records;
pub mod redact;
