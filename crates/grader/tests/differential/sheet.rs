//! Byte comparisons with sheet.py recorded through a shell hydra stub.
use super::*;
use std::collections::BTreeMap;
use std::os::unix::fs::PermissionsExt;
use std::process::Stdio;

fn normalize(text: &str, root: &Path) -> String {
    text.replace(&root.display().to_string(), "<root>")
        .lines()
        .map(|line| {
            if line.starts_with("env: HYDRA_ROOT=") && !line.contains("<root>") {
                "env: HYDRA_ROOT=<tmp>".to_owned()
            } else if line.trim_start().starts_with("\"date\": \"") {
                let indent = &line[..line.len() - line.trim_start().len()];
                format!("{indent}\"date\": \"<date>\",")
            } else {
                line.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
        + if text.ends_with('\n') { "\n" } else { "" }
}
fn snapshot(root: &Path, input: &Value) -> BTreeMap<String, String> {
    listing(root)
        .into_iter()
        .filter_map(|entry| {
            let (path, text) = entry.split_once('\t').unwrap();
            let planted = matches!(input, Value::Dict(d) if d.get(path).is_some());
            (!planted || path.starts_with("home/") || path == "data/vendors.json")
                .then(|| (path.to_owned(), normalize(text, root)))
        })
        .collect()
}
#[test]
fn sheet_commands_match_the_python() {
    let mut tally = Tally::new();
    for (i, case) in items(field(&fixture("sheet/scenarios.json"), "cases"))
        .iter()
        .enumerate()
    {
        let scratch = Scratch::new(&format!("sheet-{i}"));
        let root = &scratch.0;
        std::fs::create_dir_all(root.join(string(field(case, "corpus")))).unwrap();
        let input = field(case, "input");
        let Value::Dict(files) = input else {
            panic!("files")
        };
        for (path, text) in files.iter() {
            let path = root.join(path.lossy());
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, string(text)).unwrap();
        }
        std::fs::set_permissions(
            root.join("bin/hydra"),
            std::fs::Permissions::from_mode(0o755),
        )
        .unwrap();
        for step in items(field(case, "steps")) {
            let Value::Dict(put) = field(step, "put") else {
                panic!("put")
            };
            for (path, text) in put.iter() {
                std::fs::write(root.join(path.lossy()), string(text)).unwrap();
            }
            let argv: Vec<String> = items(field(step, "argv")).iter().map(string).collect();
            let done = Command::new(RUST)
                .arg("sheet")
                .args(&argv)
                .current_dir(root)
                .env_clear()
                .env("HYDRA_ROOT", root.join("root"))
                .env("HYDRA_PUPPETS_HOME", root.join("home"))
                .env("HYDRA_PUPPETS_DATA", root.join("data"))
                .env("HYDRA_BIN", root.join("bin/hydra"))
                .env("PATH", "/usr/bin:/bin")
                .stdin(Stdio::null())
                .output()
                .unwrap();
            let label = format!("{} {argv:?}", string(field(case, "name")));
            tally.check(
                &format!("{label} code"),
                done.status.code().unwrap().to_string(),
                py::str(field(step, "code")).lossy(),
            );
            let stdout = String::from_utf8(done.stdout).unwrap();
            if argv.iter().any(|a| a == "--help") {
                tally.check(&format!("{label} help"), stdout.starts_with("usage:"), true);
            } else {
                tally.check(
                    &format!("{label} stdout"),
                    normalize(&stdout, root),
                    string(field(step, "stdout")),
                );
            }
            let stderr = String::from_utf8(done.stderr).unwrap();
            let stderr = if done.status.code() == Some(1) {
                stderr
                    .trim()
                    .split(':')
                    .next()
                    .unwrap_or_default()
                    .to_owned()
                    + ":"
            } else if done.status.code() == Some(2) {
                stderr.trim().lines().last().unwrap_or_default().to_owned()
            } else {
                stderr
            };
            tally.check(
                &format!("{label} stderr"),
                normalize(&stderr, root),
                string(field(step, "stderr")),
            );
            let Value::Dict(expected) = field(step, "files") else {
                panic!("snapshot")
            };
            let expected: BTreeMap<String, String> = expected
                .iter()
                .map(|(k, v)| (k.lossy(), string(v)))
                .collect();
            let actual = snapshot(root, input);
            tally.check(
                &format!("{label} file names"),
                actual.keys().collect::<Vec<_>>(),
                expected.keys().collect::<Vec<_>>(),
            );
            for (path, wanted) in &expected {
                tally.check(&format!("{label} {path}"), actual.get(path), Some(wanted));
            }
        }
    }
    tally.finish();
}
