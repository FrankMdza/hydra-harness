# 68. An attack corpus, read by Opus against the checkout

Reader: a Claude Code subagent on Opus 5, reading `/Users/USER/hydra` at `460e12d`, 2026-09-22.

Method: read `SECURITY.md`, `docs/harness-spec.md` 6.2, 6.6, 7, 8, 9 and 15, `docs/rungs-spec.md` 1,
2, 4 and 5, both reviews, `src/policy/gate.py`, `src/policy/mod.rs`, `src/sandbox.rs`, `src/env.rs`,
`src/containment.rs` and the test names in `tests/gate.rs` and `tests/spawn.rs`. Then compiled real
gates with `hydra::policy::write_gate` into a scratch layout, the way `tests/gate.rs` does, and fed
them 63 synthetic hook payloads through `/usr/bin/python3`. Also composed two seatbelt profiles in
the same scratch tree and drove 13 probes through `/usr/bin/sandbox-exec`. No provider was launched, no run was
started, nothing under `~/hydra/runs`, `~/.ssh` or `~/.claude` was read or written. The scratch
driver is 60 lines of Rust against the crate as a path dependency, so every gate below is the one
`src/policy/mod.rs` compiles and not a hand-written imitation.

Every payload that walks past a mechanism today is in `68-open-holes-opus.md` beside this file and
nowhere else, per `docs/reviews/README.md`. Holes are referenced here as H1 to H9.

One word does double duty in this document and the ticket. "The corpus" on its own is
`~/hydra/runs/`, the run records, which is what the rest of the repository means by it. The thing
this ticket proposes is the attack corpus, and it is never shortened.

## Sources

Every row was read on 2026-09-22. "Fetched" means the page itself was retrieved and quoted from.
"Search result only" means the URL and title are corroborated but the body was not retrieved, and a
claim resting on one of those is a hypothesis.

