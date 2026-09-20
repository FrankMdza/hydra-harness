# IMPLEMENTATION.md against the code

Written by `cursor-agent` on Grok 4.6 Fast, run 01a0bcdb-a7f2 under `standard`, 2.7M tokens.
All fifteen sentences are rewritten in `IMPLEMENTATION.md` in the commit that publishes this
report, so the detail stays.

## Method

I read IMPLEMENTATION.md in full, then checked each claim that names a file, a function, a field, a test, a number or a behaviour against src/, tests/, policies/ and config/. I used the file tools on src/ and grep from the shell for names. I did not run cargo test. I did not run cargo build. A sentence is a finding when another sentence in IMPLEMENTATION.md, or the code, contradicts it.

## Findings

| Id | Claim | Against | Rating |
| --- | --- | --- | --- |
| 1 | Interactive runs and harvest are not built | `src/interactive.rs`, `tests/harvest.rs`, later sections of the same file | stale |
| 2 | The shipped MCP document has no server | `config/mcp.md`, `tests/mcp.rs` | stale |
| 3 | Policy and config paths resolve only through `CARGO_MANIFEST_DIR` | `src/paths.rs`, `PolicySet::discover` | stale |
| 4 | The normalizer has two providers | `src/normalize/agy.rs` | stale |
| 5 | `HYDRA_ROOT` exists for one test file | `tests/common/mod.rs`, `tests/delegation.rs` | stale |
| 6 | Harvest still keys on `starts_with("hydra/")` | `src/normalize/claude.rs` | stale |
| 7 | A hook answer is joined by tool name and order | later section on `hook_id` | stale |
| 8 | A `permissive` run can rewrite `seed-index` because the gate protects only the gate script | `denied_write_paths`, `own_write_paths` | stale |
| 9 | `floor_lifted` is null on an interactive run | gated path in `src/interactive.rs` | stale |
| 10 | `reap` is not a command this binary answers to yet | `src/main.rs`, `src/cli.rs` | stale |
| 11 | Three of four refusal codes are still free to move | `src/record/lines.rs` | stale |
| 12 | `tests/gate.rs` asserts 23 answers | 72 `#[test]` functions in that file | stale |
| 13 | `tests/drift.rs` has two tests, both free | a third test launches `claude` | stale |
| 14 | Nothing else in the crate derives `Serialize` | `src/record/lines.rs`, `src/status.rs` | stale |
| 15 | The command listing is the `hydra run` surface | `--interactive` in `src/cli.rs` | stale |

## 1. Interactive mode and harvest are built

IMPLEMENTATION.md says this build launches no interactive runs, that harvest is missing from `hydra status`, and that both sit under not built:

```1712:1716:IMPLEMENTATION.md
**Eight of the ten types have a producer.** `message`, `thinking`, `tool_call`,
`tool_result`, `permission_decision` and `usage`, plus the two stored Hydra lines.
`message_delta` needs `--include-partial-messages` and 7.4 keeps it off in v1.
`correction` comes from an interactive harvest and this build launches no interactive
runs.
```

```2199:2202:IMPLEMENTATION.md
**Harvest is the half of `hydra status` that is not here.** Section 10 also has it
complete a dead interactive run from the vendor's transcript. No command in this build
launches an interactive run, so there is no unharvested body on this machine, and a
harvester with no producer would be untested code guarding a case that cannot arise.
```

```3222:3230:IMPLEMENTATION.md
- Harvest, ticket 12. Contract 7.2.1's transcript vocabulary has no reader on any
  provider. All three of 7.2's stream columns are read now, `agy`'s as of ticket 10.
- A rate-limited or logged-out `agy` run. `provider_refused` keys on `api_error_status`,
  which that provider does not write, and neither path has been measured, so a run
  refused for quota comes out `failed`.
- Nothing about `agy` is unbuilt. One recorded run exists, `read-only`, and no `agy` run
  under any other policy has been made, so `standard` and `permissive` on that provider
  are checked by `tests/gate.rs` against the measured payload shape and by nothing else.
- Interactive runs and harvest. 5.6, 12.
```

