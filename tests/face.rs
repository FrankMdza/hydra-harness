//! The MCP delegate face, harness spec 9, driven the way a provider drives it.
//!
//! Every test here spawns the built binary as `hydra delegate-face` and speaks
//! JSON-RPC 2.0 to it over a pipe. That is the whole point: the face is a process
//! an orchestrator's own provider starts, so a test that called the functions
//! in-process would check everything except the thing that breaks, which is a
//! stray line on stdout.
//!
//! **No provider is launched, and none has to be installed.** The one test that
//! reaches `hydra run` names a policy its parent does not contain, and 6.1 item 3
//! refuses that before the environment is built or the binary is asked for its
//! version. So the whole file runs on a machine with no agent CLI on it at all.
//!
//! `HYDRA_ROOT` points the whole thing at a throwaway `~/hydra`. It exists for
//! this file and `src/paths.rs` says so.

use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

use serde_json::{Value, json};

const PARENT: &str = "01a07610-0000-7000-8000-0000000000c1";

/// A throwaway `~/hydra` with `runs/` in it, one per test.
fn root() -> PathBuf {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let root = std::env::temp_dir().join(format!(
        "hydra-face-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("runs")).unwrap();
    root
}

/// The parent's record: the two lines a launcher writes, by hand.
fn write_parent(root: &Path, run_id: &str, policy: &str, depth: u32) {
    let line = |seq: u64, hydra: Value| {
        let mut text = json!({
            "run_id": run_id,
            "parent_run_id": null,
            "seq": seq,
            "ts": "2026-09-06T09:12:04.100Z",
            "provider": "claude",
            "model": null,
            "secret_flag": null,
            "hydra": hydra,
        })
        .to_string();
        text.push('\n');
        text
    };
    let mut text = line(
        0,
        json!({
            "type": "run_start",
            "schema_version": 1,
            "hydra_version": "0.2.0",
            "command": "run",
            "launch_mode": "recorded",
            "provider_version": "stub",
            "provider_session_id": null,
            "task": "the orchestrator's own task",
            "policy": policy,
            "policy_sha": null,
            "cwd": "/tmp",
            "worktree": null,
            "depth": depth,
            "pid": null,
            "process_start_time": null,
            "subscription": "anthropic",
        }),
    );
    text.push_str(&line(
        1,
        json!({"type": "run_end", "exit_code": 0, "body_source": "stream"}),
    ));
    std::fs::write(root.join("runs").join(format!("{run_id}.jsonl")), text).unwrap();
}

/// The face as a client sees it: a process, a pipe in and a pipe out.
struct Face {
    child: Child,
    input: ChildStdin,
    output: BufReader<ChildStdout>,
    root: PathBuf,
}

impl Face {
    fn start(parent: Option<&str>) -> Self {
        Self::spawn(parent, None)
    }

    /// The face wrapped in a minimal seatbelt profile, the way a macOS run inherits one.
    fn start_under_seatbelt(parent: Option<&str>) -> Self {
        let root = root();
        let profile = root.join("min.sb");
        std::fs::write(&profile, "(version 1)\n(allow default)\n").unwrap();
        Self::spawn(parent, Some(profile))
    }

    fn spawn(parent: Option<&str>, profile: Option<PathBuf>) -> Self {
        let root = root();
        let mut command = if let Some(profile) = profile {
            let mut wrapped = Command::new("/usr/bin/sandbox-exec");
            wrapped.arg("-f").arg(profile);
            wrapped.arg(env!("CARGO_BIN_EXE_hydra"));
            wrapped
        } else {
            Command::new(env!("CARGO_BIN_EXE_hydra"))
        };
        command.arg("delegate-face");
        if let Some(parent) = parent {
            command.arg("--parent").arg(parent);
        }
        let mut child = command
            .env("HYDRA_ROOT", &root)
            // Outside a git repo, so a launch that got past the checks would have
            // no worktree to create. Nothing here gets that far.
            .current_dir(&root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap();
        let input = child.stdin.take().unwrap();
        let output = BufReader::new(child.stdout.take().unwrap());
        Self {
            child,
            input,
            output,
            root,
        }
    }

    /// One request, one response. The face answers in the order it was asked.
    fn call(&mut self, id: u64, method: &str, params: Value) -> Value {
        let request = json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params});
        writeln!(self.input, "{request}").unwrap();
        self.input.flush().unwrap();

        let mut line = String::new();
        self.output.read_line(&mut line).unwrap();
        assert!(!line.trim().is_empty(), "{method} answered nothing");
        let response: Value = serde_json::from_str(&line)
            .unwrap_or_else(|err| panic!("{method} answered {line:?}, which is not JSON: {err}"));
        assert_eq!(response["jsonrpc"], "2.0");
        assert_eq!(response["id"], id, "answered the wrong request");
        response
    }

    /// A notification gets no answer, so nothing is read.
    fn notify(&mut self, method: &str) {
        let request = json!({"jsonrpc": "2.0", "method": method});
        writeln!(self.input, "{request}").unwrap();
        self.input.flush().unwrap();
    }

    /// The initialize handshake, which every client does first.
    fn initialize(&mut self) -> Value {
        let answer = self.call(
            1,
            "initialize",
            json!({
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": {"name": "tests/face.rs", "version": "0"},
            }),
        );
        self.notify("notifications/initialized");
        answer
    }

    fn delegate(&mut self, id: u64, arguments: Value) -> Value {
        self.call(
            id,
            "tools/call",
            json!({"name": "delegate", "arguments": arguments}),
        )
    }
}

impl Drop for Face {
    fn drop(&mut self) {
        // Closing stdin ends the loop, which is how a provider stops one of these.
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

#[test]
fn the_handshake_answers_the_protocol_version_the_client_sent() {
    let mut face = Face::start(None);
    let answer = face.initialize();
    assert_eq!(answer["result"]["protocolVersion"], "2025-06-18");
    assert_eq!(answer["result"]["serverInfo"]["name"], "hydra");
    assert!(
        answer["result"]["capabilities"]["tools"].is_object(),
        "tools is the only capability this face has: {answer}"
    );
}

#[test]
fn the_face_lists_one_tool_with_four_arguments_and_no_model() {
    // Harness spec 9: `delegate(provider, task, files?, policy?)` and nothing
    // more. The four that are missing are how a delegation escapes its worktree,
    // outlives its budget, spawns a tree or drops to an older model, and they are
    // human decisions rather than per-call ones a model makes unattended.
    let mut face = Face::start(None);
    face.initialize();

    let listed = face.call(2, "tools/list", json!({}));
    let tools = listed["result"]["tools"].as_array().unwrap();
    assert_eq!(tools.len(), 1, "{listed}");
    assert_eq!(tools[0]["name"], "delegate");

    let schema = &tools[0]["inputSchema"];
    let properties = schema["properties"].as_object().unwrap();
    let mut named: Vec<&String> = properties.keys().collect();
    named.sort();
    assert_eq!(
        named,
        vec!["files", "policy", "provider", "task"],
        "{schema}"
    );
    assert_eq!(
        schema["required"].as_array().unwrap(),
        &vec![json!("provider"), json!("task")]
    );
    assert_eq!(schema["properties"]["files"]["type"], "array");
    assert_eq!(schema["properties"]["files"]["items"]["type"], "string");
    assert_eq!(schema["additionalProperties"], false);
    // The description says which binaries there are, because a model that guesses
    // a provider name gets an error rather than a run.
    let providers = schema["properties"]["provider"]["enum"].as_array().unwrap();
    assert_eq!(providers.len(), 3, "{schema}");
}

#[test]
fn a_method_the_face_does_not_know_is_a_json_rpc_error() {
    // Everything the model can get wrong comes back as a tool result it reads.
    // A method that does not exist is the client's mistake, not the model's, and
    // the protocol has a code for it.
    let mut face = Face::start(None);
    face.initialize();

    let answer = face.call(2, "resources/list", json!({}));
    assert_eq!(answer["error"]["code"], -32601, "{answer}");
    assert!(answer.get("result").is_none());
}

#[test]
fn a_tool_this_face_does_not_have_is_a_result_rather_than_a_protocol_error() {
    // The model picks the tool name, so getting it wrong is the model's mistake to
    // read. Only a method it never chose is an error in the protocol's sense.
    let mut face = Face::start(None);
    face.initialize();

    let answer = face.call(2, "tools/call", json!({"name": "spawn", "arguments": {}}));
    assert!(answer.get("error").is_none(), "{answer}");
    assert_eq!(answer["result"]["isError"], true, "{answer}");
    let text = answer["result"]["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("delegate"), "{text}");
}

#[test]
fn a_provider_that_is_not_one_of_the_three_is_an_error_the_model_reads() {
    // `isError` rather than a JSON-RPC error, so the sentence lands in the
    // model's context and it can try again with a name that exists.
    let mut face = Face::start(None);
    face.initialize();

    let answer = face.delegate(2, json!({"provider": "gemini", "task": "count to three"}));
    assert!(answer.get("error").is_none(), "{answer}");
    assert_eq!(answer["result"]["isError"], true, "{answer}");
    let text = answer["result"]["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("gemini"), "{text}");
}

#[test]
fn an_argument_the_face_does_not_take_is_refused_rather_than_ignored() {
    // Harness spec 9 keeps `model` off this face on purpose: `agy` serves a model
    // three releases behind what `claude --model opus` resolves to, so a `model`
    // argument is a lever for dodging a limit the run noticed. Silently dropping
    // it would tell the model it got what it asked for.
    let mut face = Face::start(None);
    face.initialize();

    let answer = face.delegate(
        2,
        json!({"provider": "claude", "task": "count to three", "model": "opus"}),
    );
    assert_eq!(answer["result"]["isError"], true, "{answer}");
    let text = answer["result"]["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("model"), "{text}");
}

#[test]
fn delegation_from_inside_a_seatbelt_is_refused_in_hydras_words() {
    let mut face = if hydra::sandbox::inside_seatbelt() {
        // The test process already inherited a seatbelt, and nested sandbox-exec
        // is refused on this machine, so the child inherits what we have.
        Face::start(None)
    } else if hydra::sandbox::exec_available() {
        Face::start_under_seatbelt(None)
    } else {
        return;
    };
    face.initialize();

    let answer = face.delegate(2, json!({"provider": "claude", "task": "count to three"}));
    assert!(answer.get("error").is_none(), "{answer}");
    assert_eq!(answer["result"]["isError"], true, "{answer}");
    let text = answer["result"]["content"][0]["text"].as_str().unwrap();
    assert!(text.starts_with("hydra/seatbelt-delegation:"), "{text}");
    assert!(
        text.contains("orchestrator") && text.contains("outside"),
        "{text}"
    );
    assert!(text.contains("64"), "{text}");
    // Build ticket 247: the refusal ends with the way out. This run has no
    // launcher, so the way out is the human and not the `panel` tool.
    assert!(
        text.ends_with("ask the human to launch the child from outside this run with `hydra run`."),
        "{text}"
    );
    assert_eq!(
        std::fs::read_dir(face.root.join("runs")).unwrap().count(),
        0,
        "no child record was written"
    );
}

#[test]
fn a_child_wider_than_its_parent_comes_back_as_a_result_and_a_record() {
    if hydra::sandbox::inside_seatbelt() {
        return;
    }
    // Contract 8.3 through the face: the refusal is a run record and a result the
    // model reads, rather than a protocol error it cannot act on.
    let mut face = Face::start(Some(PARENT));
    write_parent(&face.root, PARENT, "read-only", 0);
    face.initialize();

    let answer = face.delegate(
        2,
        json!({
            "provider": "claude",
            "task": "rewrite the parser",
            "policy": "standard",
            "files": ["src/parser.rs"],
        }),
    );
    assert!(answer.get("error").is_none(), "{answer}");
    assert_eq!(
        answer["result"]["isError"], false,
        "a refused child is a result, not a failure of the call: {answer}"
    );

    let text = answer["result"]["content"][0]["text"].as_str().unwrap();
    let result: Value = serde_json::from_str(text).unwrap();
    assert_eq!(result["outcome"], "gated", "{text}");
    assert_eq!(result["outcome_source"], "refusal", "{text}");

    let run_id = result["run_id"].as_str().unwrap();
    let record = face.root.join("runs").join(format!("{run_id}.jsonl"));
    let written = std::fs::read_to_string(&record).unwrap();
    hydra::record::validate::check(&written, Some(run_id)).assert_ok();

    let start: Value = serde_json::from_str(written.lines().next().unwrap()).unwrap();
    assert_eq!(start["parent_run_id"], PARENT, "contract 4's edge");
    assert_eq!(start["hydra"]["depth"], 1);
    assert_eq!(start["hydra"]["policy"], "standard");
    assert_eq!(start["hydra"]["parent_policy"], "read-only");
    // Contract 8.3's "no provider fields". Nothing ran `claude --version`, which
    // is what lets this test pass on a machine with no provider installed.
    assert!(start["hydra"]["provider_version"].is_null(), "{start}");
    assert_eq!(
        start["provider"], "claude",
        "contract 4 still names the binary"
    );
    // The face's `files` is text in the task and nothing else, so the record
    // shows exactly what the child would have been told.
    assert!(
        start["hydra"]["task"]
            .as_str()
            .unwrap()
            .contains("src/parser.rs"),
        "{start}"
    );
}