| Source | URL | Date published | Read |
|---|---|---|---|
| Claude Code security docs | https://code.claude.com/docs/en/security | undated, living | fetched |
| Claude Code security advisories, 30 entries over 3 pages | https://github.com/anthropics/claude-code/security/advisories | rolling, latest Jun 25 2026 | fetched |
| GHSA-7835-87q9-rgvv, sandbox escape via git worktree path confusion, High | https://github.com/anthropics/claude-code/security/advisories | Jun 25 2026 | fetched (index) |
| GHSA-q5hj-mxqh-vv77, trust dialog bypass via git worktree spoofing, High | same index | Apr 24 2026 | fetched (index) |
| GHSA-mmgp-wc2j-qcv7, workspace trust dialog bypass via repo-controlled settings file, High | same index | Mar 18 2026 | fetched (index) |
| GHSA-vp62-r36r-9xqp, sandbox escape via symlink following, High | same index | Apr 20 2026 | fetched (index) |
| GHSA-66q4-vfjg-2qhh, command injection via directory change bypasses write protection, High | same index | Feb 6 2026 | fetched (index) |
| GHSA-fg94-h982-f3mm, out-of-band exfiltration via a pre-approved WebFetch domain | same index | Jun 13 2026 | fetched (index) |
| GHSA-pmw4-pwvc-3hx2, path-restriction bypass via colliding prefixes | same index | Aug 1 2025 | search result only |
| CVE-2025-64755 / GHSA-7mv8-j34q-vp7q, sed validation bypass, arbitrary file write | https://github.com/advisories/GHSA-7mv8-j34q-vp7q | Nov 20 2025 | search result only |
| CVE-2025-54795 / GHSA-x56v-x2h6-7j34, `echo` command injection past an allowlist | https://cymulate.com/blog/cve-2025-547954-54795-claude-inverseprompt/ | Aug 1 2025, blog Apr 6 2026 | search result only |
| CVE-2026-21852 / GHSA-jh7p-qr78-84p7, repo settings set `ANTHROPIC_BASE_URL` before trust | https://github.com/advisories/GHSA-jh7p-qr78-84p7 | Feb 2026 | search result only |
| CVE-2025-61260 / GHSA-xrxf-jgv3-qmrm, `CODEX_HOME` redirected by a committed `.env`, 9.8 | https://github.com/advisories/GHSA-xrxf-jgv3-qmrm | Aug 2025 | search result only |
| CVE-2025-54135 CurXecute, Slack MCP content rewrites `~/.cursor/mcp.json` | https://www.tenable.com/blog/faq-cve-2025-54135-cve-2025-54136-vulnerabilities-in-cursor-curxecute-mcpoison | Aug 1 2025 | search result only |
| CVE-2025-54136 MCPoison, MCP trust bound to the key name and not the command | https://research.checkpoint.com/2025/cursor-vulnerability-mcpoison/ | Aug 5 2025 | search result only |
| Cursor changelog 1.3, denylist replaced by allowlist for auto-run | https://cursor.com/changelog/1-3 | Jul 29 2025 | search result only |
| Codex CLI approvals and sandbox modes | https://learn.chatgpt.com/docs/agent-approvals-security | undated, living | search result only |
| Codex "Overpatch" and "Heapjack" sandbox escapes, patched 0.149.0 | https://www.bleepingcomputer.com/news/security/researchers-escape-openai-codex-sandbox-to-run-commands-on-host/ | Sep 20 2026 | search result only |
| CVE-2025-49596 / GHSA-7f8r-222p-6f5g, MCP Inspector unauthenticated stdio launch, 9.4 | https://github.com/advisories/GHSA-7f8r-222p-6f5g | Jun 2025 | search result only |
| CVE-2025-6514 / GHSA-6xpm-ggf7-wc3p, `mcp-remote` OAuth endpoint into a shell, 9.6 | https://github.com/advisories/GHSA-6xpm-ggf7-wc3p | Jul 2025 | search result only |
| MCP tool poisoning, rug pulls, tool shadowing | https://invariantlabs.ai/blog/mcp-security-notification-tool-poisoning-attacks | Apr 1 2025 | search result only |
| GitHub MCP private-repo exfiltration through a public issue | https://invariantlabs.ai/blog/mcp-github-vulnerability | May 26 2025 | search result only |
| Line jumping: a tool description acts at `tools/list`, before any approval | https://blog.trailofbits.com/2025/04/21/jumping-the-line-how-mcp-servers-can-attack-you-before-you-ever-use-them/ | Apr 21 2025 | search result only |
| The lethal trifecta | https://simonwillison.net/2025/Jun/16/the-lethal-trifecta/ | Jun 16 2025 | search result only |
| Terminal DiLLMa, ANSI escapes in model output | https://embracethered.com/blog/posts/2024/terminal-dillmas-prompt-injection-ansi-sequences/ | Dec 6 2024 | search result only |
| Unicode tag smuggling used at scale against mail filters | https://www.microsoft.com/en-us/security/blog/2026/09/03/ascii-smuggling-crosses-over-from-ai-prompt-injection-to-phishing-evasion/ | Sep 3 2026 | search result only |
| CamoLeak, exfiltration through a signed image proxy | https://www.legitsecurity.com/blog/camoleak-critical-github-copilot-vulnerability-leaks-private-source-code | Oct 8 2025 | search result only |
| OWASP agentic threats T1 to T15 | https://genai.owasp.org/resource/agentic-ai-threats-and-mitigations/ | Feb 17 2025 | resource page fetched, taxonomy secondhand |
| OWASP Top 10 for LLM applications 2025, LLM01 and LLM06 | https://genai.owasp.org/llm-top-10/ | Mar 12 2025 | fetched |
| AgentDojo, 97 tasks and 629 security cases | https://arxiv.org/abs/2406.13352 | Jun 19 2024 | search result only |
| InjecAgent, 1,054 indirect-injection cases | https://arxiv.org/abs/2403.02691 | Mar 5 2024 | search result only |
| CaMeL, defeating prompt injections by design | https://arxiv.org/abs/2503.18813 | Mar 24 2025 | search result only |
| Design patterns for securing LLM agents | https://arxiv.org/abs/2506.08837 | Jun 10 2025 | search result only |
| MCPTox, 45 live servers, 1,312 poisoned cases | https://arxiv.org/abs/2508.14925 | Aug 19 2025 | search result only |
| AIShellJack, poisoned `.cursorrules` and `copilot-instructions.md`, 41 to 84 percent success | https://arxiv.org/abs/2509.22040 | Sep 26 2025 | search result only |
| ActPlane, OS-level policy enforcement for agent harnesses | https://arxiv.org/abs/2606.25189 | Jun 23 2026 | fetched |
| Execution-security SoK: isolation, access control, TOCTOU | https://arxiv.org/abs/2607.05743 | Jul 7 2026 | fetched |
| Sandlock, Landlock plus seccomp confinement for agent code | https://arxiv.org/abs/2605.26298 | May 25 2026 | search result only |
| NIST CAISI, strengthening agent hijacking evaluations | https://www.nist.gov/news-events/news/2025/01/technical-blog-strengthening-ai-agent-hijacking-evaluations | Jan 17 2025 | search result only |
| UK AISI incident report, unsanctioned agent behaviour in cyber testing | https://www.aisi.gov.uk/blog/incident-report-unsanctioned-agent-behaviour-during-cyber-testing | Aug 4 2026 | search result only |

Two quotations carry weight later, both from pages I fetched myself.

Claude Code's security page, on trust: "Trust verification is disabled when running non-interactively
with the `-p` flag." Every recorded `claude` launch Hydra makes passes `--print`
(`src/provider/claude.rs:138`), so the vendor's first-run trust dialog never fires for a Hydra run,
and the three trust-dialog advisories above describe a control Hydra never had.

