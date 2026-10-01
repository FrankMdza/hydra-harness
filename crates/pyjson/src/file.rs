//! `Path.read_text` and the exception class Python raises when a file cannot be read.

use std::path::Path;

use anyhow::{Result, anyhow};

/// The Python exception class an I/O error on a file is raised as.
pub fn os_error(err: &std::io::Error) -> &'static str {
    match err.kind() {
        std::io::ErrorKind::NotFound => "FileNotFoundError",
        std::io::ErrorKind::AlreadyExists => "FileExistsError",
        std::io::ErrorKind::IsADirectory => "IsADirectoryError",
        std::io::ErrorKind::NotADirectory => "NotADirectoryError",
        std::io::ErrorKind::PermissionDenied => "PermissionError",
        _ => "OSError",
    }
}

/// `path.read_text(encoding="utf-8")`.
pub fn read_text(path: &Path) -> Result<String> {
    let raw =
        std::fs::read(path).map_err(|err| anyhow!("{}: {}", os_error(&err), path.display()))?;
    String::from_utf8(raw).map_err(|err| anyhow!("UnicodeDecodeError: {}: {err}", path.display()))
}
