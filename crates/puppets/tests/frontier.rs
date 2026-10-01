use std::path::PathBuf;
use std::process::{Command, Stdio};

fn check(args: &[&str], expected: &str, nested: bool) {
    let mut command = Command::new(env!("CARGO_BIN_EXE_hydra-puppets"));
    command.arg("frontier").args(args).stdin(Stdio::null());
    if nested {
        command.current_dir(env!("CARGO_MANIFEST_DIR"));
    } else {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        command.current_dir(root.parent().unwrap().parent().unwrap());
    }
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    assert_eq!(output.stdout, expected.as_bytes());
}

/// The trackers under `.scratch/hydra` change with every ticket, so nothing here reads them:
/// a copy of the frozen tickets is laid out as a checkout's tracker and found from a
/// directory two levels down, as `frontier.py` finds its own from where it is run.
#[test]
fn the_tracker_is_found_from_a_nested_directory() {
    let frozen = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/frontier/edge");
    let root = std::env::temp_dir().join(format!("hydra-frontier-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    for tracker in ["issues", "build"] {
        let dir = root.join(".scratch/hydra").join(tracker);
        std::fs::create_dir_all(&dir).unwrap();
        for entry in std::fs::read_dir(&frozen).unwrap() {
            let path = entry.unwrap().path();
            std::fs::copy(&path, dir.join(path.file_name().unwrap())).unwrap();
        }
    }
    let nested = root.join("a/b");
    std::fs::create_dir_all(&nested).unwrap();
    for args in [&[][..], &["build"]] {
        let output = Command::new(env!("CARGO_BIN_EXE_hydra-puppets"))
            .arg("frontier")
            .args(args)
            .current_dir(&nested)
            .stdin(Stdio::null())
            .output()
            .unwrap();
        assert!(output.status.success());
        assert_eq!(
            String::from_utf8(output.stdout).unwrap(),
            include_str!("fixtures/frontier/edge.txt")
        );
    }
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn explicit_directory_preserves_python_parsing() {
    check(
        &["--dir", "tests/fixtures/frontier/edge"],
        include_str!("fixtures/frontier/edge.txt"),
        true,
    );
}
