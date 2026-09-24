# What is built

`docs/` is the design, finished and checked. This file is the gap between it and the
code, kept current by whoever narrows the gap.

Read it before trusting a run record written by this binary. **A record whose `policy`
is null was written before the gate reached a provider.** Ticket 02 wired it, and every
record written after that carries the policy that was in force and the sha of the
directory it came from.

**`hydra_version` does not separate the two.** It used to be the field this file pointed
at, and three gated records were written under `0.1.0` during ticket 02's own
measurement, before the version moved to `0.2.0`. A null `policy` is the test that holds.

## What the gate does about `shell`, and what it still does not

Harness spec 7.5 delivers a set of paths denied outright, plus what a spawn owns carved
back out of that denial: "a hook sees the path and refuses the write before it reaches
disk". The gate delivers that for tools that carry a path in their arguments. `Bash`
carries none, `shell` is unscoped in every policy that grants it, and no argv match can
decide what a shell command will write.

**Ticket 02's code review found every path denied outright then reachable by redirect**,
the run's own gate script included, which is 7.5's induction base case. That one
composed into a full bypass: an emptied script exits 0 printing nothing, and 6.6
measured `claude` failing open on a response in a shape it does not recognise, so two
`Bash` calls under `standard` ended the gate for the rest of the run. `--permission-mode
bypassPermissions` had removed the layer previously refusing those calls, so the flag
and the hole arrived in the same change.

Two mechanisms now stand where none did. They are worth telling apart, because one is a
floor and one is not.

**A shell command that names a denied path it does not own is refused whole.** Every
token of every segment is expanded and resolved before the comparison: `~` and the variables the
child's own environment carries, the tail of a `--flag=/path` option, and the tail of a
`D=/path` assignment, which is where a variable the command sets itself is written down.
Measured on run `01a07340-67a3-7c41-8c69-8d1e28bb58a7`, `standard`, on a real `claude`:
a `Bash` append to `~/hydra/runs/` was refused with `hydra/protected-path`, the reason
reached the model, no file was created, and a second `Bash` call writing inside the
worktree in the same run was allowed and landed.

**This one is a floor and it has 7.4's failure mode.** A path the command never writes
down still walks past. `P=<parent>; echo x > $P/runs/r.jsonl` is the shape: the variable
holds the parent, no token is a protected path, and no expansion the gate can do
produces one. Catching it means evaluating the command, which 7.4 rules out.
`tests/gate.rs` pins it so that closing it breaks a test on purpose.

**The base case does not rest on that floor.** The hook config carries a sha256 of the
gate script, computed from the bytes that reached the disk, and the command checks it
before running the script. All three providers snapshot the hook config at process
start (6.6) while 7.5 has the script read fresh on every invocation, so the sha is fixed
before the child can run anything and a rewritten or deleted script fails **closed**,
answering `hydra/gate-tampered` in the provider's own dialect. That holds whatever
spelling reached the script, which is what makes it a mechanism rather than a floor.
The script still lives where 7.5 puts it; only the pointer got stricter.

Measured on run `01a07357-58a5-79e2-a25a-3df578a750b4`: `claude` runs the 717 character
hook command and the gate answers through it. `tests/spawn.rs` empties the script and
then deletes it, and asserts the refusal both times.

**The refusal is coarse and reads are refused too.** Which token is a redirect target is
not decidable without running the command, and neither is which is an argument to a
writing binary, so `cat ~/hydra/runs/r.jsonl` is refused under `curate` as well. That is
a false positive in the direction 7.4's floor already takes. The precise access is the
file tools, which carry a path: `curate` scopes `file_read` to the corpus and reads it
that way.

**What is left for the map.** Closing the residual needs a mechanism that does not read
argv, and each option trades something the design has argued about: denying `shell` its
unscoped grant rewrites 7.2's `standard`, and running the child under a second uid or a
sandbox is nowhere in the spec. The base case no longer waits on that answer, so what
remains at risk is a write to the policy directory or the corpus through a path the
command assembles.

Provider versions this build has been measured against. The table is the 2026-09-06 reading
the provider sections cite, plus `codex`'s from when it was routed; `provider::MEASURED_VERSIONS`, `README.md` and `hydra doctor` name
the 2026-09-22 reading, `claude` 2.1.268, `cursor-agent` 2026.09.18-9a7762b, `agy` 1.2.8 and
`codex` 0.154.0, on which the founding measurement was repeated that day: one-step `judge` runs
01a0c790-8a9d on `claude` (`subtype: success`, `is_error: false`, exit 0) and 01a0c790-9157 on
`agy` (`status: SUCCESS`, exit 0), both `gated`, beside 01a0c4c5-650b on `cursor-agent` and
01a0c62b-d005 on `codex` from the day before. `agy` self-updated from 1.2.7 to 1.2.8 between
`hydra doctor` and the run, an hour apart. Three of the four self-update, so a version string
here is a date, and `tests/drift.rs` is what re-measures behaviour:

| Provider | Version | Read |
|---|---|---|
| `claude` | 2.1.263 | 2026-09-06. The specs were written against 2.1.257. Every flag in 6.7 still runs, and one claim about a flag was half wrong: `--restricted` drops what settings define and keeps what `--mcp-config` passes, which the section on `judge` below has. `tests/drift.rs` re-measures it per suite run |
| `cursor-agent` | 2026.09.02-c22c1a3 | 2026-09-06, unchanged since 2026-09-02. Routed by ticket 09 on this build; the section on it below says what was measured and what was read off the bundle |
| `agy` | 1.1.26, then 1.1.27 | 2026-09-06. Routed by ticket 10, and **it self-updated in the middle of that ticket**: the probes below were measured on 1.1.26 and the recorded run an hour later reported 1.1.27. Every measured claim held across the move, and the section on it says which came from which build |
| `codex` | 0.154.0 | 2026-09-20, not routed on 2026-09-06. Built by ticket 48 from map ticket 58's reading of this version, measured on a logged-in binary on 2026-09-21, and still 0.154.0 on 2026-09-22. The section on it below says what was measured. Row added on 2026-09-24, the third hiring panel's H12 |


**Built for map ticket 40, closed by build ticket 14 on 2026-09-06.** The deny list now has the
two kinds of entry 7.5 describes, denied outright and what a spawn owns carved back out, decided by
one function, `protected_root` in `gate.py`, so the `Write` check and the shell check cannot drift
apart. `denied_write_paths` and `own_write_paths` in `src/policy/mod.rs` are the two lists, and
`Spawn` carries `layout: &Layout`, `skill_checkout: Option<&Path>` and `parent_git_dir:
Option<&Path>` so the deny list can be built from what the run actually has. `~/hydra/spawn/` is
one of the denied parents now rather than just this run's own script inside it, which closes the
hole a `permissive` run had into a concurrent run's live gate script. A run outside a repo has
`spawn/<run-id>/no-worktree` as its `worktree` scope, and `own_write_paths` carves a worktree out
only when it is not under `~/hydra/spawn/`, so that run writes nowhere, which is what `src/run.rs`
always said it did; `tests/gate.rs::a_run_with_no_checkout_writes_nowhere_at_all` pins it.

**Built for map ticket 65, closed by build ticket 74 on 2026-09-22.** `hydra_home()` joins
`denied_write_paths` whole, with the run's own worktree and a `curate` pass's checkout carved
back out as before. On this machine the checkout is the data root, so the deny is `~/hydra` and
everything a run legitimately writes is already an own path; on a machine where the two differ,
both are denied. `denied_spellings` leaves `hydra_home()` to the token pass when an own path lives
under it, so the substring floor does not refuse the run's own worktree directory. The seatbelt
profile picks the deny up through `write_denies` before the own allows. `tests/gate.rs::
the_checkout_is_denied_and_the_own_worktree_still_writes` asserts a `Write` to
`<hydra_home>/target/release/hydra` and to `<hydra_home>/src/policy/gate.py` is `protected-path`
under `permissive`, and that a write inside the fixture's own worktree still allows;
`tests/spawn.rs::the_seatbelt_refuses_a_checkout_write_and_allows_the_worktree` drives the real
profile: `echo x > <hydra_home>/target/probe` is refused and the worktree write succeeds.

**Built for map ticket 76, closed by build ticket 110 on 2026-09-24.** `layout.puppets()` joins
`denied_write_paths` under every policy. Master of Puppets keeps one append-only JSONL per goal
there, and a run that rewrites its own assignment or approval walks around the mode. Reads stay
as they were: the log is data a run may read if its scope reaches it, and under `standard` it
does not. The seatbelt profile picks the deny up through `write_denies` like every other entry.
`tests/spawn.rs::the_goal_log_is_denied_and_the_seatbelt_profile_names_puppets` compiles a
`standard` gate for a scratch layout, feeds a `Write` to `<layout>/puppets/x.jsonl` and reads
`protected-path` back, and asserts the profile the same spawn writes names the directory.

Ticket 40's hand-measured matrix is two tests in `tests/gate.rs` now,
`a_write_the_matrix_allowed_on_2026_09_06_is_protected_now` and
`a_shell_command_the_matrix_allowed_is_refused_now_unless_it_is_the_runs_own_scratch`: 13 write probes across all five policies and a shell table for
`standard` and `permissive`. Every row that read `ALLOW` under `standard` or `permissive` on
2026-09-06 reads `protected-path` now: `rm -rf` on `~/hydra/pruned/`, on a sibling's worktree, on
`~/hydra/skills/.git`, on the snapshots, on the parent repo's `.git`, and a `Write` into a
sibling's gate script under `permissive`. `git gc`, `git prune` and every `git worktree` form but
`list` hit the floor now, the three forms 7.4 names.

**Five rows stay allowed on purpose, decision 11 of ticket 40:** `git checkout -- .`, `git stash
drop`, `git clean -f`, `git worktree list`, and `rm -rf` on the run's own worktree. The
own-worktree `Write` under `standard` and `permissive` asserts `allow`, and the own-checkout
`Write` under `curate` asserts `allow`; those two are the carve-out 7.5 names.

**One thing the matrix showed that the hand measurement had missed.** Under `judge`, `read-only`
and `curate`, a shell command naming a denied path answers `protected-path`, not `capability`: the
path denial runs before the policy is consulted, so a policy with no shell grant at all still gets
the path-shaped answer rather than the capability-shaped one. The pre-ticket-40 matrix had already
shown this for `~/hydra/runs/`; the new one shows it everywhere.

**Built for build ticket 44 on 2026-09-12: a run cannot rewrite its own hook config.** The
worktree is the own carve-out, and on `cursor-agent` and `agy` the hook config sits inside it
at `.cursor/hooks.json` and `.agents/hooks.json`, so until this ticket a `standard` run could
`Write` over the file that points at its gate and the gate answered `allow`. Whether either
provider reloads hooks mid-session is still unmeasured; the file is committed by `hydra reap`
either way and inherited by the next run seeded from that branch. `protected_worktree_paths`
in `src/policy/mod.rs` is the list, seven entries built from the provider modules: the two hook
configs, the two MCP configs beside them, and the three project-scope files `claude` reads from
cwd (`.claude/settings.json`, `.claude/settings.local.json`, `.mcp.json`). It rides into the
gate as `protected_worktree_paths`, and `protected_root` in `gate.py` checks it before the own
carve-out, so both the `Write` check and the shell check answer `hydra/protected-path` on it
under every policy and on every provider's gate. The three skill roots are a second list,
`write_denied_worktree_paths`, read by the `file_write` check alone: the first version of this
ticket put them on the whole-command list, the corpus walk caught `ls -la .claude/skills/` on
run `01a09405-f451` reading as a leak, and a shell command naming a skill root is how two of the
three providers read a skill, so the roots are denied to writes and to nothing else. The
`.gitignore` files Hydra drops beside those configs stay writable, and the tests pin that.
`tests/gate.rs` has four tests for it: every file on every dialect under all five policies,
the ordinary write still allowed, the compiled list resolved inside the worktree, and the
shell forms. One consequence to know about: `hydra show` re-derives every recorded decision
through the same compiler, so a record in which a run wrote one of these files and was allowed
read as a gate leak the day this landed. `policy_sha` did not move, because the list is code
rather than a file under `policies/`. That is what build ticket 49 answered with `gate_sha`,
and a record written before the field now reads as a limit rather than a leak.

**The demo scripts read the checkout they live in, since build ticket 52.** `demo.sh` derives it
from `BASH_SOURCE` instead of hardcoding `$HOME/hydra`, `verdict.py` takes the data root from
`HYDRA_ROOT`, and `record.sh` already cd'd relative to itself. The ticket also claimed the
checked-in cast was not the one the GIF came from, because its header said 80x24 while `record.sh`
passed `--cols 118 --rows 30`. That was the script lying, not the cast: asciinema 3 dropped `--cols`
and `--rows` and ignores both without a word, measured on 3.2.1 with `asciinema rec --cols 118
--command "tput cols"`, which records a pty answering 80. The flag is `--window-size 118x30`.

**The shipped configuration moves with `HYDRA_HOME` since build ticket 50.** `paths::hydra_home`
answers where `policies/` and `config/` live, the variable first and the compile-time
`CARGO_MANIFEST_DIR` second, and five call sites go through it. `ensure_shipped_config` refuses
before any policy file is opened, naming the directory and the variable, and `hydra status` prints
`policies <path>` under the root line. `strings target/release/hydra | grep -c <checkout>` went from
3 to 1, the one left being the default inside the resolver. The binary is still not relocatable
without the variable, and `README.md` says so.

**The three readers do not apply the same bar, and the record says so since build ticket 55.**
Contract 7.7 carries the table: `claude`'s init frame lists every tool it offered, `cursor-agent`'s
carries none, and `agy` raises a leak only where the tool reported success. `note_finding_bar` puts
one `Limit` on every `cursor-agent` and `agy` record naming which bar it held, and none on `claude`,
which is the reference. The same ticket found `claude.rs`'s harvest path testing
`starts_with("hydra/")` where its stream path went through `hydra_voice`: a tool error reading
`hydra/runs/x.jsonl: permission denied`, a guard quoting a path, was recorded as a gate that denied
the call. Both paths validate the code now. `unhooked` and `capped-argument` stay off `GATE_CODES`
on purpose: the reader writes those into a decision's reason and the gate writes the others into a
refusal, and one list for both would make a reader's sentence readable as the gate's.

**Two shell rows and one MCP door, closed by build ticket 59.** `evaluate` in `gate.py` reached the
path denials of 7.5 through two doors, a `file_write` carrying a path and a `shell` carrying a
command string, and two things went through the gap.

`WriteShellStdin` on `cursor-agent` and `send_command_input` on `agy` write to the stdin of a
command that is already running. The command was judged when it started; what is typed into it
afterwards is not, and the row carries no `command_field`, so `raw_command` was empty and both the
floor and the path check were skipped. They carry `Capability::OpaqueShell` now, which no policy
grants, so the gate answers `hydra/capability` by name rather than letting a `permissive` run feed a
protected path to a shell it had already opened. **The cost is real**: a run cannot answer an
interactive prompt on those two providers. It gets the facility back the day the gate can see what
is being typed. `command_status` stays `shell`, because reading a running command's output is not a
command nobody saw, and the pair no longer shares a capability for that reason.

`mcp_call` met neither door. An MCP server's arguments are its own shape, so the table names no path
field and `target_path` read nothing, and under `permissive` an MCP tool writing the corpus was
allowed. `protected_path_named_in` walks every string in the call's arguments, at any depth, and
searches each for a denied spelling, which is the raw pass of build ticket 56 applied to a second
door. Measured: disable it and the same call comes back `hydra/allow: policy permissive allows
mcp_call`. `config/mcp.md` admits no writing server today, so nothing had fired.

**The corpus walks honour `HYDRA_ROOT` since build ticket 51.** `corpus_runs_dir` in
`tests/common/mod.rs` is the one place that answers where they read, and `tests/record_contract.rs`
goes through it rather than building the path from `real_home()` itself. The CI workflow had been
setting the variable and claiming the walks read it, which was false; CI passed because the runner's
home holds no corpus. Four tests measure a particular record rather than checking every one, and
those return early on an empty corpus through `corpus_or_skip`. Measured: against an empty
`HYDRA_ROOT`, `normalize`, `outcome` and `record_contract` pass 139 of 139 in under a second;
against the real 213 records they take six minutes. Since build ticket 84 the CI job ends with a
step that runs the contract walk alone and writes its "checked N record(s)" line to the job
summary, so a green run on the runner's empty root says `checked 0` where a reader can see it.
**Since build ticket 101 CI walks a fixture corpus too** (panel findings F32 and H14).
`tests/fixtures/records/` holds ten synthetic records in five directories named by the binary
that wrote each shape: `claude@2.1.261`, `cursor-agent@2026.09.02-c22c1a3`, `agy@1.1.26`,
`codex@0.154.0` and `harness`. The ignored test `write_the_fixture_corpus` in
`tests/fixture_corpus.rs` writes them from the builders in `tests/common`, so no byte comes from
`~/hydra/runs`, and three tests there guard the directory label, the coverage of every routed
provider and the harness, and the absence of secrets, home paths and live run ids. Each live walk
in `record_contract.rs`, `normalize.rs` and `outcome.rs` has a fixture twin with the same
assertions, and the CI job summary prints both counts, zero live records and ten fixtures. Two
limits. The fixture leak walk can catch a reader that invents a leak and cannot catch a real one,
because a frozen fixture names no gate this build compiles, so a disagreement reads as
`StalePolicy`; the synthetic tests in `tests/normalize.rs` hold the leak rule. And no fixture
has the 2026-09-22 versions yet; a shape change on one of those goes in a new directory.

**A run cannot relaunch the binary and name its own policy since build ticket 57, and the stop is a
marker.** Harness spec 7.3's rule was enforced nowhere: `Lineage::of` returns depth 0 with no bound
when `--parent` is absent, and nothing marked a process as being inside a run. `src/containment.rs`
is the one place that decides it. `hydra run` sets `HYDRA_RUN_ID` in every gated child's
environment, reads it back out of its own environment at launch, and looks it up in the corpus: a
marker naming a live run puts the caller inside it, and a marker naming an ended run or none at all
is `Stale` and refused rather than read as permission, which is also the shape a relocated
`HYDRA_ROOT` takes. A caller inside a run that names no parent, or names another run, writes a
`containment` refusal and launches nothing. It derives to `gated`, the same arm
`policy_narrowing` takes, because it is one rule caught at two moments.

Measured while building it: disable the check and the same launch completes. `hydra run --provider
claude --policy permissive` with no `--parent`, from a process carrying the marker, ran a real
`claude` under `permissive` and came back `completed`. The attack was not theoretical.

**Until build ticket 97 the marker reached no recorded child.** The launcher set it on its copy
of the child's environment, which is what `env_allowlist` reports, but the watcher rebuilds the
child's environment from the allowlist and the marker was not on the launch spec. Ticket 57's
measurement and `tests/delegation.rs` planted it by hand, so nothing caught it. Records written
before build 97 name `HYDRA_RUN_ID` in `env_allowlist` for a child that never had it.
`src/run.rs::compose_child_env` now puts every name Hydra sets on the spec, and
`every_name_hydra_sets_travels_to_the_detached_half` fails when one is left off. Measured after
the merge: `claude` on Haiku under `standard`, run 01a0d1fa-c301, ran `echo $HYDRA_RUN_ID` and
printed its own run id.

**What it does not close.** A child that unsets the variable walks through, because a variable is a
thing a process removes from its own children's environment. The check closes the absent flag, which
was the whole of the attack as the review measured it, and `SECURITY.md` says the residual out loud
rather than letting the check read as containment. Map ticket 62 is where the uid alternative goes.
The ungated interactive arm is deliberately unmarked: it compiles no gate, claims no containment,
and marking it would refuse the `hydra run` a human types in their own session.

**The validator checks three classes of field since build ticket 54.** `RUN_START_FIELDS` is what
every record has carried since the first one and a missing name there is an error. The groups of
contract 5.3 and 5.4 are absent when the question does not apply, so a null is the error.
`LATER_RUN_START_FIELDS` is the third: `floor_lifted`, `gate_sha`, `scopes`, `env_allowlist`,
`parent_policy` and `resumed_from`, each checked for its shape when present and never for presence,
because a presence check on a field added last week fails every record from the week before and the
walk has to keep reading those. Measured 2026-09-15 over 213 records: `gate_sha` absent on 205,
`scopes` on 36, `floor_lifted` on 27. `floor_lifted` carries a flag saying null is an answer, which
is what contract 5.5 has a harness run write.

The four leaves inside `auth` are checked too. The group was tested for non-null and its contents
never, which is how `allow_billed` stayed a field nothing writes and nothing noticed. All 213 records
pass, and the walk costs 0.44 seconds because it validates structure rather than deriving anything.

**Both sides fold case the same way since build ticket 60.** `src/policy/mod.rs` lowercased with
Rust's `to_lowercase` and `gate.py` compared with Python's `casefold`, which disagree on `ß` and on
`ﬁ`. A layout root holding one of them compiled a deny entry the gate could never match, so every
denial and every scope stopped applying, with no message. `fold_ascii` on each side now lowercases
ASCII letters and leaves everything else alone, which is what harness spec 15.2 asks for: a macOS
filesystem's case-insensitivity, not Unicode equivalence. `tests/gate.rs` builds the same gate under
`Straße` and under an ASCII control and asserts a `permissive` write into the corpus is refused on
both; reverting the Python fold alone brings back `hydra/allow` on the first and leaves the second
refused.

**The shell check reads the command twice since build ticket 56.** The token pass resolves each
`shlex` token, which is what catches a relative path and a symlink, and it sees whole tokens
only. So a denied path spelled in full inside a larger one walked through: `sh -c 'cat
<corpus>/x.jsonl'` is one token, and so is `python3 -c "open('<corpus>/x','w')"`. Verified
against the real compiled gate of run `01a0a07b-e8a6`, where the bare `cat` was refused and the
`sh -c` was allowed. A raw pass now runs first and searches the case-folded command for each
denied path as a substring. `denied_spellings` in `src/policy/mod.rs` emits two forms per path,
resolved and literal, because on macOS `/var/folders/...` and the `/private/var/folders/...` it
resolves to are both things a command can write down and a substring search resolves nothing.

The residual is written down in `gate.py` beside the code. A denied path with one of the
spawn's own paths under it cannot go on the raw list: `~/hydra/worktrees` is denied while the
run's own worktree sits inside it, and a substring search cannot say "this one and not that
one". So a sibling run's worktree spelled inside a larger token still reaches the token pass
and still walks through. The corpus, the store, the snapshots, the spawn directory and the
per-run config files carry no own path and are covered.

## The commands

```
hydra run --provider <claude|cursor-agent|agy|codex> --task "..." [--model <id>] [--policy <name>]
          [--parent <run-id>] [--timeout <dur>] [--allow-floor-escape] [--interactive]
hydra status [<run-id>]
hydra logs <run-id> [--follow]
hydra reap [<run-id>] [--delete-branch]
hydra prune [<run-id>...] [--delete] [--dry-run]
hydra curate [--model <id>] [--timeout <dur>] [--dry-run]
hydra show <run-id> [--events]
hydra usage recount [<run-id>]
hydra export --out <file> | --dry-run
hydra resume <run-id> [--task "..."] [--timeout <dur>]
hydra evolve [--task "..."] [--model <id>] [--timeout <dur>] [--dry-run]
```

A launch builds the child's environment from the allowlist (and since build ticket 88 the pty
clears the inherited environment before applying it; before, recorded runs got the launching
shell's whole environment, `SECURITY.md` says what that exposed; since build ticket 97 the
watcher starts from the child's environment too), reads the provider's
version, loads the policy set, the preamble and the canonical MCP document, refuses if
the concurrency cap is already met, mints a UUIDv7 run id, creates a git worktree and
seeds it with the parent's uncommitted work, writes the servers the run's policy admits
into `~/hydra/spawn/<run-id>/mcp.json`, links a snapshot of the skill store into the
checkout, asks the provider in one free probe what it resolved of both, compiles the
run's gate into the same spawn directory, composes `claude`'s argv and the whole of line
0, and hands all of it to a detached half that spawns the provider on a pty, writes
`run_start`, tees every frame verbatim into `~/hydra/runs/<run-id>.jsonl` and writes
`run_end` when the child exits. The foreground tails that file, and on a run that ends
inside its budget it reads the file back through the normalizer and prints contract 17's
JSON on stdout.

Section 9 gives `hydra run` eight optional flags and five are here: `--model`,
`--policy`, `--parent`, `--timeout` and `--allow-floor-escape`. The three that are not
each belong to a ticket that is not built: `--files` is on the delegate face and not on
the command line, `--shared-tree` answers a worktree question nothing has asked yet, and
`--allow-billed` belongs to the raw-model plane.

Harness spec 17 lists eleven commands and all eleven are built: `show` since build ticket
18, `usage recount` since build ticket 31, `export` since build ticket 34, `resume` since
build ticket 32 and `evolve` since build ticket 36, the last three with sections of their own
below. That last one walks the corpus, or one record, and
prints how many runs carry each `usage_source`, then says there is nothing to recount, which
spec 17 already promised for v1: no provider produces `estimated`, and the command exists so
the tool is here the day one does. On this machine on 2026-09-08 it said `provider: 58`,
`unavailable: 5`, `estimated: 0`. `hydra watch` is a twelfth and
is not one of them: it is the detached half of a launch, `hydra run` is the only thing
that types it, and `clap` hides it from the help for that reason. `hydra delegate-face`
is a thirteenth, hidden the same way: it is the MCP delegate face of section 9, a run's
own provider starts it, and the section on delegation below says what it hands a model.

`hydra reap` commits a checkout to the child's branch, then removes the checkout and
the run's spawn directory. `hydra prune` moves run files to `~/hydra/pruned/`. Neither
appends to or removes a run record the other owns, and the section below says what each
guarantees and how.

`hydra curate` is rung one: it reads the corpus, launches one recorded run
under the `curate` policy with what it found, and commits what that run proposed onto
a branch in the skill store that a human merges or does not. `--dry-run` reads the
corpus and launches nothing, which is the only half that works before ticket 07
creates the store.

`cursor-agent` is routed as of ticket 09 and `agy` as of ticket 10, and each has a
section below that is the one to read before trusting a record it wrote. All three of the
design's routable providers are now routed; `hermes` is the fourth that ticket 22
unrouted, and its name still comes back as an unknown provider.

## Cleanup: `hydra reap` and `hydra prune`

Ticket 06. Harness spec 10 gives both to a human to type and Rungs to neither, and
harness spec 7.5 already ruled out enforcing that with the gate: the binary is reached
through `shell`, so any rule against it is an argv match with the pattern floor's failure
mode. What holds it here is that nothing in `src/` calls `reap::reap` or `prune::prune`
except `main`.

**Rung one ran against the real corpus for the first time on 2026-09-10**, run
`01a08d66-76fd-7e10-b4f7-ae106d2dcfac`, sonnet under `curate`, 4.69M tokens, outcome
`completed/derived`, 73 runs in its brief. It proposed one skill, `ouroboros-curate`, 70 lines
on how the curator reads the corpus with no shell, no directory listing and nothing ToolSearch
can hand it, attributed to run `01a07513-baa3`, a `curate` pass that made 21 tool calls and
failed 15 before reading a record. That is the repeated-tool-errors class of spec 3.2, found
and attributed correctly. The proposal sits on `ouroboros/curate/01a08d66-...` in the store,
unmerged, as spec 3.7 requires. One defect surfaced at exit: the pass reported
`left behind: git branch -d ouroboros/curate/<id> failed: not fully merged`. A proposal
branch should never be a candidate for deletion, so that call is wrong rather than merely
noisy. Build ticket 43 fixed it on 2026-09-12: `propose()` reads the checkout's `HEAD`
before it commits, and after the checkout goes it deletes the branch only if `git rev-parse`
still gives that sha. A branch that moved is the proposal, and the pass no longer asks git
to delete it, so a real pass ends with nothing in `left`.

**`agy` cuts every string parameter it streams at 513 characters, build ticket 45.** Measured
2026-09-12 over every `agy` record on this machine: no parameter longer, sixty at exactly 513,
59 of the 71 over 300 characters cut inside a quote. The hook reads the whole argument and the
record keeps the cap, so the replay of contract 7.7 was judging commands the hook never saw:
run `01a0902c-8d66` had two `run_command` calls the floor "could not parse" that had run with
output, and the corpus walk called them leaks. A deny on a capped argument with a clean result
is now a `Limit` plus an `allow` decision whose reason starts `hydra/capped-argument`. The
earlier claim in this file that the provider truncates nothing was measured on results and
holds for them.

**Two parallel calls to one tool on `claude` are each joined to their own hook start, build
ticket 46.** On 2.1.268 a batch emits every `tool_use` first and every `hook_started` after,
so the first call's window held no start and the second's held both; the first read as
`Ungated` on run `01a095e6-626f`. A call with an empty window now borrows the earliest start
from a later same-tool window holding more than one, and only from one holding more than one,
because run `01a07f37-8a67` has `Read` calls refused on their input with no start at all and
they must not take the next real call's.

**`hydra show` and `hydra reap` take a run-id prefix, build ticket 39.** `src/resolver.rs`
resolves a short string against the run files: one match is that run, none or several is an
error naming which. A full uuid is never looked up and passes through whether or not a record
exists, because reaping a run whose record was pruned predates the ticket. `is_uuid` checks
hyphen positions and hex digits rather than counting to 36, since counting admits 36 characters
of anything, which is the hole a rung-two proposer walked through on pass `01a084d3-bdd3`.

**One contract moved with it.** `hydra reap` on a short string that matches no record now
errors instead of reporting nothing to do, because a typo that reports success is worse than a
typo that fails. An absent run is still benign and the test that pins it now says so with a
well-formed uuid. Rung two proposed this feature twice and both branches were blocked; map
ticket 57 says why the second block was wrong, and the code was typed rather than launched
because the seven-day Claude window stood at 0.83 that morning.

**Reap also sweeps `~/hydra/skills-snapshots/`, added by ticket 07.** It is harness spec
5's second row and it has a lifetime of its own: keyed by sha, shared between the runs
that pinned it, and reaped when no live run references it. The sweep runs whether or not
a run id was named, because reaping one run does not release a snapshot another still
holds and reaping none can release several. The section on the skill store below has the
refcount and the two guards on it.

**Reap also sweeps a leftover curate checkout, added by build ticket 14.** Map ticket 40's
decision 5: before this, reap took a run's worktree, its spawn directory and its snapshot
and left `~/hydra/skill-worktrees/<run-id>/` standing with whatever the curator had
written into it, because `hydra curate` only removes the checkout of a pass it watches to
the end. `commit_then_remove` in `src/reap.rs` generalizes what used to be
`remove_checkout` to serve both directories: it takes the branch Hydra made,
`hydra/<run-id>` for the worktree and `ouroboros/curate/<run-id>` for the checkout, and the
commit message, `hydra reap: what curate run <run-id> left uncommitted` for the checkout.
The checkout is swept between the worktree and the spawn directory, so a commit that fails
there stops the spawn directory removal the same way a failed worktree commit already did.
`git branch -d` follows, `-D` only under `--delete-branch`, so an empty pass's branch goes
and a real proposal's stays.

`Done` is `worktree`, `store_checkout` and `spawn_removed` now, one `Checkout` per directory,
and `report()` prints one block per checkout labelled `worktree` or `skill store checkout`. The
commit half is `commit_everything` in `src/worktree.rs`, shared with the curate pass's `propose`,
so the two places Hydra commits a model's work spell it once.

**Reap commits nothing of Hydra's own, since build ticket 30.** `provider::own_worktree_paths`
lists the six files Hydra writes into a checkout, built from the two providers' `gitignore`
functions rather than retyped: `.cursor/.gitignore`, `.cursor/hooks.json`, `.cursor/mcp.json`,
`.agents/.gitignore`, `.agents/hooks.json`, `.agents/mcp_config.json`. `commit_everything`
takes that list from reap and from the curate sweep, stages with `git add -A`, then takes
those paths back out of the index with `git reset`, commits, and removes whichever of them
are untracked so `git worktree remove` without `--force` still takes the checkout. A repo that
tracks its own `.cursor/.gitignore` keeps it, and an ignored file is left where it is, since
the removal does not count it as dirt. The emptiness check lists untracked files by name, so a
checkout holding only Hydra's files commits nothing and drops its empty branch.
`provider::claim` still warns when it finds the human's ignore file, now saying reap will not
commit the run's gate pointer or server list. **The first cut used `:(exclude)` pathspecs and
failed on the first real reap**: `git add` refuses the whole command when a named path is
ignored, and on a real worktree Hydra's own files always are. The model's test repo ignored
nothing, so its tests passed; the orchestrating session rewrote the staging after two reaps
failed. The sweep with no run id walks `skill-worktrees/` with the
same three filters `orphan_spawns` applies to the spawn directory, and a live run's
checkout is left alone by name and by sweep, the rule the worktree already had. Five tests
in `tests/cleanup.rs` cover it: a proposal reaches its branch before the checkout goes, an
empty checkout takes its empty branch with it, a live pass's checkout is never reaped, the
sweep finds a checkout with no worktree and no spawn directory beside it, and the store's
own working tree is never written to by a reap.

