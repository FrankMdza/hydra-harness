use std::ffi::OsStr;
use std::process::Command;

/// Keep fixture objects in their repository, independent of the launching run.
/// Callers can still set explicit object stores after constructing the command.
pub fn command() -> Command {
    isolated_command("git")
}

/// Also isolate fixture scripts that launch git internally.
pub fn isolated_command(program: impl AsRef<OsStr>) -> Command {
    let mut command = Command::new(program);
    command
        .env_remove("GIT_OBJECT_DIRECTORY")
        .env_remove("GIT_ALTERNATE_OBJECT_DIRECTORIES");
    command
}
