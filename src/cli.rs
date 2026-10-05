//! The command surface.
//!
//! Harness spec 17 lists eleven commands. All eleven are here, and `doctor`,
//! build ticket 67, is the twelfth a human types.
//!
//! **`reap`, `prune` and `stop` are typed by a human and reachable from no run.**
//! Nothing in the library calls any of them, which is the only mechanism there is: harness spec
//! 7.5 says the gate cannot keep a run away from a binary reached through `shell`,
//! so the property lives in Hydra's own source. `curate` is typed by a human too
//! and is the one command that then launches a run of its own.
//!
//! **`watch` is a thirteenth command and is not one of the twelve.** It is the
//! detached half of a launch, `hydra run` is the only thing that types it, and it
//! is hidden from the help for that reason. The alternative was `fork` in process,
//! which would have put a pty, an allocator and an open record file on the wrong
//! side of a call nobody can test.
//!
//! **`delegate-face` is a fourteenth and is not one of the twelve either.** It is
//! the MCP face of harness spec 9, a stdio JSON-RPC server a run's own provider
//! spawns from the canonical MCP document, and section 17 lists commands a human
//! types. Hidden for the same reason `watch` is: typing it yourself gets you a
//! process waiting for a protocol you are not speaking.
//!
//! **`bench` is a further command a human types, build ticket 273.** It is the
//! attack-corpus runner: cells, repetitions, resume, a Wilson report, and a
//! `--dry-run` that launches nothing.
//!
//! **`cost` is a further command a human types, build ticket 274.** It reports
//! orchestrator tokens against the agents of each named run (started after it,
//! in its repository), with the median and spread. It launches nothing.
//!
//! **`models` is a further command a human types, build ticket 292.** It prints
//! the last refresh of each harness's models. `refresh` runs the three listing
//! commands and reads Claude from aliases. The print launches nothing.

use std::time::Duration;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "hydra", version, about = "A router over the agent CLIs")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

/// Keep the removed flag recognizable so the CLI can explain the replacement
/// before discovering a layout or starting any cockpit component.
#[derive(Clone)]
struct RefuseCockpitPolicy;

impl clap::builder::TypedValueParser for RefuseCockpitPolicy {
    type Value = String;

    fn parse_ref(
        &self,
        _cmd: &clap::Command,
        _arg: Option<&clap::Arg>,
        _value: &std::ffi::OsStr,
    ) -> Result<String, clap::Error> {
        Err(clap::Error::raw(
            clap::error::ErrorKind::InvalidValue,
            "gated chat uses hydra run --interactive --provider <p> --policy <name>\n",
        ))
    }
}

#[derive(Subcommand)]
pub enum Command {
    /// Launch a provider on a recorded run and write its run record.
    Run {
        /// Which binary to route to: `claude`, `cursor-agent`, `agy` or `codex`.
        #[arg(long, value_parser = parse_provider)]
        provider: crate::provider::Provider,

        /// The task string, passed to the provider verbatim. Optional only
        /// with `--interactive`, which opens with no prompt when it is
        /// absent.
        #[arg(long)]
        task: Option<String>,

        /// A model inside the provider. Never chosen on your behalf.
        #[arg(long)]
        model: Option<String>,

        /// One of the five policies. `standard` is reads, writes, shell and
        /// network reads inside the worktree. On a recorded run it defaults to
        /// `standard` when omitted. On an interactive run it defaults to none.
        #[arg(long)]
        policy: Option<String>,

        /// The run id this one is a child of. Its depth plus one is this run's,
        /// and its policy is the widest this one may ask for (7.3).
        #[arg(long)]
        parent: Option<String>,

        /// Lift the pattern floor for this run, harness spec 16. A human types
        /// this on one run; no policy can set it and no delegation inherits it.
        #[arg(long)]
        allow_floor_escape: bool,

        /// How long to wait before degrading to a handle: `600`, `30s`, `10m`,
        /// `2h`. The child is never killed, so `0` means launch and return.
        #[arg(long, default_value = "10m", value_parser = parse_duration)]
        timeout: Duration,

        /// Harness spec 3's second launch mode: the vendor draws its own
        /// interface in this pane and Hydra waits for it to exit, rather than
        /// owning a pty and streaming the frames into the record. Without
        /// `--policy` there is no gate, no skill injection and no preamble.
        /// With `--policy` on `claude` the launch compiles a gate, seeds a
        /// worktree and links the snapshot the way a recorded run does.
        /// `--parent` and `--allow-floor-escape` are ignored. `claude` gated or
        /// not, `codex` ungated only since build ticket 255, and `cursor-agent`
        /// ungated only since build ticket 266.
        #[arg(long)]
        interactive: bool,
    },