The same file later says the opposite, and the code matches the later account. `src/interactive.rs` is the launch. `interactive::harvest` writes `body_source: harvest`. `normalize/claude.rs` has a harvest reader that emits `correction`. `src/status.rs` harvests an abandoned interactive `claude` run. `tests/harvest.rs` drives the reader, including `a_human_rejection_and_its_retry_become_a_correction_with_the_words_kept`.

```2542:2554:IMPLEMENTATION.md
## The interactive mode, on `claude`, since build ticket 27

`hydra run --interactive --provider claude [--task ...] [--model ...]` is harness spec 3's
second launch mode. Hydra mints a run id and a `claude` session id, writes `run_start` with
`launch_mode: interactive`, `policy`, `policy_sha` and `worktree` null and no `skills`, and
starts `claude --session-id <id>` on the caller's own terminal in the caller's cwd, inheriting
stdio, under the child environment 6.2 gives every provider, with `--mcp-config` and no strict
flag. No pty of Hydra's, no detached half, no gate, no preamble, no timeout. When the process
exits, `interactive::harvest` reads the transcript at the path `claude::transcript_path`
predicts, wraps every record of it as one line under `raw`, and writes `run_end` with
`body_source: harvest`; `hydra status` does the same for a dead run whose body nobody harvested,
and a second harvest writes nothing. `normalize` reads the harvested shape by contract 7.2.1,
`correction` included, and derives no `permission_decision`.
```

```92:108:src/status.rs
/// Whether a view is a dead interactive `claude` run this build knows how to
/// harvest: an abandoned pid, a `claude` envelope, `launch_mode: interactive`.
fn is_abandoned_interactive_claude(view: &RunView) -> bool {
    view.state == State::Abandoned
        && view.provider.as_deref() == Some("claude")
        && view.launch_mode.as_deref() == Some("interactive")
}

/// Harvests every dead interactive `claude` run this scan found. Returns
/// whether anything changed, so the caller rescans only when it has to.
fn harvest_abandoned_interactive(layout: &Layout, views: &[RunView], home: &Path) -> bool {
    let mut any = false;
    for view in views {
        if !is_abandoned_interactive_claude(view) {
            continue;
        }
        match crate::interactive::harvest_in(layout, &view.run_id, None, home) {
```

I believe the code and the interactive section. The earlier sentences were true before build tickets 27 and 28 and were not rewritten when those tickets landed. A reader of "Not built" would skip `--interactive` and treat `correction` as an unused enum variant.

## 2. The shipped MCP document has a server

IMPLEMENTATION.md says the document ships empty and that the tests pin that emptiness:

```960:963:IMPLEMENTATION.md
**The document ships with no server**, because nobody has configured an MCP server on
this machine. What is in it is the format and one commented example per transport. Both
the unit tests and `tests/mcp.rs` assert that emptiness, so the day a human adds a real
server the suite says which tests now describe something else.
```

```3160:3163:IMPLEMENTATION.md
**The document ships with no server on this machine.** Adding one is a human's decision:
15.1 calls a server definition a command Hydra spawns inside every future run. The one
in `tests/fixtures/mcp-probe-server.py` is a fixture for measuring, not a server, and it
is not admitted to anything.
```

`config/mcp.md` declares the delegate face. `tests/mcp.rs` asserts that admission, not emptiness. The same IMPLEMENTATION.md file already records the later rule.

```62:64:config/mcp.md
One, since map ticket 48 on 2026-09-07: the delegate face of harness spec 9, admitted under
`standard` and `permissive` and under nothing narrower. Below it, the format and one example of
each transport, in comments, for a human to copy.
```

```66:72:config/mcp.md
[[server]]
name = "hydra"
transport = "stdio"
command = "/Users/USER/hydra/target/release/hydra"
args = ["delegate-face"]
admit = ["standard", "permissive"]
```

