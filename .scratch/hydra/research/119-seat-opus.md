# Map 119, seat opus: what wraps a launch on a Linux VM

Written 2026-10-13 by one seat of four, inside a Hydra worktree on the Mac. Nothing here ran on
Linux, and I could not read `~/cerebro/research/cloud-vm.md`: it sits outside this worktree, so the
ticket's claims about distros and arm64 builds are taken as stated and marked as such below.

One finding comes before the four questions, because each of them assumes Hydra runs on Linux.

## Before the questions: Hydra has never been compiled for Linux

`src/procinfo.rs:16-24` reads a process start time through `sysctl` with `CTL_KERN = 1`,
`KERN_PROC = 14` and `libc::KERN_PROC_PID`. Those are Darwin's numbers and the module header says
so, at lines 7-11. The module has no `cfg` gate, `src/lib.rs:44` declares it for every target, and
`src/live.rs:397`, `src/watch.rs:215` and `src/interactive.rs:422` and `:581` call it. Every
liveness answer, and so `status`, `reap` and `prune`, goes through it.

What I measured: `rustup target list --installed` answers `aarch64-apple-darwin` alone, and
`.github/workflows/ci.yml:11` builds on `macos-latest` only. The one job on `ubuntu-latest`,
`security.yml:12`, runs `cargo-deny` and compiles nothing. `grep -rn target_os src` finds 7 sites in
2 files, `sandbox.rs` and `worktree.rs`, and none in `procinfo.rs`. What I believe and did not
measure: `libc` defines `KERN_PROC_PID` for the BSDs and Apple and not for Linux, so the crate does
not compile there. I did not install a Linux target to check, because that writes outside the
worktree.

So the first build ticket on this branch of the map is a Linux `cargo build` on the VM and a
`/proc/<pid>/stat` field 22 reader behind `cfg(target_os = "linux")`. Linux recycles pids too, so
the pid and start time pair of contract 5.2 carries over unchanged.

## 1. Hook-only on the VM, or recorded runs stay on the Mac

**What the code does today.** `src/sandbox.rs:46-48` makes `wraps_launches()` true on macOS
only. `src/spawn_config.rs:122-128` then writes no profile and returns `None` for both the path
and the sha, and `src/record/lines.rs:54` serializes that as `"sandbox": null`. It has no
`skip_serializing_if`, so the field is null rather than empty or absent, which is what contract
5.1's row says. The ticket's "empty" is a small misreading.

The ticket names the loss as the corpus spellings the substring floor misses. That one is real
and `SECURITY.md:44` and `:67` already say it. A second loss is larger and no document names it.
`inside_seatbelt()` at `src/sandbox.rs:74-85` returns `false` on every non-macOS target, and five
production checks lean on it as the one fact about ancestry a run cannot shed:

| Site | What it refuses inside a run on the Mac | On Linux |
|---|---|---|
| `src/approve.rs:85` | `hydra approve`, list or approve a seat request | allowed |
| `src/approve.rs:147` | `hydra deny` | allowed |
| `src/stop.rs:73` | `hydra stop` on any run | allowed |
| `src/run/mod.rs:295` | `HYDRA_ROOT` moving where a child is recorded | allowed |
| `src/face.rs:374` | delegation from inside a run, `hydra/seatbelt-delegation` | falls through to `run::run` |

I checked whether the gate catches any of these by pattern. `grep` for `approve`, `hydra stop`
and `HYDRA_ROOT` over `policies/` finds nothing, and harness spec 7.5 plus the `CLAUDE.md`
paragraph say the shell check reads the floor and the denied write paths and nothing else.
`hydra approve` with no id lists what waits, and neither command names the approvals directory.
So under `standard` on Linux a run can list the pending seat requests and approve its own, which
map 82 put in the human's hands alone. `IMPLEMENTATION.md:6088` already concedes the
`HYDRA_ROOT` row for Linux. The approve, deny and stop rows are not written down anywhere I found.

