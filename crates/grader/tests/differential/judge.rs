//! Build ticket 218: `judge.py`'s `label` and `agree`, step by step.
//!
//! `record_judge.py` ran the Python on each step of `fixtures/judge.json`: the files it
//! planted, the arguments, the environment, and the replies a stub model server on a free
//! port gave. This runs `hydra-grader judge` on the same steps against a stub server of its
//! own, started here on another free port, and requires the same exit code, stdout, stderr,
//! files and requests. No real model server is contacted.
//!
//! Equal to the byte: `judge-labels.jsonl` but for its timestamps, which holds
//! `prompt_sha256`; the body of every request, which holds the whole prompt; the method,
//! path and content type of every request; what the stub `hydra` was called with; stdout;
//! and stderr where the Python wrote a sentence.
//!
//! Normalized, and only these:
//! - `judged_at` is the wall clock. Each one must be a second of UTC ending in `Z` and is
//!   then read as `<time>` on both sides.
//! - The stub server's port, which is `<port>` in the fixture, and the scratch directory,
//!   which is `<root>`.
//! - Where the Python ended in a traceback, the last line of stderr must start with the
//!   same exception class. Where `argparse` refused the command line, the message after
//!   `error: ` must be the same. A help text is compared by its first words.

use super::*;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::os::unix::fs::PermissionsExt;
use std::process::Stdio;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use std::time::Duration;

const UNEXPECTED: &[u8] = b"HTTP/1.0 599 Unexpected\r\n\r\n";

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
    let mut lines = head.split("\r\n");
    let mut first = lines.next().unwrap_or_default().split(' ');
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
    thread: JoinHandle<Vec<Request>>,
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
            requests
        });
        Server { port, stop, thread }
    }

    fn close(self) -> Vec<Request> {
        self.stop.store(true, Ordering::Relaxed);
        self.thread.join().unwrap()
    }
}

/// A port nothing listens on: bound once, then closed.
fn free_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.local_addr().unwrap().port()
}

/// `text` with each `judged_at` checked to be a second of UTC and read as `<time>`.
fn timeless(text: &str) -> String {
    const MARKER: &str = "\"judged_at\": \"";
    let mut out = String::new();
    let mut rest = text;
    while let Some(at) = rest.find(MARKER) {
        let from = at + MARKER.len();
        let stamp = &rest[from..from + 20];
        let bytes = stamp.as_bytes();
        assert!(
            bytes[4] == b'-' && bytes[10] == b'T' && bytes[19] == b'Z' && &stamp[..2] == "20",
            "{stamp}"
        );
        assert_eq!(&rest[from + 20..from + 21], "\"", "{stamp}");
        out.push_str(&rest[..from]);
        out.push_str("<time>");
        rest = &rest[from + 20..];
    }
    out.push_str(rest);
    out
}

fn names(root: &Path, here: &Path, found: &mut Vec<String>) {
    for entry in std::fs::read_dir(here).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            names(root, &path, found);
        } else {
            found.push(path.strip_prefix(root).unwrap().display().to_string());
        }
    }
}

/// What a failed run is compared by: the exception class of a traceback's last line, or the
/// message `argparse` gave.
fn error(text: &str, traceback: bool) -> String {
    let last = text.lines().last().unwrap_or_default();
    if traceback {
        // The exception's line is the first one under the last traceback that is not
        // indented. It is not always the last: 3.14 prints an exception's notes after it,
        // and a `BadStatusLine` ends with the line end the server sent.
        let lines: Vec<&str> = text.lines().collect();
        let from = lines
            .iter()
            .rposition(|line| line.starts_with("Traceback "))
            .map_or(0, |at| at + 1);
        let raised = lines[from..]
            .iter()
            .find(|line| !line.starts_with(' '))
            .copied()
            .unwrap_or_default();
        let class = raised.split(':').next().unwrap_or_default();
        class.rsplit('.').next().unwrap_or_default().to_string()
    } else {
        last.split_once(": error: ")
            .map_or(last, |(_, message)| message)
            .to_string()
    }
}

struct Ran {
    code: Option<i32>,
    stdout: String,
    stderr: String,
    requests: Vec<Request>,
    port: u16,
}

