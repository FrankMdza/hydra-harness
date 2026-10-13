//! `hydra-local-llm` must match the recorded Python MCP server responses.
//!
//! `record_ask.py` ran the Python once per case of `fixtures/ask.json`, from a scratch
//! directory and an empty environment, against a stub Ollama on a free port, and kept what
//! it gave. This test plants nothing, runs the built binary the same way, and compares. It
//! starts no `python3`.
//!
//! What is compared, and it must be equal to the byte: the exit code, stdout, stderr, and
//! the body of every POST the stub received.
//!
//! These are normalized, and only these:
//! - The stub server's port, which is `<port>` in the fixture.
//! - The OS text after `ollama is unreachable: `, which is `<os>`.
//!
//! One recorded case is a stated difference (map 95, question C): a URL that names any host
//! but this machine's loopback. The Python connects and reports it unreachable; the Rust
//! refuses before a socket opens, with a sentence that says the server has to be on this
//! machine.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use std::time::Duration;

use hydra_pyjson::json::{self, Value};

const RUST: &str = env!("CARGO_BIN_EXE_hydra-local-llm");
const UNEXPECTED: &[u8] = b"HTTP/1.0 599 Unexpected\r\nContent-Length: 0\r\n\r\n";
const UNREACHABLE: &str = "ollama is unreachable: ";

fn field<'a>(value: &'a Value, key: &str) -> &'a Value {
    match value {
        Value::Dict(dict) => dict.get(key).unwrap_or(&Value::Null),
        _ => &Value::Null,
    }
}

fn string(value: &Value) -> String {
    match value {
        Value::Str(text) => text.to_utf8().expect("fixture text is UTF-8"),
        other => panic!("not a string: {other:?}"),
    }
}

fn strings(value: &Value) -> Vec<String> {
    match value {
        Value::List(items) => items.iter().map(string).collect(),
        _ => Vec::new(),
    }
}

fn unhex(text: &str) -> Vec<u8> {
    (0..text.len())
        .step_by(2)
        .map(|at| u8::from_str_radix(&text[at..at + 2], 16).unwrap())
        .collect()
}

fn pairs(value: &Value) -> Vec<(String, String)> {
    match value {
        Value::Dict(dict) => dict
            .iter()
            .map(|(key, item)| (key.to_utf8().unwrap(), string(item)))
            .collect(),
        _ => Vec::new(),
    }
}

fn read_body(stream: &mut TcpStream) -> String {
    let mut data = Vec::new();
    let mut chunk = [0u8; 65536];
    let split = loop {
        if let Some(at) = data.windows(4).position(|window| window == b"\r\n\r\n") {
            break at;
        }
        match stream.read(&mut chunk) {
            Ok(n) if n > 0 => data.extend_from_slice(&chunk[..n]),
            _ => break data.len(),
        }
    };
    let head = String::from_utf8_lossy(&data[..split.min(data.len())]).into_owned();
    let mut body = data.get(split + 4..).unwrap_or_default().to_vec();
    let length: usize = head
        .split("\r\n")
        .skip(1)
        .find_map(|line| {
            let (key, value) = line.split_once(':')?;
            key.trim()
                .eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse().ok())
                .flatten()
        })
        .unwrap_or(0);
    while body.len() < length {
        match stream.read(&mut chunk) {
            Ok(n) if n > 0 => body.extend_from_slice(&chunk[..n]),
            _ => break,
        }
    }
    String::from_utf8(body).unwrap()
}

struct Server {
    port: u16,
    stop: Arc<AtomicBool>,
    thread: JoinHandle<Vec<String>>,
}

impl Server {
    fn start(replies: Vec<Vec<u8>>) -> Server {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        listener.set_nonblocking(true).unwrap();
        let stop = Arc::new(AtomicBool::new(false));
        let stopped = Arc::clone(&stop);
        let thread = std::thread::spawn(move || {
            let mut replies = replies.into_iter();
            let mut bodies = Vec::new();
            while !stopped.load(Ordering::Relaxed) {
                let Ok((mut stream, _)) = listener.accept() else {
                    std::thread::sleep(Duration::from_millis(2));
                    continue;
                };
                stream.set_nonblocking(false).unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(10)))
                    .unwrap();
                bodies.push(read_body(&mut stream));
                let reply = replies.next().unwrap_or_else(|| UNEXPECTED.to_vec());
                let _ = stream.write_all(&reply);
            }
            bodies
        });
        Server { port, stop, thread }
    }

    fn close(self) -> Vec<String> {
        self.stop.store(true, Ordering::Relaxed);
        self.thread.join().unwrap()
    }
}