**Which policies this touches.** I read the five policy files. Two grant `shell`, `standard`
and `permissive`. `judge`, `read-only` and `curate` grant none, and harness spec 7.2 has
`read-only`'s bounded list waiting on the human. Without shell, the spellings the floor misses
and the five commands above are out of reach, and the file tools are scoped by the hook as on the
Mac. One exception holds: on a provider whose hook fails open, `cursor-agent` by spec 7.5 and
`codex` by `tests/drift.rs`'s paid probe, a dead hook leaves a no-shell run ungated, and on the
Mac the seatbelt still stood behind it.

**A precedent on the map.** Ticket 22's rule for hermes reads: a provider Hydra cannot gate is
given a config root Hydra owns or is not routed, "never launched with the gap recorded". The
seatbelt is defence rather than gate, map 67, so the rule does not bind here word for word. Its
spirit argues against a population whose whole difference is a field that says the defence was
missing.

**On `policy_sha` as the label.** It hashes `policies/` and the floor, which are the same bytes
on both machines at one commit. Making it differ means shipping a second policy set for Linux,
and then 7.7's replay compares rules that differ for a reason that has nothing to do with rules.
`sandbox: null` already says no sandbox wrapped the launch. It is ambiguous, because a plain
`--interactive` run on the Mac and every refused launch write null too, and question 4's `host`
field is what removes that.

**Recommendation.** Recorded runs under `standard` and `permissive` stay on the Mac until Linux
has a wrapper. `judge`, `read-only` and `curate` runs on `claude` and `agy` may run hook-only on
the VM once the five checks above get a Linux answer, labelled by `sandbox: null` plus a `host`
field and not by a second `policy_sha`.

**What would change my mind.** If the human's VM sessions are plain `--interactive` runs with no
`--policy`, the orchestrator itself is ungated on the Mac today too, and only the agents it
launches lose anything. If those agents are mostly panels and judges, hook-only costs little and
the line above is the whole policy. If they are mostly `standard` builders, as the build tickets
suggest, the wrapper is the price of moving at all.

## 2. Bubblewrap or Landlock, and what changes in row 4

**What the profile does today.** `compose()` at `src/sandbox.rs:105-277` writes `(allow
default)` and then twelve kinds of rule. SBPL takes the last matching rule, and the profile is
built on that: it denies a parent and allows the child back.

1. A `TIOCSTI` ioctl deny, line 110.
2. Write denies on every path 7.5 names, lines 111-116.
3. Read denies on the corpus, every spawn directory, `pruned/` and `approvals/`, lines 117-140.
4. A unix-socket connect deny under `spawn/`, then the run's own `seats.sock`, lines 147-156.
5. Under a bounded shell, `file-read-data` denies on the layout and the checkout, lines 157-165.
6. Read allows for the own spawn directory and the bounded seat's tree, lines 171-190.
7. A metadata-only allow on the spawn parent, for git's `lstat`, lines 196-201.
8. Write allows for what the run owns, lines 204-239.
9. Write denies on the worktree's private `HEAD`, `commondir` and `gitdir`, lines 216-226.
10. A write deny on `codex`'s copied skills, lines 243-249.
11. Write denies on the plugin paths, plus two regex rules for the manifest directory anywhere,
    lines 254-266.
12. A final worktree write deny under a bounded shell, lines 270-275.

`narrow_for_copilot` at lines 288-349 adds a deny on all of `HOME`, allows back, and a regex
prefix for `~/.claude.json` because `claude` renames a sibling over it.

**Bubblewrap.** A mount laid later covers one laid earlier, so `--dev-bind / /`, then
`--ro-bind P P` for each write deny, then `--tmpfs` or an empty directory over each read deny,
then `--bind` of what the run owns, is `compose()` in the same order with the same last-wins
rule. Kinds 2, 3, 6, 8, 9, 10 and 12 translate line for line. Kind 4 gets better: with `spawn/`
covered and the own spawn directory bound back, a sibling's socket does not exist in the run's
view, so no connect rule is needed. Kind 5 works because bwrap creates the mount points the
re-binds need, so ancestors still `lstat`. Bwrap can also cover `/run/user/<uid>/bus`, which
closes the `systemd-run --user` door, the Linux shape of the `launchctl submit` class that map 67
left open on the Mac.

