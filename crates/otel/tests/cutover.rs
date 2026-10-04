//! Build ticket 236: `normalized` refuses a run id holding `/` or `..` before joining
//! it into a path under the temporary `runs/`. The Python did not.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const RUST: &str = env!("CARGO_BIN_EXE_hydra-otel");

struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Scratch {
        let path =
            std::env::temp_dir().join(format!("hydra-otel-cutover-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        Scratch(path)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn write_record(dir: &Path, run_id: &str) -> PathBuf {
    let escaped = run_id.replace('\\', "\\\\").replace('"', "\\\"");
    let path = dir.join("record.jsonl");
    std::fs::write(
        &path,
        format!(
            "{{\"run_id\":\"{escaped}\",\"seq\":1,\"ts\":\"2026-01-01T00:00:00Z\",\"hydra\":{{\"type\":\"run_start\"}}}}\n"
        ),
    )
    .unwrap();
    path
}

fn convert(record: &Path, tmp: &Path) -> std::process::Output {
    Command::new(RUST)
        .arg(record)
        .env_clear()
        .env("TMPDIR", tmp)
        .env("PATH", "/usr/bin:/bin")
        .stdin(Stdio::null())
        .output()
        .unwrap()
}

/// Without the check, `runs.join("{id}.jsonl")` writes outside the temporary `runs/`
/// directory. The test fails unless that write is refused first.
#[test]
fn a_run_id_holding_slash_or_dotdot_is_refused_before_it_is_joined() {
    let scratch = Scratch::new("refuse");
    let tmp = scratch.0.join("tmp");
    std::fs::create_dir(&tmp).unwrap();
    let outside = scratch.0.join("outside");
    std::fs::create_dir(&outside).unwrap();
    let abs = outside.join("abs");
    let abs_id = abs.to_str().expect("scratch path is UTF-8").to_string();

    let cases: [(&str, PathBuf); 2] = [
        ("../../pwned", tmp.join("pwned.jsonl")),
        (&abs_id, outside.join("abs.jsonl")),
    ];

    for (run_id, canary) in cases {
        let record = write_record(&scratch.0, run_id);
        let ran = convert(&record, &tmp);
        let stderr = String::from_utf8_lossy(&ran.stderr);
        assert_eq!(ran.status.code(), Some(1), "{run_id}: {stderr}");
        assert!(
            stderr.contains("ValueError: run id must not contain '/' or '..'"),
            "{run_id}: {stderr}"
        );
        assert!(
            !canary.exists(),
            "{run_id}: wrote {} before refusing",
            canary.display()
        );
    }
}

/// `session_id` joins `parent_run_id` into a sibling path and reads the record there, so a
/// parent id holding `..` would read a record from outside the directory.
#[test]
fn a_parent_run_id_holding_dotdot_is_refused_before_it_is_read() {
    let scratch = Scratch::new("parent");
    let tmp = scratch.0.join("tmp");
    std::fs::create_dir(&tmp).unwrap();
    let records = scratch.0.join("records");
    std::fs::create_dir(&records).unwrap();
    std::fs::write(
        scratch.0.join("outside.jsonl"),
        "{\"run_id\":\"outside\",\"seq\":1,\"ts\":\"2026-01-01T00:00:00Z\",\"hydra\":{\"type\":\"run_start\"}}\n",
    )
    .unwrap();
    let record = records.join("child.jsonl");
    std::fs::write(
        &record,
        "{\"run_id\":\"child\",\"parent_run_id\":\"../outside\",\"seq\":1,\"ts\":\"2026-01-01T00:00:00Z\",\"hydra\":{\"type\":\"run_start\"}}\n",
    )
    .unwrap();
    // A stub `hydra show` that answers `{}`, so the conversion gets past `normalized`.
    let stub = scratch.0.join("hydra");
    std::fs::write(&stub, "#!/bin/sh\necho '{}'\n").unwrap();
    std::fs::set_permissions(&stub, std::os::unix::fs::PermissionsExt::from_mode(0o755)).unwrap();
    let ran = Command::new(RUST)
        .arg(&record)
        .env_clear()
        .env("TMPDIR", &tmp)
        .env("PATH", "/usr/bin:/bin")
        .env("HYDRA_BIN", &stub)
        .stdin(Stdio::null())
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&ran.stderr);
    assert_eq!(ran.status.code(), Some(1), "{stderr}");
    assert!(
        stderr.contains("ValueError: parent run id must not contain '/' or '..'"),
        "{stderr}"
    );
}