```518:537:tests/mcp.rs
#[test]
fn the_shipped_document_admits_the_face_to_standard_and_permissive_only() {
    // Protected config #5 as it stands on this machine since map ticket 48:
    // one server, the delegate face, admitted to the two policies a human
    // chose and to nothing narrower. Every other policy still launches with an
    // empty `mcpServers` object and refuses nothing.
    let document = Document::load().unwrap();
    for policy in ["judge", "curate", "read-only"] {
        let plan = mcp::plan(&document, Provider::Claude, policy, Checkout::Hydra);
        assert!(plan.admitted.is_empty(), "{policy}");
        assert!(
            plan.shortfall().is_none(),
            "a policy the face is not admitted to refuses nothing, {policy} included"
        );
    }
    for policy in ["standard", "permissive"] {
        let plan = mcp::plan(&document, Provider::Claude, policy, Checkout::Hydra);
        let names: Vec<&str> = plan.admitted.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, ["hydra"], "{policy}");
    }
}
```

```2581:2583:IMPLEMENTATION.md
**The delegate face is admitted under `standard` and `permissive`, since build ticket 26.**
`config/mcp.md` carries the block, and a server whose `command` has the basename `hydra` gets
`--parent <run-id>` appended in the file Hydra writes for the run, so a delegated child's depth
```

The cursor section still says an MCP tool is unmeasured "because no server is configured here." That sentence is the same stale claim in a third place.

I believe `config/mcp.md` and `tests/mcp.rs`. A reader who trusted "ships with no server" would think a `standard` run starts no MCP process.

## 3. Shipped config follows `HYDRA_HOME`, not only `CARGO_MANIFEST_DIR`

The file disagrees with itself.

```150:156:IMPLEMENTATION.md
**The shipped configuration moves with `HYDRA_HOME` since build ticket 50.** `paths::hydra_home`
answers where `policies/` and `config/` live, the variable first and the compile-time
`CARGO_MANIFEST_DIR` second, and five call sites go through it. `ensure_shipped_config` refuses
before any policy file is opened, naming the directory and the variable, and `hydra status` prints
`policies <path>` under the root line. `strings target/release/hydra | grep -c <checkout>` went from
3 to 1, the one left being the default inside the resolver. The binary is still not relocatable
without the variable, and `README.md` says so.
```

```589:597:IMPLEMENTATION.md
**Hydra finds its own policy directory through `CARGO_MANIFEST_DIR`, so the binary
only works from its source tree.** `PolicySet::discover()`, `preamble::path()` and the
four `config/` paths are compiled against the manifest directory, which is right for a
binary run out of
`target/debug` and wrong for an installed one. A copied `hydra` would compile gates
against a `policies/` that is not where it thinks. Nothing has needed an installed
binary yet, and the answer is not obvious, since the policy directory is protected
config #1 and a run must not be able to point Hydra at a different one. Naming it here
rather than leaving it as a doc comment in `src/policy/mod.rs`.
```

The code matches the first paragraph, not the second.

```24:37:src/paths.rs
/// The environment variable that overrides where `policies/` and `config/` are read from.
pub const HYDRA_HOME_VAR: &str = "HYDRA_HOME";

/// Where the shipped `policies/` and `config/` directories live.
///
/// Reads [`HYDRA_HOME_VAR`] when set, otherwise the compile-time
/// `CARGO_MANIFEST_DIR`. The compile-time path stays the default because
/// `cargo run` and the test suite both find those directories beside the source
/// tree with no setup, and because the binary has run from its own checkout
/// since the first slice. Build ticket 50 gave it an override; nothing gave it
/// an install path, and `README.md` says the binary is not relocatable away
/// from a tree that holds both directories.
pub fn hydra_home() -> PathBuf {
    hydra_home_from(std::env::var_os(HYDRA_HOME_VAR))
}
```

```250:256:src/policy/mod.rs
impl PolicySet {
    /// The set shipped beside [`crate::paths::hydra_home`].
    pub fn discover() -> Result<Self> {
        let home = crate::paths::hydra_home();
        ensure_shipped_config(&home)?;
        Self::load(home.join("policies"))
    }
```

`preamble::path`, `mcp::path` and the export allowlist path all go through `hydra_home` as well. I believe `src/paths.rs` and the ticket 50 paragraph. The later paragraph was the gap ticket 50 closed and it was left in place. A reader would think `HYDRA_HOME` does nothing.

## 4. The normalizer reads three providers, not two

