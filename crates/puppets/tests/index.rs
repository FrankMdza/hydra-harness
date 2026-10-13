use std::path::PathBuf;
use std::process::Command;

#[test]
fn build_index_matches_the_plan_on_three_tickets() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixture = manifest.join("tests/fixtures/index");
    let plan = include_str!("fixtures/index/.scratch/hydra/build/plan.md");
    let expected = plan
        .split_once("## The tickets\n\n")
        .unwrap()
        .1
        .split_once("\n## Out of scope")
        .unwrap()
        .0;
    for (cwd, args) in [
        (fixture.join(".scratch/hydra/build"), vec![]),
        (
            manifest,
            vec!["--dir", "tests/fixtures/index/.scratch/hydra/build"],
        ),
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_hydra-puppets"))
            .args(["index", "build"])
            .args(args)
            .current_dir(cwd)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.stderr.is_empty());
        assert_eq!(output.stdout, expected.as_bytes());
    }
}
