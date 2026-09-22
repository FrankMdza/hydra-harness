# Seven readers as a hiring panel

Date: 2026-09-22, on commit 54eb13c. Every finding below is either fixed in the commit that
publishes this report, held under a ticket number with nothing a reader could use before the fix,
or a limit `SECURITY.md` now states. The column on the right says which.

Method: the repository was handed to seven readers as a portfolio submission for a harness
engineering role at a frontier lab, with the candidate described as a self-taught data analyst
with no computer science degree, and each was told to decide whether it earns a technical
interview and to cite a file and lines for every claim. Four ran through `hydra run` under
`standard` in their own worktree, one per provider, on the best model each account would run that
day: `claude` on Opus 5 (run 01a0c793-4b3c, 9.3M tokens, ended `rate_limited` after writing its
file), `codex` on its default (01a0c793-726c, 7.4M), `cursor-agent` on Composer 2.5
(01a0c795-f0a5, 1.9M; Opus 5 and GPT-5.6 on that account came back `rate_limited` at launch) and
`agy` on Gemini 3.1 Pro (01a0c799-18d2, 0.5M). Three more were Claude Code subagents on Opus 5,
each with one brief: the public documents against the code, the whole crate as a pull request,
and the threat model as an attacker who is the model inside a run. The session that launched
them read the code beside them, drove a real seatbelt profile and a real compiled gate by hand,
and fetched the two arXiv papers the README cites, which nobody in the repository had done. No
reader ran the test suite; CI on macOS was green on the commit.

## Verdicts

All seven said advance to interview. The scores are 1 to 10 on the axes the brief named; the
subagents' briefs replaced one axis each and the row says so.

| Reader | Rust | Tests | Threat model | Docs | Depth | Verdict, in one line |
|---|---|---|---|---|---|---|
| `claude`, Opus 5 | 8 | 8 | 9 | 7 | 9 | the central claim is implemented rather than asserted; the public numbers are stale |
| `codex` | 7 | 7 | 5 | 5 | 8 | evidence survives interpretation; executing proposed code before review is a trust failure to repair |
| `cursor-agent`, Composer 2.5 | 8 | 8 | 9 | 9 | 9 | a contract-first harness, not a wrapper; the interview must establish who owns the decisions |
| `agy`, Gemini 3.1 Pro | 8 | 9 | 9 | 9 | 10 | read eight files in full and scored the highest; the least evidence per score |
| subagent, documents against code | 8 | 8 | 9 | 6 | 8 | the mechanisms are real where checked; the drift is in the marketing prose |
| subagent, the crate as a PR | 8 | 8 | 7 (process and filesystem) | 7 (as visible in code) | 9 | the ordering is load-bearing and tested; seven real bugs, two serious |
| subagent, the threat model | 8 (gate) | 5 (seatbelt) | 9 | 4 (secrets and export) | 9 | built a gate, attacked it, pinned an open hole as a passing test |

Two of the seven weighted the same fact opposite ways. The README's authorship table, which says
which build tickets an agent wrote, is the thing one reader would probe hardest in an interview
and the thing another called the honesty most candidates never offer. Both are right, and the
table is the answer to both: it is the corpus, by run id.

## Findings

