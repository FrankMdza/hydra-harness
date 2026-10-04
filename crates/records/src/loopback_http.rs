//! The one network call the grader makes: a POST to the model server on this machine.
//!
//! Map ticket 94: the client is written here over `TcpStream`, speaks plain HTTP, and talks
//! to this machine only. A URL that names any other host is refused by its name, before a
//! name is looked up or a socket opened. `judge.py` does this call with `urllib`, which
//! goes wherever the URL points, follows a redirect and honours a proxy. This does none of
//! the three.
//!
//! What it shares with `urllib` is what `judge.py` can tell apart: a status from 200 to 299
//! is an answer and anything else is the status; a server that is not there, hangs up
//! before a status line or goes quiet for the timeout is "not answering"; a status line
//! that is not one, or a body shorter than it was announced, is the exception `http.client`
//! raises, which `judge.py` does not catch.
//!
//! A status outside 200 to 299 also carries the reply's body, which `local-llm` prints and
//! the judge does not. A body that cannot be read is an empty one, so the judge's output
//! does not change.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr, TcpStream};
use std::time::Duration;

/// The longest line of a reply's head `http.client` reads.
const MAX_LINE: usize = 65536;
/// How many header lines `http.client` reads.
const MAX_HEADERS: usize = 100;

/// Why a call gave no body.
#[derive(Debug, PartialEq, Eq)]
pub enum Failure {
    /// The URL is not plain HTTP to this machine. Nothing was connected to. The text says
    /// what was wrong with it.
    NotLocal(String),
    /// `URLError` or `OSError`: nothing listens there, or it hung up or went quiet.
    NotAnswering,
    /// `HTTPError`: a status outside 200 to 299, and the reply's body, empty when it could
    /// not be read.
    Status(u16, Vec<u8>),
    /// What `http.client` raises and `judge.py` lets through, as `Class: message`.
    Raised(String),
}

/// Where a URL points, once it is known to be this machine.
#[derive(Debug, PartialEq, Eq)]
pub struct Target {
    /// The addresses to try, in order.
    pub addrs: Vec<SocketAddr>,
    /// What goes in the `Host` header: the URL's host and port as written.
    pub authority: String,
    /// The path and query the request line names.
    pub selector: String,
}

/// Read `url` as `http://HOST[:PORT][/PATH]`, where HOST is `127.0.0.1`, `localhost` or
/// `[::1]`. The host is compared as text: nothing is resolved, so a name that would resolve
/// to this machine is refused like any other.
pub fn local_target(url: &str) -> Result<Target, Failure> {
    let refuse = |why: String| Err(Failure::NotLocal(why));
    let Some((scheme, rest)) = url.split_once("://") else {
        return refuse(format!("{url:?} is not an http:// URL"));
    };
    if !scheme.eq_ignore_ascii_case("http") {
        return refuse(format!(
            "{url:?} is a {scheme} URL and the judge speaks plain http only"
        ));
    }
    let end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let (authority, tail) = rest.split_at(end);
    if authority.contains('@') {
        return refuse(format!("{url:?} carries a user name"));
    }
    // The authority is sent back as the `Host` header, so a line end in it would be a
    // header of the caller's choosing.
    if authority.bytes().any(|b| b <= b' ' || b == 0x7f) {
        return refuse(format!("{url:?} holds a space or a control character"));
    }
    let (host, port) = match authority.strip_prefix('[') {
        Some(bracketed) => match bracketed.split_once(']') {
            Some((host, "")) => (host, None),
            Some((host, after)) => match after.strip_prefix(':') {
                Some(port) => (host, Some(port)),
                // Anything after the bracket but a port is not part of a host.
                None => return refuse(format!("{url:?} has a host that does not read")),
            },
            None => return refuse(format!("{url:?} has a host that does not read")),
        },
        None => match authority.split_once(':') {
            Some((host, port)) => (host, Some(port)),
            None => (authority, None),
        },
    };
    let bracketed = authority.starts_with('[');
    let ips: Vec<std::net::IpAddr> = match (host, bracketed) {
        ("127.0.0.1", false) => vec![Ipv4Addr::LOCALHOST.into()],
        ("::1", true) => vec![Ipv6Addr::LOCALHOST.into()],
        (name, false) if name.eq_ignore_ascii_case("localhost") => {
            vec![Ipv4Addr::LOCALHOST.into(), Ipv6Addr::LOCALHOST.into()]
        }
        _ => {
            return refuse(format!(
                "the host {host:?} is not 127.0.0.1, localhost or [::1]"
            ));
        }
    };
    // `http.client` reads "host:" as the default port.
    let port = match port {
        None | Some("") => 80,
        Some(digits) => match digits.parse::<u16>() {
            Ok(port) if digits.bytes().all(|b| b.is_ascii_digit()) => port,
            _ => return refuse(format!("{url:?} has a port that is not a number")),
        },
    };
    // The fragment is never sent, and no path is the root.
    let tail = tail.split('#').next().unwrap_or_default();
    let selector = if tail.starts_with('/') {
        tail.to_string()
    } else {
        format!("/{tail}")
    };
    if selector.bytes().any(|b| b <= b' ' || b == 0x7f) {
        return refuse(format!("{url:?} holds a space or a control character"));
    }
    Ok(Target {
        addrs: ips
            .into_iter()
            .map(|ip| SocketAddr::new(ip, port))
            .collect(),
        authority: authority.to_string(),
        selector,
    })
}

