# What is built

`docs/` is the design, finished and checked. This file is the gap between it and the
code, kept current by whoever narrows the gap.

Read it before trusting a run record written by this binary. **The first slice is
not spec-complete, and three of its records' fields say so.**

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

## The three departures from the record contract

**`policy` and `policy_sha` are null on a recorded run.** This is the big one. The
gate does not exist, so nothing is in force, and null is the true answer rather than
a missing value. Contract 5.6 already gives a null policy a defined reach, for the
interactive run: `permission_decision` has no producer, because 7.7 derives it by
re-evaluating the run's own policy, and the gate-leak check goes with it. Every one
of those consequences holds here for the same reason. What differs from 5.6 is that
`launch_mode` reads `recorded`, so a reader cannot tell these records apart by launch
mode. **`hydra_version` is the field that tells them apart**, which is the job
contract 15 already gives `provider_version` for provider drift. Any `0.1.x` record
was written before the gate existed.

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

- The gate. Harness spec 6.6 and 7. Nothing is compiled, nothing is passed, and the
  three consequences above follow.
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
