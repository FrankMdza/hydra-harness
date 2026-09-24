"""Drive hydra-local-llm over a pipe the way a provider drives an MCP stdio server."""

from __future__ import annotations

import json
import os
import shutil
import socket
import subprocess
import sys
import tempfile
import threading
import unittest
from http.server import BaseHTTPRequestHandler, HTTPServer
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
SERVER = ROOT / "hydra-local-llm"


class StubOllamaHandler(BaseHTTPRequestHandler):
    response_text = "stub answer"
    status = 200
    body: bytes = b'{"response":"stub answer"}'

    def log_message(self, format: str, *args: Any) -> None:  # noqa: A003
        return

    def do_POST(self) -> None:  # noqa: N802
        if self.path != "/api/generate":
            self.send_response(404)
            self.end_headers()
            return
        length = int(self.headers.get("Content-Length", "0"))
        raw = self.rfile.read(length)
        try:
            payload = json.loads(raw.decode("utf-8"))
        except json.JSONDecodeError:
            payload = {}
        if payload.get("stream") is not False:
            self.send_response(400)
            self.end_headers()
            self.wfile.write(b'{"error":"stream must be false"}')
            return
        self.send_response(self.status)
        self.send_header("Content-Type", "application/json")
        self.end_headers()
        self.wfile.write(self.body)


def free_port() -> int:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind(("127.0.0.1", 0))
        return sock.getsockname()[1]


class LocalLlmTestCase(unittest.TestCase):
    def setUp(self) -> None:
        self.tmp = tempfile.mkdtemp()
        self.port = free_port()
        self.stub_url = f"http://127.0.0.1:{self.port}/api/generate"
        StubOllamaHandler.status = 200
        StubOllamaHandler.body = json.dumps({"response": "stub answer"}).encode("utf-8")
        self.httpd = HTTPServer(("127.0.0.1", self.port), StubOllamaHandler)
        self.thread = threading.Thread(target=self.httpd.serve_forever, daemon=True)
        self.thread.start()

        self.env = os.environ.copy()
        self.env["HYDRA_LOCAL_LLM_URL"] = self.stub_url
        self.env["HYDRA_LOCAL_MODEL"] = "test-model"
        self.process = subprocess.Popen(
            [sys.executable, str(SERVER)],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            env=self.env,
        )
        self._next_id = 0

    def tearDown(self) -> None:
        if self.process.stdin:
            self.process.stdin.close()
        if self.process.stdout:
            self.process.stdout.close()
        if self.process.stderr:
            self.process.stderr.close()
        self.process.kill()
        self.process.wait(timeout=5)
        self.httpd.shutdown()
        self.thread.join(timeout=5)
        self.httpd.server_close()
        shutil.rmtree(self.tmp)

    def call(self, method: str, params: dict[str, Any] | None = None) -> dict[str, Any]:
        self._next_id += 1
        request_id = self._next_id
        request = {
            "jsonrpc": "2.0",
            "id": request_id,
            "method": method,
            "params": params or {},
        }
        assert self.process.stdin is not None
        assert self.process.stdout is not None
        self.process.stdin.write(json.dumps(request) + "\n")
        self.process.stdin.flush()
        line = self.process.stdout.readline()
        self.assertTrue(line.strip(), f"{method} answered nothing")
        response = json.loads(line)
        self.assertEqual(response["jsonrpc"], "2.0")
        self.assertEqual(response["id"], request_id)
        return response

    def notify(self, method: str) -> None:
        request = {"jsonrpc": "2.0", "method": method}
        assert self.process.stdin is not None
        self.process.stdin.write(json.dumps(request) + "\n")
        self.process.stdin.flush()

    def initialize(self) -> dict[str, Any]:
        answer = self.call(
            "initialize",
            {
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": {"name": "test_local_llm.py", "version": "0"},
            },
        )
        self.notify("notifications/initialized")
        return answer


class LocalLlmTests(LocalLlmTestCase):
    def test_the_handshake_answers_the_protocol_version_the_client_sent(self) -> None:
        answer = self.initialize()
        self.assertEqual(answer["result"]["protocolVersion"], "2025-06-18")
        self.assertEqual(answer["result"]["serverInfo"]["name"], "hydra-local-llm")
        self.assertIsInstance(answer["result"]["capabilities"]["tools"], dict)

    def test_tools_list_names_ask(self) -> None:
        self.initialize()
        listed = self.call("tools/list", {})
        tools = listed["result"]["tools"]
        self.assertEqual(len(tools), 1)
        self.assertEqual(tools[0]["name"], "ask")
        schema = tools[0]["inputSchema"]
        self.assertEqual(schema["required"], ["prompt"])
        self.assertIn("prompt", schema["properties"])
        self.assertIn("system", schema["properties"])
        self.assertIn("max_tokens", schema["properties"])

    def test_a_call_returns_the_stub_text(self) -> None:
        self.initialize()
        answer = self.call(
            "tools/call",
            {"name": "ask", "arguments": {"prompt": "what is two plus two?"}},
        )
        self.assertIsNone(answer.get("error"))
        self.assertFalse(answer["result"]["isError"])
        text = answer["result"]["content"][0]["text"]
        self.assertEqual(text, "stub answer")

    def test_ollama_unreachable_is_a_tool_error(self) -> None:
        dead_port = free_port()
        dead_url = f"http://127.0.0.1:{dead_port}/api/generate"
        env = os.environ.copy()
        env["HYDRA_LOCAL_LLM_URL"] = dead_url
        env["HYDRA_LOCAL_MODEL"] = "test-model"
        process = subprocess.Popen(
            [sys.executable, str(SERVER)],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            env=env,
        )
        try:
            request = {
                "jsonrpc": "2.0",
                "id": 1,
                "method": "tools/call",
                "params": {"name": "ask", "arguments": {"prompt": "hello"}},
            }
            assert process.stdin is not None
            assert process.stdout is not None
            process.stdin.write(json.dumps(request) + "\n")
            process.stdin.flush()
            line = process.stdout.readline()
            answer = json.loads(line)
            self.assertIsNone(answer.get("error"))
            self.assertTrue(answer["result"]["isError"])
            text = answer["result"]["content"][0]["text"]
            self.assertIn("unreachable", text.lower())
        finally:
            if process.stdin:
                process.stdin.close()
            if process.stdout:
                process.stdout.close()
            if process.stderr:
                process.stderr.close()
            process.kill()
            process.wait(timeout=5)

    def test_an_overlong_prompt_is_refused(self) -> None:
        self.initialize()
        answer = self.call(
            "tools/call",
            {"name": "ask", "arguments": {"prompt": "x" * 8001}},
        )
        self.assertIsNone(answer.get("error"))
        self.assertTrue(answer["result"]["isError"])
        text = answer["result"]["content"][0]["text"]
        self.assertIn("8000", text)

    def test_a_malformed_request_is_a_json_rpc_error(self) -> None:
        assert self.process.stdin is not None
        assert self.process.stdout is not None
        self.process.stdin.write("not json at all\n")
        self.process.stdin.flush()
        line = self.process.stdout.readline()
        response = json.loads(line)
        self.assertEqual(response["error"]["code"], -32700)
        self.assertIsNone(response.get("result"))

    def test_a_method_this_server_does_not_know_is_a_json_rpc_error(self) -> None:
        self.initialize()
        answer = self.call("resources/list", {})
        self.assertEqual(answer["error"]["code"], -32601)
        self.assertIsNone(answer.get("result"))


if __name__ == "__main__":
    unittest.main()