fn raised(class: &str, message: &str) -> Failure {
    Failure::Raised(format!("{class}: {message}"))
}

/// One line of the head, with its line end. An empty one is the end of the stream.
fn line(reader: &mut impl BufRead) -> Result<Vec<u8>, Failure> {
    let mut out = Vec::new();
    reader
        .by_ref()
        .take(MAX_LINE as u64 + 1)
        .read_until(b'\n', &mut out)
        .map_err(|_| Failure::NotAnswering)?;
    if out.len() > MAX_LINE {
        return Err(raised("LineTooLong", "got more than 65536 bytes"));
    }
    Ok(out)
}

fn trimmed(line: &[u8]) -> &[u8] {
    let line = line.strip_suffix(b"\n").unwrap_or(line);
    line.strip_suffix(b"\r").unwrap_or(line)
}

/// The header lines up to the blank one, as lowercased name and value.
fn headers(reader: &mut impl BufRead) -> Result<Vec<(String, String)>, Failure> {
    let mut found = Vec::new();
    // `http.client` counts the lines it reads, a continuation or a line with no colon
    // included, and the blank one that ends them: 99 header lines are read and the 100th
    // raises (probed on CPython 3.14.7).
    let mut lines = 0;
    loop {
        let raw = line(reader)?;
        let text = trimmed(&raw);
        if text.is_empty() {
            return Ok(found);
        }
        lines += 1;
        if lines == MAX_HEADERS {
            return Err(raised("HTTPException", "got more than 100 headers"));
        }
        let text = String::from_utf8_lossy(text);
        if let Some((name, value)) = text.split_once(':') {
            found.push((name.trim().to_ascii_lowercase(), value.trim().to_string()));
        }
    }
}

/// The status of a reply, past any `100 Continue`, and the headers under it.
fn head(reader: &mut impl BufRead) -> Result<(u16, Vec<(String, String)>), Failure> {
    loop {
        let raw = line(reader)?;
        if raw.is_empty() {
            // `RemoteDisconnected`, which is a `ConnectionResetError`.
            return Err(Failure::NotAnswering);
        }
        let text = String::from_utf8_lossy(trimmed(&raw)).into_owned();
        let bad = || raised("BadStatusLine", &text);
        let mut words = text.splitn(3, ' ');
        let version = words.next().unwrap_or_default();
        let status = words.next().unwrap_or_default();
        if !version.starts_with("HTTP/") {
            return Err(bad());
        }
        let code: u16 = status.parse().map_err(|_| bad())?;
        if !(100..=999).contains(&code) || !status.bytes().all(|b| b.is_ascii_digit()) {
            return Err(bad());
        }
        let found = headers(reader)?;
        if code != 100 {
            return Ok((code, found));
        }
    }
}

fn incomplete(read: usize) -> Failure {
    raised(
        "IncompleteRead",
        &format!("IncompleteRead({read} bytes read)"),
    )
}

