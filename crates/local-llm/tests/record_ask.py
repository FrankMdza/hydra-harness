"""Record what `hydra-local-llm` gives, so `ask.rs` can compare the Rust.

Build ticket 237, the MCP server of `integrations/local-llm/`. From the checkout's root:

    python3 crates/local-llm/tests/record_ask.py

It starts itself again with nothing in the environment, and then runs
`integrations/local-llm/hydra-local-llm` once per case below as a child, with nothing
in the child's environment but what the case names, plus `LC_CTYPE`, `PYTHONUNBUFFERED`
and `PYTHONDONTWRITEBYTECODE` so CPython will decode the pipe. A case feeds JSON-RPC
lines on stdin and, when it reaches the network, a stub Ollama on a free port of
127.0.0.1 with the replies the case queued. No real model server is contacted.

What a case keeps: the exit code, stdout, stderr, and the body of every POST the stub
received.

Two things are replaced before the fixture is written, so that recording twice gives the
same bytes: the stub server's port is `<port>`, and the OS text after
`ollama is unreachable: ` is `<os>`. A traceback would name the checkout as `<repo>`. It
writes `crates/local-llm/tests/fixtures/ask.json`.
"""

from __future__ import annotations

import json
import os
import re
import socket
import subprocess
import sys
import threading
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[2]
SCRIPT = REPO / "integrations" / "local-llm" / "hydra-local-llm"
MARK = "HYDRA_LOCAL_LLM_RECORDING"
PORT = "<port>"
UNREACHABLE = "ollama is unreachable: "

UNEXPECTED = b"HTTP/1.0 599 Unexpected\r\nContent-Length: 0\r\n\r\n"


class Server:
    """Answers each connection with the next queued reply and keeps every request body."""

    def __init__(self, replies):
        self.replies = list(replies)
        self.requests = []
        self.sock = socket.socket()
        self.sock.bind(("127.0.0.1", 0))
        self.sock.listen(16)
        self.sock.settimeout(0.05)
        self.port = self.sock.getsockname()[1]
        self.stop = False
        self.thread = threading.Thread(target=self.serve, daemon=True)
        self.thread.start()

    def serve(self):
        while not self.stop:
            try:
                conn, _ = self.sock.accept()
            except OSError:
                continue
            with conn:
                conn.settimeout(10)
                data = b""
                while b"\r\n\r\n" not in data:
                    more = conn.recv(65536)
                    if not more:
                        break
                    data += more
                head, _, body = data.partition(b"\r\n\r\n")
                lines = head.decode("latin-1").split("\r\n")
                headers = {}
                for line in lines[1:]:
                    key, _, value = line.partition(":")
                    headers[key.strip().lower()] = value.strip()
                length = int(headers.get("content-length", "0"))
                while len(body) < length:
                    body += conn.recv(65536)
                self.requests.append(body.decode("utf-8"))
                conn.sendall(self.replies.pop(0) if self.replies else UNEXPECTED)

    def close(self):
        self.stop = True
        self.thread.join()
        self.sock.close()


def free_port() -> int:
    with socket.socket() as sock:
        sock.bind(("127.0.0.1", 0))
        return sock.getsockname()[1]


def http(code=200, body=b"", reason="OK", headers=(), version="HTTP/1.0") -> bytes:
    listed = list(headers)
    if not any(line.lower().startswith("content-length:") for line in listed):
        listed.append(f"Content-Length: {len(body)}")
    head = f"{version} {code} {reason}\r\n" + "".join(f"{line}\r\n" for line in listed)
    return head.encode() + b"\r\n" + body


def says(answer: str) -> bytes:
    return http(
        body=json.dumps({"response": answer, "done": True}).encode(),
        headers=["Content-Type: application/json"],
    )


def rpc(method, params=None, id=1):
    request = {"jsonrpc": "2.0", "method": method}
    if id is not ...:
        request["id"] = id
    if params is not None:
        request["params"] = params
    return json.dumps(request)


def initialize(id=1):
    return rpc(
        "initialize",
        {
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": {"name": "record_ask.py", "version": "0"},
        },
        id=id,
    )


def ask(prompt="what is two plus two?", **fields):
    arguments = {"prompt": prompt, **fields}
    return rpc("tools/call", {"name": "ask", "arguments": arguments})


