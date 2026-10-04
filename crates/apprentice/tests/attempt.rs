//! `hydra-apprentice attempt` and `score` must give what `apprentice.py` gives.
//!
//! `record_attempt.py` ran the Python on each step of `fixtures/attempt.json`: the files it
//! planted, the arguments, the environment, and the replies a stub model server on a free
//! port gave. This runs the built binary on the same steps against a stub server of its
//! own, started here on another free port. It starts no `python3` and contacts no real
//! model server.
//!
//! Equal to the byte: the exit code; stdout; every file a step wrote or changed, which is
//! `attempts.jsonl` but for its `seconds`; the method, path, content type and body of each
//! request the stub received, the body holding the whole prompt; and stderr where the
//! Python wrote a sentence.
//!
//! Normalized, and only these:
//! - `seconds` is the wall clock. Each must be a number rounded to two places, and is then
//!   read as `<seconds>` on both sides.
//! - The stub server's port is `<port>` and the scratch directory `<root>`.
//! - The program's name: the Python says `apprentice.py`, the Rust `hydra-apprentice`.
//! - Where the Python ended in a traceback, the Rust's last line of stderr must start with
//!   the same exception class, without the module the Python names it by.
//!
//! One case is a stated difference, marked `differs`: a URL that names another host. The
//! Python tried to connect and said Ollama was not answering. The Rust refuses the host by
//! name, connects to nothing, and must say so in one sentence with exit 1.

use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use std::time::Duration;

use hydra_pyjson::json::{self, Value};

const RUST: &str = env!("CARGO_BIN_EXE_hydra-apprentice");
const UNEXPECTED: &[u8] = b"HTTP/1.0 599 Unexpected\r\n\r\n";

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

fn items(value: &Value) -> &[Value] {
    match value {
        Value::List(items) => items,
        other => panic!("not a list: {other:?}"),
    }
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

fn unhex(text: &str) -> Vec<u8> {
    (0..text.len())
        .step_by(2)
        .map(|at| u8::from_str_radix(&text[at..at + 2], 16).unwrap())
        .collect()
}

#[derive(Debug, PartialEq)]
struct Request {
    method: String,
    path: String,
    content_type: Option<String>,
    body: String,
}

fn read_request(stream: &mut TcpStream) -> Request {
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
    let head = String::from_utf8_lossy(&data[..split]).into_owned();
    let mut body = data.get(split + 4..).unwrap_or_default().to_vec();
    let mut first = head.split("\r\n").next().unwrap_or_default().split(' ');
    let (method, path) = (
        first.next().unwrap_or_default().to_string(),
        first.next().unwrap_or_default().to_string(),
    );
    let header = |name: &str| {
        head.split("\r\n").skip(1).find_map(|line| {
            let (key, value) = line.split_once(':')?;
            key.trim()
                .eq_ignore_ascii_case(name)
                .then(|| value.trim().to_string())
        })
    };
    let length: usize = header("content-length")
        .and_then(|value| value.parse().ok())
        .unwrap_or(0);
    while body.len() < length {
        match stream.read(&mut chunk) {
            Ok(n) if n > 0 => body.extend_from_slice(&chunk[..n]),
            _ => break,
        }
    }
    Request {
        method,
        path,
        content_type: header("content-type"),
        body: String::from_utf8(body).unwrap(),
    }
}

/// The stub model server: answers each connection with the next reply and keeps every
/// request.
struct Server {
    port: u16,
    stop: Arc<AtomicBool>,
    thread: JoinHandle<(Vec<Request>, usize)>,
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
            let mut requests = Vec::new();
            while !stopped.load(Ordering::Relaxed) {
                let Ok((mut stream, _)) = listener.accept() else {
                    std::thread::sleep(Duration::from_millis(2));
                    continue;
                };
                stream.set_nonblocking(false).unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(10)))
                    .unwrap();
                requests.push(read_request(&mut stream));
                let reply = replies.next().unwrap_or_else(|| UNEXPECTED.to_vec());
                let _ = stream.write_all(&reply);
            }
            (requests, replies.len())
        });
        Server { port, stop, thread }
    }

    /// The requests, and how many replies were never asked for.
    fn close(self) -> (Vec<Request>, usize) {
        self.stop.store(true, Ordering::Relaxed);
        self.thread.join().unwrap()
    }
}

/// A port nothing listens on: bound once, then closed.
fn free_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.local_addr().unwrap().port()
}