    /// Block until the named runs end, without launching or harvesting.
    Wait {
        #[arg(required = true, num_args = 1..)]
        runs: Vec<String>,
        /// Return when the first named run is no longer live.
        #[arg(long)]
        any: bool,
        #[arg(long, value_parser = parse_duration)]
        timeout: Option<Duration>,
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

    /// The full record a delegation result deliberately omits, harness spec 17.
    Show {
        run_id: String,

        /// Every event in file order, with the findings the normalizer raised,
        /// beside the same projection printed alone without this flag.
        #[arg(long)]
        events: bool,
    },

    /// Continue a finished run's provider session under a fresh run id.
    Resume {
        /// The run whose `provider_session_id` this launch hands back to the
        /// provider on its own resume flag.
        run_id: String,

        /// The task string for the continuation turn. Without it, a one-line
        /// default is used.
        #[arg(long)]
        task: Option<String>,

        /// How long to wait before degrading to a handle. The child is never
        /// killed, so `0` means launch and return.
        #[arg(long, default_value = "10m", value_parser = parse_duration)]
        timeout: Duration,
    },

    /// Commit a worktree's uncommitted changes to the child's branch, then remove
    /// the worktree and the run's spawn directory.
    Reap {
        /// One run. Without it, every checkout no live run is using.
        run_id: Option<String>,

        /// Remove the branch even when the commit left work on it. Without this,
        /// a branch holding commits survives the reap.
        #[arg(long)]
        delete_branch: bool,
    },

    /// Stop a live run: SIGTERM to its process group, SIGKILL after 10 s.
    ///
    /// Build ticket 169. The run's watcher still writes `run_end`, and the
    /// record derives `stopped`. Typed by a human outside any session, and
    /// refused inside a seatbelt as `hydra approve` is.
    Stop {
        /// A run id or an unambiguous prefix of one.
        #[arg(conflicts_with_all = ["all", "session"], required_unless_present_any = ["all", "session"])]
        run_id: Option<String>,

        /// Stop every live run.
        #[arg(long, conflicts_with = "session")]
        all: bool,

        /// Stop this session and every live run it launched, the seats first.
        /// Its launcher starts no seat once the stop began.
        #[arg(long, value_name = "RUN")]
        session: Option<String>,
    },

    /// Read the corpus and propose skill edits: rung one of the rungs.
    ///
    /// One recorded run under the `curate` policy, whose proposal lands on a
    /// branch in the skill store that a human merges or does not.
    Curate {
        /// A model inside the provider. Never chosen on your behalf.
        #[arg(long)]
        model: Option<String>,

        /// How long to wait before degrading to a handle. The run is never
        /// killed, and a pass that outlives its call commits nothing.
        #[arg(long, default_value = "10m", value_parser = parse_duration)]
        timeout: Duration,

        /// Print what the curator would be told and launch nothing.
        #[arg(long)]
        dry_run: bool,
    },

    /// One rung-two pass: proposer, admission gate, panel of four, branch.
    Evolve {
        /// A model inside the provider. Never chosen on your behalf.
        #[arg(long)]
        model: Option<String>,

        /// What to change. Without it the proposer picks its own target from
        /// the corpus, which is what the spec's brief asks for.
        #[arg(long, conflicts_with = "review")]
        task: Option<String>,

        /// A file or directory, repository-relative, the pass may edit. Repeat it
        /// for more. Without it the pass may edit `src/normalize/` and
        /// `src/show.rs`. A patch touching anything else is refused before
        /// review, and naming a protected path refuses the pass before it starts.
        /// `tests` holds the protected corpus and cannot be named whole, so a new
        /// test file is named by its own path.
        #[arg(long = "may-edit", value_name = "PATH", conflicts_with = "review")]
        may_edit: Vec<String>,

        /// Blocked pass id to re-review with human evidence. Map ticket 57.
        #[arg(long, requires = "evidence")]
        review: Option<String>,

        /// File whose text every judge sees as human-supplied correction evidence.
        #[arg(long, requires = "review")]
        evidence: Option<std::path::PathBuf>,

        /// How long to wait before degrading to a handle on each child run.
        #[arg(long, default_value = "10m", value_parser = parse_duration)]
        timeout: Duration,

        /// Compose the proposer's brief and launch nothing.
        #[arg(long)]
        dry_run: bool,
    },

    /// Token usage across the corpus.
    Usage {
        #[command(subcommand)]
        command: UsageCommand,
    },

    /// Move run files out of `runs/`. Deletes only behind a flag.
    Prune {
        /// The runs to prune. Without any, every run that is not live.
        run_id: Vec<String>,

        /// Delete rather than move. Anything deleted is training data destroyed.
        #[arg(long)]
        delete: bool,

        /// Print what would happen and change nothing.
        #[arg(long)]
        dry_run: bool,
    },

    /// Report what a launch would find: the data root, policies, provider versions,
    /// policy sha and protected configs. Launches no provider and runs no probe.
    Doctor {
        /// Walk every record under `runs/` with the same assertions the fixture
        /// walks use, and exit non-zero on a finding.
        #[arg(long)]
        walk: bool,
    },

    /// Approve a seat request that matches no approved shape, map ticket 82.
    ///
    /// Typed by a human in a terminal outside any session. Refused inside a
    /// seatbelt, so no model approves, the session that asked included.
    Approve {
        /// The request id the session was told. Without it, every request
        /// still waiting, with what each asks for.
        request_id: Option<String>,
    },

    /// Deny a seat request that waits for approval, build ticket 170.
    ///
    /// Typed by a human in a terminal outside any session, and refused inside
    /// a seatbelt as `hydra approve` is. The session hears one fixed code,
    /// `hydra/request-denied`, and the store keeps who denied and when.
    Deny {
        /// The request id the session was told. `hydra approve` with no id
        /// lists the ones still waiting.
        request_id: String,
    },

    /// A full-screen view of one goal's tasks, the runs by state, and one run's
    /// detail, build ticket 150. Reads files and draws; its one write is the
    /// approval of a waiting seat request, the same as `hydra approve <id>`.
    Cockpit {
        /// The goal log to show. Without it, the one that changed last.
        #[arg(long)]
        goal: Option<String>,

        /// How often to read the disk again: `2`, `5s`, `1m`.
        #[arg(long, default_value = "2", value_parser = parse_duration)]
        interval: Duration,

        /// Leave the dragon out of the header and give its rows to the table.
        #[arg(long)]
        no_mascot: bool,

        /// Launch `hydra run --interactive --provider <SESSION>` on a pty the
        /// cockpit owns and draw it in the left panel, build ticket 151. `F2`
        /// moves the keys between the chat and the cockpit; `Ctrl-q` quits.
        #[arg(long)]
        session: Option<String>,

        /// The model for `--session`, passed to `hydra run` as it is.
        #[arg(long, requires = "session")]
        model: Option<String>,

        /// Gated chats use `hydra run --interactive --provider <p> --policy <name>`.
        #[arg(long, requires = "session", value_parser = RefuseCockpitPolicy)]
        policy: Option<String>,
    },

    /// The one path by which anything leaves this machine, harness spec 14.
    Export {
        /// Where the header and one JSON line per eligible run are written.
        #[arg(long)]
        out: Option<std::path::PathBuf>,

        /// Print one line per run with its eligibility reasons and write nothing.
        #[arg(long)]
        dry_run: bool,
    },

    /// The attack-corpus benchmark, map 106. Launches each class through
    /// `hydra run --timeout 0`, or prints the cost without launching.
    Bench {
        /// `provider:policy`. Repeat. Without it, every cell of four by five.
        #[arg(long = "cell")]
        cells: Vec<String>,
        /// Times each attack is launched in each supported cell.
        #[arg(long, default_value_t = 3)]
        repeat: u32,
        /// How many of those launches may be live at once.
        #[arg(long, default_value_t = 2)]
        jobs: usize,
        /// Print the launch count and a token estimate, and launch nothing.
        #[arg(long)]
        dry_run: bool,
        /// The corpus table. Without it, `tests/corpus/README.md` beside the
        /// shipped policies.
        #[arg(long)]
        corpus: Option<std::path::PathBuf>,
    },

    /// Orchestrator cost against its agents, map 107. Prints the ratio per
    /// named run and the median. Launches nothing.
    Cost {
        /// Orchestrator run ids of the fixed goal set. Prefixes resolve.
        #[arg(required = true, num_args = 1..)]
        runs: Vec<String>,
        /// Real sessions printed beside the set as a check that it is typical.
        #[arg(long = "check", num_args = 1..)]
        check: Vec<String>,
    },

    /// The models each harness offers, from the last refresh. Launches nothing.
    Models {
        /// One provider. Without it, all four.
        #[arg(long, value_parser = parse_provider)]
        provider: Option<crate::provider::Provider>,
        /// The file as JSON rather than a table.
        #[arg(long)]
        json: bool,
        #[command(subcommand)]
        command: Option<ModelsCommand>,
    },

    /// The detached half of a launch. `hydra run` types this; a human does not.
    #[command(hide = true)]
    Watch {
        /// The launch spec `hydra run` wrote into the run's spawn directory.
        spec: std::path::PathBuf,
    },

    /// The MCP delegate face of harness spec 9, spoken over stdin and stdout.
    /// An orchestrator's provider runs this; a human does not.
    #[command(hide = true)]
    DelegateFace {
        /// The run whose children these are. Without it the face spawns runs at
        /// depth 0, which is what a human's own session gets.
        #[arg(long)]
        parent: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum UsageCommand {
    /// Tally each run's usage source and report whether anything needs recounting.
    Recount {
        /// One run. Without it, every record under `~/hydra/runs/`.
        run_id: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum ModelsCommand {
    /// Run each CLI's listing command and write the table.
    Refresh,
}

fn parse_provider(text: &str) -> Result<crate::provider::Provider, String> {
    text.parse().map_err(|_| {
        format!("{text} is not a routable provider; expected claude, cursor-agent, agy or codex")
    })
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
    /// Build ticket 67's command was lost once, on 2026-09-22, when a later
    /// ticket's `cli.rs` was copied over it from a worktree seeded before 67
    /// landed; nothing parsed it, so the suite and CI stayed green. This does.
    #[test]
    fn doctor_is_a_command_this_binary_answers_to() {
        use clap::Parser;
        let cli = super::Cli::try_parse_from(["hydra", "doctor"]).unwrap();
        assert!(matches!(
            cli.command,
            super::Command::Doctor { walk: false }
        ));
        let cli = super::Cli::try_parse_from(["hydra", "doctor", "--walk"]).unwrap();
        assert!(matches!(cli.command, super::Command::Doctor { walk: true }));
    }

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

    #[test]
    fn a_bad_provider_name_is_refused_with_the_valid_list() {
        use clap::Parser;
        let Err(err) =
            super::Cli::try_parse_from(["hydra", "run", "--provider", "hermes", "--task", "x"])
        else {
            panic!("hermes parsed as a provider");
        };
        let text = err.to_string();
        assert!(
            text.contains("claude")
                && text.contains("cursor-agent")
                && text.contains("agy")
                && text.contains("codex"),
            "{text}"
        );
    }
}