```1696:1704:IMPLEMENTATION.md
## Reading a record back

**Two providers.** `src/normalize/` turns a stored `claude` or `cursor-agent` run into
contract 7's events, one adapter per column of 7.2, picked by the envelope's `provider`.
`read_file` takes a path and the policy set; `read_with` takes text and a layout, which
is how the tests ask a real question about a synthetic run without touching `~/hydra`.
Nothing is written, which contract 11 requires and `tests/normalize.rs` checks by
comparing every byte under `runs/` before and after a walk. What follows in this
section is the `claude` adapter; the `cursor-agent` section below has the other.
```

`src/normalize/agy.rs` exists and is the third adapter. The not-built list already says all three stream columns are read:

```3222:3223:IMPLEMENTATION.md
- Harvest, ticket 12. Contract 7.2.1's transcript vocabulary has no reader on any
  provider. All three of 7.2's stream columns are read now, `agy`'s as of ticket 10.
```

I believe the code and the later sentence. The "Two providers" heading was true when only `claude` and `cursor-agent` had readers.

## 5. `HYDRA_ROOT` is not a one-test override

```2522:2526:IMPLEMENTATION.md
**`HYDRA_ROOT` overrides `Layout::discover`, and it exists for one test file.** Every other
test builds a `Layout::at` a temporary directory and calls the library; `tests/face.rs`
drives the built binary as a subprocess, and the binary is what calls `discover`. It is
read in `src/paths.rs` and nowhere else, and it never reaches a child provider, because
6.2 builds that environment from an allowlist this name is not on. The rough edge below
says what is left of it.
```

The same file already records that corpus walks read the variable. `tests/common/mod.rs` reads it itself. `tests/delegation.rs` sets it on a subprocess too.

```190:197:IMPLEMENTATION.md
**The corpus walks honour `HYDRA_ROOT` since build ticket 51.** `corpus_runs_dir` in
`tests/common/mod.rs` is the one place that answers where they read, and `tests/record_contract.rs`
goes through it rather than building the path from `real_home()` itself. The CI workflow had been
setting the variable and claiming the walks read it, which was false; CI passed because the runner's
home holds no corpus. Four tests measure a particular record rather than checking every one, and
those return early on an empty corpus through `corpus_or_skip`. Measured: against an empty
`HYDRA_ROOT`, `normalize`, `outcome` and `record_contract` pass 139 of 139 in under a second;
```

```866:874:tests/common/mod.rs
/// The `runs/` directory corpus walks read.
///
/// When `HYDRA_ROOT` is set, walks `$HYDRA_ROOT/runs`. When it is unset, walks
/// `~/hydra/runs`, where `~` is the real home from the passwd entry, not `HOME`.
/// Returns `None` only when `HYDRA_ROOT` is unset and the real home cannot be
/// located.
pub fn corpus_runs_dir() -> Option<PathBuf> {
    let root = if let Some(root) = std::env::var_os("HYDRA_ROOT") {
        PathBuf::from(root)
```

`Layout::discover` still reads the variable only in `src/paths.rs`. The claim that is false is "it exists for one test file" and "every other test builds a `Layout::at`". I believe `tests/common/mod.rs` and the ticket 51 paragraph.

## 6. The harvest reader validates a gate code, it does not prefix-match `hydra/`

```2567:2569:IMPLEMENTATION.md
The harvest reader derives a `deny` by
`hydra_policy` from every error `tool_result` whose text starts with `hydra/` and pushes one
`limit` finding saying allows were not recorded, so a reader tells the two record shapes apart
```

The same file already says both paths go through `hydra_voice`. The harvest reader does that.

```162:167:IMPLEMENTATION.md
The same ticket found `claude.rs`'s harvest path testing
`starts_with("hydra/")` where its stream path went through `hydra_voice`: a tool error reading
`hydra/runs/x.jsonl: permission denied`, a guard quoting a path, was recorded as a gate that denied
the call. Both paths validate the code now. `unhooked` and `capped-argument` stay off `GATE_CODES`
on purpose: the reader writes those into a decision's reason and the gate writes the others into a
refusal, and one list for both would make a reader's sentence readable as the gate's.
```