**Reap's commit-first rule is enforced by git rather than by `src/reap.rs`.** That is the
whole point of it. `git commit` leaves the checkout clean, and only a clean checkout is
one `git worktree remove` will take without `--force`, which reap never passes. So a
commit that did not happen stops the removal instead of preceding it, and the guarantee
is not a sentence in a doc comment. Harness spec 10 calls this the Hermes lesson turned
on our own tooling.

**The branch survives whatever it carries, decided by the same mechanism.**
`git branch -d` refuses a branch that is not fully merged, so the commit reap just made
is exactly what keeps the branch alive. A branch with nothing on it is litter and goes.
`--delete-branch` is `git branch -D`, and it is the same shape prune gives deletion:
recoverable by default, destructive only when a human typed the flag.
`docs/rungs-spec.md` section 4.11 already expects a leftover branch to be removed
with `git branch -D`; the flag is that sentence typed through the command.

**Only the branch Hydra named for the run is ever deleted.** A child that checked out one
of the human's own branches gets its work committed there and the branch left alone.

**A branch that survives is reported in git's own words.** `git branch -d` refuses for two
causes that want different answers: the branch holds commits nothing else has, or it is
checked out in another worktree. The second is refused by `-D` as well, so a report that
said "it holds commits" both times told the human the wrong reason and handed them a flag
that cannot help.

**A detached HEAD is refused rather than reaped.** A commit on one is reachable from
nothing once the checkout is gone, which is the single shape where committing first saves
nothing, so reap leaves the pair and says why.

**Both commands take the launch grace period as an argument, the way
`status::abandoned_worktrees` does, and `main` passes `run::LAUNCH_BUDGET`.** A checkout
is created and seeded before its run file exists, and the file itself exists before line 0
does, so for ten seconds a launch in progress has a checkout and an empty record and looks
exactly like a run whose record was pruned. `RunView::launched` plus the age is what tells
them apart; the section on `hydra status` below has the detail. Reaping there removes the
checkout out from under a child that is starting, and pruning there moves the record out
from under the writer that has it open, so the launcher waits ten seconds for a line 0
being written into `pruned/` and then reports a running run as one that wrote nothing.
`--delete` destroys it outright. Naming a run id skips the grace for reap, because the
grace is a launch window and a human who typed the id has answered that question; prune
keeps it even on a named run, because there the file is being written rather than merely
sitting there.

**Prune works on the path the scan found, not on a path rebuilt from a run id.**
`read_view` takes the run id off line 0 and only falls back to the filename, so a record
whose envelope disagrees with its name would otherwise have the sweep move a file that is
not there, and the named form find no view and skip the liveness check entirely.

**A move that cannot finish takes its copy back out.** The rename is what happens on one
filesystem; the copy-and-remove fallback is for a home directory spanning two mounts. A
copy that lands followed by a remove that fails would leave the record in both places, and
the destination check would then refuse every retry as an overwrite, turning one bad
minute into a permanent refusal.

**A sweep walks `spawn/` as well as `worktrees/`.** Harness spec 5 reaps the pair, and
the pair is missing on both sides in practice: the three oldest runs on this machine
predate the spawn directory, and a run outside a repo gets a spawn directory and no
checkout at all.

**Prune moves to `~/hydra/pruned/<run-id>.jsonl`, and the destination is this build's
choice.** Harness spec 5's table says prune "moves rather than deletes" and names no
destination, the same gap 19.1 leaves for the filenames inside the spawn directory. A
sibling of `runs/` keeps the move a rename on one filesystem, and the run id stays on the
file, so putting a record back is a `mv`. A destination that already holds that run id is
a refusal rather than an overwrite. `pruned/` is in `.gitignore` beside `runs/`.

**Prune with no run ids takes every run that is not live, and that is safe because the
default is a move.** The command that destroys something needs a flag; the command that
tidies does not need a list. A live run is never pruned under either form: contract 6
gives a run file one writer, and a rename does not take the file away from an open
descriptor, so the writer would keep appending into a path nothing looks in.

**Typing `hydra prune` on this machine would empty the corpus
`tests/record_contract.rs` walks.** That test reads every `.jsonl` under `~/hydra/runs/`
and returns quietly when there are none, so a prune sweep costs the check silently. The
records are the evidence and they are cheap; prune the ones you have finished with by
name.

**Reap, then prune.** Reap derives liveness from the pid in each run record, so pruning
first leaves it reading records that are no longer there.

## The policy set and the gate, on a run

Tickets 01 and 02. `policies/` holds the five policies of harness spec 7.2 and the
pattern floor of 7.4, `src/policy/` loads and hashes them, `hydra::policy::write_gate`
compiles a policy name into the script of 7.5, and every `hydra run` writes one into
`~/hydra/spawn/<run-id>/` and passes it to `claude` on `--settings`. `tests/gate.rs`
feeds the compiled script synthetic hook payloads and asserts its answers, 23 of them when
ticket 02 closed and 72 tests today;
`tests/spawn.rs` checks that a run writes the script where the run will find it.

**The `PreToolUse` matcher is `*`, and one entry covers every tool.** `claude` routes
reads through `PreToolUse` like everything else, so one event is enough on this
provider. Cursor is the one that needs `beforeReadFile` as well, and that is ticket 09.
The wildcard is documented rather than measured in the research, so ticket 02 measured
it: the gate fired on `PreToolUse:Write` on all three probes below.

**The hook command carries the dialect and the script path is shell-quoted.** The
config's `command` is a string a shell runs, and the spawn directory sits under a home
directory whose name Hydra does not choose. A space in it would split the command and
run the gate with no `argv[1]`, which denies in two shapes and exits 2. That fails
closed, but it fails closed on every call of every run, so the quoting is what keeps it
from being a silent outage.

**No hook `timeout` is set**, so `claude`'s default applies. A hook that fails to answer
fails open on this provider, and inventing a shorter budget than the default only moves
the same failure closer.

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

**Nineteen of `claude`'s twenty-nine tools answer to no capability the five policies speak
in.** 7.1 denies a tool name the table does not recognise, so a table holding only the
nine names that answer to the seven capabilities of the coverage matrix would deny the
other nineteen under `permissive`, which is supposed to allow everything. They map to one
capability, `provider_native`, granted by `permissive` alone. One row rather than nineteen
invented categories, and 7.1's drift check still fires on a thirtieth name.

**It was twenty-one until 2026-09-06, when two moved out in one day.** Map ticket 39 split
`Skill` off to `skill_invoke` and ticket 43 sent `ToolSearch` after it. That is what
splitting one of them costs: a row in `src/policy/table.rs`, an entry in four policy
files, and a `policy_sha` that moves. `ToolSearch` cost only the row, because every policy
granting `skill_invoke` already grants what it needed. The section on the skill store
below has the first change in full.

**`standard` allows `ToolSearch` since build ticket 16.** MCP tools arrive deferred in
print mode on 2.1.263, measured off the probe's `deferred_tools_delta`, so a policy that
admits a server and denies `ToolSearch` has admitted a listing entry the model cannot
call. The tool fetches schemas and executes nothing, and the gate judges the call that
follows, which is why it answers to `skill_invoke` rather than to `mcp_call`. `judge`
still refuses it with `hydra/capability`, along with everything else.

**Two protected configs are still paths rather than files.** 7.5 denied seven paths under
every policy before ticket 40 grew the list to the full deny list the section above
describes. Five of the original seven are real files or directories: `policies/`,
`~/hydra/runs/`, the per-spawn gate script, `config/preamble.md` from ticket 02 and
`config/mcp.md` from ticket 08. The other two are named at `config/env-allowlist.json`
and `config/export-allowlist.json`, and **neither of those files exists**. The
environment allowlist is still Rust code in `src/env.rs`, which 15.2 protects at
admission rather than at run time, so the config
the table means has no file. Denying a write to a path that holds nothing is the same
denial it will be the day the file lands, which is why the names went in early rather
than being added later and forgotten. `tests/gate.rs` asks the compiled gate about all
six paths under `permissive` and `curate`, so the two that are still empty are covered
by the same test as the four that are not.

**Until build ticket 50, Hydra found its policy directory through `CARGO_MANIFEST_DIR`
alone, so the binary only worked from its source tree.** `HYDRA_HOME` overrides it now, and
the paragraph near the top of this file says how. The reasoning that ticket kept:
`PolicySet::discover()`, `preamble::path()` and the
four `config/` paths were compiled against the manifest directory, which is right for a
binary run out of
`target/debug` and wrong for an installed one. A copied `hydra` would compile gates
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

## The skill store, the snapshot and what a run is linked

Harness spec 6.4, built by ticket 07 for `claude`. `~/hydra/skills/` is a git repo of
its own inside the `~/hydra` working tree, ignored by it, created on the first launch
with an empty root commit so there is always a sha to pin. A dirty store is committed
before pinning, because launching from `HEAD` and ignoring the working tree would mean
editing a skill, running, and watching nothing happen.

**The commit trailer names the run that pinned the store, not the one that dirtied it.**
6.4 asks for authorship in the trailer and Hydra cannot supply the author: a `curate`
child writes into the store with its own tool calls and is long gone by the time the
next launch finds the change. `Hydra-Pinned-By: <run-id>` is what Hydra actually knows.
The committer is `hydra <hydra@localhost>` rather than whatever git identity the machine
carries, so the commit is attributable to the harness and a machine with no global
identity can still launch.

**The links point into `~/hydra/skills-snapshots/<sha>/` and never into the store.** The
snapshot is a detached git worktree of the store at the pinned sha, shared between every
run on that sha. `tests/skills.rs` proves the thing this is for: a skill edited after the
pin does not reach the run that pinned it.

**The snapshot is checked out beside its target and renamed into place, since 2026-09-12.**
Build ticket 47 watched three launches in one second on a sha with no snapshot yet. One made
it and two died, one on "already exists" and one on an `index.lock` inside the store's
`.git/worktrees/`, and the dead ones left checkouts under `worktrees/` with no record behind
them. `git worktree add` straight into `skills-snapshots/<sha>` was a check-then-create with a
window a whole checkout wide. Now each launch checks out into its own
`skills-snapshots/<sha>.tmp-<pid>-<n>` and asks `git worktree move` to put it at `<sha>`. The
move is one `rename(2)`, and a directory cannot be renamed onto a non-empty one, so exactly
one launch wins; a loser removes its own staging checkout through git and takes the winner's.
The rename alone was not enough. Every `git worktree` command starts by reading every entry
under the store's `.git/worktrees/`, and an entry another launch is mid-way through making
has no `commondir` yet, which git treats as fatal; four threads on unique staging paths hit
it one time in four. So the git calls run under `flock(2)` on `skills-snapshots/.lock`, taken
by the launch for its few calls and by the reap sweep's `remove_snapshot` for its one. The
kernel releases the lock with the descriptor, so a launch that dies holding it wedges nobody,
and the file carries nothing and is never read. Before any existing directory is used,
`HEAD` in it has to be the sha and the store has to list it. A directory the store does not
list gets `git worktree repair`, which is what a rename behind git's back needs. A directory
at another sha is refused with an error naming what to remove, because deleting it under a
run that might be reading it is not the launch's call. A staging checkout a crashed launch
leaves behind is just a snapshot with an odd name to the reap sweep, which removes it once it
is older than the grace and no live run names it. `tests/skills.rs` races four threads on one
fresh sha and checks that all four get the one directory, the store lists exactly one worktree
beside itself, and no staging directory survives.

**`hydra reap` sweeps snapshots, and which command does that is a choice.** Harness spec
5 says a snapshot is reaped "when no live run references it" and names no command. Reap
already removes a run's scratch and is already typed by a human, so it got this too. The
refcount is the corpus rather than a counter file: `skills::referenced_shas` reads the
`skills.store_sha` off every live run's line 0, which is the same scan `hydra status`
uses. Two guards sit on top, and the second is the interesting one. A snapshot younger
than the launch grace is left alone, the same rule spawn directories get. And a checkout
younger than the grace whose record has not spoken stands the whole sweep down, because a
launch reuses an existing snapshot without touching it, so age cannot see that one and
removing it would leave a running child with dangling links.

**The injected root ignores itself and the test says so against real git.** A
`.gitignore` holding `*` goes into `<worktree>/.claude/skills/` before the links, and
`tests/skills.rs` asserts that `git status` reports nothing and `git add -A` stages
nothing. An existing `.gitignore` is left alone: a repo that tracks its own would see the
overwrite as a modification to the human's checkout.

**Passes 7 and 8 are the first two given a task, and the second found what the first flag
broke.** Pass `01a084d3-bdd3` produced 131 lines across four files, the first patch a panel
reviewed as a patch, and `agy` blocked it correctly for editing a baseline test to hide a
regression in `hydra reap <unknown-id>`; `claude` at 26K tokens and `cursor-agent` at 84K both
returned zero findings. Pass `01a08684-67f4` ran the same target with the test clause in the
brief and the two broken contracts named in the task. The proposer obeyed both, put the
resolver in its own module and wrote an `is_full_uuid` that checks hyphens and hex digits
rather than counting to 36, and `agy` blocked it anyway on `self_consistency` for the reap and
show asymmetry the task had ordered. `judge_task` hands a judge the checklist and the diff and
never the task, so the judge could not know. Its second finding, that let-chains fail on stable
Rust, is false on this machine's rustc 1.94 and the admission gate had already built and tested
the patch. Costs: proposer 4.53M then 6.81M tokens, panels 235K then 194K, 20.8 then 45.1
minutes. Map ticket 57 carries the decision.

**The store held no skills until 2026-09-09, and `linked` was empty on every run before
it.** Map ticket 56 wrote the first one, `hydra-self-edit`, and measured the whole channel on
probe run `01a086aa-b63b`: the launch committed the dirty store and pinned `c44b43d`, `linked`
carried the skill, the worktree symlink resolved into the snapshot, and `claude`'s own
discovery put it fourth in `skills.resolved` of 27. Every wire works unmodified.

**A skill in `resolved` is offered to the model, and whether the model opens it depends on
the model.** Two runs on 2026-09-09, same task, same `standard` policy, same store sha, same
worktree shape: edit `reap`'s doc comment, change no behaviour and no test, then say which
guidance you consulted. Run `01a086b2-5102` on haiku made three tool calls, `Read`, `Edit`,
`Bash`, never invoked the skill, and named the repo's `CLAUDE.md`. Run `01a086b3-ca7c` on
sonnet opened with `Skill { skill: "hydra-self-edit" }` before touching a file and named the
skill and two of its rules. An earlier haiku probe asked a question matching no trigger and
went hunting for `~/.claude/skills/hydra-self-edit/SKILL.md`, took the refusal `standard` owes
the human's own skill root, and reported the skill unavailable, having tried neither the Skill
tool nor the snapshot; `standard` grants both, `skill_invoke` plus a `file_read` scoped to
`skill_snapshot`.

Nothing in Hydra is wrong in any of the three. The gap is between a skill arriving and a skill
being used, and it is the whole question for anything built on the store: the same knowledge
written into `brief()` changed haiku's behaviour on pass 8, and written into a linked skill it
did not reach haiku at all. The prompt is frozen in Rust and the store is the versioned,
proposable, mergeable thing, so that is backwards from what training wants. Map tickets 55
and 56.

**A name the repo already carries is not linked over.** The store's copy is held back
rather than overwriting a project skill the human's repo tracks, and the collision check
reports it. `hydra.toml`'s `providers` is how a skill opts out of a provider; an absent
sidecar admits all of them.

### Asking `claude` what it resolved

`skills.resolved` comes from a free probe: `claude -p x --model
hydra-skill-preflight-not-a-model --session-id <uuid>` in the checkout. Claude Code
rejects an unknown model before it calls the API and still writes the session transcript,
which carries one `attachment` line whose `attachment.type` is `skill_listing`, holding
`skillCount` and a `names` array. Measured 2026-09-05 on 2.1.263: 1.8s, no API call, no
tokens.

Three details are load-bearing and each was measured. `--session-id` names the transcript
file, so the read is a path Hydra can predict rather than the newest file in a directory
another session may be writing to. Stdin is `/dev/null`, or the binary waits three
seconds for input that is not coming. And the transcript directory is
`~/.claude/projects/<slug>/`, where the slug is the checkout's path with every character
that is not a letter, a digit or a hyphen replaced by a hyphen; that rule was measured by
running the probe in a directory called `slug.probe_1`.

**Four things about the probe are worth knowing before trusting it.**

Its freeness is a property of 2.1.263 rather than a promise. A build that started
resolving unknown model names would turn this into a paid call, which is why the model
name says what it is.

It answers names and not descriptions. On the probe's path every filesystem-discovered
skill renders as a bare name, so the listing settles identity and precedence, which is
what the collision check asks, and cannot say whether a description survived.

It runs the human's own `UserPromptSubmit` hooks, because it is an ordinary `claude`
invocation with the user's settings loaded.

It leaves a transcript in `~/.claude/projects/` that nothing reaps, which is worth
holding against D9 and then letting go. Hydra never writes there; `claude` does, as it
already does for every recorded run in the same directory. The probe adds a second file
per run to a directory ticket 02's runs were already filling.

A probe that cannot answer does not refuse the run. `skills.resolved` goes in as null,
which contract 5.3 already requires a reader to treat as unanswered rather than as an
empty listing, and the third of the three collision tests does not run. The other two do,
and the launcher says so on stderr.

### The write check, and rung one as its caller

6.4 checks collisions twice: "at write, the curator refuses to create or rename a store
skill to a name the index already holds, so no collision Hydra causes reaches a run".
`skills::admit` is that check. The curator writes through its own gated tool calls, so
the check sits where those writes become a store commit, in `curate::propose`. Build
ticket 91 wired it, after the second hiring panel's finding G23 pointed out that it had
been written with a test and no caller.

Before the commit, `propose` lists every skill directory in the checkout whose `SKILL.md`
was not in the tree at the proposal branch's sha, read from the store's own refs rather than the
checkout's `HEAD`, which the curator can write, and calls `admit` on each against
the index over the user's seven roots. The validator makes a skill's name its directory's
name, so a rename always shows up as a new directory. A skill the curator only edited is
not re-checked, because the launch check already covers the names the store holds. One
refusal and the pass commits nothing and removes nothing: the curator's work stays in its
checkout and the branch stays where it was cut, for a human to rename by hand.
`Proposal::refused` carries `admit`'s own sentence, which names the skill and the user
root holding it, and `hydra curate` prints each one. `admit` reads the frontmatter first, so
a new skill whose `SKILL.md` does not validate is refused at write too, which is spec 6.4's
validator running where the spec puts it.

**The listing walks the working tree, not `git diff`.** The ticket suggested `git diff
--name-status` against the starting sha, and that lists tracked paths only, so it never
shows a directory the curator just created. Measured on a scratch repository while
writing the check.

The launch check is not a substitute. It fails the whole run, which 6.4 calls harsh and
justifies by what the write check does to the odds. With `admit` called at write, a badly
chosen name costs a refused commit rather than a refused launch.

### The three collision tests, and why the cheap two are not redundant

A store skill whose name the checkout already holds is a collision, decided before anyone
is asked. A linked name the user-scope index holds is a collision. A linked name the
provider did not list is a collision.

The first test reads both keys, the directory name and the frontmatter name, on both
scopes. A store skill Hydra could not link at all is a hit whatever holds the name, even
a directory that is not a skill, because a store skill silently absent from a run is what
this check exists to prevent. A skill that did link can still clash on a project skill's
frontmatter name, which the filesystem let through because `claude` keys on the directory
and `agy` does not.

The middle one looks redundant against the third and is not. Ticket 24 put one name in
`~/.claude/skills` and in the injected root: the provider listed it once, carrying the
user-scope copy, and the store's entry was silently absent. The name is in the listing,
so the diff sees nothing. Only the index comparison catches that one.

The index scans all seven user-scope roots of 6.4, including `~/.codex/skills` and
`~/.grok/skills`, which do not exist on this machine and appear the day the matching CLI
is installed. `Provider::user_skill_roots` is the per-provider filter over it, and today
`claude` reads one of the seven.

**Measured end to end on 2026-09-06.** A store holding `unslop`, a name
`~/.claude/skills` also holds, refused the launch in 2.3s with `outcome:
skill_collision`, `outcome_source: refusal`, a null exit code and no child. Contract 8's
`skill_collision` has a producer now.

### The write-time validator, and a ticket claim that expired

The store's validator parses the frontmatter with a real YAML parser rather than scanning
for `: `, because a scan is wrong in both directions. Measured on this machine 2026-09-06
with the validator itself: `writing-fragments` and `writing-shape` both carry `: ` inside
a **quoted** description and are valid YAML, and `3:1` with no space is valid too. What
breaks is a bare colon in an unquoted scalar, which is what `agy` drops and Cursor empties.

**Ticket 07 says `~/.claude/skills/handoff-frank` fails this today, and it no longer
does.** Ticket 24 measured that on 31 August and the file has been edited since; its
description is 656 characters and parses cleanly. Nothing in `~/.claude/skills` fails the
YAML rule as of 2026-09-06, so the validator's proof is a fixture rather than a file on
this machine.

Two rules beyond the file format, both of which the store needs and 6.4 implies. The
frontmatter `name` must equal the directory name, because `claude` keys on the directory
and `agy` on the frontmatter and a store entry where they disagree is a different skill to
each of them. And `disable-model-invocation: true` is refused, because it hides the skill
from the provider's own listing, which is the answer the launch check reads. Eight of the
eleven skills in `~/.claude/skills` set it, and none of the eight appears in `claude`'s
listing.

A store skill the validator refuses is skipped with a line on stderr rather than failing
the launch. The store is a directory a human and a curator both write to, and one bad
file in it must not stop every launch on the machine.

### What injection did to the listing, measured

The side effect 6.4 and contract 5.3 both warn about reproduced on this build. One real
`claude --model haiku` run with two injected links: the listing was 7,950 characters over
28 entries, and 5 of the 28 rendered as a bare name with no description. **Both injected
store skills were among the five.** So the model was shown two names it had no reason to
reach for, and `store_sha` plus `linked` would have claimed otherwise. That is the whole
argument for `skills.resolved` existing, arriving on the first run that used it.

`resolved` records identity and not description survival, for the reason above: the free
probe strips every filesystem description, so the field cannot carry what the paid run's
transcript showed. That is a smaller field than contract 5.3's paragraph implies and it
is the largest one that costs nothing. The run's own transcript holds the rest and
nothing reads it.

### A linked skill is invocable and readable, built

Harness spec 6.4's "a run may invoke what it was linked and may read it", 7.1's
`skill_invoke` and 7.2's `skill_snapshot`, decided by map ticket 39 on 2026-09-06 and built by
build ticket 13 the same day. Frank authorized the edit to protected config #1 that carries it.

`Skill` answers to `Capability::SkillInvoke` in `src/policy/table.rs`, out of the twenty-one
that answered to `provider_native` and now nineteen, `ToolSearch` having followed it out under
build ticket 16. `read-only`, `standard`, `curate` and
`permissive` grant both; `judge` grants nothing and refuses them with `hydra/capability`.

`Scope::SkillSnapshot` is the run's own checkout of the store,
`~/hydra/skills-snapshots/<sha>/`, on `file_read` for those same four and on no policy's
`file_write`. A run that could write there could edit its own instructions mid-session, which
is the thing copying the skill into the worktree would have allowed and is why the copy was
rejected.

`Layout::skill_snapshot_scope` is the whole rule, and both sides call it: the launch passes the
sha `inject_skills` pinned, and contract 7.7's re-evaluation passes the sha it reads back off
`hydra.skills.store_sha`. Neither side can drift, the same shape `Layout::skill_checkout` has
now that `skill_store_scope` is gone: a pure function of the layout and the run id, with each
caller deciding on its own whether this spawn gets one at all. A run with no sha gets `<spawn
dir>/no-snapshot`, a path nothing creates, for the reason a run with no repo gets `no-worktree`:
pointing at `skills-snapshots/` would hand every run every other run's snapshot.

**Measured on one real `standard` run**, `01a078bc-420f-7b23-8e67-2dfc9f98f859`, `claude`
2.1.263 on `haiku`, with a two-file marker skill in the store. Both sentinels came back. The
gate answered `hydra/allow: policy standard allows skill_invoke.` on the `Skill` call and
`hydra/allow: policy standard allows file_read.` on a `Read` of a sibling through the link
path, which is the path the provider names to the model as the skill's base directory. The
gate resolves that link into the snapshot before comparing, so the same path opened for
writing is `hydra/out-of-scope`. Reading it back derives both decisions again and reports no
gate leak, which `tests/normalize.rs` checks against the corpus and against a synthetic record
whose only difference is the missing sha.

**`policy_sha` moved and 18 of the 22 records on this machine now read as `StalePolicy` to
rung one.** Accepted while the corpus is probes. The ticket predicted 13 of 17; the corpus
grew between the decision and the build. Three of the 22 carry no `policy_sha` at all, which
is what a harness run writes.

**The two capabilities that used to reach the snapshot for writing are closed, by build
ticket 14.** `skill_snapshot` still appears on no policy's `file_write`, and now the
snapshots directory is one of 7.5's paths denied outright as well, so both routes that
used to bypass the read-only scope land on the denial instead of on it. `shell`, under
`standard` and `permissive`, and `file_write` with scope `anywhere`, under `permissive`,
both answer `hydra/protected-path` on a snapshot path now, and `tests/gate.rs`'s
`a_write_through_a_skill_link_lands_on_a_denied_snapshot` runs the check under both
policies. Before this, a mutated snapshot was one `hydra reap` could never sweep, since
`skills::remove_snapshot` goes through `git worktree remove`, which refuses a dirty
checkout, and snapshots are shared per sha, so the run a write there corrupted could be
one that was still going.

**One sentence here turned out wrong.** Denying `~/hydra/skills-snapshots/` whole was
thought to also stop `curate` from ever writing a proposal through a checkout of its own.
It does not: the checkout lives at `~/hydra/skill-worktrees/<run-id>/`, a sibling of the
snapshots directory rather than something inside it, so denying one leaves the other
untouched.

**`cursor-agent` and `agy` have no `Skill` tool the specs know of**, so `skill_invoke` maps to
no row in their tables until one appears. The snapshot scope applies to them unchanged: the
gate resolves paths the same way on all three, and `src/normalize/cursor.rs` recomputes the
scope off the same field.

## The canonical MCP document, and what a run is handed

Harness spec 6.5, built by ticket 08 for `claude`. `config/mcp.md` is protected config
#5 and the only place a server is declared; `src/mcp.rs` reads it and decides what one
launch gets; `src/provider/claude.rs` turns that into the file `--mcp-config` names.
`tests/mcp.rs` drives the whole path and launches nothing.

**The document is prose with one fenced `toml` block in it, and the block is the data.**
Everything else in the file is for whoever edits it. Exactly one block is allowed, and a
document with none is an error rather than a document with no servers, because a human
who breaks the fence would otherwise get a launch that silently admits nothing.

**A server is refused at parse time for four things**, each of which would otherwise
become a silent shortfall or an ambiguous record: an unknown field, so `admits` for
`admit` fails instead of admitting the server to nothing; an `admit` entry that is not
one of the five policy names; a name carrying `__`, because `claude` joins a server and
a tool with it and the record's `resolved` is read back off that join; and a server that
names both a command and a url, which `agy` refuses and which no one document can
generate for all three providers.

**Admission is not permission, and a real record now says so.** `mcp_call` is granted by
`permissive` alone (7.2), so a server admitted to `standard` connects, its tools reach
the model, and the gate refuses every call to it. Measured on run
`01a077c4-68e9-7203-9cc9-030044661116`: the model called `mcp__hydra-probe__ping` and
got `hydra/capability: policy standard does not allow mcp_call
(mcp__hydra-probe__ping).` back as the tool result. That is ticket 27's decision working
as written, and it is the same shape as the linked-skill finding above, which is map
ticket 39. Nothing here was changed to fix it and the document says it out loud instead.

**`judge` is the one launch shape that can be handed no server**, so 6.1 item 6's check
has exactly one instance today. 6.7's `judge` argv carries `--strict-mcp-config` and no
`--mcp-config`, so a
document admitting a server to `judge` is a launch that cannot deliver, and the run is
refused with `mcp_unavailable` before anything spawns. Everything else delivers what it
admits, and a policy that admits nothing proceeds untouched, which is what keeps `judge`
runs launching on a machine whose document names servers for other policies.

**A run that admits nothing still gets a file.** `{"mcpServers":{}}` plus
`--strict-mcp-config` is the difference between a run with no servers and a run with
whatever the human has configured, and it is what lets the record claim the set.

**The `codex` fragment is serialized by the `toml` crate, since build ticket 100.** So any string
`config/mcp.md` holds, quotes, backslashes and newlines included, reaches `.codex/config.toml` as
the same string, and a dot or a quote in a server name no longer splits the table header.
`provider::codex::mcp_config` built it with `format!` before, escaping quotes in `args` and
nothing else. `env` and `http_headers` are written as sub-tables now rather than inline tables,
which is the same TOML.

**`mcp.doc_sha` is a sha256 of the whole file, prose included.** So `shasum -a 256
config/mcp.md` reproduces what a record carries, and a reader holding a record can check
the document in front of them with one command. It is the opposite call from
`policy_sha`, which leaves `policies/README.md` out of its hash so a wording fix does
not move a value every record carries. The two differ because there the prose is a
separate file, and a sha over part of a file is one nobody can recompute by hand.

**The document shipped with no server until build ticket 26**, which added the delegate
face under `standard` and `permissive`; the interactive-mode section below has the block.
Until then what was in it was the format and one commented example per transport, and both
the unit tests and `tests/mcp.rs` asserted that emptiness. `tests/mcp.rs` asserts the
admission now, which is what that emptiness check was for.

**Measured on two real runs, 2026-09-06, `claude` 2.1.263, `--model haiku`.** Run
`01a077c3-e32f-7612-bb19-89227bf6f531`, `standard`, on the shipped document: both flags
were accepted, `mcp.json` was `{"mcpServers":{}}`, the init frame reported
`mcp_servers: []`, the record carried `doc_sha` plus two empty lists, and the whole call
took 6.8s. The `doc_sha` is what `shasum -a 256 config/mcp.md` prints, checked by hand.
Run `01a077c4-68e9-7203-9cc9-030044661116`, `standard`, with one stdio server admitted:
`injected` and `resolved` both read `["hydra-probe"]`, the init frame independently said
`[{"name":"hydra-probe","status":"connected"}]` and listed `mcp__hydra-probe__ping` among
its tools, and the call took 12.6s. **The second run's document is not the one that
ships.** The server was added by hand for the measurement and taken out again, so that
record's `doc_sha` names a document no longer on disk, which is the honest thing for a
record to do and worth knowing before anyone tries to reproduce it.

### `--restricted` drops what settings define, and `judge` says so on the argv

Ticket 23 measured `claude` 2.1.257 dropping all MCP under `--restricted` "despite the
help text saying otherwise", and 6.5 and 6.7 were both written from that: 6.7 left
`--mcp-config` off the `judge` argv because "`--restricted` drops it anyway", and 6.5
says `claude` closes ticket 27's inherited-server residue.

Build ticket 08 found half of that expiring on 2.1.263. `claude --restricted --mcp-config
<file>` resolved the server in that file, and 2.1.263's own `--help` now says
`--restricted` ignores user, project and local settings files and "add
--strict-mcp-config to skip MCP servers too". From one measurement it inferred another,
that settings-defined servers ride the same path and a `judge` run therefore inherits the
human's. **That inference was wrong**, and map ticket 43 measured the rest on 2026-09-06
rather than reasoning about it. The free probe, a stub user-scope server in a scratch
`HOME`'s `.claude.json` and a stub project-scope server in the cwd's `.mcp.json`:

| Server defined in | no flags | `--restricted` | `--restricted --strict-mcp-config` |
|---|---|---|---|
| `~/.claude.json`, user scope | loads | dropped | dropped |
| `.mcp.json` in the cwd, project scope | loads | dropped | dropped |
| `--mcp-config <file>` | loads | loads | loads |

So `--restricted` drops what settings define and keeps what the argv passes. A `judge`
run passes nothing, and got an empty set on 2.1.263 before this ticket touched anything.
The scratch `HOME` is what made the measurement possible without writing a server into
the human's own config.

**Build ticket 16 carries the code, 2026-09-06.** Three things landed, and a fourth
turned up on the way.

**`judge` gains `--strict-mcp-config`, with no `--mcp-config` beside it.** Not to close a
hole, since there is none on this build, but because the vendor's help ties skipping MCP
to that flag rather than to `--restricted`, so the argv names the guarantee instead of
resting on how `--restricted` reads settings this month.

**The probe carries the launch's own MCP flags, from one function.**
`claude::mcp_flags` answers what a shape does about `--restricted`, `--mcp-config` and
`--strict-mcp-config`, and `claude::argv` and `claude::probe_argv` both read it, so the
probe cannot answer for a server set the run will not have. `crate::mcp::Probe` used to
be `Strict(file)` or `Unanswerable` and is now the policy name and the file, which is
what those flags come from. A `judge` record's `mcp.resolved` is `[]` and no longer null.

**`tests/drift.rs` re-measures the vendor per suite run.** Two free tests, a third under
`HYDRA_DRIFT_PAID=1` that the thinking-text section describes, since build ticket 48 a
`codex` login probe, and since build ticket 101 a paid `codex` probe of a hook that dies before
it answers, which passed on its first run on 2026-09-24 and so measured the fail-open
`SECURITY.md` states; those are the only tests in the suite that launch a provider. The first drives the launch's own producer,
`claude::resolve_in`, with a `judge` `Probe` and reads its `Resolved` back: `mcp` is
`Some([])` and `skills` is `None`. The second runs the same stubs with the flags off and
gets both servers, which is what keeps the first from passing because a stub was
unreadable. They skip with a printed reason when `claude` is not on the PATH, and every
call they make, the version check included, runs under a scratch `HOME` and a scratch cwd
that a passing test deletes. The failure message carries the version string, so the day
the answer moves the failure says which build moved it. The pair costs 0.7s to 1.3s here.

`resolve_in` is `resolve` with the home and the environment as arguments, and that test is
why it exists. `ChildEnv::build` drops a `HOME` that disagrees with the passwd entry,
which is right for a launch and wrong for measuring the binary, so the test sets it back
with `ChildEnv::set` and passes the same path for the transcript.

**A restricted probe reports no skills either, which the ticket did not anticipate.**
Measured the same evening on 2.1.263, a scratch `HOME` with one user-scope skill and a
cwd with one project-scope skill: the `skill_listing` attachment held 14 names with no
flags and the binary's own 12 under `--restricted`. Filesystem-discovered skills are
dropped along with the servers. Left alone, that would have told the launch-time collision
check that every skill Hydra linked had lost to something else, and refused every `judge`
run on a machine with a non-empty store. So `claude::resolve` answers `None` for
`skills.resolved` when the probe ran restricted, which is the stance contract 5.3 already
takes on an unanswered listing, and prints one line on stderr saying it did. That line is
for parity: a probe that fell over already prints, and without it the two nulls read the
same on the way past. **The finding underneath it reaches no field.** A
`judge` run reads none of the skills Hydra linked into its checkout. Nothing is lost
today, because `judge` grants no `skill_invoke` and could not have called them, and the
question of whether a `judge` run should be linked skills at all belongs on the map.

### Asking `claude` which servers it got

**One probe answers both `resolved` fields**, except under `judge`, where it answers the
MCP one and the section above says why. Ticket 07's free probe already ran per
launch; it now carries the run's own `--mcp-config` and `--strict-mcp-config`, and its
transcript's `deferred_tools_delta` attachment is the answer. Measured 2026-09-06 on
2.1.263 against `tests/fixtures/mcp-probe-server.py`, a stdio server written for this:
`addedNames` gained `mcp__hydra-probe__ping`, and the same attachment carries
`pendingMcpServers`, `needsAuthMcpServers` and `failedMcpServers`, the last as objects
with a `name` and an error string. The probe costs 1.8s with an empty config and 1.9s
with one server in it, against the 1.8s ticket 07 measured with no MCP flags at all.

**The probe carries the run's flags so that its answer is the run's.** Without them it
would resolve the human's servers and the record would claim them for a run that will
not have them. `--strict-mcp-config` on the probe is chosen for that reason rather than
reached for as a speed lever, which is the decision ticket 07 left open. The cost is
that every admitted server starts twice per launch, once for the probe and once for the
run.

**A `judge` run's `resolved` is a measured `[]`** since build ticket 16. The probe carries
that policy's own `--restricted --strict-mcp-config`, so the empty array is what the
binary reported rather than what Hydra assumed. Two producers of null are left: a probe
that failed, and a run with no checkout, where the probe never runs because it is the same
process that answers `skills.resolved` and 6.4 gives that run no worktree to ask in.
Contract 5.3 already requires a reader to tell both apart from an empty listing.

**A server that connects and offers no tool is invisible to this reading**, since the
answer is derived from tool names. Nothing on this machine has one, and the run's own
init frame is where a later reader would go: it carries `mcp_servers` with a `status`
per server, measured as `[{"name":"hydra-probe","status":"connected"}]` on the run
above. That frame arrives after line 0 is written, so it corroborates the probe rather
than replacing it, and a reader wanting the provider's own words takes it from `raw`.

**A server that will not start does not refuse the launch.** 6.5 gives the fatal
pre-flight to Cursor alone and says why: on `claude` the strict flag already guarantees
the set. So a failure is a line on stderr and a difference between `injected` and
`resolved` in the record, which is exactly what that pair of fields is for.

## `cursor-agent`, the second provider

Ticket 09. `src/provider/cursor.rs` holds every fact that is true of that binary,
`src/normalize/cursor.rs` reads its records back, and `src/policy/table.rs` carries its
capability table beside `claude`'s. Every claim below was measured on 2026.09.02-c22c1a3
on 2026-09-06 unless it says it was read off the bundle at
`~/.local/share/cursor-agent/versions/2026.09.02-c22c1a3/`, and contract 9.3's warning
applies to all of it: the display name in `model` tracks the account's selection, not
the binary, so a claim about it can expire without a version moving.

**Two real runs, both under `--model composer-2.5`, and the corpus walks pass with
both in it.** Run `01a07891-a339-7173-96d9-70beb88e5efb`, `standard`, was asked to read
`Cargo.toml`, run `echo T09HYDRA` and write `t09-probe.txt`: all three went through,
the record is 33 lines, `completed` with `usage_source: provider` (12,856 input, 320
output, 73,256 cache read, 0 cache write), `diff_stat` one file and one insertion, and
11.9 seconds from line 0 to `run_end`. Run `01a07892-459d-7420-904e-91d6a6986159`,
`read-only`, was asked to read `/etc/hosts`, read `Cargo.toml` and write the same file:
the first was refused with `hydra/out-of-scope`, the second went through, the third was
refused with `hydra/capability`, the model answered `BLOCKED\nOK\nBLOCKED`, and the
record derives two `permission_decision` denies by `hydra_policy` beside one allow, no
finding, and `completed`, because one call got through and that is the predicate the
outcome section above defends. The first run was reaped afterwards to check that a
spawn directory holding Cursor's socket file goes cleanly; it did, and the record stays.
Two probes outside Hydra preceded them, one exercising six tools under a logging hook
to learn the hook's spellings and one denying a read at `beforeReadFile`, and both are
what the frame shapes in `tests/common/mod.rs` are copied from.

**The gate answers in Cursor's own shape, on both events, and 6.6's "two shapes rather
than three" sentence is one shape short.** 6.6 says the script answers in two shapes
because Cursor accepts Claude's response dialect on `preToolUse`. It does, through a
translation the bundle applies to that one event and to no other, and `beforeReadFile`
gets none: research 26 read it accepting `allow` or `deny` in Cursor's own `permission`
field, and the second probe measured that shape denying a read. So the `cursor` dialect
answers `permission` plus `user_message` plus `agent_message` on both events, which
research 30's rig had already used on `preToolUse` and had obeyed. The tamper wrapper
speaks the same shape, because a refusal Cursor cannot parse is an invalid response that
`failClosed` blocks with the reason lost. Three shapes, then, one per provider, and the
gate still decides once and only speaks at the end.

**The gate is compiled for one provider and refuses to be asked in another's dialect.**
The compiled config carries `dialect`, and `hydra/wrong-dialect` is the answer when
argv names a different one. A gate compiled for Cursor that was asked in `claude`'s
shape would otherwise answer for a tool table the provider never calls.

**`preToolUse` fires for reads on this build, so `beforeReadFile` is the second line
rather than the first.** 6.6 registers `beforeReadFile` because "`preToolUse` alone
leaves the file-read path ungated on that provider", from ticket 26. Measured on
2026-09-06: a `Read` fires `preToolUse` with `tool_name: Read` and `tool_input:
{file_path}` before the read executes, and `beforeReadFile` fires after the read
succeeds inside the provider and before the content reaches the model, carrying the
path, the content and no tool name. On the shipped gate the first hook refuses an
out-of-scope read and the second never sees it; the second probe, with a hook that
allowed at `preToolUse`, measured the second event refusing on its own. Both are
registered, with `failClosed: true` on each, and the gate maps a `beforeReadFile`
payload onto `Read` with its measured path field. Whether the next build still fires
the first for reads is exactly the claim 6.6 declined to rest on.

**The hook's spelling of a tool is not the stream's, and the table holds the hook's.**
Five names arrived on the hook across the two probes: `Read`, `Shell`, `Write`, `Grep`
and `Delete`. Thirteen more are read off the bundle's hook descriptors, each of which
builds the `tool_input` the hook sees: `List`, `ReadLints`, `Fetch`, `WriteShellStdin`,
`task`, `FetchMcpResource`, `ListMcpResources`, `ComputerUse`, `RecordScreen`,
`askQuestion`, `createPlan`, `generateImage` and `updateTodos`. `WriteShellStdin` is
`shell` with no command in its input, so the pattern floor cannot see what it feeds a
background shell, which is 7.4's failure mode on a second spelling and is the reason
the row says so. An MCP tool arrives as
`MCP:<tool>`, read off the same bundle and unmeasured because no server is configured
here. Three spellings are Cursor's own and would otherwise surprise somebody. The
listing tool arrives as `Grep` carrying `output_mode: files_with_matches`, not as a tool
of its own, though the stream calls it `globToolCall`. A `Write` fires a `Read` hook for
the same `tool_use_id` first, so a write under `read-only` is refused on the second
hook. And a `Delete` is `file_write` on the path it names. There is no init frame to
check the table against, unlike `claude`, so 7.1's run-time table check has nothing to
read on this provider and a name it does not know is caught by the denial that reveals
it.

**The `Skill` split landed on `Read` here, as expected.** Cursor has no `Skill` tool. A
linked skill is rendered into the prompt as an `<agent_skill fullPath=...>` element and
the model reads its `SKILL.md` through the ordinary read tool. So when map ticket 39 gave
`Skill` its own capability on 2026-09-06 this table had no row to move, and
`skill_invoke` maps to nothing here until Cursor ships such a tool. What moved is the
scope: a `Read` of a path resolving under `~/hydra/skills-snapshots/<sha>/` is now inside
`skill_snapshot` rather than outside `worktree`, and `src/normalize/cursor.rs` recomputes
that scope off the same `hydra.skills.store_sha` the `claude` side reads. Unmeasured on a
live Cursor run: no skill has been linked into one.

**The normalizer rebuilds the hook's spelling from the stream's, and that is what
`tool_call.arguments` carries.** The stream says `editToolCall` with `path` and
`streamContent`; the hook that gated it saw `Write` with `file_path` and `content`.
Contract 7.7 re-evaluates the policy against what the gate saw, so the adapter maps
each measured oneof key back to the hook's name and input the way the bundle's own
`createToolInput` does, and the stream's object survives in `raw`. A key it does not
know keeps its own spelling with no capability, which is an `UnknownTool` finding.
Both events for a call come from frames sharing one id, the frame's `call_id` and the
object's `toolCallId` agreed on every frame measured, and a `completed` frame for a
refused call carries `args: null`, so the call is taken from `started` and the result
alone from `completed`.

**Hydra's voice is read out of the refusal text, and nothing else is.** There are no
hook frames on this provider, so the tool result is the only second source on a
refusal, and 7.7 says its reason text cannot carry attribution. It cannot, and the
decider is derived from the policy as on `claude`. What the text can carry is whether
Hydra's gate spoke and which code it spoke, because every gate answer starts with a
`hydra/<code>:` prefix and no other hook's does, and the code is checked against the
gate's own list so a guard quoting `~/hydra/runs` is not mistaken for it. Cursor
concatenates every hook's message into one reason, allowing hooks included, so
`hydra/allow` inside a refusal means Hydra allowed and something else denied, which is
`external`. A refusal with no code at all beside Cursor's own `Agent note: Do not
suggest workarounds to the blocked tool.` line, or a `rejected` case on a shell command,
is somebody else's. Where a denied read lands on the stream: the message is
`readToolCall.result.error.errorMessage`, with `File read was blocked by a hook: ` in
front of it when `beforeReadFile` refused and nothing in front when `preToolUse` did; a
denied write is `editToolCall.result.error.error`; a denied shell command is
`shellToolCall.result.rejected.reason`.

**An allowed call raises no `Ungated` finding on this provider, and the reason is the
config.** On `claude` a hook that fails to start fails open, so a call with no gate
answer behind it is worth a finding. Here the config says `failClosed: true`, a hook
that failed to run would have blocked the call, and the stream carries no gate answer
for an allowed call anyway, so its absence says nothing. The finding survives for the
one shape where it means something: the policy denies, something refused, and Hydra's
voice is not in the text, which is a config the loader did not run, and the loader
refuses a symlinked one silently.

**No terminator means `failed`, not `crashed`, on this provider.** Harness spec 10:
`cursor-agent` writes its `result` frame only on success, and its every hard failure
is the `catch` path, one sentence on stderr, no frame, exit 1. Over the pty Hydra owns
that sentence is a line of the record, kept as a string per contract 3, and the
normalizer keeps those lines on `Run::noise`. Two of them are refusals rung one must
not learn from, and the derivation reads them: `ActionRequiredError` beside `usage
limit` is `rate_limited`, which research 33 measured twice on 2026-09-04 by pinning a
premium model, and `Authentication required` is `auth_failed`, which research 21 read
out of the bundle's logged-out messages. That is a text match and it is written down as
one; a reworded sentence sends the run back to `failed`. Neither was reproduced on a
Hydra run, because reproducing the first means burning a premium model's refusal and
the second means logging out. A signal death reports the same exit code 1 through the
pty, which is why the detached half now writes the signal into `run_end.error` and the
derivation reads that note first.

**Four tools Cursor runs without asking the hook, and the table says so, since build
ticket 35.** Map ticket 51 measured it three times on 2026-09-08: `ReadLints` with a
list-valued `paths`, `getMcpToolsToolCall` listing an MCP server's tools, and
`awaitToolCall` waiting on a background task; build ticket 113 added `WebSearch` on
2026-09-24. All completed with results under
`read-only` and `standard`, none with a `hydra gate:` line in the spawn's `cursor-data`,
while a `Read` outside the worktree in the same runs was refused with Hydra's reason. They
are `unhooked(name)` rows now, `Capability::Unhooked`, which the compiler leaves out of
`gate.py`'s config so the gate is unchanged, and the replay records a call to one as a
`permission_decision` with `decision: allow`, `decider: provider` and the reason
`hydra/unhooked: <tool> is run by cursor-agent without asking the hook`: not a leak,
because no gate was asked, and not `external`, because nothing denied. If the hook ever
fires for one, the gate's own answer stands and the replay reports a `Disagreement`.
**The risk per tool today.** `ReadLints` returns `totalFiles: 0` for every path, `/etc/hosts`
and a file that cannot parse included, in print mode and in the interactive TUI: there is no
language server behind the CLI, so it reads nothing. `getMcpToolsToolCall` names the tools
of a server the run's own MCP document admitted, which the run could call anyway.
`awaitToolCall` turned out to be hooked after all, the same evening: Cursor answers it by
reading the background task's output file under the run's own `cursor-data/` and fires the
hook on that read, which three runs and then the probe 01a08111-1e5e showed refused
`out-of-scope`, since the spawn directory is a denied parent. The row stays `unhooked` for
the replay, which sees a `taskId` and no path, and the runtime answer stands as a
`Disagreement` when the two differ. **Build ticket 38 carved the run's own `cursor-data/`
out for reads**: `own_read_paths` beside `own_write_paths` in the compiled config, one entry
on `cursor-agent` and none elsewhere, checked in `gate.py` before the denied-parent check
and only when the policy grants `file_read` at all, so `judge` still refuses and a sibling's
spawn directory stays denied. Measured on run 01a0811e-d401 the same evening: a `standard`
Composer run started `sleep 8 && echo slept` in the background, awaited it, and got `slept`
and exit 0 back, with no disagreement on the record. A fourth unhooked tool shows up the way
these did, `unknown_tool` beside a completed result. The two corpus walks pass with no skip.

**One more shape from the same day.** An `editToolCall` completed frame arrived with no `args`
at all and "Invalid arguments: old_string: Required" as its result, which the replay used to
file as a `Write` gate leak; it is a limit now, since a call the provider refused on its own
arguments never ran and the record cannot say what path it would have taken.

**Two noise lines per run, and one of them is new.** Cursor writes the same bare
show-cursor escape `claude` does after its last frame, and before its first frame it
writes a coloured tip, `Tip: You can start the Cursor CLI with 'agent' (same as
'cursor-agent').`, on stderr, which the pty merges into the record. Both are kept as
strings and normalize to nothing.

**The fatal pre-flight is three free commands, and its listing names the servers the run
will start.**
6.1 item 7, built as 6.5 writes it: `cursor-agent mcp enable <name>` for each injected
server, run with cwd inside the worktree because `enable` resolves the project from cwd
and ignores `--workspace`; `mcp list`; `mcp disable <name>` for every server the listing
names that Hydra did not inject, which is 6.5's per-run kill switch for the human's
user-scope servers under the redirect; then `mcp list` again. Any injected server the
final listing does not report `ready` refuses the run with `mcp_unavailable` and a
remediation naming the pre-flight and the status each server reported. A disable that
fails is a line on stderr rather than a refusal, because the gate refuses every call
to a server Hydra did not inject whatever the policy, so the cost is exposure and not
execution, and the final listing reports the server `ready` for the record to keep.
The sequence is pinned by a unit test that drives it without the binary. Measured by hand
on 2026-09-06 against a worktree naming `tests/fixtures/mcp-probe-server.py` and
`/bin/echo` under a scratch `CURSOR_DATA_DIR`: both `not loaded (needs approval)` before,
`hydra-probe: ready` and `broken: Error: Connection failed` after the enables, the
approval written under the scratch directory, and `~/.cursor/projects` untouched.
`tests/mcp.rs` drives the verdict and the refusal record from that listing shape.
**Not measured through `hydra run`**, because `config/mcp.md` admits no server and is
protected config #5; the two real runs each listed nothing, wrote `{"mcpServers":{}}`,
and recorded `injected: []` beside `resolved: []`.

**`mcp.resolved` is the pre-flight's listing, free, and `skills.resolved` is null.**
Ticket 08's note to this ticket said both answers were paid. The MCP half is not: the
listing after the approvals is the provider reporting what the run will start, so the
`ready` names are `resolved`. The skill half is paid, because ticket 24 put it in
`~/.cursor/chats/<hash>/<session>/store.db` after a real run, so the launcher says so on
stderr and writes null, which contract 5.3 makes optional on this provider, and the
third collision test does not run. The other two do, against six user-scope roots
rather than `claude`'s one, and `~/.cursor/skills-cursor` holds twenty-odd of Cursor's
own skills on this machine, so a store skill named `review` or `loop` would refuse a
Cursor launch today.

**`CURSOR_DATA_DIR` moves more than the approval state, and all of it is reaped.** 6.5
says the redirect moves `projects/<slug>/` with `mcp-approvals.json` and
`mcp-disabled.json`. Measured after the first run, the run's `cursor-data/` also held
`mcp-auth.json`, `repo.json`, `worker.log`, a `worker.sock` under a second project
directory, and `agent-transcripts/<session>/<session>.jsonl`, the transcript the hook
payload's `transcript_path` points at. `~/.cursor/projects` gained nothing across both
runs. What did not move is what 6.5 said would not: `~/.cursor/chats/<hash>/<session>/`
gained a `store.db` for each run, so the harvest source of section 10 is where the
design expects it and the per-project transcript is a second copy that dies with the
spawn directory under `hydra reap`. The variable's name lands in `env_allowlist`,
because contract 5.3 records the names passed and this one was, and its value travels
to the detached half in the launch spec as a path Hydra computed rather than a value
the launcher inherited.

**A `cursor-agent` run needs a checkout of Hydra's own, and two things about the
parent repo refuse it before a run id exists.** The hook config, the MCP config and the
skill links are all read from cwd, and cwd is the worktree, so a run outside a git repo
would have to write `.cursor/` into a directory Hydra does not own. Harness spec 2 says
a provider Hydra cannot gate is not launched, so the launch stops there with an error
and no record, the same way a wrong PATH does. And a repo that already carries
`.cursor/hooks.json` or `.cursor/mcp.json` is refused the same way, because the worktree
is seeded from it and Hydra has to write both files: writing over the human's copies
edits their work and `hydra reap` would commit the edit, and loading theirs launches an
ungated run. `write_spawn_config` and `mcp::write_config` refuse a checkout that holds
either file as the second line. 6.5's `--shared-tree` is the same shape one flag over,
and `mcp::plan` takes whose tree the run stands in so that a Cursor run with no tree of
Hydra's admits no server, which is what `Checkout::None` compiles to today.

**The two files Hydra writes into `.cursor/` are ignored by name.** The skill root
ignores itself with `*` (6.4); `.cursor/` cannot, because the human's repo may track its
own `.cursor/rules/` and a `*` there would hide a file the child creates from the commit
`hydra reap` makes. So `.cursor/.gitignore` lists `/hooks.json`, `/mcp.json` and itself,
an existing one is left alone, and the first run's reap committed exactly the probe
file and nothing of Hydra's.

**The argv is 6.7's line with two spellings settled.** `-p` is `--print` on this binary
and the prompt is a positional, so 6.7's `-p <task>` is one boolean flag and one
argument, task last. No `--workspace`, because cwd is the worktree and `mcp enable`
ignores the flag anyway. `--force` covers the workspace trust gate research 14 found
as well as the permission layer.

**`permissionMode: "default"` on every init frame is the hardcoded literal research
01 found**, and it still lies under `--force` on this build. Nothing reads it.

## `agy`, the third provider

Ticket 10. `src/provider/agy.rs` holds every fact that is true of that binary,
`src/normalize/agy.rs` reads its records back, and `src/policy/table.rs` carries its
capability table beside the other two. Every claim below was measured on 1.1.26 on
2026-09-06 unless it says otherwise.

**This provider self-updated in the middle of the ticket, which is the ticket's own
warning arriving on schedule.** The probes ran against 1.1.26 and the recorded run an
hour later reported 1.1.27. Nothing measured changed across the move: the fifty-seven
tool names on the init frame are identical, the hook config format still loads, the four
argument field names still arrive, and the gate still refused two calls out of three. So
every number below is a 1.1.26 measurement confirmed on 1.1.27 unless it names one build.

**One paid probe, five free ones, and one recorded run.** The paid probe ran in a scratch
directory under `--model gemini-3.8-flash-low` with a `PreToolUse` hook that logged its
stdin and answered `allow`, and it is where the tool argument names, the stream shapes and
the error strings come from. The free ones are the read-only slash commands: `/hooks`
before and after writing a config, with and without `--add-dir`, and `/skills` with an
injected skill in the tree. Every one came back `num_turns: 0` with every count in `usage`
at zero, which is what makes the gate-mount check below free.

**The recorded run is `01a07915-c9cf-7cc0-b156-204771091a76`**, `read-only`,
`--model gemini-3.8-flash-low`, on 1.1.27. It was asked to read `Cargo.toml`, read
`/etc/hosts` and write a file. The first went through, the second came back
`hydra/out-of-scope` and the third `hydra/capability`, both denials carrying Hydra's reason
verbatim inside the tool error behind the provider's own `tool call denied by pre-tool
hook: ` prefix. The record is 14 lines, `completed` with two `hydra_policy` denies beside
one allow and no finding, `usage_source: provider` at 29,133 input and 353 output, 8.4
seconds from line 0 to `run_end`, and `diff_stat` empty because the one thing the run tried
to write was refused. It passes the contract checker and both corpus walks.

**`permission_mode` reads `always-proceed` under `--dangerously-skip-permissions`**, where
the same field reads `request-review` without it. Nothing reads the field; it is here
because research 26 recorded only the second value and a reader who found the first would
go looking for a bug.

### The hook config format, and a correction to research 26

**The file is a JSON object keyed by hook name.** Research 26 read the `/hooks` listing
and recorded its shape as the input format, which it is not. The binary's own embedded
`Lifecycle Hooks (hooks.json)` reference says each top-level key is a hook name carrying
event arrays, and a `PreToolUse` entry wraps its handlers in a `matcher` group:

```json
{ "hydra-gate": { "enabled": true,
  "PreToolUse": [ { "matcher": "*", "hooks": [ { "type": "command", "command": "..." } ] } ] } }
```

The listing's shape, with `name`, `enabled`, `source` and a flat `actions` array, is the
output. **Written in the output shape the file loads as nothing**: `hooks: []`, no error,
which is how this was found. The handler field is `timeout` and not the listing's
`timeout_seconds`, seconds, default 30, left at the default. The working directory of the
hook command is the directory holding `hooks.json`, so `.agents/`, which is why Hydra's
command names the gate by absolute path.

**`--add-dir` is still required and cwd is still not enough**, re-measured on this build:
the same `hooks.json` in cwd listed as `hooks: []`, and listed in full the moment the
directory arrived on the flag. That is the ticket's worst failure mode, a gate nothing
loaded with nothing anywhere saying so.

**So the gate mount is checked before every `agy` spawn, and this is a departure the
spec does not ask for.** 6.5 gives the fatal pre-flight to Cursor alone and 6.6 only
notes the empty listing. But `agy -p="/hooks"` costs nothing, and the failure it rules
out is a run that looks gated and is not, so `run.rs` runs it after writing the config
and compares the listing's `source` against the file Hydra wrote. A launch that fails it
writes a record through `never_started` and derives `failed`, which is the shape a spawn
that could not fork already gets: it is not one of contract 6's four refusals and it is
not dressed as one. The check is on the source path rather than the hook's name, because
a hook of Hydra's name coming out of somebody else's file would pass a name check and
gate nothing.

### What the gate reads, and the four field names it was given

**The payload is a different shape from the other two providers'**, camelCase protojson
with the call nested under `toolCall` and no `cwd` field at all. `gate.py`'s
`as_tool_call` unwraps it and takes the cwd from `args.Cwd` when the tool sends one, then
from `workspacePaths[0]`, which is the directory Hydra passed on `--add-dir`. It keys on
the payload rather than on the dialect, so the same script answers a live hook and
contract 7.7's read-time replay, which sends the normalized shape for every provider.

**The seven measured argument names**, off real hook payloads:
`run_command(CommandLine, Cwd)`, `view_file(AbsolutePath)`, `list_dir(DirectoryPath)`,
`write_to_file(TargetFile, CodeContent, Overwrite)` and, since 2026-09-08,
`grep_search(Query, SearchPath)`, `find_by_name(Pattern, SearchDirectory)` and
`replace_file_content(TargetFile, ...)`. The rest of the table's path fields come off the
binary's own `jsonschema` struct tags, which is the same class of evidence Cursor's rows
come from: `TargetFile` for `multi_replace_file_content` and `sed_file`, `NotebookPath`
for `notebook_edit`. A wrong field name here is a `hydra/no-path` deny on every scoped
policy, which fails closed and makes the provider unusable rather than unsafe, and
`tests/gate.rs` drives the measured seven both inside and outside the worktree.
**The init frame advertises tools the binary then refuses to run.** Run 01a0801d-77b1,
`gemini-3.8-flash-high` on 1.1.27, was asked to call `sed_file` and `notebook_edit`; both
names are on that run's own init list, the gate answered each by name, and the binary
answered `unknown tool: "sed_file" — check spelling` and the same for the other. So the
fifty-seven names are a superset of what a given model can call, the two rows stay in
the table because the gate speaks before the binary does, and the payload the model
improvised for a tool that does not exist (`sed_file` with `command`, `pattern` and a
lowercase `path` beside `TargetFile`) is not evidence of anything. **`grep_search` was that wrong name for two days.** The table said
`SearchDirectory`, read off the tag on `find_by_name`'s struct, and the first `agy` run
under `standard`, 01a07ffe-4808 on 1.1.27, had its first search denied `hydra/no-path`.
The binary's own description of the glob filter names `SearchPath` as the search root.

**Fifty-seven tools on the init frame**, against research 20's fifty-six on a Claude id
two days earlier; `list_permissions` is the difference and whether it is new or
model-dependent is unmeasured. Like `claude`, this provider lists them all at process
start, so the normalizer compares the list against the table and a rename comes back as
a finding on the first record written after it.

**MCP is a tool name here rather than a prefix.** `call_mcp_tool` is a row in the table,
so `Table::mcp_prefix` became an `Option` and this provider's compiled config carries
null. An empty string there would have matched every tool name and called the whole
provider an MCP call.

**Three of the fifty-seven are `shell` and carry no command for the floor to read.**
`send_command_input` writes stdin to a command already running, `command_status` collects
its output, and `notebook_execution` runs the notebook's code. They are `shell` so that
no policy denying shell hands a run half of it, and 7.4's named failure mode applies to
all three. `execute_browser_javascript` is worse and is `provider_native`: it runs code
the model wrote, which only `permissive` grants, and the floor cannot see inside a
JavaScript string in a page.

### The stream, and what the adapter makes of it

**One frame type carries almost everything.** `step_update` is the message, the tool
call, the tool result and the usage, told apart by `step_type` and `state`. `init` has
the model, the conversation id and the tool list; `result` closes the run.

**`text_delta` is a buffered flush** (contract 7.3), and the adapter concatenates over
`step_index` into one `message` with the step index on `Message::block`. Measured: an
`agent_response` step whose whole job was to call a tool carries no `text_delta` on
either frame, so a step with nothing to concatenate produces no message rather than an
empty one.

**The pairing is `step_index` because there is no id**, so the `call_id` is the step
index as a string: unique inside one run and meaningless outside it.

**The stream's `tool_info.parameters` is a subset of the hook's args.** The hook payload
for `write_to_file` carried five fields and the frame carried `TargetFile` alone;
`run_command`'s hook payload carried `Cwd` and the frame did not. Every path field the
gate reads survives, which is what keeps 7.7's re-evaluation answerable, and the `Cwd`
that does not survive is the run's own checkout on every launch Hydra makes.

**Usage is summed and no frame names it** (contract 9.2). Measured twice. The paid
probe's four `agent_response` steps summed to input 41,945, output 455, cache read 16,254
and total 42,400, and its `result.usage` reported exactly those four; the recorded run's
two steps summed to 14,072 plus 15,061 input and 269 plus 84 output, and its `result.usage`
reported 29,133 and 353. So on a fresh run the two agree, and 9.2's warning is about a
resumed one, where `result` carries the predecessor's tokens as well. The single `Usage`
event therefore carries real counts with `seq: None`, a combination no other adapter
produces: there is no one frame to point at, and naming the last step would point at a
frame that reported a fraction of the total.

**`text_delta` split mid-word on the recorded run**, between `...204771091` and
`1a76/Cargo.toml...`, in the middle of a file URL. Research 20's one sample split at a
newline, which left open the reading that the frames are whole lines. They are not. The
concatenation over `step_index` is load-bearing rather than tidy, and a reader that treated
each frame as a message would report two messages, one of them a truncated path.

**`cache_write_tokens` stays absent rather than becoming zero**, which section 9's table
asks for, and `thinking_tokens` was 0 on every step of a non-thinking model.

**Contract 7.6's `unclassified` earned its place in one run.** `write_to_file` failed
with `declaring permissions: cortex tool write_to_file: ... is not a valid artifact
path` and `run_command` failed with `permission check failed for command ...: user denied
permission to run command`. Same `state: ERROR`, same `error.type: TOOL_ERROR`, both
containing the word "permission", and one is a headless refusal while the other is the
provider rejecting an argument. So the adapter classifies an error only when it carries
Hydra's own `hydra/<code>:` prefix, the provider's `denied by pre-tool hook` marker, or
the `jsonhook__` id it names a failing hook by; everything else is `unclassified`. That
changes one finding: an `unclassified` result under a policy that denies the call is
**not** reported as a gate leak, because the shape a denial and a failure share is no
evidence either way. A `Finding::Limit` says so instead.

**`agy` reports how its run went in a field of its own.** The terminator carries `status`,
`SUCCESS` or `ERROR`, with an `error` string beside it, and no `is_error` at any level;
measured on a slash command it refused in print mode. `outcome::derive` reads it on this
provider alone, because `claude` puts `subtype` in the same role and it says `success` on
a 429. `SUCCESS` on its own is worth nothing: ticket 26 measured a run reporting it with
every tool call denied and exit 0, which is why `refused_everything` is checked first.

**The provider aborts its own turn with "timeout waiting for response", and the run
derives `failed`.** Measured 2026-09-08 on 1.1.27, twice, the second time on purpose. Run
01a07ffe-4808, `gemini-3.8-flash-high` under `standard` on build ticket 31, read fifty-five
steps and 3.2M tokens of source, then ran `cargo test --release` in a cold worktree; 104
seconds into that step the `result` frame arrived with `status: ERROR`, `error: "timeout
waiting for response"`, an empty response, exit 1, and the tool's step never reached `DONE`.
The cutoff is not the tool's wall clock: run 01a08004-c497, the same binary asked to run
`sleep 150 && echo slept`, waited 150 seconds on the one step and came back `SUCCESS`. Nor is
it `--print-timeout`, which the run argv does not pass and defaults to five minutes. What
differed was the context: a quarter of a million input tokens with a compile's output on the
way in. The honest reading is a model-response wait the binary does not expose as a flag.
Two things follow for whoever launches this provider. A Rust ticket that has to run the
suite is out of its reach on this build, whatever the model, and the two `agy` failures on
this map's memory, one on `gemini-3.1-pro-high` on 2026-09-07 and this one, are the same
death. And a short closed probe is well within it: run 01a08008-cb50 measured a field name
in 48K tokens and two calls. The outcome is `failed`, which is correct in the sense that the
provider's own status says so and wrong in the sense that rung one will learn a skill lesson
from a wait, the same shape contract 8 warns about for a rate limit; no rule keys on the
sentence yet, and it is written down here so the next reader recognizes it.

