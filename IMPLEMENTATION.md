# What is built

`docs/` is the design, finished and checked. This file is the gap between it and the
code, kept current by whoever narrows the gap.

Read it before trusting a run record written by this binary. **No record this binary
has written was gated, and three of its fields say so.** Build ticket 01 compiled the
policy set and the gate; ticket 02 is what passes one to a running provider, and until
it lands every record still carries a null policy.

Provider versions this slice was built and measured against, read 2026-09-05:

| Provider | Version | Read |
|---|---|---|
| `claude` | 2.1.261 | 2026-09-05. The specs were written against 2.1.257; every flag in harness spec 6.7 survives the move |
| `cursor-agent` | 2026.09.02-c22c1a3 | 2026-09-05, unchanged since 2026-09-02 |
| `agy` | 1.1.26 | 2026-09-05, unchanged since 2026-09-04 |

## The one command

```
hydra run --provider claude --task "..." [--model <id>]
```

It builds the child's environment from the allowlist, reads the provider's version,
mints a UUIDv7 run id, creates a git worktree and seeds it with the parent's
uncommitted work, spawns `claude` on a pty it owns, writes `run_start`, tees every
frame verbatim into `~/hydra/runs/<run-id>.jsonl`, and writes `run_end` when the
child exits. It prints JSON on stdout.

`cursor-agent` and `agy` are routable in the design and refused here by name, with an
error that says the slice routes one provider rather than that the provider is
unknown.

## The policy set and the gate, compiled but not yet passed

Build ticket 01. `policies/` holds the five policies of harness spec 7.2 and the pattern
floor of 7.4, `src/policy/` loads and hashes them, and `hydra::policy::write_gate`
compiles a policy name into the script of 7.5. **Nothing launches it.** No provider is
spawned with `--settings`, so a run record's `policy` is still null and the departures
below are unchanged. `tests/gate.rs` feeds the compiled script synthetic hook payloads
and asserts its answers, 23 of them.

**The script is Python, and its shebang is `/usr/bin/python3` rather than
`/usr/bin/env python3`.** The language is what 7.5's ticket expected: the gate needs
JSON and path matching, macOS ships an interpreter, and the user's own 418 lines of
`PreToolUse` guards are already Python. The absolute path is the part worth stating.
`env` resolves through the child's PATH, the child's PATH is inherited, and on `claude`
a hook that fails to start fails **open** (6.6). A PATH lookup is the one dependency
this script cannot afford. `/usr/bin/python3` is 3.9.6 on this machine, read 2026-09-05.

**The suite runs the gate on that interpreter and not on the shell's.** `python3` on
this shell's PATH is Homebrew's 3.14.7. Testing it would leave the interpreter that
actually runs the gate unmeasured, which is the drift the build plan's second note
warns about, one layer down from the providers.

**`policy_sha` is a sha256 over every `.json` in `policies/`, in name order, each file's
name and body framed by its length.** `README.md` is left out: it is prose about the
policies rather than one of them, and a wording fix should not move a value every run
record carries. The framing is what stops two directories colliding by concatenation.

**Nineteen of `claude`'s twenty-five tools answer to no capability the five policies
speak in.** 7.1 denies a tool name the table does not recognise, so a table covering
only the six capabilities of the coverage matrix would deny nineteen tools under
`permissive`, which is supposed to allow everything. They map to one capability,
`provider_native`, granted by `permissive` alone. One row rather than nineteen invented
categories, and 7.1's drift check still fires on a twenty-sixth name. Splitting it is a
design decision and this build did not make it.

**`standard` therefore denies `ToolSearch` and `Skill`.** That follows from the
paragraph above and it is a real capability loss on ordinary coding runs, in the
direction 7.1 chose on purpose: an allowlist fails toward a run that cannot do its job,
which someone notices. Ticket 02's first gated run is where it gets noticed, and the
fix, if the records ask for one, is a policy file edit rather than code.

**The four protected configs that are not files yet are denied by path anyway.** 7.5
denies writes to seven paths under every policy. Two exist: `policies/` and
`~/hydra/runs/`. The generated gate script is written per spawn. The other four are
named at `config/env-allowlist.json`, `config/export-allowlist.json`,
`config/preamble.md` and `config/mcp.md`, and **none of those files exists**. The
environment allowlist is still Rust code in `src/env.rs`, which 15.2 protects at
admission rather than at run time, so the config the table means has no file. Denying a
write to a path that holds nothing is the same denial it will be the day the file lands,
which is why the names went in now rather than being added later and forgotten.

**Hydra finds its own policy directory through `CARGO_MANIFEST_DIR`, so the binary
only works from its source tree.** `PolicySet::discover()` and the four `config/` paths
are compiled against the manifest directory, which is right for a binary run out of
`target/debug` and wrong for an installed one: a copied `hydra` would compile gates
against a `policies/` that is not where it thinks. Nothing has needed an installed
binary yet, and the answer is not obvious, since the policy directory is protected
config #1 and a run must not be able to point Hydra at a different one. Naming it here
rather than leaving it as a doc comment in `src/policy/mod.rs`.