```1340:1356:src/normalize/claude.rs
        /// **`hydra_voice` and not `starts_with("hydra/")`, since build ticket
        /// 55.** The stream path has validated the code against [`GATE_CODES`]
        /// since ticket 01 and this one did not, so the two halves of the same
        /// reader disagreed about what counts as Hydra speaking. The loose test
        /// reads `hydra/runs/x.jsonl: permission denied`, which is a guard
        /// quoting a path, as a gate that denied the call.
        fn derive_policy_decisions(&self) -> Vec<Option<PermissionDecision>> {
            if self.policy.is_none() {
                return Vec::new();
            }
            let results = self.results_by_call();
            let mut decisions = Vec::new();
            for event in &self.events {
                if let Event::ToolCall(call) = event {
                    let decision = results.get(&call.call_id).and_then(|(status, output)| {
                        (*status == ResultStatus::Error
                            && crate::normalize::hydra_voice(output).is_some())
```

I believe the code and the ticket 55 paragraph. The interactive section kept the old test. A reader would think a guard quoting `~/hydra/runs` still becomes a harvest deny.

## 7. A gate answer is joined by `hook_id`

```1733:1741:IMPLEMENTATION.md
**A hook frame carries no `tool_use_id`, so the corroboration is joined by tool name and
order, inside a window.** `hook_name` reads `PreToolUse:<Tool>` and the frame lands
between the `tool_use` block and its result. Ticket 02's run fired four `PreToolUse:Bash`
responses for one call, one Hydra's and three with an empty `output`, so filtering on a
parseable `permissionDecision` picks Hydra's out. **The window closes at the next call
that came later**, and that bound is load-bearing rather than tidy: without it the first
of two `Bash` calls takes the refusal a guard wrote about the second, and both decisions
come out wrong, one denied for somebody else's reason and one allowed. Two calls inside a
single assistant frame share a `seq` and so share a window, and they are handed answers
in hook order, which is an assumption and not a measurement. That last part reaches only
the disagreement check, never the decision.
```

```2529:2538:IMPLEMENTATION.md
## A gate answer is joined to its call by `hook_id`, since build ticket 22

The normalizer used to join a `PreToolUse` answer to its call by tool name and seq order,
because the frame carries no `tool_use_id`. Sonnet 5 broke that on 2026-09-07 in run
01a07efe-97f4: it issues the next call before the previous result arrives, so a first call's
answer lands inside what order alone reads as the second call's window, and three of sixty
calls came out `Ungated` with their answers sitting in the record. `hook_started` and
`hook_response` share a `hook_id`, and a start fires the moment a call is emitted, so the
normalizer now groups starts to calls per tool in seq order and takes each call's answer by
the ids its starts carried. A record with no `hook_started` frames keeps the old join.
```

The first paragraph states the old join as current. The second paragraph says the old join is the fallback. I believe the `hook_id` section. A reader of "Reading a record back" would think order is still the join.

## 8. The run's own spawn directory is not writable, including `seed-index`

```2245:2248:IMPLEMENTATION.md
**A `permissive` run can make the stat wrong.** Writes are scoped to the worktree under
four of the five policies, so nothing can reach the index files; `permissive` grants
unscoped writes and the gate protects only the gate script inside the spawn directory.
Such a run can rewrite its own `seed-index`, which costs the diff stat and nothing else.
```

The deny list now denies the spawn parent, and the run's own spawn directory is not an own path.

```588:608:src/policy/mod.rs
    // The three per-run parents. A sibling's worktree holds work `hydra reap`
    // has not committed yet, a sibling's checkout holds a proposal no human has
    // seen, and a sibling's spawn directory holds the gate its every call runs
    // through: a replaced one ends that run as `hydra/gate-tampered`.
    paths.push(layout.worktrees());
    paths.push(layout.spawns());
    paths.push(layout.skill_worktrees());
    paths
}

/// What this one spawn owns, carved out of the denials above.
///
/// 7.5's second kind of entry. The check both halves of the gate apply is
/// "inside a denied path and not inside an own path", which is what lets the
/// parents be denied while a run still writes in its own worktree, and a
/// `curate` pass in its own checkout of the store.
///
/// **The run's own spawn directory is deliberately not here.** Hydra writes it
/// at spawn, and a write into a sibling's copy is how a `permissive` run
/// replaces that sibling's gate. The vendor's own data root under it is a read
/// carve-out, not a write one: see [`own_read_paths`].
pub fn own_write_paths(spawn: &Spawn<'_>) -> Vec<PathBuf> {
```