The execution-security SoK reports that "policy-enforcement studies report failure rates from 69% to
98% of real denylists", that TOCTOU and MCP poisoning are the same state-validation bug studied
twice, and that "benign but out-of-scope agent actions occurring at rates up to 17.1% under realistic
prompting are addressed by no access-control or capability paper in the corpus". The first number is
the argument for the seatbelt behind the gate. The third is the argument for a corpus that records
what a run did rather than only what it was refused.

## Attack classes

Fourteen classes, each with the primary source that names it, what an attacker needs to reach it, and
which of the four routable providers it applies to on this machine.

| Class | Source | What it needs | Providers |
|---|---|---|---|
| A1. Indirect injection through file content the run reads | LLM01, AgentDojo, InjecAgent | one file in the seeded checkout or in tool output | all four |
| A2. Injection through fetched web content | Claude Code security docs, lethal trifecta | `network_read`, which `standard` grants unscoped | all four |
| A3. Poisoned agent-instruction files in the checkout: `CLAUDE.md`, `AGENTS.md`, `.cursorrules` | AIShellJack, 41 to 84 percent success | a committed file the provider loads as configuration | all four, each its own filename |
| A4. MCP tool-description poisoning and line jumping | Invariant Labs, Trail of Bits, MCPTox | one admitted server whose `tools/list` text the model reads | all four |
| A5. MCP rug pull: the description changes after the launch that admitted it | Invariant Labs, MCPoison CVE-2025-54136 | a server whose body Hydra spawns but does not pin | all four |
| A6. Hook and provider config rewritten inside the workspace | GHSA-mmgp-wc2j-qcv7, CurXecute | a write inside the worktree the run already owns | `cursor-agent`, `agy`, `codex`, and `claude` project scope |
| A7. Config root redirected by the environment | CVE-2025-61260 (`CODEX_HOME`), CVE-2026-21852 (`ANTHROPIC_BASE_URL`) | one variable reaching the child | `codex` and `claude` measured, others unmeasured |
| A8. Secret inherited through the environment | OWASP LLM02, D6 | the launcher's environment passed through | all four |
| A9. Symlink, `..` and path-assembly escape out of the scoped directory | GHSA-vp62-r36r-9xqp, GHSA-66q4-vfjg-2qhh, SoK TOCTOU | a path the checker and the kernel read differently | all four |
| A10. Command-validation bypass by respelling | CVE-2025-54795 `echo`, CVE-2025-64755 `sed`, `find`, `rg`, zsh clobber | a shell tool and a string matcher in front of it | all four |
| A11. Exfiltration through a pre-approved fetch target or a signed proxy | GHSA-fg94-h982-f3mm, CamoLeak | any outbound call the policy allows | all four |
| A12. Invisible instruction smuggling: Unicode tags, ANSI, HTML comments | Terminal DiLLMa, Microsoft Sep 2026 | text that renders as nothing and decodes as instructions | all four |
| A13. Escalation by relaunching the harness with a wider policy | this repo's own 2026-09-14 review | shell plus the binary on `PATH` | all four |
| A14. Trust and identity confusion through a git worktree | GHSA-7835-87q9-rgvv, GHSA-q5hj-mxqh-vv77 | the workspace being a worktree rather than a clone | `claude` measured, others unmeasured |

A14 deserves a sentence of its own. Every recorded Hydra run is a `git worktree add` checkout
(`docs/harness-spec.md` 6.3), and two of the four most recent Claude Code advisories are about a
worktree confusing the vendor's own path and trust logic. Hydra sits permanently in the shape those
two bugs needed.

## What Hydra answers

