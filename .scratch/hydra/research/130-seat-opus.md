# 130, seat Opus: the docket read against the code

Read on 2026-10-15 at `452f637`, which is `e10c63a` plus the commit that opened map 130. Nothing
was built or run; every claim below names the file it came from. "Readings" means
`.scratch/hydra/research/130-panel-readings.md`.

## 1. The Linux job

**Recommendation: narrow it today, but not to the command the readings name, then one ticket per
group, all before the flip.** The readings' option 1 runs `cargo test --release --lib --workspace`,
and that is not green. `crates/puppets/src/lib.rs` exists and `sheet.rs` is one of its modules, so
`sheet::tests::lgamma_is_cpythons_and_not_the_c_librarys` (`crates/puppets/src/sheet.rs:633`) is
a lib test. It is one of the group g failures, and `--lib` still runs it. The narrowed job needs
that test skipped by name or gated by `cfg` until group g lands. After that, each ticket removes one
skip, so the job only ever widens. Option 2 leaves red on the first click, which is the visitor
problem item 8 exists to fix. Option 3 is option 1 with the gate moved to the end.

What I checked, group by group:

- **a.** True. `tests/seats.rs:44-47` spells the root `/private/tmp/hs{pid}-{n}` for the socket
  path. A test-only fix: `/tmp` off macOS.
- **b.** True. `tests/corpus/mod.rs:672-675` panics when `CI` is set and `sandbox-exec` is
  missing, on any OS. The fix is `CI` and `cfg!(target_os = "macos")` together.
- **c.** True, and wider than the readings say. `identity()` (`src/worktree.rs:256-279`) returns
  no arguments when neither key is set, "and git then falls back to its own guess". `git_as_author`
  (`src/worktree.rs:351-356`) is the only path, and `commit_everything` has two callers,
  `src/reap.rs:510` and `src/rungs/curate.rs:317`. So the fallback names curate's proposal
  commits too, not only reap's. `tests/git_config.rs:353` is the `committed.unwrap()` of the
  attributes test. Its child (`isolated_test`, lines 22-45) redirects `XDG_CONFIG_HOME` and
  inherits `HOME`, which is why the Mac passes on the human's own `~/.gitconfig`.
- **d.** The readings are right about the outcome, but the test's own comment is wrong.
  `tests/gate.rs:3446-3468` says the single-call entry "reads PWD from the environment ... and
  never from the payload". `evaluate` in `src/policy/gate.py:1425-1427` takes `payload["cwd"]`
  whenever it is a non-empty string, and `resolve_shown` (lines 142-151) puts it through
  `os.path.realpath`. So the NUL reaches `os.path`. I did not find which call raises on 3.12 and
  not on 3.9.
- **e.** The claim matches the code. `tests/pty_tee.rs:145-184` pins one 200,000-byte line, and
  the tee in `src/pty.rs:196` onward reads 64 KiB chunks into `pending`. Unmeasured. If the tee is
  wrong, it is a product bug: Linux records would lose frames. It has to be measured before a
  Linux run records anything, not before the flip.
- **f.** `src/record/validate.rs:31` is the `assert!` inside `Report::assert_ok`. That is where
  the test panicked, not why. Unmeasured.
- **g.** Partly false. `c_mul_add` (`crates/puppets/src/sheet.rs:70-82`) already branches on
  `target_arch`: fused on aarch64, unfused elsewhere. `ubuntu-latest` is x86-64, and the
  expected bits were recorded on the arm64 Mac. The readings say the differential runs "against
  the runner's live CPython". It does not. `python_said` (`crates/puppets/tests/differential.rs:1703`)
  reads `tests/fixtures/python/`, and `crates/grader/tests/differential.rs:7` says "No test starts
  `python3`". The doc comment at `sheet.rs:75`, "checks the choice against the `python3` on the
  machine", is stale in the same way. So the x86-64 branch has never been compared with anything.
  `crates/pyjson/src/math.rs:5-29` is a thin wrapper over std (`exp`, `ln`, `powf`), so it
  reproduces whatever libm it links. On glibc, Rust and CPython would agree with each other and
  disagree with the Mac's hashes.
- **h.** Plausible. 65 files under `crates/puppets/tests/fixtures` contain
  `__CF_USER_TEXT_ENCODING`. A test-only fix: drop that key on both sides of the compare and
  normalize the program name in a stub's error line.