The top of IMPLEMENTATION.md already says `~/hydra/spawn/` is a denied parent. `seed-index` lives under `~/hydra/spawn/<run-id>/`. A file-tool write or a shell command that names that path is `protected-path`, including under `permissive`. I believe `denied_write_paths` and `own_write_paths`. The residual in `gate.py` still lets a command that never writes the path down walk past; that is not the same as "the gate protects only the gate script."

## 9. A gated interactive run records `floor_lifted: false`

```1652:1658:IMPLEMENTATION.md
**`--allow-floor-escape` is on the run record, since build ticket 15.** Harness spec 16
says a per-run human flag "lands in the run record as an argument, so the record shows
why a run was allowed to do the thing", and `--allow-billed` has `auth.allow_billed` for
exactly that. The floor flag had no field until map ticket 42 named one, because
inventing a field is a contract change and ticket 03 left the gap where it found it.
`run_start.floor_lifted` is a boolean now, filled from `Request::floor_lifted` on every
recorded run and null on a harness run and an interactive one, where no gate is compiled.
```

The ungated arm still writes null. The gated arm compiles a gate and writes `Some(false)`.

```246:248:src/interactive.rs
        policy: Some(policy.to_string()),
        policy_sha: Some(policies.sha().to_string()),
        floor_lifted: Some(false),
```

```377:379:src/interactive.rs
        policy: None,
        policy_sha: None,
        floor_lifted: None,
```

I believe the code. The sentence treats every interactive run as a run with no gate. That was true until build ticket 28.

## 10. `hydra reap` is a command this binary answers

```3071:3074:IMPLEMENTATION.md
**The cap's refusal names `hydra logs --follow` and `hydra status`, not `hydra reap`.**
The count is of live runs, so a run that has ended has already given its slot back and
reaping it frees nothing, and `reap` is not a command this binary answers to yet either.
A refusal that tells you to run something that fails twice over is worse than one that
just says no.
```

The first half still matches `TooManyLive`. The last clause does not. `Command::Reap` is in `src/cli.rs` and `main.rs` calls `reap::reap`. I believe `src/cli.rs`. The refusal message is still right; the reason "because reap is not built" is not.

## 11. The refusal codes are settled in the enum

```3118:3123:IMPLEMENTATION.md
**Two refusal codes are minted here, not in a ticket.** Contract 6 names four Hydra-side
checks in prose and no closed ticket gives them strings. `record::Refusal` is the
vocabulary, one code each, in the shape ticket 01 gave the gate's `hydra/` reason prefix
and for the same reason: the derivation matches on it, and a sentence is not something to
match on. One of the four is written now, `skill_collision`, and a record carries it, so
that spelling is settled and the other three are still free to move.
```

```227:258:src/record/lines.rs
pub enum Refusal {
    /// A child named a policy its parent's does not contain, measured as
    /// capability set containment (contract 8.3). [`crate::run::Lineage`] decides
    /// it and 7.3 is the rule.
    PolicyNarrowing,
    /// A linked skill name the provider did not resolve as Hydra's. Ticket 07.
    SkillCollision,
    /// The MCP launch check, or Cursor's pre-flight. Ticket 08.
    ///
    /// Contract 8's `mcp_unavailable` covers both and harness spec 12 says which
    /// one fired is a field Hydra sets rather than a second outcome, so this is
    /// one code and the remediation says which check it was.
    McpUnavailable,
    /// The auth pre-flight. Not built, and the mid-run half of `auth_failed`
    /// derives from the stream instead.
    AuthPreflight,
    /// A launch from inside a live run that named no parent, or named another
    /// run as its parent. Build ticket 57 and `src/containment.rs`. It derives
    /// to `gated` the way `policy_narrowing` does, because it is the same rule
    /// caught one step earlier: 7.3's bound, missed by a flag left off rather
    /// than by a policy named too wide.
    Containment,
}

impl Refusal {
    pub fn as_str(self) -> &'static str {
        match self {
            Refusal::PolicyNarrowing => "policy_narrowing",
            Refusal::SkillCollision => "skill_collision",
            Refusal::McpUnavailable => "mcp_unavailable",
            Refusal::AuthPreflight => "auth_preflight",
            Refusal::Containment => "containment",
```