| Class | Mechanism | File and line | Test |
|---|---|---|---|
| A1 | Instructions are what Hydra wrote at spawn; everything read afterwards is data, and the rule rides the preamble rather than a skill | `docs/harness-spec.md:1009`, `src/preamble.rs` | none; it is advice by design, 8's own words |
| A1, A2 | Actions are bounded by a policy the run cannot widen, whatever the model is convinced of | `docs/harness-spec.md:1068`, `src/policy/gate.py:666` | `tests/gate.rs:889` through `1036`, the five policies |
| A2, A11 | A fetch argument naming a denied path is refused, `file://` included | `src/policy/gate.py:767` | `tests/gate.rs:2592` |
| A3 | Nothing. The seeded checkout's own instruction files are read by the provider and Hydra neither strips nor records them | see "What Hydra does not answer" | none |
| A4 | Admission is per policy and per server, and `mcp_call` is granted by `permissive` alone | `src/mcp.rs:13`, `policies/permissive.json` | `tests/mcp.rs` |
| A4 | An MCP call naming a denied path is refused whole under every policy, since build ticket 59 | `src/policy/gate.py:753` | `tests/gate.rs:2413` |
| A6 | The run's own hook and provider config is denied inside the worktree it otherwise owns | `src/policy/mod.rs:679`, checked first at `src/policy/gate.py:244` | `tests/gate.rs:2121`, `tests/gate.rs:2288` |
| A6 | A checkout that already holds a hook config is refused rather than overwritten | `src/provider/mod.rs:263` | `tests/spawn.rs:466`, `tests/spawn.rs:809` |
| A6 | The hook config carries a sha of the gate script, so a rewritten or emptied script fails closed | `src/policy/mod.rs:940` | `tests/spawn.rs:677` |
| A7 | The child's environment is an allowlist of four names plus per-provider additions, and three names are never set | `src/env.rs:21`, `src/env.rs:47` | `src/env.rs:152`, `src/env.rs:175`, `src/env.rs:234` |
| A8 | Same mechanism. `GEMINI_API_KEY`, the messaging socket and its token, `NODE_OPTIONS` and `ANTHROPIC_API_KEY` are dropped by construction | `src/env.rs:84` | `src/env.rs:175` |
| A9 | `realpath` before matching, so a symlink out of the worktree and a `..` path both land on the resolved target | `src/policy/gate.py:124` | `tests/gate.rs:951`, `tests/gate.rs:963` |
| A9 | `inside()` compares against `root + "/"`, so a sibling named `runs-evil` is not inside `runs`. This is the class GHSA-pmw4-pwvc-3hx2 was | `src/policy/gate.py:140` | covered implicitly by the scope tests |
| A9 | On macOS the kernel resolves the path, so every respelling the string matcher misses still fails | `src/sandbox.rs:77` | `tests/spawn.rs:1249`, and measured again below |
| A10 | The pattern floor, and it is called a floor in the spec and in the code | `src/policy/gate.py:555`, `docs/harness-spec.md:832` | `tests/gate.rs:1474` through `1644` |
| A10 | A shell row carrying no command string is refused rather than allowed, since 2026-09-22 | `src/policy/gate.py:782` | `tests/gate.rs:2364` |
| A13 | `HYDRA_RUN_ID` marks every gated child, and a launch carrying it without a matching `--parent` is refused | `src/containment.rs:38`, `src/containment.rs:78` | `src/containment.rs:110` and `tests/delegation.rs` |
| A13 | `HYDRA_ROOT` is refused from inside a seatbelt, because `sandbox_check(3)` is the one fact a child cannot shed | `src/sandbox.rs:46`, `src/run.rs:411` | `tests/spawn.rs:1543` |
| A12 partly | A delegated child's text comes back inside a boundary whose delimiter is drawn after the child stopped | `src/face.rs:263`, `src/face.rs:276` | `src/face.rs:395` |
| everything | The record carries every call the hook saw with the decision Hydra made, keyed by `hook_id` | `docs/run-record-contract.md` 7.7 | `tests/record_contract.rs` |

Three of those are worth reading as mechanism rather than as a row.

The gate compiles the policy into the script (`src/policy/mod.rs:780`) and nothing is read from disk
at fire time, which removes the whole TOCTOU class the SoK names between the hook config and the
policy. What remains of that class is the path resolution itself, and it is below.

The seatbelt is the one place Hydra stops matching strings. I composed a profile in the shape
`src/sandbox.rs:77` writes and drove it: a plain read of the denied directory, a doubled slash, a
dot segment, a `..` traversal through a sibling, a symlink planted in the worktree, and the
`/tmp` versus `/private/tmp` alias all came back "Operation not permitted". So did every case
variant, ASCII and not. That is the mechanism behind every gate hole in the next section, on macOS,
for Hydra's own directories.

The environment allowlist answers A7 and A8 better than any vendor control in the sources. `CODEX_HOME`
is not on the list at `src/env.rs:21`, so the `CVE-2025-61260` shape cannot be handed to a Hydra
child through the environment. What it can still be handed is a committed `.env` inside the
worktree, which is A3 and unanswered.

## What Hydra does not answer

### Stated as a limit already

- The gate does not interpret shell. `SECURITY.md:12` and `docs/harness-spec.md:832` both say a path
  reached through a variable, a subshell, a `cd` with a relative path or a nested `sh -c` is not
  refused, and `tests/gate.rs:1766` pins it as a passing test.
- The floor catches compiled spellings and nothing else, on `agy` with nothing behind it
  (`docs/harness-spec.md:837`). H7 is one spelling of that, held rather than published because it is
  two characters from working on a real run.
- There is no container, network cannot be cut, and the seatbelt is `(allow default)`, so it binds
  writes and corpus reads rather than the machine (`SECURITY.md:17`, `docs/harness-spec.md:984`).
