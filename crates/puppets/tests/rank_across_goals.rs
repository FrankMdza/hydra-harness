//! Build 327: `assign` ranks from the verdicts of every goal log under the home, and states
//! on the goal log the prior it started from.
use hydra_puppets::json::{self, Value};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const GOAL: &str = r#"{"event":"goal","text":"x","repo":"/r","base_sha":"a"}"#;
const TASK: &str = r#"{"event":"task","id":"t1","text":"t","class":"docs","grade":"low"}"#;
/// A class the default table does not hold, so the task stays unassigned and every `assign`
/// ranks it again.
const WEIRD: &str = r#"{"event":"task","id":"t2","text":"t","class":"weird","grade":"low"}"#;
const SHEET: &str = r#"{"provider":"claude","model":"sonnet","available":true,"policies":["standard"],"runs":[{"run_id":"r1","task_id":"t1"}],"cost":{"p50":0.02,"n":1}}"#;

struct Home(PathBuf);
impl Drop for Home {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
impl Home {
    fn new(name: &str) -> Home {
        let path = std::env::temp_dir().join(format!("rank-across-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(path.join("agents")).unwrap();
        std::fs::write(path.join("agents/claude--sonnet.json"), SHEET).unwrap();
        Home(path)
    }
    fn log(&self, goal: &str, lines: &[&str]) {
        let body: String = lines.iter().map(|l| format!("{l}\n")).collect();
        std::fs::write(self.0.join(format!("{goal}.jsonl")), body).unwrap();
    }
    fn assign(&self, goal: &str) -> Output {
        let done = Command::new(env!("CARGO_BIN_EXE_hydra-puppets"))
            .args(["assign", goal])
            .env_clear()
            .env("HYDRA_PUPPETS_HOME", &self.0)
            .env("HYDRA_PUPPETS_DATA", data())
            .env("HYDRA_ROOT", self.0.join("root"))
            .env("PATH", "/usr/bin:/bin")
            .output()
            .unwrap();
        assert!(
            done.status.success(),
            "{}",
            String::from_utf8_lossy(&done.stderr)
        );
        done
    }
    fn events(&self, goal: &str) -> Vec<Value> {
        std::fs::read_to_string(self.0.join(format!("{goal}.jsonl")))
            .unwrap()
            .lines()
            .map(|l| json::loads_str(l).unwrap())
            .collect()
    }
}

fn data() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../integrations/puppets")
}

fn field(event: &Value, key: &str) -> Value {
    match event {
        Value::Dict(d) => d.get(key).cloned().unwrap_or(Value::Null),
        _ => Value::Null,
    }
}

fn named<'a>(events: &'a [Value], name: &str) -> Vec<&'a Value> {
    events
        .iter()
        .filter(|e| json::dumps_default(&field(e, "event")) == format!("\"{name}\""))
        .collect()
}

fn text(value: &Value) -> String {
    match value {
        Value::Str(s) => s.lossy(),
        other => json::dumps_default(other),
    }
}

#[test]
fn another_goals_verdict_settles_this_goals_assignment() {
    let priors = data().join("priors.json").display().to_string();

    // Goal b alone: sonnet has only its prior, so the default table assigns.
    let home = Home::new("alone");
    home.log("b", &[GOAL, TASK]);
    let out = String::from_utf8(home.assign("b").stdout).unwrap();
    assert!(
        out.starts_with("t1: ") && out.contains("(default)"),
        "{out}"
    );
    let events = home.events("b");
    let stated = named(&events, "rank_prior");
    assert_eq!(stated.len(), 1);
    assert_eq!(
        text(&field(stated[0], "text")),
        format!(
            "rank read 0 verdicts across 1 goals, starting from the priors in {priors}: \
             docs claude sonnet Beta(5, 1) and 0 verdicts"
        )
    );

    // Goal a holds an applied verdict on sonnet's run for its own `t1`.
    let home = Home::new("across");
    home.log(
        "a",
        &[
            GOAL,
            TASK,
            r#"{"event":"human_verdict","task":"t1","run_id":"r1","result":"applied","note":""}"#,
        ],
    );
    home.log("b", &[GOAL, TASK, WEIRD]);
    let out = String::from_utf8(home.assign("b").stdout).unwrap();
    assert!(out.contains("t1: claude sonnet (ranking)"), "{out}");
    let events = home.events("b");
    let stated = named(&events, "rank_prior");
    assert_eq!(stated.len(), 1);
    let event = stated[0];
    assert_eq!(text(&field(event, "goals")), "2");
    assert_eq!(text(&field(event, "verdicts")), "1");
    assert_eq!(text(&field(event, "priors")), priors);
    assert_eq!(
        text(&field(event, "text")),
        format!(
            "rank read 1 verdicts across 2 goals, starting from the priors in {priors}: \
             docs claude sonnet Beta(5, 1) and 1 verdicts; \
             weird claude sonnet Beta(5, 1) and 0 verdicts"
        )
    );
    // The event is the last line, after the assignment it explains.
    assert_eq!(text(&field(events.last().unwrap(), "event")), "rank_prior");

    // `t2` is still unassigned, so a second `assign` ranks again, but the import already
    // has its `rank_prior`.
    home.assign("b");
    assert_eq!(named(&home.events("b"), "rank_prior").len(), 1);
    // Goal a's log is read, never written.
    assert!(named(&home.events("a"), "rank_prior").is_empty());
}