fn free_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.local_addr().unwrap().port()
}

fn normalize(text: &str, port: Option<u16>) -> String {
    let mut text = text.to_string();
    if let Some(port) = port {
        text = text.replace(&format!("127.0.0.1:{port}"), "127.0.0.1:<port>");
        text = text.replace(&format!("[::1]:{port}"), "[::1]:<port>");
    }
    let mut out = String::new();
    let mut rest = text.as_str();
    while let Some(at) = rest.find(UNREACHABLE) {
        out.push_str(&rest[..at + UNREACHABLE.len()]);
        rest = &rest[at + UNREACHABLE.len()..];
        let end = rest.find(['"', '\\']).unwrap_or(rest.len());
        out.push_str("<os>");
        rest = &rest[end..];
    }
    out.push_str(rest);
    out
}

struct Ran {
    exit: i128,
    stdout: String,
    stderr: String,
    bodies: Vec<String>,
}

fn run(case: &Value) -> Ran {
    let kind = string(field(case, "server"));
    let replies: Vec<Vec<u8>> = strings(field(case, "replies"))
        .iter()
        .map(|h| unhex(h))
        .collect();
    let (port, server) = match kind.as_str() {
        "up" => {
            let server = Server::start(replies);
            (Some(server.port), Some(server))
        }
        "down" => (Some(free_port()), None),
        _ => (None, None),
    };
    let stdin = strings(field(case, "stdin")).join("\n") + "\n";
    let mut command = Command::new(RUST);
    command
        .current_dir(std::env::temp_dir())
        .env_clear()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (key, value) in pairs(field(case, "env")) {
        let value = match port {
            Some(port) => value.replace("<port>", &port.to_string()),
            None => value,
        };
        command.env(key, value);
    }
    let mut child = command.spawn().unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(stdin.as_bytes())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    let bodies = server.map(|server| server.close()).unwrap_or_default();
    Ran {
        exit: i128::from(out.status.code().unwrap_or(-1)),
        stdout: normalize(&String::from_utf8(out.stdout).unwrap(), port),
        stderr: normalize(&String::from_utf8(out.stderr).unwrap(), port),
        bodies: bodies
            .into_iter()
            .map(|body| normalize(&body, port))
            .collect(),
    }
}

#[test]
fn the_server_gives_what_the_python_gave() {
    let here = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/ask.json");
    let fixture = json::loads_str(&std::fs::read_to_string(&here).unwrap()).unwrap();
    let Value::List(cases) = field(&fixture, "cases") else {
        panic!("no cases");
    };
    let mut failures = Vec::new();
    for case in cases {
        let name = string(field(case, "name"));
        let ran = run(case);
        let Value::Int(exit) = field(case, "exit") else {
            panic!("{name}: no exit code");
        };
        let difference = match field(case, "difference") {
            Value::Str(text) => text.to_utf8().unwrap(),
            _ => String::new(),
        };
        if difference == "loopback" {
            let tool_error = ran.stdout.contains("this machine") && ran.stdout.contains("isError");
            if !tool_error {
                failures.push(format!(
                    "{name}: loopback refusal\n  rust: {:?}",
                    ran.stdout
                ));
            }
            if !ran.bodies.is_empty() {
                failures.push(format!(
                    "{name}: posted to a host that is not this machine: {:?}",
                    ran.bodies
                ));
            }
            continue;
        }
        if ran.exit != *exit {
            failures.push(format!(
                "{name}: exit {} where the Python gave {exit}",
                ran.exit
            ));
        }
        if ran.stdout != string(field(case, "stdout")) {
            failures.push(format!(
                "{name}: stdout\n  rust:   {:?}\n  python: {:?}",
                ran.stdout,
                string(field(case, "stdout"))
            ));
        }
        if ran.stderr != string(field(case, "stderr")) {
            failures.push(format!(
                "{name}: stderr\n  rust:   {:?}\n  python: {:?}",
                ran.stderr,
                string(field(case, "stderr"))
            ));
        }
        let want = strings(field(case, "bodies"));
        if ran.bodies != want {
            failures.push(format!(
                "{name}: bodies\n  rust:   {:?}\n  python: {want:?}",
                ran.bodies
            ));
        }
    }
    println!("{} cases compared", cases.len());
    assert!(
        failures.is_empty(),
        "{} of {} comparisons failed:\n{}",
        failures.len(),
        cases.len() * 4,
        failures.join("\n")
    );
    assert!(cases.len() > 30, "{} cases", cases.len());
}
