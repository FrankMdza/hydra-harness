# Installing Hydra

macOS, a Rust toolchain (1.88 or newer), `/usr/bin/python3` with the standard-library `json` and
`shlex` modules, and at least one of the four CLIs installed and logged in with its own
subscription. `hydra doctor` checks `/usr/bin/python3` since build ticket 358. Hydra spawns the real binaries and never touches their credentials;
that is the documented path for scripted use on all four, and `SECURITY.md` says what their terms
still forbid.

```
git clone https://github.com/FrankMdza/hydra-harness ~/hydra && cd ~/hydra
cargo build --release --workspace   # --workspace also builds hydra-puppets, hydra-grader and the rest
mkdir -p ~/.local/bin && ln -sf ~/hydra/target/release/hydra ~/.local/bin/hydra
ln -sf ~/hydra/integrations/copilot/bin/masterofpuppets ~/.local/bin/masterofpuppets
hydra doctor            # its "hydra on PATH" line should say "this binary"
```

Check that `~/.local/bin` is on your `PATH` (`echo $PATH`), and add it to your shell profile if
not. A shell alias is not enough. The delegate face is spawned by the vendor's CLI, which sees
`PATH` and knows nothing about your aliases. Hydra writes its own path into a run's face config,
so a run works either way, but `hydra` typed at a prompt needs the symlink.

## Where Hydra writes

**Hydra writes its runs, worktrees and skill store under `~/hydra/`.** The install path above
makes the two the same directory on purpose. Since build ticket 67, a launch from a checkout that
is not `~/hydra` refuses unless `HYDRA_ROOT` is set, so a foreign clone never reads or writes
`~/hydra` silently. `hydra status` prints the root it read on its first line and the policy
directory on its second. `HYDRA_ROOT=/some/dir` moves the runs, worktrees and skill store for one command;
`HYDRA_HOME=/some/checkout` moves `policies/` and `config/`, which otherwise resolve through the
source directory at compile time, so the binary is not relocatable without both.

## Logins, and what Hydra does about them

Hydra never handles an API key. You log in once with each vendor's own CLI, and that stored login
is the only way a run authenticates. The child's environment is built from an allowlist
(`src/env.rs` is the whole list), and a test there checks that `ANTHROPIC_API_KEY`,
`GEMINI_API_KEY`, `OPENAI_API_KEY`, `CURSOR_API_KEY` and `CURSOR_AUTH_TOKEN` do not reach it, for
all four providers since build ticket 359. The `run_start` line
of every launch carries an `auth` block with `mechanism: subscription-oauth`, `source: keychain`
and `allow_billed: false`. Hydra writes those three values itself, the same for all four
providers, from the fact that it passed no key. It does not ask the vendor where the login is
stored or which pool the run bills to, so read the block as what Hydra did, not as what the
vendor charged. The record's `subscription` field names who pays: `anthropic`, `cursor`, `google`
or `openai`.

| Provider | Login | What Hydra does about it |
|---|---|---|
| `claude` | its own login, read from the macOS keychain | passes the real `HOME` and never sets `CLAUDE_CONFIG_DIR`, which would lose the login |
| `cursor-agent` | its own login; its init frame reports `apiKeySource: "login"` | passes no Cursor key or token |
| `agy` | its Google login | never sets `AGY_ADC_AUTH` and passes no Google key |
| `codex` | `codex login` on a ChatGPT plan | runs `codex login status` before the launch and refuses when it says logged out |

## Versions

`hydra doctor` compares the installed binaries against the versions Hydra was last measured
with, `MEASURED_VERSIONS` in `src/provider/mod.rs`. `tests/drift.rs` re-measures what the `judge`
launch relies on in `claude` and whether `codex login status` still answers; the hook dialects
themselves are measured by hand and recorded by run id in `IMPLEMENTATION.md`. When a vendor
ships a new version, the most useful thing to report is what `hydra doctor` and `tests/drift.rs`
say about it; there is an issue template for that.

## The local model

`hydra-local-llm` is a Rust stdio MCP server with one tool, `ask`, backed by local Ollama. The
workspace build produces `target/release/hydra-local-llm`. To admit it, the human adds this block
to the fenced TOML in `config/mcp.md`, with `command` set to the binary's path on their machine:

```toml
[[server]]
name = "local-llm"
transport = "stdio"
command = "/absolute/path/to/hydra/target/release/hydra-local-llm"
args = []
admit = ["permissive"]
```

`integrations/local-llm/README.md` has the environment variables, the tool arguments and the
tests.

## Ports

Hydra is measured on macOS. Linux compiles and runs the test suite in CI since builds 315 and 331.
No sandbox wraps a Linux launch yet: the seatbelt has no Linux counterpart until build ticket 318
composes a bubblewrap profile from the same rules. Recorded runs stay on the Mac until then. A
Windows port is larger: the pty crate supports ConPTY, but the liveness test, the gate's shell
parsing (POSIX today) and the hook paths all assume a Unix shell. Both are
welcome as forks or pull requests; open an issue first so the design map gets a ticket and the
record contract does not fork with the code.