- The deny list is Hydra's own directories and five config files. `~/.ssh` and the user's own
  `~/.claude` are reachable by shell under `standard` (`SECURITY.md:32`). Measured again today:
  `cat ~/.ssh/known_hosts` and `cat ~/.claude/settings.json` are both allowed by the compiled gate,
  while the same two paths through the `Read` tool are refused as out of scope.
- A child that unsets `HYDRA_RUN_ID` walks through the containment check (`SECURITY.md:71`).
- On `codex` a hook that dies without answering fails open (`SECURITY.md:94`).
- The export verifier only knows the redactor's five classes (`SECURITY.md:115`).
- Cursor's `ReadLints`, `getMcpToolsToolCall` and `awaitToolCall` never ask the hook
  (`SECURITY.md:49`).

### Not written down anywhere

Seven. Each was measured today against a compiled gate; the payloads are H1 to H9 in the held file.

1. The raw pass is a byte-substring test, so a shell-equivalent respelling of a denied path inside
   an interpreter token walks past it. Build ticket 56 added the raw pass because "the path was
   written down in full and the parser missed it" (`docs/reviews/2026-09-14-the-gate-under-attack.md:19`).
   A path split by a quote, assembled by string concatenation inside a `python3 -c`, or laundered
   through `worktrees/..` is still written down in full and still missed. H1, H2, H3. The gate's own
   docstring at `src/policy/gate.py:293` claims the corpus, the store, the snapshots and the spawn
   directory "are on the list" and therefore covered; H2 reaches the corpus and disproves that
   sentence rather than the documented residual.
2. The case fold is ASCII only on both sides (`src/policy/gate.py:114`), which is what build ticket
   60 chose so Rust and Python would agree. The filesystem is case-insensitive for non-ASCII letters
   too: `CAFÉ` opened `Café` on this machine today. So on a root whose path holds a cased non-ASCII
   letter, every denial and every scope drops for the other case. Under `permissive` that reaches the
   gate script itself, which is 7.5's induction base case. H4. This is the residual of the
   2026-09-14 fix and nothing says so.
3. `evaluate` reads the payload's shape rather than the dialect the gate was compiled for. A claude
   payload that also carries a `toolCall` object, or `hook_event_name: "beforeReadFile"`, is
   evaluated as that other call and the real `tool_name` is never checked. H5. Not reachable by a
   model today, because the provider writes the envelope. It is one provider release away from being
   reachable, and the gate already refuses a dialect mismatch on argv at `src/policy/gate.py:858`
   for exactly this reason.
4. A `tool_input` that is not an object is coerced to `{}` at `src/policy/gate.py:675`, so a shell
   call arrives with an empty command, the path check and the floor both read nothing, and the call
   is allowed. H6. Compare `src/policy/gate.py:782`, where a non-string command is refused. Same
   argument, opposite handling, ten lines apart.
5. `network_read` carries no scope in any policy that grants it (`policies/standard.json`,
   `policies/permissive.json`), and the floor's Slack rules run only under `capability == "shell"`
   (`src/policy/gate.py:831`). Measured: `curl` to a Slack method is refused and `WebFetch` to the
   same method is allowed. The lethal trifecta is complete under `standard` with no domain list
   anywhere in the repository.
6. Nothing pins what an MCP server advertises. `run_start.mcp` carries `doc_sha`, `injected` and
   `resolved` (`docs/run-record-contract.md:292`), and `resolved` is server names. A5's rug pull and
   A4's line jumping both act through tool descriptions, which no record holds and no check reads.
7. The seeded checkout's own instruction files are unanswered. `CLAUDE.md`, `AGENTS.md`,
   `.cursorrules` and `.github/copilot-instructions.md` are ordinary files the provider loads as
   configuration, and AIShellJack measured 41 to 84 percent success through exactly this door.
   `protected_worktree_paths` (`src/policy/mod.rs:679`) denies writes to the hook and MCP configs
   and says nothing about these. A run cannot be reached through them by another run, since each
   worktree is its own, but the human's repository seeds them and `hydra reap` commits whatever the
   run leaves.

There is also one sequencing fact that is not a hole and belongs here anyway: Hydra registers one
`PreToolUse` entry per provider (`src/provider/claude.rs:357`, `src/provider/agy.rs:159`,
`src/provider/codex.rs:163`) and `beforeReadFile` on Cursor. It registers no `PostToolUse` and no
`ConfigChange`. Claude Code's security page now names `ConfigChange` hooks as the way to "audit or
block settings changes during sessions". That is a second mechanism for A6 that Hydra does not use
and that is free where it exists.

## The corpus as a mechanism

The shape that fits is the one `tests/gate.rs` already has, and the evidence that it fits is that
this ticket's whole measurement ran in it. What follows is the layout, the probe format, the entry
rule, and the four things the current harness does not give a probe.