fn judge(
    root: &Path,
    argv: &[String],
    env: &[(String, String)],
    replies: Option<Vec<Vec<u8>>>,
) -> Ran {
    let server = replies.map(Server::start);
    let port = server.as_ref().map_or_else(free_port, |server| server.port);
    let fill = |text: &str| {
        text.replace("<root>", root.to_str().unwrap())
            .replace("<port>", &port.to_string())
    };
    let ran = Command::new(RUST)
        .arg("judge")
        .args(argv)
        .current_dir(root)
        .env_clear()
        .envs(env.iter().map(|(key, value)| (key, fill(value))))
        .stdin(Stdio::null())
        .output()
        .unwrap();
    Ran {
        code: ran.status.code(),
        stdout: String::from_utf8(ran.stdout).unwrap(),
        stderr: String::from_utf8(ran.stderr).unwrap(),
        requests: server.map(Server::close).unwrap_or_default(),
        port,
    }
}

fn pairs(value: &Value) -> Vec<(String, String)> {
    let Value::Dict(dict) = value else {
        panic!("an object")
    };
    dict.iter()
        .map(|(key, value)| (key.lossy(), string(value)))
        .collect()
}

/// Where `plant` puts a file, made empty.
fn planting(root: &Path, path: &str) -> PathBuf {
    let path = root.join(path);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let _ = std::fs::remove_file(&path);
    path
}