**A rate-limited or logged-out `agy` run has no measured shape.** `provider_refused`
keys on `api_error_status`, which this provider does not write, and nothing was measured
on either path. So a run refused for quota comes out `failed` today, which is the
mistake contract 8 exists to prevent, and it is a gap rather than a decision.

### What it writes, where, and what it refuses

**`.agents/` holds three of Hydra's files and the ignore file lists two by name.** The
skill root at `.agents/skills/` ignores itself with `*` (6.4), and `.agents/.gitignore`
lists `/hooks.json`, `/mcp_config.json` and itself, for the reason Cursor's does: this
provider reads `rules/`, `plugins/` and `workflows/` out of the same directory, and a
`*` there would hide a file the child created from the commit `hydra reap` makes.

**The MCP config is `<worktree>/.agents/mcp_config.json` with `serverUrl`** on a remote
server, which is this provider's own spelling of what Cursor calls `url`. A server naming
both a command and a url cannot reach the writer: `mcp::Document` refuses that pair at
parse time with `agy` in the error.

**A run outside a git repo is refused before a run id exists**, the same as
`cursor-agent`, and `worktree_preconditions` is now one function for both: no repo means
nowhere Hydra owns to put a gate, and a repo already carrying the provider's own
`hooks.json` or `mcp_config.json` is refused too.

**Both free probes run under the environment the run will get, and both are bounded by
the provider's own flag.** `ChildEnv::build` is called inside the probe the way
`claude::resolve` calls it, because inheriting Hydra's environment would hand the probe the
`GEMINI_API_KEY` on line 1 of `~/.zshrc` and let it authenticate by a route the child
cannot: the answer would then be about a different process than the run. Measured after the
scrub: the `/skills` listing still returned all eleven names, so this provider reads the
keychain under `HOME` and the key was never what authenticated it. `--print-timeout 30s`
bounds the print-mode wait, because both probes sit on the foreground launch path after the
worktree exists and a hang there would leave an orphan checkout and no run id to type. A
hang before print mode begins is still unbounded, which is a gap `cursor-agent`'s
pre-flight shares.

**The skill listing is free and the MCP set has no free route at all.**
`agy --add-dir <worktree> -p="/skills" --output-format json` answered with one record per
surviving skill, keyed on the frontmatter `name`, with the injected probe skill among
them, for zero tokens. The MCP half is worse than research 23 found: on 1.1.26 `/mcp` is
refused in print mode outright, `/mcp is not available in print mode (it opens the
interactive MCP server view)`, so the four paid turns that ticket spent are no longer
even available. `mcp.resolved` is null on every `agy` record, which contract 5.3 requires
a reader to treat as unanswered rather than as an empty listing.

**The environment is the base four and the proxies.** No `NODE_EXTRA_CA_CERTS`: 6.2 gives
it to "the two Node providers" and this binary is a 177 MB arm64 Mach-O built from Go.
No `SHELL` or `TMPDIR`, which 6.2 gives `claude` alone. `HOME` matters more here than
anywhere, because this provider hard-fails without it and every path it uses is hardcoded
under `~/.gemini/`, which is why 6.2 rules out the only lever there is.

**6.5's inherited-server residue is this provider's alone and nothing changed it.**
`claude` closes it with `--strict-mcp-config` and Cursor under the data-directory
redirect; this one has neither, and Hydra still never calls `agy mcp disable`, because
that command is global rather than per run.

### 19.3's three unverified facts, still three

None of the three moved, and one gained a neighbour.

`--effort` exists on this build with `low|medium|high` and is documented as "Reasoning
effort for the current CLI session". Whether it does anything to a Claude model id needs
two paid runs on such an id and was not spent. **Caveat.**

`thinking_tokens` was 0 on every step of the measured run, on `gemini-3.8-flash-low`,
which is a non-thinking model and therefore says nothing about whether the field is
unwired for Claude ids. **Caveat.**

The 16,338-token floor on a resumed `--conversation` run was not measured, and neither
`--conversation` nor `--continue` is reachable from Hydra, which mints a fresh run id per
resume.

**What did land is a pattern nobody has an explanation for: `cache_read_tokens`
alternates step to step inside one run.** Across three runs, every step that reported a
cache read reported about 8,120 of them beside roughly 6,000 input, and every step that
reported zero reported roughly 14,000 input. The paid probe alternated 8,131 / 0 / 8,123 /
0 across four steps; the recorded `read-only` run reported zero on both of its steps; a
third run reported zero on three steps and 8,120 on the fourth. So the two numbers sum to
about the same prompt either way, which points at a cache hit being reported rather than
avoided, and it is not keyed on the conversation, the run or the clock. **Caveat, and one
worth a ticket if anyone ever wants to cost an `agy` run.**

## `codex`, the fourth provider, since build ticket 48

Built on 2026-09-20 from map ticket 58's measurement of `codex-cli` 0.154.0, then measured on
2026-09-21 on a logged-in binary across four recorded runs and ten direct probes. What the
measurement changed is in the paragraphs below, and every `MEASURE 48.5` marker is gone.

**The hook rides the command line, as a file.** `src/provider/codex.rs` composes `codex exec
--json -C <worktree> --sandbox workspace-write -c approval_policy="never"
--dangerously-bypass-hook-trust -c hooks.PreToolUse=[...]`, the last override naming
`/usr/bin/python3 <spawn>/hook.py claude`, and `write_spawn_config` writes that file from
`policy::hook_source`, the same Python the other three dialects carry inline. Three measurements
forced the shape. `exec` has no `--ask-for-approval` flag, that one is the interactive command's,
and the first two launches died on it with exit 2. A `<repo>/.codex/hooks.json` fires in a plain
checkout and is never read inside a git worktree, from the worktree or from its main checkout,
and every Hydra run is a worktree: run 01a0c223-9ec2 ran `cat ~/hydra/runs/nothing.jsonl` with
the file in place and the gate silent. The `-c` override is read in a worktree, but its value is
TOML and a basic string cannot hold the newlines the inline command carries: run 01a0c222-4305
died at config load with `invalid type: string ... in hooks`. Run 01a0c231-6583 is the one
where all of it holds: `apply_patch` refused `hydra/no-path`, the read of `~/hydra/runs/`
refused `hydra/protected-path`, the shell write and `ls` allowed, two deny decisions by
`hydra_policy` on the normalized record and no finding. The `.codex/hooks.json` in the worktree
is still written, for a launch with no checkout, and the doc comment says it is unread otherwise.

**A hook that exits non-zero with no JSON answer fails open**, probe p3: the marker was written
and the command ran. The sha check in `hook.py` therefore answers a tamper with a deny in
`claude`'s dialect rather than an exit code, and a Python crash inside the gate is the fail-open
`SECURITY.md` names for this provider.

**A denied call leaves no `item.*` frame.** The only trace is one stderr line, `ERROR
codex_core::tools::router: error=Command blocked by PreToolUse hook: <reason>. Command:
<command>`, with a patch body continuing on the lines after it. `src/normalize/codex.rs` reads
that line into the call, an error result carrying the reason, and a decision; a run refused
throughout reads `gated` the way it does elsewhere. Everything else on stderr is noise, the
hook-trust warning included, which arrives twice per run as an `item.completed` of type `error`.

**Field names.** The session id is `thread_id` on a `thread.started` frame. A shell call is
`command_execution` with `command`, `aggregated_output`, `exit_code` and a `status` of
`completed` or `failed`, and `command` is what the provider ran, `/bin/zsh -lc '<what the
model wrote>'`, while the hook judged the bare inner text: the reader takes the wrapper off
before the replay, because a protected path inside a quoted token is the shape build ticket
56 documented the token pass does not see, and the wrapped form derived allow on run
01a0c223-9ec2 for a call the live gate refuses; an applied patch is `file_change` with `changes: [{path, kind}]` and
never the patch text, which the hook alone sees under `tool_input.command`. `turn.completed`
carries `usage` with `input_tokens`, `cached_input_tokens`, `cache_write_input_tokens`,
`output_tokens` and `reasoning_output_tokens`. The stream never names the model; the hook
payload does, `gpt-6-astra` on this account, and the record's `model` stays null unless
`--model` was passed. `resolve` returns null for `skills.resolved` and `mcp.resolved`, because
no free probe exists yet.

**A `codex` turn that fails ends on `turn.failed`, and since build ticket 100 that frame is the
run's terminator.** A message containing "You've hit your usage limit" derives `rate_limited`,
and any other `turn.failed` derives `failed`. This is a text match, like `cursor-agent`'s. Three
records on this machine read `rate_limited` now: 01a0cfdb-e5c3, 01a0d02d-f812 and
01a0cc9d-8f15. The first two derived `failed` before, so rung one was learning from a quota
failure, the third hiring panel's H4. The last one finished its answer before a compaction step
hit the limit, and derived `completed` before. A top-level `error` frame is noise, and
`thread.started`, `turn.started`, `item.started` and `item.updated` are dropped by name. Any
other frame type the reader does not know raises one `Limit` finding per type instead of
vanishing.

**Routable under `standard` and `permissive` only.** Codex has no read tool, so under `read-only`
and `judge` every read would be a shell call the policy denies; the human decided on 2026-09-20
that those two policies refuse a `codex` launch at pre-flight, in Hydra's words, naming ticket 48,
and that a bounded shell read for `codex` alone was map ticket 63. Map ticket 63 closed on
2026-09-21 on a fact it did not have: `judge` grants no tool on any provider, so a `codex` judge
needs no read. `judge` opened to `codex` with build ticket 61, the `evolve` panel became four,
and only `read-only` is still refused. The auth pre-flight, `codex login status`, is the first
`auth_preflight` refusal any provider writes.

**`apply_patch` is a pathed write since build ticket 69, and was refused `hydra/no-path` under
`standard` before it**, measured on 01a0c231-6583 and again on the first run of build ticket 63,
which could not write at all and stopped. `parse_patch_paths` in `gate.py` reads every `*** Add
File`, `*** Update File`, `*** Delete File` and `*** Move to` header out of the body and the
scope check runs over all of them, refusing on the first outside; `policy/mod.rs` compiles the
row's `patch_paths` flag into the gate config; the replay reads `changes[].path` off the stream.
A body with no header is still `hydra/no-path`. Where Hydra's seatbelt wraps the launch, `codex`
runs with `--sandbox danger-full-access`, because its own sandbox is a seatbelt too and a second
profile cannot be applied inside Hydra's: every command on run 01a0c62d-b98d died with
`sandbox_apply: Operation not permitted` until then, the allowed ones included.

## What the record does not carry

**Both injection groups are written now.** Ticket 08 closed the last one: every recorded
`claude` run carries `mcp` with `doc_sha`, `injected` and `resolved`, so the section
this one replaces, which said `mcp` was absent because nothing MCP was injected, no
longer describes anything. On `cursor-agent` both groups are present too, and one half
of one is null on purpose: `skills.resolved` is the paid listing, so the record says
nobody asked rather than claiming an empty one, and `mcp.resolved` is the pre-flight's
own listing, which is free. The section on that provider has the reason for each.

**`skills` is present on every run with a checkout and absent on the rest.** Ticket 07
closed that half: a run in a repo carries `store_sha`, `linked` and `resolved`. A run
outside a repo gets no worktree, and 6.4 puts injection in the worktree and nowhere
else, so there is no location to link into and the question does not apply.

**`mcp` is present whether or not there is a checkout**, and the asymmetry is 6.5's
rather than an oversight here. On `claude` the servers ride the command line, so a run
outside a repo still gets exactly the set Hydra wrote. A record with `mcp` and no
`skills` is that run.

Ticket 02 closed the other two. `policy` and `policy_sha` carry the policy that was in
force and the sha of the directory it was compiled from, and the preamble rides the task
string, so `run_start.task` is the preamble followed by a blank line and the caller's
task verbatim.

**`--allow-floor-escape` is on the run record, since build ticket 15.** Harness spec 16
says a per-run human flag "lands in the run record as an argument, so the record shows
why a run was allowed to do the thing", and `--allow-billed` has `auth.allow_billed` for
exactly that. The floor flag had no field until map ticket 42 named one, because
inventing a field is a contract change and ticket 03 left the gap where it found it.
`run_start.floor_lifted` is a boolean now, filled from `Request::floor_lifted` on every
recorded run and null on a harness run and on an interactive run without `--policy`; a
gated interactive run, build ticket 28, compiles a gate and writes `false`.
It is a fact about the request rather than about the compile, so a refused launch carries
it too. `RecordedGate` in `src/normalize/mod.rs` reads it back and hands it to the
replay's `Spawn`, and a record without it re-derives as if the floor was in force, which
is what every record on this machine does.
`tests/normalize.rs::the_floor_lift_on_the_record_reaches_the_replay` is the pair: the
same `git push --force` derives allow on a record that carries the flag and `hydra/floor`
on one that does not.

**The floor's `git` parser consumes the value after `-c` and `--config-env`, since 2026-09-21.**
`git_subcommand` in `gate.py` skips the values of the flags `consume_before_subcommand` in
`policies/floor.json` lists before it reads the subcommand, and until build ticket 58 that list
had `-C`, `--git-dir`, `--work-tree`, `--namespace` and `--exec-path` and not the two config
flags, so `git -c core.pager=cat push --force origin main` was read as the subcommand
`core.pager=cat` and walked past every push rule; the adversarial review of 2026-09-14 measured
it. The two entries are protected config #1 and the human added them; the test in `tests/gate.rs`
was written first and sat under `#[ignore]` for six days. The edit moved `policy_sha`, and every
record before that day carries the old one, which is the case `policy_moved` exists for.

**Every `claude` thinking block written before build ticket 17 is empty, and the count
beside it is right.** Contract 7.5 says `claude` gives thinking text with a signature, and
43 blocks across the 27 records on this machine on 2026-09-07 held a signature and an empty
string, while the 35 `cursor-agent` blocks all held text. Claude Code 2.1.72 started
sending a `redact-thinking-2026-02-12` header that empties the block before it reaches
the stream. Four probes on haiku on 2.1.263, same task, `--print --output-format
stream-json --verbose`:

| argv | text chars | `thinking_tokens` |
|---|---|---|
| nothing | 0 | 190 |
| `--settings '{"showThinkingSummaries":true}'` | 504 | 266 |
| `CLAUDE_CODE_DISABLE_ADAPTIVE_THINKING=1` | 0 | 181 |
| both | 272 | 173 |

The env var does nothing and the key is the whole fix, so `claude::settings` carries it
beside the hook config on every policy. Two caveats travel with it. The text is the
summary Anthropic generates and not the trace, which is all any Claude 4 model emits.
And it is per model: run 01a07efe-97f4 on `claude-sonnet-5` on 2026-09-07, with the key in
force, wrote 35 thinking blocks and every one of them empty, while Haiku 4.5 wrote text on
every run since. Issue 56356 on anthropics/claude-code reports the same on Opus 4.7. So the
text is measured on Haiku 4.5 and on nothing else, and the drift test runs on haiku. The setting has regressed
twice in the binary's history, at 2.1.8 and 2.1.72, which is why `tests/drift.rs` runs
a paid probe behind `HYDRA_DRIFT_PAID=1` and names the version in its failure.

Everything else in contract sections 2 to 6 is met, and `tests/record_contract.rs`
checks it rather than asserting it. That test also walks every record under
`~/hydra/runs/` on this machine and fails if any of them drifted.

## Reading a record back

**Four providers, since build tickets 10 and 48.** `src/normalize/` turns a stored
`claude`, `cursor-agent`, `agy` or `codex` run into
contract 7's events, one adapter per column of 7.2, picked by the envelope's `provider`.
`read_file` takes a path and the policy set; `read_with` takes text and a layout, which
is how the tests ask a real question about a synthetic run without touching `~/hydra`.
Nothing is written, which contract 11 requires and `tests/normalize.rs` checks by
comparing every byte under `runs/` before and after a walk. What follows in this
section is the `claude` adapter; the `cursor-agent`, `agy` and `codex` sections below have
the others.

**`usage` says `unavailable` for an object it cannot read, not zero.** A `result` frame
with no `usage`, and one whose `usage` holds none of section 9's four count fields, both
produce `source: unavailable` with no counts. Zeroes there would be 7.8's
null-against-zero confusion arriving through the other door.

**Eight of the ten types have a producer.** `message`, `thinking`, `tool_call`,
`tool_result`, `permission_decision` and `usage`, plus the two stored Hydra lines.
`message_delta` needs `--include-partial-messages` and 7.4 keeps it off in v1.
`correction` comes from an interactive harvest, built since ticket 27 and read by the
interactive section below. Both types are in the enum, because a type the reader cannot name is a type nobody
knows is missing.

**`permission_decision` runs the run's own policy again**, compiled into the same gate
script 7.5 writes and asked about each recorded call. Two costs, both accepted: a
`/usr/bin/python3` process per record, and that interpreter is now a read-time
dependency as well as a run-time one. It was one process per tool call until build
ticket 86 added a batch entry (`<dialect> batch`, a JSON array in and one answer per
payload out) that runs the same `decide_payload` the hook runs; a record with 3,770 lines
went from 10.3 s to 0.16 s in `hydra show`, with identical output. The live hook never
passes `batch`, its argv is built by Hydra. The batch entry sets `PWD` from each payload's
`cwd` inside that payload's `try`, the single-call entry keeps the environment it
inherited, and a batch that fails as a whole is asked again one call at a time. A second implementation of the rules in Rust was
the alternative and it is the wrong trade. The gate decides with the deny list,
the scope resolution, the `mcp__` prefix rule and a floor that tokenizes shell with
`shlex`, and a Rust copy that answered `allow` where the gate answered `deny` would
report a gate leak that never happened.

**`decider` reads the `hydra/` prefix and nothing else.** Every reason the gate writes
starts with the stable code ticket 01 added, so a `PreToolUse` deny whose reason carries
no prefix belongs to somebody else, which on this machine means the user's own guards.
7.7 forbids parsing reason *text* for attribution and this is not that: the prefix is a
code Hydra writes, the prose after it is what 7.7 is about.

**A hook frame carries no `tool_use_id`, so the corroboration was joined by tool name and
order, inside a window; since build ticket 22 the join is by `hook_id`, in its own section
below, and what follows is the fallback for a record with no `hook_started` frames.** `hook_name` reads `PreToolUse:<Tool>` and the frame lands
between the `tool_use` block and its result. Ticket 02's run fired four `PreToolUse:Bash`
responses for one call, one Hydra's and three with an empty `output`, so filtering on a
parseable `permissionDecision` picks Hydra's out. **The window closes at the next call
that came later**, and that bound is load-bearing rather than tidy: without it the first
of two `Bash` calls takes the refusal a guard wrote about the second, and both decisions
come out wrong, one denied for somebody else's reason and one allowed. Two calls inside a
single assistant frame share a `seq` and so share a window, and they are handed answers
in hook order, which is an assumption and not a measurement. That last part reaches only
the disagreement check, never the decision.

**The record carries its scopes, and the rebuild is the fallback.** Build ticket 15 landed
map ticket 42 on 2026-09-06: `run_start.scopes` is six absolute paths keyed by scope name,
`worktree`, `skill_store`, `skill_checkout`, `corpus`, `gate_script` and `parent_git_dir`,
written from the same `Spawn` the gate was compiled from. `Spawn::paths` in
`src/policy/mod.rs` is that one function, and `compile`'s own `scopes` block and
`SpawnFiles::scopes` are two views over it, so the script and the record cannot name
different paths. `skill_snapshot` is not a key, because it recomputes from
`skills.store_sha`, which line 0 already carries. Two of the six are not scopes at all:
`gate_script` and `parent_git_dir` are per-run denied paths, and they are in the same
object because the record is the only copy of either. The contract's own key list stays a
list of keys and says nothing about that split, which is why it is here.

**No refused spawn carries the object, and all three carry `floor_lifted`.** The policy
narrowing check of build ticket 11, the skill collision of 6.1 item 5 and the MCP
shortfall of item 6 all refuse before `write_spawn_config` runs, so no gate was compiled
and no scope was resolved: the key is absent rather than filled with what the compile
would have produced. The flag is different, because it is a fact about the request and a
human typed it whether or not the launch got that far.

**A replay recovers the scopes and not the denials.** `Spawn` carries the six paths above
as arguments and the rest of 7.5's deny list off `layout`, which on the read side is the
reader's own `~/hydra`. So a record read on the machine that wrote it loses nothing, and a
record from another root resolves its scopes against the stored paths while denying
against the reader's directories. That is half the compile recovered rather than all of
it. Carrying the whole deny list would mean putting a dozen paths on every line 0 to
recover a case that only arises when `~/hydra` moves, and ticket 42 costed the six at 213
bytes on a 1,759-byte median line 0 before deciding what to store.

`RecordedGate` in `src/normalize/mod.rs` reads the object off line 0 and `ReplaySpawn`
prefers it key by key. A key the record does not carry is rebuilt the way build ticket 14
left it: `worktree` off `run_start.worktree.path` and otherwise the launch's own
`spawn/<run-id>/no-worktree` placeholder; `corpus`, `skill_store` and `gate_script` from
the reader's own `Layout`; `skill_checkout` from the record's `policy` field, `Some` on
`curate` alone; and `parent_git_dir` from `git rev-parse --git-common-dir` inside the
record's worktree, through the same `crate::worktree::git_common_dir` the launch calls
from the parent. Not `repo_root` on the record's `cwd`: that `cwd` is the worktree itself,
`--show-toplevel` answers with the worktree there, and `.git` under it is a pointer file
the carve-out then owns, which is what the first version did until the spec review caught
it. A worktree `hydra reap` has taken answers nothing, which loses a `GateLeak` it could
have reported and never invents one, and that is the case the stored path closes: ticket
14 called `parent_git_dir` the one scope only the record can keep, and the record keeps it
now.

**A rebuilt key is one `Finding::Limit` per record and never a disagreement.**
`ReplaySpawn::scope_limit` names the keys it stood in for, and adds that nothing answered
for `parent_git_dir` when the git call came back empty. `src/rungs/corpus.rs` routes
every `Limit` to `signals.limits`, which the ranking does not weigh, rather than to
`signals.disagreements`, which weighs 2 and reads in the brief as "the record and the
policy disagree". Getting that arm wrong is the failure ticket 42 measured: one clean
record would have arrived at the top of a pass's brief as a run whose gate leaked twenty
times. Every record on this machine predates the field, so every one of them with a
decision to derive reports the limit once, and the corpus walk in `tests/normalize.rs`
asserts it stays at once.

**A record with no `scopes` can no longer report a gate leak, and that is the price
decision 5 named.** A `GateLeak` and a `Disagreement` both claim two sources answered one
question and differed. Rebuild a path the question resolves against and they did not, so
`ReplaySpawn::stood_in` turns both into a limit that names the rebuilt keys, the way
`policy_moved` turns a disagreement into one when the sha moved. Every record on this
machine is in that state, so the corpus-wide leak check now proves less than it did on
2026-09-06: it would take a leak on a record written after this build to fail. What kept
it worth running is that the check is unchanged for records that carry their scopes, which
is every record from here on, and `tests/normalize.rs` reaches both a `GateLeak` and a
`Disagreement` through `Record::gated` fixtures that carry them. This is the one place
where a decision on the map made the reader quieter rather than louder, and ticket 42
chose it with the alternative measured: twenty findings on one record whose gate leaked
none.

`Layout::skill_store_scope`, which used to make the `curate`-or-not decision for
`skill_store` by itself, is gone, and `skill_store` means the same path under every policy
on both sides, so a curator's record does not come back as a gate that leaked for the
store's sake, only for its checkout's.

**The replay's no-worktree fallback used to disagree with the launch's, and the
carve-out is why that now matters.** A run outside a repo has no checkout, and the live
gate scopes `worktree` at `~/hydra/spawn/<run-id>/no-worktree`, a path nothing creates.
The normalizers used to fall back to the record's own `cwd` instead, which cost nothing
while `worktree` was the only thing a run owned outright, but a fallback that wide would
let a replay treat the whole working directory as the run's own now that a spawn can own
a checkout too. The three normalizers mirror the launch's fallback since build ticket 14.

**`policy_sha` does not cover the gate script, and `gate_sha` does.** `policy_sha` hashes
`policies/`, and the rules that read a policy live in Rust and in `src/policy/gate.py`.
Four times, three of them on 2026-09-12 and one on 2026-09-14, a change to that code turned
every record written before it into a gate leak while `policy_sha` stood still. Build
ticket 49 put `gate_sha` on `run_start`: the sha256 of the compiled script's bytes, which
carry the policy, the deny list, the scopes the spawn resolved and the Python itself.
`src/run.rs` hashes the file it wrote; `policy::gate_sha` compiles the same bytes without
writing them, and contract 7.7's re-evaluation calls it once per record to ask whether the
gate it is about to compile is the gate that answered.

Three marks come out of that, in `GateMark`. `Todays` is the record whose sha is the sha
this build compiles, and only that record can report a `GateLeak` or a `Disagreement`.
`Foreign` names a gate whose code has changed since. `Unnamed` is every record written
before the field, which is all 207 on this machine today, and every run that compiled no
gate. The last two downgrade the finding to a `Limit` that says which mark is missing.

**A matching `gate_sha` does not make the replay a pure function of the record.** The gate
resolves each path with `realpath` against the disk as it is when the record is read, not as it
was during the run, so a symbolic link that changed or went away since can change the answer
under the same hash. And `src/policy/replay.rs` runs the gate with the reader's own environment
and interpreter, setting only `PWD`, while the gate expands `~` and variables from its
environment. `GateMark` and `cannot_back` downgrade a foreign gate or rebuilt scopes and nothing
else, so a `Disagreement` on today's gate can come from either. Recording the resolved path with
each live decision would close the first; it is not built. The third hiring panel's H42.

`ReplaySpawn::cannot_back` is the one door all seven call sites in the three normalizers go
through, so the adapters cannot drift apart on which question they ask, which is one axis of
build ticket 55 closed. It names every reason rather than the first: a record can lack the
gate mark and have had its scopes rebuilt, and a reader fixing one wants to know about the
other before re-running.

**One place still writes `gate_sha: null`.** The harness record `hydra evolve` writes compiles
no gate at all, and null is the truth there.

**`parent_run_id` is lifted off the envelope onto the normalized run.** Contract 4
puts it there rather than in the body, because the parent writes it at spawn and
nothing reconciles it against the child. Rung one reads it twice, for the retry
class and to leave out the runs a curator pass spawned, so a reader that had only
the body would have neither. Ticket 11 gave it a producer: a run launched with
`--parent`, or through the delegate face with one, carries it on every line.

**Findings are not events.** An event says what happened; a finding says two sources
disagree or that the derivation could not do its job. Seven of them: a gate leak,
subagents spawned under a policy that denies them, a disagreement between the derivation
and the gate's recorded answer, a gated call the gate never answered, a tool name the
capability table does not know, a `policy_sha` that is not today's, and a bare limit for
anything else. The fourth is the fail-open case 6.6 measured: a hook that will not start
lets the call through on this provider, `--include-hook-events` is on for every recorded
run, so a call with no gate answer and no refusal in Hydra's own voice is one that reached
the model with nothing in front of it. Four things produce a `Limit` and two of them matter most. A file with
no `run_start` reads exactly like a pre-gate run, since both carry a null policy and no
decisions, so the derivation says which one it is looking at. And a record that stops
between a `tool_use` block and its result leaves a call whose fate nothing recorded, so a
policy that denies it reports a limit rather than a leak: "the file ends here" is not
evidence that the call went through. Ticket 14's abandoned-run class is exactly that
shape, and so is the tail of a killed run: the writer syncs per line, so a torn last line
costs its own frames and a limit rather than the whole read. The other two are a record
that names a policy and carries no `policy_sha`, whose decisions then answer for today's
`policies/` rather than the run's, and a `tool_use` block with no id, which nothing can
join a result or a hook answer to. The decision still follows the policy in both. 7.1's table check reads the
`system`/`init` frame, which lists every tool at no cost. On 2026-09-06 `claude` 2.1.263
offered twenty-nine names, four more than the twenty-five 2.1.261 offered the day before:
`TaskCreate`, `TaskGet`, `TaskList` and `TaskUpdate`. Until this change gave them rows
they denied under every policy, `permissive` included, which is what the check is for.
They are `provider_native` now, so it is quiet again and the next rename breaks the first
record written after it. Claude Code 2.1.281 renamed `Task` to `Agent` for subagent
spawn; the table keeps both.

**A `claude` result with no `is_error` is classified from `tool_use_result`, since build
ticket 37.** On 2.1.263 a successful `Read`, `Edit` or `Write` result carries no `is_error`
at all, while `Bash` always does, and the normalizer filed every one of those `unclassified`
under contract 7.6: half the results of every Sonnet run that did real work, and with them
`outcome_source: unknown`, an export refusal and a `completed/unknown` line in the evolve
brief. The `user` frame that carries the result also carries a top-level `tool_use_result`,
measured on runs 01a07f37-9cab and 01a07f18-910f: an object on success (`{"type": "text",
"file": {...}}` for `Read`, `filePath` with `structuredPatch` for `Edit`, `type: "create"` for
`Write`, `stdout` and `stderr` for `Bash`), and a string starting `Error:` on failure. So
`result_status` reads `is_error` when present and that field when not, an object as `Ok`, an
`Error:` string as `Error`, anything else still `Unclassified`. Absence is never read as
success; the object is presence. Harvest shares the function and the field is absent on a
transcript, so nothing changes there.

