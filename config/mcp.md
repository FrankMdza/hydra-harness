# The canonical MCP document

Protected config #5 of harness spec 15.1, and the document harness spec 6.5 asks for: one
declaration of every MCP server Hydra may hand a run, which generates the per-provider file and is
none of them. `claude` reads its copy from `~/hydra/spawn/<run-id>/mcp.json` on `--mcp-config`,
`cursor-agent` from `<worktree>/.cursor/mcp.json`, and `agy` from
`<worktree>/.agents/mcp_config.json`. The stdio server body is the same on all three; the file
name, the directory and the remote key are not.

**A server here is a command Hydra spawns inside a run.** That is why this file is protected: the
gate denies writes to it under every policy, and a rung-two patch touching it is refused before
review. Adding a server is a human's decision, made by editing this file by hand.

**Hydra reads one thing in this file: the single fenced `toml` block below.** Everything outside it
is prose for whoever edits it. The block has to be there, and there has to be exactly one, so a
broken fence fails the launch instead of quietly admitting nothing.

**`mcp.doc_sha` on every run record is `shasum -a 256 config/mcp.md`**, over the whole file. Editing
this prose moves that value, which is the honest answer: the document changed.

## The fields

| Field | Required | Means |
|---|---|---|
| `name` | yes | the key the provider sees. Letters, digits, hyphen and underscore, and never `__`, which is what `claude` joins a server and a tool with |
| `transport` | yes | `stdio`, `http` or `sse` |
| `admit` | yes | the policy names this server reaches, from `judge`, `curate`, `read-only`, `standard`, `permissive` |
| `command` | stdio | the binary Hydra spawns |
| `args` | no | its arguments |
| `env` | no | environment for that command, as a table of strings |
| `url` | http, sse | the endpoint |
| `headers` | no | headers for it, as a table of strings |

A server that names both a command and a url is refused, because `agy` refuses that pair and a
document that cannot generate all three files is not doing its job. An unknown field is refused
too: `admits` for `admit` would otherwise parse and admit the server to nothing.

## What `admit` does and does not do

It decides which runs are handed the server. It does not decide whether they may call it: the
delegation policy does that. `permissive` grants `mcp_call` for every server, and `standard`
for `hydra` alone. So a server admitted to `standard` under any other name starts, its tools
reach the model, and the gate refuses every call to it. Admitting a server to a policy that
cannot call it costs a process and some context, and buys nothing.

`judge` is narrower still. Harness spec 6.7 gives a `judge` run `--restricted` and no
`--mcp-config`, so nothing written here reaches it. A server admitted to `judge` fails the launch
with `mcp_unavailable` rather than launching a run that quietly came up short.

## Hydra's own binary as a server

A server whose `command` has the basename `hydra` gets `--parent <run-id>` appended to its
`args` in the file Hydra writes for the run, and nowhere else: this document and `mcp.injected`
on the record stay as declared. Map ticket 48 chose this over a token or a field so the format
here does not grow a rule a human has to know to write an ordinary server. The match is on the
basename, so an absolute path to the binary enters too, and the block below uses one because a
recorded run's `PATH` is the human's and `hydra` is not on it: measured 2026-09-07, a bare
`hydra` came up `failed` in `claude`'s init frame and `mcp.resolved` said `[]`. It is a rule keyed
on a string, which is why it is written here beside the table rather than left in the code.

## The servers

One, since map ticket 48 on 2026-09-07: the delegate face of harness spec 9, admitted under
`standard` and `permissive` and under nothing narrower. Below it, the format and one example of
each transport, in comments, for a human to copy.

```toml
[[server]]
name = "hydra"
transport = "stdio"
command = "hydra"
args = ["delegate-face"]
admit = ["standard", "permissive"]

# [[server]]
# name = "filesystem"
# transport = "stdio"
# command = "npx"
# args = ["-y", "@modelcontextprotocol/server-filesystem", "/Users/you/projects"]
# admit = ["permissive"]
#
# [server.env]
# SOME_TOKEN = "..."

# [[server]]
# name = "docs"
# transport = "http"
# url = "https://example.com/mcp"
# admit = ["standard", "permissive"]
#
# [server.headers]
# Authorization = "Bearer ..."
```

The `[server.env]` and `[server.headers]` tables belong to the `[[server]]` above them, which is
TOML's own rule about the last array entry. Keep them directly underneath it.
