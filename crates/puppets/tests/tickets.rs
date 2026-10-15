use std::os::unix::fs::PermissionsExt;
use std::process::Command;

#[test]
fn tickets_resolve_multiple_ambiguous_missing_and_all_prose_classes() {
    let base =
        std::path::PathBuf::from("tests").join(format!("ticket-report-{}", std::process::id()));
    std::fs::create_dir_all(base.join("tickets")).unwrap();
    std::fs::create_dir_all(base.join("records")).unwrap();
    struct Cleanup(std::path::PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            std::fs::remove_dir_all(&self.0).unwrap();
        }
    }
    let _cleanup = Cleanup(base.clone());
    for (name, outcome) in [
        (
            "20-two.md",
            "Applied as it was on 01a11111-1111-7000-8000-000000000001 and 01a11111-1111-7000-8000-000000000002; repeated 01a11111-1111-7000-8000-000000000001.",
        ),
        ("3-ambiguous.md", "Needed correction on 01a11111-1111."),
        ("4-missing.md", "The session finished run 01a99999-9999."),
        ("5-rejected.md", "Not applied: discarded 01a22222-2222."),
    ] {
        std::fs::write(base.join("tickets").join(name), format!("# Ticket\nType: build\nStatus: resolved\n\n## Outcome\n\n## Outcome\n{outcome}\n\n## Later\nrejected 01afffff-ffff\n")).unwrap();
    }
    for (id, outcome, count) in [
        ("01a11111-1111-7000-8000-000000000001", "completed", 100),
        ("01a11111-1111-7000-8000-000000000002", "failed", 201),
        ("01a22222-2222-7000-8000-000000000003", "gated", 60),
    ] {
        // Fake records carry the precomputed normalized show answer on line one.
        // The stub is a harness reader boundary; production uses hydra show.
        std::fs::write(base.join("records").join(format!("{id}.jsonl")), format!("{{\"provider\":\"codex\",\"model\":\"requested-model\",\"projection\":{{\"outcome\":\"{outcome}\",\"usage\":{{\"counts\":{{\"total_tokens\":{count}}}}}}},\"events\":[{{\"type\":\"run_start\",\"body\":{{\"provider\":\"codex\",\"model\":\"test-model\",\"policy\":\"standard\"}}}}]}}\n{{\"provider\":\"codex\",\"model\":\"test-model\",\"hydra\":{{\"type\":\"run_end\"}}}}\n")).unwrap();
    }
    let stub = base.join("show-stub");
    std::fs::write(&stub, "#!/bin/sh\n[ \"$1\" = show ] && [ \"$3\" = --events ] || exit 2\nhead -n 1 \"$HYDRA_ROOT/runs/$2.jsonl\"\n").unwrap();
    std::fs::set_permissions(&stub, std::fs::Permissions::from_mode(0o755)).unwrap();
    let run = |records: bool| {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_hydra-puppets"));
        cmd.args(["report", "tickets", "--dir"])
            .arg(base.join("tickets"))
            .env("HYDRA_BIN", &stub);
        if records {
            cmd.arg("--runs").arg(base.join("records"));
        }
        let output = cmd.output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.stderr.is_empty());
        String::from_utf8(output.stdout).unwrap()
    };
    let text = run(true);
    let expected = concat!(
        "provider\tmodel\ttickets\truns\ttotal_tokens\tmedian_tokens\tcompleted\tfailed\trest\tapplied as is\tcorrected\tfinished by hand\trejected\n",
        "codex\ttest-model\t2\t3\t361\t100\t1\t1\t1\t1\t0\t0\t1\n",
        "\nticket\truns\tmodel\ttokens\trecord outcome\tprose class\tmatched phrase\tpolicy\n",
        "3\t01a11111-1111 (ambiguous)\tunavailable\tunavailable\tambiguous\tcorrected\tNeeded correction\tunavailable\n",
        "4\t01a99999-9999 (missing)\tunavailable\tunavailable\tmissing\tfinished by hand\tThe session finished\tunavailable\n",
        "5\t01a22222-2222-7000-8000-000000000003\ttest-model\t60\tgated\trejected\tNot applied\tstandard\n",
        "20\t01a11111-1111-7000-8000-000000000001; 01a11111-1111-7000-8000-000000000002\ttest-model; test-model\t301\tcompleted; failed\tapplied as is\tApplied as it was\tstandard; standard\n",
        "\nunsaid: 0 of 4 tickets\n"
    );
    assert_eq!(text, expected);
    let alone = run(false);
    assert!(alone.contains("01a11111-1111 (unavailable)"));
    assert!(alone.contains("unavailable\tunavailable\t4\tunavailable"));
    // Keep four tickets while checking unsaid and an even token median.
    std::fs::write(
        base.join("tickets/5-rejected.md"),
        "Type: build\n\n## Outcome\nNo decision.\n",
    )
    .unwrap();
    let text = run(true);
    assert!(text.contains("codex\ttest-model\t1\t2\t301\t150.5\t1\t1\t0\t1\t0\t0\t0"));
    assert!(text.contains("unsaid: 1 of 4 tickets"));
}