def plain(text: str, port: int | None) -> str:
    if port is not None:
        text = text.replace(f"127.0.0.1:{port}", f"127.0.0.1:{PORT}")
        text = text.replace(f"[::1]:{port}", f"[::1]:{PORT}")
    text = text.replace(str(REPO), "<repo>")
    return re.sub(re.escape(UNREACHABLE) + r'[^"\\]*', UNREACHABLE + "<os>", text)


def play(case: dict) -> dict:
    stdin = "".join(line + "\n" for line in case["stdin"]).encode("utf-8")
    server_kind = case["server"]
    replies = [bytes.fromhex(item) for item in case["replies"]]
    port = None
    server = None
    env = dict(case["env"])
    # CPython needs a locale to decode pipes; the case still names every Hydra variable.
    env.setdefault("PYTHONUNBUFFERED", "1")
    env.setdefault("PYTHONDONTWRITEBYTECODE", "1")
    env.setdefault("LC_CTYPE", "C.UTF-8")
    if server_kind == "up":
        server = Server(replies)
        port = server.port
    elif server_kind == "down":
        port = free_port()
    for key, value in list(env.items()):
        env[key] = value.replace(PORT, str(port) if port is not None else "0")
    ran = subprocess.run(
        [sys.executable, "-B", str(SCRIPT)],
        env=env,
        input=stdin,
        capture_output=True,
        timeout=15,
    )
    bodies = []
    if server is not None:
        server.close()
        bodies = [plain(body, port) for body in server.requests]
    recorded_env = {
        key: value.replace(str(port), PORT) if port is not None else value
        for key, value in case["env"].items()
    }
    result = {
        "name": case["name"],
        "stdin": case["stdin"],
        "env": recorded_env,
        "server": server_kind,
        "replies": case["replies"],
        "exit": ran.returncode,
        "stdout": plain(ran.stdout.decode("utf-8"), port),
        "stderr": plain(ran.stderr.decode("utf-8"), port),
        "bodies": bodies,
    }
    if case.get("difference"):
        result["difference"] = case["difference"]
    return result


