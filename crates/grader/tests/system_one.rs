//! Build ticket 289: `hydra-grader system-one` fits a logistic head over task embeddings
//! from a stub `/api/embed` server and a synthetic dataset. Nothing here reaches a real
//! Ollama or `~/hydra`.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use std::time::Duration;

use hydra_pyjson::json::{self, Value};

const RUST: &str = env!("CARGO_BIN_EXE_hydra-grader");
const EARLY_MS: u64 = 1_788_220_800_000;
const CALIB_MS: u64 = 1_789_430_400_000;
const TEST_MS: u64 = 1_790_208_000_000;

struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Scratch {
        let path = std::env::temp_dir().join(format!(
            "hydra-grader-system-one-{name}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        Scratch(path)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[derive(Debug)]
struct Request {
    path: String,
    body: String,
}

struct Server {
    port: u16,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<Vec<Request>>>,
}

impl Server {
    fn start() -> Server {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        listener.set_nonblocking(true).unwrap();
        let stop = Arc::new(AtomicBool::new(false));
        let stopped = Arc::clone(&stop);
        let thread = std::thread::spawn(move || {
            let mut requests = Vec::new();
            while !stopped.load(Ordering::Relaxed) {
                let Ok((stream, _)) = listener.accept() else {
                    std::thread::sleep(Duration::from_millis(2));
                    continue;
                };
                let mut stream = stream;
                stream.set_nonblocking(false).unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(10)))
                    .unwrap();
                let request = read_request(&mut stream);
                // Ollama answers an empty input with no vector at all.
                let body = if request.body.contains("\"input\": \"\"") {
                    "{\"embeddings\":[]}".to_string()
                } else {
                    format!("{{\"embeddings\":[{}]}}", embedding_for(&request.body))
                };
                let reply = format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = stream.write_all(reply.as_bytes());
                requests.push(request);
            }
            requests
        });
        Server {
            port,
            stop,
            thread: Some(thread),
        }
    }

    fn close(mut self) -> Vec<Request> {
        self.stop.store(true, Ordering::Relaxed);
        self.thread.take().unwrap().join().unwrap()
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn read_request(stream: &mut TcpStream) -> Request {
    let mut buf = Vec::new();
    let mut tmp = [0u8; 1024];
    loop {
        let read = stream.read(&mut tmp).unwrap_or(0);
        if read == 0 {
            break;
        }
        buf.extend_from_slice(&tmp[..read]);
        if let Some(at) = find_headers_end(&buf) {
            let head = String::from_utf8_lossy(&buf[..at]).into_owned();
            let length = content_length(&head).unwrap_or(0);
            while buf.len() < at + length {
                let read = stream.read(&mut tmp).unwrap_or(0);
                if read == 0 {
                    break;
                }
                buf.extend_from_slice(&tmp[..read]);
            }
            let path = head
                .lines()
                .next()
                .unwrap_or_default()
                .split_whitespace()
                .nth(1)
                .unwrap_or_default()
                .to_string();
            let body = String::from_utf8_lossy(&buf[at..]).into_owned();
            return Request { path, body };
        }
    }
    Request {
        path: String::new(),
        body: String::new(),
    }
}

fn find_headers_end(buf: &[u8]) -> Option<usize> {
    buf.windows(4)
        .position(|w| w == b"\r\n\r\n")
        .map(|at| at + 4)
}

fn content_length(head: &str) -> Option<usize> {
    for line in head.lines() {
        let (name, value) = line.split_once(':')?;
        if name.eq_ignore_ascii_case("content-length") {
            return value.trim().parse().ok();
        }
    }
    None
}

fn embedding_for(body: &str) -> &'static str {
    if body.contains("fail") {
        "[1.0, 0.0, 0.0, 0.0]"
    } else {
        "[0.0, 1.0, 0.0, 0.0]"
    }
}

fn uuid(millis: u64, n: u32) -> String {
    format!(
        "{:08x}-{:04x}-7{:03x}-8{:03x}-{:012x}",
        (millis >> 16) as u32,
        millis as u16,
        0u16,
        0u16,
        u64::from(n)
    )
}

fn write_dataset(path: &std::path::Path) {
    let mut n = 0u32;
    let mut lines = String::new();
    for (millis, _) in [(EARLY_MS, "fit"), (CALIB_MS, "calib"), (TEST_MS, "test")] {
        for i in 0..8u32 {
            n += 1;
            let fail = i % 2 == 0;
            let provider = if i % 4 < 2 { "codex" } else { "claude" };
            let task = if fail {
                "fail: the agent should miss this"
            } else {
                "ok: add a passing test"
            };
            let outcome = if fail { "failed" } else { "completed" };
            let id = uuid(millis + u64::from(i) * 1000, n);
            lines.push_str(&format!(
                "{{\"run_id\":\"{id}\",\"provider\":\"{provider}\",\"model\":\"gpt\",\"policy\":\"standard\",\"task_text\":\"{task}\",\"outcome\":\"{outcome}\"}}\n"
            ));
        }
    }
    std::fs::write(path, lines).unwrap();
}

fn run_cmd(scratch: &Scratch, url: &str, model: &str, extra: &[&str]) -> std::process::Output {
    let dataset = scratch.0.join("dataset.jsonl");
    let mut args = vec![
        "system-one".to_string(),
        "--dataset".to_string(),
        dataset.display().to_string(),
        "--home".to_string(),
        scratch.0.display().to_string(),
        "--test-from".to_string(),
        "2026-09-23".to_string(),
    ];
    args.extend(extra.iter().map(|s| (*s).to_string()));
    Command::new(RUST)
        .args(&args)
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("HOME", scratch.0.join("no-home"))
        .env("HYDRA_ROOT", scratch.0.join("no-hydra"))
        .env("HYDRA_GRADER_HOME", &scratch.0)
        .env("HYDRA_LOCAL_LLM_URL", url)
        .env("HYDRA_EMBED_MODEL", model)
        .stdin(Stdio::null())
        .output()
        .unwrap()
}

fn number(body: &Value, path: &[&str]) -> f64 {
    let mut cur = body;
    for key in path {
        cur = match cur {
            Value::Dict(dict) => dict.get(key).unwrap_or(&Value::Null),
            _ => panic!("not an object at {key}"),
        };
    }
    match cur {
        Value::Float(x) => *x,
        Value::Int(n) => *n as f64,
        other => panic!("not a number: {other:?}"),
    }
}

fn flag(body: &Value, path: &[&str]) -> bool {
    let mut cur = body;
    for key in path {
        cur = match cur {
            Value::Dict(dict) => dict.get(key).unwrap_or(&Value::Null),
            _ => panic!("not an object at {key}"),
        };
    }
    matches!(cur, Value::Bool(true))
}

#[test]
fn a_stub_embed_server_trains_an_outcome_head_that_beats_the_provider_rate() {
    let scratch = Scratch::new("train");
    write_dataset(&scratch.0.join("dataset.jsonl"));
    let server = Server::start();
    let url = format!("http://127.0.0.1:{}/api/embed", server.port);
    let ran = run_cmd(&scratch, &url, "nomic-embed-text", &[]);
    let requests = server.close();
    assert_eq!(
        ran.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&ran.stderr)
    );
    assert!(!requests.is_empty(), "the stub received no embed calls");
    for request in &requests {
        assert_eq!(request.path, "/api/embed", "{:?}", request.path);
        assert!(
            request.body.contains("nomic-embed-text"),
            "{}",
            request.body
        );
        assert!(request.body.contains("\"input\""), "{}", request.body);
    }
    let out = String::from_utf8(ran.stdout).unwrap();
    let report = json::loads_str(&out).unwrap();
    assert!(number(&report, &["model", "rank_auc"]) > 0.9, "{out}");
    assert!(
        number(&report, &["model", "brier"]) < number(&report, &["provider_base_rate", "brier"]),
        "{out}"
    );
    assert!(flag(&report, &["beats_provider_base", "brier"]), "{out}");
    assert!(number(&report, &["temperature"]) > 0.0, "{out}");
    assert_eq!(number(&report, &["embed_dim"]), 4.0);
}

#[test]
fn the_embed_model_env_is_sent_to_the_stub() {
    let scratch = Scratch::new("model");
    write_dataset(&scratch.0.join("dataset.jsonl"));
    let server = Server::start();
    let url = format!("http://127.0.0.1:{}/api/embed", server.port);
    let ran = run_cmd(&scratch, &url, "stub-embed:v0", &[]);
    let requests = server.close();
    assert_eq!(
        ran.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&ran.stderr)
    );
    assert!(
        requests
            .iter()
            .all(|request| request.body.contains("stub-embed:v0")),
        "{requests:?}"
    );
}

