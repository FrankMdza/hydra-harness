"""The BitNet shim, against a stub binary that mimics llama-cli's stdout. No model is loaded."""

from __future__ import annotations

import importlib.util
import json
import os
import stat
import sys
import tempfile
import threading
import unittest
import urllib.error
import urllib.request
from pathlib import Path
from unittest import mock

APPRENTICE = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("bitnet_serve", APPRENTICE / "bitnet_serve.py")
shim = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = shim
spec.loader.exec_module(shim)

STUB = f"""#!{sys.executable}
import sys
argv = sys.argv
prompt = argv[argv.index("-p") + 1]
if prompt == "boom":
    sys.stderr.write("stub failure")
    sys.exit(3)
system = argv[argv.index("-sys") + 1] if "-sys" in argv else ""
print("banner line\\n\\nbuild : stub\\n")
print("> " + prompt + "\\n")
print(f"reply n={{argv[argv.index('-n') + 1]}} temp={{argv[argv.index('--temp') + 1]}} sys={{system}}")
print("\\n[ Prompt: 9.9 t/s | Generation: 1.1 t/s ]\\n\\nExiting...")
"""


def post(server, path: str, body: dict):
    url = f"http://127.0.0.1:{server.server_port}{path}"
    request = urllib.request.Request(
        url, data=json.dumps(body).encode(), headers={"Content-Type": "application/json"}
    )
    with urllib.request.urlopen(request, timeout=30) as reply:
        return json.loads(reply.read())


class ShimTests(unittest.TestCase):
    def setUp(self):
        self.dir = tempfile.TemporaryDirectory()
        stub = Path(self.dir.name) / "stub-cli"
        stub.write_text(STUB)
        stub.chmod(stub.stat().st_mode | stat.S_IXUSR)
        patcher = mock.patch.dict(os.environ, {"BITNET_BIN": str(stub), "BITNET_MODEL": "m.gguf"})
        patcher.start()
        self.addCleanup(patcher.stop)
        self.server = shim.serve(0)
        threading.Thread(target=self.server.serve_forever, daemon=True).start()
        self.addCleanup(self.dir.cleanup)
        self.addCleanup(self.server.server_close)
        self.addCleanup(self.server.shutdown)

    def test_generate_returns_the_reply_without_banner_echo_or_stats(self):
        got = post(self.server, "/api/generate", {
            "model": "bitnet-2b", "prompt": "line one\nline two", "stream": False,
            "options": {"temperature": 0, "num_predict": 42},
        })
        self.assertEqual(got["response"], "reply n=42 temp=0.0 sys=")
        self.assertTrue(got["done"])

    def test_chat_completions_has_the_openai_shape(self):
        got = post(self.server, "/v1/chat/completions", {
            "model": "bitnet-2b", "max_tokens": 7, "temperature": 0,
            "messages": [
                {"role": "system", "content": "be brief"},
                {"role": "user", "content": "hello"},
            ],
        })
        message = got["choices"][0]["message"]
        self.assertEqual(got["object"], "chat.completion")
        self.assertEqual(message["role"], "assistant")
        self.assertEqual(message["content"], "reply n=7 temp=0.0 sys=be brief")
        self.assertEqual(got["choices"][0]["finish_reason"], "stop")

    def test_binary_failure_is_a_500_with_the_stderr(self):
        with self.assertRaises(urllib.error.HTTPError) as caught:
            post(self.server, "/api/generate", {"prompt": "boom"})
        self.assertEqual(caught.exception.code, 500)
        self.assertIn("stub failure", json.loads(caught.exception.read())["error"])
        caught.exception.close()

    def test_missing_prompt_and_unknown_path(self):
        with self.assertRaises(urllib.error.HTTPError) as caught:
            post(self.server, "/api/generate", {"model": "x"})
        self.assertEqual(caught.exception.code, 400)
        caught.exception.close()
        with self.assertRaises(urllib.error.HTTPError) as caught:
            post(self.server, "/nope", {"prompt": "x"})
        self.assertEqual(caught.exception.code, 404)
        caught.exception.close()

    def test_extract_passes_plain_output_through(self):
        self.assertEqual(shim.extract("just text\n", "prompt"), "just text")

    def test_extract_keeps_a_reply_that_quotes_the_prompt(self):
        # Found by review on 2026-09-30: the old cut took the last occurrence of the prompt.
        self.assertEqual(shim.extract("The answer is hello world.\n", "hello"),
                         "The answer is hello world.")


if __name__ == "__main__":
    unittest.main()