fn plant(root: &Path, path: &str, data: &[u8], exec: bool) {
    let path = planting(root, path);
    std::fs::write(&path, data).unwrap();
    if exec {
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
}

#[test]
fn the_judge_gives_what_the_python_gave() {
    let recorded = fixture("judge.json");
    assert!(string(field(&recorded, "version")).starts_with("3.14.7 "));
    let stub = string(field(&recorded, "stub"));
    let scratch = Scratch::new("judge");
    let stubs = Stubs::new(scratch.0.join("stubs"), [("hydra", stub.as_str())]);
    let mut tally = Tally::new();
    let mut calls = 0;
    for (n, case) in items(field(&recorded, "cases")).iter().enumerate() {
        let root = scratch.0.join(n.to_string());
        std::fs::create_dir_all(&root).unwrap();
        for (m, step) in items(field(case, "steps")).iter().enumerate() {
            let label = format!("{}, step {m}", string(field(case, "name")));
            for put in items(field(step, "puts")) {
                let Value::Dict(spec) = put else {
                    panic!("a put")
                };
                let path = string(field(put, "path"));
                match (spec.get("stub"), spec.get("hex")) {
                    (Some(_), _) => stubs.link("hydra", &planting(&root, &path)),
                    (_, Some(hex)) => plant(&root, &path, &unhex(&string(hex)), false),
                    _ => plant(&root, &path, string(field(put, "text")).as_bytes(), false),
                }
            }
            let argv: Vec<String> = items(field(step, "argv")).iter().map(string).collect();
            let replies = (string(field(step, "server")) == "up").then(|| {
                items(field(step, "replies"))
                    .iter()
                    .map(|reply| unhex(&string(reply)))
                    .collect()
            });
            let ran = judge(&root, &argv, &pairs(field(step, "env")), replies);
            let plain = |text: &str| {
                text.replace(root.to_str().unwrap(), "<root>")
                    .replace(&format!("127.0.0.1:{}", ran.port), "127.0.0.1:<port>")
            };

            let Value::Int(code) = field(step, "exit") else {
                panic!("an exit code")
            };
            tally.check(&format!("{label}: exit"), ran.code, Some(*code as i32));
            let (pyout, pyerr) = (string(field(step, "stdout")), string(field(step, "stderr")));
            if pyout.starts_with("usage:") {
                tally.check(
                    &format!("{label}: help"),
                    ran.stdout.starts_with("usage: hydra-grader judge"),
                    true,
                );
            } else {
                tally.check(&format!("{label}: stdout"), plain(&ran.stdout), pyout);
            }
            let traceback = pyerr.contains("Traceback (most recent call last):");
            if *code == 2 || traceback {
                tally.check(
                    &format!("{label}: error"),
                    error(&ran.stderr, traceback),
                    error(&pyerr, traceback),
                );
            } else {
                tally.check(&format!("{label}: stderr"), plain(&ran.stderr), pyerr);
            }

            let mut left = Vec::new();
            names(&root, &root, &mut left);
            left.sort();
            let python: Vec<String> = items(field(step, "names")).iter().map(string).collect();
            tally.check(&format!("{label}: the files left"), left, python);
            for file in items(field(step, "files")) {
                let path = string(field(file, "path"));
                let rust = std::fs::read_to_string(root.join(&path))
                    .map_or_else(|_| "<missing>".to_string(), |text| timeless(&plain(&text)));
                tally.check(
                    &format!("{label}: {path}"),
                    rust,
                    timeless(&string(field(file, "text"))),
                );
            }
            let _ = std::fs::remove_file(root.join("bin/calls"));

            let python: Vec<Request> = items(field(step, "requests"))
                .iter()
                .map(|request| Request {
                    method: string(field(request, "method")),
                    path: string(field(request, "path")),
                    content_type: optional(field(request, "content_type")),
                    body: string(field(request, "body")),
                })
                .collect();
            calls += python.len();
            tally.check(
                &format!("{label}: how many requests"),
                ran.requests.len(),
                python.len(),
            );
            for (k, (rust, python)) in ran.requests.into_iter().zip(python).enumerate() {
                tally.check(&format!("{label}: request {k}"), rust, python);
            }
        }
    }
    assert!(calls > 50, "only {calls} requests were compared");
    tally.finish();
}

/// Map ticket 94: a host that is not this machine by name is refused, and nothing is sent.
/// `127.1` is a spelling the system resolver takes for `127.0.0.1`, and a stub listens
/// there, so a client that resolved the name would reach it.
#[test]
fn another_host_is_refused_by_name_and_never_called() {
    let scratch = Scratch::new("judge-host");
    let root = &scratch.0;
    plant(
        root,
        "home/dataset.jsonl",
        b"{\"run_id\": \"r\", \"outcome\": \"gated\"}\n{\"run_id\": \"a\", \"task_text\": \"t\"}\n",
        false,
    );
    for host in [
        "127.1",
        "LOCALHOST.",
        "0.0.0.0",
        "user@127.0.0.1",
        "example.com",
    ] {
        let env = [
            ("HYDRA_ROOT".to_string(), "root".to_string()),
            ("HYDRA_GRADER_HOME".to_string(), "home".to_string()),
            ("HYDRA_BIN".to_string(), "<root>/no-hydra".to_string()),
            (
                "HYDRA_LOCAL_LLM_URL".to_string(),
                format!("http://{host}:<port>/api/generate"),
            ),
        ];
        let ran = judge(root, &["label".to_string()], &env, Some(Vec::new()));
        assert_eq!(ran.code, Some(1), "{host}: {}", ran.stderr);
        assert_eq!(ran.requests, Vec::new(), "{host}");
        assert_eq!(ran.stdout, "", "{host}");
        assert_eq!(ran.stderr.lines().count(), 1, "{host}: {}", ran.stderr);
        assert!(
            ran.stderr
                .starts_with("judge: the model server has to be on this machine and ")
                && ran.stderr.contains("so the pass stopped and kept the"),
            "{host}: {}",
            ran.stderr
        );
        let _ = std::fs::remove_file(root.join("home/judge-labels.jsonl"));
    }
    // The same pass with the host spelled as this machine reaches the stub.
    let env = [
        ("HYDRA_GRADER_HOME".to_string(), "home".to_string()),
        ("HYDRA_ROOT".to_string(), "root".to_string()),
        ("HYDRA_BIN".to_string(), "<root>/no-hydra".to_string()),
        (
            "HYDRA_LOCAL_LLM_URL".to_string(),
            "http://localhost:<port>/api/generate".to_string(),
        ),
    ];
    let reply = b"HTTP/1.0 200 OK\r\n\r\n{\"response\": \"{\\\"reason\\\": \\\"r\\\", \\\"result_ok\\\": true}\"}";
    let ran = judge(
        root,
        &["label".to_string()],
        &env,
        Some(vec![reply.to_vec()]),
    );
    assert_eq!(ran.code, Some(0), "{}", ran.stderr);
    assert_eq!(ran.requests.len(), 1);
    assert_eq!(ran.stdout, "labelled 2, unparsed 0, already labelled 0\n");
}