What does not translate. Kind 11's regex has no mount equivalent, and neither has the copilot's
`~/.claude.json` prefix: a read-only `HOME` refuses the rename-over, and a writable `HOME` hands
back every denial under it that a later mount does not restore. A read deny reads as an empty
directory rather than "operation not permitted", so `tests/spawn.rs`'s error-text assertions need
a second spelling. Kind 1 needs `--new-session`, which detaches the controlling terminal from an
interactive TUI, or the host sysctl `dev.tty.legacy_tiocsti=0` on kernel 6.2 and later, which
`hydra doctor` can read from `/proc/sys/dev/tty/legacy_tiocsti`. Recorded runs get a pty the
watcher owns, so for them kind 1 protects Hydra's pty and not the human's terminal.

**Landlock.** Landlock rules only allow. Every right a ruleset handles is denied everywhere no
rule grants it, and a grant on a directory covers everything beneath. "Allow default, deny
`~/hydra/runs`" has no direct spelling. It works in one shape: the protected tree lives outside
every directory the run must write. Then you grant reads on `/usr`, `/etc`, `/home` and the rest,
leave `/srv/hydra` ungranted, and grant the own worktree and spawn directory inside it. Landlock
allows a grant on a subtree whose parent has none, and path lookup through ancestors is not
restricted, so that part is clean. It breaks on `HOME`. `claude` must write `HOME` itself for the
rename-over, a grant on `HOME` covers `~/hydra`, and the default layout is `~/hydra`
(`src/paths.rs:120-126`). So Landlock needs the layout, the checkout `hydra_home()` names, and the
repositories runs work on all moved out of `HOME`. That is a migration on the VM, and at that
point the profile is an allow-list shaped like `narrow_for_copilot`, not like `compose()`.

Kernel floors I believe and did not measure here. Truncation by path is handled from Landlock
ABI 3, kernel 6.2, so Debian 12's 6.1 lets a run truncate a corpus file it cannot open for
writing. Device ioctls, `TIOCSTI` included, from ABI 5, kernel 6.10. I know of no Landlock right
that restricts `connect` on a pathname unix socket up to ABI 6, so kind 4 would stay open.

**Closer match.** The ticket calls Landlock "path-based like the seatbelt's rules". Both are path
based, but SBPL here is a deny list with carve-outs and Landlock is an allow list, so bwrap is the
closer translation. Two outside facts point the same way, and I hold both as belief from before
my cutoff. Anthropic's sandbox runtime pairs `sandbox-exec` on macOS with bubblewrap on Linux.
And `.scratch/hydra/research/21-provider-env-requirements.md:201` found `CLAUDE_CODE_BUBBLEWRAP`
in the `claude` binary, so the vendor already runs under it.

**Nesting, which the Mac forbids and Linux may not.** `src/provider/codex.rs:82-86` and
`:114-118` hand `codex` `danger-full-access` wherever Hydra wraps the launch, because a second
seatbelt cannot apply inside the first. On Linux today `wraps_launches()` is false, so `codex`
runs `workspace-write` under its own sandbox, an argv no one has measured. Landlock stacks, and
bwrap nests when the outer one does not pass `--disable-userns`, so under either wrapper `codex`
might keep its own mode. Whether `workspace-write` lets it write the worktree's private index
under the parent's `.git` is unmeasured.

**What changes in row 4.** Little. `src/spawn_config.rs:122-128` picks the writer by
`wraps_launches()` and stores a path and a sha. On Linux it writes the bwrap argv beside the gate,
say `spawn/<run-id>/sandbox.bwrap`, one argument per line, and hashes those bytes. The doc
comments at lines 31-34 stop saying macOS. The rest lands outside row 4:

- `sandbox.rs`, the unnumbered code row of 15.2, gets a `compose_bwrap` beside `compose`.
- `wrap_command`, line 417, returns `/usr/bin/sandbox-exec` by name.
- `src/rungs/evolve.rs:1265` hard-codes `/usr/bin/sandbox-exec` for admission.
- `exec_available()` and both "missing on this Mac" refusals, `run/mod.rs:864` and
  `interactive.rs:357`, need a bwrap twin.
- `inside_seatbelt()` needs a Linux witness. A write probe against the corpus works under either
  wrapper. A bwrap mount namespace is inherited and its mounts are locked against a nested
  namespace, so it cannot be shed, and that is the property the five checks need.
- Contract 5.1's `sandbox` row needs a kind beside the sha, because a sha does not say whether it
  hashed SBPL or a bwrap argv. That is a new field under section 15's rule.

**Recommendation.** Bubblewrap, composed from the same `Spawn` in the same order as `compose()`,
with the `TIOCSTI` rule moved to a sysctl `hydra doctor` checks. Regex kind 11 and the copilot's
`~/.claude.json` prefix become named residuals in `SECURITY.md`.

**What would change my mind.** A VM where unprivileged user namespaces are off, or a container
sold as a VM, where `bwrap` cannot start. Ubuntu 23.10 and later gate them behind AppArmor, and
whether the stock `bwrap` profile lets Hydra's argv through is a measurement, not a belief. If
`bwrap --dev-bind / / --ro-bind ~/hydra/runs ~/hydra/runs true` fails there, Landlock with the
layout moved to `/srv` is the fallback, on kernel 6.2 or later.

## 3. Does the drift rule get a per-OS row

**What the code does today.** `src/provider/mod.rs:29-34` holds one `MEASURED_VERSIONS` row per
provider, with no OS. `src/doctor.rs:46-47` compares the installed `--version` against it.
`.scratch/hydra/build/plan.md:27-30` has every session re-read the versions and date every
provider claim. Each record already carries `provider_version` from the binary that ran.

**What differs by OS today, read from the code.** The `codex` argv differs:
`danger-full-access` on the Mac and `workspace-write` on Linux, `codex.rs:82-86`, and the test at
`codex.rs:647` encodes that branch. The copilot block names `/private/tmp`, `/private/var/folders`
and `~/Library/Caches/claude-cli-nodejs`, `sandbox.rs:323-331`, all macOS paths. `claude`'s
project slug is tested against `/private/var/folders`, `provider/claude.rs:1178-1190`. And the
fail-open measurement in `tests/drift.rs` ran under the seatbelt with `codex`'s own sandbox off,
a launch shape Linux does not have.

I did not run the four `--version` calls. The brief asks for no provider launch, and the table
moves per session anyway. I could not check the report's arm64 claim either, since it lives
outside this worktree.

**What I believe.** A version string names a release, and the behaviour `tests/drift.rs`
measures depends on the release and on the launch shape. On the Mac and on Linux the launch shape
differs for at least `codex`, so the same string is two measurements. Three of four providers also
self-update on their own clock, per `IMPLEMENTATION.md:92-100`, so two machines drift apart
regardless of OS. The table is per machine in practice, and the record's `provider_version` is
already the per-run truth.

**Recommendation.** Keep one version table, give it an OS column that names where each row was
measured, and have `hydra doctor` on the VM report "measured on macos only" until the drift tests
have run there. Do not split the rows by OS until a vendor's version strings actually differ
between platforms.

**What would change my mind.** A vendor whose version string differs per platform or per arch
for one release, `cursor-agent`'s `date-hash` shape being the likely one. Then the table needs a
row per OS because the strings force it.

## 4. A corpus written on two machines