**A path is compared after resolving the longest ancestor that exists.** Plain
`canonicalize` fails outright on a path whose file has not been created, and the gate
script is exactly that at compile time. The first version fell back to the unresolved
path, which left `/var/folders/...` on one side and `/private/var/folders/...` on the
other, so **the gate script was writable under `permissive`** until `tests/gate.rs`
caught it. Any protected path sitting under a symlinked directory had the same hole.
The gate script resolves the same way at fire time, with `realpath`, and both sides
lowercase because 15.2 matches case-insensitively.

**The floor refuses rather than evaluates, and the refusal is coarse.** 7.4 says a
pattern must refuse a subcommand arriving from a shell substitution. Which segment a
substitution feeds is not decidable without running it, so a command carrying `$(`,
`${` or a backtick **and** naming git, aws or Slack anywhere in it is refused whole.
`echo $(date) && git status` is refused. That is a false positive on purpose, and
`--allow-floor-escape` is the human's way past it.

**`git push` with no arguments is not checked against the protected branches.** The
floor cannot know the current branch from argv. It is the failure mode 7.4 names out
loud, written down here so nobody reads the passing tests as coverage.

## The three departures from the record contract

**`policy` and `policy_sha` are null on a recorded run.** This is the big one. A gate
compiles now, and no run is launched with one, so nothing is in force and null is the
true answer rather than a missing value. Writing a `policy` on a record whose child
never saw a hook would be the worse kind of wrong: a field that claims a bound the run
did not have. Contract 5.6 already gives a null policy a defined reach, for the
interactive run: `permission_decision` has no producer, because 7.7 derives it by
re-evaluating the run's own policy, and the gate-leak check goes with it. Every one
of those consequences holds here for the same reason. What differs from 5.6 is that
`launch_mode` reads `recorded`, so a reader cannot tell these records apart by launch
mode. **`hydra_version` is the field that tells them apart**, which is the job
contract 15 already gives `provider_version` for provider drift. Any `0.1.x` record
was written before a gate was passed to a provider.

**`skills` and `mcp` are absent, not null.** Nothing is injected, so which sha the run
pinned is not a question with a missing answer. Contract 5.5 set the rule and 5.6
applied it; this is the third place it applies.

**No preamble rides the task string.** Harness spec 8 has the preamble assert that a
run's actions are bounded by a policy it cannot widen. There is no policy, so the
sentence would be false, and a preamble that overclaims is the Hermes failure with
the roles swapped. `task` is therefore exactly what the caller passed.

Everything else in contract sections 2 to 6 is met, and `tests/record_contract.rs`
checks it rather than asserting it. That test also walks every record under
`~/hydra/runs/` on this machine and fails if any of them drifted.

## What the command line does not carry

**`--permission-mode bypassPermissions` is not passed.** Harness spec 6.7 justifies
it in one sentence: a hook deny beats it, so the gate does the refusing. With no
gate compiled, that sentence has no subject, and the flag would leave a detached
child with nothing between it and the filesystem. The provider's own default applies
until the gate lands.

Measured on 2026-09-05, which is worth more than the argument. A run asked to write
`probe.txt` inside its own worktree got `system/permission_denied`, wrote no file,
and closed with `subtype: success`, `is_error: false`, `terminal_reason: completed`
and exit code 0. The refusal works and the terminator lies about it, which is exactly
what contract section 8 says to expect of all three providers.

**`< /dev/null` is dropped**, and harness spec 6.7 now says why.

## What this slice measured

Three findings, all on `claude` 2.1.261 over a pty Hydra owned, 2026-09-05. **All
three are in the documents now**, so what follows is the short version and the
document is the source: harness spec 6.7 for the first two, contract 3 for where a
non-JSON line goes, contract 7.1 and 7.7 for the third.

**`< /dev/null` was a finding about a pipe.** Harness spec 6.7 adds the redirect so
`claude` does not wait three seconds for stdin and write `no stdin data received in
3s, proceeding without it` into the record. On a pty there is no wait and no warning.
Measured off the record's own timestamps: 0.83 s from spawn to the first frame, which
includes the provider's startup and the user's own `UserPromptSubmit` hook, and 2.37 s
for the whole run. There is no three-second stall anywhere in it. Hydra
owns a pty because it has to be the parent to tee anyway, so the redirect is not
reachable from here and is not needed. The spec sentence is true for the shape it was
measured on and does not carry to the shape Hydra launches.

**`claude` writes a bare `ESC[?25h` after its last frame, with no trailing newline.**
Show-cursor, on the way out, because stdout is a tty. Any tee that assumes every line
is a frame will either drop it or fail to parse it. This slice stores it as a JSON
string in `raw`, because never-discard has no exception for noise, and a
string-valued `raw` is how a reader tells a line that was not JSON from a frame that
was.