## Deriving an outcome, and what `hydra run` returns

Ticket 04. `src/outcome.rs` turns a normalized run into contract 8's nine outcomes and
8.2's `outcome_source`, and `hydra::run::result` projects that plus the diff stat into
contract 17's JSON. The record writer types, `RunStart`, `RunEnd` and `Auth`, and
`status::Report` derive `Serialize` too; nothing derives it for a normalized run, and that
is where contract 11's never-store rule stops being a comment.

**The order of `derive`'s checks is the design**, and the first one that answers wins. It
runs from what Hydra decided toward what the run got to try:

| # | Check | Answers | Why it sits here |
|---|---|---|---|
| 1 | `run_end.refusal` is set | the refusal's outcome | contract 6's one stated outcome, and there is no stream to read |
| 2 | `provider_refused` | `auth_failed`, `rate_limited` | the provider refused its own service, so the run never started its work |
| 3 | `refused_everything` | `gated` | every call refused and at least one refusal was the gate |
| 4 | no provider | `completed`, or `failed` when `run_end.error` is set | a harness run describes the pass, spec 4.10, not the patch |
| 5 | `claude_harvest_limit_refusal` on a harvested run | `rate_limited` | the limit lives in the transcript, not on a terminator |
| 6 | the terminator | `completed`, `failed`, `crashed` by provider | a harvested body, no terminator, or a `result` frame, read three ways |
| 7 | `outcome_source` | `unknown` on a `completed` with unclassified calls or an unwatched harvest | the outcome stands and the source says how much to trust it |

**Nothing on a streamed run reads the exit status to pick an outcome**, and the rate-limited
run below is why. It exited 1. On a streamed run the one thing the derivation reads off
`exit_code` is whether it exists at all, which separates a child that died from a spawn that
never happened. A harvested interactive run is the exception, row 6's first reading: it has no
terminator, so `derive` maps `run_end.exit_code` 0 to `completed` and any other code to
`failed`, and a null code to `completed` with `outcome_source` saying the harvest was not
watched. Until 2026-09-24 this paragraph said nothing read the exit status at all, which the
third hiring panel's H10 caught.

**`gated` is "the gate refused everything", as a predicate.** That is contract 8's own
phrase, and harness spec 12 and tickets 27 and 14 all use the same words. It comes out
`gated` when the run made at least one tool call, no call got through, and at least one
refusal was Hydra's own gate. Three things it deliberately is not:

- **Not one refused call.** Contract 8.1 dropped `denied` partly because it "read as a
  per-tool-call fact", and a run that made fifty calls, had one denied and finished the
  task is a trajectory rung one should learn from. The denial stays in the record per
  call, where 7.7 puts it.
- **Not a gate leak.** A `Finding::GateLeak` is the policy denying a call nothing
  actually refused, so the call went through. An outcome says what happened, and filing a
  leak as `gated` would report the run as blocked by the hole that let it through.
- **Not somebody else's hooks.** A `decider` of `external` is the user's own `PreToolUse`
  guards, which 7.7 spends a section separating from Hydra's gate. A run blocked entirely
  by those comes out `completed`.

**The rule has one weak spot beyond that.** A run with one allowed `Read` and four
denied `Write`s comes out `completed`, because something got through. That run genuinely
failed on too narrow a policy, which ticket 27 calls a policy bug rather than a skill bug,
and nothing in the record says what it was trying to do. Fixing it means a threshold like
"mostly refused", which is a number no ticket settled. So the predicate stays literal, and
`tests/outcome.rs` pins both directions so a change to it breaks a test on purpose.

**A pre-gate record with a real denial in it comes out `completed`.** Contract 7.7 gives
a run with no policy no `permission_decision` at all, and `external` "presumes a policy
exists to have allowed", so there is nothing for the `gated` rule to read.
`01a07218-8817-7581-9ddb-47565e5d5ba7` is that record: one `Write`, refused, `policy:
null`, derived `completed`. It is the design's own consequence rather than a hole, and it
applies to the three pre-gate records on this machine and to nothing written since.

**`unknown` has one producer and it is the only soft outcome.** A `completed` reached by
nothing saying otherwise, on a run holding a tool result the adapter returned
`unclassified` for, is `outcome_source: unknown`. Every other outcome rests on something
a source said out loud, and an unreadable tool result does not weaken those: a 429 beside
an unclassified result is still `rate_limited`, `derived`.

**`auth_failed` from the stream is a hypothesis and `rate_limited` is not.** 429 is
measured, on the run below. 401 and 403 are what an expired token should produce and
neither has been seen on a record here, so they are in the match with that said out loud
rather than presented as a reading. The direction of the guess is deliberate: an
authorization refusal filed as `failed` teaches rung one a skill lesson from a lapsed
login. Anything else the provider reports falls through to `is_error`, so a 500 is a real
failure of the run.

**`crashed` reads the terminator's absence, and now one signal note.** `claude` writes
exactly one `result` frame on every path, so a `run_end` with no terminator behind it is
a child that stopped mid-run. With a null `exit_code` it is `failed` instead, because no
process ran and nothing died. Ticket 09 added the one witness the record can have: the
pty reports a signal death as exit code 1, so the detached half writes `the child died
on signal <name>` into `run_end.error` when it saw one, and the derivation reads that
note ahead of the tee-error branch. It matters on `cursor-agent`, where exit 1 with no
terminator is also the provider's orderly failure path; the section on that provider
has the split.

**`run_end.error` is read on one side of that split and not the other.** With a
terminator present the run reached its own end, so the field is a fidelity note: a lossy
line means the record has every line and one is degraded, and filing that run as `failed`
would teach rung one a skill lesson from a UTF-8 byte. With no terminator it is the only
thing in the file that says which side stopped. So a `run_end` with no terminator, a real
exit code and an `error` is Hydra's tee having broken on a child that may have gone on to
finish, and it comes out `failed` rather than `crashed`, because contract 8 defines
`crashed` as "the process died" and nothing witnessed one. `crashed` is left for the
shape where the child ended without its terminator and Hydra has nothing to say about
why.

**The tee caps a line at 16 MiB, since build ticket 33.** `pty::LINE_CAP`. A provider
that streams one frame with no newline used to grow `pending` without limit; now the tee
writes what it holds as a frame of its own when the cap is reached and keeps reading, so
the remainder becomes the next frame and nothing is dropped. The count lands in
`run_end.error` beside the lossy UTF-8 count, in the same sentence shape, and stays null
when nothing was capped; a piece that is not JSON normalizes as `noise` the way any other
non-JSON line does, and every adapter pushes one `Finding::Limit` naming how many frames
the tee split. `tests/pty_tee.rs` drives a 20 MiB line from a `dd | tr` stub, since a
payload that size cannot ride argv the way the 200 KB one does, and gets two frames, one
capped line named on `run_end`, and a record that validates.

**The final assistant text is the final turn, not the final block.** Contract 7.3 gives
one `message` event per content block, which is what `Message::block` numbers, so a frame
that interleaves thinking with prose arrives as `[text, thinking, text]`. Taking the last
event returned the second half of what the model said and dropped the first. The blocks of
the last frame are joined in block order, separated by a blank line.

**A run with no `run_end` gets no outcome, and `derive` returns `Option` to say so.**
Either the child is still going or the run is ticket 14's abandoned class, and telling
those apart needs 5.2's pid liveness test, which contract 8.6 gives to `hydra status`.
Flattening it to `crashed` would file every live run in the corpus as a dead one. That
reader is now built: `hydra status` answers `live` or `abandoned` where the derivation
answers nothing.

**A record Hydra cannot read back does not fail the command.** The file is whole and
synced before the read starts, so propagating the error would report a run that worked as
one that did not and would take the run id and the path to the answer with it. The
fallback returns contract 17's field list with the derivations null and `error` naming the
read failure, which is what stops `outcome: null` reading as a measurement. That null now
has two producers, a run with no `run_end` and a record that would not read, and `error`
says which. What triggers it is not tested and cannot cheaply be: it needs `read_file` to
fail on a file `hydra run` just wrote, and `hydra run` launches a provider. The fallback's
field shape is tested.

**A spawn that never started still writes a record and returns no JSON.** The detached
half writes `run_start` and `run_end` with the reason in `error` and exits; the foreground
reads that record back like any other, sees a null `exit_code` with no `refusal` beside
it, and returns the error rather than the projection, so `hydra run` exits non-zero with
nothing on stdout. A launch that fails before the fork, so that there is no detached half
to write anything, writes the same record from the foreground instead. Build ticket 92
split the launch into phases and every error after the worktree exists reaches
`run::never_started`: MCP config write, skill injection, spawn-config compile, argv build,
sandbox refusal, agy hook mount and detach failures all land on one path, not only the
spawn half. By then a worktree, its branch and a run id are on disk and stay there for
`hydra reap`; once compile succeeded a gate is on disk too. Under D10 a file has to exist
saying so. `tests/launch_phases.rs` pins the handoff failure through `run()` itself.
The record derives `failed` and `tests/outcome.rs` pins it, so nothing is lost for the
corpus; what is missing is the JSON for a caller that wanted the projection rather than
an exit code. Making it print instead would mean exiting 0 on a run that never launched,
which breaks any script reading the status. The delegate face is where that hurts, because
a caller there has no exit code to read, so the face turns the error into a tool result
with `isError` and the sentence in it. On the command line the exit code is still the
answer.

**That test is "no exit code and no refusal", and it is a reading of the record rather
than a fact the launcher kept.** The launcher no longer runs the spawn, so it has nothing
in memory to check; contract 8.3's refused spawns carry a `refusal` code and a null exit
code, which is why the second half of the test is there. `tests/detached.rs` drives a
binary that is not on disk and asserts the record, the null pid and the closing line.

**The result is nine fields and `provider` is not among them.** Contract 17 gives the
list and then says "Nothing else", so `provider` and `provider_session_id`, which the
first slice returned, are gone. Both are in the record and `run_file` is on the list.
`branch` and `diff_stat` are absent rather than null on a run with no checkout, following
5.5's rule; the rest are always present.

**Reading the record back costs one Python process per record.** Since build ticket 86, the
replay hands the gate every tool call of a record in one batch, and falls back to one process
per call only when the batch fails (`src/policy/replay.rs`). Measured on `hydra show
01a0c924-a494`: 10.8 s before, 0.21 s after, output identical under `cmp`.
That is the price of `permission_decision` being a re-evaluation rather than a copy of the
gate's answer, which contract 7.7 requires and the "Reading a record back" section above
argues for.

**Detaching moved the child off the caller's critical path and not that read.** A blocking
call still pays it, because a projection of the record is what it asked for. What changed
is that the cost is now optional: a call that gives up pays nothing, since a handle is
built from line 0 and derives nothing, and `hydra status <run-id>` pays it later on a run
it did not launch. `hydra status` with no argument pays none of it at all: it reads line 0
and the tail of each file and never normalizes. Measured on this machine, twelve records:
under 10ms for the listing, 20 to 30ms for one ended run with no tool calls in it.

## The detached spawn, the timeout, and what `hydra status` reconciles

Ticket 05. Harness spec 9 makes every child detached and blocking a view over it, and
10 makes process exit the authoritative end of a run with liveness derived and never
announced. Both are built. `src/watch.rs` is the half that owns the pty and the record,
`src/run.rs` is the half a human talks to, `src/live.rs` is contract 5.2's test, and
`src/status.rs` and `src/logs.rs` are the two commands that read what the test says.

**`hydra run` re-execs itself as `hydra watch <spec>` in a session of its own.** The
launcher composes everything provider-specific first, writes it to
`~/hydra/spawn/<run-id>/launch.json`, and forks a copy of itself with `setsid` and its
three standard descriptors on `~/hydra/spawn/<run-id>/watcher.log`. Closing a Ghostty
window sends `SIGHUP` to the foreground process group of its tty, and a process in its
own session has no controlling terminal to be hung up on. `tests/detached.rs` asserts
the session id rather than a symptom of it.

**An in-process `fork` was the alternative and it is the wrong trade.** It would have
put an allocator, a pty and an open record file on the child side of a call with no
`exec` after it, and nothing about it would be reachable from a test. Re-exec costs one
process start per run, which is noise beside a provider launch.

**The launch spec is not a test hook and it is not a second source of truth.** It holds
the binary, the argv, the cwd, the run file and the whole of line 0 with `pid` and
`process_start_time` null, which are the only two fields the detached half fills in and
the only two that cannot exist before a process does. It is read once, at launch, before
the record exists, so harness spec 5's rule holds: nothing needs a spawn artifact to
interpret a run. Taking a binary and an argv rather than a provider is what lets
`tests/detached.rs` drive the whole path with `/bin/sh` and spend no subscription.

**No environment values go in it.** `HTTPS_PROXY` can carry a password, and contract 5.3
keeps values out of the record for the same reason, so the detached half rebuilds the
child's environment from the allowlist out of what it inherited. Names reach the record;
values never touch the disk. The names Hydra computes itself, the marker, `CURSOR_DATA_DIR`
and the run's two object directories, travel by value as `hydra_env`, because they are paths
and ids and not secrets.

**Every process Hydra starts that runs a provider binary or proposer code starts with
`env_clear`, since build ticket 97**: the watcher, the pty child, the five probes including the
`codex` login check, both interactive launches and rung two's admission steps. The watcher
inherits the child's environment and nothing else, so a same-uid `ps -E` against it reads the
allowlist; `tests/detached.rs::the_watcher_holds_the_childs_environment_and_nothing_the_launcher_carried`
reads it from the kernel. `git` and the read-time gate replay still inherit, and build ticket
97's Outcome has the table of every `Command::new` in `src/`.

**The timeout returns a handle and the handle is not contract 17's result.** Harness spec
9 degrades a blocking call rather than thinning it, so the two shapes are different on
purpose: a run that has not ended has no derived fields, and printing contract 17's list
with nulls in it would offer a page of measurements nobody took. The handle is `run_id`,
`outcome`, `alive`, `pid`, `timeout`, `branch`, `run_file` and the `hydra logs --follow`
line that answers "what happened next".

**`timed_out` is the call's outcome and never the record's, and this is a reading the
ticket left open.** Contract 8's row is "the watcher gave up; the child may still be
running", which is a sentence no `run_end` can carry: by the time a record has a closing
line the child has stopped. Ticket 05 says both "the timeout's handle carries no outcome"
and "`timed_out` has no producer until you write one", and those point opposite ways. What
is built takes the second: `Outcome::TimedOut` is produced where the giving up happens,
on the handle, and the record keeps its own answer for whoever reads it later. Measured on
run `01a07471-0a3e-74d2-b576-153ac0e7427b`, `read-only`, `--timeout 4s`: the call returned
the handle with `outcome: timed_out` and `alive: true`, the child carried on with nobody
watching, and the record it closed by itself derives `completed`. The alternative reading
would have filed that run in the corpus as a timeout, which is a lesson about the caller's
patience dressed up as one about the task.

**There is no `outcome_source` on the handle.** Contract 8.2's three values all describe
how a record was read and this outcome was not read from one. A fourth value is a decision
for the map rather than a field for this build to invent.

**Nothing kills a child, ever.** Not the timeout, not the cap, not `hydra status`. The
only thing that removes anything is `hydra reap`, which a human types, and that is ticket
06. So a wedged run stays wedged until someone looks: harness spec 19.5 rules out a stall
detector and contract 18.4 says why, and a timeout with a decimal point on it would be
exactly that detector under another name.

**Liveness is one function and nothing else asks the kernel.** `procinfo::is_alive` was
already built and tested; `live::read_view` is what feeds it a record's pid and start
time. Three states come out: a file with a closing line has ended, one without whose pid
still matches is live, one without whose pid is gone is contract 6's abandoned class.
`tests/status.rs` builds all three by hand, because the reconciliation is what is under
test rather than the launch that produced the files.

**A status pass reads line 0 and the last 64 KiB and nothing between.** `run_end` is
always the last line and is always small, so a fixed tail either holds it whole or the
file has not ended, whatever the frames above it look like. A whole-file read would make
every status pass cost the size of the corpus.

**A run file the scan cannot read is skipped with a note.** The scan runs before every
launch, so propagating that error would mean one unreadable entry under `~/hydra/runs/`
stops `hydra run` and `hydra status` alike until a human finds it by hand. It is the same
stance the normalizer takes on a torn record, one level up.

**A dead pid is not by itself an ending, and two places had to learn that.** The detached
half reaps its child and then writes the closing line, so every ordinary run passes
through a window where the liveness test says gone and the record has not ended. It is an
`fsync` wide against a 100ms poll: small on this machine, and hit often enough to matter.
`wait_for_end` and `hydra logs --follow` both re-check the file before they conclude anything,
and `tests/status.rs` holds that window open with a pid killed before the record was written.
One re-check was not enough on a loaded CI runner: on 2026-09-21 the follow test's writer took
longer than the poll and the tail called a finishing run abandoned, run 35643315083, while the
same suite passed on the commits either side. `--follow` now gives the file twenty polls, two
seconds, after the pid reads dead, which an abandoned run pays once and a finishing one never
reaches.

**A checkout younger than the launch budget is a launch in progress, not a reap, and the
record being there does not change that.** `hydra run` creates the worktree and seeds it
before the detached half writes line 0, and on a large repo that is seconds. Age is what
tells that apart from a run whose record was pruned, and `abandoned_worktrees` takes the
grace period as an argument so ticket 06's reaper decides it at the call site rather than
inheriting it.

The first version applied the grace only when there was no record at all, and ticket 06's
code review found the hole: `watch` creates the run file before it spawns the child, so
during a launch there is an empty record that reads as abandoned with a null pid, and a
view existed, so age was skipped. `RunView::launched` is the test now, meaning the record
carries a pid or has ended. Everything else is judged on age. A refused spawn passes
because it has a closing line, which is what makes contract 5.2's null pid a claim rather
than a silence.

A filesystem with no creation time keeps the directory on the list, which is the safe end
for a command that only reports.

**A record `hydra status <run-id>` cannot normalize costs the projection and not the
command.** `hydra run` already degrades that way. `state`, `pid` and `run_file` come off
line 0 and are what lead a human to the file, and a status command that refuses to report
a damaged record hides the run most worth looking at.

**A `--timeout` bigger than the clock can reach means "do not give up".** `Instant +
Duration` panics on overflow and the flag takes any `u64` of seconds, so the deadline is
computed with `checked_add` and `None` is a budget that never expires. Panicking there
would have aborted the launcher after the worktree, the gate and the child already
existed.

**A torn line 0 still reports a run.** The run id falls back to the filename, which is
where it came from, and the state falls back to abandoned. A status command that refuses
to report a damaged record is one that hides the run most worth looking at.

**`hydra status <run-id>` returns the same projection `hydra run` does, off the same
function, on a run it did not launch.** That is contract 17 being a projection of the
record rather than a summary of a launch, made checkable. It appears only on an ended run:
a live one has no ending to derive and gets `state` instead.

**The diff stat's base is recovered from the seed index rather than persisted.** The sha
`worktree::create` returned lives in one process's memory and a detached run returns to a
caller that never had it. `~/hydra/spawn/<run-id>/seed-index` is the index the seed
snapshot staged into and nothing writes to it again, so `git write-tree` against it
answers with the same sha it answered with at launch. A file holding the sha beside it
would be a second source of truth for a fact the index already carries. Measured on
`01a073ef-701c-7522-b5bb-56d5ecba0f57`, launched by ticket 04 and read back by a later
binary: base tree `7c222fb9d996872a39801ad14ef7db1d835280fb`, the same one that run
reported.

**Abandoned worktrees are every checkout no live run is using**, which after any finished
run is most of them. Nothing is cleaned up automatically (harness spec 10), so that list
is work for `hydra reap` rather than a list of faults. A directory whose run file has been
pruned is on it too: a checkout nobody can explain is exactly what a human wants told.

**`hydra logs` prints the run file verbatim and does nothing else.** Observation is
read-only: no attach, no input, no answering a permission prompt from outside. `--follow`
stops at the closing line, and also when the liveness test says the writer is gone, so
following an abandoned run ends with a line saying so rather than waiting forever.

**Harvest was the half of `hydra status` that was not here until build ticket 27.** Section
10 has it complete a dead interactive run from the vendor's transcript, and the interactive
section below says how `status` does that now.

## The diff stat, and the seed it has to subtract

Contract 17's one returned field with no record equivalent, and the trap in it is real
rather than theoretical. Harness spec 6.3 seeds a run's worktree with the parent's
uncommitted work, so `git diff HEAD` inside the checkout counts the human's work in
progress as the child's.

**So the base is a tree, snapshotted after seeding.** `worktree::seed_tree` stages the
freshly seeded checkout into a scratch index and calls `git write-tree`; the sha rides on
the `Worktree` and the closing `git diff-tree -r --numstat` is against it. The returned
`diff_stat` carries `base_tree` and a `covers` sentence, because the number is useless to
anyone who does not know what it is measured from.

**The scratch index is why the snapshot is invisible.** `git add -A` against the
checkout's own `.git/index` would leave the child looking at a fully staged tree, and a
child asked to finish work in progress would see something the parent does not have.
`GIT_INDEX_FILE` points at `~/hydra/spawn/<run-id>/seed-index` instead, with a second
file for the closing pass so staging cannot destroy the base. `tests/worktree.rs` asserts
the child's own `git status` still shows the seed as uncommitted work.

Measured on this repo, 2026-09-05: a worktree seeded from nine dirty files, one file
written into it afterwards. `diff_stat` reported one file and one insertion; `git diff
HEAD` in the same checkout reported nine files.

**Committing the seed was the alternative and it is the wrong trade.** One commit on the
run's branch would make `git diff HEAD` correct for free, including for the child itself.
It also hides the work in progress from a child asked to continue it, which is what 6.3
seeds the worktree for, and it changes what the branch means. The snapshot changes
nothing the run can see.

**A stat that cannot be computed says so on stderr.** Neither failure refuses a run: by
the time the closing diff runs the record is already whole, and a seed snapshot that will
not write costs the field and nothing else. Both print a line, because an absent
`diff_stat` otherwise reads as "this run had no checkout".

**A record with no `run_id` gets no diff stat.** A torn line 0 normalizes to an empty
`run_id` with a `Finding::Limit` rather than failing the read, and `Layout::spawn("")` is
`~/hydra/spawn/` itself, so the scratch index would be one file shared by every such
record and two at once would stage into each other. The guard skips the stat, the same
way a missing seed tree does.

**A `permissive` run could make the stat wrong until the deny list covered the spawn
parent.** Writes are scoped to the worktree under four of the five policies; `permissive`
grants unscoped writes, and `~/hydra/spawn/` is a denied path under every policy since build
ticket 14, so a file-tool write or a shell command that names `seed-index` is
`protected-path`. A shell command that reaches the path without naming it, the residual
`SECURITY.md` states, still can, and that costs the diff stat and nothing else.
The record is what carries the truth about what a run wrote.

## What a rate-limited run really looks like

The first `hydra run` after ticket 04 landed hit the session limit, which measured
contract 8's nastiest shape on a live run rather than on a synthetic frame. Run
`01a073ef-701c-7522-b5bb-56d5ecba0f57`, `claude` 2.1.261, `read-only`, 2026-09-05:

```
subtype: "success"      is_error: true       api_error_status: 429
terminal_reason: "api_error"                 stop_reason: "stop_sequence"
exit code 1             usage: every count present and 0
```

Derived `rate_limited` with `outcome_source: derived`, and `result_text` carried the
provider's own words: "You've hit your session limit · resets 7:50pm (America/Bogota)".

**Two things the contract did not already say.** `terminal_reason` reads `api_error`,
which is a second honest signal beside `api_error_status`; nothing keys on it, and it is
written down because a provider that drops the status field would leave it as the only
one. And the exit code was 1, so a reader keying off the exit status would have filed
this as an ordinary failure and rung one would have learned a skill lesson from an
exhausted quota.

**`usage` is `provider` with zeroes here, and that is correct.** The frame reported every
count and every count was zero, because the request was rejected before a token moved.
Contract 7.8's rule is about a provider that reported *nothing*, which is the
`unavailable` case; a provider reporting zero is a measurement.

**The `rate_limit_event` frame does not have the shape contract 8.5 describes.** 8.5 puts
`rateLimitType`, a `status`, a `resetsAt` epoch and a per-window `utilization` on the
frame. Measured, they are nested one level down under `rate_limit_info`, `utilization`
lives under `unifiedWindows.<window>`, and three fields 8.5 does not name are beside
them: `overageStatus`, `overageDisabledReason` and `isUsingOverage`. Rung two's early
abort became this frame's one reader and read `raw.status`, the top of the frame, so it
never fired. Build ticket 41 pointed it at `rate_limit_info.status` on 2026-09-12, with
the top kept as a fallback for a frame in 8.5's old shape. The `resetsAt` staleness
test 8.5 asks for is still not written; the abort fires on the enum alone.

**A harvested `claude` run has no `result` frame, so the sentence is the signal, since
build ticket 29.** Run 01a07f6c-ba21, an interactive session harvested on 2026-09-08, had
one assistant turn, "You've hit your session limit · resets 1:50am (America/Bogota)", and
derived `completed` with it as `result_text`. `outcome::claude_harvest_limit_refusal` now
files a harvested `claude` run whose last assistant text starts with `You've hit your` and
contains `limit` as `rate_limited`, after the gate-refused-everything check and before the
`completed` fallback, so a gated run that then met the limit stays `gated`. It is a text
match written down as one, the same standing `cursor_stderr_refusal` has: a reworded
sentence sends the run back to `completed`, and rung one would learn from it again. A
recorded run keeps reading the 429 off its frame; `tests/outcome.rs` pins that the sentence
alone does nothing on a streamed body.

## What the command line does not carry

**`--mcp-config` and `--strict-mcp-config` are passed together on every non-`judge`
recorded run**, which is how 6.7 pairs them, and the section on MCP below says what the
file they point at holds. A `judge` run carries the strict flag alone, which is what makes
its empty server set an argument rather than a side effect of `--restricted`, and the
absent file is why its policy can be handed no server.

**`< /dev/null` is dropped**, and harness spec 6.7 now says why.

`--permission-mode bypassPermissions` is passed, and the section below says what was
measured before it went on.

## What ticket 02 measured: the gate on a real run

Three probes on `claude` 2.1.261, 2026-09-05, each a run asked to write `probe.txt`
inside its own worktree with the file tool and to stop if blocked. The records are on
this machine and the run ids are below.

**A hook deny beats `bypassPermissions`, on this wiring and not only in ticket 26's
rig.** Run `01a07308-39e3-79c2-8901-ac052723b79a`, policy `read-only`, `permissionMode:
bypassPermissions` on its own init frame. The gate answered `deny` on
`PreToolUse:Write`, the reason `hydra/capability: policy read-only does not allow
file_write (Write).` reached the model verbatim as the tool result with `is_error:
true`, no file was written, and `result` closed `subtype: success`, `is_error: false`,
exit 0. **The refusal holds and the terminator lies about it**, which is what contract
section 8 says to expect of all three providers. `result.permission_denials[]` carries
the denial with its `tool_use_id`, contract 8.5's corroboration. Since Claude Code
2.1.280 the same `is_error: true` result carries `PreToolUse:<Tool> hook error: ` in
front of the reason; build ticket 116 strips it before `reason_code` reads the gate
code.

**The flag went on after that, not before.** Run
`01a07307-2693-7ef0-b572-a565ffcc3190` is the same probe with `bypassPermissions` held
back, and it is what proved the hook was mounted and the `*` matcher matched before the
provider's own permission layer was taken away. Same denial, same reason, no file. The
sequencing is the point: a gate that had failed to mount would have been invisible under
the provider's default mode and wide open the moment the flag went on.

**The gate discriminates rather than blanket-denying.** Run
`01a07308-a250-7792-9e8b-0d99697c99d9`, policy `standard`, same probe, same flag: the
gate answered `allow` with `hydra/allow: policy standard allows file_write.` and
`probe.txt` is on disk in that worktree. Without this run the other two prove only that
something says no.

**The user's own hooks reach the child, and the record shows it.** Every one of the
three carries a `UserPromptSubmit` `hook_response` from the user's `~/.claude`
guards, alongside Hydra's `PreToolUse`. That is 6.6's composition working rather than
isolation, and it is also 6.6's warning arriving: the record now carries the stdout of
a script the user edits.

**`standard` did not hit the `ToolSearch` and `Skill` denial.** Ticket 02 expected the
first gated `standard` run to notice the capability loss below. This probe was one tool
call long and never reached for either.

**Ticket 07's first injected run hit it, on 2026-09-06.** Run
`01a07512-d36e-7651-b8d9-e9e279e5730b`, `standard`, `--model haiku`, two skills linked
and a task naming one of them. The gate refused `Skill` with `hydra/out-of-scope`, the
model replied "The policy compiled for this run does not allow the Skill tool", and the
run derived `gated`. The loss is real and it is now on the record twice. The section on
the skill store above has the other half, which is that the link resolves outside the
`worktree` scope so the `SKILL.md` cannot be read either.

**Both halves closed on 2026-09-06.** Map ticket 39 decided and build ticket 13 built
`skill_invoke` and the `skill_snapshot` scope, and run
`01a078bc-420f-7b23-8e67-2dfc9f98f859` is the same probe under `standard` with both calls
allowed. The two paragraphs above stand as what the records said before that, which is
the point of keeping them. `ToolSearch` followed the same day, on map ticket 43 and build
ticket 16, so neither half is denied under `standard` now. No run has exercised it: the
gate answers it in `tests/gate.rs` and nothing has asked a model for it.

## What the first slice measured

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

## Delegation, and the face

`hydra run --parent <run-id>` makes a child of another run, and `hydra delegate-face` is
the MCP server a model delegates through. Ticket 11 built both, and `tests/delegation.rs`
and `tests/face.rs` cover them without launching a provider.

**What the attempt was is decided before what the machine is.** 6.1's items 2 and 3 now
run ahead of the environment build, the provider version read and the two repo-file checks.
The order matters for one case: a child reaching for `permissive` on a provider nobody
installed used to error out with nothing on disk, which is the invisible attempt 7.3 exists
to prevent. So the narrowing refusal writes first and the machine's own facts are read
after. The record that refusal writes carries no provider fields, contract 8.3's phrase:
`provider_version` is null because no binary was asked and `env_allowlist` is absent
because no child environment was built. The envelope still names the provider, which is
contract 4's field and a fact about the request. `tests/face.rs` is the proof, since it
now passes on a machine with no agent CLI installed at all.

**A child reads two fields off its parent's record and nothing else.** `run::Lineage::of`
takes `depth` and `policy` off line 0 of `~/hydra/runs/<parent>.jsonl` and refuses three
shapes before a run id exists: a parent this machine has no record of, a parent whose
`policy` is null, which is a pre-gate record or an interactive one, and a child that would
land past the cap. Contract 4 makes `parent_run_id` an edge the parent writes at spawn, so
nothing is ever reconciled against the child. The edge rides the launch spec into the
detached half, and it is on every line of a child's file, the two a refused spawn gets
included.

**The cap is `live::MAX_DEPTH` and it is 2.** Harness spec 9 says "capped at 2,
configurable" and names no mechanism, so configurable means this line moves and the binary
is rebuilt. A flag was the obvious alternative and a child that can raise its own cap is
not a cap, and a protected config for one integer would be a sixth of those. A child of a
depth 2 run is refused with a typed error and no run id, the shape 6.1 item 4 gives the
concurrency cap. Contract 8.3 promises a record for the narrowing refusal and for nothing
else, and the parent's own record already carries the refused call and the sentence that
came back.

**The narrowing refusal is a record, and `policy_narrowing` is the third of contract 6's
four codes to get a producer.** It is written after the run id is minted and before the
worktree is created, so what the attempt leaves behind is `run_start`, `run_end`, and
nothing on disk beside them. Line 0 carries the requested policy in `policy` and the
parent's in `parent_policy`, a field contract 5.1 did not have: 8.3 asks for both and 5.1
named one. It is absent on a run with no parent, under 5.5's rule, and contract 15 covers
the addition because a reader that ignores it reads the record exactly as before. The
remediation names both policies and says which of the two failures this is: `permissive`
under a `read-only` parent is wider, and `curate` under a `standard` parent is
incomparable, since the five are a partial order rather than a chain (7.3). Both are
refused. Only the first is an attempted escalation, and the words are different so a
reader can tell them apart.

