#!/usr/bin/python3
"""The smallest MCP server that answers a handshake, for measuring injection.

It speaks JSON-RPC 2.0 over stdio, offers one tool called `ping` and does
nothing else. Ticket 08 used it to find out whether a `claude` run reports the
servers Hydra passed on `--mcp-config`, which is the only source
`mcp.resolved` can honestly come from.

Nothing in the suite launches it. It is a fixture for a human measuring by
hand, and it lives here rather than in `config/` because `config/mcp.md` is a
protected config and a real server in it is a human's decision.

Run it by hand with:

    claude -p x --mcp-config <a file naming this script> --strict-mcp-config
"""

import json
import sys

TOOLS = [
    {
        "name": "ping",
        "description": "Answers pong. It exists so the server has a tool.",
        "inputSchema": {"type": "object", "properties": {}},
    }
]


def reply(rid, result):
    sys.stdout.write(json.dumps({"jsonrpc": "2.0", "id": rid, "result": result}) + "\n")
    sys.stdout.flush()


def main():
    for line in sys.stdin:
        line = line.strip()
        if not line:
            continue
        try:
            message = json.loads(line)
        except ValueError:
            continue
        method = message.get("method")
        rid = message.get("id")
        if method == "initialize":
            asked = message.get("params", {}).get("protocolVersion", "2025-06-18")
            reply(
                rid,
                {
                    "protocolVersion": asked,
                    "capabilities": {"tools": {}},
                    "serverInfo": {"name": "hydra-probe", "version": "0.1.0"},
                },
            )
        elif method == "tools/list":
            reply(rid, {"tools": TOOLS})
        elif method == "tools/call":
            reply(rid, {"content": [{"type": "text", "text": "pong"}]})
        elif rid is not None:
            reply(rid, {})


if __name__ == "__main__":
    main()