#[test]
fn tickets_removes_its_temporary_root_even_when_show_leaves_directories() {
    use std::path::PathBuf;

    let base = PathBuf::from("tests").join(format!("ticket-cleanup-{}", std::process::id()));
    std::fs::create_dir_all(base.join("tickets")).unwrap();
    std::fs::create_dir_all(base.join("records")).unwrap();
    struct Cleanup(PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            std::fs::remove_dir_all(&self.0).unwrap();
        }
    }
    let _cleanup = Cleanup(base.clone());
    let base = base.canonicalize().unwrap();
    let id = "01a11111-1111-7000-8000-000000000001";
    std::fs::write(
        base.join("tickets/1-cleanup.md"),
        format!("Type: build\nStatus: resolved\n\n## Outcome\nApplied as it was on {id}.\n"),
    )
    .unwrap();
    let record = base.join("records").join(format!("{id}.jsonl"));
    let text = "{\"provider\":\"codex\",\"hydra\":{\"type\":\"run_end\"}}\n";
    std::fs::write(&record, text).unwrap();
    let stub = base.join("show-stub");
    std::fs::write(&stub, "#!/bin/sh\nmkdir -p \"$HYDRA_ROOT/spawn/$2/nested\"\nprintf '%s\\n' '{\"projection\":{\"outcome\":\"completed\"},\"events\":[]}'\n").unwrap();
    std::fs::set_permissions(&stub, std::fs::Permissions::from_mode(0o755)).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_hydra-puppets"))
        .current_dir(&base)
        .args(["report", "tickets", "--dir", "tickets", "--runs", "records"])
        .env("HYDRA_BIN", &stub)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(std::fs::read_dir(&base).unwrap().all(|entry| {
        !entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".puppets-report-")
    }));
    assert_eq!(std::fs::read_to_string(record).unwrap(), text);
}