The three decisions:

- **c, whose identity.** A fixed literal, per key: `user.name=Hydra` and
  `user.email=hydra@localhost.invalid`, only for a key `identity()` found empty. Not the OS user
  name, which ticket 316 already rules out for `host`. Not the hostname either, which leaks the
  same way. Test: a child with `GIT_CONFIG_GLOBAL=/dev/null`, `GIT_CONFIG_NOSYSTEM=1` and a scratch
  `HOME` reaps, and `git log -1 --format='%an <%ae>'` prints the literal. A second case sets only
  `user.name` and gets the literal email beside the configured name.
- **d, the NUL `cwd`.** Refuse, on purpose. No POSIX path holds a NUL, so no real call carries
  one, and refusing when it cannot decide is what the gate is designed to do. Make it explicit:
  `evaluate` answers `hydra/bad-payload` for a `cwd` containing `\0` before any `os.path` call.
  Then both Pythons give the same deny, and the test asserts that deny instead of an answer
  measured on 3.9. `src/policy/` is on `PROTECTED_CODE_PATHS` (`src/rungs/evolve.rs:595`), which
  protects it from rung two, not from a build ticket.
- **g, per platform or a tolerance.** Per platform, no tolerance. `near_tie_ranks_as_python_lgamma`
  (`sheet.rs:650-657`) exists because a last bit swapped two agents in build 207, and a one-ulp
  tolerance gives that up. The ranking runs on the Mac. So gate the bit-exact tests to
  `cfg(all(target_os = "macos", target_arch = "aarch64"))`, with a comment saying why, and record
  a second fixture set only once a Linux machine ranks. A cheap experiment worth one CI run first:
  `ubuntu-24.04-arm` takes the fused branch, which would leave only the libm differences. If the
  human's VM runs on Apple Silicon it is arm64 too, so it cannot record x86-64 fixtures anyway.

Test I would add: the README's Linux sentence (`README.md:6-7`) names the same command the job
runs, so the two cannot drift apart.

## 2. Build 316

**Recommendation: additive, and narrower than the readings' additive. Land `host` now, leave
`sandbox` a string, and add `sandbox_kind` with build 318, when a second kind exists.**

The readings blame the run for something the ticket asked for. Ticket 316 says "Make it an object
`{kind, sha}` ... Write the new shape only; read both." Contract section 15
(`docs/run-record-contract.md:1416-1420`) says re-typing a `run_start` field "increments the
version". So bumping to 2 was the contract applied correctly to the instruction. The ticket was
wrong, not the run.

Two things I found that the readings do not say:

- Nothing reads `run_start.sandbox` today. Grepping `src/normalize/`, `src/walk.rs`,
  `src/show.rs`, `src/rungs/` and `crates/*/src` found no reader. Only `src/record/lines.rs` and
  `src/record/validate.rs` touch it. So 316 teaches four normalizers to read a field that no
  normalizer read before. `SCHEMA_VERSION` is 1 (`src/record/mod.rs:57`), and no reader branches on
  `schema_version`.
- `host.os` answers the problem statement on its own. A null `sandbox` on a `linux` host means the
  platform has no wrapper. On `macos` it means a choice, or a nested seatbelt. `kind` carries no
  information until there is a second value for it.

Tests beyond the readings' four: the validator rejects `host: null` and a `host` missing any of its
three leaves, per section 15's "a group's leaves are checked too" (`docs/run-record-contract.md:1433-1436`).
And the regenerated fixtures under `tests/fixtures/records/` carry a synthetic `machine`, not this
Mac's hostname, because those files go public.

## 3. Build 326

**Recommendation: treatment one, commit the files, with two changes to the ticket's text.**

Two corrections to the readings. They say the staged case exits 128 with `'sub/.git' not
recognized`. Build 325's Outcome says that is false, and `tests/cleanup.rs:1934-1940` asserts what
it really is: `git status --porcelain` fails with `not a git repository` and `failed in submodule
sub`. The 128 line belongs to the unstaged case's `git worktree remove`. The readings also call
both cases "red tests". They are green tests that pin a failed reap. And the unstaged case is worse
than "leaves the checkout": `tests/cleanup.rs:2013-2016` asserts the branch already holds a
`160000` gitlink commit when the removal fails.

