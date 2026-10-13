use hydra_puppets::json::{self, Value};
use std::path::PathBuf;
use std::process::Command;

struct Fixture(PathBuf);
impl Fixture {
    fn new(name: &str) -> Self {
        let path = PathBuf::from("tests").join(format!("builders-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&path).unwrap();
        for (name, contents) in [
            ("a.jsonl", include_str!("fixtures/builders/a.jsonl")),
            ("b.jsonl", include_str!("fixtures/builders/b.jsonl")),
            ("c.jsonl", include_str!("fixtures/builders/c.jsonl")),
        ] {
            std::fs::write(path.join(name), contents).unwrap();
        }
        Self(path)
    }
    fn run(&self, args: &[&str]) -> String {
        let output = Command::new(env!("CARGO_BIN_EXE_hydra-puppets"))
            .args(["report", "builders"])
            .args(args)
            .env("HYDRA_PUPPETS_HOME", &self.0)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            String::from_utf8(output.stderr).unwrap(),
            "goal log c: unterminated tail; counted complete lines\n"
        );
        String::from_utf8(output.stdout).unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}
fn rows(text: &str) -> Vec<Value> {
    text.lines()
        .map(|line| json::loads_str(line).unwrap())
        .collect()
}
fn field(row: &Value, name: &str) -> Value {
    let Value::Dict(d) = row else { panic!("row") };
    d.get(name).unwrap().clone()
}
fn check_rows(actual: &[Value], expected: &[Value]) {
    assert_eq!(actual.len(), expected.len());
    for (actual, expected) in actual.iter().zip(expected) {
        assert_eq!(json::dumps_default(actual), json::dumps_default(expected));
    }
}
#[test]
fn every_column_and_builder_total_in_text_and_json() {
    let fixture = Fixture::new("columns");
    assert_eq!(
        fixture.run(&[]),
        include_str!("fixtures/builders/expected.txt")
    );
    let expected = rows(include_str!("fixtures/builders/expected.jsonl"));
    check_rows(&rows(&fixture.run(&["--json"])), &expected);
    // The reader leaves the torn file byte for byte intact.
    assert_eq!(
        std::fs::read_to_string(fixture.0.join("c.jsonl")).unwrap(),
        include_str!("fixtures/builders/c.jsonl")
    );
    let filtered = rows(&fixture.run(&["--json", "--class", "docs", "--model", "sonnet"]));
    let mut total = expected[0].clone();
    if let Value::Dict(d) = &mut total {
        d.set("total", Value::Bool(true));
        d.set("class", Value::Null);
    }
    check_rows(&filtered, &[expected[0].clone(), total]);
    assert!(fixture.run(&["--json", "--model", "unknown"]).is_empty());
}
#[test]
fn resumed_acceptance_and_repeated_events_do_not_inflate_counts() {
    let fixture = Fixture::new("resumed");
    let log = include_str!("fixtures/builders/a.jsonl")
        .replace("\"result\":\"fixed\"", "\"result\":\"applied\"");
    std::fs::write(fixture.0.join("a.jsonl"), log).unwrap();
    check_rows(
        &rows(&fixture.run(&["--json"])),
        &rows(include_str!("fixtures/builders/expected.jsonl")),
    );
    // A repeated verdict and end are observations of the same attempt.
    let b = include_str!("fixtures/builders/b.jsonl");
    let repeated = b
        .lines()
        .rev()
        .take(2)
        .map(|s| format!("{s}\n"))
        .collect::<String>();
    std::fs::write(fixture.0.join("b.jsonl"), format!("{b}{repeated}")).unwrap();
    let actual = rows(&fixture.run(&["--json"]));
    check_rows(
        &actual,
        &rows(include_str!("fixtures/builders/expected.jsonl")),
    );
    assert!(matches!(
        field(&actual[3], "first_pass_accepted"),
        Value::Int(1)
    ));
}

#[test]
fn an_applied_second_attempt_without_an_earlier_verdict_is_not_first_pass() {
    let fixture = Fixture::new("late-verdict");
    let log = include_str!("fixtures/builders/a.jsonl")
        .lines()
        .filter(|line| !line.contains("\"result\":\"rejected\""))
        .map(|line| format!("{line}\n"))
        .collect::<String>()
        .replace("\"result\":\"fixed\"", "\"result\":\"applied\"");
    std::fs::write(fixture.0.join("a.jsonl"), log).unwrap();
    let mut expected = rows(include_str!("fixtures/builders/expected.jsonl"));
    for (at, rejections) in [(0, 0), (3, 1)] {
        if let Value::Dict(d) = &mut expected[at] {
            d.set("rejections", Value::Int(rejections));
        }
    }
    check_rows(&rows(&fixture.run(&["--json"])), &expected);
}
