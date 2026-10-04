//! The grader in Rust, a ticket at a time.
//!
//! This is the port of the Python that was `integrations/grader/`. The last ticket of map
//! 94, build ticket 220, deleted that Python; what it printed on every test input is under
//! `tests/fixtures/`, and the modules below still say which file each is the port of.
//! Build ticket 212 ported the two modules
//! every other one loads: `records.py` without `refresh_show_answer` and `iter_records`
//! (`records.rs`), and `redact.py` whole (`redact.rs`). `isotime.rs` is the part of
//! Python's `datetime` that `duration_seconds` leans on. Build ticket 224 moved all three,
//! and the judge's client, to `crates/records`.
//!
//! `tests/differential.rs` calls these on the inputs `tests/record.py` gave the Python and
//! requires what the Python returned, wrote and printed. Neither module has a command line,
//! so the comparison is function by function.
//!
//! Build ticket 213 ported the half of `baseline.py` that trains, reports and scores
//! (`baseline.rs`) and its two commands (`baseline_cli.rs`), which the binary runs as
//! `hydra-grader baseline train` and `hydra-grader baseline score`.
//!
//! Build ticket 215 ported `monitor.py` whole (`monitor.rs`) and its three commands
//! (`monitor_cli.rs`), which the binary runs as `hydra-grader monitor features`, `train`
//! and `score`.
//! Build ticket 214 ported `labels.py` (`labels.rs`), `grader.py` and
//! `records.refresh_show_answer` (`collect.rs`), and the four commands of `grader.py`
//! (`collect_cli.rs`): `hydra-grader once`, `watch`, `rebuild` and `backfill`.
//! Build ticket 218 ported `judge.py` whole (`judge.rs`, `judge_cli.rs`) as `hydra-grader judge
//! label` and `agree`, over a client that calls this machine only (`loopback_http.rs` in
//! `crates/records`).
//!
//! Build ticket 220 made one copy of two things three commands each carried: the reading of
//! `argparse` (`argparse.rs`) and where the data directory is (`data.rs`). It also gave a
//! pass a lock on the grader home (`collect.rs`).
//!
//! There is no `serde_json` here, for the reason `hydra-pyjson` gives.

pub mod argparse;
pub mod baseline;
pub mod baseline_cli;
pub mod collect;
pub mod collect_cli;
pub mod data;
pub mod jev;
pub mod jev_cli;
pub mod judge;
pub mod judge_cli;
pub mod labels;
pub mod monitor;
pub mod monitor_cli;
pub mod originals;
pub mod originals_cli;
pub mod queue;
pub mod triage;
pub mod triage_cli;

pub mod sheet;
pub mod sheet_cli;