fn chunked(reader: &mut impl BufRead) -> Result<Vec<u8>, Failure> {
    let mut body = Vec::new();
    loop {
        let raw = line(reader)?;
        let text = String::from_utf8_lossy(trimmed(&raw)).into_owned();
        let digits = text.split(';').next().unwrap_or_default().trim();
        let Ok(size) = usize::from_str_radix(digits, 16) else {
            return Err(incomplete(body.len()));
        };
        if size == 0 {
            // The trailers, to the blank line or the end of the stream.
            while !trimmed(&line(reader)?).is_empty() {}
            return Ok(body);
        }
        let before = body.len();
        reader
            .by_ref()
            .take(size as u64)
            .read_to_end(&mut body)
            .map_err(|_| Failure::NotAnswering)?;
        if body.len() - before < size {
            return Err(incomplete(body.len()));
        }
        // The line end after the chunk.
        let mut end = [0u8; 2];
        if reader.read_exact(&mut end).is_err() {
            return Err(incomplete(body.len()));
        }
    }
}

/// The body of a reply whose status was `code`, by the framing its headers name.
fn body(
    reader: &mut impl BufRead,
    code: u16,
    headers: &[(String, String)],
) -> Result<Vec<u8>, Failure> {
    let named = |name: &str| {
        headers
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    };
    if code == 204 {
        return Ok(Vec::new());
    }
    if named("transfer-encoding").is_some_and(|value| value.eq_ignore_ascii_case("chunked")) {
        return chunked(reader);
    }
    let mut out = Vec::new();
    match named("content-length").and_then(|value| value.parse::<u64>().ok()) {
        Some(length) => {
            reader
                .by_ref()
                .take(length)
                .read_to_end(&mut out)
                .map_err(|_| Failure::NotAnswering)?;
            if (out.len() as u64) < length {
                return Err(incomplete(out.len()));
            }
        }
        None => {
            reader
                .read_to_end(&mut out)
                .map_err(|_| Failure::NotAnswering)?;
        }
    }
    Ok(out)
}