The two changes:

1. **Do not rename to `.git.nested` inside the checkout.** The very next `git add -A` would commit
   `.git.nested/` into the run's branch as plain files: HEAD, config, refs, index, the sample hooks.
   Nor does the rename keep the history. The nested `.git` has no `objects/` (`tests/cleanup.rs:1466-1469`);
   its objects live in the run's store and arrive in the parent unreferenced, which is 325's three.
   Move the nested `.git` out of the tree, into the run's spawn directory, or exclude it from the
   add. Then say in reap's line that the nested history is not kept.
2. **Find `.git` files as well as directories.** A `git worktree add` or a submodule leaves a
   `gitdir:` file, not a directory.

Why treatment one: a refusal leaves the checkout exactly where today's failure leaves it, and reap
is typed by a human who reads its printed line. Tests beyond the readings' three: the branch tree
holds no `160000` entry and no path under any `.git` name, and one case covers a nested `.git`
file.

## 4. Build 321

**Recommendation: accept "written by the hook, not tamper-proof" and write it into the contract.
Do not sign. Add a corroboration check on the reader side instead.**

Signing cannot work inside one profile. `src/sandbox.rs:122-130` denies reads of every spawn
directory, and `183-185` reopens the run's own, because "the provider reads its hook config, its
MCP config and the gate itself out of there". The hook is a child of the provider, under the same
profile. So any secret the hook can read at the kernel, the run's shell can read at the kernel too,
and only the string floor stands between them. `CLAUDE.md`'s "Nothing here is enforced" section
records that the floor does not stop a determined shell read. Signing would be the floor wearing a
different hat.

What the code shows about the risk. The kernel exception is one literal path
(`src/sandbox.rs:235-240`). `copy_gate_refusals` (`src/watch.rs:336-366`) checks each line with
`valid_gate_refusal` and writes it through `writer.hydra_line`. A forged row therefore becomes a
line Hydra appears to have authored. That is the real cost, and the contract sentence should name
it. The hook writes shell refusals only: `gate.py:1797-1798` tests `capability_of(tool) == "shell"`.

Corroboration: every row carries `call_id`. The run's shell cannot write the record file, because
`src/sandbox.rs:117-121` denies the corpus. So `hydra doctor --walk` can flag a `gate_refusal`
whose `call_id` matches no `tool_call` in the provider's stream, or whose call has a successful
`tool_result`. A forger would then have to produce a real call with that id that really failed.
That costs far more than appending a line.

The test the readings say is missing, I would demand: a stub run, in the style of
`tests/detached.rs`, that appends a forged row to the sidecar mid-run. The record is read back,
and `doctor --walk` flags the row.

## 5. Build 320

**Recommendation: leave the three facts in the text, and spend no probes. If structured fields
are wanted, put them on the 321 sidecar row, which Hydra writes, Hydra reads, and no provider
parses.** That route has no drift risk, so it needs no probe.

The readings' reason checks out in substance. `docs/harness-spec.md:503` says `claude` "fails
**open** on a response in a dialect it does not recognise". An extra key inside a known dialect is
a different case, and nothing measures it, so "an extra key could turn a deny into an allow" is
untested rather than shown. 320's Outcome says the code before the colon did not change, so
`reason_code` and the walk read what they read before. The only reader of a field in the hook
reply is the provider, which does nothing with it. The limit of the sidecar route: it carries shell
refusals only (`gate.py:1797`), so a file-tool refusal's link stays in the text.

Test, if the sidecar route is taken: the 321 gate tests also assert the entry, token and link
fields, and assert that they equal what the reason text says.

## 6. Map 126

**Question 1: no carve-out in 7.7. Use `Foreign` and count it, which is mostly what the code does
today.**

The readings get 7.7 backwards. They say it "forbids the reader to take the layout off the record".
`docs/run-record-contract.md:816-820` says re-evaluation "resolves its scopes against
`run_start.scopes` where the record has it", and that "a reader never prefers its own layout over
a stored path". What the reader rebuilds from its own layout is what the record does not store:
root, home, the deny list.