def cases() -> list[dict]:
    url = f"http://127.0.0.1:{PORT}/api/generate"
    env = {"HYDRA_LOCAL_LLM_URL": url, "HYDRA_LOCAL_MODEL": "test-model"}
    stub = says("stub answer")
    stub_hex = stub.hex()
    out = []

    def case(name, stdin, replies=None, env=env, server="up", difference=None):
        if replies is None:
            replies = [stub_hex] if server == "up" else []
        item = {
            "name": name,
            "stdin": stdin,
            "env": env,
            "server": server,
            "replies": replies,
        }
        if difference is not None:
            item["difference"] = difference
        out.append(item)

    case(
        "the handshake answers the protocol version the client sent",
        [initialize(), rpc("notifications/initialized", {}, id=...)],
        server="none",
    )
    case(
        "tools/list names ask",
        [initialize(), rpc("tools/list", {})],
        server="none",
    )
    case(
        "a call returns the stub text",
        [initialize(), ask()],
    )
    case(
        "a call with a system prompt and max_tokens",
        [ask("sum", system="be brief", max_tokens=32)],
    )
    case(
        "an empty system prompt is omitted from the body",
        [ask("hello", system="")],
    )
    case(
        "max_tokens true is a Python int and is sent as true",
        [ask("hello", max_tokens=True)],
    )
    case(
        "max_tokens false is refused",
        [ask("hello", max_tokens=False)],
        server="none",
    )
    case(
        "max_tokens zero is refused",
        [ask("hello", max_tokens=0)],
        server="none",
    )
    case(
        "max_tokens a float is refused",
        [ask("hello", max_tokens=1.5)],
        server="none",
    )
    case(
        "max_tokens a string is refused",
        [ask("hello", max_tokens="256")],
        server="none",
    )
    case(
        "an overlong prompt is refused",
        [ask("x" * 8001)],
        server="none",
    )
    case(
        "a prompt of 8000 characters is posted",
        [ask("x" * 8000)],
    )
    case(
        "a prompt of 8001 unicode characters is refused",
        [ask("é" * 8001)],
        server="none",
    )
    case(
        "an empty prompt is posted",
        [ask("")],
    )
    case(
        "a missing prompt is refused",
        [rpc("tools/call", {"name": "ask", "arguments": {}})],
        server="none",
    )
    case(
        "a prompt that is a number is refused",
        [rpc("tools/call", {"name": "ask", "arguments": {"prompt": 7}})],
        server="none",
    )
    case(
        "a system that is a number is refused",
        [ask("hello", system=7)],
        server="none",
    )
    case(
        "a system of null is omitted",
        [ask("hello", system=None)],
    )
    case(
        "arguments that are not an object",
        [rpc("tools/call", {"name": "ask", "arguments": ["prompt"]})],
        server="none",
    )
    case(
        "a tool this server does not have",
        [rpc("tools/call", {"name": "other", "arguments": {"prompt": "x"}})],
        server="none",
    )
    case(
        "a call that named none",
        [rpc("tools/call", {"arguments": {"prompt": "x"}})],
        server="none",
    )
    case(
        "a call whose name is a number",
        [rpc("tools/call", {"name": 1, "arguments": {"prompt": "x"}})],
        server="none",
    )
    case(
        "a malformed request is a json-rpc error",
        ["not json at all"],
        server="none",
    )
    case(
        "a line that is a JSON array",
        ["[1, 2]"],
        server="none",
    )
    case(
        "a method this server does not know",
        [initialize(), rpc("resources/list", {})],
        server="none",
    )
    case(
        "params that are a list",
        [rpc("initialize", [])],
        server="none",
    )
    case(
        "a notification writes nothing",
        [rpc("initialize", {"protocolVersion": "2025-06-18"}, id=...)],
        server="none",
    )
    case(
        "an id of null writes nothing",
        [rpc("initialize", {"protocolVersion": "2025-06-18"}, id=None)],
        server="none",
    )
    case(
        "an id of zero, a string, and true",
        [initialize(id=0), initialize(id="abc"), initialize(id=True)],
        server="none",
    )
    case(
        "blank lines are skipped",
        ["", "   ", initialize(), "\t"],
        server="none",
    )
    case(
        "ollama unreachable is a tool error",
        [ask("hello")],
        replies=[],
        server="down",
    )
    case(
        "a 404 with an error field",
        [ask("hello")],
        replies=[http(404, json.dumps({"error": "model not found"}).encode(), reason="Not Found").hex()],
    )
    case(
        "a 500 with a plain body",
        [ask("hello")],
        replies=[http(500, b"nope", reason="Internal Server Error").hex()],
    )
    case(
        "a 404 with no error field uses the body",
        [ask("hello")],
        replies=[http(404, json.dumps({"detail": "gone"}).encode(), reason="Not Found").hex()],
    )
    case(
        "a 200 that is a JSON array",
        [ask("hello")],
        replies=[http(body=b"[1, 2]").hex()],
    )
    case(
        "a 200 with no response text",
        [ask("hello")],
        replies=[http(body=b'{"done": true}').hex()],
    )
    case(
        "a 200 whose response is a number",
        [ask("hello")],
        replies=[http(body=b'{"response": 7}').hex()],
    )
    case(
        "the default model when none is named",
        [ask("hello")],
        env={"HYDRA_LOCAL_LLM_URL": url},
    )
    case(
        "a unicode prompt is posted escaped",
        [ask("café \U0001f409")],
    )
    case(
        "extra argument keys are ignored",
        [ask("hello", unused=True)],
    )
    case(
        "two asks in one session",
        [ask("one"), ask("two")],
        replies=[says("first").hex(), says("second").hex()],
    )
    case(
        "the protocol version the client sent is echoed",
        [rpc("initialize", {"protocolVersion": "2024-11-05"})],
        server="none",
    )
    case(
        "a host that is not this machine",
        [ask("hello")],
        server="none",
        env={"HYDRA_LOCAL_LLM_URL": "not-a-url",
             "HYDRA_LOCAL_MODEL": "test-model"},
        difference="loopback",
    )
    return out


def main() -> None:
    if os.environ.get(MARK) != "1":
        os.execve(
            sys.executable,
            [sys.executable, str(Path(__file__).resolve()), *sys.argv[1:]],
            {MARK: "1", "PYTHONDONTWRITEBYTECODE": "1"},
        )
    out = Path(sys.argv[1]).resolve() if len(sys.argv) > 1 else HERE / "fixtures"
    out.mkdir(parents=True, exist_ok=True)
    recorded = []
    for case in cases():
        recorded.append(play(case))
    fixture = {"python": sys.version.split()[0], "cases": recorded}
    (out / "ask.json").write_text(
        json.dumps(fixture, indent=1, ensure_ascii=False) + "\n",
        encoding="utf-8",
    )


if __name__ == "__main__":
    main()