/// `text` with each `"seconds": N` checked to be rounded to two places and read as
/// `<seconds>`, as the recorder wrote it.
fn timeless(text: &str) -> String {
    const MARKER: &str = "\"seconds\": ";
    let mut out = String::new();
    let mut rest = text;
    while let Some(at) = rest.find(MARKER) {
        let from = at + MARKER.len();
        let end = rest[from..]
            .find([',', '}'])
            .map_or(rest.len(), |end| from + end);
        let number = &rest[from..end];
        let (whole, places) = number.split_once('.').unwrap_or((number, ""));
        assert!(
            !whole.is_empty()
                && whole.bytes().all(|b| b.is_ascii_digit())
                && (1..=2).contains(&places.len())
                && places.bytes().all(|b| b.is_ascii_digit()),
            "seconds {number:?}"
        );
        out.push_str(&rest[..from]);
        out.push_str("\"<seconds>\"");
        rest = &rest[end..];
    }
    out.push_str(rest);
    out
}

/// The class of the exception a stderr ends with: in the Python, the first line under the
/// last traceback that is not indented, since 3.14 prints notes after it; in the Rust, its
/// last line. A class the Python names with its module is compared without it.
fn exception_class(stderr: &str, python: bool) -> String {
    let lines: Vec<&str> = stderr.lines().collect();
    let raised = if python {
        let from = lines
            .iter()
            .rposition(|line| line.starts_with("Traceback "))
            .map_or(0, |at| at + 1);
        lines[from..]
            .iter()
            .find(|line| !line.starts_with(' '))
            .copied()
            .unwrap_or_default()
    } else {
        lines.last().copied().unwrap_or_default()
    };
    let class = raised.split(':').next().unwrap_or_default();
    class.rsplit('.').next().unwrap_or_default().to_string()
}

/// What of a stderr is compared: all of it, or with a traceback what came before it and
/// the exception class.
fn told(stderr: &str, python: bool, traceback: bool) -> String {
    if !traceback {
        return stderr.to_string();
    }
    let before = if python {
        stderr.split("Traceback ").next().unwrap_or_default()
    } else {
        stderr
            .trim_end_matches('\n')
            .rsplit_once('\n')
            .map_or("", |(before, _)| before)
    };
    format!("{before}|{}", exception_class(stderr, python))
}

struct Ran {
    exit: i128,
    stdout: String,
    stderr: String,
    written: Vec<(String, String)>,
    requests: Vec<Request>,
    unused_replies: usize,
}

fn walk(root: &Path, dir: &Path, found: &mut BTreeMap<String, Vec<u8>>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk(root, &path, found);
        } else {
            let name = path.strip_prefix(root).unwrap().display().to_string();
            found.insert(name, std::fs::read(&path).unwrap());
        }
    }
}

/// One step: plant its files, start the stub or name a dead port, run the binary, and keep
/// what it gave. `given` is every file as the last step left it.
fn step(root: &Path, step: &Value, given: &mut BTreeMap<String, Vec<u8>>) -> Ran {
    for (name, text) in pairs(field(step, "planted")) {
        let path = root.join(&name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, text.as_bytes()).unwrap();
        given.insert(name, text.into_bytes());
    }
    let server = py_eq(field(step, "server"), "up").then(|| {
        Server::start(
            items(field(step, "replies"))
                .iter()
                .map(|reply| unhex(&string(reply)))
                .collect(),
        )
    });
    let port = server.as_ref().map_or_else(free_port, |server| server.port);
    let root_text = root.to_str().unwrap();
    let fill = |text: &str| {
        text.replace("<root>", root_text)
            .replace("<port>", &port.to_string())
    };
    let args: Vec<String> = items(field(step, "argv")).iter().map(string).collect();
    let out = Command::new(RUST)
        .args(&args)
        .current_dir(root)
        .env_clear()
        .envs(
            pairs(field(step, "env"))
                .into_iter()
                .map(|(key, value)| (key, fill(&value))),
        )
        .stdin(Stdio::null())
        .output()
        .unwrap();
    let (requests, unused_replies) = server.map(Server::close).unwrap_or_default();
    let plain = |data: &[u8]| {
        String::from_utf8(data.to_vec())
            .unwrap()
            .replace(root_text, "<root>")
            .replace(&format!("127.0.0.1:{port}"), "127.0.0.1:<port>")
            .replace("hydra-apprentice", "apprentice.py")
    };
    let mut found = BTreeMap::new();
    walk(root, root, &mut found);
    let mut written = Vec::new();
    for (name, data) in found {
        if given.get(&name) != Some(&data) {
            written.push((name.clone(), timeless(&plain(&data))));
            given.insert(name, data);
        }
    }
    Ran {
        exit: i128::from(out.status.code().unwrap_or(-1)),
        stdout: plain(&out.stdout),
        stderr: plain(&out.stderr),
        written,
        requests,
        unused_replies,
    }
}