The code settles which reading applies. `GateMark::of` (`src/normalize/mod.rs:755-763`) compiles
this build's gate for the replayed spawn and compares shas. A mismatch is `Foreign`, and
`why_not` says "the gate it names is not the one compiled here" (line 770). `cannot_back`
(lines 980-1010) turns that into a limit, not a leak, and the comment at 975-979 says
`src/rungs/corpus.rs` keeps limits out of the weighted disagreements. A Linux record's deny list
holds Linux paths, so its sha differs and it lands in `Foreign`. That is Opus's reading on map 119.
Gemini's path-check failure needs a record with no `gate_sha`, and no Linux record will lack one,
because `gate_sha` landed with build 49, more than a month before 315.

Replaying with the record's layout would let a record supply its own deny list. 7.7 exists to stop
exactly that. Refusing to walk would hide the records. The one change I would make: once `host`
exists, `Foreign` says "another host" apart from "another build", and the walk prints a count per
host.

**Question 2: the readings' "exclude from proposals that touch the gate" describes an empty set.**
`hydra evolve` refuses any patch reaching `src/policy`, `policies`, `src/sandbox.rs` or
`src/containment.rs` before review (`src/rungs/evolve.rs:591-604`, `628-636`, `813-814`), and
`curate` writes skills only. No Rungs proposal touches the gate. What is left is whether a
hook-only record counts as evidence for a skill or harness proposal. I would have Rungs refuse a
corpus that mixes hosts, at load, until build 318. Map 119 already puts the VM's records in their
own root, so a mixed corpus only happens on purpose. A weight for hook-only records is a number
nobody can justify yet.

Tests: a fixture with a foreign `host`, walked and counted under that host, never reported as a
leak. And `curate` over a two-host corpus refuses with a message naming both hosts.

## 7. Map 127

**Question 1: yes. Exempt the run's own `skill_snapshot` directory from the shell token pass,
compiled only when the launch is wrapped.**

The file tools already read it. `tests/gate.rs:1328-1345` asserts a read through a skill link is
allowed under `read-only`, `standard` and `curate`, through the `skill_snapshot` scope. The shell
refusal sits at `tests/gate.rs:2992-3059`, in a test still named
`a_shell_command_naming_the_skill_root_is_how_two_providers_read_a_skill` that now asserts the
opposite for files. The exemption brings the shell in line with the file tool. It tells the run
nothing it cannot already read. The readings say `cursor-agent` and `agy` read "through their own
file tools, which the floor does not see". The floor does not see them, but the gate's path scopes
do, and they allow the read.

The wrapped-only condition matters. The token pass cannot tell `cat` from a `cp` onto the same
path, so with the exemption in place a write there rests on the kernel's deny.

**Question 2: the own snapshot only.** The other read-allowed directory under `~/hydra` is the
run's own spawn directory (`src/sandbox.rs:183-185`). It holds `launch.json`, which carries the
task and the whole `run_start`, and the gate script itself. Exempting it hands the shell the gate's
source to study. Keep it out.

**Question 3:** moot if question 1 is yes. If no, the sentence belongs in `config/preamble.md`,
protected config 4, and the human writes it.

Tests: the 2992 cases flip to allowed for the own snapshot when wrapped and stay refused when not.
Another sha's snapshot stays refused. A `sandbox_*` test shows a write through the link failing at
the kernel. Then the readings' live measurement on `cursor-agent` and `agy`.

## 8. The publication list

**Order: the scan, then the `.gitignore`, then CI, the release, metadata, the allowlist sentence,
and `git gc` last.** The `.gitignore` moves up because it costs a minute and prevents the one
accident that cannot be undone. Four corrections:

- **Do not tag `v0.1.0`.** `CHANGELOG.md:103` already reads `## 0.2.0, 2026-09-08`,
  `Cargo.toml:6` says `version = "0.2.0"`, and `git tag` lists nothing. Closing Unreleased as
  0.1.0 would publish a version older than one the changelog already shipped. Close it as 0.3.0,
  or whatever the human picks, and bump `Cargo.toml` in the same commit. Either tag 0.2.0's commit
  or say in the notes that it was never tagged.
