//! The command surface.
//!
//! Harness spec 17 lists eleven commands. Three are here: `run`, `status` and
//! `logs`. Section 9 gives `hydra run` eight optional flags and four are here.
//! The four that are not each belong to a ticket that is not built.
//!
//! **`watch` is a twelfth command and is not one of the eleven.** It is the
//! detached half of a launch, `hydra run` is the only thing that types it, and it
//! is hidden from the help for that reason. The alternative was `fork` in process,
//! which would have put a pty, an allocator and an open record file on the wrong
//! side of a call nobody can test.

use std::time::Duration;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "hydra", version, about = "A router over the agent CLIs")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Launch a provider on a recorded run and write its run record.
    Run {
        /// Which binary to route to. `claude` is the only one this slice builds.
        #[arg(long)]
        provider: String,

        /// The task string, passed to the provider verbatim.
        #[arg(long)]
        task: String,

        /// A model inside the provider. Never chosen on your behalf.
        #[arg(long)]
        model: Option<String>,

        /// One of the five policies. `standard` is reads, writes, shell and
        /// network reads inside the worktree.
        #[arg(long, default_value = "standard")]
        policy: String,

        /// Lift the pattern floor for this run, harness spec 16. A human types
        /// this on one run; no policy can set it and no delegation inherits it.
        #[arg(long)]
        allow_floor_escape: bool,

        /// How long to wait before degrading to a handle: `600`, `30s`, `10m`,
        /// `2h`. The child is never killed, so `0` means launch and return.
        #[arg(long, default_value = "10m", value_parser = parse_duration)]
        timeout: Duration,
    },

    /// Reconcile live pids against open run files, and report abandoned worktrees.
    Status {
        /// One run, in full. Without it, everything under `~/hydra/runs/`.
        run_id: Option<String>,
    },

    /// Tail a run file. The only way to watch an unwatched child.
    Logs {
        run_id: String,

        /// Keep reading until the run closes.
        #[arg(long, short)]
        follow: bool,
    },

    /// The detached half of a launch. `hydra run` types this; a human does not.
    #[command(hide = true)]
    Watch {
        /// The launch spec `hydra run` wrote into the run's spawn directory.
        spec: std::path::PathBuf,
    },
}

/// `600`, `30s`, `10m` or `2h`.
///
/// **Bare digits are seconds**, which is what every other tool on this machine
/// means by a naked timeout, and the suffixes are the three units a delegation
/// budget is ever written in. Days are not a unit here: a call nobody is waiting
/// on for a day should have returned a handle and been polled.
pub fn parse_duration(text: &str) -> Result<Duration, String> {
    let text = text.trim();
    if text.is_empty() {
        return Err("a timeout needs a number".to_string());
    }
    // By character rather than by byte: `split_at` on `text.len() - 1` panics
    // partway through a multi-byte one, and a timeout is a string a human types.
    let (digits, seconds_each) = match text.chars().next_back() {
        Some('s') => (&text[..text.len() - 1], 1),
        Some('m') => (&text[..text.len() - 1], 60),
        Some('h') => (&text[..text.len() - 1], 3600),
        _ => (text, 1),
    };
    let count: u64 = digits
        .parse()
        .map_err(|_| format!("{text:?} is not a duration: try 600, 30s, 10m or 2h"))?;
    count
        .checked_mul(seconds_each)
        .map(Duration::from_secs)
        .ok_or_else(|| format!("{text:?} is longer than this machine can count"))
}

/// The way back, for a handle that has to say what budget ran out.
pub fn render_duration(duration: Duration) -> String {
    let secs = duration.as_secs();
    match secs {
        0 => "0".to_string(),
        s if s % 3600 == 0 => format!("{}h", s / 3600),
        s if s % 60 == 0 => format!("{}m", s / 60),
        s => format!("{s}s"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_four_spellings_a_timeout_is_written_in() {
        assert_eq!(parse_duration("600").unwrap(), Duration::from_secs(600));
        assert_eq!(parse_duration("30s").unwrap(), Duration::from_secs(30));
        assert_eq!(parse_duration("10m").unwrap(), Duration::from_secs(600));
        assert_eq!(parse_duration("2h").unwrap(), Duration::from_secs(7200));
        assert_eq!(parse_duration(" 10m ").unwrap(), Duration::from_secs(600));
    }

    #[test]
    fn zero_is_a_timeout_and_not_an_error() {
        // Harness spec 9 never kills a child, so a zero budget is "launch it and
        // hand me the handle" rather than "give it no time to run".
        assert_eq!(parse_duration("0").unwrap(), Duration::ZERO);
    }

    #[test]
    fn a_unit_this_does_not_know_is_refused_rather_than_guessed() {
        // The last one is a byte-index trap: `é` is two bytes, so splitting the
        // string one byte from the end lands inside a character.
        for bad in ["", "ten", "10d", "1.5m", "-3", "m", "10é"] {
            assert!(parse_duration(bad).is_err(), "{bad:?} parsed");
        }
    }

    #[test]
    fn a_rendered_duration_parses_back_to_itself() {
        for secs in [0, 45, 600, 3600, 7200, 90] {
            let text = render_duration(Duration::from_secs(secs));
            assert_eq!(parse_duration(&text).unwrap().as_secs(), secs, "{text}");
        }
    }
}
