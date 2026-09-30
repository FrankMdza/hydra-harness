# Map ticket 88, seat `architect` (claude)

Read 2026-09-30 from this worktree at 97eb3d5, plus pi.dev and pi-mono's `docs/extensions.md`.

### Not confirmed

- **The BitNet shim is not an OpenAI-compatible endpoint, and it does not exist yet.** Build 161
  (`.scratch/hydra/build/161-bitnet-as-the-second-apprentice.md:24-27`) specifies a shim that
  answers Ollama's `/api/generate` with `{"response": ...}`. That is plain completion with no
  tool calls. `integrations/apprentice/` is absent from this worktree. The ticket's "so would the
  BitNet shim of build 161" is false as written.
- **Whether `gemma3:1b-it-qat` emits usable tool calls through Ollama at all.** Nothing in the
  repo measures it. Map 87 says it "cannot use tools reliably" and failed as a judge (3 of 15).
  A harness gives it hands; it does not give it the skill to use them.
- **pi's session file format.** pi.dev says "sessions are stored as trees" in one file; the docs
  page I tried for the entry fields returned 404, so `id`/`parentId` is the ticket's reading and
  not mine.
- **Whether pi can be launched with no extensions but Hydra's.** Its extension docs say a
  `tool_call` handler can return `{ block: true, reason }`, and that an extension "runs inside the
  Pi process with the same operating-system permissions". I did not find a flag that stops user
  or project extensions from loading beside Hydra's.
- **The session costs below are my estimate**, scaled from build 48 as the ticket reports it (14
  files, 1,365 insertions). I did not measure how many sessions build 48 took. Today 24 files under
  `src/` and `tests/` name codex.

## 1. Would routing pi as a fifth provider extend Hydra?

Barely, and it would move the gate from outside the provider to inside it. Every other provider
runs Hydra's gate as a separate process the binary calls (`src/policy/mod.rs:1260`
`hook_command`, harness spec 6.6 at `docs/harness-spec.md:485`); on pi the only veto is a
TypeScript `tool_call` handler that shares a process with any other extension that loads, which is
exactly the widening 7.1 exists to refuse. pi takes API keys, and `Provider::subscription`
(`src/provider/mod.rs:181-188`) has no answer for a key, while passing one through the env
allowlist means editing protected config #2 (`docs/harness-spec.md:1431`). The four-tool table
would be the smallest in `src/policy/table.rs` (codex has five rows at `:390`), so that part is
cheap. The expensive parts are the ones codex paid: a `Provider` variant (`src/provider/mod.rs:20`),
a dialect (`:57-63`, `src/policy/mod.rs:613`, `gate.py:41`), an argv arm (`src/run.rs:1470-1496`),
a worktree precondition arm (`src/run.rs:756-771`), a normalizer (`src/normalize/mod.rs:503-507`),
and the envelope's provider enum (`docs/run-record-contract.md:153`). What it would buy for the
minis is real, since pi speaks to Ollama, but the gate on those runs would rest on code inside a
process Hydra does not own.

## 2. What a Rust harness of Hydra's own needs, and what to copy from pi

**The gate needs nothing new if the harness speaks Claude's dialect.** Codex already does
(`src/provider/mod.rs:61`), and `gate.py` reads only `tool_name`, `tool_input` and `cwd`
(`src/policy/gate.py:939-958`) on stdin and answers in `answer_for`'s shape (`:48-66`). The harness
pipes each call to the compiled `hook_command` before running it, and the one rule it must hold
that `claude` does not is to fail closed: 6.6 records that `claude` allows on an unrecognised
answer, and our loop should treat anything but a parsed `allow` as deny. The provider table needs
one four-row `Table` in `src/policy/table.rs:418` with `Bash` as `Shell`, `Read` as `FileRead` on
`file_path`, and `Write` and `Edit` as `FileWrite` on `file_path`. Using Claude's names and fields
keeps `path_fields` and `command_field` identical to `CLAUDE`'s. The record needs a fifth value in
the envelope's `provider` column (`docs/run-record-contract.md:153`), a normalizer arm
(`src/normalize/mod.rs:503`, which today reads an unknown provider with `claude`'s adapter and
emits a finding), and a producer column entry per event type in contract 7.2 (`:585`). Emitting
frames shaped like `claude`'s stream-json (`assistant` with `tool_use`, `user` with `tool_result`)
would let the new adapter reuse `normalize/claude.rs`'s block parsing without claiming to be
`claude`. Copy the four tools, the one stream on stdout that `watch.rs` tees, and "primitives, not
features"; do not copy extensions, the fifteen-provider client, RPC mode for now, or session trees,
because Hydra's lineage already lives across files in `parent_run_id` (contract 10) and a second
tree inside one file is a second lineage model.

## 3. The smallest cut that gives the minis hands

A Rust loop over one OpenAI-compatible `/v1/chat/completions` endpoint (Ollama's, not the BitNet
shim's), four tools, every call through `hook_command` in the Claude dialect, stdout frames the
watcher tees, `print` launch mode only, no MCP, no skills, no resume
(`Provider::supports_resume` at `src/provider/mod.rs:192` would need its first `false`). It runs
inside the same seatbelt profile as every launch (`src/sandbox.rs`), so the gate is the veto and
the sandbox is still the wall. It is a provider in the contract's sense, not a "harness run":
contract 5.5 (`docs/run-record-contract.md:393`) already uses that name for an `evolve` pass with
a null provider, and the new thing should not borrow it. For map 87 it turns the bank's open class
into tasks the model does and a record grades: "write the `hydra run` command" becomes "run it",
the gate's refusals become verifiable labels, and every attempt is a run record the existing
readers (`hydra show`, the replay, `export_preference.rs`) already consume. My estimate is three
sessions: one for the loop and the gate call against a stub endpoint, launching nothing; one for
the `Provider` variant through the files codex touched; one for a live run on Ollama with a refusal
measured by reverting the fail-closed rule.

### Recommendation

Do not route pi; build the small harness, and only after one measurement that costs no session:
ask `gemma3:1b-it-qat` through Ollama's `/v1/chat/completions` with a `tools` array for ten bank
tasks and count well-formed tool calls. If that count is near zero, the harness gives a model hands
it cannot use, and the answer is to wait for the 4B map 87 already names. If the model does call
tools, the cut in question 3 costs about three sessions by my estimate, adds no dialect and no
protected config, and is the first provider whose gate Hydra enforces in its own code rather than
trusting a vendor binary to call it. One thing in it is a decision rather than an implementation:
where the endpoint URL and model name live, since a run that can rewrite them picks its own model,
and that belongs on the map before build.