/// POST `content` as JSON to `url` and return the body of a 2xx reply. Each connect, write
/// and read may take `timeout` and no longer.
pub fn post_json(url: &str, content: &[u8], timeout: Duration) -> Result<Vec<u8>, Failure> {
    let target = local_target(url)?;
    let mut stream = target
        .addrs
        .iter()
        .find_map(|addr| TcpStream::connect_timeout(addr, timeout).ok())
        .ok_or(Failure::NotAnswering)?;
    let io = |_: std::io::Error| Failure::NotAnswering;
    stream.set_read_timeout(Some(timeout)).map_err(io)?;
    stream.set_write_timeout(Some(timeout)).map_err(io)?;
    let mut request = format!(
        "POST {} HTTP/1.1\r\nAccept-Encoding: identity\r\nContent-Length: {}\r\nHost: {}\r\n\
         User-Agent: hydra-grader\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n",
        target.selector,
        content.len(),
        target.authority
    )
    .into_bytes();
    request.extend_from_slice(content);
    stream.write_all(&request).map_err(io)?;
    stream.flush().map_err(io)?;
    let mut reader = BufReader::new(stream);
    let (code, headers) = head(&mut reader)?;
    if !(200..300).contains(&code) {
        let said = body(&mut reader, code, &headers).unwrap_or_default();
        return Err(Failure::Status(code, said));
    }
    body(&mut reader, code, &headers)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn refused(url: &str) -> bool {
        matches!(local_target(url), Err(Failure::NotLocal(_)))
    }

    #[test]
    fn only_this_machine_by_name_is_a_target() {
        let target = local_target("http://127.0.0.1:11434/api/generate").unwrap();
        assert_eq!(target.addrs, ["127.0.0.1:11434".parse().unwrap()]);
        assert_eq!(target.authority, "127.0.0.1:11434");
        assert_eq!(target.selector, "/api/generate");

        let target = local_target("HTTP://LocalHost?x=1#top").unwrap();
        assert_eq!(
            target.addrs,
            ["127.0.0.1:80".parse().unwrap(), "[::1]:80".parse().unwrap()]
        );
        assert_eq!(target.selector, "/?x=1");
        assert_eq!(
            local_target("http://[::1]:8080").unwrap().addrs,
            ["[::1]:8080".parse().unwrap()]
        );
        assert_eq!(local_target("http://localhost:/a").unwrap().addrs.len(), 2);
    }

    #[test]
    fn any_other_host_is_refused_before_a_socket_opens() {
        for url in [
            "",
            "127.0.0.1:11434/api/generate",
            "https://127.0.0.1:11434/api/generate",
            "http://example.com/api/generate",
            "http://127.0.0.2/",
            "http://127.0.0.1.example.com/",
            "http://localhost.example.com/",
            "http://127.1/",
            "http://0.0.0.0/",
            "http://2130706433/",
            "http://user@127.0.0.1/",
            "http://127.0.0.1@example.com/",
            "http://::1/",
            "http://[::1/",
            "http://[::ffff:127.0.0.1]/",
            "http://127.0.0.1:x/",
            "http://127.0.0.1:99999/",
            "http://127.0.0.1:+80/",
            "http://127.0.0.1/a b",
            "http:///api/generate",
            // Text after the bracket that is not a port went into the `Host` header.
            "http://[::1]evil/",
            "http://[::1]\r\nX-Injected: yes/",
            "http://127.0.0.1\r\nX-Injected: yes/",
            "http://localhost\t/",
            "http://127.0.0.1:80\n/",
        ] {
            assert!(refused(url), "{url:?}");
        }
    }

    #[test]
    fn a_status_outside_2xx_carries_the_body() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = Vec::new();
            let mut buf = [0u8; 1024];
            while !request.ends_with(b"{}") {
                let read = stream.read(&mut buf).unwrap();
                assert!(read > 0, "the client hung up mid-request");
                request.extend_from_slice(&buf[..read]);
            }
            stream
                .write_all(
                    b"HTTP/1.1 404 Not Found\r\nContent-Length: 14\r\n\r\n{\"error\":\"no\"}",
                )
                .unwrap();
        });
        let got = post_json(
            &format!("http://127.0.0.1:{port}/api/generate"),
            b"{}",
            Duration::from_secs(5),
        );
        server.join().unwrap();
        assert_eq!(
            got,
            Err(Failure::Status(404, b"{\"error\":\"no\"}".to_vec()))
        );
    }

    #[test]
    fn every_header_line_counts_toward_the_limit() {
        // `http.client` reads 99 header lines and raises on the 100th, whether or not a
        // line has a colon.
        let reply = |lines: usize, line: &[u8]| {
            let mut reply = b"HTTP/1.1 200 OK\r\n".to_vec();
            for _ in 0..lines {
                reply.extend_from_slice(line);
            }
            reply.extend_from_slice(b"\r\nbody");
            reply
        };
        for line in [&b"X: a\r\n"[..], b" continuation\r\n"] {
            let ok = reply(99, line);
            assert_eq!(head(&mut BufReader::new(&ok[..])).unwrap().0, 200);
            let over = reply(100, line);
            assert!(head(&mut BufReader::new(&over[..])).is_err());
        }
    }

    #[test]
    fn a_reply_is_read_by_the_framing_it_names() {
        let read = |reply: &[u8]| {
            let mut reader = BufReader::new(reply);
            let (code, headers) = head(&mut reader)?;
            body(&mut reader, code, &headers)
        };
        assert_eq!(
            read(b"HTTP/1.0 200 OK\r\n\r\nto the end").unwrap(),
            b"to the end"
        );
        assert_eq!(
            read(b"HTTP/1.1 100 Continue\r\n\r\nHTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nabc")
                .unwrap(),
            b"ab"
        );
        assert_eq!(
            read(b"HTTP/1.1 200 OK\r\nTransfer-Encoding: Chunked\r\n\r\n3;x=1\r\nabc\r\n2\r\nde\r\n0\r\n\r\n")
                .unwrap(),
            b"abcde"
        );
        assert_eq!(read(b"HTTP/1.1 204 No Content\r\n\r\n").unwrap(), b"");
        assert_eq!(read(b""), Err(Failure::NotAnswering));
        for (reply, class) in [
            (&b"SSH-2.0-OpenSSH\r\n"[..], "BadStatusLine"),
            (b"HTTP/1.1 2x0 OK\r\n\r\n", "BadStatusLine"),
            (
                b"HTTP/1.1 200 OK\r\nContent-Length: 9\r\n\r\nabc",
                "IncompleteRead",
            ),
            (
                b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n5\r\nabc",
                "IncompleteRead",
            ),
        ] {
            match read(reply) {
                Err(Failure::Raised(text)) => assert!(text.starts_with(class), "{text}"),
                other => panic!("{other:?}"),
            }
        }
    }
}