**The face is a stdio JSON-RPC server written by hand in `src/face.rs`.** Four methods:
`initialize`, which answers the protocol version the client sent, `notifications/initialized`,
which is ignored, `tools/list`, and `tools/call`. Anything else is `-32601`. No dependency
was added for it, because the protocol this speaks is four methods wide and `serde_json`
was already here, and what that buys is one writer on stdout. A stray line there breaks
the client's parser and the failure looks like a hung server, so diagnostics go to stderr
beside the ones `hydra run` already prints.

**One tool, `delegate(provider, task, files?, policy?)`, and everything the model gets
wrong comes back as a tool result rather than a protocol error.** An unroutable provider
name, an argument the face does not take, a policy the parent does not contain, a call from
inside a macOS seatbelt: all of them are `isError` with a sentence, because a JSON-RPC error
is the client's to handle and the model never sees it. A refused launch is not even that. It
is `isError: false` with contract 17's JSON in the content block, `outcome: gated` inside,
which is a result the model can act on. `model`, the timeout, `--shared-tree` and depth are
not in the schema and passing one is refused by name, because dropping `model` silently would
tell a model it got the model it asked for. The seatbelt refusal carries
`hydra/seatbelt-delegation`, on `normalize::GATE_CODES` since build ticket 76, and names map
ticket 64.

**The marker is `<<<hydra-result TOKEN>>>`, the text, then `<<<end hydra-result TOKEN>>>`.**
The token is a UUIDv7 with the first six bytes thrown away, 20 hex characters, drawn after
the child returned. Those six bytes are the millisecond clock and a child that knows
roughly when it stopped could guess them; what is left is 74 bits it has no way to see.
`/dev/urandom` would give 128 bits and one more failure mode on a path with nowhere to
report one. `hydra run`'s own JSON is unchanged and `result_text` stays a plain string
there: the face is what wraps, because the face is where a child's words reach a model
that did not run it. A result with no text carries no boundary, which is every refusal.

**`files` is text in the task string and grants nothing.** Harness spec 9 names `--files`
on the CLI and on the face and defines it nowhere, so this is a decision rather than a
reading: the paths land under one heading at the end of the task, which means they are in
`run_start.task` verbatim and the record shows exactly what the child was told. What the
child may read is its policy's business, and a path here that the policy denies is still
denied. `--files` is not on `hydra run` itself, because a human typing a command line can
write the paths into the task.

**How a run gets the face.** Through the canonical MCP document, protected config #5,
which a human edits by hand. The block to add:

```toml
[[server]]
name = "hydra"
transport = "stdio"
command = "hydra"
args = ["delegate-face"]
admit = ["permissive"]
```

`command = "hydra"` needs the binary on the child's PATH, which is the launcher's PATH
under 6.2's allowlist; an absolute path works and is what a machine that runs Hydra out of
a cargo target directory wants. `permissive` is the only admission worth writing, because
`mcp_call` is granted by that policy alone (7.2), so a face admitted to `standard` would
start, list its tool, and have every call to it refused by the gate.

**`--parent <run-id>` is injected per run, since build ticket 26, and the paragraph that
used to stand here said the opposite.** It described the gap ticket 11 found: the document
is read once per launch and copied as written, so a face spawned through it had no
`--parent` and its children were depth 0. Map ticket 48 closed that without a new field:
Hydra recognizes its own binary by basename and appends `--parent <run-id>` to that
server's arguments in the file it writes for the run, and nowhere else. The section "`judge`
is not injected, and the face is admitted" below carries the current rule. This paragraph
is rewritten on 2026-09-12 because a `cursor-agent` run reading the repo cold, run
`01a09402-eb2e`, quoted the stale version as a reason not to adopt Hydra, which is what a
sentence left behind by a later ticket costs.

**`hydra show <run-id> [--events]` is built, since build ticket 18.** Without the flag it
prints contract 17's projection alone, the object `hydra status <id>` prints under `result`.
With it, one object: `run_id`, `projection`, `events` with every normalized event by `seq` and
`type` and that type's fields, and `findings`. `run_start` and `run_end` carry the body the
record holds under `body`. A parent that wants more than contract 17's nine fields asks here,
which is what section 9 meant by explicitly.

**`HYDRA_ROOT` overrides `Layout::discover`, and it was read for one test file until build
ticket 51.** Most tests build a `Layout::at` a temporary directory and call the library;
`tests/face.rs` and `tests/delegation.rs` drive the built binary as a subprocess, and the
binary is what calls `discover`; the corpus walks read the variable themselves through
`corpus_runs_dir`. In the library it is read in `src/paths.rs` and nowhere else, and it never reaches a child provider, because
6.2 builds that environment from an allowlist this name is not on. The rough edge below
says what is left of it.

## A gate answer is joined to its call by `hook_id`, since build ticket 22

The normalizer used to join a `PreToolUse` answer to its call by tool name and seq order,
because the frame carries no `tool_use_id`. Sonnet 5 broke that on 2026-09-07 in run
01a07efe-97f4: it issues the next call before the previous result arrives, so a first call's
answer lands inside what order alone reads as the second call's window, and three of sixty
calls came out `Ungated` with their answers sitting in the record. `hook_started` and
`hook_response` share a `hook_id`, and a start fires the moment a call is emitted, so the
normalizer now groups starts to calls per tool in seq order and takes each call's answer by
the ids its starts carried. A record with no `hook_started` frames keeps the old join. That
finding is what would have reached Rungs as a policy lesson, and 153 hook pairs for 60
calls on the same run say the count of hooks per call is not a join key either.

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

Measured 2026-09-07 in a Herdr pane on haiku: Herdr detected the session as `claude idle` on
its own, the first time any run Hydra launched registered with Herdr's detection, and the
record ended at 24 lines with outcome `completed`. `cursor-agent` and `agy` are not launched
interactively; the launch refuses them by name. An interactive run may take a policy since map
ticket 50, which is build ticket 28.

**With `--policy <name>`, since build ticket 28, an interactive `claude` run is gated.** The
launch reuses the recorded pre-flight: worktree seeded with `base_tree`, snapshot linked, gate
compiled, `settings.json` handed to `claude` on `--settings`, `run_start` carrying the policy and
its sha. The argv carries no `--permission-mode bypassPermissions`, because interactively that
flag opens a consent dialog with nobody to answer it; a human answers the vendor's own prompt
for an allow and a hook deny beats the prompt either way. The harvest reader derives a `deny` by
`hydra_policy` from every error `tool_result` whose text `hydra_voice` reads as a gate code,
since build ticket 55, and pushes one
`limit` finding saying allows were not recorded, so a reader tells the two record shapes apart
by `policy` being null or not. Measured on 2026-09-08 on run 01a07ff5-2389: a `Write` asked
for under `read-only` came back as one `permission_decision` denied by `hydra_policy` with the
gate's own reason, the worktree gained no file, and the outcome derived `gated`.

**Since build ticket 73, a gated interactive launch runs under the seatbelt on macOS.** The
gated arm routes `claude` through `sandbox::wrap_command` with the profile `write_spawn_config`
already wrote, and `run_start` carries `gate_sha` and `sandbox` the way a recorded launch does.
The ungated arm stays bare. `tests/spawn.rs::a_gated_interactive_launch_wraps_claude_in_the_seatbelt`
asserts the prep yields a profile path and that the argv starts with `/usr/bin/sandbox-exec`.
A human must still run one session and confirm the vendor's TUI works under the wrap.

## `judge` is not injected, and the face is admitted

**A `judge` run is linked no skills, since build ticket 25.** Map ticket 47 decided it: the
policy allows nothing and the diff rides the prompt, so the links were never reachable and the
`skills` group is absent on a `judge` record the way it is on a run with no checkout. The probe
under `judge` is not asked about skills.

**The delegate face is admitted under `standard` and `permissive`, since build ticket 26.**
`config/mcp.md` carries the block, and a server whose `command` has the basename `hydra` gets
`--parent <run-id>` appended in the file Hydra writes for the run, so a delegated child's depth
is its parent's plus one and section 9's cap holds. The command in the document is the absolute
path to the binary, because a recorded run's `PATH` is the human's and `hydra` is not on it:
measured on 2026-09-07, a bare `hydra` came up `failed` in the init frame and the absolute path
came up `connected` with `mcp__hydra__delegate` on the tool list. Nothing before the spawn
checks that a server's command exists.

**A call Claude Code refused on its input schema is not `Ungated`.** Five `Read` calls on a
haiku run passed `offset` as an array and came back as `InputValidationError` results with no
hook frame beside them, because validation runs before any hook. The normalizer reads that
result shape and does not count the gate's silence against it.