**What the code does today.** `base_tree` is the sha `git write-tree` answered against the seed
index, contract 5.1's `worktree` row. A tree sha is content-addressed, so two machines seeding
the same tree write the same sha. That makes it the right name for "what the run started from"
and the wrong one for "where the run ran". It is null under `--shared-tree` too. Nothing on
`run_start` names the host. What tells the machines apart today is a fingerprint: `cwd` and
`scopes` begin `/Users/` or `/home/`, and `sandbox` is a sha or null. Contract 5.1 rejected a
fingerprint once, for `command` on the harness run, because a fingerprint stops being true when
the shape moves.

**Where the readers break, read from the code.**

- **`doctor --walk` stays honest and goes blind.** `denied_write_paths` puts
  `crate::paths::hydra_home()` and the reader's own layout into the gate,
  `src/policy/mod.rs:795-834`, and `:1175` writes that list into the compiled JSON. `gate_sha`
  at `:1257-1263` hashes those bytes. A VM record read on the laptop compiles a gate naming the
  laptop's paths, so `GateMark::of` at `src/normalize/mod.rs:743-751` files it `Foreign`, and
  `check_gate_leaks` claims no leak, `src/walk.rs:308-360`. No false alarm, and no 7.7
  re-evaluation either, for any record from the other machine.
- **`status` and `prune` act on runs that are not theirs.** `src/live.rs:55-62` calls a run
  without `run_end` `Live` only when a local process matches the pid and start time. A VM run
  still going, backed up to the laptop, reads `abandoned` there. `hydra prune` with no ids moves
  every non-live record, `src/prune.rs:72-89`, so it would move a running VM run's copy into
  `pruned/`. The next one-way sync writes it back to `runs/`, and a second prune then refuses,
  because the destination already holds that run id, `prune.rs:9-13`. `reap` reads the same
  liveness, and its git calls name a `/home/...` directory that does not exist on the laptop. I
  did not trace what `reap` does then.
- **Rungs reads both corpora as one.** `src/rungs/curate.rs` reads no `sandbox` field, and
  `docs/rungs-spec.md` names none. A VM run under `standard` could have edited the corpus with
  a spelling the floor misses, which is 15.2 row 12's promise without its kernel half. A failure
  that only happens on Linux, a skill that calls `pbcopy` or names `/Users`, ranks beside Mac
  failures with nothing to split them.

Run ids are UUIDv7 and do not collide across machines. Prefixes minted in the same second on two
machines can share more characters, which `resolver.rs` already treats as ambiguity.

**Recommendation.** Add a `host` object to `run_start`, `os`, `arch` and a machine name, under
contract section 15's rule for a new field. Back the VM's records up to their own root on the
laptop, not into `~/hydra/runs/`, so `status`, `prune` and `reap` there never see them, and read
them with `HYDRA_ROOT` pointed at that root. `base_tree` stays what it is.

**What would change my mind.** If the backup is read-only and nobody types `prune` or `reap` on
the laptop again, the separate root only buys clarity, and the `host` field alone is enough.
If the human wants one Rungs pass over both machines, the walk needs `hydra_home` and the layout
read off the record rather than the reader. That is a decision for the map, because contract
7.7 says the reader compiles the gate this build would write.

## Recommendation summary

0. Linux has never been compiled, and `procinfo.rs` is Darwin-only, so a Linux build with a `/proc` start-time reader comes before anything below.
1. `standard` and `permissive` stay on the Mac until a wrapper exists; no-shell policies on `claude` and `agy` may run hook-only once the five `inside_seatbelt()` checks, `hydra approve` among them, get a Linux answer.
2. Bubblewrap, because its mount order is SBPL's last-match rule and `compose()` translates almost line for line; row 4 changes only the writer it picks and the sha it stores, and the record's `sandbox` gains a kind.
3. One version table with an OS column saying where each row was measured, and `tests/drift.rs` run on the VM before its rows count there.
4. `base_tree` cannot tell machines apart because it names content; add `run_start.host`, and keep the VM's backup in its own root so the laptop's liveness, `prune` and `reap` never act on it.
