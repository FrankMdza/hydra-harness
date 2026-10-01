//! Assignment consumes a sheet the Rust grader just built, with Python's recorded rank.
use hydra_grader::{labels::Env, sheet};
use hydra_pyjson::{
    json::{self, Value},
    models, py,
};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

struct Layout(PathBuf);
impl Drop for Layout {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn field<'a>(v: &'a Value, k: &str) -> &'a Value {
    py::item(v, k).unwrap()
}
fn text(v: &Value) -> String {
    py::str(v).lossy()
}
fn list(v: &Value) -> &[Value] {
    let Value::List(v) = v else { panic!("list") };
    v
}
fn load(p: &Path) -> Value {
    json::loads_str(&std::fs::read_to_string(p).unwrap()).unwrap()
}

#[test]
fn assign_ranks_sheets_written_by_grader_as_python_did() {
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("../grader/tests/fixtures");
    let reference = load(&fixtures.join("sheet/assignment.json"));
    let scenarios = load(&fixtures.join("sheet/scenarios.json"));
    let case = &list(field(&scenarios, "cases"))[0];
    let path = std::env::temp_dir().join(format!("sheet-assign-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).unwrap();
    let layout = Layout(path.canonicalize().unwrap());
    let root = &layout.0;
    let Value::Dict(input) = field(case, "input") else {
        panic!("input")
    };
    for (path, value) in input.iter() {
        let path = root.join(path.lossy());
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text(value)).unwrap();
    }
    std::fs::set_permissions(
        root.join("bin/hydra"),
        std::fs::Permissions::from_mode(0o755),
    )
    .unwrap();
    let aliases = models::load_aliases(&root.join("data/aliases.json")).unwrap();
    let vendors = load(&root.join("data/vendors.json"));
    let home = root.join("home");
    let sheets = sheet::build(
        &root.join("root/runs"),
        &home,
        &vendors,
        &root.join("bin/hydra").display().to_string(),
        1.0,
        &aliases,
        &Env::from_pairs([("PATH", "/usr/bin:/bin")]),
    )
    .unwrap();
    let Value::Dict(expected) = field(&list(field(case, "steps"))[0], "files") else {
        panic!("files")
    };
    for (path, value) in expected
        .iter()
        .filter(|(path, _)| path.lossy().starts_with("home/agents/"))
    {
        assert_eq!(
            std::fs::read_to_string(root.join(path.lossy())).unwrap(),
            text(value)
        );
    }
    std::fs::write(
        root.join("data/priors.json"),
        json::dumps(field(&reference, "priors"), Some(2)),
    )
    .unwrap();
    let goal = list(field(&reference, "goal"));
    let ranked = hydra_puppets::sheet::rank(
        &sheets,
        &Value::text("rust_launch_path"),
        &Value::text("medium"),
        goal,
        "standard",
        &root.join("data/priors.json"),
    )
    .unwrap();
    let recorded = list(field(&reference, "ranked"));
    assert_eq!(ranked.len(), recorded.len());
    for (row, expected) in ranked.iter().zip(recorded) {
        assert!(py::eq(&row.provider, field(expected, "provider")));
        assert!(py::eq(&row.model, field(expected, "model")));
        assert_eq!(
            row.bound.to_bits(),
            match field(expected, "bound") {
                Value::Float(n) => n.to_bits(),
                _ => panic!("bound"),
            }
        );
        assert_eq!(
            row.score.unwrap().to_bits(),
            match field(expected, "score") {
                Value::Float(n) => n.to_bits(),
                _ => panic!("score"),
            }
        );
    }
    let log = goal
        .iter()
        .map(|v| json::dumps_default(v) + "\n")
        .collect::<String>();
    std::fs::write(home.join("g.jsonl"), log).unwrap();
    let done = Command::new(env!("CARGO_BIN_EXE_hydra-puppets"))
        .args(["assign", "g"])
        .env_clear()
        .env("HYDRA_PUPPETS_HOME", &home)
        .env("HYDRA_PUPPETS_DATA", root.join("data"))
        .env("HYDRA_ROOT", root.join("root"))
        .env("HYDRA_GRADER_HOME", root.join("grader"))
        .env("PATH", "/usr/bin:/bin")
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert!(
        done.status.success(),
        "{}",
        String::from_utf8_lossy(&done.stderr)
    );
    assert!(
        String::from_utf8(done.stdout)
            .unwrap()
            .contains("next: claude sonnet (ranking)")
    );
}
