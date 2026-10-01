//! Frozen triage CLI replay. Only wall-clock timestamps, CLI usage, exception traceback
//! frames and the permitted sample selection differ. Sample blocks must be exact members
//! of the reference population, unique, correctly counted and reproducible.
use super::*;
use std::collections::BTreeMap;
use std::process::Stdio;

fn map(v: &Value) -> BTreeMap<String, String> {
    let Value::Dict(d) = v else { panic!("dict") };
    d.iter().map(|(k, v)| (k.lossy(), string(v))).collect()
}
fn normalize(text: &str) -> String {
    text.split_inclusive('\n')
        .map(|line| {
            let row = json::loads_str(line).unwrap();
            let Value::Dict(d) = row else { panic!("row") };
            let stamp = string(d.get("triaged_at").unwrap());
            assert!(stamp.ends_with("+00:00"), "{stamp}");
            assert!(
                hydra_grader::isotime::fromisoformat(&PyStr::from(stamp.clone())).is_some(),
                "{stamp}"
            );
            if let Some((_, fraction)) = stamp.trim_end_matches("+00:00").split_once('.') {
                assert_eq!(fraction.len(), 6, "{stamp}");
            }
            let quoted = json::dumps_default(&Value::Str(PyStr::from(stamp)));
            let marker = "\"triaged_at\": ";
            let at = line.rfind(marker).unwrap() + marker.len();
            assert!(line[at..].starts_with(&quoted));
            format!("{}\"<time>\"{}", &line[..at], &line[at + quoted.len()..])
        })
        .collect()
}

fn paths(root: &Path, here: &Path) -> Vec<String> {
    let mut found = Vec::new();
    for entry in std::fs::read_dir(here).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            found.extend(paths(root, &path));
        } else {
            found.push(
                path.strip_prefix(root)
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .to_string(),
            );
        }
    }
    found.sort();
    found
}
fn error(text: &str, traceback: bool) -> String {
    let last = text.lines().last().unwrap_or("");
    if traceback {
        last.split(':')
            .next()
            .unwrap_or("")
            .rsplit('.')
            .next()
            .unwrap_or("")
            .to_string()
    } else {
        last.split_once(": error: ")
            .map_or(last, |(_, m)| m)
            .to_string()
    }
}
#[test]
fn triage_commands_match_the_recorded_python() {
    let data = fixture("triage.json");
    assert!(string(field(&data, "version")).starts_with("3.14.7"));
    let scratch = Scratch::new("triage");
    let mut tally = Tally::new();
    for (i, c) in items(field(&data, "cases")).iter().enumerate() {
        let name = string(field(c, "name"));
        let root = scratch.0.join(i.to_string());
        std::fs::create_dir_all(&root).unwrap();
        for (p, t) in map(field(c, "seed")) {
            let path = root.join(p);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, t).unwrap();
        }
        let argv: Vec<String> = items(field(c, "argv")).iter().map(string).collect();
        let run = || {
            Command::new(RUST)
                .arg("triage")
                .args(&argv)
                .current_dir(&root)
                .env_clear()
                .envs(map(field(c, "env")))
                .stdin(Stdio::null())
                .output()
                .unwrap()
        };
        let ran = run();
        let Value::Int(code) = field(c, "code") else {
            panic!("code")
        };
        tally.check(
            &format!("{name}: exit"),
            ran.status.code(),
            Some(*code as i32),
        );
        let out = String::from_utf8(ran.stdout).unwrap();
        let err = String::from_utf8(ran.stderr).unwrap();
        let pyout = string(field(c, "stdout"));
        let pyerr = string(field(c, "stderr"));
        if pyout.starts_with("usage:") {
            tally.check(
                &format!("{name}: help"),
                out.starts_with("usage: hydra-grader triage"),
                true,
            );
        } else if argv.first().is_some_and(|a| a == "sample") && *code == 0 {
            let seed = map(field(c, "seed"));
            let rows = seed
                .get("home/triage.jsonl")
                .map(|s| {
                    s.lines()
                        .map(|l| json::loads_str(l).unwrap())
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            let population: Vec<String> = rows
                .iter()
                .map(|r| {
                    format!(
                        "Run {} - Cause: {}\nEvidence: {}",
                        py::str(py::item(r, "run_id").unwrap()).lossy(),
                        py::str(py::item(r, "cause").unwrap()).lossy(),
                        py::str(py::item(r, "evidence").unwrap()).lossy()
                    )
                })
                .collect();
            let blocks: Vec<&str> = out
                .trim_end_matches('\n')
                .split("\n\n")
                .filter(|s| !s.is_empty())
                .collect();
            let pycount = pyout
                .trim_end_matches('\n')
                .split("\n\n")
                .filter(|s| !s.is_empty())
                .count();
            tally.check(&format!("{name}: count"), blocks.len(), pycount);
            tally.check(
                &format!("{name}: exact blocks"),
                blocks.iter().all(|b| population.iter().any(|p| p == b)),
                true,
            );
            let mut unique = blocks.clone();
            unique.sort();
            unique.dedup();
            tally.check(&format!("{name}: unique"), unique.len(), blocks.len());
            tally.check(
                &format!("{name}: repeat seed"),
                run().stdout,
                out.as_bytes().to_vec(),
            );
        } else {
            tally.check(&format!("{name}: stdout"), out, pyout);
        }
        if *code == 2 || pyerr.contains("Traceback (most recent call last):") {
            let trace = *code != 2;
            tally.check(
                &format!("{name}: error"),
                error(&err, trace),
                error(&pyerr, trace),
            );
        } else {
            tally.check(&format!("{name}: stderr"), err, pyerr);
        }
        let expected_files = map(field(c, "left"));
        tally.check(
            &format!("{name}: file names"),
            paths(&root, &root),
            expected_files.keys().cloned().collect(),
        );
        for (p, t) in expected_files {
            let actual =
                std::fs::read_to_string(root.join(&p)).unwrap_or_else(|_| "<missing>".into());
            let (actual, t) = if p == "home/triage.jsonl"
                && argv.first().is_some_and(|a| a == "run")
                && actual != "<missing>"
            {
                (normalize(&actual), normalize(&t))
            } else {
                (actual, t)
            };
            tally.check(&format!("{name}: {p}"), actual, t);
        }
    }
    tally.finish();
}