**A `Write` whose parent is not a directory fails the binary's own `stat` before any hook.**
Measured 2026-09-21 on run 01a0c5b2-8048, sonnet on 2.1.268, launched by the other session: a
`Write` into a linked worktree's `.git`, which is a file, came back `<tool_use_error>Error calling
tool (Write): ENOTDIR` with four `PreToolUse:Write` starts in the record for the run's other
writes and none for this one. The reader read it as `Ungated`, the corpus walk stopped on it, and
that spelling is the fifth on the list below of calls the binary refused before asking.

**A `Write` Claude Code refuses before reading the file never reached a hook.** Measured 2026-09-12 on run `01a096ee-8791`, `claude` 2.1.268 on haiku under `standard`: a `Write` into the skill store came back `<tool_use_error>File has not been read yet. Read it first before writing to it.` with no `hook_started` for it, in a record holding 43 hook starts for everything else. It is the fourth spelling of the binary's own input refusal that `src/normalize/claude.rs` knows, beside `InputValidationError`, `No such tool available` and `<tool_use_error>Blocked:`, and the corpus walk had read it as ungated and as a leak at once. The store file it named was edited and committed by a human four minutes later, not by the run.

## The Herdr bridge, on Herdr's side

Map ticket 44 made Herdr the interactive layer and said nothing crosses from Hydra to it, and
build ticket 19 built the other direction: `integrations/herdr/` is a Herdr plugin that nothing
under `src/` imports. `bin/hydra-in-pane` runs inside a pane, launches `hydra run --timeout 0`,
puts `run_id`, `provider` and `policy` on the pane as tokens, and replaces itself with `hydra
logs --follow`. `bin/hydra-bridge` polls the panes carrying a `run_id`, reads `hydra status`
for each and reports on change. Herdr holds four states and refuses any other name, so Hydra's
outcomes ride as display labels over `idle`, `working`, `blocked` and `unknown`; the table at
the top of the bridge is the whole mapping. Map ticket 49 put `timed out` and an abandoned run
on `unknown`, and since build ticket 24 the pane also carries `state`, `alive`, `outcome`,
`tokens`, `elapsed` and `last_tool` as tokens and a title such as `claude 01a07f1a · completed`,
so a human reads the run off the pane without Herdr classifying it.
Driven on 2026-09-07 on herdr 0.8.2: a haiku run launched from a pane ended, and eight seconds
later the pane read `done` with the label `completed` with nothing but the bridge having touched
it. `README.md` in that directory says how to link it.

`bin/hydra-graph`, build ticket 20, draws one run from `hydra show <id> --events` JSON as one
line per event with a call joined to its result and its gate decision, and `--follow` regrows
it every two seconds. Written by `cursor-agent` on Composer 2.5 against the JSON shape ticket 18
specified, before ticket 18 landed.

**`run_start.worktree.base_tree` is on the record since build ticket 23.** Map ticket 49 put
it there: the sha `write-tree` answered against the seed index at launch, so a reader of a
reaped run can tell the seed from the run's own edits, and `hydra run`'s diff stat no longer
needs the spawn directory to name its base. It is a name and not a guarantee, since the tree
hangs from no ref. Every record written before 2026-09-07 lacks it and re-derives from the seed
index while the spawn directory exists, and carries no diff stat after a reap, as before.

**A `standard` run reaches the human's live Herdr session.** The socket is a file and `herdr`
is on the `PATH`, and the `agy` run that built the bridge proved it by putting a test token on
the human's own pane. Harness spec 7.5 already says the gate cannot keep a shell away from a
binary, and this is that hole with a new name rather than a new one.

**Since build ticket 107, an interactive run has a pane launcher too.** `bin/hydra-in-pane`
discards stdout and tails the log, which suits a detached recorded run and not a vendor TUI, so
`bin/hydra-tui-in-pane` keeps the terminal for the vendor, tees stderr for the `hydra: run <id>`
line, and puts the same tokens on the pane plus `launch_mode=interactive`. `hy.zsh` is the one-word
entry: `hy [model] [policy]` splits a pane and runs it. Measured with a stub in a live pane; the
real `claude` session under the seatbelt is still the human's to run. The record of an
interactive run stays `run_start` alone until the harvest at exit, so the bridge labels the pane
alive with no tool names until then; whether the bridge may tail the vendor transcript instead is
map ticket 76's decision G. `bin/hydra-mascot` beside them is a prototype renderer from map ticket
75 that reads no record yet.

## Master of Puppets, since build tickets 108 and 109

Map ticket 75's panel and ticket 76's answers gave the mode its shape: an orchestrator outside
every run, a log per goal, approvals bound to a plan revision, and a ranking of agents from the
corpus. `integrations/puppets/` is that, in Python, and nothing under `src/` imports it.

`puppets.py` keeps one append-only JSONL per goal under `~/hydra/puppets/` and derives state by
folding it, the way an outcome is derived from a record and never stored. Its events are the
fourteen the README lists; `revision` is the count of `task` and `assignment` events, and an
approval carries the revision it was given at. A table approval covers every launch at its
revision (decision C, the table once per batch); a `launch` approval, required on `codex` and on
`high` or `agi` tasks on `claude` because those gates can fail open, and a `diff` approval, required
before a verdict of `applied` or `fixed`, are each spent by one use. `launch` writes
`launch_intent` before it spawns `hydra run --timeout 0` and `run` the moment the id is read off
stderr; an intent with no run after it blocks every launch until the human settles it with
`reconcile`, and the helper never relaunches on its own. Plans are launched one proposer per seat
with `prompts/plan.md`, read back from each run's `result_text`, and imported as tasks from the
chosen plan's fenced JSON block. The judged choice and the diff review are build ticket 111;
until then `choose` records an empty judge list.

`sheet.py` builds one JSON per provider and model pair from the records: provider, model and
timestamps from line 0, policy and subscription from `run_start`, the outcome and usage from
`hydra show --events` (cached by content hash, never the exit code), denials from the
`permission_decision` events, judge answers checked against `parse_judge_answer`'s vocabulary,
and resumed runs grouped under their root. `vendors.json` holds the hand-kept facts, null until
someone dates them, and cost is window share: the `seven_day` delta on `claude`, tokens over a
vendor budget elsewhere, null when the budget is null. `rank` orders agents by cost over the
10th percentile of a beta posterior on first-pass acceptance, seeded from `priors.json` and
moved only by `human_verdict` events. `puppets.py assign` takes the ranking only when it rests on
at least one verdict, and falls back to a default table, recorded as `source: default`, until
then. Built over the 380 records on 2026-09-24: 26 pairs, three of them spellings of one model
(`sonnet`, `Sonnet 5`, `claude-sonnet-5`), which the sheet keeps apart on purpose.

**The judged steps, since build ticket 111.** `choose --judge` and `review` launch three seats,
`claude` opus and composer-2.5 under `judge` and `codex` under `standard` (map ticket 63), each
answering one JSON object; every checklist item carries a severity ceiling so an advisory item
cannot block, a plan verdict needs every seat parsed and unanimous, and major or critical diff
findings go into the human's Hunk session as comments before it opens. The panel never shrinks
or substitutes; a missing seat is named and blocks.

**A local model, since build ticket 121.** `integrations/local-llm/hydra-local-llm` is a stdio
MCP server with one tool, `ask`, over Ollama at `127.0.0.1:11434` (`gemma3:1b-it-qat` by
default), prompts capped at 8,000 characters, Ollama down as a tool error. It reaches a run only
when the human adds it to `config/mcp.md`, and its README carries the block to paste, admitting
`permissive` only. Nothing in the mode depends on it (map ticket 77).

Not built: any automatic execution, which map ticket 76 makes wait for bounded retries and crash
recovery, and the fourth panel's findings on `puppets.py` (build ticket 118).

## `hydra resume`, since build ticket 32

Harness spec 5's session identity and spec 17's ninth row. `src/resume.rs`, written by
Composer 2.5 on run 01a0801d-7382, launched from a Herdr pane.

**A resume is a `hydra run` with three more fields on the request.** `resume_in` reads the
source record, refuses in one sentence a run that is still live, one with no
`provider_session_id` on line 0 or on its stream, an interactive one, and a provider with no
resume concept, then calls `run::run` with the source's provider, policy, model, cwd and
`floor_lifted`, `command: "resume"`, `resumed_from` set to the source id, and the provider's
own session id. `run_start` carries `command` and `resumed_from`, the row contract 5.1 gained
on 2026-09-08. The source record is never rewritten, and the continuation is a run of its own
with a fresh id, worktree and gate.

**The flags are measured on the four binaries**: `claude --resume <session-id>`,
`cursor-agent --resume <chatId>`, `agy --conversation <id>`, inserted before the task
string on the first two and at the front of the argv on `agy`, and `codex exec resume
<thread_id> --json -c approval_policy="never" --dangerously-bypass-hook-trust -c
hooks.PreToolUse=[...] [-m model] <task>` with no `-C` and no `--sandbox`, measured on
0.154.0 on 2026-09-22. The default task when `--task` is omitted is one line, "Continue
from where the previous run left off." A resumed `agy` run carries one `Finding::Limit`
saying its `result` usage covers earlier turns, which is the map's fog on usage scope
left as it is.

**Measured on `claude`, 2026-09-08.** Run 01a08042-7cf1 resumed run 01a07f3f-3241, a haiku
run that had been asked "what is 6+6?", with the question "what arithmetic question were you
asked earlier and what did you answer?", and the model said 6+6 and 12: `command: resume`,
`resumed_from` set, the same `session_id` on both records, `--resume <id>` on the argv,
`completed`, 23K tokens. It also came back `worktree: null`, because the source's `cwd` is
its own worktree and that worktree was reaped, so `resume` now seeds from the parent of the
source's `scopes.parent_git_dir` when the source had a worktree, and falls back to `cwd`
otherwise. `tests/resume.rs` builds the argv each provider would get and drives the four
refusals against synthetic records, launching nothing. Whether `cursor-agent`'s recorded
session id is the `chatId` its flag wants, and whether `agy`'s `--conversation` takes the
recorded `conversation_id`, are not measured on a live launch.

**Measured on `codex`, 2026-09-22.** A thread started in scratch directory A with `codex exec
--json --sandbox danger-full-access -c 'approval_policy="never"' 'print the working
directory' </dev/null` reported `pwd` as A. Resumed from scratch directory B with `codex exec
resume <thread_id> --json 'print the working directory again' </dev/null`, the turn reported
the caller's cwd as B, not A. A resume therefore runs in the fresh worktree Hydra seeds for
the continuation, not in the reaped worktree of the source run. `append_resume_args` replaces
the recorded `exec --json -C ...` argv with the resume shape above rather than splicing three
words ahead of it.

**The run that built it reached the real harness from inside the gate.** The model's first
tests called `resume()` with `Layout::discover()`, which found `~/hydra`, and three spawn
directories with a compiled gate, a `settings.json` and a `launch.json` naming `claude
--resume <fake-id>` appeared under `~/hydra/spawn/` before the tests gained a scratch
layout; no record and no provider process followed, because the seeded cwd was a reaped
worktree, and `hydra reap` swept the three directories. Harness spec 7.5 already says the
gate cannot keep a run from the binary it reaches through `shell`, and `cargo test` is that
binary with its own arguments. The rough-edge list carries it.

## `hydra export`, since build ticket 34

Harness spec 14 as the audit it says is implementable today. `src/export.rs` and
`src/normalize/paths.rs`, `tests/export.rs`, written by Composer 2.5 on run 01a0801d-7811
and corrected twice by the orchestrating session.

**The allowlist is protected config #3 and a human writes it.** `config/export-allowlist.json`
is a JSON array of absolute repo paths, read at export time only, and `hydra export`
refuses in one sentence naming the file when it is missing. Nothing creates it. Its sha
rides the output header.

**Eligibility is a list of named reasons per run**, and `--dry-run` prints them:
`harness_run`, `path_outside_allowlist`, `unclassified_path`, `unknown_outcome`,
`attachment`, `secret_flag`, `no_run_end`, `auth_failed`, `curators_child`,
`child_ineligible`, `still_running`. The path check reads every tool call through the
capability table's path field, a shell command's tokens coarsely the way the floor reads
them, and canonicalizes both sides, which was the model's one false assumption: on this
machine `/var` is `/private/var`, and without canonicalizing every run looked out of scope
and the export wrote zero lines. A raw Cursor name the normalizer could not map is
`unclassified` and the run stays home. A child's reasons are its parent's.

**One correction on outcome.** The model excluded every outcome rung one never learns from.
Contract 16.2 keys three exclusions on outcome and only one of them is an outcome:
`auth_failed`, in which no task happened. Everything else exports with its outcome
attached, gated and rate-limited runs included, because the exporter judges validity and
never quality. `Reason::AuthFailed` replaced `NeverOutcome`.

**`preference.jsonl` has a writer since build ticket 65, and a second source.** `src/export_preference.rs`
reads correction pairs off eligible runs and judge-derived pairs off the `evolve` passes: the
rejected half is the diff on `rungs/blocked/<pass-id>` or the older `ouroboros/blocked/` against the
proposer's `base_tree`, the chosen half is the diff on `rungs/<pass-id>` or in a later `master`
commit whose message names the pass, and a pass with no chosen half is reported withheld. A pair
leaves only when `config/export-signoff.json` lists the pass id, the judge run id and the finding
item a human confirmed; the file is the human's, denied to runs beside the allowlist, and its sha is
in the preference file's header. `gated` judges are read like `completed` ones. Map ticket 55. The
redaction helpers are `export.rs`'s, made `pub(crate)`; the run had copied them and the session
removed the copies before the suite.

**The seatbelt, since build ticket 68, and what it cost to measure.** `src/sandbox.rs` composes the
profile from `denied_write_paths`, parents included, then allows the own worktree and the
vendor's own data root, then denies reads of the corpus; `spawn_config` writes it as `sandbox.sb`
beside the gate and puts its sha on `run_start.sandbox`; `pty::spawn` prepends
`/usr/bin/sandbox-exec -f <profile>` on macOS and a Mac without the binary refuses the launch.
Three things the first draft got wrong, each caught by a run: it skipped a denied parent when it
held an own path, which left sibling worktrees open at the kernel, and SBPL's last-match rule
means the parent can stay denied; `codex`'s own sandbox could not apply inside Hydra's and every
command died, so it runs with its own off; and `cursor-agent` creates `cursor-data/` under the
spawn directory before the model runs, so its first `mkdir` failed with EPERM until the profile
allowed writes under `own_read_paths`. Measured after: one run per provider under the profile,
all four completed, and on `codex` a corpus write spelled as two variables failed at the kernel
while the write in its own worktree succeeded. Delegation from inside a run does not work under
the profile, by the human's choice on map ticket 64. Since build ticket 76 the delegate face
checks `sandbox::inside_seatbelt()` before it calls `run::run` and answers `hydra/seatbelt-delegation`
in Hydra's words rather than an EPERM from a corpus read. `tests/face.rs::delegation_from_inside_a_seatbelt_is_refused_in_hydras_words`
drives the built binary under `/usr/bin/sandbox-exec` with a minimal `(allow default)` profile and
reads the refusal back as a tool result; the same request outside a profile still reaches `run::run`.

**What the seatbelt is and is not, measured 2026-09-22.** `tests/spawn.rs` now drives a real
profile through `sandbox-exec` on a corpus read spelled in full, a corpus write assembled from two
variables, a write through `python3`, and a read and a write through a symlink out of the
worktree, and every one is refused; the same file drives a sibling spawn directory, which is
read-denied since that day because its `launch.json` carries the task and the whole `run_start`,
against the run's own, which stays readable and whose gate stays unwritable; one pong per provider
under the new profile completed, 01a0c7c7-b4d8 on `cursor-agent`, 01a0c7c7-d431 on `codex`,
01a0c7c7-f370 on `agy` and 01a0c7c8-12b6 on `claude`. Three things it does
not do. It is `(allow default)` behind the denials, so process launching and mach services are
open and a process launchd starts on the run's behalf does not inherit it; one `launchctl submit`
probe that day did not reproduce the class, and map ticket 67 asks what the profile is for. The
worktree's private git directory at `parent_git_dir/worktrees/<run-id>/` and the shared object
store at `parent_git_dir/objects/` are own write paths since build ticket 70, carved out of the
deny on the parent repo's `.git`. The run's ticket carved out the private directory alone and
its test could not run inside the run's own seatbelt; measured by the session on a real profile,
`git add` then failed with "unable to create temporary file: Operation not permitted", because
the blob goes into `objects/` before the index is touched. This paragraph then said a run could
add objects and not alter one because the store is content-addressed. That was wrong: map ticket
74 measured that git serves a forged pack's bytes for a sha when the pack is the only copy, and
since build ticket 103 a run writes into a store of its own instead, "Git that Hydra runs in a
run's checkout" below. `refs/` and `HEAD` stay denied, so no branch moves from
inside and `git commit` fails on its own ref, which is the line this build draws, because `hydra
reap` commits from outside and the record's diff is against the seeded tree either way.
`tests/spawn.rs::the_seatbelt_allows_git_add_in_the_worktrees_private_git_directory` drives a real
`git init`, a real `git worktree add` and a real profile: `git status` and `git add` succeed, `git
commit` and writes to the parent's `refs/heads/` and `HEAD` are refused. And a gated interactive
launch spawned `claude` bare with `sandbox: None` on its record until build ticket 73 the same day
routed it through the same profile a recorded launch gets, with `gate_sha` and `sandbox` on
`run_start`; the human measured it the same day on run 01a0c9ca-1591, which drew its interface under
`sandbox-exec`, answered and exited 0 with both shas on line 0.

**`HYDRA_ROOT` is refused inside a seatbelt, since 2026-09-22.** A `standard` run has shell,
shell reaches the binary, and `env -u HYDRA_RUN_ID HYDRA_ROOT=/tmp/x hydra run --policy
permissive` walked past both the marker check and the foreign-checkout refusal and wrote a
`permissive` record under `/tmp/x/runs/`, where the kernel denies nothing and `hydra status`
never looks. The seatbelt is inherited and cannot be shed, `sandbox_check(3)` with a null
operation says whether the caller is inside one, and `run.rs` reads it before it honours the
variable. `tests/spawn.rs::hydra_root_is_refused_from_inside_a_seatbelt` drives the built binary
under a minimal profile and asserts the refusal and that no root was created, with the same
launch outside a profile as the control.

**A launch from a foreign checkout refuses, and `hydra doctor`, since build ticket 67.**
`refuse_foreign_checkout` in `run.rs` runs after `Layout::discover` and before `layout.ensure`,
compares the checkout `paths::hydra_home` names against the root, and refuses with both paths and
the variable when `HYDRA_ROOT` is unset and they differ; nothing is written first, because a
refusal that writes into the root is the surprise itself. `src/doctor.rs` prints what a launch
would find and exits 1 when something a launch needs is missing; since 2026-09-22 the
environment allowlist is reported as in code, which it is by design, and the export allowlist and
the sign-off manifest are reported without setting the exit code, because a launch does not need
them and `hydra export` says so itself. On this machine it exits 0 now. `provider::MEASURED_VERSIONS`
is the one table of measured versions. The interactive launch does not pass through the refusal
yet. Map ticket 61.

**Redaction is the four classes and nothing else.** Repository names taken from the
allowlist entries that are not the exported run's own, replaced by a labelled marker;
secrets by the same five prefixes the write-time sweep uses, plus OpenAI keys by a matcher of
their own that the sweep does not have, six secret classes in all; the user's own hook stdout
dropped as the `hook_response` frames; and customer data inside a fixture named in the
header as the residue no pattern reaches. Section 5.2's fields are dropped with
`run_start`, `run_end`, `permission_decision` and `usage`. Every output line is then checked
again by `verify_export_line`, which since build ticket 85 has its own scan rather than the
redactor's: it decodes the JSON, reads keys as well as values, and aborts the whole export with
nothing written on a PEM header or one of the known prefixes. A long high-entropy token that no
named class covers does not abort. It is a warning, printed after the export as run id, event
field, token class and length, never the value, for the human to read before signing the
manifest. Aborting on entropy was measured first and would have stopped 108 of 321 records on
this machine, 145 of the hits being Cursor's `contentBlobId` digests; as a warning it is the same
108 records and 291 lines of output to read.

**The output is one header line and one JSON line per run**, `{"events": [...]}` in the
normalized vocabulary, to `--out <file>`. Nothing on this machine has been exported: the
allowlist does not exist, and `hydra export --dry-run` says so. Since build ticket 100
`open_events_out` refuses an `--out` that is the same file as `preference.jsonl` beside it,
compared by device and inode before a byte is written. It opens `--out` without truncating,
removes a file it created only for the comparison, and truncates only once the comparison
passes. That covers `Preference.jsonl` on a case-insensitive volume, where the name check alone
let the preference write replace the events file, and a symbolic or hard link by any name.

## Git that Hydra runs in a run's checkout, since build ticket 95

**Nothing Hydra runs discovers a repository through a checkout's `.git` file.** That file lives
in the run's worktree and the run can rewrite it; until build ticket 95, reap, the diff stat at a
run's end, rung two's patch read and rung one's proposal all ran `git` with `current_dir` set to
the checkout and nothing else, outside any seatbelt, so a run could point them at a configuration
it wrote. `worktree::CheckoutGit` holds the checkout path, the parent's common git directory and
the private directory `common/worktrees/<name>`, taken from the record's
`scopes.parent_git_dir`, or from `worktrees/<run-id>.git-context.json` saved at launch beside the
checkout and under the deny list. Every call through it sets `GIT_DIR`, `GIT_COMMON_DIR` and
`GIT_WORK_TREE` and passes `core.hooksPath=/dev/null` and `core.fsmonitor=false`. Calls in a
parent repository or the skill store still discover normally and keep the human's hooks.

**Since build ticket 99, no git Hydra starts reads global or system configuration.**
`GitLocation::command` sets `GIT_CONFIG_GLOBAL=/dev/null` and `GIT_CONFIG_NOSYSTEM=1` for the
checkout runner and the plain one alike, and `repo_root`, `git_common_dir`, `head_exists` and
reap's `drop_branch` go through it now. Commits pass the identity git would have resolved, read
as data by `worktree::identity` and put in front with `-c` by `git_as_author`, because without it
git guessed one from the hostname and committed anyway. The seed's `git diff HEAD` passes
`--no-ext-diff --no-textconv`. Git LFS's filters live in global configuration and are not read,
so a repository that uses LFS gets pointer files in its checkout. `run::refuse_launch_inside_checkout`
refuses a launch whose directory is under `worktrees/`, before discovery runs git there: the
third hiring panel reproduced a decoy `post-checkout` hook firing from a checkout's rewritten
`.git` file. `worktree::create` rolls the checkout, its branch and its context file back if any
step after `git worktree add` fails. A failed skill injection records a `skills` group naming
every skill it started, so reap holds those names back. `tests/git_config.rs` plants a global
and a system git config and checks that nothing they name runs from Hydra's git.

Since build ticket 105, three values still come from the wider configuration, each read as data
by `git config` and passed back with `-c` by `worktree::pinned_config`: `core.excludesFile`, or
git's default `~/.config/git/ignore` when it is unset, and every `safe.directory`. Build 99 had
dropped the first, so the seed copied files the human ignores only through a global excludes file
into the run's checkout and reap committed them. `core.attributesFile` is pinned to `/dev/null`,
because git reads `~/.config/git/attributes` even with global configuration off, and an
attribute there changes the bytes reap commits. An excludes file that is not a regular file
becomes `/dev/null`. Every one of these reads, and `identity`'s, gives up after five seconds,
`worktree::CONFIG_READ_TIMEOUT`, so an `[include]` naming a fifo in `~/.gitconfig` cannot hang
reap. Each git call costs one extra `git config` process for this, not measured against the
suite's time before. Hydra's commits are unsigned, because `commit.gpgSign` is global
configuration. A launch is refused under `skill-worktrees/` as well as `worktrees/`, and `hydra
evolve` refuses there before it discovers a repository. A gated interactive launch that fails
after its checkout exists rolls the checkout, branch and spawn directory back. Two failures after
`claude` has started, `procinfo::start_time` and the record's first write, used to leave no
record and `claude` running with nobody waiting. Since build ticket 112, `record_then_wait`
writes `run_start` and a harvested `run_end` whose `error` names the pid when the start time
will not read, still waits for the session either way, and `derive` reads that error as
`failed`. `tests/harvest.rs::a_start_time_that_will_not_read_leaves_a_failure_record_naming_the_pid`,
`a_record_that_cannot_be_written_still_waits_for_the_session`,
`a_recorded_start_carries_the_pid_and_its_start_time` and
`a_session_that_wrote_no_transcript_derives_failed` pin it.

- Reap refuses a checkout on any branch but its own before it stages anything, rewrites the
  `.git` file to the trusted directory because `git worktree remove` reads it, and removes the
  private `logs/` before committing: a checkout launched before this ticket could write there,
  and a link planted as `logs` would have taken reap's reflog line to any file.
- The seatbelt and the gate grant the private directory's `index` and `index.lock` only.
- A checkout with neither trusted source fails its reap with a message saying how to remove it by
  hand. No checkout on this machine was in that state on 2026-09-23.
- Replaying an old record no longer rediscovers `parent_git_dir` through the checkout for records
  written before scopes carried it; such a record reports the scope as missing instead.
- Since build ticket 103, map ticket 74's answer: a run's own git writes into
  `Layout::run_objects`, `spawn/<run-id>/objects`, through `GIT_OBJECT_DIRECTORY`, and reads the
  parent's store through `GIT_ALTERNATE_OBJECT_DIRECTORIES`. Both travel on `hydra_env`.
  `own_write_paths` grants that directory instead of the parent's `objects/`, so the parent's
  `.git` is denied whole again. The profile also allows metadata reads of the spawn parent
  itself, because git resolves the path one component at a time, and `denied_spellings` keeps
  `spawn/` on the substring list. `GitLocation::configured` removes both variables, because the
  watcher inherits them. Reap runs `import_run_objects` and then `unstage_missing_objects` before
  it commits: `git pack-objects` over the store, piped into `git index-pack --stdin --strict` in
  the parent, which names each object from its bytes, and then every index entry whose object
  did not arrive is hashed again from the checkout. Since build ticket 112, reap copies the
  store into `spawn/<run-id>/reap-import` with `copy_run_store` and runs `pack-objects` on that
  copy, which the run cannot write after reap has started, so a process the run left running
  cannot plant `info/alternates` or a link in the directory git reads. Build ticket 106 scrubbed
  the store in place; 112 replaced that with the copy. A store over `RUN_STORE_IMPORT_CAP`, 1 GiB,
  imports nothing, and `index-pack` gets `--max-input-size` at the same cap. Whatever is not
  imported is hashed again from the checkout. That window is closed;
  `tests/cleanup.rs::an_alternates_file_written_while_reap_imports_brings_nothing_in` pins it.
  The diff stat, rung two's read and `hydra status` stage from indexes
  under the spawn directory and need nothing from the run's store. Git inside a run, in any
  repository, uses that store. Measured after the merge on 2026-09-24: under `standard` each of
  the four providers ran `git add` in a real run, printed its own store as
  `GIT_OBJECT_DIRECTORY`, and `hydra reap` committed the file, `claude` 01a0d227-23ce, `codex`
  01a0d226-d533, `cursor-agent` 01a0d227-ef37 and `agy` 01a0d228-206c. None strips the variables
  from its shell tool. Until build ticket 103 the parent's `objects/` kept a full write grant.

## Rung two: `hydra evolve`, since build ticket 36

**The pass branch carries the read tree, since build ticket 89.** Hydra seeds one tree
when reading the proposer patch and derives its stat, paths and diff from that SHA against
`base_tree`. The size cap, protected-path check and judges therefore see the same tree.
`ApprovedPatch` carries that SHA, and the verdict commit names it as `read-tree`.
The pass branch is written with `git commit-tree` in the parent repository, with the read
tree and, as parent, the checkout's `HEAD` read at the same moment, before admission runs any
proposer code. Files or a `HEAD` changed during admission, or by a surviving proposer process,
therefore reach neither the offered or blocked branch's tree nor its parent; the review of the
agent's version found the parent read after admission, and `tests/evolve.rs` now moves `HEAD`
between the read and the commit. A fresh review also commits its captured source tree, on the
blocked commit's parent. Two things changed with it. `hydra/<proposer>` no longer carries the
verdict: the checkout stays as admission left it, and reaping it commits that tree to the
proposer's own branch, which is the run's output and not an offer. And the judges' diff comes
from `git diff-tree -p` of the read tree, so a rename reaches them as a delete and an add.

**The module is `src/rungs/` since build ticket 62, on 2026-09-21, and was `src/ouroboros/`
before.** razzant/ouroboros is earlier work under that name with a paper behind it, and the
README already spoke of rungs, so the human chose the word. `cursor-agent` on Composer 2.5 did
the mechanical half in run 01a0c4a6-cf6d, `git mv` so history follows; the spec file is
`docs/rungs-spec.md` and every document names it so. Two things stay as they were on disk:
the branches earlier passes wrote, `ouroboros/<pass-id>` and `ouroboros/blocked/<pass-id>`,
which `pass_run_id_from_branch` still reads beside the new `rungs/` prefix with a test on both
spellings, and the run records that mention the old name, which are evidence.

**What a judge receives, since build ticket 63.** `judge_task` in `src/rungs/evolve.rs` takes the
proposer patch and the pass's task and composes four sections in order: the task under "Intent the
patch was written against" followed by `TASK_DOES_NOT_WAIVE_CHECKLIST`, a named constant so a test
can assert it; the checklist with definitions, from `review::CHECKLIST_ENTRIES`, one table of
thirty items that `ceiling` reads too, so the prompt and the clamp cannot drift; the baseline of every
file under `tests/` the diff touches, read with `git show <base_tree>:<path>` and headed by the
instruction to compare old assertions against new behaviour; and the diff. Map tickets 53 and 57
decided it; `tests/evolve.rs` builds the prompt from fixtures and a scratch repo. Written by
`cursor-agent` on Composer 2.5 after a `codex` run under `standard` could not write at all, which
is build ticket 69's subject.

**The spawn-config writer is `src/spawn_config.rs`, since build ticket 66.** `SpawnFiles` and
`write_spawn_config` moved out of `run.rs` with a `pub use` back, so `hydra::run::write_spawn_config`
still resolves at every call site, and the file is harness spec 15.2's row 4 in
`PROTECTED_CODE_PATHS`; `src/policy/` is row 9 alone now. `run.rs` stays editable by rung two, which
build ticket 40 measured as the loop's right size. Map ticket 60. Since build ticket 96 a pass edits
it only when the human names it with `--may-edit`.

**A human asks for a fresh review, since build ticket 64.** `hydra evolve --review <pass-id>
--evidence <file>` reads the blocked patch off `rungs/blocked/<pass-id>` against the proposer's
`base_tree`, runs the four judges again under a new pass id with the evidence under
`HUMAN_EVIDENCE_HEADING` in every prompt, lands the result on a fresh branch through `commit-tree`,
and names the reviewed pass in the commit. A pass that is not blocked, or that also has an offerable
branch, is refused. `ApprovedPatch` is still built by `panel_verdict` alone. Map ticket 57.

**Passes nine and ten, 2026-09-21, and the first offerable patch.** Both were directed at the same
two-word change, "panel of three" to four in `src/cli.rs` and `src/run.rs`, with a haiku proposer
and `codex` on the panel for the first time. Pass nine, 01a0c4a6-fe42, was blocked at admission
by a corpus test that selected the ticket 06 record as "the one `standard` run with exactly two
`Bash` calls": the pass's own proposer had just written a record of that shape, both calls
allowed, and the assertion read `Allow` where it expected `Deny`. The selector names the corpus
path now and the test says why. Pass ten, 01a0c4b4-81ec, cleared admission and every judge said
pass in the closed shape: `claude` haiku in 20k tokens, `cursor-agent` in 101k, `agy` in 141k
with one minor finding that read the four as a contradiction of a spec it had not seen updated,
and `codex` in 28k, its first answer as a judge, a bare JSON object with no prose around it. Two
judges came out `gated`, `cursor-agent` and `agy`, the way judges on those providers have since
pass seven, because `judge` grants nothing and both try a read before answering. The patch sits on
`rungs/01a0c4b4-81ec-7752-ab39-451ff5dfc144`, spec 4.11, and nothing here merges it. The human
did, on 2026-09-21, as commit 24cade6: the two files taken off the branch, two lines, the first
change to land through rung two. A plain `git merge` of a pass branch conflicts, because the
branch carries the proposer's whole worktree as it was seeded, so the merge was a checkout of
the files the panel judged; the branch stays, since git does not see it as merged.


Rungs spec 4 as written, with the three details map ticket 52 closed. `src/rungs/evolve.rs`
and `src/rungs/review.rs`, `tests/evolve.rs`, written by Composer 2.5 on run 01a08053-3d85,
launched into a Herdr pane. **Six passes had run on this machine when this was written, and eight as of 2026-09-12**,
the sixth through
the full panel; the paragraphs below say what each one found.

**A pass is a harness run with four children.** `evolve` mints a run id with `command: evolve`,
writes contract 5.5's two-line record, and every child carries it as `parent_run_id`: one proposer
and four judges since build ticket 61, three before it. The proposer is a recorded `claude` run under `standard` in a worktree of this
repo, its task the brief spec 4.1 describes, composed by `brief()` from the same corpus selection
rung one reads. `--model` names the proposer's model and nothing chooses one on the human's behalf.

**These refusals happen before any judge runs, in this order.** The proposer's diff is read
against `base_tree` the way `hydra reap` reads one; a diff over `PATCH_LINE_CAP`, 400 lines
changed, insertions plus deletions, is rejected for size and the proposer is told to split it.

**Since build ticket 96 the human names what a pass may edit, over a floor.** Map ticket 73.
`hydra evolve` takes a repeatable `--may-edit <path>`, a file or a directory prefix relative to
the repo root. With none, `DEFAULT_MAY_EDIT` applies: `src/normalize/` and `src/show.rs`.
`may_edit_set` runs first in `evolve`, before the layout is read, and refuses the pass when a
name reaches the floor: a floor path, a path inside one, a directory holding one such as `src`
or `tests`, a file called `build.rs`, or a name outside the repo or equal to it. Then
`admission_refusal` reads the patch's `changed_paths` in three steps, and the first hit lands the
pass on the blocked branch with its reason. `diff-tree` runs without rename detection, so a
rename lists both of its paths.

1. The floor, relative to the repo root and case-insensitively: `policies/`, `config/` and its
   four files, `PROTECTED_CODE_PATHS`, and any file called `build.rs`. Until 2026-09-12 only the
   config rows were there, and a patch to `src/rungs/review.rs` reached the panel; build ticket
   40 added the code rows of harness spec 15.2, `src/rungs/review.rs`, `src/rungs/evolve.rs`,
   `src/policy/`, `src/record/` and `src/env.rs`, and `src/spawn_config.rs`, `src/sandbox.rs`,
   `src/containment.rs` and `tests/corpus` followed. Ticket 96 added `src/pty.rs`,
   `src/watch.rs`, `src/paths.rs`, `Cargo.toml`, `Cargo.lock` and `.cargo/`. The build files are
   there because the human's own `cargo build` runs them unsandboxed after a merge. The doc
   comment on the constant says which file each role of the spec's table maps to.
2. Build ticket 80: a changed path naming a file under `tests/` at `base_tree`, listed with
   `git ls-tree`, is refused whatever the set says. An edit, a delete, a rename or a mode change
   to a baseline test never reaches a judge. A new test file passes where the set allows it.
3. The set: a changed path no named entry holds is refused. `src/run.rs` is off the floor, as
   harness spec 15.2 and map ticket 60 keep it, so it is refused unless named.

The pass's harness `run_start` carries the set as `may_edit`, a list of paths. Contract 5.5
documents the field, and the validator checks its type when it is there. The brief lists the
set and names both refusals, so a proposer does not spend a pass on a file it cannot edit.
Map ticket 73's answer leaves two things open: a named file is fully editable, and a unit-test
probe inside one can be weakened along with the code it guards. A fresh review with `--review`
runs the same three refusals on the blocked patch, under the set its pass recorded or the
default for an older pass, and refuses before any judge; until the review of ticket 96 it ran
none, so a blocked `Cargo.toml` could have come back offerable. The comparison the floor and the
set use is a copy of its own in `evolve.rs`, because `src/normalize/` is in the default set, and
`existing_test_hit` reads `base_tree` through the checkout's pinned git directory with replace
refs off. `rust-toolchain` and `rust-toolchain.toml` are on the floor with the build files.

Then the admission gate runs `cargo fmt --check`, `clippy`, `build`
and `test`, in the proposer's worktree until build ticket 98 and on a copy of the read tree
since, and a failure is a rejection carrying the output. **Since build
ticket 75 on 2026-09-22** each step on macOS runs through `/usr/bin/sandbox-exec -f` with the
profile the proposer's spawn already wrote at `spawn/<proposer-run-id>/sandbox.sb`, and with
`HYDRA_ROOT` pointed at an empty scratch root under `spawn/<pass-id>/admission-root/` so the
corpus walks do not read a corpus the profile denies. On Linux the same four commands run bare
with the same variable. **Since build ticket 90** both branches share one tail that clears the
environment and passes `ADMISSION_ENV_ALLOWLIST` from `src/env.rs` plus `HYDRA_ROOT`; until then the
steps inherited the orchestrator's whole shell, the second hiring panel's G43. A missing profile on macOS fails closed naming the path rather than
running bare. The pass's harness `run_end` carries `admission_sandbox`, the profile sha, and
`admission_hydra_root`, the scratch root. A hostile test can still read `~/.ssh`; map ticket 66
closed on that honest step. `tests/evolve.rs::admission_runs_under_the_proposers_seatbelt_and_refuses_a_hostile_test`
plants `tests/zz_hostile.rs` in a scratch worktree, asserts `cargo test` fails and
`<layout>/runs/hostile` does not appear, and returns early when `sandbox::exec_available()` is
false or the test process is already inside a seatbelt, because nested `sandbox-exec` is refused. Each refusal lands on `ouroboros/blocked/<pass-id>` with its reason in the commit
message, spec 4.9.

**Since build ticket 98 the four steps run on a copy of the read tree, not in the proposer's
checkout.** Until then they ran where the proposer, or a process it left running, could still
change the bytes after the read, and read a `~/.cargo/config.toml` the proposer could write,
whose `runner` turned a failing `cargo test` into exit 0: the third hiring panel's H38 and H2.
`prepare_admission` writes `patch.tree` byte for byte from the object store to
`spawn/<pass-id>/admission/tree` after the proposer's run ends, with `git ls-tree` and `git
cat-file --batch`. No filter, attribute or export rule applies. A submodule, or a symbolic link
that is absolute or climbs with `..`, blocks the patch. A file the proposer's checkout ignores
is not in the tree, so admission no longer compiles it, which closes H19. The proposer's profile
denies that directory to reads and writes, and a process the proposer left running keeps its
profile, so nothing done after the read reaches what admission compiles. Nothing in `src/` stops
those processes; the copy's location is what makes that unnecessary on macOS. The steps run
under `spawn/<pass-id>/admission.sb`, beside the admission directory and outside what the steps
can write (build ticket 105; 98 had put it inside). It is the proposer's profile plus three
things: `admission/` opened, metadata of each ancestor and its direct children opened, and in
every ancestor of the copy, reads of `.cargo/config`, `.cargo/config.toml` and the `.cargo` name
itself denied. The `.cargo` holding the toolchain keeps its name readable. Each deny is written
twice, for `file-read*` and for `file-read-metadata`, because a seatbelt rule on
`file-read-metadata` outranks one on `file-read*` whatever the order, which build ticket 105
measured with a `.cargo` swapped for a link. Cargo reads a `config.toml` from each ancestor of its
working directory as well as from `CARGO_HOME`, and `~/.cargo` is one of those ancestors.
`CARGO_HOME` is an empty `admission/cargo-home`, and `CARGO_HOME` left `ADMISSION_ENV_ALLOWLIST`.
`cargo` is called as `<sysroot>/bin/cargo`, with the sysroot read from `rustc --print sysroot`
before the proposer launches, and that directory goes first on `PATH`. `admission_sandbox` is
the sha of `admission.sb`. A copy that cannot be written blocks the patch like a failing step,
and the pass record still closes. The copy and the cargo home are removed after the steps
finish. Every pass needs the network: `CARGO_HOME` starts empty, so cargo downloads the crates in
`Cargo.lock` each time, and the four steps took 416 seconds on this repository in build 98's
measurement. Admission needs `rustc` on every platform, Linux included, because the toolchain
is found with `rustc --print sysroot`. What stays open: the toolchain's own files are writable
by the user when Homebrew installed them, and `cc`, `git` and `python3` still resolve through the
orchestrator's `PATH`. Both are the `PATH` decision the third panel's H2 names. `HOME` still
reaches admission, so `~/.cargo/credentials.toml` is readable, the home-readable caveat of
build ticket 90. On Linux no seatbelt stands between a leftover proposer process and the copy.

**Admission runs the whole suite inside a seatbelt, and since build ticket 104 it passes
there**: all four steps passed under a real admission profile on a copy of this repository in
515 s. From build ticket 75 to 104 two tests failed inside any seatbelt, so every rung-two pass
on macOS was blocked at `cargo test`. Three tests cannot mean anything inside a seatbelt, so they
skip and say so on stderr. `procinfo::tests::our_own_start_time_matches_ps` skips because the
seatbelt will not exec setuid `/bin/ps`. Two tests in `tests/delegation.rs` skip because the
binary refuses `HYDRA_ROOT` inside a seatbelt. Outside a seatbelt all three run. The seatbelt
probes of `tests/corpus/mod.rs` skip inside a seatbelt too, because `sandbox-exec` does not
nest, so no probe of `tests/spawn.rs` or the corpus runs during admission (the third panel's
H35); CI and a human's `cargo test` are where they run.

**Since build ticket 101 the path after the proposer is a function a test can drive.**
`land_proposer_patch` and `review_blocked_in` take the layout, the repository and the judge
panel as arguments; `hydra evolve` passes the real ones. `tests/evolve.rs` runs both on a
scratch layout with a panel closure: a fresh review refuses a floor path, and a path outside the
set its pass recorded, before the panel, and a pass whose checkout changes after admission
still offers the tree it read. That is build 89's tree half run rather than reasoned.

**The panel is one `judge` run per provider**, `claude`, `cursor-agent`, `agy` and `codex`, each handed the
whole diff and the checklist of spec 5 in the prompt, no skills and no MCP as `judge` already
runs. Each answers one closed JSON shape, `{"verdict", "findings": [{"item", "severity",
"note"}]}`, and `review::parse_judge_answer` takes the whole response as that object: prose
outside it, a missing field or a wrong value is a block carrying the parse error, never a pass
and never a retry. Spec 5.1's ceiling clamps a reported severity to the checklist item's own, and
one critical finding blocks whatever the `verdict` field says. Spec 4.7's abort reads the newest
`claude` record in the corpus, which is the proposer the pass just ran, and takes the `status`
off its last `rate_limit_event` frame before any judge spawns. Until build ticket 41 closed on
2026-09-12 it read `raw.status`, a field no frame carries, so every pass under a rate-limited
`claude` spent a proposer to find out. `evolve::rate_limit_abort` now reads
`rate_limit_info.status` and falls back to the top, and `tests/evolve.rs` drives both shapes,
the `allowed` case and a run warned early and allowed later, on synthetic records. Since
2026-09-20 that abort ends the pass `completed` on `ouroboros/blocked/<pass-id>` with the reason
in the commit message, the same shape as a proposer whose outcome is `rate_limited`. Until then it
ended the pass `failed`, which spec 4.10 reserves for git or the worktree breaking; the review of
2026-09-19 found the two paths disagreeing and neither this file nor the spec named it.

**The verdict lives in the commit message**, spec 4.9, on `ouroboros/<pass-id>` when the panel
passed and `ouroboros/blocked/<pass-id>` when it did not, naming the pass, the proposer and the
four judges by run id, with the four answers in the body. A pass constructs `ApprovedPatch`
only through `panel_verdict`, which is the one function that can build it, and since build
ticket 82 `panel_verdict` takes a `PanelAnswers` with one slot per `Provider`, filled by
`run_judge_panel` from a run of that slot's binary, so a panel of four answers from one binary
cannot be built; a list-shaped answer that fails that shape ends `Blocked`. Nothing merges:
the human does, spec 4.11. A pass's outcome describes the pass and not the patch, spec 4.10, so
a refused or blocked patch is a `completed` pass and `failed` is git or the worktree breaking.

**The first real pass, 01a08129-de0c on 2026-09-08, found five things before any judge ran.**
The human chose a haiku proposer. The pass before it, 01a08127-6f58, never spawned the
proposer: `Lineage::of` refused a parent with no policy, and a harness run has none, so a
harness parent now bounds nothing, its authority being the human's who typed the pass, and
`tests/delegation.rs` pins it. The second pass spawned the proposer and `hydra status`
reported the pass abandoned while its process was alive, because the pass wrote its start
time at millisecond precision and `procinfo::is_alive` compares exact equality against the
microseconds `watch.rs` writes; microseconds now, and no fallback to the clock. The proposer
was seeded with the orchestrating session's uncommitted edits, read them as the patch under
review, and undid them, which left its tree equal to HEAD; `commit_everything` found nothing,
and the blocked branch pointed at master with the verdict nowhere, so the pass now makes an
empty commit carrying the message when the tree is clean, spec 4.9's one branch (since build
ticket 89, `commit-tree` makes one whether or not the tree changed). The
admission gate then failed, on `cargo fmt --check` and `clippy -D warnings` against master's
own code rather than the proposer's: master did not pass the four commands spec 4.5 names, so
no patch could have. Master is formatted and clippy-clean since that pass. And the closed
pass record derived `crashed`, the streamed reading of a missing terminator, where spec 4.10
says a pass whose gate did its job is `completed`; a harness run now derives `completed` from
its own `run_end`, `failed` when that carries an error. The proposer's own record then failed both corpus walks
on master: a `Bash` the binary's own guard refused with `<tool_use_error>Blocked: sleep 70
...` before any hook, read as `Ungated`, and an `Edit` sent with no `file_path` that
`InputValidationError` refused, read as a `GateLeak`; the `claude` adapter now treats the
`Blocked:` spelling like the validation error it already knew, and a `no-path` deny with a
provider error as a limit, the rule the Cursor adapter gained the same day. And the walk
that checks every record normalizes had no branch for a harness run. Two lessons for whoever
types the next pass: commit first, because the proposer inherits the uncommitted tree as its
seed and reads it as work in progress; and the pass cost 5.77M tokens on haiku for a patch
that undid a diff, which is what a proposer given no task beyond the corpus brief does.

**The second real pass, 01a0814a-c730 the same night, reached the admission gate with a
real patch and was blocked by the gate's own spelling.** With the tree committed first, the
haiku proposer (5.18M tokens) read the brief, found the leak walk failing on two old records
inside its worktree, the same failure every agent since ticket 31 had reported, and wrote
fifty lines across the three normalizers to make the replay refuse a shell command naming a
protected path. `cargo fmt --check` passed. `cargo clippy -D warnings` failed with "unexpected
argument '-D'", because the gate ran it without the `--` cargo needs before clippy's own
flags; fixed. The verdict commit landed on `hydra/<proposer>`, the design then and not since build ticket
89, and the blocked
branch then named master's HEAD instead of it, because `commit_pass_branch` resolved `HEAD`
in the parent repo; it resolves the checkout's now. The pass record read `completed` and
`hydra status` reported it live while it ran, so two of the first pass's fixes held. Haiku's
patch is on `ouroboros/blocked/01a0814a-c730` for whoever wants to read what a proposer does
with the corpus brief and no task: it went after the one red test it could see.

**The third pass, 01a08169-eb3b, found the one that made every pass impossible.** Its haiku
proposer met the five-hour window with zero files changed, the pass put that empty diff
through the admission gate anyway, and `cargo test` failed on `tests/record_contract.rs`. The
cause is the pass reading its own record: the admission gate runs the suite, the suite walks
`~/hydra/runs/`, and the pass's harness record is open at that moment with one line, which
the validator refused because contract 5.5 says two. Contract 6 already says a run with no
`run_end` is not corrupt, so an open harness record is a pass still running; one line is
allowed now and three is not. Without that, no pass could ever pass its own admission gate.
Beside it, a proposer that met its limit or lost its login now ends the pass before the gate:
spec 4.7 aborts a review a judge cannot answer, and an empty diff from a rate-limited or
logged-out proposer is the same shape, so the branch carries the reason and no `cargo` command
runs. A `failed` or `timed_out` proposer still reaches the gate, because either may have
written a real partial patch and the gate is what says so.

**The fourth pass, 01a0822e-bc48, was blocked by its own proposer's trajectory.** The haiku
proposer spent 8.07M tokens, invented a lowercase `bash` tool, and `claude` answered "No such
tool available: bash". The gate was never asked, the table is right not to know an invented
name, and the replay filed the call as both `Ungated` and a `GateLeak` for the unknown tool;
the leak walk then failed inside the worktree and the admission gate blocked the patch. A call
the provider refused on its own is a limit on both paths now, the third spelling of a rule that
already covered `InputValidationError` and the binary's own `Blocked:`.

**That is the structural fact worth writing down, rather than the third spelling.** The
admission gate runs `cargo test`, this suite walks every record under `~/hydra/runs/`, and the
proposer's own record is one of them, live, while the gate reads it. So a pass judges its
proposer's trajectory as well as its patch, and a model that types something odd blocks a
patch it never touched. Three of the four passes so far died there: the open harness record,
the invented tool, and the walk that had no branch for a harness run. Each was a real reading
bug worth fixing, which is the argument for leaving it as it is; the argument against is that
the corpus grows with every run and the gate's failure surface grows with it.

**The fifth pass, 01a08241-831d, ran the panel.** Three judges answered for the first time:
`claude` on haiku for 29K tokens, `cursor-agent` for 105K, `agy` for 200K, the last two coming
out `gated` because `judge` allows nothing and their diff rode the prompt, which is ticket 47's
design working. Both of those wrote real reviews with checklist items, severities and notes.
The verdict was blocked and three things came out of it.

**The `claude` judge returned the exact object inside a ```json fence** and the parser called
it a syntax error, so a clean pass was recorded as a block. Map ticket 52's answer is amended:
one fence wrapping the whole answer comes off, `review::strip_one_fence`, and prose beside the
object, two fences, an unclosed fence and a fence tagged anything else all still block.

**Both parsing judges spent their review on the human's own page.** The proposer had reverted
153 lines of `docs/status.html`, which is uncommitted on this machine and therefore seeded into
every worktree, and the diff read against `base_tree` counts undoing it as the proposer's
change. The brief now tells the proposer that the seeded tree carries the human's uncommitted
work and is not its to revert, tidy or finish. Nothing protects that file from a proposer that
ignores the sentence; the gate cannot, because the file is inside the worktree the run owns.

**That page left the repository on 2026-09-13.** `docs/status.html` is gitignored now and untracked, and `seed_tree` stages with `git add -A`, which honours `.gitignore`, so it no longer reaches a worktree at all. The paragraph above is history: the failure it describes cannot happen again to that file, and the brief's rule about leaving the seeded tree alone stands for every other uncommitted file.

**And the `agy` judge read the prose against the code and found a gap.** `IMPLEMENTATION.md`
said a proposer that could not run ends the pass before the gate, and the code checks
`rate_limited` and `auth_failed` alone, so a `failed` or `timed_out` proposer still reaches the
admission gate. That is deliberate, since a timed-out proposer may have written a real partial
patch, and the sentence above now says which two outcomes it covers.

**Two things the brief shows that are worth knowing.** It lists every run rung one may learn
from with its outcome and `outcome_source`, and on this machine that second column reads
`unknown` for a run carrying one unclassified tool result, contract 8.2. On the day the
brief first printed, the Sonnet run 01a07f37-9cab read that way with ninety-two of them, a
`claude` normalizer gap build ticket 37 closed the same day; twelve runs still read `unknown`,
the Cursor ones carrying an `unknown_tool` and the oldest records. And the brief hands the proposer run ids it
cannot open, since `~/hydra/runs/` is denied under every policy; that is the read-only brief of
spec 4.3 as a convention, which map ticket 52 kept.

## Rung one: `hydra curate`

Ticket 12. `src/rungs/` is the whole of it. `corpus.rs` decides which runs the
curator may learn from, `curate.rs` composes what it is told, launches it as an
ordinary recorded run under `curate`, and commits whatever that run wrote into its
checkout of the skill store onto `ouroboros/curate/<run-id>`. `tests/curate.rs`
drives both halves and launches no provider.

**The curator's run writes `command: curate`, since build ticket 42 on 2026-09-12.**
Contract 5.1 said so from the start and `curate.rs` wrote `run`, so a curator record from
before 2026-09-12 carries `run` and a reader has to accept both. Nothing switched on the
old word: `validate.rs` takes all four values, `corpus.rs` keys on `provider`, `run.rs`
keys on `evolve` and `normalize/agy.rs` on `resume`. The launch is built by
`curate::request`, and `tests/curate.rs` reads the field off it and off the line 0 a
launch writes from it, without launching.

**The corpus is read through the normalizer and never off `raw`.** Contract 11 is
the reason: an adapter fixed next month reads last month's records correctly
because nothing was migrated, and a curator that greps the stored frame gets none
of that. `tests/curate.rs` also checks that a walk changes no byte under `runs/`.

**Three filters, each a field rather than a guess.** A harness run goes on its
kind, `provider: null`, which is contract 8.6 and is what keeps an interrupted
`hydra evolve` from reading as an abandoned run: it is a dead pid with no
`run_end`, which is exactly the shape rung one's first class looks for. Contract
8's right-hand column goes on `Outcome::rung_one_learns`. A run that is still
writing is skipped before its file is opened, because the liveness test of 5.2 is
cheaper than a normalize and the record would look different in a minute.

**A fourth filter is not in the design, and the corpus is what made it
necessary.** A recorded run with a null policy predates the gate, so contract 7.7
derives no `permission_decision` for it and a refusal inside it is invisible:
`01a07218-8817-7581-9ddb-47565e5d5ba7` is one `Write`, refused, derived
`completed`. **The test is a null policy beside `launch_mode: recorded`, and
dropping the second half would take the interactive population with it**, since an
interactive run carries a null policy legitimately (5.6) and is the only source
rung one's human-rejection class has.

**Skipped is not thrown away and the brief says so.** It carries a count per
reason, because a curator told only what it may learn from cannot tell an absent
signal from an absent producer. Two of the five classes have no producer in this
build at all, human rejection and the retry loop, and the brief names them as
that rather than printing a zero somebody could read as a measurement.

**The curator writes into a git worktree of the store and never into
`~/hydra/skills/`** (rungs spec 3.6), so the run's `skill_checkout` scope
points at that checkout, a scope of its own since build ticket 14 rather than a
second meaning for `skill_store`. `hydra::run::Request` gained one field for it,
`run_id`: the checkout is named after the run, so the id has to exist before the
gate is compiled, which is the one thing `hydra run` mints for itself and this
caller cannot.

**Where that scope pointed was a rule on the layout rather than an argument, and a
real record is why.** `Layout::skill_store_scope` took a run id and a policy name
and answered with the checkout for `curate` and the store for everything else.
Both sides read it: the launch compiled the gate against it, and contract 7.7's
read-time re-evaluation rebuilt it. The first version passed the path in at launch
and left the reader to guess, and the pass measured below came back with ten
`Finding::GateLeak`, one per file the curator wrote or probed inside its own
checkout, because the reader was still resolving `skill_store` to `~/hydra/skills/`.
Nothing on the record carried a scope and inventing a field was a contract change,
so the fix was a rule both sides derived from the run id. The corpus-wide
gate-leak check in `tests/normalize.rs` is what caught it, and `tests/curate.rs`
pinned the round trip.

