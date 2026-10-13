//! Import refusals leave the seeded goal log byte-for-byte unchanged.
use std::path::PathBuf;
use std::process::Command;

use hydra_puppets::{fold, json, log};

struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn import(name: &str, array: &str, offences: &[&str]) {
    let root = PathBuf::from(format!("task-graph-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let scratch = Scratch(root);
    let plan = scratch.0.join("plan.md");
    std::fs::write(&plan, format!("```json\n{array}\n```\n")).unwrap();
    let chosen = json::dumps_default(&json::Value::text(plan.to_str().unwrap()));
    let seed = format!(
        "{{\"event\":\"panel_verdict\",\"chosen\":\"test\",\"path\":{chosen}}}\n\
         {{\"event\":\"approval\",\"kind\":\"plan\",\"ref\":null,\"revision\":0,\"yes\":true}}\n"
    );
    let log = scratch.0.join("g.jsonl");
    std::fs::write(&log, &seed).unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_hydra-puppets"))
        .args(["tasks", "g"])
        .env_clear()
        .env("HYDRA_PUPPETS_HOME", &scratch.0)
        .output()
        .unwrap();
    let stderr = String::from_utf8(result.stderr).unwrap();
    assert!(result.stdout.is_empty());
    if offences.is_empty() {
        assert!(result.status.success(), "{stderr}");
        assert!(stderr.is_empty());
        let home = log::Home {
            puppets: scratch.0.clone(),
            grader: scratch.0.join("grader"),
            vendors: None,
            root: scratch.0.clone(),
        };
        let events = log::read_log(&home, "g").unwrap();
        let state = fold::fold("g", &events).unwrap();
        assert_eq!(state.tasks.len(), 2);
        assert_eq!(state.revision, 2);
        assert_eq!(state.tasks[0].deps.len(), 1);
        assert_eq!(json::dumps_default(&state.tasks[0].deps[0]), "\"b\"");
        assert_eq!(state.tasks[0].paths.len(), 1);
        assert_eq!(
            json::dumps_default(&state.tasks[0].paths[0]),
            "\"not-created-yet.md\""
        );
        assert!(std::fs::read_to_string(&log).unwrap().starts_with(&seed));
    } else {
        assert_eq!(result.status.code(), Some(1), "{stderr}");
        for offence in offences {
            assert!(stderr.contains(offence), "missing {offence}: {stderr}");
        }
        assert_eq!(std::fs::read(&log).unwrap(), seed.as_bytes());
    }
}

#[test]
fn duplicate_ids_refuse_without_writing() {
    import(
        "duplicate",
        r#"[{"id":1,"text":"a","class":"docs","grade":"low"},
            {"id":1.0,"text":"b","class":"docs","grade":"low"}]"#,
        &["duplicate task id 1.0"],
    );
    import(
        "combined",
        r#"[{"id":"a","text":"a","class":"docs","grade":"low","deps":["a","missing"]},
            {"id":"a","text":"b","class":"docs","grade":"low"}]"#,
        &[
            "duplicate task id 'a'",
            "unknown dep 'missing'",
            "cycle includes task 'a'",
        ],
    );
}

#[test]
fn unknown_dependencies_refuse_without_writing() {
    import(
        "unknown",
        r#"[{"id":"a","text":"a","class":"docs","grade":"low","deps":["missing","other"]}]"#,
        &[
            "task 'a' has unknown dep 'missing'",
            "task 'a' has unknown dep 'other'",
        ],
    );
}

#[test]
fn two_task_cycle_refuses_without_writing() {
    import(
        "cycle",
        r#"[{"id":"a","text":"a","class":"docs","grade":"low","deps":["b"]},
            {"id":"b","text":"b","class":"docs","grade":"low","deps":["a"]}]"#,
        &["cycle includes task 'a'", "cycle includes task 'b'"],
    );
}

#[test]
fn valid_plan_imports_forward_deps_and_future_paths() {
    import(
        "valid",
        r#"[{"id":"a","text":"a","class":"docs","grade":"low","deps":["b"],"paths":["not-created-yet.md"]},
            {"id":"b","text":"b","class":"docs","grade":"low"}]"#,
        &[],
    );
}