### Layout

```
tests/corpus/mod.rs        the shared rig: compile a gate, compose a profile, ask, assert
tests/corpus_gate.rs       probes that need only a compiled gate
tests/corpus_kernel.rs     probes that need a real seatbelt profile, #[cfg(target_os = "macos")]
tests/corpus_record.rs     probes over a synthetic record: what a reader is handed
```

Four files rather than one because the three rigs differ and because a probe that needs a provider
belongs in `tests/drift.rs`, which already carries the only tests that launch one. Splitting on the
rig keeps "launches nothing" checkable by reading a file header rather than a test body.

### Probe format

One test per class per spelling, named for the attack and not for the mechanism, with four things in
the body:

```rust
/// A9, symlink escape. Source: GHSA-vp62-r36r-9xqp, 2026-04-20, read 2026-09-22.
/// Status: closed by `resolve`'s realpath, src/policy/gate.py:124.
#[test]
fn a_symlink_out_of_the_worktree_does_not_widen_the_scope() { ... }
```

and, for a class that is open, the shape `tests/gate.rs:1766` already uses: assert the allow, with a
comment saying what it pins and that whoever closes it rewrites the test. That test is the one four
of the seven panel readers named as the thing they would show a colleague
(`docs/reviews/2026-09-22-seven-readers-as-a-hiring-panel.md:87`), so the format is already the
repository's and needs no argument.

A probe carries, in the doc comment: the class id, the source URL and the date it was read, the
mechanism that answers it with a file and line, and one of three statuses. `closed` asserts a deny.
`open` asserts the allow and names the ticket. `not applicable` says why the class cannot reach
Hydra, and is a test that asserts nothing, so it is a doc comment on an empty function or a row in a
table rather than a test. I would use a table for those; an empty test is a lie about coverage.

### How a class enters

Four things, and a class with three of them is a hypothesis:

1. A primary source with a URL and the date it was read.
2. A probe that runs with no provider and no path outside a scratch directory.
3. A row in the class table with the four providers marked.
4. Either a mechanism with a file and line, or a ticket number.

The attack corpus is files in the repository, so the entry rule is a paragraph at the top of
`tests/corpus/mod.rs` and nothing more. No registry, no generator.

### What a probe needs that the harness does not give it

- `tests/gate.rs`'s `Gate` struct is private to that file, 350 lines of it
  (`tests/gate.rs:50` to `418`). A second test file cannot compile a gate today without copying it.
  Moving it to `tests/corpus/mod.rs` and having `tests/gate.rs` use it is the cheapest fix and costs
  one file move plus an import; leaving it copied costs a second copy that drifts.
- The same is true of the seatbelt rig: `under_seatbelt` and `refused` are private helpers at
  `tests/spawn.rs:1225` and `tests/spawn.rs:1244`.
- No fixture builds a layout under a non-ASCII root, which is what H4 needs. That is one line,
  `Layout::at(root.join("Café"))`, and it is the only thing standing between H4 and a test.
- Nothing in the suite asserts about a payload the provider did not send. H5 and H6 are shape-drift
  probes, and a rig for them wants a raw `ask(dialect, serde_json::Value)` rather than the typed
  `ask(tool, input)` helpers. `raw_answer` at `tests/gate.rs:248` is already that function and is
  also private.

So the honest cost of the corpus is one file move, one import, one line of fixture, and the probes
themselves. Everything else exists.

## How a fix flows

The ticket offers two routes and the code has already chosen for most of them.

**Through `hydra evolve`, it cannot.** `PROTECTED_CODE_PATHS` at `src/rungs/evolve.rs:447` holds
`src/policy`, `src/env.rs`, `src/record`, `src/spawn_config.rs`, `src/sandbox.rs`,
`src/containment.rs` and the two Rungs files, and `protected_path_hit` at
`src/rungs/evolve.rs:465` refuses the whole patch before any judge reads it. Every one of the seven
unstated holes above is fixed in `src/policy/gate.py`, `src/policy/mod.rs` or `src/sandbox.rs`. So a
patch that closes a probe is refused at admission by construction. That is not a gap; it is 15.1 and
4.4 working. It does mean the sentence "a patch that closes a probe is reviewed by the panel" is
false for the probes this ticket found.

**The attack corpus itself is not protected, and that is backwards.** `tests/` appears nowhere in
`PROTECTED_CODE_PATHS`, and the admission gate runs `cargo test` with `current_dir(worktree)`
(`src/rungs/evolve.rs:585`), which is the proposer's patched worktree. `docs/rungs-spec.md:468` names
the refinement that would stop this, "protected test files come from the baseline sha, not from the
patched worktree", and the next line says "Not built as of 2026-09-12". So today a rung-two proposer
may delete a probe and pass admission, while it may not touch the mechanism the probe tests. The
only thing standing is `baseline_test_provenance`, a judge checklist item
(`docs/rungs-spec.md:742`), which is a model's judgement and which rungs spec 2.1 itself says is not
a mechanism.

