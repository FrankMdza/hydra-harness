//! The GitHub workflow files parse, and every step's `run` is a string.
//!
//! A `run:` line that ends in a colon, such as `cargo test --lib sandbox::tests::`,
//! is a YAML mapping and not a command. GitHub then refuses the whole file and
//! starts no job, so CI went dark on master from 2026-10-28 to 2026-11-01 with
//! nothing red in the local suite. This test reads the files the way GitHub does.

use std::path::Path;

use yaml_rust2::{Yaml, YamlLoader};

fn load(path: &Path) -> Yaml {
    let text = std::fs::read_to_string(path).unwrap();
    let mut docs = YamlLoader::load_from_str(&text)
        .unwrap_or_else(|err| panic!("{} does not parse: {err}", path.display()));
    assert_eq!(docs.len(), 1, "{}", path.display());
    docs.remove(0)
}

#[test]
fn every_workflow_parses_and_every_run_is_a_string() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join(".github");
    let mut workflows: Vec<_> = std::fs::read_dir(root.join("workflows"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "yml"))
        .collect();
    workflows.sort();
    assert!(!workflows.is_empty());

    for path in &workflows {
        let doc = load(path);
        let jobs = doc["jobs"].as_hash().unwrap_or_else(|| {
            panic!("{} has no jobs mapping", path.display());
        });
        for (name, job) in jobs {
            let Some(steps) = job["steps"].as_vec() else {
                continue;
            };
            for step in steps {
                let run = &step["run"];
                assert!(
                    run.is_badvalue() || run.as_str().is_some(),
                    "{} job {:?}: a step's run is not a string: {run:?}",
                    path.display(),
                    name.as_str()
                );
            }
        }
    }
    load(&root.join("dependabot.yml"));
}