#[test]
fn tickets_skip_pasted_tables_and_match_widened_phrases() {
    let base =
        std::path::PathBuf::from("tests").join(format!("ticket-phrases-{}", std::process::id()));
    std::fs::create_dir_all(base.join("tickets")).unwrap();
    std::fs::create_dir_all(base.join("records")).unwrap();
    struct Cleanup(std::path::PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            std::fs::remove_dir_all(&self.0).unwrap();
        }
    }
    let _cleanup = Cleanup(base.clone());
    let pasted = concat!(
        "Applied as it was on 01a33333-3333-7000-8000-000000000004.\n\n",
        "```tsv\n",
        "provider\tmodel\trejected\n",
        "agy\tgemini-3.1-pro-high\t01a44444-4444-7000-8000-000000000005\n",
        "```\n\n",
        "| ticket | runs |\n",
        "| 312 | 01a55555-5555-7000-8000-000000000006 |\n",
    );
    // Each new phrase quotes a real Outcome sentence.
    for (name, outcome) in [
        ("400-pasted.md", pasted),
        (
            "401-to-master.md",
            "applied to master on 2026-10-13 after the diff was read whole",
        ),
        ("402-as-written.md", "Applied as written."),
        (
            "403-by-session.md",
            "Built 2026-10-04 by an Opus run (`01a10603`), reviewed and applied by the session.",
        ),
        (
            "404-unchanged.md",
            "applied\nunchanged; the session ran the live checks on master.",
        ),
        (
            "405-session-applied.md",
            "the session applied it as written.",
        ),
        (
            "406-it-as-it-was.md",
            "The session read the diff and applied it as it was: 7 lines in `grader.py`.",
        ),
        ("407-this-run.md", "Built in this run."),
        (
            "408-without-changes.md",
            "applied without changes. Built: `codex_turn_failed`",
        ),
        (
            "409-fixed-by.md",
            "then fixed by the session after an Opus review.",
        ),
        (
            "410-one-change.md",
            "applied by the session with one change: the status message.",
        ),
        (
            "411-one-sentence.md",
            "diff read by the session and applied to master with one sentence added.",
        ),
        (
            "412-session-corrected.md",
            "What the session corrected: G7, where the counts were wrong",
        ),
        (
            "413-corrected-there.md",
            "applied to master and corrected there.",
        ),
        (
            "414-adjusted.md",
            "Applied to master and then adjusted by hand once the real `hydra show` output existed",
        ),
        ("415-fixes.md", "with two fixes by the session."),
        (
            "416-added-change.md",
            "The session added one change to the agent's diff and verified one test by breaking it.",
        ),
    ] {
        std::fs::write(
            base.join("tickets").join(name),
            format!("Type: build\nStatus: resolved\n\n## Outcome\n{outcome}\n"),
        )
        .unwrap();
    }
    let id = "01a33333-3333-7000-8000-000000000004";
    std::fs::write(
        base.join("records").join(format!("{id}.jsonl")),
        "{\"provider\":\"codex\",\"model\":\"requested-model\",\"projection\":{\"outcome\":\"completed\",\"usage\":{\"counts\":{\"total_tokens\":10}}},\"events\":[{\"type\":\"run_start\",\"body\":{\"provider\":\"codex\",\"model\":\"test-model\",\"policy\":\"standard\"}}]}\n{\"provider\":\"codex\",\"model\":\"test-model\",\"hydra\":{\"type\":\"run_end\"}}\n",
    )
    .unwrap();
    let stub = base.join("show-stub");
    std::fs::write(
        &stub,
        "#!/bin/sh\n[ \"$1\" = show ] && [ \"$3\" = --events ] || exit 2\nhead -n 1 \"$HYDRA_ROOT/runs/$2.jsonl\"\n",
    )
    .unwrap();
    std::fs::set_permissions(&stub, std::fs::Permissions::from_mode(0o755)).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_hydra-puppets"))
        .args(["report", "tickets", "--dir"])
        .arg(base.join("tickets"))
        .arg("--runs")
        .arg(base.join("records"))
        .env("HYDRA_BIN", &stub)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(
        text.contains(&format!(
            "400\t{id}\ttest-model\t10\tcompleted\tapplied as is\tApplied as it was\tstandard\n"
        )),
        "{text}"
    );
    assert!(
        !text.contains("01a44444-4444-7000-8000-000000000005"),
        "{text}"
    );
    assert!(
        !text.contains("01a55555-5555-7000-8000-000000000006"),
        "{text}"
    );
    let expected_rows = [
        "401\t-\tunavailable\tunavailable\tunavailable\tapplied as is\tapplied to master\tunavailable",
        "402\t-\tunavailable\tunavailable\tunavailable\tapplied as is\tApplied as written\tunavailable",
        "403\t-\tunavailable\tunavailable\tunavailable\tapplied as is\tapplied by the session\tunavailable",
        "404\t-\tunavailable\tunavailable\tunavailable\tapplied as is\tapplied unchanged\tunavailable",
        "405\t-\tunavailable\tunavailable\tunavailable\tapplied as is\tthe session applied\tunavailable",
        "406\t-\tunavailable\tunavailable\tunavailable\tapplied as is\tapplied it as it was\tunavailable",
        "407\t-\tunavailable\tunavailable\tunavailable\tapplied as is\tBuilt in this run\tunavailable",
        "408\t-\tunavailable\tunavailable\tunavailable\tapplied as is\tapplied without changes\tunavailable",
        "409\t-\tunavailable\tunavailable\tunavailable\tcorrected\tfixed by the session\tunavailable",
        "410\t-\tunavailable\tunavailable\tunavailable\tcorrected\twith one change\tunavailable",
        "411\t-\tunavailable\tunavailable\tunavailable\tcorrected\twith one sentence\tunavailable",
        "412\t-\tunavailable\tunavailable\tunavailable\tcorrected\tthe session corrected\tunavailable",
        "413\t-\tunavailable\tunavailable\tunavailable\tcorrected\tcorrected there\tunavailable",
        "414\t-\tunavailable\tunavailable\tunavailable\tcorrected\tadjusted by hand\tunavailable",
        "415\t-\tunavailable\tunavailable\tunavailable\tcorrected\tfixes by the session\tunavailable",
        "416\t-\tunavailable\tunavailable\tunavailable\tcorrected\tadded one change\tunavailable",
    ];
    for row in expected_rows {
        assert!(text.contains(row), "missing {row} in {text}");
    }
    assert!(text.contains("unsaid: 0 of 17 tickets"), "{text}");
}