**Build ticket 14 split that one rule into two, on 2026-09-06.** `skill_store`
means `~/hydra/skills/` on every spawn now, no exceptions, and the policy-name
match inside `src/paths.rs` is gone with it. `Layout::skill_checkout(run_id)`
answers the checkout's path as a pure function of the layout and the run id, and
whether a spawn gets one at all is a decision each caller makes from the policy
name: `src/run.rs`, `src/rungs/curate.rs` and `ReplaySpawn` all pass
`Some` on `curate` and `None` on everything else, and since build ticket 15 the
read side asks that question only of a record carrying no `scopes`. `Spawn`
carries the result as
`skill_checkout: Option<&Path>`, and the deny list denies the store outright under
every policy, `curate` included, which is what makes the two scopes different
names for different places rather than one name that used to move.

**The checkout lives at `~/hydra/skill-worktrees/<run-id>/`, which is a sixth
place and a choice.** Section 9.1 leaves the location open and says the spawn
directory would do. It will not: `hydra reap` removes that directory whole, and a
pass whose run outlived its call leaves an uncommitted proposal in the checkout.
Work is not something a cleanup command should be able to take. `hydra curate`
was the only thing that created or removed one until build ticket 14; `hydra reap`
sweeps a leftover checkout now too, committing to `ouroboros/curate/<run-id>`
first, so a proposal a curator never came back to collect is not lost.

**The commit is the pass's only post-run action, which is why the pass gets no run
record of its own** (contract 5.5). Its order is git's guarantee rather than a
sentence here: `git worktree remove` without `--force` refuses a checkout with
changes in it, so a commit that did not happen stops the removal instead of
preceding it, exactly as in `hydra reap`. A pass that proposed nothing has its
branch deleted; a pass that committed keeps it, and since 2026-09-12 the pass
decides which by comparing the branch's tip to the sha the checkout started
from rather than by letting `git branch -d` refuse. The old way was correct in
outcome and wrong in report: the first real pass, `01a08d66-76fd`, kept its
branch and said `left behind: not fully merged` about its own proposal.

**The commit message keeps Hydra's facts and the model's words apart.** The
curator's rule against taking an intent declaration from a summary afterwards
exists because a model's free text stopped matching what it did. A pass commits
after the run, so a summary afterwards is all there is; what it can do is label
the halves, and it does.

**`hydra curate` prints text rather than contract 17's JSON.** That projection
belongs to `hydra run` and the delegate face, where a caller switches on it. This
command is typed by a human like reap and prune are, and its answer is a branch.

**A curate run still gets a checkout of whatever repo you typed the command in**,
because harness spec 6.3 gives every recorded run one and this is an ordinary
recorded run. Under `curate` nothing may write to it, so it is a checkout the run
cannot use, and `hydra reap` is what removes it.

**Measured on this machine, 2026-09-06.** Thirteen records: 6 learnable, 7 skipped
as 3 pre-gate, 3 `gated` and 1 `rate_limited`, and the walk costs 0.55 s end to
end including process start. One full pass ran against a real `claude`, run
`01a07513-baa3-7cf0-b20a-ded11ede6a38`: `completed`/`unknown`, 489,440 tokens, and
it proposed nothing. It read the six records, declined to propose a skill edit for
the one denied `Bash` call because the brief says a denial is a policy bug, ranked
the one `unclassified` result without diagnosing from it, and said the store held
no skill to edit. The empty branch was deleted, which is the shape above working
rather than a pass that failed.

**The pass pins the store before it checks out, settled by map ticket 41 on
2026-09-06.** 6.4 has a launch commit a dirty store before pinning it, and `hydra
curate` checks out before it launches, so until this the curator read the store as
it was before its own launch tidied it. Now `curate` calls `Store::pin` with the run
id it minted and checks out at the sha it gets back; `run::run` pins a store that is
already clean and gets the same sha. Chosen over moving the checkout after the
launch because ticket 40 left the store two writers, a human and `Store::pin`, so
the pass committing is Hydra doing the write it already reserved for itself. The
trailer on that commit names a run that has not started yet, which is accepted.
`tests/curate.rs` covers the one shape where the orderings differ, a pass on a dirty
store. The pass measured above had hit the harmless half, an empty `HEAD` under a
store whose skills were not committed.

## Choices this build made that no ticket settled

**`--policy` defaults to `standard`.** Section 9 makes the flag optional and names no
default. `standard` is the ordinary coding shape and the only one of the five that fits
a run a human typed without thinking about policy. The alternative worth naming is
having no default and refusing without the flag, which is safer on paper and would mean
every `hydra run` carries a flag whose value is `standard` in almost every case.

**A run with no repo scopes `worktree` at a path nothing creates.** `hydra run` outside
a git repo has no checkout, and the child runs where Hydra was invoked. Pointing the
`worktree` scope at that directory would hand the run write access to whatever the human
happened to be standing in, so it points at `~/hydra/spawn/<run-id>/no-worktree`, which
nothing creates. Under `standard` that scopes reads to the same missing directory, so
such a run reads nothing through the file tools either, and shell is what it has left.
That is the honest reading of D9: Hydra writes only under a path it created, and a run
with no checkout has none.

**Reap deletes a branch only when git says nothing would be lost, and a flag goes
further.** Harness spec 10 and 17 both stop at the worktree and the spawn directory, and
neither says what becomes of the branch. Committing to it and then deleting it would make
the commit theatre, which is the failure the section is about, so the default is
`git branch -d` and `--delete-branch` is the human's way past it. Ticket 06's "done when"
asked for the thirteen branches on this machine to be gone by the command; they were kept
instead, and the ticket's outcome says why.

**The spawn directory's filenames are `gate` and `settings.json`.** Harness spec 19.1
leaves the names inside the spawn directory open. `settings.json` comes from 6.6's own
table; `gate` comes from 7.5's prose. Neither is a ticket's decision.

**The preamble and the caller's task are joined by a blank line and nothing else.**
Section 8 allows the preamble three assertions and no more, and a marker line saying the
task follows is a fourth sentence in Hydra's voice. The cost is that a reader of
`run_start.task` has to know where the preamble ends, and the preamble is a fixed string
in `config/preamble.md`, so they can.

**The concurrency cap is eight.** Harness spec 9 asks for "a cap on live children" and
names no number. Eight is more than the parallel tool calls an orchestrator issues in one
turn and few enough that eight `claude` processes on one subscription is a bill you notice
before the machine does. It refuses rather than queues, which is what section 9 asks for:
a queue holds a slot the caller can neither see nor cancel. The refusal is a type,
`live::TooManyLive`, carrying the run ids, because a caller that hits it has to decide
between waiting on one and reaping it and neither is decidable from a sentence.

**Since build ticket 81 the cap is a reservation, not a count.** `live::reserve` takes an
`flock` on `<root>/reservations/.lock` and holds it across reading the pending reservations,
scanning the live runs and renaming `<run_id>.json` into place, so two launchers that see one
slot are serialized and the second sees the first's file. The file holds the `hydra run` pid
and its start time; the guard drops it once line 0 carries the child's pid, and a file whose
launcher is dead, reused or unparseable is removed by the next reservation. The directory sits
beside `spawns/` rather than inside it, because `hydra reap` treats every directory under
`spawns/` as a run's and would have deleted the lock (`tests/cleanup.rs::a_sweep_leaves_pending_reservations_alone`).
It is on `denied_write_paths`, so the seatbelt denies it too. Interactive launches reserve
nothing, but they are counted because an interactive launch writes line 0 with the child's pid
right after the spawn and `live::reserve` counts every live view. The uncounted window is
between the spawn and line 0.

**A run refused by the cap writes no record.** Contract 8.3 makes a refused spawn a record
when the refusal is one of section 6's four codes, and the cap is not one of them. Minting
a fifth is a contract change rather than an implementation, so the refusal goes to stderr
and no run id is minted.

**Two launches racing for the last slot cannot both take it.** The count comes from a
directory scan, and a lock on `<root>/reservations/.lock` is held between that scan and the
reservation. With seven live, the first launcher to take the lock renames its reservation into
place, and the second reads that file under the same lock and is refused
(`tests/status.rs::concurrent_launchers_reserve_exactly_the_cap_without_records`). The cap is a brake on an orchestrator issuing
parallel tool calls, not a quota anyone is defending.

**`--timeout` takes `600`, `30s`, `10m` or `2h`, and defaults to `10m`.** Section 9 says
"roughly ten minutes" and names no spelling. Bare digits are seconds, which is what every
other tool on this machine means by a naked timeout. Days are not a unit: a call nobody is
waiting on for a day should have returned a handle and been polled. `0` is legal and means
launch and hand me the handle, which follows from nothing here ever killing a child.

**The view polls the run file every 100ms.** So a blocking call can add up to that after
the child exits, which is noise beside the record read that follows it. Watching the file
with a kernel notification would remove it and would put a platform dependency on the one
path a test can already drive by hand.

**The cap's refusal names `hydra logs --follow` and `hydra status`, not `hydra reap`.**
The count is of live runs, so a run that has ended has already given its slot back and
reaping it frees nothing. `reap` did not exist when this was written; it does since build
ticket 06, and the refusal still does not name it, for the first reason.
A refusal that tells you to run something that fails twice over is worse than one that
just says no.

**The spawn directory holds two more files: `launch.json` and `watcher.log`.** Harness
spec 19.1 leaves the names inside it open. Both die with the worktree under `hydra reap`,
and neither is read to interpret a run.

**Worktrees live at `~/hydra/worktrees/<run-id>/`, on branch `hydra/<run-id>`.** No
closed ticket names the location, which is the same gap harness spec 19.1 flags for
the filenames inside the spawn directory. A Hydra-owned path keeps D9 true when the
repo being worked on lives somewhere else. Confirm it rather than inherit it.

**`run_start` is written after the spawn, not before it, and by the detached half.**
Contract section 5 says the line is written before the child process starts and 5.2 puts
the child's pid on it. Both cannot hold literally. What run_end's derivation leans on is
that the pid is in the file, and a pty buffers everything the child writes until the tee
reads it, so spawning first loses nothing and `run_start` is still line 0. The launcher
composes every other field on it; `pid` and `process_start_time` are the two the half
that owns the process fills in.

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

**The result is read back off the file rather than assembled in memory.** Contract 17
makes it a projection of the record, and `hydra run` still has every value in scope when
the child exits, so building it from those would be faster and would also let the JSON
carry a fact the record does not have. Section 6 says that is the mistake ticket 11 made.
Reading the file is what keeps the projection true rather than intended, and it costs
what the section below measures.

**Two refusal codes are minted here, not in a ticket.** Contract 6 names four Hydra-side
checks in prose and no closed ticket gives them strings. `record::Refusal` is the
vocabulary, one code each, in the shape ticket 01 gave the gate's `hydra/` reason prefix
and for the same reason: the derivation matches on it, and a sentence is not something to
match on. All five are written now: `skill_collision` first, then `policy_narrowing`,
`mcp_unavailable`, `containment`, and since build ticket 48 `auth_preflight`, on `codex`
only. Records carry them, so the spellings are settled.

**`hydra reap` is what removes a skills snapshot.** Harness spec 5 says a snapshot is
reaped when no live run references it and names no command. Reap is the one already typed
by a human that already removes a run's scratch.

**The store's pin commit is attributed to `hydra <hydra@localhost>` with the run id in a
trailer.** 6.4 asks for authorship in the trailer and does not say whose. Hydra cannot
know which run dirtied the store, so the trailer names the run that turned the working
tree into a commit.

**The store's validator holds two rules the file format does not.** The frontmatter
`name` must equal the directory name, and `disable-model-invocation: true` is refused.
The section on the skill store above gives the reason for each. Both are stricter than
6.4 says in words and both follow from what it asks the store to do; confirm them rather
than inherit them.

**The skill probe leaves a transcript under `~/.claude/projects/` per run and nothing
reaps it.** That is the user's own directory and Hydra writes nothing else in it.

**The canonical MCP document is Markdown with one fenced `toml` block in it.** 6.5 fixes
the file's name, says it is none of the three provider files, and requires a per-server
admission field keyed on a policy name. The rest was open. Markdown because the file is
protected config #5 and a human edits it by hand, so the format that carries the rules
beside the data wins; TOML for the block because the skill store's `hydra.toml` sidecars
already use it and a table of servers reads well in it. One block rather than one per
server, so a broken fence is an error rather than a server nobody notices is missing.

**`mcp.doc_sha` is a sha256 of the whole file.** The section above has the argument and
the alternative it beat.

**A server name may not carry `__`, and an unknown field is refused.** Neither rule is in
6.5. The first is what makes `mcp.resolved` exact rather than a guess, since the free
answer is `mcp__<server>__<tool>` split back apart. The second is the same instinct
`deny_unknown_fields` serves everywhere: a typo in an admission field is a server that
reaches no run, which is the failure the launch check exists to make loud.

**The document ships with one server on this machine, the delegate face, since build
ticket 26.** Adding another is a human's decision:
15.1 calls a server definition a command Hydra spawns inside every future run. The one
in `tests/fixtures/mcp-probe-server.py` is a fixture for measuring, not a server, and it
is not admitted to anything.

**The `cursor` dialect is Cursor's own shape, on both events.** 6.6 counts two response
shapes; ticket 09 found `beforeReadFile` takes only Cursor's, and the section on that
provider has the measurement. One shape per provider is the rule that survives.

**A `cursor-agent` run outside a repo, or in a repo carrying `.cursor/hooks.json` or
`.cursor/mcp.json`, is refused before a run id exists.** Harness spec 2 gives the rule
and 6.6 the mechanism; which check runs and when is this build's, and the same section
argues it.

**`.cursor/.gitignore` names the two files rather than everything.** 6.4's `*` is right
for a directory Hydra alone writes into and wrong for one the human's repo may track.

**`CURSOR_DATA_DIR` is on `env_allowlist`.** Contract 5.3 says the names passed, and 6.2
calls it a variable Hydra sets rather than one it passes through; the record follows
5.3, so a reader sees the redirect was in force.

**`mcp.resolved` on `cursor-agent` is the pre-flight listing's `ready` names.** Ticket
08 expected null. The listing is free and is taken after the approvals and the
disables, so it is the provider's own account of what the run will start.

**`tool_call.arguments` on `cursor-agent` is the hook's spelling of the input, rebuilt
from the stream's.** Contract 7.6 names the field and not its shape; 7.7 re-evaluates
against what the gate saw, and the gate saw the hook's spelling. The stream's object is
in `raw`.

**A signal death goes into `run_end.error` as `the child died on signal <name>`.**
Contract 6 defines the field as text Hydra captured that is not on the stream, and this
is that. It exists because the pty reports a signal death as exit code 1, which on
`cursor-agent` is also an orderly failure.

**Two `cursor-agent` outcomes are read off stderr.** `rate_limited` and `auth_failed`
have no frame to derive from on that provider's failure path, so the derivation reads
the noise lines the record keeps, keyed on the provider's own error class and phrase.
The section on that provider says why that is a text match and what happens when the
sentence moves.

## Not built

Out of scope for this slice, and each one has a document that already says what it is:

- One of harness spec 6.1's seven pre-flight checks on three of the four providers. Auth
  is built on `codex` alone, since build ticket 48, where `codex login status` answers before
  a spawn; on `claude`, `cursor-agent` and `agy` a logged-out binary still fails in its own
  words. What runs before a spawn today, in the order it runs, is the
  policy name, the preamble file, the MCP document, the depth cap, the concurrency cap,
  the policy containment check, then the environment, the provider version and the two
  repo files on the providers that read their gate out of a worktree, then the skill
  collision check, the MCP launch check, on `cursor-agent`, item 7's fatal pre-flight,
  and on `agy`, the free gate-mount check the spec does not ask for and the section on
  that provider argues for.
- `--shared-tree`, so nothing in this build runs two children in one checkout, and the
  MCP cost 6.5 gives that flag on Cursor has no producer.
- Every one of contract 8's nine outcomes has a producer now. `mcp_unavailable` was the
  last without one and it arrives through `run_end.refusal`, written by 6.1 item 6 when
  a policy admits a server the launch cannot deliver. `skill_collision` is 6.1's fifth
  check, and `timed_out` is the handle rather than the record, which the section above
  argues. No record on this machine carries `mcp_unavailable`: `tests/mcp.rs` drives the
  refusal against a fixture document, and the shipped document admits nothing to
  anybody, so producing one for real means a human adding a server.
- Nothing about harvest is unbuilt: `src/normalize/claude.rs` reads contract 7.2.1's
  transcript vocabulary since build ticket 27, and all four of 7.2's stream columns are read.
- A rate-limited or logged-out `agy` run. `provider_refused` keys on `api_error_status`,
  which that provider does not write, and neither path has been measured, so a run
  refused for quota comes out `failed`.
- Nothing about `agy` is unbuilt. When this was written one recorded run existed,
  `read-only`; since build ticket 19 on 2026-09-07 `agy` has written tickets and probes under
  `standard`, and `permissive` on that provider is still checked by `tests/gate.rs` against
  the measured payload shape and by no recorded run.
- An offerable `hydra evolve` patch has been merged by a human. The command is built, since
  build ticket 36, and ten passes have run: the sixth reached the panel and was blocked, and
  the tenth, on 2026-09-21, was offered by all four judges, so `rungs/<pass-id>` and an
  `ApprovedPatch` built from a real verdict exist now. The section above says what each pass
  found. Spec 4.11's merge is the human's, and the first one is commit 24cade6, the same day.

## What the hiring-panel review of 2026-09-22 changed

Seven readers took the repository as a portfolio submission for a harness engineering role: the
four providers through `hydra run` under `standard`, `claude` on Opus 5 (01a0c793-4b3c, 9.3M
tokens, ended `rate_limited` after writing its file), `codex` (01a0c793-726c, 7.4M),
`cursor-agent` on Composer 2.5 (01a0c795-f0a5, 1.9M; Opus and GPT-5.6 on that account were
`rate_limited` at launch) and `agy` on Gemini 3.1 Pro (01a0c799-18d2, 0.5M), plus three Claude
Code subagents on Opus with separate briefs: the public documents against the code, the Rust as
a pull request, and the threat model as an attacker. All seven said advance to interview;
`docs/reviews/2026-09-22-seven-readers-as-a-hiring-panel.md` is the consolidated report. What
changed in the code that day, each with its test:

- The gate answers a deny in the caller's dialect for a `cwd` that is not a string, a shell row
  whose command is not a string, a `file_write` row that names no path under every policy
  including `permissive`, a document nested past the interpreter's limit, a `network_read`
  whose URL spells a protected path, and anything `evaluate` raises. `tests/gate.rs`, five
  tests, 81 in the file. `hydra/gate-error` is a code the normalizer knows.
- `hydra export` redacts a PEM block to its footer, renders and verifies the preference lines
  before either file is created, and refuses an `--out` named `preference.jsonl`.
  `tests/export.rs`, two tests.
- `HYDRA_ROOT` is refused inside a seatbelt; a sibling's spawn directory is read-denied by the
  profile; the corpus denial is pinned under four spellings. `tests/spawn.rs`, three tests.
- `src/sandbox.rs` and `src/containment.rs` are on `PROTECTED_CODE_PATHS`.
- `hydra resume` on `codex` builds `exec resume <thread_id> --json` with the belt flags and
  hook override and neither `-C` nor `--sandbox`, measured on 0.154.0 on 2026-09-22. Build
  ticket 71.
- The harvest of an abandoned interactive run holds `flock(2)` on the record from before the
  first `has_run_end` read to the end, so two `hydra status` cannot both append the transcript.
- The delegate face reaps the watchers it started, one zombie per delegation until then.
- `changed_paths` and `dirty_outside` read git with `-z`, so a path git would C-quote reaches
  `protected_path_hit` as itself.
- `Provider::version` runs the vendor binary under the child's allowlist rather than the
  launcher's whole environment.
- The `codex` hook path is shell-quoted before it is TOML-escaped.
- `prune`, `status`, `usage recount` and `resume` take a run-id prefix through `resolver`, the
  way `show`, `logs` and `reap` did.
- `hydra doctor` reports the environment allowlist as in code and does not exit 1 for the two
  files only `hydra export` needs.

What it found that was a decision went to the map and the human answered the same day: the
whole checkout joins the deny list (map 65, build 74); admission runs under the proposer's own
profile with an empty `HYDRA_ROOT` (map 66, build 75); the seatbelt is defence behind the gate and
not a boundary (map 67, no code). Six build tickets followed, each on a `cursor-agent` run on
Composer 2.5 and applied by the session after review: git's index lock under the seatbelt (70,
run 01a0c924-5609), `codex` resume (71, 01a0c924-7d46, which measured that `codex exec resume`
runs in the caller's cwd and lifted the refusal), a deadline on the pty tee (72, 01a0c924-a494;
the review found the probe in `persistent_eof` dropping a real byte and `spawn` copied whole, both
fixed by hand), the gated interactive launch under the seatbelt (73, 01a0c955-f7a1), the checkout
on the deny list (74, 01a0c956-213e) and admission under the seatbelt (75, 01a0c956-462b). Build
70's run could not measure its own test inside its seatbelt and asserted that `git add` would
work; it did not, because the blob goes into the shared object store, and the session added that
carve-out. One pong per provider completed under the profile that denies the checkout, 01a0c960-a3a5
on `cursor-agent`, 01a0c960-c1f5 on `codex`, 01a0c960-e174 on `agy` and 01a0c961-009d on `claude`,
and a recorded `codex` resume under the seatbelt, 01a0c95b-5844, answered its second task.

**The replay runs the gate with `PWD` set to the record's `cwd`, since build ticket 76's run.**
The gate expands `$VAR` out of its own environment, which on a live run is the provider's, so
`HYDRA_ROOT=$PWD/.scratch-root cargo test` resolved under the worktree and was allowed. The reader
inherited its own `PWD`, the checkout, which build ticket 74 put on the deny list that afternoon, so
the corpus walk read five allowed commands on run 01a0c9d9-6652 as leaks the gate never made. The
suite that closed build 76 was reported green before that line was read, which was the session's
mistake; `tests/normalize.rs::a_variable_the_gate_expands_is_read_against_the_records_cwd_and_not_the_readers`
fails without the one-line fix in `src/policy/replay.rs` and passes with it, measured both ways.

What it found that this file already said and the README did not: the corpus walks are vacuous in
CI, `gated` needs every call refused, the concurrency cap was a check without a reservation
(closed by build ticket 81), and
`run_start` took thirteen positional arguments with two adjacent same-typed `Option` pairs (since
build ticket 83 it takes one `RunStartRequest` with named fields).

The second and third panels, the same four providers on 2026-09-23 and again that evening, are
`docs/reviews/2026-09-23-four-readers-second-panel.md` and
`docs/reviews/2026-09-24-four-readers-third-panel.md`. Build tickets 88 to 96 carry the second's
findings, G1 to G49, and 97 to 106 the third's, H1 to H48; each paragraph of this file they
changed names its ticket.

## What map ticket 68 changed

Map ticket 68 asked for an immune loop and got a narrower answer: no schedule, a corpus rung two
cannot edit, and the gate's claims made to match its code. Build tickets 77 and 78 carry the code.

**The gate reads a payload in the dialect it was compiled for** (build 77). `as_tool_call` reads
`toolCall` only on an `agy` gate and `beforeReadFile` only on a `cursor` gate, and a payload that
carries another dialect's envelope is refused as `hydra/wrong-envelope`, the rule argv already
followed with `hydra/wrong-dialect`. A `tool_input`, or an `agy` `toolCall.args`, that is present and
not an object is refused as `hydra/bad-payload`; before, it was read as `{}`, so a shell call with a
list for input reached neither the path check nor the floor. A missing `tool_input` is still read as
empty, for tools that send none. `hydra/wrong-envelope` is on `normalize::GATE_CODES`.

**The floor's Slack rules also run on a fetch tool's URL** (build 77): `url` on `WebFetch` and
`Fetch`, `Url` on `agy`'s `read_url_content`, which is the one field name here nobody has measured.
Search tools are not checked. Whether `network_read` gets a scope at all is map ticket 69.

**The raw pass says what it does** (build 77). It refuses a denied prefix written whole and in
canonical form. A path split by a quote, assembled by concatenation, percent-encoded, or spelled
with `..`, `//` or `/./` inside an interpreter or nested-shell token walks past it, and
`tests/gate.rs::a_protected_path_split_or_assembled_still_walks_past` pins each spelling as open
beside the older substitution pin. `SECURITY.md` names the class under "does not interpret shell".
On macOS the seatbelt still refuses what those spellings reach.

**The attack corpus lives in `tests/corpus/`** (build 79). `tests/corpus/mod.rs` holds the helpers
`tests/gate.rs` and `tests/spawn.rs` both used to carry: compile a gate into a scratch layout, ask
it, run a command under the real profile. `tests/corpus/README.md` is one row per attack class
from the map 68 research, each naming the mechanism that answers it and the test that probes it,
`Open` where Hydra does not answer (instruction files in the checkout, an MCP server changing its
descriptions after admission, both stated in `SECURITY.md`), and `not applicable` only where the
class cannot reach Hydra. `tests/corpus` is on `PROTECTED_CODE_PATHS`, so rung two's admission
refuses a patch that edits the corpus its patches are judged against. A new class enters as a row
and, where there is something to pin, a probe in `tests/gate.rs` or `tests/spawn.rs` using the
shared helpers. Baseline-test provenance, which protects every test file present at a pass's
`base_tree` rather than one directory, closed as build ticket 80 with build ticket 96. `tests/corpus`
stays a floor row as well, so a new file there is refused too.

**A non-ASCII root is refused before launch** (build 78). The gate folds case ASCII only, and the
macOS filesystem folds `É` too, so on a root with a cased non-ASCII letter every denial missed the
other case. `paths::refuse_non_ascii_roots` checks the resolved form of the data root and of
`hydra_home()` on `run`, `resume`, both interactive paths, `curate`, `evolve` and the face, and
`hydra doctor` prints each root as `ascii` or `non-ascii`, failing on the second. The launching
repository's `.git` is a third path the gate folds and it is not checked: a repo under a non-ASCII
directory still launches with that one denial case-blind. Whether such a repo is refused is open.

## `codex` and its skills, since build ticket 87

Map ticket 71 found that `codex` could not open a single skill Hydra injected: it reads only by
shell, the snapshot sits under `~/hydra`, and build ticket 74 made the shell check refuse any
command naming that. A panel of three readers chose a copy. `skills::inject` copies on
`Provider::reads_skills_by_shell()`, true for `codex` only, with files 0444 and directories 0555,
and keeps a symlink inside a skill as a link rather than following it. The seatbelt profile of a
`codex` launch ends with a write deny on `<worktree>/.codex/skills`. At exit the watcher compares
the copy with the snapshot in both directions, and a link swapped in or a `hydra.toml` added
counts, appending a note to `run_end.error` after the signal and tee notes;
`tests/outcome.rs::a_codex_skill_copy_that_changed_does_not_change_the_outcome` holds that the
note changes no outcome, because a completed `codex` run has a terminator.

**Reap holds back what Hydra injected and nothing else.** `reap::injected_skills` reads
`skills.linked` from line 0 of the run's record, or its pruned copy, and excludes `<root>/<name>` for
each; Hydra's own `*` `.gitignore` in a skill root is dropped before staging when it is untracked
and holds exactly `*`, so a skill the run authored under `.claude/skills` is committed. Before this
ticket a run that deleted that `.gitignore` got whatever it planted there committed on every
provider. With no record, reap holds back the four roots whole. The read-only copies need their
write bits back before `git worktree remove`, which `worktree::restore_write_bits` does.

Since build ticket 101 reap reads line 0 through a typed view, `LineZero` in `src/reap.rs`, with
the three fields it acts on: `provider`, `hydra.skills.linked` and `hydra.scopes.parent_git_dir`.
A `skills` block whose `linked` is not a list of strings, or one under a provider this build
cannot name, makes the line unreadable, and reap holds every skill root back as it does when
there is no record. The `Value` reading before it took both as "nothing was injected".
Since build ticket 99 a failed injection still says what it left: `skills::inject` returns
`InjectError { partial, error }`, naming each skill before its link or copy is made, and
`run::inject_skills` writes that list into the launch's `skills` group even on error, so the
failure record names what reached the checkout and reap holds it back.

## Rough edges worth knowing

- **A test that launches the built binary with `HYDRA_ROOT`, or execs a setuid binary, has to
  skip inside a seatbelt**, or it fails rung two's admission, which runs the suite in one.
  `tests/delegation.rs::launch_with_marker` asserts it is outside one, so a forgotten skip fails
  loudly. Build ticket 104.

- **`~/hydra` is this checkout on the machine this is developed on**, so everything
  harness spec 5 writes lands inside the source repo and `.gitignore` is what keeps it
  out of git. Ticket 07 found the trap that comes with it: the patterns were unanchored,
  so `skills/` matched `src/skills/` at any depth and hid a new source directory from
  `git status` with no warning. They are anchored to the repo root now. A new directory
  under `src/` sharing a name with one of Hydra's own is worth a `git status` glance
  either way.
- **A record carries the paths its scopes resolved to, and the corpus predates the
  field.** `policy` and `policy_sha` pin the rules and `scopes` pins what they were
  applied to, since build ticket 15. The edge that is left is the records already on
  disk: none of them carries the object, so each one re-derives against this build's
  `Layout` and says so once as a limit. Seen for real on 2026-09-06, which is why the
  field exists: a concurrent branch pointed `skill_store` at a per-run worktree of the
  store, and its run's ten in-scope reads re-derived on this branch as ten `GateLeak`
  findings. Nothing was wrong with either build. Moving a scope now rewrites the reading
  of records written before the move and of nothing written after it.
- **A run's own test suite can drive the real harness.** Build ticket 32's Composer run
  called `hydra::resume::resume` from a test with `Layout::discover()`, which read `~/hydra`
  from inside the gate, and wrote three spawn directories there before the test learned to
  take a scratch layout. Nothing launched, by luck of a dead cwd. The gate sees tool calls
  and `cargo test` is one `shell` call; the only thing between a worktree's tests and the
  corpus is a test author who passes a layout. Every launch text now says so, and a
  reviewer reads a new test file for `Layout::discover()` before applying the diff.
- **`HYDRA_ROOT` never reaches a recorded child, and on macOS a run cannot set it for its
  own.** 6.2's allowlist drops it and, since build ticket 88, `pty::open` clears the
  environment first. A run that exports it and calls `hydra run` is refused by `src/run.rs`
  inside a seatbelt (`tests/spawn.rs::hydra_root_is_refused_from_inside_a_seatbelt`). Linux
  launches unwrapped, so there a run can still move where its children are recorded.

- **An interactive session that exits before its first turn writes no transcript**, and the
  harvest writes a `run_end` with an error naming the path it looked for, since 2026-09-08.
  Before that the run sat with no `run_end` and `hydra status` re-reported the missing file.
- **A run with no `run_end` is not corrupt.** Contract section 6 says so. Killing the
  detached half mid-run produces one, and nothing completes it: `hydra status` reports it
  as abandoned and stops there, because completing it means harvest and no run on this
  machine has a vendor transcript to harvest from.
- **Killing `hydra run` no longer kills the run.** Ctrl-C on the foreground stops the
  view and leaves the child, its writer and its record alone, which is what harness spec
  9 asks for and is also a way to leak a run you meant to stop. `hydra status` finds it;
  nothing in this build can end it. That is deliberate rather than missing: nothing here
  signals a child, and the design gives removal to `hydra reap`, which a human types.
- **A frame that is not valid UTF-8 is stored lossily** and the count lands in
  `run_end.error`. It has not happened; `claude` writes JSON through Node.
- **A gate compiled before build ticket 14 has no `own_write_paths` key, and `gate.py`
  reads it with an empty default rather than failing.** A `KeyError` there would exit
  non-zero printing nothing on stdout, and 6.6 measured `claude` treating exactly that
  shape as permission for the call to proceed. The empty default is what keeps an
  old gate answering rather than failing open the day the deny list grows again.
- **A gate that fails to start still fails open on `claude`.** 6.6 measured it. The
  shebang is absolute and the script imports nothing outside the standard library,
  which is what that risk is spent on, and the sha check now catches a script that
  changed rather than one that will not run. An interpreter that is missing or broken
  is the case still open, and nothing checks at launch that the script runs. Adding
  such a check would be an eighth entry in 6.1's pre-flight list, which is a design
  decision rather than an implementation.