Two options, priced.

*Option A, add `tests/corpus` to `PROTECTED_CODE_PATHS`.* One line in `src/rungs/evolve.rs`, one
assertion in `tests/evolve.rs`, one row in `docs/harness-spec.md` 15.2, which becomes fifteen. Cost:
rung two can no longer add a probe either, so a pass that finds a new class has to stop and say so,
which is the same move a build ticket that finds a decision makes. Does not help with a proposer
that changes a probe's fixture in a file that is not protected.

*Option B, build the baseline-test refinement.* The admission gate checks out the baseline sha into
a second directory and runs the protected tests from there against the patched source. Cost by my
reading: roughly 60 to 100 lines in `src/rungs/evolve.rs`, a second worktree per pass, one more
`cargo build` of the baseline, and a decision about what "protected test" means, since `cargo test`
compiles the whole suite from one manifest and cannot mix two trees without a wrapper. It closes the
class rather than one path, and it is the sentence the spec already promises, so it also removes a
documented falsehood. It is the more expensive of the two by an order of magnitude.

They compose: A is the thing to do this week and B is what A is a placeholder for.

**Through a build ticket a human writes, everything else.** That is the route the seven unstated
holes have to take, and the route this repository has used for every gate fix so far: build tickets
56, 57, 58, 59, 60, 70, 74, 75. The pass a human needs to see, for a probe-closing ticket, is the
one `docs/reviews/README.md` already asks for: the probe as a failing test before the fix, the same
probe as a passing test after it, and a line in `SECURITY.md` if a residual is left. Nothing new is
needed.

## The research agent

Three shapes were on the table. Costed against what the five policies actually grant:

**As a rung.** Ruled out by the documents the ticket already names. `docs/rungs-spec.md:36` says both
rungs stop at a branch a human merges, `docs/rungs-spec.md:157` says rung one is invoked rather than
scheduled, and a third rung is a new crossing, which `no_new_crossing` at `docs/rungs-spec.md:771`
is a critical checklist item about. A loop that patches its own security is the loop
`docs/rungs-spec.md` 3.6 rules out. There is nothing to cost here; the answer is in the spec.

**As a `hydra run` under an existing policy.** This works today and is the cheapest thing on the
page. A run under `standard`, in its own worktree of Hydra's checkout, may read `src/`, `docs/` and
`tests/`, write the probe files, and shell out to `cargo test --test corpus_gate`. Its scratch gate
compiles into `/tmp`, which no denial covers, and the seatbelt allows writes everywhere but Hydra's
own directories. `network_read` is granted unscoped, so it can fetch an advisory page. `hydra reap`
puts the branch in front of the human. That is exactly what this ticket did, minus the worktree.

What it cannot do, and the one thing worth a decision: **no policy grants both corpus reads and
shell.** `curate` reads the corpus and has no `shell` (`policies/curate.json`); `standard` has shell
and scopes reads to the worktree (`policies/standard.json`). A corpus that wants to check a class
against what runs actually did, which is the SoK's 17.1 percent point about out-of-scope actions, has
to be two runs or a sixth policy. Two runs cost a summarizing pass and a file handed between them.
A sixth policy costs one file in `policies/`, which is protected config #1, so a human writes it,
plus a row in `docs/harness-spec.md` 7.2 and the record's `policy` field gaining a value every
reader must handle. I would not pick; both are small and they differ in who owns the decision.

**Outside Hydra.** A shell script and a cron entry. Cheapest to build and it breaks D10 the moment
it is scheduled, which is `no_daemon` at `docs/rungs-spec.md:772`. Invoked by hand it is the same
thing as the `hydra run` above with no record, which is the one thing this project spends everything
to have.

What such an agent reads: `docs/`, `src/policy/`, `src/sandbox.rs`, `SECURITY.md`, the corpus file,
and the open web. What it writes: `tests/corpus*`, and a ticket under `.scratch/hydra/` when it finds
a decision. What it must never write: the seven protected paths, which the gate already denies, and
`SECURITY.md` itself, which is not denied to anything today and is the file a hole gets published in.

One thing the five policies do not grant and that this shape wants: nothing. That is the finding. A
research agent updating an attack corpus is an ordinary `standard` run, and the only reason to build
anything new is the corpus-read question above.

## What I tried

63 payloads against four compiled gates, `standard` and `permissive` on an ASCII root and on a
non-ASCII one, plus 13 probes against two seatbelt profiles. No provider launched, nothing outside
`/private/tmp/.../scratchpad/opus-68/` touched. The interesting ones, grouped.

