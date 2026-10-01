//! `hydra-grader`: the commands of `integrations/grader/grader.py`, once they are ported.
//!
//! Build ticket 212 ported the record scan and the redactor, which are a library and have
//! no command line in the Python either. Until the commands arrive this says so and exits
//! 2, which is what `argparse` exits with for a command it does not know.

use std::process::ExitCode;

fn main() -> ExitCode {
    eprintln!(
        "hydra-grader: no command is ported yet; run integrations/grader/grader.py (build ticket 212 ported the record scan and the redactor as a library)"
    );
    ExitCode::from(2)
}