- **The `security` workflow file is not protected ground.** `.github` appears in none of harness
  spec 15.1's six, nor in `PROTECTED_CODE_PATHS` or `PROTECTED_CONFIG_PATHS`
  (`src/rungs/evolve.rs:591-636`). A grep for `github` in `docs/harness-spec.md` found nothing. The
  session may edit the file, and the Actions settings stay the human's. In
  `.github/workflows/security.yml`, the `osv-pr` job calls a reusable workflow and grants only
  `contents: read` and `actions: read`. A called workflow that asks for more, such as
  `security-events: write`, fails at startup. That is a likely cause, and I could not check it.
- **The `sha2` fix is not a grader-only change.** `format!("{:x}", Sha256::digest(..))` appears at
  nine sites: `src/export.rs:42`, `src/export_preference.rs:74`, `src/sandbox.rs:100` and `306`,
  `src/mcp.rs:123` and `621`, `src/spawn_config.rs:138`, `src/rungs/evolve.rs:1227` and
  `crates/grader/src/sheet.rs:201`. All four `Cargo.toml` files pin `sha2 = "0.10"`. If the bump
  reaches the root crate, the sandbox sha and the hook-config sha go through the new formatting,
  and any change in their output turns every old record `Foreign`. Demand a fixed-vector test: one
  known input's hex string, byte for byte, before and after.
- **Missing from the list: two more untracked files one `git add -A` away.**
  `.scratch/hydra/labeling/2026-09-29/shard-c.jsonl` (157,929 bytes) and
  `classes-c-agy-discarded.jsonl` (40,489) are untracked and not ignored. `.gitignore:30` already
  ignores `tasks-*.jsonl` in that tree, so someone has judged labeling files private before. Read
  both before the flip. Also add 316's synthetic hostname from section 2.

Confirmed: `config/export-allowlist.json:2` holds `"/Users/USER/hydra"`. It is protected config 3,
so the human edits it. I agree with research 129 about not rewriting the commit email.

Test to add: `git add -A --dry-run` lists neither labeling file and not `pendintes_hydra.md`.

## 9. The unopened tickets

1. **The `Ungated` Edit on build 327's run. Open it first.** A hook that went silent once is a
   security finding until shown otherwise. The finding comes from `src/normalize/claude.rs:615`,
   and `src/show.rs:179` prints it. Step one is reading that record, not building anything. I could
   not read it: `~/hydra/runs` is refused to this run.
2. **The five-second wall-clock test.** `tests/cockpit.rs:3865-3875` asserts
   `5 s <= waited < 7 s`. It is the same shape build 330 removed, and a test that flakes under load
   undermines a green CI right when strangers start looking.
3. **`hydra show --verdict`.** `src/show.rs` has no verdict printing today. Small and pleasant,
   low priority.
4. **The JSON refusal fields behind four probes.** Do not open. Section 5 offers the sidecar route,
   or nothing.
5. **OpenCode as a fifth provider.** Do not open before the flip. A provider means a
   `src/provider/` file, a `src/normalize/` file, a capability table row, a hook dialect and drift
   probes. The four that exist have open Linux and evidence work in this very ticket.

## Vote

Narrow the Linux job today with the lgamma test skipped, since the readings' `--lib` command is
not green, and fix the eight groups before the flip, deciding c as a fixed literal identity, d as
an explicit refusal, and g as per-platform with the bit-exact tests gated to arm64 macOS. Take the
smaller and additive option on every record question: `host` without re-typing `sandbox`, `Foreign`
as the code already does it, a hook-written sidecar with a reader-side corroboration check instead
of signatures, and the facts left in text. Reap commits nested files with the nested `.git` moved
out of the tree, the shell may read the run's own snapshot when wrapped, and the publication list
needs a 0.3.0 tag, not 0.1.0, plus two more untracked files on its ignore line.

## What I could not verify

- The CI log of run 38695261071: the count of 51 and each group's message. GitHub was not reached.
- Build 316's 27 files. They sit uncommitted in another worktree, outside this one; its branch
  holds no commit past `14a8746`.
- Which line of `gate.py` raises `ValueError` on Python 3.12 and not on 3.9.
- Whether the tee or the test is wrong in group e; group f entirely.
- That the unfused x86-64 branch is what produced `1.7917594692280554`.
- Build 327's `Ungated` record, the 952-record count, the Dependabot PR's diff, and the `security`
  workflow's actual startup error.
- No `cargo` command and no test was run, as the task asked.