fn py_eq(value: &Value, text: &str) -> bool {
    matches!(value, Value::Str(held) if *held == text)
}

/// The Rust's one sentence for a URL that names another host.
fn refused_by_name(stderr: &str) -> bool {
    stderr.starts_with(
        "attempt: the model server has to be on this machine and the host \"hydra-stub.invalid\" \
         is not 127.0.0.1, localhost or [::1]",
    ) && stderr.ends_with(", so the pass stopped and kept the 0 attempts it had written.\n")
        && stderr.lines().count() == 1
}

#[test]
fn attempt_and_score_give_what_the_python_gave() {
    let here = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/attempt.json");
    let fixture = json::loads_str(&std::fs::read_to_string(here).unwrap()).unwrap();
    assert_eq!(
        string(field(&fixture, "prompt_sha")),
        hydra_apprentice::attempt::prompt_sha()
    );
    let root =
        std::env::temp_dir().join(format!("hydra-apprentice-attempt-{}", std::process::id()));
    let mut failures = Vec::new();
    let mut compared = 0;
    let cases = items(field(&fixture, "cases"));
    for case in cases {
        let name = string(field(case, "name"));
        let differs = matches!(field(case, "differs"), Value::Bool(true));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let mut given = BTreeMap::new();
        for (n, recorded) in items(field(case, "steps")).iter().enumerate() {
            let name = format!("{name}, step {n}");
            let ran = step(&root, recorded, &mut given);
            let mut check = |what: &str, same: bool, detail: String| {
                compared += 1;
                if !same {
                    failures.push(format!("{name}: {what}\n{detail}"));
                }
            };
            let Value::Int(exit) = field(recorded, "exit") else {
                panic!("{name}: no exit code");
            };
            check(
                "exit",
                ran.exit == *exit,
                format!("  rust {} python {exit}", ran.exit),
            );
            let stdout = string(field(recorded, "stdout"));
            check(
                "stdout",
                ran.stdout == stdout,
                format!("  rust:   {:?}\n  python: {stdout:?}", ran.stdout),
            );
            let python_err = string(field(recorded, "stderr"));
            if differs {
                check(
                    "the refusal by name",
                    refused_by_name(&ran.stderr) && ran.requests.is_empty(),
                    format!("  rust: {:?}", ran.stderr),
                );
            } else {
                let traceback = python_err.contains("Traceback (most recent call last):\n");
                let (rust, python) = (
                    told(&ran.stderr, false, traceback),
                    told(&python_err, true, traceback),
                );
                check(
                    "stderr",
                    rust == python,
                    format!("  rust:   {rust:?}\n  python: {python:?}"),
                );
            }
            let want: Vec<(String, String)> = items(field(recorded, "written"))
                .iter()
                .map(|file| (string(field(file, "path")), string(field(file, "text"))))
                .collect();
            check(
                "files",
                ran.written == want,
                format!("  rust:   {:?}\n  python: {want:?}", ran.written),
            );
            let want: Vec<Request> = items(field(recorded, "requests"))
                .iter()
                .map(|request| Request {
                    method: string(field(request, "method")),
                    path: string(field(request, "path")),
                    content_type: match field(request, "content_type") {
                        Value::Null => None,
                        other => Some(string(other)),
                    },
                    body: string(field(request, "body")),
                })
                .collect();
            check(
                "requests",
                ran.requests == want && ran.unused_replies == 0,
                format!(
                    "  rust:   {:?} ({} replies unused)\n  python: {want:?}",
                    ran.requests, ran.unused_replies
                ),
            );
        }
    }
    let _ = std::fs::remove_dir_all(&root);
    println!("{} cases, {compared} comparisons", cases.len());
    assert!(
        failures.is_empty(),
        "{} of {compared} comparisons failed:\n{}",
        failures.len(),
        failures.join("\n")
    );
    assert!(cases.len() > 100, "{} cases", cases.len());
}