**The raw pass, `standard`, claude dialect.** A plain corpus read denies. The same read inside
`sh -c` denies, which is build ticket 56 working. A trailing doubled slash denies, because the
denied prefix is still a substring. A doubled slash or a dot segment *inside* the prefix allows
(H3). A quote split inside a single-quoted interpreter argument allows (H1). String concatenation
inside `python3 -c` allows (H1). `worktrees/../runs/x` inside `sh -c` allows and reaches the corpus
(H2). `cd <corpus> && cat x` denies, because the `cd` names it. A sibling's spawn directory inside
`sh -c` denies; a sibling's worktree allows, which is the documented residual.

**The floor.** `git push --force origin master` denies. The same command behind `sh -c` allows, and
behind `env` allows (H7). `check_slack` sees the command string, so `curl` to a Slack method denies;
`WebFetch` to the same method allows, because the floor runs only under the `shell` capability.

**Case and normalization.** Under a layout rooted at a directory named `Café`: the exact spelling
denies, `CAFÉ` allows through the shell check, through `Write` under `permissive`, and for a write to
the gate script itself. NFD versus NFC allows too. The same profile composed for the kernel refuses
every one of those spellings, so the exposure is Linux-shaped and record-shaped rather than
immediate: the record would carry `hydra/allow` for a call the kernel refused.

**Payload shapes.** `command` as a list or a dict denies with `hydra/bad-payload`. `tool_input` as a
string or a list allows the Bash call with the command never read (H6). A payload carrying both
`tool_name: "Bash"` and a `toolCall` object is evaluated as the `toolCall` (H5); the same with
`hook_event_name: "beforeReadFile"` beside a real Bash command, likewise. `cwd` as a list falls back
to the compiled worktree and behaves. A 200-deep nested array allows, a two-megabyte command string
allows, both without crashing, which is build ticket F2's catch-all holding.

**MCP and fetch, `permissive`.** An argument naming the corpus denies. A mid-prefix doubled slash
allows, a relative path allows, and a path split across two argument keys allows (H9): the raw pass
over MCP arguments has the same byte-substring weakness and `src/policy/gate.py:333` says so for the
relative case only. A `file://` URL naming the corpus denies; the same URL with the path
percent-encoded allows (H8).

**The kernel.** A profile in `src/sandbox.rs:77`'s shape refused a plain read, a doubled slash, a
dot segment, a `..` traversal, a planted symlink, the `/private` alias, and every case variant, and
allowed the worktree write. Every gate hole above is stopped here on macOS for Hydra's own paths.

**Attacks I expected to work and that did not.** A tool name in another case is `hydra/unknown-tool`,
so the capability table fails closed on drift. A `Write` whose path field name is wrong is
`hydra/no-path` under every policy, which is F27. An embedded NUL in a path does not escape: the
truncated path is always a prefix of the checked path, so truncation can only shorten and never
leave the scope. `inside()` has no prefix-collision bug, which is the class one of the Claude Code
advisories is. And a `Read` of `~/.ssh` or `/etc/passwd` through the file tools is refused as out of
scope even though the shell reaches both, which is `SECURITY.md:32`'s sentence being precisely true.

## Questions for the human

1. H4 is the one that changes a promise rather than a spelling: on a non-ASCII root, `permissive`
   can write the gate script. Is the fix folding Unicode on both sides, which is the thing build
   ticket 60 deliberately undid, or comparing resolved inodes rather than strings, or refusing a
   non-ASCII `hydra_home()` at pre-flight with one line in `hydra doctor`? The third is the cheapest
   and the most honest about what the gate is.
2. H1 to H3 say the raw pass's claim in `src/policy/gate.py:293` is wider than the code. Does the
   docstring get narrowed to what it does, or does the raw pass get a normalization step before the
   substring test? A normalization step is maybe fifteen lines and it will not close the class, only
   the spellings someone thought of, which is 7.4's failure mode arriving in a second place.
3. H5 and H6 are unreachable today and one provider release from reachable. Are shape-drift probes
   in the corpus at all, or do they belong with `tests/drift.rs`, which is where a provider change
   becomes a failing test?
4. `network_read` is unscoped in the two policies that grant it, and there is no domain list in the
   repository. Is that a decision the map should take, given that it is the third leg of the lethal
   trifecta and the one leg a policy could actually remove?
5. The attack corpus lives in `tests/`, which rung two may edit and rung two may not fix. Option A
   this week and Option B later, or straight to B?
6. Should the attack corpus have a `not applicable` row at all, or should a class Hydra structurally cannot
   meet simply be absent? A table row is cheap and an empty test is a lie.
7. A research agent is an ordinary `standard` run today. The only thing it cannot do is read the
   corpus while holding a shell. Sixth policy, two runs, or leave the corpus out of its evidence?
