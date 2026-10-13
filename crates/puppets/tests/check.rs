//! A draft gets the import checks without a goal or any home files.
use std::path::PathBuf;
use std::process::Command;

struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn check_reports_import_offences_without_a_goal() {
    let root = PathBuf::from(format!("check-plan-{}", std::process::id()));
    std::fs::create_dir(&root).unwrap();
    let scratch = Scratch(root);
    // Resolving or writing a goal under this home would fail: it is a regular file.
    let home = scratch.0.join("home");
    std::fs::write(&home, "unchanged").unwrap();
    let plan = scratch.0.join("plan.md");
    let clean = r#"[{"id":"a","text":"a","class":"docs","grade":"low","deps":["b"],"done":"verified"},
        {"id":"b","text":"b","class":"docs","grade":"low","done":""},
        {"id":"c","text":"c","class":"docs","grade":"low"}]"#;
    for (array, expected_stderr) in [
        (clean, ""),
        (
            "[{\"id\":\"a\",\"text\":\"raw\nnewline\",\"class\":\"docs\",\"grade\":\"low\"}]",
            "hydra-puppets: JSONDecodeError: the plan's task array\n",
        ),
        (
            r#"[{"id":"a","text":"a", // comment
                "class":"docs","grade":"low"}]"#,
            "hydra-puppets: JSONDecodeError: the plan's task array\n",
        ),
        (
            r#"[{"id":"a","text":"a","class":"docs","grade":"low","deps":["missing"]}]"#,
            "task 'a' has unknown dep 'missing'\n",
        ),
        (
            r#"[{"id":"a","text":"a","class":"docs","grade":"low","deps":["b"]},
                {"id":"b","text":"b","class":"docs","grade":"low","deps":["a"]}]"#,
            "dependency cycle includes task 'a'\ndependency cycle includes task 'b'\n",
        ),
        (
            r#"[{"id":"a","text":"a","class":"docs"}]"#,
            "task a lacks grade\n",
        ),
        (
            r#"[{"id":"a","text":"a","class":"unknown","grade":"low"}]"#,
            "task a has an unknown class or grade\n",
        ),
        (
            r#"[{"id":1,"text":"a","class":"docs","grade":"low"},
                {"id":1.0,"text":"b","class":"docs","grade":"low"}]"#,
            "duplicate task id 1.0\n",
        ),
        ("[]", ""),
    ] {
        let contents = format!("```json\n{array}\n```\n");
        std::fs::write(&plan, &contents).unwrap();
        let result = Command::new(env!("CARGO_BIN_EXE_hydra-puppets"))
            .arg("check")
            .arg(&plan)
            .env_clear()
            .env("HYDRA_PUPPETS_HOME", &home)
            .env("HYDRA_ROOT", &home)
            .env("HYDRA_GRADER_HOME", &home)
            .output()
            .unwrap();
        assert_eq!(
            result.status.code(),
            Some(i32::from(!expected_stderr.is_empty())),
            "{array}"
        );
        assert_eq!(
            String::from_utf8(result.stderr).unwrap(),
            expected_stderr,
            "{array}"
        );
        let stdout = String::from_utf8(result.stdout).unwrap();
        if array == clean {
            assert_eq!(stdout, "3 tasks, 2 with empty done\n");
        } else if array == "[]" {
            assert_eq!(stdout, "0 tasks, 0 with empty done\n");
        } else {
            assert!(stdout.is_empty(), "{array}: {stdout}");
        }
        assert_eq!(std::fs::read_to_string(&plan).unwrap(), contents);
        assert_eq!(std::fs::read_to_string(&home).unwrap(), "unchanged");
        assert_eq!(std::fs::read_dir(&scratch.0).unwrap().count(), 2);
    }
}
