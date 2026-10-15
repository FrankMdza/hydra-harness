//! A composed panel verdict is written only when a seat attaches a fenced task array.
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use hydra_puppets::json::{self, Value};

const HELPER: &str = env!("CARGO_BIN_EXE_hydra-puppets");

const STUB: &str = r#"#!/bin/bash
state="$STUB_STATE"
prog=$(basename "$0")
case "$prog" in hydra*) prog=hydra ;; esac
key="$prog-${1:-none}"
mkdir -p "$state/calls" "$state/show"
n=$(( $(cat "$state/count-$key" 2>/dev/null || echo 0) + 1 ))
echo "$n" > "$state/count-$key"
call="$state/calls/$key-$n"
printf '%s\0' "$@" > "$call.argv"
case "$key" in
  hydra-run)
    id="run-$n"
    mkdir -p "$HYDRA_ROOT/runs"
    printf '%s\n' \
      '{"hydra":{"type":"run_start"},"run_id":"'"$id"'","seq":0}' \
      '{"hydra":{"type":"run_end"},"run_id":"'"$id"'","seq":1}' \
      > "$HYDRA_ROOT/runs/$id.jsonl"
    if [ -f "$state/answers/$n" ]; then cp "$state/answers/$n" "$state/show/$id"; fi
    echo "hydra: run $id" >&2
    ;;
  hydra-show)
    if [ -f "$state/show/$2" ]; then
      cat "$state/show/$2"
      exit 0
    fi
    echo "hydra: no run record for $2" >&2
    exit 1
    ;;
esac
"#;

struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn json_string(text: &str) -> String {
    let mut out = String::from("\"");
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            ' '..='~' => out.push(c),
            c => {
                let mut units = [0u16; 2];
                for unit in c.encode_utf16(&mut units) {
                    out.push_str(&format!("\\u{unit:04x}"));
                }
            }
        }
    }
    out.push('"');
    out
}

fn shown(result_text: &str) -> String {
    format!(
        r#"{{"run_id":"shown","outcome":"completed","outcome_source":"provider","exit_code":0,"error":null,"result_text":{},"usage":{{"usage_source":"provider","counts":{{"input_tokens":5,"output_tokens":7,"raw_input":5,"total_tokens":12}}}},"run_file":"/f.jsonl"}}"#,
        json_string(result_text)
    )
}

fn field<'a>(value: &'a Value, key: &str) -> &'a Value {
    match value {
        Value::Dict(dict) => dict.get(key).unwrap_or_else(|| panic!("no {key}")),
        other => panic!("{key} of {other:?}"),
    }
}

fn last_event(log: &str) -> Value {
    json::loads_str(log.lines().last().unwrap()).unwrap()
}

fn data_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../integrations/puppets")
}

