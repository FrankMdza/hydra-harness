#!/usr/bin/env python3
"""A small HTTP shim in front of the bitnet.cpp binary.

It answers Ollama's POST /api/generate with {"response": ...} and POST /v1/chat/completions in the
OpenAI shape. Each request runs the binary once with the prompt, so the model reloads every time
(about nine seconds on the M2, see README.md). Requests run one at a time: the machine has 8 GB.

    python3 integrations/apprentice/bitnet_serve.py [--port 11435]
    HYDRA_LOCAL_LLM_URL=http://127.0.0.1:11435/api/generate HYDRA_LOCAL_MODEL=bitnet-2b ...

Standard library only. The binary and the model are read from BITNET_BIN and BITNET_MODEL and
default to the llama-cli and the i2_s gguf under ~/models/BitNet.
"""

from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
import threading
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

BITNET_HOME = Path.home() / "models" / "BitNet"
DEFAULT_BIN = BITNET_HOME / "build" / "bin" / "llama-cli"
DEFAULT_MODEL = BITNET_HOME / "models" / "BitNet-b1.58-2B-4T" / "ggml-model-i2_s.gguf"
DEFAULT_PREDICT = 300
TIMEOUT_SECONDS = 900
STATS = re.compile(r"\n*\[ Prompt: .*$", re.S)
ANSI = re.compile(r"\x1b\[[0-9;]*[A-Za-z]")

RUN_LOCK = threading.Lock()


def binary() -> str:
    return os.environ.get("BITNET_BIN", str(DEFAULT_BIN))


def model_path() -> str:
    return os.environ.get("BITNET_MODEL", str(DEFAULT_MODEL))


def extract(stdout: str, prompt: str) -> str:
    """The reply out of llama-cli's stdout: after the echoed prompt, before the stats line.

    A binary that prints only the reply, with neither echo nor stats, passes through unchanged.
    """
    text = ANSI.sub("", stdout)
    at = text.rfind(prompt.strip())
    if at != -1:
        text = text[at + len(prompt.strip()):]
    return STATS.sub("", text).strip()


def run_binary(prompt: str, predict: int, temperature: float, system: str | None = None) -> str:
    argv = [
        binary(), "-m", model_path(), "-p", prompt, "-n", str(predict),
        "-t", os.environ.get("BITNET_THREADS", "4"), "--temp", str(temperature),
        "-ngl", "0", "-st", "--no-display-prompt", "--simple-io", "--no-warmup",
    ]
    if system:
        argv += ["-sys", system]
    with RUN_LOCK:
        done = subprocess.run(
            argv, stdin=subprocess.DEVNULL, capture_output=True, text=True,
            timeout=TIMEOUT_SECONDS, check=False,
        )
    if done.returncode != 0:
        raise RuntimeError(f"{Path(argv[0]).name} exited {done.returncode}: {done.stderr[-300:]}")
    return extract(done.stdout, prompt)


def chat_parts(messages) -> tuple[str | None, str]:
    """System text and the rest of the conversation flattened into one prompt."""
    system, rest = [], []
    for message in messages:
        content = message.get("content", "")
        if isinstance(content, list):
            content = "".join(p.get("text", "") for p in content if isinstance(p, dict))
        (system if message.get("role") == "system" else rest).append(str(content))
    return ("\n".join(system) or None), "\n".join(rest)


class Handler(BaseHTTPRequestHandler):
    def _send(self, code: int, body: dict) -> None:
        raw = json.dumps(body).encode()
        self.send_response(code)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(raw)))
        self.end_headers()
        self.wfile.write(raw)

    def do_POST(self):
        try:
            request = json.loads(self.rfile.read(int(self.headers.get("Content-Length", 0))))
            if not isinstance(request, dict):
                raise ValueError("body is not an object")
        except (ValueError, json.JSONDecodeError) as err:
            return self._send(400, {"error": f"bad request: {err}"})
        model = request.get("model", "bitnet-b1.58-2b-4t")
        if self.path == "/api/generate":
            options = request.get("options") or {}
            predict = options.get("num_predict", DEFAULT_PREDICT)
            temperature = options.get("temperature", 0)
            prompt, system = request.get("prompt"), request.get("system")
        elif self.path == "/v1/chat/completions":
            predict = request.get("max_tokens", DEFAULT_PREDICT)
            temperature = request.get("temperature", 0)
            system, prompt = chat_parts(request.get("messages") or [])
        else:
            return self._send(404, {"error": f"no such path {self.path}"})
        if not isinstance(prompt, str) or not prompt:
            return self._send(400, {"error": "no prompt"})
        try:
            text = run_binary(prompt, int(predict), float(temperature), system)
        except (RuntimeError, subprocess.TimeoutExpired, OSError, ValueError) as err:
            return self._send(500, {"error": str(err)})
        if self.path == "/api/generate":
            return self._send(200, {"model": model, "response": text, "done": True})
        self._send(200, {
            "id": f"chatcmpl-{int(time.time() * 1000)}",
            "object": "chat.completion",
            "created": int(time.time()),
            "model": model,
            "choices": [{
                "index": 0,
                "message": {"role": "assistant", "content": text},
                "finish_reason": "stop",
            }],
        })

    def log_message(self, *args):
        pass


def serve(port: int) -> ThreadingHTTPServer:
    return ThreadingHTTPServer(("127.0.0.1", port), Handler)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--port", type=int, default=11435)
    args = parser.parse_args(argv)
    server = serve(args.port)
    print(f"bitnet_serve: listening on 127.0.0.1:{server.server_port}", file=sys.stderr)
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        pass
    return 0


if __name__ == "__main__":
    sys.exit(main())