| Id | Rating | Found by | Summary | Status |
|---|---|---|---|---|
| F1 | bug | crate, threat model | `HYDRA_ROOT` plus an unset marker let a `standard` run launch a wider child whose record landed where the kernel denies nothing | fixed: refused inside a seatbelt, `tests/spawn.rs` |
| F2 | bug | session, by probe | the gate died with a traceback on a hook payload whose `cwd` was not a string, which is a call that runs on `codex` | fixed: five payload shapes answered, catch-all, `tests/gate.rs` |
| F3 | bug | threat model, Opus | a PEM block was redacted to its header; the verifier then looked for the header | fixed: redaction runs to the footer, `tests/export.rs` |
| F4 | bug | `codex` | preference lines were verified after the events file existed, and `--out preference.jsonl` truncated itself | fixed, `tests/export.rs` |
| F5 | bug | `codex` | the `codex` resume argv carried two `exec`s and flags the subcommand does not take; no test covered it | fixed the same evening, build ticket 71: `codex exec resume` measured to run in the caller's cwd, the argv rebuilt, a recorded resume under the seatbelt answered |
| F6 | bug | crate | two concurrent `hydra status` could both harvest one abandoned interactive record | fixed: `flock(2)` on the record for the whole harvest |
| F7 | bug | crate | the delegate face never reaped the watchers it started, one zombie per delegation | fixed |
| F8 | bug | crate | `changed_paths` and `dirty_outside` split git output on newlines, so a path git C-quotes passed the protected-path check as a quoted literal | fixed: `-z` |
| F9 | bug | crate | `prune`, `status`, `usage recount` and `resume` joined a raw run id into a path | fixed: the resolver everywhere |
| F10 | bug | `codex` | the `codex` hook path went into the TOML override unquoted for the shell | fixed |
| F11 | bug | crate | `Provider::version` ran the vendor binary with the launcher's whole environment | fixed: the child's allowlist |
| F12 | design gap | Opus | `src/sandbox.rs` and `src/containment.rs` were outside rung two's protected set, the third mechanism to land outside it | fixed; spec 15.2 says so |
| F13 | open exposure | threat model | the binary `config/mcp.md` names is under `~/hydra/target/`, which the deny list does not cover | fixed the same evening: map ticket 65 closed with the human and build ticket 74 denies the whole checkout, `tests/spawn.rs` |
| F14 | open exposure | `codex`, crate | rung two's admission gate runs `cargo test` on a proposer's worktree outside the seatbelt before any judge reads the patch | fixed the same evening: map ticket 66 closed with the human and build ticket 75 runs admission under the proposer's profile, `tests/evolve.rs` |
| F15 | documentation overclaim | threat model | the seatbelt is `(allow default)` and binds the process tree; `SECURITY.md` read as if it bound the machine | documented, and map ticket 67 closed with the human the same evening: defence behind the gate, spec 7.5 says so |
| F16 | bug | session, by probe | inside a run `git add`, `git commit` and `git stash` fail on the index lock, because the private git directory is under the denied `.git` | fixed the same evening, build ticket 70: the private git directory and the object store are carved out, `git add` works, `git commit` still refused |
| F17 | bug | crate | the pty tee has no deadline, so a grandchild holding the slave leaves a record open forever | fixed the same evening, build ticket 72: the tee polls with a deadline after the child exits; on macOS the pty is revoked at leader exit, so the path is Linux's |
| F18 | documentation gap | documents, Opus | a gated interactive launch is recorded, compiles a gate and runs unwrapped | fixed the same evening, build ticket 73: the gated interactive launch is wrapped, measured by the human on run 01a0c9ca-1591 |
| F19 | documentation gap | documents, Opus, `codex` | the README's `gated` bullet said one denied call; the code needs every call refused | fixed |
| F20 | documentation gap | all four providers | "three" providers in five places, "26 of 60" tickets, "eleven" commands, "62 decision tickets", measured versions from 2026-09-15, `codex` "under two policies" | fixed: four, 35 of 69, twelve, 63 of 64, 2026-09-22, three |
| F21 | documentation gap | documents | `config/README.md` said `mcp.md` declares a server and, twenty lines later, that it ships with none | fixed |
| F22 | documentation gap | documents, threat model | `config/mcp.md` names an absolute path under this user's home; a clone elsewhere cannot start the face | documented in `config/README.md`; protected config #5 is the human's to edit |
| F23 | documentation gap | `codex` | the record contract said the gate sha is carried and, a paragraph later, that it is not | fixed |
| F24 | documentation gap | documents | CONTRIBUTING and the drift template asked for three version strings | fixed |
| F25 | documentation gap | Opus, `cursor-agent` | module docs drifted: "three providers" in the normalizer, a record module that said the normalizer did not exist, two doc blocks glued to the wrong function, one paragraph twice | fixed |
| F26 | design smell | Opus | `hydra doctor` exited 1 on a healthy install because the environment allowlist is code by design | fixed: reported as in code |
| F27 | open exposure | threat model | a write row whose table field name is wrong was allowed under `permissive` with no denied-path check | fixed: a write with no path is refused under every policy |
| F28 | open exposure | threat model | a `file://` URL through a fetch tool got no path check | fixed: `network_read` shares `mcp_call`'s raw pass |
| F29 | open exposure | threat model | every past run's `launch.json`, task string included, was readable from inside a run | fixed: sibling spawn directories read-denied, `tests/spawn.rs` |
| F30 | design smell | `codex` | the concurrency cap is a check without a reservation; two launchers seeing one slot both proceed | stated in `IMPLEMENTATION.md`, not fixed |
| F31 | design smell | Opus, crate | `run_start` takes thirteen positional arguments with two adjacent same-typed `Option` pairs | stated in `IMPLEMENTATION.md`, not fixed |
| F32 | test gap | all three subagents, Opus, `codex` | the corpus walks are vacuous in CI, which points `HYDRA_ROOT` at an empty root | README now says what green CI means |
| F33 | open exposure | threat model | the export verifier searches the same five prefixes the redactor removed, so it cannot fire on a class it knows | `SECURITY.md` states it; an independent verifier is not built |

Map ticket 64, delegation under the seatbelt, was not a panel finding and closed the same evening too:
the face refuses in Hydra's words from inside a seatbelt, build ticket 76. By the end of the day 27 of
the 33 rows were code with a test and six were limits the documents state.

Three things the readers checked and found to hold that are worth saying out loud: the two arXiv
papers exist under the ids and figures the README quotes, fetched that day; the outcome is
derived from the stream with the exit code carried as a fact, which every reader traced through
`src/outcome.rs`; and `tests/gate.rs` asserts that the assembled-path bypass is still open, with
a comment telling whoever closes it to rewrite the test, which four readers named as the thing
they would show a colleague.

## What the panel would ask

The union of the questions the seven readers said they would put to the candidate, each pointing
at a decision in the code rather than at a fact about it:

- `src/sandbox.rs` opens `(allow default)`. What can a process inside it still ask launchd to do,
  and what breaks on each provider when you deny it?
- `src/rungs/evolve.rs`: which process runs a proposer's `cargo test`, under what environment,
  and what does moving it cost the pass?
- `src/pty.rs` reads to EOF and `wait` runs afterwards. Where does the deadline go, and what does
  `run_end` say when it fires?
- `src/outcome.rs`: what does `completed` assert, and which labels are safe to train on?
- `src/env.rs` drops a wrong `HOME` rather than forwarding it. Why?
- `src/rungs/review.rs`: `ApprovedPatch` has one constructor and it checks that there are four
  verdicts. Four from one provider would pass. What would it cost to make the type carry the
  independence?
- `tests/gate.rs` pins an open hole as a passing test. What happens organisationally when
  someone closes it?

## Method, and what was not verified

Each reader's own method section lists what it read in full, skimmed and skipped; the four
provider reports are on the reaped branches' records and the session kept copies outside the
repository. The competitor counts in the README's comparison table have no source in the
repository and none of the readers could check them. "None merged unread" in the authorship
table is an attestation. The `codex` fail-open on a dead hook is measured by hand in build ticket
48 and has no automated test. The `agy` reader read eight files and gave the highest scores; its
report is included in the count and weighted accordingly here.