fn choose(
    name: &str,
    answers: [&str; 3],
) -> (i32, String, String, String, Option<String>, Vec<u8>) {
    let root = std::env::temp_dir().join(format!(
        "hydra-puppets-choose-{name}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let scratch = Scratch(root);
    let home = &scratch.0;
    for sub in ["puppets", "root/runs", "stub/answers", "bin"] {
        std::fs::create_dir_all(home.join(sub)).unwrap();
    }
    let stub = home.join("bin/hydra-stub");
    std::fs::write(&stub, STUB).unwrap();
    std::fs::set_permissions(&stub, std::fs::Permissions::from_mode(0o755)).unwrap();
    let plans = home.join("plans");
    std::fs::create_dir_all(&plans).unwrap();
    std::fs::write(plans.join("codex.md"), "Codex plan.\n```json\n[]\n```\n").unwrap();
    std::fs::write(plans.join("claude.md"), "Claude plan.\n").unwrap();
    let log = format!(
        "{{\"event\":\"goal\",\"text\":\"test\",\"repo\":\"/repo\",\"base_sha\":\"abc\"}}\n\
         {{\"event\":\"plan_proposed\",\"provider\":\"codex\",\"run_id\":\"p1\"}}\n\
         {{\"event\":\"plan_proposed\",\"provider\":\"claude\",\"run_id\":\"p2\"}}\n\
         {{\"event\":\"plan_ready\",\"provider\":\"codex\",\"run_id\":\"p1\",\"path\":{}}}\n\
         {{\"event\":\"plan_ready\",\"provider\":\"claude\",\"run_id\":\"p2\",\"path\":{}}}\n",
        json_string(plans.join("codex.md").to_str().unwrap()),
        json_string(plans.join("claude.md").to_str().unwrap())
    );
    std::fs::write(home.join("puppets/g1.jsonl"), &log).unwrap();
    for (n, answer) in answers.iter().enumerate() {
        std::fs::write(home.join(format!("stub/answers/{}", n + 1)), shown(answer)).unwrap();
    }
    let result = Command::new(HELPER)
        .args(["choose", "g1", "--judge"])
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("HOME", home)
        .env("HYDRA_PUPPETS_HOME", home.join("puppets"))
        .env("HYDRA_ROOT", home.join("root"))
        .env("HYDRA_GRADER_HOME", home.join("grader"))
        .env("HYDRA_PUPPETS_DATA", data_dir())
        .env("HYDRA_BIN", &stub)
        .env("STUB_STATE", home.join("stub"))
        .output()
        .unwrap();
    let after = std::fs::read_to_string(home.join("puppets/g1.jsonl")).unwrap();
    let composed = std::fs::read_to_string(home.join("puppets/g1/plans/composed.md")).ok();
    let argv = std::fs::read(home.join("stub/calls/hydra-run-1.argv")).unwrap();
    (
        result.status.code().unwrap_or(-1),
        String::from_utf8(result.stdout).unwrap(),
        String::from_utf8(result.stderr).unwrap(),
        after,
        composed,
        argv,
    )
}

#[test]
fn a_composed_answer_with_a_task_array_writes_the_plan() {
    let bare = r#"{"chosen":"composed"}"#;
    let with_block = "{\"chosen\":\"composed\"}\n```json\n[{\"id\":\"a\"}]\n```\n";
    let (code, stdout, stderr, log, composed, argv) =
        choose("with-block", [bare, with_block, bare]);
    let prompt = String::from_utf8_lossy(&argv);
    assert!(prompt.contains("When you choose `composed`"), "{prompt}");
    assert!(prompt.contains("<label>|composed"), "{prompt}");
    assert_eq!(code, 0, "{stderr}");
    assert!(stderr.is_empty(), "{stderr}");
    assert!(stdout.contains("chosen composed -> "), "{stdout}");
    let event = last_event(&log);
    assert_eq!(json::dumps_default(field(&event, "chosen")), "\"composed\"");
    let path = match field(&event, "path") {
        Value::Str(path) => path.to_utf8().unwrap(),
        other => panic!("path {other:?}"),
    };
    assert!(path.ends_with("g1/plans/composed.md"), "{path}");
    let plan = composed.expect("composed.md");
    assert_eq!(
        plan,
        "Composed plan.\n\n```json\n[\n  {\n    \"id\": \"a\"\n  }\n]\n```\n"
    );
}

#[test]
fn composed_without_a_task_array_refuses_and_names_the_gap() {
    let bare = r#"{"chosen":"composed"}"#;
    let (code, stdout, stderr, log, composed, argv) = choose("no-block", [bare, bare, bare]);
    let prompt = String::from_utf8_lossy(&argv);
    assert!(prompt.contains("When you choose `composed`"), "{prompt}");
    assert_eq!(code, 1, "{stderr}");
    assert!(stdout.contains("judge "), "{stdout}");
    assert!(!stdout.contains("chosen composed"), "{stdout}");
    assert_eq!(stderr, "composed chosen but no task block found\n");
    let event = last_event(&log);
    assert!(matches!(field(&event, "chosen"), Value::Null));
    assert!(composed.is_none());
}