`policy_narrowing`, `mcp_unavailable` and `containment` each have a producer. `auth_preflight` is named and unused. I believe the enum. A reader would think those strings can still be renamed without breaking records.

## 12. `tests/gate.rs` is far past 23 answers

```514:517:IMPLEMENTATION.md
Tickets 01 and 02. `policies/` holds the five policies of harness spec 7.2 and the
pattern floor of 7.4, `src/policy/` loads and hashes them, `hydra::policy::write_gate`
compiles a policy name into the script of 7.5, and every `hydra run` writes one into
`~/hydra/spawn/<run-id>/` and passes it to `claude` on `--settings`. `tests/gate.rs`
feeds the compiled script synthetic hook payloads and asserts its answers, 23 of them;
`tests/spawn.rs` checks that a run writes the script where the run will find it.
```

`tests/gate.rs` has 72 `#[test]` functions. The named tests the later gate paragraphs cite are in that file. I believe the current file. Twenty-three was the count when tickets 01 and 02 closed.

## 13. `tests/drift.rs` launches a provider three times, one of them paid

```1020:1028:IMPLEMENTATION.md
**`tests/drift.rs` re-measures the vendor per suite run.** Two tests, both free, and the
only two in the suite that launch a provider. The first drives the launch's own producer,
`claude::resolve_in`, with a `judge` `Probe` and reads its `Resolved` back: `mcp` is
`Some([])` and `skills` is `None`. The second runs the same stubs with the flags off and
gets both servers, which is what keeps the first from passing because a stub was
unreadable. They skip with a printed reason when `claude` is not on the PATH, and every
call they make, the version check included, runs under a scratch `HOME` and a scratch cwd
that a passing test deletes. The failure message carries the version string, so the day
the answer moves the failure says which build moved it. The pair costs 0.7s to 1.3s here.
```

A third test, `a_claude_run_still_writes_its_thinking_text`, launches `claude` under `HYDRA_DRIFT_PAID=1`. IMPLEMENTATION.md already describes that probe in the thinking section. I believe the file. The "two tests, both free" sentence was true before the paid thinking check landed.

## 14. `Serialize` is not confined to the result projection

```1900:1905:IMPLEMENTATION.md
## Deriving an outcome, and what `hydra run` returns

Ticket 04. `src/outcome.rs` turns a normalized run into contract 8's nine outcomes and
8.2's `outcome_source`, and `hydra::run::result` projects that plus the diff stat into
contract 17's JSON. Nothing else in the crate derives `Serialize`, so the module boundary
is where contract 11's never-store rule stops being a comment.
```

`src/record/lines.rs` derives `Serialize` on `RunStart`, `RunEnd` and `Auth`. `src/record/mod.rs` derives it on the writer types. `src/status.rs` derives it on `Report`. I believe those modules. The sentence was a boundary claim from ticket 04 and did not track later derives.

## 15. The command listing omits `--interactive`

The command surface in IMPLEMENTATION.md at lines 265-276 is:

```
hydra run --provider <claude|cursor-agent|agy> --task "..." [--model <id>] [--policy <name>]
          [--parent <run-id>] [--timeout <dur>] [--allow-floor-escape]
hydra status [<run-id>]
hydra logs <run-id> [--follow]
hydra reap [<run-id>] [--delete-branch]
hydra prune [<run-id>...] [--delete] [--dry-run]
hydra curate [--model <id>] [--timeout <dur>] [--dry-run]
hydra show <run-id> [--events]
hydra usage recount [<run-id>]
hydra export --out <file> | --dry-run
hydra resume <run-id> [--task "..."] [--timeout <dur>]
hydra evolve [--model <id>] [--timeout <dur>] [--dry-run]
```

`src/cli.rs` takes `--interactive` on `hydra run`, and the interactive section documents it. `hydra evolve` also takes `--task`. The listing is presented as the command surface. I believe `src/cli.rs`. A reader copying the block would not know either flag exists.