**`system/permission_denied` is a frame the contract's producer table does not
name.** Contract 7.2 lists no `claude` frame for a provider-side refusal, because
every recorded run was specified to launch with the permissive flag plus the gate, so
the decision is made before the process starts. Take the permissive flag off and the
provider makes decisions again, and it announces them: one `system/permission_denied`
carrying `tool_name`, `tool_use_id` and a message, plus an entry in
`result.permission_denials[]`, which contract 8.5 already names as corroboration. On
these records nothing derives from either, because 7.7 needs a policy. It gets no row
in the producer table for that reason and a paragraph in 7.7 instead, so the next
reader meets it in the document rather than in a record.

## Choices this slice made that no ticket settled

**Worktrees live at `~/hydra/worktrees/<run-id>/`, on branch `hydra/<run-id>`.** No
closed ticket names the location, which is the same gap harness spec 19.1 flags for
the filenames inside the spawn directory. A Hydra-owned path keeps D9 true when the
repo being worked on lives somewhere else. Confirm it rather than inherit it.

**`run_start` is written after the spawn, not before it.** Contract section 5 says the
line is written before the child process starts and 5.2 puts the child's pid on it.
Both cannot hold literally. What run_end's derivation leans on is that the pid is in
the file, and a pty buffers everything the child writes until the tee reads it, so
spawning first loses nothing and `run_start` is still line 0.

**`process_start_time` is RFC 3339 with microseconds**, read from `kinfo_proc` at
offset 0 through `sysctl`, because `libc` carries no `kinfo_proc` for Apple targets.
A test compares it against `ps -o lstart=` on every run of the suite, so the day that
offset stops being `p_starttime` the suite says so instead of the liveness check
quietly matching everything.

**A `HOME` that disagrees with the passwd entry is dropped rather than forwarded.**
Harness spec 6.2 says an absent `HOME` is safer than a wrong one, because the keychain
search list falls back to `getpwuid`. This follows that reasoning to its conclusion,
and the drop is visible after the fact because `env_allowlist` carries the names
actually passed.

**A `PATH` without `/usr/bin` refuses the launch** before a run id exists, with an
error naming the silent logout it would otherwise cause. It writes no run record: the
contract's four `refusal` values are all Hydra-side policy checks this slice does not
run, and inventing a fifth is a contract change.

**`hydra run` returns fewer fields than contract 17 specifies.** `outcome`,
`outcome_source`, `usage` and the final assistant text are all normalizer output, and
the diff stat would report the seed plus the child's work rather than the child's. A
null `usage` reads as a measurement of zero, so the JSON carries a
`not_yet_derived` list naming the five instead. That field goes away when they land.

## Not built

Out of scope for this slice, and each one has a document that already says what it is:

- The gate, on a run. Harness spec 6.6. The policy set, the capability table and the
  compiler are built (ticket 01) and nothing passes their output to a provider: no
  `--settings`, no hook config, no `--permission-mode bypassPermissions`, so the three
  consequences above still follow. Ticket 02 is the wiring, and it is the first thing
  that will find out whether a compiled deny holds on a real run.
- Skill injection and the store. 6.4.
- MCP injection and the Cursor pre-flight. 6.5.
- The spawn directory. It holds a gate script, an MCP config and Cursor's data root,
  and this slice writes none of the three, so it creates no directory.
- Delegation: `--parent`, depth, monotonic narrowing, the MCP delegate face. 7.3, 9.
- Detached spawning and the timeout. Section 9 spawns every child detached and makes
  blocking a view over it. Here the run is foreground and `hydra run` dies with it.
  Ctrl-C leaves an orphan child, a record with no `run_end`, and an abandoned
  worktree, which is exactly ticket 14's abandoned-run class arriving early.
- The normalizer, the ten event types, and outcome derivation. Contract 7 and 8.
- Interactive runs and harvest. 5.6, 12.
- Every command but `run`. Notably `hydra reap`, so worktrees accumulate. Remove one
  by hand with `git worktree remove ~/hydra/worktrees/<run-id>` and
  `git branch -D hydra/<run-id>`, and read harness spec 10 first, because the specced
  reap commits uncommitted work to the child's branch before removing anything.
- Ouroboros, both rungs.

## Rough edges worth knowing

- **A run with no `run_end` is not corrupt.** Contract section 6 says so. This slice
  produces one every time Hydra is killed mid-run, and nothing yet completes it.
- **A frame that is not valid UTF-8 is stored lossily** and the count lands in
  `run_end.error`. It has not happened; `claude` writes JSON through Node.
- **The tee has no line-length cap.** A provider that streams one enormous frame
  would grow the buffer without limit. Frames of 200 KB are tested and fine.