#[test]
fn a_host_that_is_not_this_machine_is_refused_before_a_socket_opens() {
    let scratch = Scratch::new("remote");
    write_dataset(&scratch.0.join("dataset.jsonl"));
    let ran = run_cmd(
        &scratch,
        "http://example.com/api/embed",
        "nomic-embed-text",
        &[],
    );
    assert_eq!(ran.status.code(), Some(1));
    let err = String::from_utf8_lossy(&ran.stderr);
    assert!(err.contains("this machine"), "{err}");
}

/// 27 of 855 rows in the real dataset have no task text, and Ollama answers an empty input
/// with no vector. Those rows get zeros instead of an embed call, and the run finishes.
#[test]
fn a_run_with_no_task_text_is_not_sent_and_does_not_stop_the_fit() {
    let scratch = Scratch::new("empty");
    let path = scratch.0.join("dataset.jsonl");
    write_dataset(&path);
    let text = std::fs::read_to_string(&path).unwrap();
    let blanked: String = text
        .lines()
        .enumerate()
        .map(|(n, line)| {
            if n % 5 == 1 {
                line.replace("ok: add a passing test", "")
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    assert!(blanked.contains("\"task_text\":\"\""), "no row was blanked");
    std::fs::write(&path, blanked + "\n").unwrap();
    let server = Server::start();
    let url = format!("http://127.0.0.1:{}/api/embed", server.port);
    let ran = run_cmd(&scratch, &url, "nomic-embed-text", &[]);
    let requests = server.close();
    assert_eq!(
        ran.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&ran.stderr)
    );
    assert!(
        requests
            .iter()
            .all(|request| !request.body.contains("\"input\": \"\"")),
        "an empty task was sent"
    );
}
